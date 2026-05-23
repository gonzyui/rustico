use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Default, Debug)]
pub struct AppState {
    pub seen_ann: HashSet<String>,
    pub seen_anilist: HashSet<i64>,
    pub initialized: bool,
}

#[derive(Debug, Serialize)]
pub struct DiscordWebhook {
    pub username: String,
    pub avatar_url: String,
    pub embeds: Vec<DiscordEmbed>,
}

#[derive(Debug, Serialize)]
pub struct DiscordEmbed {
    pub title: String,
    pub description: String,
    pub url: String,
    pub color: u32,
    pub footer: DiscordFooter,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<DiscordImage>,
}

#[derive(Debug, Serialize)]
pub struct DiscordImage {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct DiscordFooter {
    pub text: String,
}

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
pub struct AniListPage {
    #[serde(rename = "airingSchedules")]
    pub airing_schedules: Vec<AiringSchedule>,
}

#[derive(Debug, Deserialize)]
pub struct AiringSchedule {
    pub id: i64,
    pub episode: i32,
    #[serde(rename = "airingAt")]
    pub airing_at: i64,
    pub media: AniListMedia,
}

#[derive(Debug, Deserialize)]
pub struct AniListMedia {
    pub title: AniListTitle,
    #[serde(rename = "siteUrl")]
    pub site_url: String,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<AniListCover>,
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
