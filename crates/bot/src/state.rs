use std::collections::HashMap;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use refresher_core::shared::BotGuilds;

use crate::config::Config;

#[derive(Clone)]
pub struct Data {
    pub config: Config,
    pub db: SqlitePool,
    pub http: reqwest::Client,
    pub locks: Arc<Mutex<HashMap<u64, Arc<Mutex<()>>>>>,
    pub bot_guilds: BotGuilds,
}

impl Data {
    pub async fn guild_lock(&self, guild_id: u64) -> Arc<Mutex<()>> {
        let mut map = self.locks.lock().await;
        map.entry(guild_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
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
