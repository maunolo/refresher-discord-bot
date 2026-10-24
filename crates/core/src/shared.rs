use std::collections::HashSet;
use std::sync::{Arc, PoisonError, RwLock};

/// Servers the bot is in right now. The bot keeps it current; the web admin reads it.
#[derive(Clone, Default)]
pub struct BotGuilds(Arc<RwLock<HashSet<u64>>>);

impl BotGuilds {
    pub fn insert(&self, guild_id: u64) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(guild_id);
    }

    pub fn remove(&self, guild_id: u64) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&guild_id);
    }

    pub fn contains(&self, guild_id: u64) -> bool {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&guild_id)
    }
}
