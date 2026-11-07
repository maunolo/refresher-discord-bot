use std::collections::HashMap;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use refresher_core::link::Changes;

use crate::config::Config;
use crate::db;
use crate::settings::GuildSettings;

#[derive(Clone)]
pub struct Data {
    pub config: Config,
    pub db: SqlitePool,
    pub http: reqwest::Client,
    pub locks: Arc<Mutex<HashMap<u64, Arc<Mutex<()>>>>>,
    /// Tells open admin pages when a server's camp changed.
    pub changes: Changes,
}

impl Data {
    pub async fn guild_lock(&self, guild_id: u64) -> Arc<Mutex<()>> {
        let mut map = self.locks.lock().await;
        map.entry(guild_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// How this server's camps run: its settings from the admin page, else the `.env` values.
    pub async fn settings(&self, guild_id: u64) -> anyhow::Result<GuildSettings> {
        let defaults = GuildSettings::from_config(&self.config);
        Ok(db::guild_settings(&self.db, guild_id as i64, &defaults).await?.0)
    }
}

pub async fn with_guild_lock<F, Fut, T>(data: &Data, guild_id: u64, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let lock = data.guild_lock(guild_id).await;
    let _guard = lock.lock().await;
    f().await
}
