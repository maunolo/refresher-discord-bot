//! Per-server settings, edited on the admin page's Configurações screen.

use crate::config::Config;

/// Seconds per pick or ban; 0 means no timer, which DeadChaps caps at its 5-minute maximum.
pub const DRAFT_TIMERS: [u32; 6] = [0, 15, 30, 45, 60, 90];

/// How a server's camps run. A voice channel left empty is found by its name instead
/// (`Queue`, `Match 1 Hiddenking`, …).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildSettings {
    pub queue_voice_id: Option<u64>,
    pub match1_hiddenking_voice_id: Option<u64>,
    pub match1_archmother_voice_id: Option<u64>,
    pub match2_hiddenking_voice_id: Option<u64>,
    pub match2_archmother_voice_id: Option<u64>,
    /// Create a Deadlock custom lobby (and post its party code) when a match starts.
    pub create_lobby: bool,
    pub server_region: Option<String>,
    /// Create drafts on DeadChaps and DM captains the link. Off: the admin shares a
    /// draft link themselves, and the bot only moves players and announces the draft.
    pub use_draft_tool: bool,
    pub draft_timer_seconds: u32,
}

impl GuildSettings {
    /// For a server that never saved settings: the `.env` values, as before the admin page.
    pub fn from_config(config: &Config) -> Self {
        Self {
            queue_voice_id: config.queue_voice_channel_id,
            match1_hiddenking_voice_id: config.match1_hiddenking_voice_id,
            match1_archmother_voice_id: config.match1_archmother_voice_id,
            match2_hiddenking_voice_id: config.match2_hiddenking_voice_id,
            match2_archmother_voice_id: config.match2_archmother_voice_id,
            create_lobby: config.enable_custom_match_create,
            server_region: config.deadlock_server_region.clone(),
            use_draft_tool: true,
            draft_timer_seconds: 30,
        }
    }

    /// The voice channel picked for each slot, in the order Queue, Match 1 Hiddenking,
    /// Match 1 Archmother, Match 2 Hiddenking, Match 2 Archmother.
    pub fn voice_ids(&self) -> [Option<u64>; 5] {
        [
            self.queue_voice_id,
            self.match1_hiddenking_voice_id,
            self.match1_archmother_voice_id,
            self.match2_hiddenking_voice_id,
            self.match2_archmother_voice_id,
        ]
    }
}
