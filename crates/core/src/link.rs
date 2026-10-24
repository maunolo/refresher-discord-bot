use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// How a Discord user appears on the admin page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// Server nickname, else Discord display name, else username.
    pub name: String,
    pub avatar_url: String,
}

/// What the web admin asks of the running bot, answered through its Discord connection.
pub trait BotLink: Send + Sync {
    /// Whether the bot is in this server.
    fn in_guild(&self, guild_id: u64) -> bool;

    /// Profiles of these users in a server. Users Discord can't find are left out.
    fn profiles<'a>(
        &'a self,
        guild_id: u64,
        user_ids: &'a [u64],
    ) -> BoxFuture<'a, HashMap<u64, Profile>>;
}
