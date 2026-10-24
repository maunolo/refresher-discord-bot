use anyhow::{Context, Result};
use serenity::model::id::ChannelId;

#[derive(Clone, Debug)]
pub struct Config {
    pub discord_token: String,
    pub deadlock_api_key: Option<String>,
    pub database_url: String,
    pub deadlock_server_region: Option<String>,
    pub enable_custom_match_create: bool,
    pub draft_base_url: String,
    pub queue_voice_channel_id: Option<ChannelId>,
    pub match1_hiddenking_voice_id: Option<ChannelId>,
    pub match1_archmother_voice_id: Option<ChannelId>,
    pub match2_hiddenking_voice_id: Option<ChannelId>,
    pub match2_archmother_voice_id: Option<ChannelId>,
}

fn env_flag(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}

fn optional_channel(name: &str) -> Result<Option<ChannelId>> {
    match std::env::var(name) {
        Ok(value) if value.trim().is_empty() => Ok(None),
        Ok(value) => {
            let id = value
                .trim()
                .parse::<u64>()
                .with_context(|| format!("invalid {name}"))?;
            Ok(Some(ChannelId::new(id)))
        }
        Err(_) => Ok(None),
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();
        Ok(Self {
            discord_token: std::env::var("DISCORD_TOKEN")
                .context("DISCORD_TOKEN is required")?,
            deadlock_api_key: std::env::var("DEADLOCK_API_KEY")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/camps.db".to_string()),
            deadlock_server_region: std::env::var("DEADLOCK_SERVER_REGION")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            enable_custom_match_create: env_flag("ENABLE_CUSTOM_MATCH_CREATE", false),
            draft_base_url: std::env::var("DRAFT_BASE_URL")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| crate::draft::DEFAULT_BASE_URL.to_string()),
            queue_voice_channel_id: optional_channel("QUEUE_VOICE_CHANNEL_ID")?,
            match1_hiddenking_voice_id: optional_channel("MATCH1_HIDDENKING_VOICE_CHANNEL_ID")?,
            match1_archmother_voice_id: optional_channel("MATCH1_ARCHMOTHER_VOICE_CHANNEL_ID")?,
            match2_hiddenking_voice_id: optional_channel("MATCH2_HIDDENKING_VOICE_CHANNEL_ID")?,
            match2_archmother_voice_id: optional_channel("MATCH2_ARCHMOTHER_VOICE_CHANNEL_ID")?,
        })
    }
}
