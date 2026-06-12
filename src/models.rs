use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum Component {
    Container(Container),
    TextDisplay(TextDisplay),
    Separator(Separator),
    #[allow(dead_code)]
    MediaGallery(MediaGallery),
    Section(Section),
}

#[derive(Debug, Serialize, Clone)]
pub struct Container {
    #[serde(rename = "type")]
    pub kind: u8, // 17
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<u32>,
    pub components: Vec<Component>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TextDisplay {
    #[serde(rename = "type")]
    pub kind: u8, // 10
    pub content: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Separator {
    #[serde(rename = "type")]
    pub kind: u8, // 14
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divider: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<u8>, // 1 = small, 2 = large
}

#[derive(Debug, Serialize, Clone)]
#[allow(dead_code)]
pub struct MediaGallery {
    #[serde(rename = "type")]
    pub kind: u8, // 12
    pub items: Vec<MediaGalleryItem>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MediaGalleryItem {
    pub media: UnfurledMediaItem,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct UnfurledMediaItem {
    pub url: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Section {
    #[serde(rename = "type")]
    pub kind: u8, // 9
    pub components: Vec<Component>, // text displays
    pub accessory: Thumbnail,
}

#[derive(Debug, Serialize, Clone)]
pub struct Thumbnail {
    #[serde(rename = "type")]
    pub kind: u8, // 11
    pub media: UnfurledMediaItem,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Container {
    pub fn new(accent_color: Option<u32>, components: Vec<Component>) -> Self {
        Self {
            kind: 17,
            accent_color,
            components,
        }
    }
}

impl TextDisplay {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            kind: 10,
            content: content.into(),
        }
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
    #[allow(dead_code)]
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
                media: UnfurledMediaItem {
                    url: thumb_url.into(),
                },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_serialization() {
        let container = Container::new(
            Some(0x1E90FF),
            vec![Component::TextDisplay(TextDisplay::new("Hello"))],
        );
        let json = serde_json::to_value(&container).unwrap();
        assert_eq!(json["type"], 17);
        assert_eq!(json["accent_color"], 0x1E90FF);
        assert!(json["components"].is_array());
    }

    #[test]
    fn test_text_display_serialization() {
        let td = TextDisplay::new("Test content");
        let json = serde_json::to_value(&td).unwrap();
        assert_eq!(json["type"], 10);
        assert_eq!(json["content"], "Test content");
    }

    #[test]
    fn test_separator_serialization() {
        let sep = Separator::new(true, false);
        let json = serde_json::to_value(&sep).unwrap();
        assert_eq!(json["type"], 14);
        assert_eq!(json["divider"], true);
        assert_eq!(json["spacing"], 1);

        let large_sep = Separator::new(false, true);
        let json = serde_json::to_value(&large_sep).unwrap();
        assert_eq!(json["spacing"], 2);
    }

    #[test]
    fn test_section_with_thumbnail_serialization() {
        let section = Section::with_thumbnail(
            vec![Component::TextDisplay(TextDisplay::new("Header"))],
            "https://example.com/image.png",
        );
        let json = serde_json::to_value(&section).unwrap();
        assert_eq!(json["type"], 9);
        assert_eq!(json["accessory"]["type"], 11);
        assert_eq!(
            json["accessory"]["media"]["url"],
            "https://example.com/image.png"
        );
    }

    #[test]
    fn test_media_gallery_serialization() {
        let gallery = MediaGallery::single("https://example.com/img.png");
        let json = serde_json::to_value(&gallery).unwrap();
        assert_eq!(json["type"], 12);
        assert_eq!(json["items"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_webhook_v2_serialization() {
        let payload = DiscordWebhookV2 {
            username: "Rustico".to_string(),
            avatar_url: None,
            flags: COMPONENTS_V2_FLAG,
            components: vec![Component::Container(Container::new(
                Some(0xFF0000),
                vec![Component::TextDisplay(TextDisplay::new("Test"))],
            ))],
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["username"], "Rustico");
        assert_eq!(json["flags"], 32768);
        assert!(json["avatar_url"].is_null());
    }

    #[test]
    fn test_anilist_response_deserialization() {
        let json_str = r#"{
            "data": {
                "Page": {
                    "airingSchedules": [
                        {
                            "id": 12345,
                            "episode": 5,
                            "airingAt": 1700000000,
                            "media": {
                                "title": { "romaji": "Test Anime", "english": "Test Anime EN" },
                                "siteUrl": "https://anilist.co/anime/12345",
                                "coverImage": { "large": "https://example.com/cover.jpg" },
                                "description": "A test anime description",
                                "averageScore": 85,
                                "studios": { "nodes": [{ "name": "Studio Test" }] }
                            }
                        }
                    ]
                }
            }
        }"#;

        let response: AniListResponse = serde_json::from_str(json_str).unwrap();
        assert_eq!(response.data.page.airing_schedules.len(), 1);

        let schedule = &response.data.page.airing_schedules[0];
        assert_eq!(schedule.id, 12345);
        assert_eq!(schedule.episode, 5);
        assert_eq!(
            schedule.media.title.english.as_deref(),
            Some("Test Anime EN")
        );
        assert_eq!(schedule.media.average_score, Some(85));
    }
}
