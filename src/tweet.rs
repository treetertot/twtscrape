use crate::{
    error::{
        SResult,
        TwtScrapeError::{BadJSONSchema, TwitterBadRestId, TwitterBadTimeParse},
    },
    scrape::Scraper,
    user::{Error, TwtResult, User},
    TwitterIdType,
};
use ahash::{HashSet, HashSetExt};
use chrono::{DateTime, Utc};
use rkyv::Archive;
#[cfg(feature = "scrape")]
use scraper::{Html, Selector};
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::{
    collections::{HashMap, VecDeque},
    fmt::{self, Display, Write},
};

#[cfg(feature = "scrape")]
static LINK_SELECTOR: Selector = Selector::parse("a").unwrap();

#[cfg(feature = "scrape")]
pub(crate) const TWEET_CREATED_DATETIME: &str = "%a %b %d %T %z %Y";
pub fn twitter_request_url_thread(
    handle: impl AsRef<str> + Display,
    cursor: Option<impl AsRef<str> + Display>,
) -> String {
    match cursor {
        Some(crsr) => {
            let crsr = urlencoding::encode(crsr.as_ref());
            format!("https://twitter.com/i/api/graphql/BoHLKeBvibdYDiJON1oqTg/TweetDetail?variables=%7B%22focalTweetId%22%3A%22{handle}%22%2C%22cursor%22%3A%22{crsr}%22%2C%22referrer%22%3A%22messages%22%2C%22with_rux_injections%22%3Afalse%2C%22includePromotedContent%22%3Afalse%2C%22withCommunity%22%3Atrue%2C%22withQuickPromoteEligibilityTweetFields%22%3Atrue%2C%22withBirdwatchNotes%22%3Afalse%2C%22withSuperFollowsUserFields%22%3Atrue%2C%22withDownvotePerspective%22%3Afalse%2C%22withReactionsMetadata%22%3Afalse%2C%22withReactionsPerspective%22%3Afalse%2C%22withSuperFollowsTweetFields%22%3Atrue%2C%22withVoice%22%3Atrue%2C%22withV2Timeline%22%3Atrue%7D&features=%7B%22responsive_web_twitter_blue_verified_badge_is_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%2C%22unified_cards_ad_metadata_container_dynamic_card_content_query_enabled%22%3Atrue%2C%22tweetypie_unmention_optimization_enabled%22%3Atrue%2C%22responsive_web_uc_gql_enabled%22%3Atrue%2C%22vibe_api_enabled%22%3Atrue%2C%22responsive_web_edit_tweet_api_enabled%22%3Atrue%2C%22graphql_is_translatable_rweb_tweet_is_translatable_enabled%22%3Atrue%2C%22standardized_nudges_misinfo%22%3Atrue%2C%22tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled%22%3Afalse%2C%22interactive_text_enabled%22%3Atrue%2C%22responsive_web_text_conversations_enabled%22%3Afalse%2C%22responsive_web_enhance_cards_enabled%22%3Atrue%7D")
        }
        None => {
            format!("https://twitter.com/i/api/graphql/BoHLKeBvibdYDiJON1oqTg/TweetDetail?variables=%7B%22focalTweetId%22%3A%22{handle}%22%2C%22with_rux_injections%22%3Afalse%2C%22includePromotedContent%22%3Afalse%2C%22withCommunity%22%3Atrue%2C%22withQuickPromoteEligibilityTweetFields%22%3Atrue%2C%22withBirdwatchNotes%22%3Afalse%2C%22withSuperFollowsUserFields%22%3Atrue%2C%22withDownvotePerspective%22%3Afalse%2C%22withReactionsMetadata%22%3Afalse%2C%22withReactionsPerspective%22%3Afalse%2C%22withSuperFollowsTweetFields%22%3Atrue%2C%22withVoice%22%3Atrue%2C%22withV2Timeline%22%3Atrue%7D&features=%7B%22responsive_web_twitter_blue_verified_badge_is_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_timeline_navigation_enabled%22%3Atrue%2C%22unified_cards_ad_metadata_container_dynamic_card_content_query_enabled%22%3Atrue%2C%22tweetypie_unmention_optimization_enabled%22%3Atrue%2C%22responsive_web_uc_gql_enabled%22%3Atrue%2C%22vibe_api_enabled%22%3Atrue%2C%22responsive_web_edit_tweet_api_enabled%22%3Atrue%2C%22graphql_is_translatable_rweb_tweet_is_translatable_enabled%22%3Atrue%2C%22standardized_nudges_misinfo%22%3Atrue%2C%22tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled%22%3Afalse%2C%22interactive_text_enabled%22%3Atrue%2C%22responsive_web_text_conversations_enabled%22%3Afalse%2C%22responsive_web_enhance_cards_enabled%22%3Atrue%7D")
        }
    }
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct Tweet {
    pub id: u64,
    pub conversation_id: u64,
    pub tweet_type: TweetType,
}

#[cfg(feature = "scrape")]
impl Tweet {
    #[tracing::instrument]
    pub async fn parse_thread(
        scraper: &Scraper,
        id: impl TwitterIdType + Display,
    ) -> SResult<(Vec<Tweet>, Vec<User>)> {
        let base_request = scraper
            .api_req::<TweetRequest>(scraper.make_get_req(twitter_request_url_thread(&id, None)))
            .await?;

        base_request.json_request_filter_errors()?;

        let mut requests = Vec::with_capacity(5);

        // lets first get the conversation id
        let conversation_id = base_request
            .first_tweet()
            .ok_or(BadJSONSchema("TweetRequest", "No First Tweet".to_string()))?
            .legacy
            .conversation_id_str
            .parse::<u64>()
            .map_err(|why| TwitterBadRestId("Conversation RestID", why.to_string()))?;

        if let Some(cursor) = base_request.filter_cursor(FilterCursorTweetRequest::Top) {
            requests.append(
                &mut TweetRequest::scroll(
                    scraper,
                    &id,
                    cursor.to_string(),
                    FilterCursorTweetRequest::Top,
                )
                .await?,
            )
        }

        requests.push(base_request);

        if let Some(cursor) = base_request.filter_cursor(FilterCursorTweetRequest::Bottom) {
            requests.append(
                &mut TweetRequest::scroll(
                    scraper,
                    &id,
                    cursor.to_string(),
                    FilterCursorTweetRequest::Bottom,
                )
                .await?
                .into(),
            )
        }

        let mut tweets = Vec::with_capacity(requests.len() * 10);
        let mut users = Vec::with_capacity(requests.len() * 10);
        let mut already_parsed_users = HashSet::with_capacity(10);

        for request in requests {
            for inst in request
                .data
                .threaded_conversation_with_injections_v2
                .instructions
            {
                if let Instruction::TimelineAddEntries(add) = inst {
                    for entry in add.entries {
                        match entry {
                            Entry::Tweet(twt) => {
                                let mut tweet =
                                    Tweet::new_from_entry(&twt.item_content.tweet_results)?;
                                tweet.conversation_id = conversation_id;
                                tweets.push(tweet);

                                if let TweetResults::Ok(trr) = twt.item_content.tweet_results {
                                    if let TwtResult::User(user) = &trr.core.user_results.result {
                                        if already_parsed_users.contains(&user.id) {
                                            continue;
                                        }
                                    }
                                    let user =
                                        User::from_result(scraper, trr.core.user_results.result)
                                            .await?;
                                    already_parsed_users.insert(user.id.to_string());
                                    users.push(user)
                                }
                            }
                            Entry::ConversationThread(ct) => {
                                for thread in ct.content.items {
                                    let mut tweet = Tweet::new_from_entry(
                                        &thread.item.item_content.tweet_results,
                                    )?;
                                    tweet.conversation_id = conversation_id;
                                    tweets.push(tweet);

                                    if let TweetResults::Ok(trr) =
                                        thread.item.item_content.tweet_results
                                    {
                                        if let TwtResult::User(usr) = &trr.core.user_results.result
                                        {
                                            if already_parsed_users.contains(&usr.id) {
                                                continue;
                                            }
                                        }

                                        let user = User::from_result(
                                            scraper,
                                            trr.core.user_results.result,
                                        )
                                        .await?;
                                        already_parsed_users.insert(user.id.to_string());
                                        users.push(user)
                                    }
                                }
                            }
                            Entry::Cursor(_) => continue,
                        }
                    }
                }
            }
        }

        tweets.shrink_to_fit();
        users.shrink_to_fit();

        Ok((tweets, users))
    }

    /// HEY FUTURE ASS MF!!!
    /// MAKE SURE YOU SET THE `conversation_id` AFTERWARDS!!!!!
    pub(crate) fn new_from_entry(t: &TweetResults) -> SResult<Self> {
        match t {
            TweetResults::Ok(trr) => {
                if trr.rest_id.is_empty() || trr.rest_id == "0" {
                    return Err(TwitterBadRestId("Tweet RestID", trr.rest_id.clone()));
                }

                let id = trr
                    .rest_id
                    .parse::<u64>()
                    .map_err(|why| TwitterBadRestId("Tweet RestID", why.to_string()))?;

                if trr.legacy.conversation_id_str.is_empty()
                    || trr.legacy.conversation_id_str == "0"
                {
                    return Err(TwitterBadRestId(
                        "Conversation RestID",
                        trr.legacy.conversation_id_str.clone(),
                    ));
                }

                let conversation_id = trr
                    .legacy
                    .conversation_id_str
                    .parse::<u64>()
                    .map_err(|why| TwitterBadRestId("Conversation RestID", why.to_string()))?;

                let created = DateTime::<Utc>::from(
                    DateTime::parse_from_str(&trr.legacy.created_at, TWEET_CREATED_DATETIME)
                        .map_err(|why| TwitterBadTimeParse(why.to_string()))?,
                );

                let edit_ids = trr
                    .edit_control
                    .edit_tweet_ids
                    .into_iter()
                    .map(|id| {
                        id.parse::<u64>()
                            .map_err(|why| TwitterBadRestId("Tweet RestID", why.to_string()))
                    })
                    .collect::<SResult<Vec<u64>>>()?;

                let media = trr
                    .legacy
                    .extended_entities
                    .media
                    .into_iter()
                    .map(|x| {
                        let media_id = x
                            .id_str
                            .parse::<u64>()
                            .map_err(|why| TwitterBadRestId("Tweet RestID", why.to_string()))?;
                        Ok(Media {
                            id: media_id,
                            media_url_https: x.media_url_https,
                            r#type: x.r#type,
                            url: x.url,
                            ext_alt_text: x.ext_alt_text,
                            views: x.media_stats.map(|x| x.view_count),
                        })
                    })
                    .collect::<SResult<Vec<Media>>>()?;

                let urls = trr
                    .legacy
                    .entities
                    .urls
                    .into_iter()
                    .map(|url| url.expanded_url)
                    .collect::<Vec<String>>();

                let hashtags = trr
                    .legacy
                    .entities
                    .hashtags
                    .into_iter()
                    .map(|ht| ht.text)
                    .collect::<Vec<String>>();

                let card = trr.card.map(|tcd| Card {
                    id: tcd.rest_id,
                    url: tcd.legacy.url,
                    name: tcd.legacy.name,
                    values: tcd
                        .legacy
                        .binding_values
                        .into_iter()
                        .map(|bv| (bv.key, bv.value))
                        .collect::<HashMap<String, CardValue, ahash::RandomState>>(),
                });

                let display_text_range = {
                    if trr.legacy.display_text_range.len() != 2 {
                        (0, trr.legacy.full_text.len() as u16)
                    } else {
                        (
                            trr.legacy.display_text_range[0],
                            trr.legacy.display_text_range[1],
                        )
                    }
                };

                let replying_to = {
                    match &trr.legacy.in_reply_to_status_id_str {
                        Some(idstr) => {
                            if idstr.is_empty() || idstr == "0" {
                                None
                            } else {
                                Some(idstr.parse::<u64>().map_err(|why| {
                                    TwitterBadRestId("Reply Tweet ID", why.to_string())
                                })?)
                            }
                        }
                        None => None,
                    }
                };

                let quoting = {
                    if !trr.legacy.is_quote_status {
                        None
                    } else {
                        match &trr.legacy.quoted_status_id_str {
                            Some(qrtid) => {
                                if qrtid.is_empty() || qrtid == "0" {
                                    None
                                } else {
                                    Some(qrtid.parse::<u64>().map_err(|why| {
                                        TwitterBadRestId("Quote Tweet ID", why.to_string())
                                    })?)
                                }
                            }
                            None => None,
                        }
                    }
                };

                let source = {
                    let frag = Html::parse_fragment(&trr.legacy.source);
                    frag.select(&LINK_SELECTOR)
                        .next()
                        .map(|elem| elem.inner_html())
                        .unwrap_or_default()
                };

                Ok(Tweet {
                    id,
                    conversation_id,
                    tweet_type: TweetType::Tweet(Box::new(TweetData {
                        created,
                        edit_ids,
                        entry: Entries {
                            media,
                            mentions: trr.legacy.entities.user_mentions.clone(),
                            urls,
                            hashtags,
                        },
                        card,
                        text: trr.legacy.full_text.clone(),
                        source,
                        display_text_range,
                        stats: TweetStats {
                            quote_tweets: trr.legacy.quote_count,
                            retweets: trr.legacy.retweet_count,
                            likes: trr.legacy.favourite_count,
                            replies: trr.legacy.reply_count,
                        },
                        reply_info: ReplyInfo {
                            replying_to,
                            quoting,
                        },
                        moderated: false,
                        conversation_control: ConversationControl::None,
                        vibe: trr.vibe.map(|v| Vibe {
                            discovery_query_text: v.discovery_query_text,
                            text: v.text,
                            img_description: v.img_description,
                        }),
                    })),
                })
            }
            TweetResults::Tombstone(tomb) => Ok(Tweet {
                id: entry_id,
                conversation_id: 0,
                tweet_type: TweetType::Tombstone(tomb.tombstone.text.text.clone()),
            }),
        }
    }
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum TweetType {
    Tombstone(String),
    Tweet(Box<TweetData>),
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct TweetData {
    pub created: DateTime<Utc>,
    pub edit_ids: Vec<u64>,
    pub entry: Entries,
    pub card: Option<Card>,
    pub text: String,
    pub source: String,
    pub display_text_range: (u16, u16),
    pub stats: TweetStats,
    pub reply_info: ReplyInfo,
    pub moderated: bool,
    pub conversation_control: ConversationControl,
    pub vibe: Option<Vibe>,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct Vibe {
    pub discovery_query_text: String,
    pub text: String,
    pub img_description: String,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct TweetStats {
    pub quote_tweets: u32,
    pub retweets: u32,
    pub likes: u32,
    pub replies: u32,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum ConversationControl {
    None,
    FollowsOnly,
    MentionedOnly,
    Other(String),
}

impl From<Option<TweetConversationControl>> for ConversationControl {
    fn from(value: Option<TweetConversationControl>) -> Self {
        if let Some(v) = value {
            return match v.policy.as_str() {
                "ByInvitation" => ConversationControl::MentionedOnly,
                "Community" => ConversationControl::FollowsOnly,
                o => ConversationControl::Other(o.to_string()),
            };
        }
        ConversationControl::None
    }
}
#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct ReplyInfo {
    pub replying_to: Option<u64>,
    pub quoting: Option<u64>,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct Card {
    pub id: String,
    pub url: String,
    pub name: String,
    pub values: HashMap<String, CardValue, ahash::RandomState>,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct Entries {
    pub media: Vec<Media>,
    pub mentions: Vec<TweetUserMentions>,
    pub urls: Vec<String>,
    pub hashtags: Vec<String>,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct Media {
    pub id: u64,
    pub media_url_https: String,
    pub r#type: String,
    pub url: String,
    pub ext_alt_text: Option<String>,
    pub views: Option<u32>,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetRequest {
    pub(crate) errors: Vec<Error>,
    pub(crate) data: Data,
}

#[cfg(feature = "scrape")]
impl TweetRequest {
    pub(crate) fn first_tweet(&self) -> Option<&TweetResultResult> {
        for inst in self
            .data
            .threaded_conversation_with_injections_v2
            .instructions
        {
            if let Instruction::TimelineAddEntries(add) = inst {
                for entry in &add.entries {
                    if let Entry::Tweet(te) = entry {
                        if let TweetResults::Ok(trr) = &te.item_content.tweet_results {
                            Some(trr)
                        }
                    }
                }
            }
        }

        None
    }

    pub(crate) fn filter_cursor(&self, filter: FilterCursorTweetRequest) -> Option<&str> {
        for inst in &self
            .data
            .threaded_conversation_with_injections_v2
            .instructions
        {
            inst.filter_cursor(filter)
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
                .api_req::<TweetRequest>(
                    scraper.make_get_req(twitter_request_url_thread(&id, Some(&cursor_counter))),
                )
                .await?;

            scrolled_up_request.json_request_filter_errors()?;

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

crate::impl_json_filter!(TweetRequest);

#[derive(Clone, Copy)]
pub enum FilterCursorTweetRequest {
    Top,
    Bottom,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct Data {
    pub(crate) threaded_conversation_with_injections_v2: ThreadedConversation,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct ThreadedConversation {
    pub instructions: Vec<Instruction>,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
#[serde(tag = "type")]
pub(crate) enum Instruction {
    TimelineAddEntries(TimelineAddEntries),
    TimelineTerminateTimeline(TimelineTerminateTimeline),
}

impl Instruction {
    pub(crate) fn filter_cursor(&self, cursor: FilterCursorTweetRequest) -> Option<&str> {
        if let Instruction::TimelineAddEntries(add) = self {
            for entry in &add.entries {
                if let Entry::Cursor(c) = entry {
                    match cursor {
                        FilterCursorTweetRequest::Top => {
                            if c.entry_id.starts_with("cursor-top") {
                                return Some(c.content.item_content.value.as_str());
                            }
                        }
                        FilterCursorTweetRequest::Bottom => {
                            if c.entry_id.starts_with("cursor-bottom")
                                || c.entry_id.starts_with("cursor-showmorethreads")
                            {
                                return Some(c.content.item_content.value.as_str());
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TimelineAddEntries {
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) enum Entry {
    Tweet(TweetEnt),
    ConversationThread(ConversationThread),
    Cursor(Cursor),
}

impl<'de> Deserialize<'de> for Entry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            EntryId,
            TypeName,
            SortId,
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
                        formatter.write_str("entry type sort content")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "entryId" => Ok(Field::EntryId),
                            "__typeName" => Ok(Field::TypeName),
                            "sortId" => Ok(Field::SortId),
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
                let mut __typename: Option<String> = None;
                let mut sort_id: Option<String> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::EntryId => {
                            entry_id = Some(map.next_value()?);
                        }
                        Field::TypeName => {
                            __typename = Some(map.next_value()?);
                        }
                        Field::SortId => {
                            sort_id = Some(map.next_value()?);
                        }
                        Field::Content => {
                            if let Some(entry) = &entry_id {
                                if entry.starts_with("tweet-") {
                                    Ok(Entry::Tweet(map.next_value()?))
                                } else if entry.starts_with("conversationthread-") {
                                    Ok(Entry::ConversationThread(map.next_value()?))
                                } else if entry.starts_with("cursor-") {
                                    Ok(Entry::Cursor(map.next_value()?))
                                } else {
                                    Err(de::Error::unknown_variant(
                                        entry,
                                        &["tweet", "conversationthread", "cursor"],
                                    ))
                                }
                            }
                            Err(de::Error::unknown_variant(
                                "None",
                                &["tweet", "conversationthread", "cursor"],
                            ))
                        }
                    }
                }
                Err(de::Error::missing_field("content"))
            }
        }

        const VARIANTS: &[&str] = &["Tweet", "ConversationThread", "Cursor"];
        deserializer.deserialize_enum("Entry", VARIANTS, EntryVisitor)
    }
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetEnt {
    #[serde(rename = "itemContent")]
    pub item_content: TweetItemContent,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetItemContent {
    #[serde(rename = "itemType")]
    pub item_type: String,
    pub __typename: String,
    pub tweet_results: TweetResults,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct ConversationThread {
    pub content: ConversationThreadContent,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct ConversationThreadContent {
    #[serde(rename = "entryType")]
    pub entry_type: String,
    pub __typename: String,
    #[serde(rename = "itemContent")]
    pub items: Vec<ConversationThreadItems>,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct ConversationThreadItems {
    #[serde(rename = "entryId")]
    pub entry_id: String,
    pub item: ConversationThreadItem,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct ConversationThreadItem {
    #[serde(rename = "itemContent")]
    pub item_content: ConversationThreadItemContent,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct ConversationThreadItemContent {
    #[serde(rename = "itemType")]
    pub item_type: String,
    pub __typename: String,
    pub tweet_results: TweetResults,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct EditControl {
    pub initial_tweet_id: Option<String>,
    pub edit_tweet_ids: Vec<String>,
    pub editable_until_msecs: String,
    pub is_edit_eligible: bool,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) enum TweetResults {
    Ok(TweetResultResult),
    Tombstone(TweetTombstone),
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetTombstone {
    pub __typename: String,
    pub tombstone: TombstoneStone,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TombstoneStone {
    pub __typename: String,
    pub text: TombstoneText,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TombstoneText {
    pub rtl: bool,
    pub text: String,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetResultResult {
    pub __typename: String,
    pub rest_id: String,
    pub core: TwtRsltCore,
    pub card: Option<TwtCard>,
    pub vibe: Option<TwtVibe>,
    pub edit_control: EditControl,
    pub legacy: TweetLegacy,
    #[serde(rename = "hasModeratedReplies")]
    pub has_moderated_replies: bool,
    pub is_translatable: bool,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TwtVibe {
    #[serde(rename = "discovery_query_text")]
    pub discovery_query_text: String,
    pub text: String,
    #[serde(rename = "imgDescription")]
    pub img_description: String,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TwtCard {
    pub rest_id: String,
    pub legacy: TwtCardLegacy,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TwtCardLegacy {
    pub binding_values: Vec<TwtCardBindV>,
    pub name: String,
    pub url: String,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TwtCardBindV {
    pub key: String,
    pub value: CardValue,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct CardValue {
    pub string_value: String,
    pub r#type: String,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetLegacy {
    pub id_str: String,
    pub created_at: String,
    pub conversation_id_str: String,
    pub entities: TweetEntry,
    pub extended_entities: TweetExtEntry,
    pub favourite_count: u32,
    pub is_quote_status: bool,
    pub possibly_sensitive: bool,
    pub quote_count: u32,
    pub reply_count: u32,
    pub retweet_count: u32,
    pub source: String,
    pub full_text: String,
    pub user_id_str: String,
    pub display_text_range: Vec<u16>,
    pub conversation_control: Option<TweetConversationControl>,
    pub in_reply_to_status_id_str: Option<String>,
    pub in_reply_to_user_id_str: Option<String>,
    pub quoted_status_id_str: Option<String>,
    pub self_thread: TweetSelfThread,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetSelfThread {
    pub id_str: String,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetConversationControl {
    pub policy: String,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetExtEntry {
    pub media: Vec<TweetEntryMedia>,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetEntry {
    pub media: Vec<TweetEntryMedia>,
    pub user_mentions: Vec<TweetUserMentions>,
    pub urls: Vec<TweetEntryUrls>,
    pub hashtags: Vec<TweetEntryHashtags>,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetEntryHashtags {
    pub text: String,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetEntryMedia {
    pub id_str: String,
    pub media_url_https: String,
    pub r#type: String,
    pub url: String,
    pub ext_alt_text: Option<String>,
    #[serde(rename = "mediaStats")]
    pub media_stats: Option<TweetMediaStats>,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetMediaStats {
    pub view_count: u32,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TweetEntryUrls {
    pub display_url: String,
    pub expanded_url: String,
    pub url: String,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct TweetUserMentions {
    pub id_str: String,
    pub name: String,
    pub screen_name: String,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TwtRsltCore {
    pub user_results: UserResults,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct UserResults {
    pub result: TwtResult,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct Cursor {
    #[serde(renamee = "entryId")]
    pub entry_id: String,
    pub content: CursorContent,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct CursorContent {
    #[serde(rename = "entryType")]
    pub entry_type: String,
    pub __typename: String,
    #[serde(rename = "itemContent")]
    pub item_content: CursorItemContent,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct CursorItemContent {
    #[serde(rename = "itemType")]
    pub item_type: String,
    pub __typename: String,
    pub value: String,
    #[serde(rename = "cursorType")]
    pub cursor_type: String,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub(crate) struct TimelineTerminateTimeline {
    pub direction: String,
}
