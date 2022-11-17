use crate::error::SResult;
use crate::error::TwtScrapeError::TwitterBadRestId;
#[cfg(feature = "scrape")]
use crate::scrape::Scraper;
use rkyv::Archive;
use serde::de::{MapAccess, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::fmt;
use std::fmt::Display;
#[cfg(feature = "scrape")]
use url::Url;

#[cfg(feature = "scrape")]
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
    let query = urlencoding::encode(query.as_ref());
    url.set_query(Some(&format!("q={query}")));
    if let Some(c) = cursor {
        let c = urlencoding::encode(c.as_ref());
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

#[cfg(feature = "scrape")]
impl Search {
    #[tracing::instrument]
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
                        Entry::Item(item) => {
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
                        Entry::Cursor(op) => {
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
                                if let Entry::Item(item) = entry {
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
                                if let Entry::Cursor(op) = replace.entry {
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) enum Entry {
    Item(Item),
    Cursor(Operation),
}

impl<'de> Deserialize<'de> for Entry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            EntryId,
            SortIndex,
            Content,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("entry sort content")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "entryId" => Ok(Field::EntryId),
                            "sortIndex" => Ok(Field::SortIndex),
                            "content" => Ok(Field::Content),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct EntryVisitor;

        impl<'de> Visitor<'de> for EntryVisitor {
            type Value = Entry;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("enum Entry")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Entry, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut entry_id: Option<String> = None;
                let mut sort_index: Option<String> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::EntryId => {
                            entry_id = Some(map.next_value()?);
                        }
                        Field::SortIndex => {
                            sort_index = Some(map.next_value()?);
                        }
                        Field::Content => {
                            if let Some(entry) = &entry_id {
                                if entry.starts_with("sq-I") {
                                    Ok(Entry::Item(map.next_value()?))
                                } else if entry.starts_with("sq-cursor") {
                                    Ok(Entry::Cursor(map.next_value()?))
                                } else {
                                    Err(de::Error::unknown_variant(entry, &["sq-I", "sq-cursor"]))
                                }
                            }
                            Err(de::Error::unknown_variant("None", &["sq-I", "sq-cursor"]))
                        }
                    }
                }
                Err(de::Error::missing_field("content"))
            }
        }

        const VARIANTS: &[&str] = &["Item", "Cursor"];
        deserializer.deserialize_enum("Entry", VARIANTS, EntryVisitor)
    }
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
