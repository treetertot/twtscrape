use std::collections::VecDeque;
use crate::tweet::{Cursor, FilterCursorTweetRequest, TimelineTerminateTimeline, UserResults};
use crate::user::{Error, User};
use crate::usertweets::TimelineAddEntry;
use crate::TwitterIdType;
use rkyv::Archive;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use tracing::{warn};
use crate::error::SResult;
use crate::scrape::Scraper;

#[cfg(feature = "scrape")]
pub fn twitter_following_request(
    id: impl TwitterIdType + Display,
    following: FollowType,
    cursor: Option<impl AsRef<str>>,
) -> String {
    match following {
        FollowType::Following => match cursor {
            Some(cursor) => {
                let crsr = urlencoding::encode(cursor);
                format!("https://twitter.com/i/api/graphql/9rGM7YNDYuiqd0Cb0ZwLJw/Following?variables=%7B%22userId%22%3A%22{id}%22%2C%22count%22%3A20%2C%22cursor%22%3A%22{crsr}%22%2C%22includePromotedContent%22%3Afalse%2C%22withSuperFollowsUserFields%22%3Atrue%2C%22withDownvotePerspective%22%3Afalse%2C%22withReactionsMetadata%22%3Afalse%2C%22withReactionsPerspective%22%3Afalse%2C%22withSuperFollowsTweetFields%22%3Atrue%7D&features=%7B%22responsive_web_twitter_blue_verified_badge_is_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%2C%22unified_cards_ad_metadata_container_dynamic_card_content_query_enabled%22%3Atrue%2C%22tweetypie_unmention_optimization_enabled%22%3Atrue%2C%22responsive_web_uc_gql_enabled%22%3Atrue%2C%22vibe_api_enabled%22%3Atrue%2C%22responsive_web_edit_tweet_api_enabled%22%3Atrue%2C%22graphql_is_translatable_rweb_tweet_is_translatable_enabled%22%3Atrue%2C%22standardized_nudges_misinfo%22%3Atrue%2C%22tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled%22%3Afalse%2C%22interactive_text_enabled%22%3Atrue%2C%22responsive_web_text_conversations_enabled%22%3Afalse%2C%22responsive_web_enhance_cards_enabled%22%3Atrue%7D")
            }
            None => {
                format!("https://twitter.com/i/api/graphql/9rGM7YNDYuiqd0Cb0ZwLJw/Following?variables=%7B%22userId%22%3A%22{id}%22%2C%22count%22%3A20%2C%22includePromotedContent%22%3Afalse%2C%22withSuperFollowsUserFields%22%3Atrue%2C%22withDownvotePerspective%22%3Afalse%2C%22withReactionsMetadata%22%3Afalse%2C%22withReactionsPerspective%22%3Afalse%2C%22withSuperFollowsTweetFields%22%3Atrue%7D&features=%7B%22responsive_web_twitter_blue_verified_badge_is_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%2C%22unified_cards_ad_metadata_container_dynamic_card_content_query_enabled%22%3Atrue%2C%22tweetypie_unmention_optimization_enabled%22%3Atrue%2C%22responsive_web_uc_gql_enabled%22%3Atrue%2C%22vibe_api_enabled%22%3Atrue%2C%22responsive_web_edit_tweet_api_enabled%22%3Atrue%2C%22graphql_is_translatable_rweb_tweet_is_translatable_enabled%22%3Atrue%2C%22standardized_nudges_misinfo%22%3Atrue%2C%22tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled%22%3Afalse%2C%22interactive_text_enabled%22%3Atrue%2C%22responsive_web_text_conversations_enabled%22%3Afalse%2C%22responsive_web_enhance_cards_enabled%22%3Atrue%7D")
            }
        },
        FollowType::Followers => match cursor {
            Some(cursor) => {
                let crsr = urlencoding::encode(cursor);
                format!("https://twitter.com/i/api/graphql/_gXC5CopoM8fIgawvyGpIg/Followers?variables=%7B%22userId%22%3A%22{id}%22%2C%22count%22%3A20%2C%22cursor%22%3A%22{crsr}%22%2C%22includePromotedContent%22%3Afalse%2C%22withSuperFollowsUserFields%22%3Atrue%2C%22withDownvotePerspective%22%3Afalse%2C%22withReactionsMetadata%22%3Afalse%2C%22withReactionsPerspective%22%3Afalse%2C%22withSuperFollowsTweetFields%22%3Atrue%7D&features=%7B%22responsive_web_twitter_blue_verified_badge_is_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%2C%22unified_cards_ad_metadata_container_dynamic_card_content_query_enabled%22%3Atrue%2C%22tweetypie_unmention_optimization_enabled%22%3Atrue%2C%22responsive_web_uc_gql_enabled%22%3Atrue%2C%22vibe_api_enabled%22%3Atrue%2C%22responsive_web_edit_tweet_api_enabled%22%3Atrue%2C%22graphql_is_translatable_rweb_tweet_is_translatable_enabled%22%3Atrue%2C%22standardized_nudges_misinfo%22%3Atrue%2C%22tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled%22%3Afalse%2C%22interactive_text_enabled%22%3Atrue%2C%22responsive_web_text_conversations_enabled%22%3Afalse%2C%22responsive_web_enhance_cards_enabled%22%3Atrue%7D")
            }
            None => {
                format!("https://twitter.com/i/api/graphql/_gXC5CopoM8fIgawvyGpIg/Followers?variables=%7B%22userId%22%3A%22{id}%22%2C%22count%22%3A20%2C%22includePromotedContent%22%3Afalse%2C%22withSuperFollowsUserFields%22%3Atrue%2C%22withDownvotePerspective%22%3Afalse%2C%22withReactionsMetadata%22%3Afalse%2C%22withReactionsPerspective%22%3Afalse%2C%22withSuperFollowsTweetFields%22%3Atrue%7D&features=%7B%22responsive_web_twitter_blue_verified_badge_is_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%2C%22unified_cards_ad_metadata_container_dynamic_card_content_query_enabled%22%3Atrue%2C%22tweetypie_unmention_optimization_enabled%22%3Atrue%2C%22responsive_web_uc_gql_enabled%22%3Atrue%2C%22vibe_api_enabled%22%3Atrue%2C%22responsive_web_edit_tweet_api_enabled%22%3Atrue%2C%22graphql_is_translatable_rweb_tweet_is_translatable_enabled%22%3Atrue%2C%22standardized_nudges_misinfo%22%3Atrue%2C%22tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled%22%3Afalse%2C%22interactive_text_enabled%22%3Atrue%2C%22responsive_web_text_conversations_enabled%22%3Afalse%2C%22responsive_web_enhance_cards_enabled%22%3Atrue%7D")
            }
        },
    }
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Follows {
    pub ftype: FollowType,
    pub data: Vec<User>,
}

#[cfg(feature = "scrape")]
impl Follows {
    #[tracing::instrument]
    pub async fn get_user_follow(scraper: &Scraper, id: u64, ftype: FollowType) -> SResult<Self> {
        let mut follow_page_requests = Vec::with_capacity(50);

        let first_request = scraper
            .api_req::<FollowReq>(scraper.make_get_req(twitter_following_request(id, ftype, None)))
            .await?;
        // find the cursor
        let first_cursor = first_request.filter_cursor(FilterCursorTweetRequest::Bottom);

        if let Some(fc) = first_cursor {
            follow_page_requests.append(
                &mut FollowReq::scroll(scraper, user.id, ftype, fc)
                    .await?
                    .into(),
            );
        }

        let mut users = Vec::with_capacity(1000);

        for req in follow_page_requests {
            if let Rslt::User(tl) = req.data.result {
                for inst in tl.timeline.instructions {
                    if let Instruction::TimelineAddEntries(tl_add) = inst {
                        for entry in tl_add.entries {
                            if let  Entry::User(usr) = entry {
                                match User::from_result(scraper, usr.content.item_content.result.result).await {
                                    Ok(us) => {
                                        users.push(us);
                                    }
                                    Err(why) => {
                                        warn!(error = why, user_id = id, "Failed to get data. Skipping...")
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }


        users.shrink_to_fit();

        Ok(Self { ftype, data: users })
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum FollowType {
    Followers,
    Following,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct FollowReq {
    pub errors: Vec<Error>,
    pub data: Data,
}

#[cfg(feature = "scrape")]
impl FollowReq {
    pub(crate) fn filter_cursor(&self, filter: FilterCursorTweetRequest) -> Option<&str> {
        if let Rslt::User(tl) = &self.data.result {
            for inst in &tl.timeline.instructions {
                if let Instruction::TimelineAddEntries(tl_add) = inst {
                    for entry in &tl_add.entries {
                        if let  Entry::Cursor(crsr) = entry {
                            match cursor {
                                FilterCursorTweetRequest::Top => {
                                    if crsr.entry_id.starts_with("cursor-top") {
                                        return Some(crsr.content.item_content.value.as_str());
                                    }
                                }
                                FilterCursorTweetRequest::Bottom => {
                                    if crsr.entry_id.starts_with("cursor-bottom")
                                        || crsr.entry_id.starts_with("cursor-showmorethreads")
                                    {
                                        return Some(crsr.content.item_content.value.as_str());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    #[tracing::instrument]
    pub(crate) async fn scroll(scraper: &Scraper, id: u64, ftype: FollowType, first_cursor: &str) -> SResult<VecDeque<Self>> {
        let mut requests = VecDeque::with_capacity(5);

        let mut cursor_counter = first_cursor.to_string();
        let mut break_on_next = false;
        loop {
            let scrolled_up_request = scraper
                .api_req::<FollowReq>(scraper.make_get_req(
                    twitter_following_request(id, ftype, Some(&cursor_counter)),
                ))
                .await?;

            scrolled_up_request.json_request_filter_errors()?;

            requests.push_front(scrolled_up_request);
            if break_on_next {
                break;
            }

            match scrolled_up_request.filter_cursor(FilterCursorTweetRequest::Bottom) {
                Some(bottom) => {
                    cursor_counter = bottom.to_string();
                }
                None => break_on_next = true,
            }
        }

        Ok(requests)
    }
}

#[cfg(feature = "scrape")]
crate::impl_filter_json!(FollowReq);

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct Data {
    pub result: Rslt,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(tag = "__typename")]
pub(crate) enum Rslt {
    User(Timeline),
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct Timeline {
    pub timeline: InnerTimeline,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct InnerTimeline {
    pub instructions: Vec<Instruction>,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(tag = "type")]
pub(crate) enum Instruction {
    TimelineClearCache,
    TimelineTerminateTimeline(TimelineTerminateTimeline),
    TimelineAddEntries(TimelineAddEntry),
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct TimelineAddEntries {
    pub entries: Vec<Entry>,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) enum Entry {
    User(Usr),
    Cursor(Cursor),
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct Usr {
    pub content: Content,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct Content {
    #[serde(rename = "itemContent")]
    pub item_content: ItemContent,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct ItemContent {
    pub result: UserResults,
}
