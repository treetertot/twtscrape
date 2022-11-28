use crate::error::SResult;
use crate::error::TwtScrapeError::TwitterJSONError;
use crate::scrape::Scraper;
use crate::tweet::{Entry, FilterCursorTweetRequest, Instruction, Tweet};
use crate::user::Error;
use crate::{FilterJSON, TwitterIdType};
use rkyv::Archive;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt::Display;
use tracing::{instrument, warn};

#[cfg(feature = "scrape")]
pub fn twitter_moderated_req(
    tweet_id: impl TwitterIdType + Display,
    cursor: Option<impl AsRef<str>>,
) -> String {
    match cursor {
        Some(cursor) => {
            let crsr = urlencoding::encode(cursor.as_ref());
            format!("https://twitter.com/i/api/graphql/c9IdrvgCZw7oxPZFPBpyrg/ModeratedTimeline?variables=%7B%22rootTweetId%22%3A%22{tweet_id}%22%2C%22cursor%22%3A%22{crsr}%22%2C%22count%22%3A20%2C%22includePromotedContent%22%3Afalse%2C%22withSuperFollowsUserFields%22%3Atrue%2C%22withDownvotePerspective%22%3Afalse%2C%22withReactionsMetadata%22%3Afalse%2C%22withReactionsPerspective%22%3Afalse%2C%22withSuperFollowsTweetFields%22%3Atrue%7D%26features%3D%7B%22responsive_web_twitter_blue_verified_badge_is_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%2C%22unified_cards_ad_metadata_container_dynamic_card_content_query_enabled%22%3Atrue%2C%22tweetypie_unmention_optimization_enabled%22%3Atrue%2C%22responsive_web_uc_gql_enabled%22%3Atrue%2C%22vibe_api_enabled%22%3Atrue%2C%22responsive_web_edit_tweet_api_enabled%22%3Atrue%2C%22graphql_is_translatable_rweb_tweet_is_translatable_enabled%22%3Atrue%2C%22standardized_nudges_misinfo%22%3Atrue%2C%22tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled%22%3Afalse%2C%22interactive_text_enabled%22%3Atrue%2C%22responsive_web_text_conversations_enabled%22%3Afalse%2C%22responsive_web_enhance_cards_enabled%22%3Atrue%7D")
        }
        None => {
            format!("https://twitter.com/i/api/graphql/c9IdrvgCZw7oxPZFPBpyrg/ModeratedTimeline?variables=%7B%22rootTweetId%22%3A%22{tweet_id}%22%2C%22count%22%3A20%2C%22includePromotedContent%22%3Afalse%2C%22withSuperFollowsUserFields%22%3Atrue%2C%22withDownvotePerspective%22%3Afalse%2C%22withReactionsMetadata%22%3Afalse%2C%22withReactionsPerspective%22%3Afalse%2C%22withSuperFollowsTweetFields%22%3Atrue%7D&features=%7B%22responsive_web_twitter_blue_verified_badge_is_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%2C%22unified_cards_ad_metadata_container_dynamic_card_content_query_enabled%22%3Atrue%2C%22tweetypie_unmention_optimization_enabled%22%3Atrue%2C%22responsive_web_uc_gql_enabled%22%3Atrue%2C%22vibe_api_enabled%22%3Atrue%2C%22responsive_web_edit_tweet_api_enabled%22%3Atrue%2C%22graphql_is_translatable_rweb_tweet_is_translatable_enabled%22%3Atrue%2C%22standardized_nudges_misinfo%22%3Atrue%2C%22tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled%22%3Afalse%2C%22interactive_text_enabled%22%3Atrue%2C%22responsive_web_text_conversations_enabled%22%3Afalse%2C%22responsive_web_enhance_cards_enabled%22%3Atrue%7D")
        }
    }
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct ModeratedTweets {
    pub of_tweet: u64,
    pub tweets: Vec<Tweet>,
}

#[cfg(feature = "scrape")]
impl ModeratedTweets {
    #[instrument]
    pub async fn moderated_tweets(scraper: &Scraper, tweet_id: u64) -> SResult<Self> {
        let first_request = scraper
            .api_req::<ModTweetsReq>(scraper.make_get_req(twitter_moderated_req(tweet_id, None)))
            .await?;

        first_request.filter_json_err()?;

        let mut moderated_reqs = Vec::with_capacity(5);

        let first_cursor = first_request.filter_cursor(FilterCursorTweetRequest::Bottom);

        if let Some(cursor) = first_cursor {
            moderated_reqs.append(
                &mut ModTweetsReq::scroll(
                    scraper,
                    tweet_id,
                    cursor.to_string(),
                    FilterCursorTweetRequest::Bottom,
                )
                .await?
                .into(),
            );
        }

        let mut tweets = Vec::with_capacity(moderated_reqs.len() * 5);

        for modtwt in moderated_reqs {
            if let Rslt::TimelineResponse(tlr) = modtwt.data.tweet.result {
                for inst in tlr.instructions {
                    if let Instruction::TimelineAddEntries(entry) = inst {
                        for entry in entry.entries {
                            if let Entry::Tweet(twtent) = entry {
                                let twet =
                                    Tweet::new_from_entry(&twtent.item_content.tweet_results)?;
                                tweets.push(twet);
                            }
                        }
                    }
                }
            }
        }

        tweets.shrink_to_fit();
        Ok(ModeratedTweets {
            of_tweet: tweet_id,
            tweets,
        })
    }
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct ModTweetsReq {
    pub errors: Vec<Error>,
    pub data: Data,
}

#[cfg(feature = "scrape")]
impl ModTweetsReq {
    pub(crate) fn filter_cursor(&self, filter: FilterCursorTweetRequest) -> Option<&str> {
        if let Rslt::TimelineResponse(tlr) = &self.data.tweet.result {
            for inst in tlr.instructions {
                inst.filter_cursor(filter)
            }
        }

        None
    }

    #[tracing::instrument]
    pub(crate) async fn scroll(
        scraper: &Scraper,
        id: impl TwitterIdType + Display,
        first_cursor: String,
        filter: FilterCursorTweetRequest,
    ) -> SResult<VecDeque<Self>> {
        let mut requests = VecDeque::with_capacity(5);

        let mut cursor_counter = first_cursor.to_string();
        let mut break_on_next = false;
        loop {
            let scrolled_up_request = scraper
                .api_req::<ModTweetsReq>(
                    scraper.make_get_req(twitter_moderated_req(&id, Some(&cursor_counter))),
                )
                .await?;

            if let Err(why) = scrolled_up_request.json_request_filter_errors() {
                warn!(error = why, "Error while scrolling, ending early.")
            }

            requests.push_front(scrolled_up_request);
            if break_on_next {
                break;
            }

            match scrolled_up_request.filter_cursor(filter) {
                Some(up) => {
                    cursor_counter = up.to_string();
                }
                None => break_on_next = true,
            }
        }

        Ok(requests)
    }
}

#[cfg(feature = "scrape")]
crate::impl_filter_json!(ModTweetsReq);

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct Data {
    pub tweet: ModeratedTwt,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct ModeratedTwt {
    pub result: Rslt,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
#[serde(tag = "__typename")]
pub(crate) enum Rslt {
    #[serde(rename = "timeline_response")]
    TimelineResponse(TimelineResponse),
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TimelineResponse {
    pub instructions: Vec<Instruction>,
}
