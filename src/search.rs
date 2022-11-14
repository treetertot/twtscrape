use crate::error::SResult;
use crate::error::TwtScrapeError::TwitterBadRestId;
use crate::scrape::Scraper;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use rkyv::Archive;
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
    url.set_query(Some("tweet_search_mode=live"));
    url.set_query(Some(&format!("q={query}")));
    if let Some(c) = cursor {
        url.set_query(Some(&format!("cursor={c}")))
    }

    url.to_string()
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Search {
    pub tweets: Vec<u64>,
}

impl Search {
    pub async fn make_query(scraper: &Scraper, query: impl AsRef<str>) -> SResult<Self> {
        let first_request = scraper
            .api_req::<SearchRequest>(
                scraper.make_get_req(twitter_request_url_search(query.as_ref(), None)),
            )
            .await?;

        let mut tweets = Vec::with_capacity(20);

        let mut next_cursor: Option<String> = None;

        for inst in first_request.timeline.instructions {
            if let Instruction::AddEntry(add) = inst {
                for entry in add.entries {
                    match entry.content {
                        Content::Item(item) => {
                            if item.content.tweet.id.is_empty() || item.content.tweet.id == "0" {
                                return Err(TwitterBadRestId(
                                    "Search Tweet RestID",
                                    item.content.tweet.id,
                                ));
                            }

                            tweets.push(item.content.tweet.id.parse::<u64>().map_err(|why| {
                                TwitterBadRestId("Search Tweet RestID", why.to_string())
                            })?);
                        }
                        Content::Operation(op) => {
                            if entry.entry_id.starts_with("sq-cursor-bottom") {
                                next_cursor = Some(op.cursor.value)
                            }
                        }
                    }
                }
            }
        }

        if let Some(next) = next_cursor {
            let mut cursor_counter = next;
            loop {
                let mut request = scraper
                    .api_req::<SearchRequest>(scraper.make_get_req(twitter_request_url_search(
                        query.as_ref(),
                        Some(&cursor_counter),
                    )))
                    .await?;

                for inst in request.timeline.instructions {
                    match inst {
                        Instruction::AddEntry(add) => {
                            for entry in add.entries {
                                if let Content::Item(item) = entry {
                                    if item.content.tweet.id.is_empty()
                                        || item.content.tweet.id == "0"
                                    {
                                        return Err(TwitterBadRestId(
                                            "Search Tweet RestID",
                                            item.content.tweet.id,
                                        ));
                                    }

                                    tweets.push(item.content.tweet.id.parse::<u64>().map_err(
                                        |why| {
                                            TwitterBadRestId("Search Tweet RestID", why.to_string())
                                        },
                                    )?);
                                }
                            }
                        }
                        Instruction::ReplaceEntry(replace) => {
                            if replace.entry_id_to_replace.starts_with("sq-cursor-bottom") {
                                if let Content::Operation(op) = replace.entry.content {
                                    match op.cursor.value.strip_prefix("") {
                                        Some(new) => {
                                            cursor_counter = new.to_string();
                                        }
                                        None => break,
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        tweets.shrink_to_fit();

        Ok(Self { tweets })
    }
}

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
pub(crate) enum Instruction {
    #[serde(rename(deserialize = "addEntry"))]
    AddEntry(AddEntry),
    #[serde(rename(deserialize = "replaceEntry"))]
    ReplaceEntry(ReplaceEntry),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ReplaceEntry {
    #[serde(rename = "entryIdToReplace")]
    pub entry_id_to_replace: String,
    pub entry: Entry,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct AddEntry {
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Entry {
    #[serde(rename = "entryId")]
    pub entry_id: String,
    #[serde(rename = "sortIndex")]
    pub sort_index: String,
    pub content: Content,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum Content {
    #[serde(rename(deserialize = "item"))]
    Item(Item),
    #[serde(rename(deserialize = "operation"))]
    Operation(Operation),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Operation {
    pub cursor: Cursor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Cursor {
    pub value: String,
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
