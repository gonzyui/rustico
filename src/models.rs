use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Default, Debug)]
pub struct AppState {
    pub seen_ann: HashSet<String>,
    pub seen_anilist: HashSet<i64>,
    pub initialized: bool,
}

// =====================================================
//                DISCORD COMPONENTS V2
// =====================================================

pub const COMPONENTS_V2_FLAG: u32 = 32768;

#[derive(Debug, Serialize)]
pub struct DiscordWebhookV2 {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub flags: u32,
    pub components: Vec<Component>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Component {
    Container(Container),
    TextDisplay(TextDisplay),
    Separator(Separator),
    MediaGallery(MediaGallery),
    Section(Section),
}

#[derive(Debug, Serialize)]
pub struct Container {
    #[serde(rename = "type")]
    pub kind: u8, // 17
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<u32>,
    pub components: Vec<Component>,
}

#[derive(Debug, Serialize)]
pub struct TextDisplay {
    #[serde(rename = "type")]
    pub kind: u8, // 10
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct Separator {
    #[serde(rename = "type")]
    pub kind: u8, // 14
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divider: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<u8>, // 1 = small, 2 = large
}

#[derive(Debug, Serialize)]
pub struct MediaGallery {
    #[serde(rename = "type")]
    pub kind: u8, // 12
    pub items: Vec<MediaGalleryItem>,
}

#[derive(Debug, Serialize)]
pub struct MediaGalleryItem {
    pub media: UnfurledMediaItem,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UnfurledMediaItem {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct Section {
    #[serde(rename = "type")]
    pub kind: u8, // 9
    pub components: Vec<Component>, // text displays
    pub accessory: Thumbnail,
}

#[derive(Debug, Serialize)]
pub struct Thumbnail {
    #[serde(rename = "type")]
    pub kind: u8, // 11
    pub media: UnfurledMediaItem,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Container {
    pub fn new(accent_color: Option<u32>, components: Vec<Component>) -> Self {
        Self { kind: 17, accent_color, components }
    }
}

impl TextDisplay {
    pub fn new(content: impl Into<String>) -> Self {
        Self { kind: 10, content: content.into() }
    }
}

impl Separator {
    pub fn new(divider: bool, large: bool) -> Self {
        Self {
            kind: 14,
            divider: Some(divider),
            spacing: Some(if large { 2 } else { 1 }),
        }
    }
}

impl MediaGallery {
    pub fn single(url: impl Into<String>) -> Self {
        Self {
            kind: 12,
            items: vec![MediaGalleryItem {
                media: UnfurledMediaItem { url: url.into() },
                description: None,
            }],
        }
    }
}

impl Section {
    pub fn with_thumbnail(text_components: Vec<Component>, thumb_url: impl Into<String>) -> Self {
        Self {
            kind: 9,
            components: text_components,
            accessory: Thumbnail {
                kind: 11,
                media: UnfurledMediaItem { url: thumb_url.into() },
                description: None,
            },
        }
    }
}

// =====================================================
//                       ANILIST
// =====================================================

#[derive(Debug, Deserialize)]
pub struct AniListResponse {
    pub data: AniListData,
}

#[derive(Debug, Deserialize)]
pub struct AniListData {
    #[serde(rename = "Page")]
    pub page: AniListPage,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AniListPage {
    pub airing_schedules: Vec<AiringSchedule>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiringSchedule {
    pub id: i64,
    pub episode: i32,
    pub airing_at: i64,
    pub media: AniListMedia,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AniListMedia {
    pub title: AniListTitle,
    pub site_url: String,
    pub cover_image: Option<AniListCover>,
    pub description: Option<String>,
    pub average_score: Option<i32>,
    pub studios: Option<AniListStudios>,
}

#[derive(Debug, Deserialize)]
pub struct AniListStudios {
    pub nodes: Vec<AniListStudio>,
}

#[derive(Debug, Deserialize)]
pub struct AniListStudio {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AniListTitle {
    pub romaji: Option<String>,
    pub english: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AniListCover {
    pub large: Option<String>,
}
