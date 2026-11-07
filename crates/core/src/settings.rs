//! Per-server settings, edited on the admin page's Configurações screen.

use crate::config::Config;
use crate::draft::{DraftOptions, DraftView};

/// Seconds per pick or ban; 0 means no timer, which DeadChaps caps at its 5-minute maximum.
pub const DRAFT_TIMERS: [u32; 6] = [0, 15, 30, 45, 60, 90];

/// Reserve seconds per side, spent once a turn's timer runs out. DeadChaps allows up to 10 minutes.
pub const DRAFT_RESERVES: [u32; 8] = [0, 30, 60, 90, 120, 180, 300, 600];

/// The most bans per team DeadChaps allows; 0 turns bans off.
pub const MAX_DRAFT_BANS: u32 = 3;

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
    /// Create drafts on DeadChaps. Off: the admin shares a draft link themselves, and the bot
    /// only moves players and announces the draft.
    pub use_draft_tool: bool,
    pub draft_timer_seconds: u32,
    pub draft_reserve_seconds: u32,
    pub draft_bans_per_team: u32,
    pub draft_view: DraftView,
    /// DM each captain the draft's invite link. Either way, captains can open it from the
    /// match message's captain-only button.
    pub dm_captains: bool,
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
            draft_reserve_seconds: 90,
            draft_bans_per_team: 2,
            draft_view: DraftView::Cinematic,
            dm_captains: true,
        }
    }

    /// The rules of drafts created on DeadChaps.
    pub fn draft_options(&self) -> DraftOptions {
        DraftOptions {
            timer_seconds: self.draft_timer_seconds,
            reserve_seconds: self.draft_reserve_seconds,
            bans_per_team: self.draft_bans_per_team,
            view: self.draft_view,
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
