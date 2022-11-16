use crate::as_option;
use crate::error::SResult;
use crate::error::TwtScrapeError::{
    TwitterBadRestId, TwitterBadTimeParse, TwitterJSONError, UserResultError,
};
#[cfg(feature = "scraper")]
use crate::scrape::Scraper;
use chrono::{DateTime, Utc};
use rkyv::Archive;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Display;
#[cfg(feature = "scraper")]
pub const TWITTER_IGNORE_ERROR_CODE: i32 = 37;
// "Fri Oct 09 08:16:38 +0000 2015"
#[cfg(feature = "scraper")]
pub const JOINDATE_PARSE_STR: &str = "%a %b %d %T %z %Y";

#[cfg(feature = "scraper")]
pub fn twitter_request_url_handle(handle: &str) -> String {
    format!("https://twitter.com/i/api/graphql/ptQPCD7NrFS_TW71Lq07nw/UserByScreenName?variables%3D%7B%22screen_name%22%3A%22{handle}%22%2C%22withSafetyModeUserFields%22%3Atrue%2C%22withSuperFollowsUserFields%22%3Atrue%7D%26features%3D%7B%22responsive_web_twitter_blue_verified_badge_is_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%7D")
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct User {
    pub id: u64,
    pub avatar: Avatar,
    pub name: ProfileName,
    pub profile_stats: ProfileStats,
    pub additional_info: ProfileAdditionalInfo,
    pub bio: String,
    pub pinned_tweet_id: Option<u64>,
    pub is_sensitive: bool,
    pub is_protected: bool,
}

impl User {
    #[tracing::instrument]
    pub(crate) async fn from_result(scraper: &Scraper, result: TwtResult) -> SResult<Self> {
        if let TwtResult::User(user) = result {
            if user.rest_id.is_empty() || user.rest_id == "0" {
                return Err(TwitterBadRestId("user", user.rest_id));
            }

            let website = {
                let redirect = scraper
                    .api_req_raw_request(scraper.make_get_req(user.legacy.url))
                    .await?;
                as_option!(redirect.url().to_string(), "")
            };

            let joined = DateTime::<Utc>::from(
                DateTime::parse_from_str(&user.legacy.created, JOINDATE_PARSE_STR)
                    .map_err(|why| TwitterBadTimeParse(why.to_string()))?,
            );

            let birthday = match user.legacy_extended_profile {
                Some(lep) => lep.birthdate,
                None => None,
            };

            let pinned = {
                if user.legacy.pinned_tweet_ids_str.is_empty() {
                    None
                } else {
                    user.legacy.pinned_tweet_ids_str[0].parse::<u64>().ok()
                }
            };

            let affiliation = match user.affiliates_highlighted_label {
                Some(affiliate) => Some(UserAffiliation {
                    badge: affiliate.label.badge.url,
                    url: affiliate.label.url.url,
                    description: affiliate.label.description,
                }),
                None => None,
            };

            return Ok(Self {
                id: user.rest_id.parse()?,
                avatar: Avatar {
                    url: user.legacy.profile_image_url_https,
                    banner: user.legacy.profile_banner_url,
                    is_nft: user.has_nft_avatar,
                },
                name: ProfileName {
                    display: user.legacy.screen_name,
                    handle: user.legacy.name,
                },
                profile_stats: ProfileStats {
                    tweets: user.legacy.statuses_count,
                    following: user.legacy.friends_count,
                    followers: user.legacy.followers_count,
                    likes: user.legacy.favourites_count,
                    media_tweets: user.legacy.media_count,
                    verified: user.legacy.verified,
                    blue_verified: user.is_blue_verified,
                },
                additional_info: ProfileAdditionalInfo {
                    affiliation,
                    profession: user.professional,
                    location: as_option!(user.legacy.location, "", "0"),
                    website,
                    joined,
                    birthday,
                },
                bio: user.legacy.description,
                pinned_tweet_id: pinned,
                is_sensitive: user.legacy.possibly_sensitive,
                is_protected: user.legacy.protected,
            });
        }

        Err(UserResultError)
    }

    #[tracing::instrument]
    pub async fn new(scraper: &Scraper, handle: impl AsRef<str>) -> SResult<Self> {
        let req = scraper
            .api_req::<UserRequest>(
                scraper.make_get_req(twitter_request_url_handle(handle.as_ref())),
            )
            .await?;
        // check for errors
        if let Some(why) = req.errors.first() {
            if why.code != TWITTER_IGNORE_ERROR_CODE {
                return Err(TwitterJSONError(why.code, why.message.clone()));
            }
        }

        Self::from_result(scraper, req.data.user.result).await
    }
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Avatar {
    pub url: String,
    pub banner: String,
    pub is_nft: bool,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct ProfileName {
    pub display: String,
    pub handle: String,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct ProfileStats {
    pub tweets: u32,
    pub following: u32,
    pub followers: u32,
    pub likes: u32,
    pub media_tweets: u32,
    pub verified: bool,
    pub blue_verified: bool,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct ProfileAdditionalInfo {
    pub affiliation: Option<UserAffiliation>,
    pub profession: Option<Professional>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub joined: DateTime<Utc>,
    pub birthday: Option<Birthday>,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct UserAffiliation {
    pub badge: String,
    pub url: String,
    pub description: String,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct UserRequest {
    pub errors: Vec<Error>,
    pub data: Data,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct Error {
    pub message: String,
    pub code: i32,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct Data {
    pub user: Usr,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct Usr {
    pub result: TwtResult,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(tag = "__typename")]
pub(crate) enum TwtResult {
    #[serde(rename(deserialize = "User"))]
    User(Box<AvailableUser>),
    #[serde(rename(deserialize = "UserUnavailable"))]
    UserUnavailable(Box<UserUnavailable>),
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct AvailableUser {
    pub id: String,
    pub rest_id: String,

    pub has_nft_avatar: bool,
    pub is_blue_verified: bool,
    pub super_follow_eligible: bool,
    pub is_profile_translatable: bool,

    pub legacy: UserLegacy,
    pub legacy_extended_profile: Option<LegacyExtendedProfile>,

    pub professional: Option<Professional>,
    pub affiliates_highlighted_label: Option<Affiliates>,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct Affiliates {
    pub label: AffiliatesLabel,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct AffiliatesLabel {
    pub badge: Badge,
    pub url: WrapperUrl,
    pub description: String,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct Badge {
    pub url: String,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct WrapperUrl {
    pub url: String,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Professional {
    pub rest_id: String,
    pub professional_type: String,
    pub category: Vec<ProfessionalCategory>,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct ProfessionalCategory {
    pub id: u64,
    pub name: String,
    pub icon_name: String,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct UserLegacy {
    pub created: String,
    pub default_profile: bool,
    pub default_profile_image: bool,
    pub description: String,
    pub favourites_count: u32,
    pub followers_count: u32,
    pub friends_count: u32,
    pub has_custom_timelines: bool,
    pub is_translator: bool,
    pub listed_count: u32,
    pub location: String,
    pub media_count: u32,
    pub name: String,
    pub normal_followers_count: u32,
    pub pinned_tweet_ids_str: Vec<String>,
    pub possibly_sensitive: bool,
    pub profile_banner_url: String,
    pub profile_image_url_https: String,
    pub profile_interstitial_type: String,
    pub protected: bool,
    pub screen_name: String,
    pub statuses_count: u32,
    pub url: String,
    pub verified: bool,
    pub withheld_in_countries: Vec<String>,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct LegacyExtendedProfile {
    pub birthdate: Option<Birthday>,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Birthday {
    day: u8,
    month: u8,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct UserUnavailable {
    pub unavailable_message: UnavailableMessage,
    pub reason: String,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub(crate) struct UnavailableMessage {
    pub rtl: bool,
    pub text: String,
}
