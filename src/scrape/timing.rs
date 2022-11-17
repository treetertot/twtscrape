use nanorand::{Rng, WyRand};
use std::env::var;
use std::sync::Arc;
use std::{future::Future, time::Duration};
use tokio::{
    join,
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
    #[tracing::instrument]
    pub async fn init(&self, expiration: Duration, token: String) {
        let mut guard = self.0.lock().await;
        *guard = TimeToken::new(expiration, token);
    }
    // We're doing io. The extra allocation absolutely isn't a bottleneck
    #[tracing::instrument]
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
            token,
            creation: Instant::now(),
            expiration,
        }
    }

    #[tracing::instrument]
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
    delay: u64,
    variation: u64,
    last_invocation: Mutex<Instant>,
    rng: Arc<Mutex<WyRand>>,
}
impl Delayer {
    pub fn new(delay: Option<Duration>, variation: Option<Duration>) -> Delayer {
        let delay = delay.unwrap_or_default();
        let last_invocation = Mutex::new(Instant::now());

        Delayer {
            delay: delay.as_millis() as u64,
            variation: variation.unwrap_or_default().as_millis() as u64,
            last_invocation,
            rng: Arc::new(Mutex::new(WyRand::new())),
        }
    }

    #[tracing::instrument]
    pub async fn wait(&self) {
        if self.delay == 0 {
            return;
        }

        let (mut last_time, mut rng) = {
            let timefut = self.last_invocation.lock();
            let rngfut = self.rng.lock();
            join!(timefut, rngfut)
        };

        let current_time = Instant::now();
        let diff = current_time.duration_since(*last_time);

        *last_time = current_time;

        if diff <= self.delay {
            let delay = Duration::from_millis(self.delay);
            let random = Duration::from_millis(rng.generate_range(0..self.variation));
            sleep((delay - diff) + random).await;
        }
    }
    //async fn wait(time: Duration) {
    //    sleep(self.delay - diff).await;
    //}
}
