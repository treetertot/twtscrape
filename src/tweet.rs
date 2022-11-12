use crate::user::{Error, TwtResult};
use serde::{Deserialize, Serialize};

pub(crate) struct TweetRequest {
    pub(crate) errors: Vec<Error>,
}

pub(crate) struct ThreadedConversation {}

pub(crate) enum Instruction {
    TimelineAddEntries(),
    TimelineTerminateTimeline(),
}

pub(crate) struct TimelineAddEntries {}

pub(crate) enum Entry {
    Tweet,
    ConversationThread,
    Cursor(Cursor),
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ConversationThread {
    #[serde(rename = "entryId")]
    pub entry_id: String,
    #[serde(rename = "sortIndex")]
    pub sort_index: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ConversationThreadContent {
    #[serde(rename = "entryType")]
    entry_type: String,
    __typename: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ConversationThreadItems {
    #[serde(rename = "entryId")]
    pub entry_id: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ConversationThreadItem {
    #[serde(rename = "entryId")]
    pub entry_id: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ConversationThreadItemContent {
    #[serde(rename = "itemType")]
    pub item_type: String,
    pub __typename: String,
    pub tweet_results: TweetResults,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct EditControl {
    pub initial_tweet_id: Option<String>,
    pub edit_tweet_ids: Vec<String>,
    pub editable_until_msecs: String,
    pub is_edit_eligible: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TweetResults {
    pub result: TweetResultResult,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TweetResultResult {
    pub __typename: String,
    pub rest_id: String,
    pub core: TwtRsltCore,
    pub edit_control: EditControl,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TweetLegacy {
    pub created_at: String,
    pub conversation_id_str: String,
    pub entities: TweetEntry,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TweetEntry {
    pub media: Vec<TweetEntryMedia>,
    pub user_mentions: Vec<TweetEntryUserMentions>,
    pub urls: Vec<TweetEntryUrls>,
    pub hashtags: Vec<TweetEntryHashtags>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TweetEntryHashtags {
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TweetEntryMedia {
    pub id_str: String,
    pub media_url_https: String,
    pub r#type: String,
    pub url: String,
    pub ext_alt_text: Option<String>,
    #[serde(rename = "mediaStats")]
    pub media_stats: Option<TweetMediaStats>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TweetMediaStats {
    pub view_count: u32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TweetEntryUrls {
    pub expanded_url: String,
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TweetEntryUserMentions {
    pub id_str: String,
    pub name: String,
    pub screen_name: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TwtRsltCore {
    pub user_results: UserResults,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct UserResults {
    pub result: TwtResult,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Cursor {
    #[serde(rename = "entryId")]
    pub entry_id: String,
    pub content: CursorContent,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CursorContent {
    #[serde(rename = "entryType")]
    entry_type: String,
    __typename: String,
    #[serde(rename = "itemContent")]
    item_content: CursorItemContent,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CursorItemContent {
    #[serde(rename = "itemType")]
    item_type: String,
    __typename: String,
    value: String,
    #[serde(rename = "cursorType")]
    cursor_type: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TimelineTerminateTimeline {
    direction: String,
}
