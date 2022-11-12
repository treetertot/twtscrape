use std::{future::Future, time::Duration};
use tokio::{
    sync::Mutex,
    time::{sleep, Instant},
};

// The parts of the scraper that evolve with time

//std mutex guard can't carry accross awaits
//std mutex is much faster!
//but we need to stop multiple processes from acquiring the token
#[derive(Debug)]
pub struct TimedToken(Mutex<TimeToken>);
impl TimedToken {
    pub fn new() -> Self {
        TimedToken(Mutex::new(TimeToken::dud()))
    }
    pub async fn init(&self, expiration: Duration, token: String) {
        let mut guard = self.0.lock().await;
        *guard = TimeToken::new(expiration, token);
    }
    // We're doing io. The extra allocation absolutely isn't a bottleneck
    pub async fn get_token<E>(
        &self,
        refresh: impl Future<Output = Result<String, E>>,
    ) -> Result<String, E> {
        let mut guard = self.0.lock().await;
        guard.get_token(refresh).await.map(|s| String::from(s))
    }
}

#[derive(Debug, Clone)]
struct TimeToken {
    token: String,
    creation: Instant,
    expiration: Duration,
}
impl TimeToken {
    fn dud() -> TimeToken {
        Self::new(Duration::ZERO, String::new())
    }
    fn new(expiration: Duration, token: String) -> TimeToken {
        TimeToken {
            token: token,
            creation: Instant::now(),
            expiration,
        }
    }
    async fn get_token<E>(
        &mut self,
        refresh: impl Future<Output = Result<String, E>>,
    ) -> Result<&str, E> {
        let now = Instant::now();
        if now.duration_since(self.creation) > self.expiration {
            let ntk = refresh.await?;
            *self = TimeToken {
                token: ntk,
                creation: now,
                expiration: self.expiration,
            };
        }
        Ok(&self.token)
    }
}

#[derive(Debug)]
pub struct Delayer {
    delay: Duration,
    last_invocation: Mutex<Instant>,
}
impl Delayer {
    pub fn new(delay: Option<Duration>) -> Delayer {
        let delay = delay.unwrap_or_default();
        let last_invocation = Mutex::new(Instant::now());
        Delayer {
            delay,
            last_invocation,
        }
    }
    pub async fn wait(&self) {
        if self.delay.is_zero() {
            return;
        }
        let mut last_time = self.last_invocation.lock().await;
        let current_time = Instant::now();
        let diff = current_time.duration_since(*last_time);
        *last_time = current_time;
        if diff < self.delay {
            sleep(self.delay - diff).await;
        }
    }
    //async fn wait(time: Duration) {
    //    sleep(self.delay - diff).await;
    //}
}
