use serde::{Deserialize, Serialize};
use std::fmt::Display;
use url::Url;

pub fn twitter_request_url_search(
    query: impl AsRef<str> + Display,
    cursor: Option<impl AsRef<str> + Display>,
) -> String {
    let mut url = Url::parse("https://twitter.com/i/api/2/search/adaptive.json").unwrap();

    url.set_query(Some("count=20"));
    url.set_query(Some("query_source=typed_query"));
    url.set_query(Some("pc=1"));
    url.set_query(Some("spelling_corrections=1"));
    url.set_query(Some(&format!("q={query}")));
    if let Some(c) = cursor {
        url.set_query(Some(&format!("cursor={c}")))
    }

    url.to_string()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Search {
    pub tweets: Vec<u64>,
}

impl Search {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct SearchRequest {
    pub timeline: Timeline,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Timeline {
    pub id: String,
    pub instructions: Vec<Instruction>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Instruction {
    pub add_entries: AddEntry,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct AddEntry {
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub entry_id: String,
    pub sort_index: String,
    pub content: Content,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Content {
    pub item: Item,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Item {
    pub content: ItemContent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ItemContent {
    pub tweet: SearchTweet,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct SearchTweet {
    pub id: String,
}
