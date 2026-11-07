use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use tokio::sync::broadcast;

use crate::queue::{CampSize, Role};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Servers whose camp just changed, so open admin pages reload it right away instead of polling.
#[derive(Clone)]
pub struct Changes(broadcast::Sender<u64>);

impl Changes {
    pub fn new() -> Self {
        Self(broadcast::channel(64).0)
    }

    /// Something on this server's camp changed: an admin action, a Discord button, the Queue
    /// voice channel. Nobody listening is fine.
    pub fn notify(&self, guild_id: u64) {
        let _ = self.0.send(guild_id);
    }

    /// Server IDs as their camps change. A listener that falls behind gets `Lagged`: it missed
    /// some, so it should reload anyway.
    pub fn subscribe(&self) -> broadcast::Receiver<u64> {
        self.0.subscribe()
    }
}

impl Default for Changes {
    fn default() -> Self {
        Self::new()
    }
}

/// A channel of a server, by ID and name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    pub id: u64,
    pub name: String,
}

/// How a Discord user appears on the admin page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// Server nickname, else Discord display name, else username.
    pub name: String,
    pub avatar_url: String,
}

/// An admin action on a camp, from the admin page. The bot makes the change and updates Discord.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CampAction {
    CreateMatch,
    CreateFinal,
    /// Ends the refresh vote now and keeps the teams.
    CloseVote { match_id: String },
    /// Ends the refresh vote now by re-sorting the teams, as if the refresh had passed.
    RefreshTeams { match_id: String },
    StartDraft { match_id: String },
    StartMatch { match_id: String },
    /// `side` is `hiddenking` or `archmother`. The MVP, one of the match's players, breaks ties
    /// in points in the final ranking.
    ReportWinner { match_id: String, side: String, mvp_user_id: Option<u64> },
    CorrectWinner { match_id: String },
    /// The latest result only, while the camp runs.
    ChangeMvp { match_id: String, user_id: u64 },
    ReopenMatch { match_id: String },
    /// Signup only. Users already on the roster, or past the camp's size, are skipped.
    AddPlayers { user_ids: Vec<u64> },
    /// Signup only.
    RemovePlayer { user_id: u64 },
    /// Signup or running; matches created afterwards use it.
    SetSRank { user_id: u64, s_rank: bool },
    /// Signup or running. The role is kept per server, for later camps too; `None` clears it.
    SetRole { user_id: u64, role: Option<Role> },
    /// Running only. Adds `delta` points by hand (negative takes them); never below zero.
    AdjustPoints { user_id: u64, delta: i64 },
    StartQueue,
    /// The queue in a new order: exactly the players queued now. Holds until the next result.
    ReorderQueue { user_ids: Vec<u64> },
    /// Mentions, in the camp's channel, the players of a match who haven't voted yet.
    RemindVoters { match_id: String },
    /// Soft: the camp keeps its data and can be resumed.
    CancelCamp,
    ResumeCamp,
    /// Posts the final ranking. Can't be undone.
    EndCamp,
}

/// What the web admin asks of the running bot, answered through its Discord connection.
pub trait BotLink: Send + Sync {
    /// Whether the bot is in this server.
    fn in_guild(&self, guild_id: u64) -> bool;

    /// Told whenever a server's camp changes, wherever the change came from.
    fn changes(&self) -> &Changes;

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

    /// Text channels of a server, in Discord's order.
    fn text_channels(&self, guild_id: u64) -> Vec<Channel>;

    /// Voice channels of a server, in Discord's order.
    fn voice_channels(&self, guild_id: u64) -> Vec<Channel>;

    /// Every member of a server except bots. Errors are for the admin.
    fn members(&self, guild_id: u64) -> BoxFuture<'_, anyhow::Result<Vec<(u64, Profile)>>>;

    /// Users in the server's Queue voice channel right now.
    fn queue_voice_members(&self, guild_id: u64) -> BoxFuture<'_, Vec<u64>>;

    /// Opens signup for a new camp in a text channel of this server and posts its message there.
    /// Returns the camp ID. A server has one active camp at a time.
    fn create_camp<'a>(
        &'a self,
        guild_id: u64,
        channel_id: u64,
        size: CampSize,
    ) -> BoxFuture<'a, anyhow::Result<String>>;
}
