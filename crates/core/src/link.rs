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

/// An admin action on a camp. The bot runs it exactly like the matching Discord button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CampAction {
    CreateMatch,
    CreateFinal,
    StartDraft { match_id: String },
    StartMatch { match_id: String },
    /// `side` is `hiddenking` or `archmother`.
    ReportWinner { match_id: String, side: String },
    CorrectWinner { match_id: String },
    ReopenMatch { match_id: String },
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

    /// Runs an admin action on a camp of this server: the database change plus everything the
    /// bot does in Discord for it (messages, voice moves, captain DMs). Errors are for the admin.
    fn act<'a>(
        &'a self,
        guild_id: u64,
        camp_id: &'a str,
        action: CampAction,
    ) -> BoxFuture<'a, anyhow::Result<()>>;
}
