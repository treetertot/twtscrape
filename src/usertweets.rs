use crate::error::SResult;
use crate::error::TwtScrapeError::TwitterJSONError;
use crate::scrape::Scraper;
use crate::tweet::{Cursor, Tweet, TweetEnt, TweetItemContent, TweetResults};
use crate::user::{Error, User};
use chrono::{DateTime, Utc};
use rkyv::Archive;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt::Display;
use tracing::{span, warn};

pub fn twitter_request_url_user_tweet_and_replies(
    id: u64,
    cursor: Option<impl AsRef<str>>,
) -> String {
    match cursor {
        Some(crsr) => {
            let crsr = urlencoding::encode(crsr.as_ref());
            format!("https://twitter.com/i/api/graphql/s0hG9oAmWEYVBqOLJP-TBQ/UserTweetsAndReplies?variables=%7B%22userId%22%3A%22{id}%22%2C%22count%22%3A40%2C%22cursor%22%3A%22{crsr}%22%2C%22includePromotedContent%22%3Afalse%2C%22withCommunity%22%3Atrue%2C%22withSuperFollowsUserFields%22%3Atrue%2C%22withDownvotePerspective%22%3Afalse%2C%22withReactionsMetadata%22%3Afalse%2C%22withReactionsPerspective%22%3Afalse%2C%22withSuperFollowsTweetFields%22%3Atrue%2C%22withVoice%22%3Atrue%2C%22withV2Timeline%22%3Atrue%7D&features=%7B%22responsive_web_twitter_blue_verified_badge_is_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%2C%22unified_cards_ad_metadata_container_dynamic_card_content_query_enabled%22%3Atrue%2C%22tweetypie_unmention_optimization_enabled%22%3Atrue%2C%22responsive_web_uc_gql_enabled%22%3Atrue%2C%22vibe_api_enabled%22%3Atrue%2C%22responsive_web_edit_tweet_api_enabled%22%3Atrue%2C%22graphql_is_translatable_rweb_tweet_is_translatable_enabled%22%3Atrue%2C%22standardized_nudges_misinfo%22%3Atrue%2C%22tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled%22%3Afalse%2C%22interactive_text_enabled%22%3Atrue%2C%22responsive_web_text_conversations_enabled%22%3Afalse%2C%22responsive_web_enhance_cards_enabled%22%3Atrue%7D")
        }
        None => {
            format!("https://twitter.com/i/api/graphql/s0hG9oAmWEYVBqOLJP-TBQ/UserTweetsAndReplies?variables=%7B%22userId%22%3A%22{id}%22%2C%22count%22%3A40%2C%22includePromotedContent%22%3Afalse%2C%22withCommunity%22%3Atrue%2C%22withSuperFollowsUserFields%22%3Atrue%2C%22withDownvotePerspective%22%3Afalse%2C%22withReactionsMetadata%22%3Afalse%2C%22withReactionsPerspective%22%3Afalse%2C%22withSuperFollowsTweetFields%22%3Atrue%2C%22withVoice%22%3Atrue%2C%22withV2Timeline%22%3Atrue%7D&features=%7B%22responsive_web_twitter_blue_verified_badge_is_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%2C%22unified_cards_ad_metadata_container_dynamic_card_content_query_enabled%22%3Atrue%2C%22tweetypie_unmention_optimization_enabled%22%3Atrue%2C%22responsive_web_uc_gql_enabled%22%3Atrue%2C%22vibe_api_enabled%22%3Atrue%2C%22responsive_web_edit_tweet_api_enabled%22%3Atrue%2C%22graphql_is_translatable_rweb_tweet_is_translatable_enabled%22%3Atrue%2C%22standardized_nudges_misinfo%22%3Atrue%2C%22tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled%22%3Afalse%2C%22interactive_text_enabled%22%3Atrue%2C%22responsive_web_text_conversations_enabled%22%3Afalse%2C%22responsive_web_enhance_cards_enabled%22%3Atrue%7D")
        }
    }
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct UserTweetsAndReplies {
    pub users: Vec<User>,
    pub tweets: Vec<Tweet>,
}

impl UserTweetsAndReplies {
    #[tracing::instrument]
    pub async fn scroll_user_timeline(scraper: &Scraper, user_handle: String) -> SResult<Self> {
        let _span_ = span!(tracing::Level::TRACE, "user_handle", user_handle).entered();

        let user = User::new(scraper, &user_handle).await?;
        let timeline_request_url = twitter_request_url_user_tweet_and_replies(user.id, None);

        let mut timelines_requests = {
            let mut max = user.profile_stats.tweets;
            if max > 3200 {
                // un oh terminally online tankie posting alert
                max = 3200;
            }
            Vec::with_capacity(max as usize / 40)
        };

        let first_request = scraper
            .api_req::<UserTweetAndRepliesRequest>(scraper.make_get_req(timeline_request_url))
            .await?;
        // find the cursor
        let first_cursor = first_request.filter_cursor();

        if let Some(fc) = first_cursor {
            timelines_requests.append(
                &mut UserTweetAndRepliesRequest::scroll(scraper, user.id, fc.to_string())
                    .await?
                    .into(),
            );
        }

        // lets parse these requests

        let (mut tweets, mut users) = {
            let mut max = user.profile_stats.tweets;
            if max > 3200 {
                // un oh terminally online tankie posting alert
                max = 3200;
            }
            (
                Vec::with_capacity(max as usize * 2),
                Vec::with_capacity((user.profile_stats.following as usize).min(200)),
            )
        };

        for request in timelines_requests {
            for inst in request.data.user.result.timeline_v2.timeline.instructions {
                if let Instruction::TimelineAddEntries(add) = inst {
                    for entry in add.entries {
                        match entry {
                            Entry::HomeConversation(homeconvo) => {
                                for hcitem in homeconvo.content.items {
                                    let twt =
                                        match Tweet::new_from_entry(&hcitem.item.tweet_results) {
                                            Ok(twt) => twt,
                                            Err(why) => {
                                                warn!(
                                            user_handle,
                                            error = why,
                                            "Failed to get tweet for user timeline. Continuing."
                                        );
                                                continue;
                                            } // TODO: observability
                                        };

                                    let user = match hcitem.item.tweet_results {
                                        TweetResults::Ok(trr) => {
                                            match User::from_result(
                                                scraper,
                                                trr.core.user_results.result,
                                            )
                                            .await
                                            {
                                                Ok(user) => user,
                                                Err(why) => {
                                                    warn!(user_handle, error = why, "Failed to get user for user timeline. Continuing.");
                                                    continue;
                                                }
                                            }
                                        }
                                        TweetResults::Tombstone(_) => continue,
                                    };

                                    tweets.push(twt);
                                    users.push(user);
                                }
                            }
                            Entry::Tweet(tweet) => {
                                let twt = match Tweet::new_from_entry(
                                    &tweet.item_content.tweet_results,
                                ) {
                                    Ok(twt) => twt,
                                    Err(why) => {
                                        warn!(
                                            user_handle,
                                            error = why,
                                            "Failed to get tweet for user timeline. Continuing."
                                        );
                                        continue;
                                    } // TODO: observability
                                };
                                let user = match tweet.item_content.tweet_results {
                                    TweetResults::Ok(trr) => {
                                        match User::from_result(
                                            scraper,
                                            trr.core.user_results.result,
                                        )
                                        .await
                                        {
                                            Ok(user) => user,
                                            Err(why) => {
                                                warn!(user_handle, error = why, "Failed to get user for user timeline. Continuing.");
                                                continue;
                                            }
                                        }
                                    }
                                    TweetResults::Tombstone(_) => continue,
                                };

                                tweets.push(twt);
                                users.push(user);
                            }
                            Entry::Cursor(_) => continue,
                        }
                    }
                }
            }
        }

        Ok(UserTweetsAndReplies { users, tweets })
    }
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct UserTweetAndRepliesRequest {
    pub errors: Vec<Error>,
    pub data: UserTARData,
}

impl UserTweetAndRepliesRequest {
    pub(crate) fn json_request_filter_errors(&self) -> SResult<()> {
        if let Some(why) = self.errors.first() {
            if why.code != 37 {
                return Err(TwitterJSONError(why.code, why.message.clone()));
            }
        }
        Ok(())
    }

    pub(crate) fn filter_cursor(&self) -> Option<&str> {
        for inst in &self.data.user.result.timeline_v2.timeline.instructions {
            if let Instruction::TimelineAddEntries(add) = inst {
                for entry in &add.entries {
                    if let Entry::Cursor(c) = entry {
                        if c.content.item_content.cursor_type.starts_with("Bottom") {
                            return Some(&c.content.item_content.value);
                        }
                    }
                }
            }
        }

        None
    }

    #[tracing::instrument]
    pub(crate) async fn scroll(
        scraper: &Scraper,
        id: u64,
        first_cursor: String,
    ) -> SResult<VecDeque<Self>> {
        let mut requests = VecDeque::with_capacity(5);

        let mut cursor_counter = first_cursor.to_string();
        let mut break_on_next = false;
        loop {
            let scrolled_up_request = scraper
                .api_req::<UserTweetAndRepliesRequest>(scraper.make_get_req(
                    twitter_request_url_user_tweet_and_replies(id, Some(&cursor_counter)),
                ))
                .await?;

            scrolled_up_request.json_request_filter_errors()?;

            requests.push_front(scrolled_up_request);
            if break_on_next {
                break;
            }

            match scrolled_up_request.filter_cursor() {
                Some(bottom) => {
                    cursor_counter = bottom.to_string();
                }
                None => break_on_next = true,
            }
        }

        Ok(requests)
    }
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct UserTARData {
    pub user: UserRslt,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct UserRslt {
    pub result: Result,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct Result {
    pub __typename: String,
    pub timeline_v2: TimelineV2,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct TimelineV2 {
    pub timeline: Timeline,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct Timeline {
    pub instructions: Vec<Instruction>,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) enum Instruction {
    TimelineClearCache,
    TimelineAddEntries(TimelineAddEntry),
    TimelinePinEntry(),
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct TimelinePinEntry {
    pub entry: TlPinEntryEntry,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct TlPinEntryEntry {
    pub content: TlPinContent,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct TlPinContent {
    #[serde(rename = "itemContent")]
    pub item_content: TweetItemContent,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct TimelineAddEntry {
    pub entries: Vec<Entry>,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) enum Entry {
    HomeConversation(HomeConversation),
    Tweet(TweetEnt),
    Cursor(Cursor),
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct HomeConversation {
    pub content: HomeConversationContent,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct HomeConversationContent {
    pub items: Vec<HCItem>,
    pub metadata: HCConversationMeta,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct HCItem {
    pub item: TweetItemContent,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct HCConversationMeta {
    #[serde(rename = "allTweetIds")]
    pub all_tweet_ids: Vec<String>,
    pub enable_deduplication: bool,
}
