use crate::error::{SResult, TwtScrapeError};
use std::fmt::Display;

pub mod error;
pub mod query;
pub mod scrape;
mod search;
pub mod timeline;
pub mod tweet;
pub mod user;

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
