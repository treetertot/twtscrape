use std::{collections::HashMap, time::Duration};

use reqwest::{Client, Proxy, RequestBuilder};

mod timing;
use serde::de::DeserializeOwned;
use serde_json::Value;
use timing::*;

//Intended to be associated with a single task
//Making the time pieces internally mutable with a mutex could make this shareable
#[derive(Debug)]
pub struct Scraper {
    bearer_token: String,
    client: Client,
    delayer: Delayer,
    guest_token: TimedToken,

    cookie: Option<String>,
    x_csrf_token: Option<String>,
}
impl Scraper {
    // equivalent to api.GetGuestToken
    async fn refresh_token(&self) -> Result<String, ScraperErr> {
        let successful_response = self
            .client
            .post("https://api.twitter.com/1.1/guest/activate.json")
            .header("Authorization", format!("Bearer {}", self.bearer_token))
            .send()
            .await
            .map_err(ScraperErr::RequestFailed)?
            .error_for_status()
            .map_err(ScraperErr::ErrRequestStatus)?;

        let mut data: HashMap<String, Value> = successful_response
            .json()
            .await
            .map_err(ScraperErr::LoadJsonFailed)?;

        match data.remove("guest_token") {
            Some(Value::String(s)) => Ok(s),
            _ => Err(ScraperErr::SchemaAccessErr),
        }
    }

    pub async fn make_get_req(&self, url: &str) -> RequestBuilder {
        self.client.get(url)
    }
    pub async fn api_req<T: DeserializeOwned>(
        &self,
        request: RequestBuilder,
    ) -> Result<T, ScraperErr> {
        self.delayer.wait().await;
        let token = self.guest_token.get_token(self.refresh_token()).await?;
        let headed = request
            .header("Authorization", format!("Bearer {}", self.bearer_token))
            .header("X-Guest-Token", token);
        let cookied = match (&self.cookie, &self.x_csrf_token) {
            (Some(cookie), Some(xtk)) => {
                headed.header("Cookie", cookie).header("x-csrf-token", xtk)
            }
            _ => headed,
        };
        cookied
            .send()
            .await
            .map_err(ScraperErr::RequestFailed)?
            .error_for_status()
            .map_err(ScraperErr::ErrRequestStatus)?
            .json()
            .await
            .map_err(ScraperErr::SchemaErr)
    }
}

#[derive(Debug)]
pub enum ScraperErr {
    RequestFailed(reqwest::Error),
    ErrRequestStatus(reqwest::Error),
    LoadJsonFailed(reqwest::Error),
    SchemaAccessErr,
    SchemaErr(reqwest::Error),
    InvalidProxy(reqwest::Error),
    ClientBuildError(reqwest::Error),
}

#[test]
fn make_scraper() {
    tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap()
    .block_on(async {
        let _scraper = ScraperBuilder::new().finish().await.unwrap();
        //println!("{:?}", scraper);
    });
}

#[derive(Debug, Clone)]
pub struct ScraperBuilder {
    bearer_token: String,
    delay: Option<Duration>,
    cookie: Option<String>,
    x_csrf_token: Option<String>,
    proxy: Option<String>,
}
impl ScraperBuilder {
    pub fn new() -> Self {
        ScraperBuilder {
            bearer_token: "AAAAAAAAAAAAAAAAAAAAAPYXBAAAAAAACLXUNDekMxqa8h%2F40K4moUkGsoc%3DTYfbDKbT3jJPCEVnMYqilB28NHfOPqkca3qaAxGfsyKCs0wRbw".into(),
            delay: None,
            cookie: None,
            x_csrf_token: None,
            proxy: None
        }
    }
    pub fn with_bearer_token(mut self, token: String) -> Self {
        self.bearer_token = token;
        self
    }
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
    pub fn with_cookie(mut self, cookie: String, x_csrf_token: String) -> Self {
        self.cookie = Some(cookie);
        self.x_csrf_token = Some(x_csrf_token);
        self
    }
    pub fn with_proxy(mut self, addr: String) -> Self {
        self.proxy = Some(addr);
        self
    }
    pub async fn finish(self) -> Result<Scraper, ScraperErr> {
        let ScraperBuilder {
            bearer_token,
            delay,
            cookie,
            x_csrf_token,
            proxy,
        } = self;
        let scpr = Scraper {
            bearer_token,
            client: {
                let builder = Client::builder();
                match proxy {
                    Some(proxy) => {
                        builder.proxy(Proxy::http(proxy).map_err(ScraperErr::InvalidProxy)?)
                    }
                    None => builder,
                }
                .timeout(Duration::from_secs(10))
                .build()
                .map_err( ScraperErr::ClientBuildError)?
            },
            delayer: Delayer::new(delay),
            guest_token: TimedToken::new(),
            cookie,
            x_csrf_token,
        };
        let token = scpr.refresh_token().await?;
        scpr.guest_token
            .init(Duration::from_secs(60 * 60 * 3), token)
            .await;
        Ok(scpr)
    }
}
