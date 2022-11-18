use ahash::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::cookie::{CookieStore, Jar};
use reqwest::{Client, Proxy, RequestBuilder, Response, StatusCode};

mod timing;
use crate::error::TwtScrapeError::{ErrRequestStatus, InvalidProxy, RequestFailed};
use crate::error::{SResult, TwtScrapeError};
use serde::de::DeserializeOwned;
use serde_json::Value;
use timing::*;
use tracing::warn;

//Intended to be associated with a single task
//Making the time pieces internally mutable with a mutex could make this shareable
#[derive(Debug)]
pub struct Scraper {
    bearer_token: String,
    client: Client,
    delayer: Delayer,
    guest_token: TimedToken,
    cookie: Arc<Jar>,
}

impl Scraper {
    // equivalent to api.GetGuestToken
    #[tracing::instrument]
    async fn refresh_token(&self) -> SResult<String> {
        let successful_response = self
            .client
            .post("https://api.twitter.com/1.1/guest/activate.json")
            .header("Authorization", format!("Bearer {}", self.bearer_token))
            .send()
            .await
            .map_err(RequestFailed)?
            .error_for_status()
            .map_err(ErrRequestStatus)?;

        let mut data: HashMap<String, Value> = successful_response
            .json()
            .await
            .map_err(TwtScrapeError::LoadJsonFailed)?;

        match data.remove("guest_token") {
            Some(Value::String(s)) => Ok(s),
            _ => Err(TwtScrapeError::SchemaAccessErr),
        }
    }

    pub fn make_get_req(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.client.get(url.as_ref())
    }

    #[tracing::instrument]
    pub async fn api_req<T: DeserializeOwned>(&self, request: RequestBuilder) -> SResult<T> {
        let response = self.api_req_raw_request(request).await?;
        response.json().await.map_err(TwtScrapeError::SchemaErr)
    }

    #[tracing::instrument]
    pub async fn api_req_raw_request(&self, request: RequestBuilder) -> SResult<Response> {
        self.delayer.wait().await;
        let token = self.guest_token.get_token(self.refresh_token()).await?;
        let headed = request
            .header("X-Guest-Token", token)
            .bearer_auth(&self.bearer_token);

        match headed
            .send()
            .await
            .map_err(RequestFailed)?
            .error_for_status()
        {
            Ok(req) => Ok(req),
            Err(why) => {
                warn!(error = why, "Got an error while asking twitter. Retrying.");
                // twitter randomly tells us to fuck off
                // retrying after a wait usually works
                let mut std_delay_secs = 1;
                if let Some(sc) = why.status() {
                    if sc == StatusCode::TOO_MANY_REQUESTS {
                        std_delay_secs = 5;
                    }
                }
                tokio::time::sleep(Duration::from_secs(std_delay_secs)).await;

                let token = self.guest_token.get_token(self.refresh_token()).await?;
                let headed = request
                    .header("Authorization", format!("Bearer {}", self.bearer_token))
                    .header("X-Guest-Token", token);

                Ok(headed
                    .send()
                    .await
                    .map_err(RequestFailed)?
                    .error_for_status()
                    .map_err(ErrRequestStatus)?)
            }
        }
    }
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
    delay: Option<u64>,
    variation: Option<u64>,
    proxy: Option<String>,
    proxy_auth: Option<(String, String)>,
    user_agent: Option<String>,
    cookie: Option<Arc<Jar>>,
}
impl ScraperBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_bearer_token(mut self, token: String) -> Self {
        self.bearer_token = token;
        self
    }
    pub fn with_delay_millis(mut self, delay: u64) -> Self {
        self.delay = Some(delay);
        self
    }

    pub fn with_delay_variation_millis(mut self, var: u64) -> Self {
        self.variation = Some(var);
        self
    }

    pub fn with_proxy(mut self, addr: String) -> Self {
        self.proxy = Some(addr);
        self
    }

    pub fn with_proxy_authentication(mut self, user: String, password: String) -> Self {
        self.proxy_auth = Some((user, password));
        self
    }

    pub fn with_cookies(mut self, cookies: HashMap<String, String>) -> Self {
        let mut jar = Jar::default();
        for (cookiek, cookieb) in &cookies {
            jar.add_cookie_str(
                &format!("{cookiek}={cookieb}"),
                "https://twitter.com".parse().unwrap(),
            )
        }
        self.cookie = Some(Arc::new(jar));
        self
    }

    pub fn with_ua(mut self, ua: String) -> Self {
        self.user_agent = Some(ua);
        self
    }

    #[tracing::instrument]
    pub async fn finish(self) -> Result<Scraper, TwtScrapeError> {
        let ScraperBuilder {
            bearer_token,
            delay,
            variation,
            proxy,
            proxy_auth,
            user_agent,
            cookie,
        } = self;

        let delayer = Delayer::new(
            delay.map(Duration::from_millis),
            variation.map(Duration::from_millis),
        );

        let jar = cookie.unwrap_or(Arc::new(Jar::default()));

        let scpr = Scraper {
            bearer_token,
            client: {
                let mut builder = Client::builder();
                if let Some(ua) = user_agent {
                    builder.user_agent(ua)
                }
                match proxy {
                    Some(proxy) => {
                        let mut proxybld = Proxy::https(proxy).map_err(InvalidProxy)?;
                        if let Some((user, password)) = proxy_auth {
                            proxybld = proxybld.basic_auth(&user, &password);
                        }
                        builder.proxy(proxybld)
                    }
                    None => builder,
                }
                .timeout(Duration::from_secs(10))
                .cookie_store(true)
                .cookie_provider(jar.clone())
                .build()
                .map_err(TwtScrapeError::ClientBuildError)?
            },
            delayer,
            guest_token: TimedToken::new(),
            cookie: jar,
        };
        let token = scpr.refresh_token().await?;
        scpr.guest_token
            .init(Duration::from_secs(60 * 60 * 3), token)
            .await;
        Ok(scpr)
    }
}

impl Default for ScraperBuilder {
    fn default() -> Self {
        ScraperBuilder {
            bearer_token: "AAAAAAAAAAAAAAAAAAAAAPYXBAAAAAAACLXUNDekMxqa8h%2F40K4moUkGsoc%3DTYfbDKbT3jJPCEVnMYqilB28NHfOPqkca3qaAxGfsyKCs0wRbw".into(),
            delay: None,
            cookie: None,
            proxy: None,
            proxy_auth: None,
            variation: None,
            user_agent: None
        }
    }
}
