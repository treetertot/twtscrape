use crate::error::{SResult, TwtScrapeError};
use std::fmt::Display;

pub mod error;
pub mod follow;
pub mod moderated_tweets;
#[cfg(feature = "scrape")]
pub mod scrape;
pub mod search;
pub mod timeline;
pub mod tweet;
pub mod user;
pub mod usertweets;

pub trait TwitterIdType: Display {
    fn to_u64(&self) -> SResult<u64>;
}

impl TwitterIdType for u64 {
    fn to_u64(&self) -> SResult<u64> {
        Ok(*self)
    }
}

impl TwitterIdType for &str {
    fn to_u64(&self) -> SResult<u64> {
        self.parse::<u64>()
            .map_err(|why| TwtScrapeError::IdParseError(why.to_string()))
    }
}

impl TwitterIdType for String {
    fn to_u64(&self) -> SResult<u64> {
        self.parse::<u64>()
            .map_err(|why| TwtScrapeError::IdParseError(why.to_string()))
    }
}

#[macro_export]
macro_rules! as_option {
    ($val:expr, $( $opt:expr ),+ ) => {
        if $( $val == $opt )||+ {
            None
        } else {
            Some($val)
        }
    };
}

#[cfg(feature = "scrape")]
pub trait FilterJSON {
    fn filter_json_err(&self) -> SResult<()>;
}

#[cfg(feature = "scrape")]
#[macro_export]
macro_rules! impl_filter_json {
    ($to:ty) => {
        impl $crate::FilterJSON for $to {
            fn filter_json_err(&self) -> SResult<()> {
                if let Some(why) = self.errors.first() {
                    if why.code != 37 {
                        return Err($crate::error::TwtScrapeError::TwitterJSONError(
                            why.code,
                            why.message.clone(),
                        ));
                    }
                }
                Ok(())
            }
        }
    };
}
