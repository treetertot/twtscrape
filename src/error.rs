use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TwtScrapeError {
    #[error("ID Parse Error: {0}")]
    IdParseError(String),
    #[error("Request Failed: {0:?}")]
    RequestFailed(reqwest::Error),
    #[error("Error Request Status: {0:?}")]
    ErrRequestStatus(reqwest::Error),
    #[error("Failed to load JSON: {0:?}")]
    LoadJsonFailed(reqwest::Error),
    #[error("Failed to access schema")]
    SchemaAccessErr,
    #[error("Schema Error: {0:?}")]
    SchemaErr(reqwest::Error),
    #[error("ID Parse Error: {0:?}")]
    InvalidProxy(reqwest::Error),
    #[error("Failed to build client: {0:?}")]
    ClientBuildError(reqwest::Error),
    #[error("Twitter JSON Error: Code {0}, {1}")]
    TwitterJSONError(i32, String),
    #[error("Bad Rest ID (type {0}): {1}")]
    TwitterBadRestId(&'static str, String),
    #[error(
        "Failed to FACE THE FEAR(time and computers), MAKE THE FUTURE(parsing the datetime): {0}"
    )]
    TwitterBadTimeParse(String),
    #[error("The User's JSON as returned by Twitter was not AvailableUser.")]
    UserResultError,
    #[error("This Timeline Entry `{0}` Not Supported")]
    BadTimelineEntry(String),
    #[error("Schema Error for {0}: {1}")]
    BadJSONSchema(&'static str, String)
}

impl From<ParseIntError> for TwtScrapeError {
    fn from(value: ParseIntError) -> Self {
        Self::IdParseError(value.to_string())
    }
}

pub type SResult<T> = Result<T, TwtScrapeError>;
