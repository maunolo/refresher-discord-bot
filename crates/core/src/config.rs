use anyhow::{bail, Context, Result};
use std::num::NonZeroU64;

/// Voice channel IDs are raw Discord snowflakes so this crate stays free of Discord types.
#[derive(Clone, Debug)]
pub struct Config {
    pub discord_token: String,
    pub deadlock_api_key: Option<String>,
    pub database_url: String,
    pub deadlock_server_region: Option<String>,
    pub enable_custom_match_create: bool,
    /// DeadChaps API key with the `drafts.write` scope; without it the bot can't create drafts.
    pub deadchaps_api_key: Option<String>,
    pub queue_voice_channel_id: Option<u64>,
    pub match1_hiddenking_voice_id: Option<u64>,
    pub match1_archmother_voice_id: Option<u64>,
    pub match2_hiddenking_voice_id: Option<u64>,
    pub match2_archmother_voice_id: Option<u64>,
    /// The admin web page, where camps are run.
    pub web: WebConfig,
}

/// Settings for the admin web page, which admins sign in to with Discord.
#[derive(Clone, Debug)]
pub struct WebConfig {
    /// Address the HTTP server listens on, e.g. `127.0.0.1:8080`.
    pub bind: String,
    /// Address admins open in the browser, without a trailing slash.
    /// Discord sends them back to `{public_url}/auth/callback` after login.
    pub public_url: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    /// The built admin page (`npm run build` in `web/`).
    pub static_dir: String,
}

fn optional_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn web_from_env() -> Result<WebConfig> {
    let (Some(discord_client_id), Some(discord_client_secret)) = (
        optional_var("DISCORD_CLIENT_ID"),
        optional_var("DISCORD_CLIENT_SECRET"),
    ) else {
        bail!(
            "DISCORD_CLIENT_ID and DISCORD_CLIENT_SECRET are required: camps are run from the admin page, \
             which admins sign in to with Discord (see README, \"Admin web page\")"
        );
    };
    Ok(WebConfig {
        bind: optional_var("WEB_BIND").unwrap_or_else(|| "127.0.0.1:8080".to_string()),
        public_url: optional_var("WEB_PUBLIC_URL")
            .unwrap_or_else(|| "http://localhost:8080".to_string())
            .trim_end_matches('/')
            .to_string(),
        discord_client_id,
        discord_client_secret,
        static_dir: optional_var("WEB_STATIC_DIR").unwrap_or_else(|| "web/dist".to_string()),
    })
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

fn optional_channel(name: &str) -> Result<Option<u64>> {
    match std::env::var(name) {
        Ok(value) if value.trim().is_empty() => Ok(None),
        Ok(value) => {
            let id = value
                .trim()
                .parse::<NonZeroU64>()
                .with_context(|| format!("invalid {name}"))?;
            Ok(Some(id.get()))
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
            deadchaps_api_key: optional_var("DEADCHAPS_API_KEY"),
            queue_voice_channel_id: optional_channel("QUEUE_VOICE_CHANNEL_ID")?,
            match1_hiddenking_voice_id: optional_channel("MATCH1_HIDDENKING_VOICE_CHANNEL_ID")?,
            match1_archmother_voice_id: optional_channel("MATCH1_ARCHMOTHER_VOICE_CHANNEL_ID")?,
            match2_hiddenking_voice_id: optional_channel("MATCH2_HIDDENKING_VOICE_CHANNEL_ID")?,
            match2_archmother_voice_id: optional_channel("MATCH2_ARCHMOTHER_VOICE_CHANNEL_ID")?,
            web: web_from_env()?,
        })
    }
}
