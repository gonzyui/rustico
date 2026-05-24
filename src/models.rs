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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub embeds: Vec<DiscordEmbed>,
}

#[derive(Debug, Serialize)]
pub struct DiscordEmbed {
    pub title: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub color: u32,
    pub footer: DiscordFooter,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<DiscordImage>,
    pub fields: Vec<DiscordField>,
}

#[derive(Debug, Serialize)]
pub struct DiscordField {
    pub name: String,
    pub value: String,
    pub inline: bool,
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