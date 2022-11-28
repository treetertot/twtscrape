use rkyv::Archive;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct GlobalTimeline {
    #[serde(alias = "globalObjects")]
    global_objects: GlobalObjects,
}

#[derive(Debug, Deserialize)]
pub struct GlobalObjects {
    tweets: HashMap<String, Tweet>,
}

#[derive(Debug, Deserialize)]
pub struct Tweet {
    pub conversation_id_str: String,
    pub created_at: String,
    pub favorite_count: i32,
    pub full_text: String,
    pub entities: Entities,
    pub extended_entities: ExtendedEntities,
    pub in_reply_to_status_id_str: String,
    pub place: Place,
}

#[derive(Debug, Deserialize)]
pub struct Entities {
    pub hashtags: Vec<Hashtag>,
    pub media: Vec<Media>,
    pub urls: Vec<Url>,
}

#[derive(Debug, Deserialize)]
pub struct Hashtag {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct Media {
    pub media_url_https: String,
    #[serde(alias = "type")]
    pub media_type: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Url {
    pub expanded_url: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct ExtendedEntities {
    pub media: ExtendedMedia,
}

#[derive(Debug, Deserialize)]
pub struct ExtendedMedia {
    pub id_str: String,
    pub media_url_https: String,
    pub ext_sensitive_media_warning: ExtSensitiveMediaWarning,
    #[serde(alias = "type")]
    pub media_type: String,
    pub url: String,
    pub video_info: VideoInfo,
}

#[derive(Debug, Deserialize)]
pub struct ExtSensitiveMediaWarning {
    adult_content: bool,
    graphic_violence: bool,
    other: bool,
}

#[derive(Debug, Deserialize)]
pub struct VideoInfo {
    variants: Vec<VideoVariant>,
}

#[derive(Debug, Deserialize)]
pub struct VideoVariant {
    bitrate: i32,
    url: String,
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
pub struct Place {
    pub id: String,
    pub place_type: String,
    pub name: String,
    pub full_name: String,
    pub country_code: String,
    pub country: String,
    pub bounding_box: BoundingBox,
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
pub struct BoundingBox {
    #[serde(alias = "type")]
    pub box_type: String,
    pub coordinates: Vec<Vec<Vec<f64>>>,
}
