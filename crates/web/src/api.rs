//! Read-only JSON for the admin page. Discord IDs go out as strings: they don't fit in a
//! JavaScript number.

use std::collections::HashMap;
use std::convert::Infallible;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use futures_util::stream::{self, Stream};
use refresher_core::db::{
    self, CampRow, CampState, CampSummary, MatchPlayerRow, MatchPoints, MatchRow, PlayerRow, PlayerStats, RecentMatch,
    RefreshVote,
};
use refresher_core::cameras::{self, CameraConfig, StreamView};
use refresher_core::draft::{self, DraftView};
use refresher_core::link::{CampAction, Channel, Profile};
use refresher_core::presets;
use refresher_core::queue::{CampSize, Role};
use refresher_core::ranking::{rank_with_movement, LatestResult, Standing};
use refresher_core::settings::{GuildSettings, DRAFT_RESERVES, DRAFT_TIMERS, MAX_DRAFT_BANS};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;

use crate::session::Session;

use crate::auth::Admin;
use crate::error::ApiError;
use crate::AppState;

#[derive(Serialize)]
pub struct Me {
    user: UserDto,
    /// Servers the user manages that the bot is also in.
    guilds: Vec<GuildDto>,
    /// The public league site, for admins of its server.
    site: Option<SiteLinkDto>,
}

#[derive(Serialize)]
struct SiteLinkDto {
    guild_id: String,
    url: String,
}

#[derive(Serialize)]
struct UserDto {
    id: String,
    name: String,
    avatar_url: Option<String>,
}

#[derive(Serialize)]
struct GuildDto {
    id: String,
    name: String,
    icon_url: Option<String>,
}

#[derive(Serialize)]
pub struct CampSummaryDto {
    id: String,
    channel_id: String,
    status: String,
    target_size: i64,
    match_size: i64,
    match_count: i64,
    created_at: i64,
    /// The camp's message in Discord, else its channel.
    discord_url: String,
}

#[derive(Serialize)]
pub struct CampDto {
    id: String,
    guild_id: String,
    channel_id: String,
    status: String,
    target_size: i64,
    match_size: i64,
    match_count: i64,
    /// In the finals: every new match is played by the top of the ranking, one at a time.
    finals: bool,
    can_create_match: bool,
    can_enter_finals: bool,
    /// Not while a finals match is open.
    can_leave_finals: bool,
    /// Refresh votes, from either team, that re-sort a match.
    refresh_votes: usize,
    players: Vec<PlayerDto>,
    /// Who the next match would take: the queue's front, or in the finals the top of the ranking.
    up_next: Vec<String>,
    open_matches: Vec<MatchDto>,
    recent_matches: Vec<RecentMatchDto>,
    /// Name and avatar per user ID, for everyone on the roster Discord could find.
    members: HashMap<String, MemberDto>,
    /// While the camp is active: who is in the Queue voice channel right now, roster or not.
    queue_voice: Vec<String>,
    /// The camp's message in Discord (signup, then queue), else its channel.
    discord_url: String,
    /// The server creates drafts on the draft tool; off, admins share a draft link themselves.
    draft_tool: bool,
    /// Captains get the draft's invite link by DM; either way it's on their match message button.
    dm_captains: bool,
    /// What the newest match gives, the starting values for the next one.
    next_points: PointsDto,
}

#[derive(Serialize)]
struct PointsDto {
    win: i64,
    loss: i64,
}

impl From<MatchPoints> for PointsDto {
    fn from(p: MatchPoints) -> Self {
        Self { win: p.win, loss: p.loss }
    }
}

#[derive(Serialize)]
pub struct ChannelDto {
    id: String,
    name: String,
}

#[derive(Serialize)]
pub struct ServerMemberDto {
    id: String,
    name: String,
    avatar_url: String,
}

#[derive(Serialize)]
struct MemberDto {
    name: String,
    avatar_url: String,
}

#[derive(Serialize)]
struct RecentMatchDto {
    id: String,
    /// Position among the camp's finished matches, 1 being the first.
    number: i64,
    winner_side: String,
    is_final: bool,
    completed_at: i64,
    points: PointsDto,
    captains: Vec<CaptainDto>,
    players: Vec<MatchPlayerDto>,
    mvp_user_id: Option<String>,
    /// The bot's rule: only the latest result, and only while its players are all back in the queue.
    correctable: bool,
    /// Only the latest result, while the camp runs; players may have moved on.
    mvp_changeable: bool,
}

#[derive(Serialize)]
struct CaptainDto {
    user_id: String,
    side: String,
}

#[derive(Serialize)]
struct PlayerDto {
    user_id: String,
    points: i64,
    location: String,
    queue_position: Option<i64>,
    s_rank: bool,
    /// Above S rank: captains their team, two a team of a match at most.
    ss_rank: bool,
    /// `tank`, `carry` or `support`; kept per server.
    role: Option<&'static str>,
    /// Finished matches only. Fewer plays first, and breaks ties in points and MVPs.
    matches_played: i64,
    wins: i64,
    /// Points admins added (or, below zero, took) by hand.
    points_adjusted: i64,
    /// Finished matches as MVP: the tie-breaker for equal points.
    mvps: i64,
    /// An admin took them out of the camp: no queue, no new matches. In an open match, they
    /// leave once it ends.
    removed: bool,
}

#[derive(Serialize)]
pub struct MatchDto {
    id: String,
    camp_id: String,
    slot: i64,
    status: String,
    is_final: bool,
    refresh_used: bool,
    vote_closed_by_admin: bool,
    /// The draft started without the draft tool: no links, the admin shares one.
    draft_manual: bool,
    /// An admin ended the vote by re-sorting the teams.
    refreshed_by_admin: bool,
    winner_side: Option<String>,
    mvp_user_id: Option<String>,
    /// What each player earns when it ends.
    points: PointsDto,
    party_code: Option<String>,
    draft: Option<DraftDto>,
    players: Vec<MatchPlayerDto>,
    votes: Vec<VoteDto>,
    /// Finished matches: position in the order they ended, 1 being the first.
    number: Option<i64>,
    completed_at: Option<i64>,
    /// The match's message in Discord, else the camp's channel.
    discord_url: String,
}

/// Admins see the captains' invite link too, so they can help a captain whose DM failed.
#[derive(Serialize)]
struct DraftDto {
    spectator_url: String,
    /// Missing on drafts made before DeadChaps, which had one link per team.
    join_url: Option<String>,
    /// The draft as an OBS overlay, for streaming it. Missing on drafts made before DeadChaps.
    obs_url: Option<String>,
}

#[derive(Serialize)]
struct MatchPlayerDto {
    user_id: String,
    side: String,
    is_captain: bool,
    points_after: Option<i64>,
}

#[derive(Serialize)]
struct VoteDto {
    user_id: String,
    refresh: bool,
}

fn avatar_url(user_id: u64, hash: &str) -> String {
    format!("https://cdn.discordapp.com/avatars/{user_id}/{hash}.png")
}

fn icon_url(guild_id: u64, hash: &str) -> String {
    format!("https://cdn.discordapp.com/icons/{guild_id}/{hash}.png")
}

/// A message in Discord, or the channel when the message isn't known.
fn discord_url(guild_id: i64, channel_id: i64, message_id: Option<i64>) -> String {
    match message_id {
        Some(message_id) => format!("https://discord.com/channels/{guild_id}/{channel_id}/{message_id}"),
        None => format!("https://discord.com/channels/{guild_id}/{channel_id}"),
    }
}

fn match_player_dto(p: &MatchPlayerRow) -> MatchPlayerDto {
    MatchPlayerDto {
        user_id: p.user_id.to_string(),
        side: p.side.clone(),
        is_captain: p.is_captain,
        points_after: p.points_after,
    }
}

fn match_dto(camp: &CampRow, row: &MatchRow, players: &[MatchPlayerRow], votes: &[RefreshVote]) -> MatchDto {
    MatchDto {
        id: row.id.clone(),
        camp_id: camp.id.clone(),
        slot: row.slot,
        status: row.status.clone(),
        is_final: row.is_final,
        refresh_used: row.refresh_used,
        vote_closed_by_admin: row.vote_closed_by_admin,
        draft_manual: row.draft_manual,
        refreshed_by_admin: row.refreshed_by_admin,
        winner_side: row.winner_side.clone(),
        mvp_user_id: row.mvp_user_id.map(|id| id.to_string()),
        points: row.points().into(),
        party_code: row.party_code.clone(),
        draft: row.draft_url.clone().map(|spectator_url| DraftDto {
            obs_url: draft::obs_url(&spectator_url),
            spectator_url,
            join_url: row.draft_join_url.clone(),
        }),
        players: players.iter().map(match_player_dto).collect(),
        votes: votes
            .iter()
            .map(|v| VoteDto {
                user_id: v.user_id.to_string(),
                refresh: v.refresh,
            })
            .collect(),
        number: None,
        completed_at: None,
        discord_url: discord_url(camp.guild_id, camp.channel_id, row.message_id),
    }
}

fn recent_dto(state: &CampState, recent: &[RecentMatch]) -> Vec<RecentMatchDto> {
    // Removed players are out of the queue but in no other match either.
    let queued = |user_id: i64| {
        state
            .players
            .iter()
            .any(|p| p.user_id == user_id && (p.location == "queued" || p.location == "removed"))
    };
    recent
        .iter()
        .enumerate()
        .map(|(i, m)| RecentMatchDto {
            id: m.id.clone(),
            number: state.camp.match_count - i as i64,
            winner_side: m.winner_side.clone(),
            is_final: m.is_final,
            completed_at: m.completed_at,
            points: m.points.into(),
            captains: m
                .players
                .iter()
                .filter(|p| p.is_captain)
                .map(|p| CaptainDto {
                    user_id: p.user_id.to_string(),
                    side: p.side.clone(),
                })
                .collect(),
            players: m.players.iter().map(match_player_dto).collect(),
            mvp_user_id: m.mvp_user_id.map(|id| id.to_string()),
            correctable: i == 0
                && state.camp.status == "running"
                && m.players.iter().all(|p| queued(p.user_id)),
            mvp_changeable: i == 0 && state.camp.status == "running",
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn camp_dto(
    state: &CampState,
    up_next: &[i64],
    recent: &[RecentMatch],
    stats: &HashMap<i64, PlayerStats>,
    profiles: &HashMap<u64, Profile>,
    queue_voice: Vec<u64>,
    settings: &GuildSettings,
    next_points: MatchPoints,
) -> CampDto {
    CampDto {
        id: state.camp.id.clone(),
        guild_id: state.camp.guild_id.to_string(),
        channel_id: state.camp.channel_id.to_string(),
        status: state.camp.status.clone(),
        target_size: state.camp.target_size,
        match_size: state.camp.match_size,
        match_count: state.camp.match_count,
        finals: state.camp.finals,
        can_create_match: state.can_create_match(),
        can_enter_finals: state.can_enter_finals(),
        can_leave_finals: state.can_leave_finals(),
        refresh_votes: state.camp.layout().refresh_votes,
        players: state
            .players
            .iter()
            .map(|p| {
                let stats = stats.get(&p.user_id).copied().unwrap_or_default();
                PlayerDto {
                    user_id: p.user_id.to_string(),
                    points: p.points,
                    location: p.location.clone(),
                    queue_position: p.queue_position,
                    s_rank: p.s_rank,
                    ss_rank: p.ss_rank,
                    role: p.role.map(Role::as_str),
                    matches_played: p.matches_played,
                    wins: stats.wins,
                    points_adjusted: stats.adjusted,
                    mvps: stats.mvps,
                    removed: p.removed,
                }
            })
            .collect(),
        up_next: up_next.iter().map(|id| id.to_string()).collect(),
        open_matches: state
            .open_matches
            .iter()
            .map(|m| match_dto(&state.camp, &m.row, &m.players, &m.votes))
            .collect(),
        recent_matches: recent_dto(state, recent),
        members: profiles
            .iter()
            .map(|(id, p)| {
                (
                    id.to_string(),
                    MemberDto {
                        name: p.name.clone(),
                        avatar_url: p.avatar_url.clone(),
                    },
                )
            })
            .collect(),
        queue_voice: queue_voice.iter().map(u64::to_string).collect(),
        discord_url: discord_url(state.camp.guild_id, state.camp.channel_id, state.camp.board_message_id),
        draft_tool: settings.use_draft_tool,
        dm_captains: settings.dm_captains,
        next_points: next_points.into(),
    }
}

fn summary_dto(guild_id: u64, camp: CampSummary) -> CampSummaryDto {
    CampSummaryDto {
        discord_url: discord_url(guild_id as i64, camp.channel_id, camp.board_message_id),
        id: camp.id,
        channel_id: camp.channel_id.to_string(),
        status: camp.status,
        target_size: camp.target_size,
        match_size: camp.match_size,
        match_count: camp.match_count,
        created_at: camp.created_at,
    }
}

pub async fn me(State(state): State<AppState>, Admin(session): Admin) -> Json<Me> {
    Json(Me {
        user: UserDto {
            id: session.user_id.to_string(),
            name: session.display_name.clone(),
            avatar_url: session.avatar.as_deref().map(|hash| avatar_url(session.user_id, hash)),
        },
        guilds: session
            .admin_guilds
            .iter()
            .filter(|g| state.link.in_guild(g.id))
            .map(|g| GuildDto {
                id: g.id.to_string(),
                name: g.name.clone(),
                icon_url: g.icon.as_deref().map(|hash| icon_url(g.id, hash)),
            })
            .collect(),
        site: state
            .config
            .site_guild_id
            .filter(|id| session.is_admin_of(*id) && state.link.in_guild(*id))
            .map(|id| SiteLinkDto {
                guild_id: id.to_string(),
                url: crate::site::site_url(&state.config),
            }),
    })
}

pub async fn guild_camps(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
) -> Result<Json<Vec<CampSummaryDto>>, ApiError> {
    if !session.is_admin_of(guild_id) {
        return Err(ApiError::NotFound);
    }
    let camps = db::list_camps_for_guild(&state.db, guild_id as i64).await?;
    Ok(Json(camps.into_iter().map(|c| summary_dto(guild_id, c)).collect()))
}

/// A camp in a server the admin manages, or 404.
pub(crate) async fn admin_camp(state: &AppState, session: &Session, camp_id: &str) -> Result<CampState, ApiError> {
    db::load_camp(&state.db, &camp_id.trim().to_uppercase())
        .await?
        .filter(|c| session.is_admin_of(c.camp.guild_id as u64))
        .ok_or(ApiError::NotFound)
}

pub async fn camp(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(camp_id): Path<String>,
) -> Result<Json<CampDto>, ApiError> {
    let camp = admin_camp(&state, &session, &camp_id).await?;
    camp_response(&state, camp).await
}

/// Every match of a camp, open ones first, then finished ones newest first.
pub async fn camp_matches(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(camp_id): Path<String>,
) -> Result<Json<Vec<MatchDto>>, ApiError> {
    let camp = admin_camp(&state, &session, &camp_id).await?;
    let matches = db::list_matches(&state.db, &camp.camp.id).await?;
    Ok(Json(
        matches
            .iter()
            .map(|m| MatchDto {
                number: m.number,
                completed_at: m.completed_at,
                ..match_dto(&camp.camp, &m.row, &m.players, &m.votes)
            })
            .collect(),
    ))
}

async fn camp_response(state: &AppState, camp: CampState) -> Result<Json<CampDto>, ApiError> {
    let recent = db::recent_matches(&state.db, &camp.camp.id, 5).await?;
    let stats = db::player_stats(&state.db, &camp.camp.id).await?;
    let user_ids: Vec<u64> = camp.players.iter().map(|p| p.user_id as u64).collect();
    let profiles = state
        .link
        .profiles(camp.camp.guild_id as u64, &user_ids)
        .await;
    let queue_voice = if matches!(camp.camp.status.as_str(), "signup" | "running") {
        state.link.queue_voice_members(camp.camp.guild_id as u64).await
    } else {
        Vec::new()
    };
    let (settings, _) = db::guild_settings(&state.db, camp.camp.guild_id, &state.default_settings).await?;
    let next_points = db::last_match_points(&state.db, &camp.camp.id).await?;
    let up_next = db::up_next(&state.db, &camp).await?;
    Ok(Json(camp_dto(&camp, &up_next, &recent, &stats, &profiles, queue_voice, &settings, next_points)))
}

/// A server the admin manages and the bot is in, or 404.
pub(crate) fn managed_guild(state: &AppState, session: &Session, guild_id: u64) -> Result<(), ApiError> {
    if session.is_admin_of(guild_id) && state.link.in_guild(guild_id) {
        Ok(())
    } else {
        Err(ApiError::NotFound)
    }
}

/// Live updates, as server-sent events: a `change` each time this server's camp changes, from
/// the page, a Discord button or the Queue voice channel, so open pages reload it right away.
pub async fn guild_events(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    Ok(change_stream(&state, guild_id))
}

/// A `change` event each time the server's camp changes.
fn change_stream(state: &AppState, guild_id: u64) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let changes = state.link.changes().subscribe();
    let events = stream::unfold(changes, move |mut changes| async move {
        loop {
            match changes.recv().await {
                Ok(id) if id != guild_id => continue,
                // Lagged: some changes went by unseen, so reload all the same.
                Ok(_) | Err(RecvError::Lagged(_)) => {
                    let change = Event::default().event("change").data(guild_id.to_string());
                    return Some((Ok(change), changes));
                }
                Err(RecvError::Closed) => return None,
            }
        }
    });
    // Comments every 15 seconds keep proxies from closing an idle stream.
    Sse::new(events).keep_alive(KeepAlive::default())
}

#[derive(Serialize)]
pub struct OverlayLinksDto {
    /// The refresh vote overlay, for an OBS browser source.
    vote_url: String,
    /// The ranking overlay.
    ranking_url: String,
    /// The commentators' cameras, one overlay per stream: stream 1, then stream 2.
    camera_urls: Vec<String>,
    /// The replay clips, one overlay per stream.
    replay_urls: Vec<String>,
}

fn overlay_links(state: &AppState, token: &str) -> OverlayLinksDto {
    let base = state.config.public_url.trim_end_matches('/');
    OverlayLinksDto {
        vote_url: format!("{base}/overlay/{token}/votacao"),
        ranking_url: format!("{base}/overlay/{token}/ranking"),
        camera_urls: (1..=cameras::STREAMS)
            .map(|n| format!("{base}/overlay/{token}/cameras{n}"))
            .collect(),
        replay_urls: refresher_core::clips::STREAMS
            .iter()
            .map(|n| format!("{base}/overlay/{token}/replay{n}"))
            .collect(),
    }
}

/// The server's commentator cameras: the VDO.Ninja room, its sources and each stream's picks.
pub async fn guild_cameras(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
) -> Result<Json<CameraConfig>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    Ok(Json(cameras::load(&state.db, guild_id as i64).await?))
}

/// Saves the server's cameras; open camera overlays switch to them within seconds.
pub async fn save_guild_cameras(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
    Json(config): Json<CameraConfig>,
) -> Result<Json<CameraConfig>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let config = config.check().map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let saved = cameras::save(&state.db, guild_id as i64, config).await?;
    tracing::info!(user_id = session.user_id, guild_id, "camera overlays saved from the web");
    state.link.changes().notify(guild_id);
    Ok(Json(saved))
}

/// One stream's cameras, for its overlay. Public: the link's secret is the only key, and it
/// carries the room's password, as the VDO.Ninja links do.
pub async fn overlay_cameras(
    State(state): State<AppState>,
    Path((token, stream)): Path<(String, usize)>,
) -> Result<Json<StreamView>, ApiError> {
    let guild_id = overlay_guild(&state, &token).await?;
    let config = cameras::load(&state.db, guild_id as i64).await?;
    config.stream(stream).map(Json).ok_or(ApiError::NotFound)
}

/// The server's OBS overlay links, made on first use.
pub async fn guild_overlays(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
) -> Result<Json<OverlayLinksDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let token = db::overlay_token(&state.db, guild_id as i64).await?;
    Ok(Json(overlay_links(&state, &token)))
}

/// New overlay links for the server; the old ones stop working.
pub async fn reset_guild_overlays(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
) -> Result<Json<OverlayLinksDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let token = db::reset_overlay_token(&state.db, guild_id as i64).await?;
    Ok(Json(overlay_links(&state, &token)))
}

/// The server behind an overlay link, or 404.
pub(crate) async fn overlay_guild(state: &AppState, token: &str) -> Result<u64, ApiError> {
    match db::overlay_guild(&state.db, token).await? {
        Some(guild_id) => Ok(guild_id as u64),
        None => Err(ApiError::NotFound),
    }
}

#[derive(Serialize)]
pub struct OverlayVotesDto {
    votes: Vec<OverlayVoteDto>,
}

/// A match's refresh vote, from its creation until the draft starts.
#[derive(Serialize)]
struct OverlayVoteDto {
    match_id: String,
    slot: i64,
    is_final: bool,
    /// `open` while players vote, then `refreshed` (new teams) or `kept`.
    state: &'static str,
    /// Refresh votes, from either side, that bring new teams.
    needed: usize,
    /// Refresh votes so far, both sides together.
    refresh: usize,
    /// Votes to keep the teams.
    keep: usize,
    voted: usize,
    total: usize,
    /// Hiddenking, then Archmother.
    teams: Vec<OverlayTeamDto>,
}

#[derive(Serialize)]
struct OverlayTeamDto {
    side: &'static str,
    /// The side's Refresh votes.
    refresh: usize,
    /// Captains first, then by role.
    players: Vec<OverlayPlayerDto>,
}

/// What the stream sees of a player: whether they voted, never how.
#[derive(Serialize)]
struct OverlayPlayerDto {
    user_id: String,
    name: String,
    avatar_url: Option<String>,
    captain: bool,
    s_rank: bool,
    ss_rank: bool,
    role: Option<&'static str>,
    voted: bool,
}

fn overlay_vote_dto(camp: &CampState, open: &db::OpenMatch, profiles: &HashMap<u64, Profile>) -> OverlayVoteDto {
    let row = &open.row;
    let state = if !row.refresh_used {
        "open"
    } else if row.refreshed_by_admin || (open.votes.is_empty() && !row.vote_closed_by_admin) {
        // A refresh clears the votes; teams kept by the vote keep them.
        "refreshed"
    } else {
        "kept"
    };
    let teams = ["hiddenking", "archmother"]
        .into_iter()
        .map(|side| {
            let mut players: Vec<(&MatchPlayerRow, Option<&PlayerRow>)> = open
                .players
                .iter()
                .filter(|p| p.side == side)
                .map(|p| (p, camp.players.iter().find(|c| c.user_id == p.user_id)))
                .collect();
            let role_order = |player: Option<&PlayerRow>| {
                player
                    .and_then(|p| p.role)
                    .and_then(|role| Role::ALL.iter().position(|r| *r == role))
                    .unwrap_or(Role::ALL.len())
            };
            players.sort_by_key(|(p, camp_player)| (!p.is_captain, role_order(*camp_player), p.user_id));
            OverlayTeamDto {
                side,
                refresh: open
                    .votes
                    .iter()
                    .filter(|v| v.refresh && open.players.iter().any(|p| p.user_id == v.user_id && p.side == side))
                    .count(),
                players: players
                    .into_iter()
                    .map(|(p, camp_player)| {
                        let profile = profiles.get(&(p.user_id as u64));
                        OverlayPlayerDto {
                            user_id: p.user_id.to_string(),
                            name: profile.map_or_else(|| p.user_id.to_string(), |profile| profile.name.clone()),
                            avatar_url: profile.map(|profile| profile.avatar_url.clone()).filter(|url| !url.is_empty()),
                            captain: p.is_captain,
                            s_rank: camp_player.is_some_and(|c| c.s_rank),
                            ss_rank: camp_player.is_some_and(|c| c.ss_rank),
                            role: camp_player.and_then(|c| c.role).map(Role::as_str),
                            voted: open.votes.iter().any(|v| v.user_id == p.user_id),
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    OverlayVoteDto {
        match_id: row.id.clone(),
        slot: row.slot,
        is_final: row.is_final,
        state,
        needed: camp.camp.layout().refresh_votes,
        refresh: open.votes.iter().filter(|v| v.refresh).count(),
        keep: open.votes.iter().filter(|v| !v.refresh).count(),
        voted: open.votes.len(),
        total: open.players.len(),
        teams,
    }
}

/// The refresh votes of the server's camp, for the vote overlay. Public: the link's secret is
/// the only key. Matches drop out once their draft starts.
pub async fn overlay_votes(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<OverlayVotesDto>, ApiError> {
    let guild_id = overlay_guild(&state, &token).await?;
    let Some(camp) = db::load_active_for_guild(&state.db, guild_id as i64).await? else {
        return Ok(Json(OverlayVotesDto { votes: Vec::new() }));
    };
    let voting: Vec<&db::OpenMatch> = camp
        .open_matches
        .iter()
        .filter(|m| m.row.status == "voting" && !m.row.draft_started())
        .collect();
    let user_ids: Vec<u64> = voting
        .iter()
        .flat_map(|m| m.players.iter().map(|p| p.user_id as u64))
        .collect();
    let profiles = if user_ids.is_empty() {
        HashMap::new()
    } else {
        state.link.profiles(guild_id, &user_ids).await
    };
    Ok(Json(OverlayVotesDto {
        votes: voting.into_iter().map(|m| overlay_vote_dto(&camp, m, &profiles)).collect(),
    }))
}

#[derive(Serialize)]
pub struct OverlayRankingDto {
    camp_id: Option<String>,
    /// Finished matches.
    match_count: i64,
    /// The camp ended: this is its final ranking.
    ended: bool,
    /// Best first.
    players: Vec<OverlayRankDto>,
}

#[derive(Serialize)]
struct OverlayRankDto {
    user_id: String,
    name: String,
    avatar_url: Option<String>,
    /// 1 is gold. Points first, then MVPs; equal on both shares the place.
    place: usize,
    points: i64,
    mvps: i64,
    wins: i64,
    played: i64,
    /// Places moved with the latest result, up positive. `null` until there's a result before
    /// it to compare with.
    moved: Option<i64>,
    /// Got points from the latest result.
    scored_last: bool,
    /// The points the latest result gave this player: the match's win or loss points.
    last_points: i64,
}

/// The camp whose ranking the overlay shows: the active one, else the latest that got going.
async fn ranking_camp(state: &AppState, guild_id: u64) -> Result<Option<CampState>, ApiError> {
    if let Some(camp) = db::load_active_for_guild(&state.db, guild_id as i64).await? {
        return Ok(Some(camp));
    }
    let latest = db::list_camps_for_guild(&state.db, guild_id as i64)
        .await?
        .into_iter()
        .find(|c| c.status != "cancelled_signup");
    match latest {
        Some(summary) => Ok(db::load_camp(&state.db, &summary.id).await?),
        None => Ok(None),
    }
}

/// The ranking of the server's camp, for the ranking overlay. Public: the link's secret is the
/// only key. After a camp ends, its final ranking, until another starts.
pub async fn overlay_ranking(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<OverlayRankingDto>, ApiError> {
    let guild_id = overlay_guild(&state, &token).await?;
    let Some(camp) = ranking_camp(&state, guild_id).await? else {
        return Ok(Json(OverlayRankingDto {
            camp_id: None,
            match_count: 0,
            ended: false,
            players: Vec::new(),
        }));
    };
    let stats = db::player_stats(&state.db, &camp.camp.id).await?;
    let recent = db::recent_matches(&state.db, &camp.camp.id, 2).await?;
    let standings: Vec<Standing> = camp
        .players
        .iter()
        .map(|p| Standing {
            user_id: p.user_id,
            points: p.points,
            matches: p.matches_played,
            mvps: stats.get(&p.user_id).map_or(0, |s| s.mvps),
            wins: stats.get(&p.user_id).map_or(0, |s| s.wins),
        })
        .collect();
    let gained: Vec<(i64, i64)> = recent
        .first()
        .map(|m| m.players.iter().map(|p| (p.user_id, m.points.for_side(p.side == m.winner_side))).collect())
        .unwrap_or_default();
    let winners: Vec<i64> = recent
        .first()
        .map(|m| m.players.iter().filter(|p| p.side == m.winner_side).map(|p| p.user_id).collect())
        .unwrap_or_default();
    let latest = LatestResult {
        gained: &gained,
        winners: &winners,
        mvp: recent.first().and_then(|m| m.mvp_user_id),
    };
    let user_ids: Vec<u64> = camp.players.iter().map(|p| p.user_id as u64).collect();
    let profiles = state.link.profiles(guild_id, &user_ids).await;
    let players = rank_with_movement(&standings, &latest)
        .into_iter()
        .map(|(standing, place, moved)| {
            let id = standing.user_id;
            let profile = profiles.get(&(id as u64));
            let player_stats = stats.get(&id).copied().unwrap_or_default();
            OverlayRankDto {
                user_id: id.to_string(),
                name: profile.map_or_else(|| id.to_string(), |p| p.name.clone()),
                avatar_url: profile.map(|p| p.avatar_url.clone()).filter(|url| !url.is_empty()),
                place,
                points: standing.points,
                mvps: standing.mvps,
                wins: player_stats.wins,
                played: player_stats.played,
                moved: (recent.len() >= 2).then_some(moved),
                last_points: gained.iter().find(|(user, _)| *user == id).map_or(0, |(_, p)| *p),
                scored_last: gained.iter().any(|(user, p)| *user == id && *p > 0),
            }
        })
        .collect();
    Ok(Json(OverlayRankingDto {
        camp_id: Some(camp.camp.id.clone()),
        match_count: camp.camp.match_count,
        ended: camp.camp.status == "ended",
        players,
    }))
}

/// Live updates for overlays: the same `change` events as the admin page, keyed by the link.
pub async fn overlay_events(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let guild_id = overlay_guild(&state, &token).await?;
    Ok(change_stream(&state, guild_id))
}

pub async fn guild_channels(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
) -> Result<Json<Vec<ChannelDto>>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    Ok(Json(
        state
            .link
            .text_channels(guild_id)
            .into_iter()
            .map(|c| ChannelDto {
                id: c.id.to_string(),
                name: c.name,
            })
            .collect(),
    ))
}

/// Every server member except bots, for adding players. The page filters them by name.
pub async fn guild_members(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
) -> Result<Json<Vec<ServerMemberDto>>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let members = state
        .link
        .members(guild_id)
        .await
        .map_err(|err| ApiError::Conflict(err.to_string()))?;
    Ok(Json(
        members
            .into_iter()
            .map(|(id, p)| ServerMemberDto {
                id: id.to_string(),
                name: p.name,
                avatar_url: p.avatar_url,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct CreateCampRequest {
    channel_id: String,
    /// `24`, `30`, `4-1v1` or `4-2v2`.
    size: String,
    /// A selection of the server's to fill the roster with.
    #[serde(default)]
    preset_id: Option<i64>,
}

pub async fn create_camp(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
    Json(request): Json<CreateCampRequest>,
) -> Result<(StatusCode, Json<CampDto>), ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let channel_id: u64 = request
        .channel_id
        .parse()
        .map_err(|_| ApiError::BadRequest("canal inválido".to_string()))?;
    let size = CampSize::parse(&request.size)
        .ok_or_else(|| ApiError::BadRequest("tamanho inválido".to_string()))?;
    if let Some(preset_id) = request.preset_id {
        if presets::load(&state.db, guild_id as i64, preset_id).await?.is_none() {
            return Err(ApiError::BadRequest("seleção não encontrada".to_string()));
        }
    }
    tracing::info!(user_id = session.user_id, guild_id, channel_id, ?size, preset = ?request.preset_id, "camp created from the web");
    let camp_id = state
        .link
        .create_camp(guild_id, channel_id, size)
        .await
        .map_err(|err| ApiError::Conflict(err.to_string()))?;
    if let Some(preset_id) = request.preset_id {
        // The camp exists either way; a failure here leaves its roster to fill by hand.
        if let Err(err) = state.link.act(guild_id, &camp_id, CampAction::ApplyPreset { preset_id }).await {
            return Err(ApiError::Conflict(format!("o camp foi criado, mas a seleção não entrou: {err}")));
        }
    }
    let camp = admin_camp(&state, &session, &camp_id).await?;
    Ok((StatusCode::CREATED, camp_response(&state, camp).await?))
}

#[derive(Serialize)]
pub struct PresetsDto {
    presets: Vec<PresetDto>,
    /// Every server member's main role, by user ID: what a new player of a selection starts with.
    roles: HashMap<String, &'static str>,
}

#[derive(Serialize)]
struct PresetDto {
    id: i64,
    name: String,
    players: Vec<PresetPlayerDto>,
    updated_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct PresetPlayerDto {
    user_id: String,
    /// `s`, `ss` or `null`.
    rank: Option<String>,
    /// `tank`, `carry`, `support` or `null`.
    role: Option<String>,
}

#[derive(Deserialize)]
pub struct PresetRequest {
    name: String,
    players: Vec<PresetPlayerDto>,
}

async fn presets_dto(state: &AppState, guild_id: u64) -> Result<PresetsDto, ApiError> {
    let presets = presets::list(&state.db, guild_id as i64).await?;
    let roles = presets::guild_roles(&state.db, guild_id as i64).await?;
    Ok(PresetsDto {
        presets: presets
            .into_iter()
            .map(|p| PresetDto {
                id: p.id,
                name: p.name,
                players: p
                    .players
                    .into_iter()
                    .map(|player| PresetPlayerDto {
                        user_id: player.user_id.to_string(),
                        rank: player.rank.map(|r| r.as_str().to_string()),
                        role: player.role.map(|r| r.as_str().to_string()),
                    })
                    .collect(),
                updated_at: p.updated_at,
            })
            .collect(),
        roles: roles.into_iter().map(|(id, role)| (id.to_string(), role.as_str())).collect(),
    })
}

fn preset_players(players: &[PresetPlayerDto]) -> Result<Vec<presets::PresetPlayer>, ApiError> {
    players
        .iter()
        .map(|p| {
            Ok(presets::PresetPlayer {
                user_id: p
                    .user_id
                    .parse()
                    .map_err(|_| ApiError::BadRequest(format!("ID de usuário inválido: {}", p.user_id)))?,
                rank: p
                    .rank
                    .as_deref()
                    .map(|name| presets::Rank::parse(name).ok_or_else(|| ApiError::BadRequest(format!("categoria inválida: {name}"))))
                    .transpose()?,
                role: p
                    .role
                    .as_deref()
                    .map(|name| Role::parse(name).ok_or_else(|| ApiError::BadRequest(format!("função inválida: {name}"))))
                    .transpose()?,
            })
        })
        .collect()
}

/// The server's selections of players, to fill a new camp's roster.
pub async fn guild_presets(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
) -> Result<Json<PresetsDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    Ok(Json(presets_dto(&state, guild_id).await?))
}

pub async fn create_preset(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
    Json(request): Json<PresetRequest>,
) -> Result<Json<PresetsDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let players = preset_players(&request.players)?;
    presets::save(&state.db, guild_id as i64, None, &request.name, &players)
        .await
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;
    Ok(Json(presets_dto(&state, guild_id).await?))
}

pub async fn save_preset(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path((guild_id, preset_id)): Path<(u64, i64)>,
    Json(request): Json<PresetRequest>,
) -> Result<Json<PresetsDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let players = preset_players(&request.players)?;
    presets::save(&state.db, guild_id as i64, Some(preset_id), &request.name, &players)
        .await
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;
    Ok(Json(presets_dto(&state, guild_id).await?))
}

pub async fn delete_preset(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path((guild_id, preset_id)): Path<(u64, i64)>,
) -> Result<Json<PresetsDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    presets::delete(&state.db, guild_id as i64, preset_id)
        .await
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;
    Ok(Json(presets_dto(&state, guild_id).await?))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideRequest {
    Hiddenking,
    Archmother,
}

/// The JSON body of `POST /api/camps/{id}/actions`, e.g. `{"type": "start_draft", "match_id": "R4T8WN"}`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionRequest {
    /// Points each winner and each loser earns; 1 and 0 when left out.
    CreateMatch {
        #[serde(default = "default_win_points")]
        win_points: i64,
        #[serde(default)]
        loss_points: i64,
    },
    EnterFinals,
    LeaveFinals,
    CloseVote { match_id: String },
    RefreshTeams { match_id: String },
    StartDraft { match_id: String },
    StartMatch { match_id: String },
    ReportWinner {
        match_id: String,
        side: SideRequest,
        #[serde(default)]
        mvp_user_id: Option<String>,
    },
    CorrectWinner { match_id: String },
    ChangeMvp { match_id: String, user_id: String },
    ReopenMatch { match_id: String },
    CancelMatch { match_id: String },
    AddPlayers { user_ids: Vec<String> },
    ApplyPreset { preset_id: i64 },
    RemovePlayer { user_id: String },
    RestorePlayer { user_id: String },
    SetSRank { user_id: String, s_rank: bool },
    SetSsRank { user_id: String, ss_rank: bool },
    /// `tank`, `carry`, `support`, or `null` to clear it.
    SetRole { user_id: String, role: Option<String> },
    /// Points to add, or below zero to take.
    AdjustPoints { user_id: String, delta: i64 },
    StartQueue,
    /// The whole queue in its new order.
    ReorderQueue { user_ids: Vec<String> },
    RemindVoters { match_id: String },
    CancelCamp,
    ResumeCamp,
    EndCamp,
}

fn default_win_points() -> i64 {
    MatchPoints::default().win
}

fn match_points(win: i64, loss: i64) -> Result<MatchPoints, ApiError> {
    MatchPoints { win, loss }
        .check()
        .map_err(|e| ApiError::BadRequest(e.to_string()))
}

fn user_id(id: &str) -> Result<u64, ApiError> {
    id.parse()
        .map_err(|_| ApiError::BadRequest(format!("ID de usuário inválido: {id}")))
}

impl TryFrom<ActionRequest> for CampAction {
    type Error = ApiError;

    fn try_from(request: ActionRequest) -> Result<Self, ApiError> {
        Ok(match request {
            ActionRequest::CreateMatch { win_points, loss_points } => CampAction::CreateMatch {
                points: match_points(win_points, loss_points)?,
            },
            ActionRequest::EnterFinals => CampAction::EnterFinals,
            ActionRequest::LeaveFinals => CampAction::LeaveFinals,
            ActionRequest::CloseVote { match_id } => CampAction::CloseVote { match_id },
            ActionRequest::RefreshTeams { match_id } => CampAction::RefreshTeams { match_id },
            ActionRequest::StartDraft { match_id } => CampAction::StartDraft { match_id },
            ActionRequest::StartMatch { match_id } => CampAction::StartMatch { match_id },
            ActionRequest::ReportWinner { match_id, side, mvp_user_id } => CampAction::ReportWinner {
                match_id,
                side: match side {
                    SideRequest::Hiddenking => "hiddenking",
                    SideRequest::Archmother => "archmother",
                }
                .to_string(),
                mvp_user_id: mvp_user_id.as_deref().map(user_id).transpose()?,
            },
            ActionRequest::CorrectWinner { match_id } => CampAction::CorrectWinner { match_id },
            ActionRequest::ChangeMvp { match_id, user_id: id } => CampAction::ChangeMvp {
                match_id,
                user_id: user_id(&id)?,
            },
            ActionRequest::ReopenMatch { match_id } => CampAction::ReopenMatch { match_id },
            ActionRequest::CancelMatch { match_id } => CampAction::CancelMatch { match_id },
            ActionRequest::AddPlayers { user_ids } => CampAction::AddPlayers {
                user_ids: user_ids.iter().map(|id| user_id(id)).collect::<Result<_, _>>()?,
            },
            ActionRequest::ApplyPreset { preset_id } => CampAction::ApplyPreset { preset_id },
            ActionRequest::RemovePlayer { user_id: id } => CampAction::RemovePlayer { user_id: user_id(&id)? },
            ActionRequest::RestorePlayer { user_id: id } => CampAction::RestorePlayer { user_id: user_id(&id)? },
            ActionRequest::SetSRank { user_id: id, s_rank } => CampAction::SetSRank {
                user_id: user_id(&id)?,
                s_rank,
            },
            ActionRequest::SetSsRank { user_id: id, ss_rank } => CampAction::SetSsRank {
                user_id: user_id(&id)?,
                ss_rank,
            },
            ActionRequest::SetRole { user_id: id, role } => CampAction::SetRole {
                user_id: user_id(&id)?,
                role: role
                    .map(|name| Role::parse(&name).ok_or_else(|| ApiError::BadRequest(format!("função inválida: {name}"))))
                    .transpose()?,
            },
            ActionRequest::AdjustPoints { user_id: id, delta } => CampAction::AdjustPoints {
                user_id: user_id(&id)?,
                delta,
            },
            ActionRequest::StartQueue => CampAction::StartQueue,
            ActionRequest::ReorderQueue { user_ids } => CampAction::ReorderQueue {
                user_ids: user_ids.iter().map(|id| user_id(id)).collect::<Result<_, _>>()?,
            },
            ActionRequest::RemindVoters { match_id } => CampAction::RemindVoters { match_id },
            ActionRequest::CancelCamp => CampAction::CancelCamp,
            ActionRequest::ResumeCamp => CampAction::ResumeCamp,
            ActionRequest::EndCamp => CampAction::EndCamp,
        })
    }
}

/// Runs an admin action through the bot, which also updates Discord, and answers with the camp
/// as it is afterwards.
pub async fn act(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(camp_id): Path<String>,
    Json(request): Json<ActionRequest>,
) -> Result<Json<CampDto>, ApiError> {
    let camp = admin_camp(&state, &session, &camp_id).await?;
    let action = CampAction::try_from(request)?;
    tracing::info!(user_id = session.user_id, camp = %camp.camp.id, ?action, "admin action from the web");
    if let Err(err) = state
        .link
        .act(camp.camp.guild_id as u64, &camp.camp.id, action)
        .await
    {
        return Err(ApiError::Conflict(err.to_string()));
    }
    let camp = admin_camp(&state, &session, &camp.camp.id).await?;
    camp_response(&state, camp).await
}

pub async fn match_info(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(match_id): Path<String>,
) -> Result<Json<MatchDto>, ApiError> {
    let match_id = match_id.trim().to_uppercase();
    let guild_id = db::match_guild_id(&state.db, &match_id)
        .await?
        .filter(|g| session.is_admin_of(*g as u64))
        .ok_or(ApiError::NotFound)?;
    let info = db::load_match_info(&state.db, &match_id, guild_id).await?;
    let camp = db::load_camp(&state.db, &info.camp_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(match_dto(&camp.camp, &info.row, &info.players, &[])))
}

/// A server's settings as the Configurações screen edits them, with the choices it offers.
#[derive(Serialize)]
pub struct SettingsDto {
    queue_voice_id: Option<String>,
    match1_hiddenking_voice_id: Option<String>,
    match1_archmother_voice_id: Option<String>,
    match2_hiddenking_voice_id: Option<String>,
    match2_archmother_voice_id: Option<String>,
    create_lobby: bool,
    server_region: Option<String>,
    use_draft_tool: bool,
    draft_timer_seconds: u32,
    draft_reserve_seconds: u32,
    draft_bans_per_team: u32,
    /// `cinematic` or `competitive`.
    draft_view: &'static str,
    dm_captains: bool,
    /// False until an admin saves this server's settings: until then it runs on the `.env` values.
    saved: bool,
    voice_channels: Vec<ChannelDto>,
    draft_timers: Vec<u32>,
    draft_reserves: Vec<u32>,
    max_draft_bans: u32,
}

/// The body of `PUT /api/guilds/{id}/settings`. A voice channel left out is found by its name.
#[derive(Deserialize)]
pub struct SettingsRequest {
    queue_voice_id: Option<String>,
    match1_hiddenking_voice_id: Option<String>,
    match1_archmother_voice_id: Option<String>,
    match2_hiddenking_voice_id: Option<String>,
    match2_archmother_voice_id: Option<String>,
    create_lobby: bool,
    server_region: Option<String>,
    /// Missing in pages from before the switch existed: the tool stays on.
    #[serde(default = "yes")]
    use_draft_tool: bool,
    draft_timer_seconds: u32,
    /// The options below are missing in pages from before they existed: the defaults stay.
    #[serde(default = "default_reserve")]
    draft_reserve_seconds: u32,
    #[serde(default = "default_bans")]
    draft_bans_per_team: u32,
    #[serde(default)]
    draft_view: Option<String>,
    #[serde(default = "yes")]
    dm_captains: bool,
}

fn yes() -> bool {
    true
}

fn default_reserve() -> u32 {
    90
}

fn default_bans() -> u32 {
    2
}

const MAX_REGION_LEN: usize = 40;

fn settings_dto(settings: GuildSettings, saved: bool, voice: Vec<Channel>) -> SettingsDto {
    let id = |v: Option<u64>| v.map(|v| v.to_string());
    SettingsDto {
        queue_voice_id: id(settings.queue_voice_id),
        match1_hiddenking_voice_id: id(settings.match1_hiddenking_voice_id),
        match1_archmother_voice_id: id(settings.match1_archmother_voice_id),
        match2_hiddenking_voice_id: id(settings.match2_hiddenking_voice_id),
        match2_archmother_voice_id: id(settings.match2_archmother_voice_id),
        create_lobby: settings.create_lobby,
        server_region: settings.server_region,
        use_draft_tool: settings.use_draft_tool,
        draft_timer_seconds: settings.draft_timer_seconds,
        draft_reserve_seconds: settings.draft_reserve_seconds,
        draft_bans_per_team: settings.draft_bans_per_team,
        draft_view: settings.draft_view.as_str(),
        dm_captains: settings.dm_captains,
        saved,
        voice_channels: voice
            .into_iter()
            .map(|c| ChannelDto {
                id: c.id.to_string(),
                name: c.name,
            })
            .collect(),
        draft_timers: DRAFT_TIMERS.to_vec(),
        draft_reserves: DRAFT_RESERVES.to_vec(),
        max_draft_bans: MAX_DRAFT_BANS,
    }
}

/// Checks a settings form against the server's voice channels.
fn parse_settings(request: SettingsRequest, voice: &[Channel]) -> Result<GuildSettings, ApiError> {
    let channel = |id: Option<String>| -> Result<Option<u64>, ApiError> {
        let Some(id) = id.map(|id| id.trim().to_string()).filter(|id| !id.is_empty()) else {
            return Ok(None);
        };
        match id.parse::<u64>() {
            Ok(id) if voice.iter().any(|c| c.id == id) => Ok(Some(id)),
            _ => Err(ApiError::BadRequest(
                "Um dos canais de voz não existe neste servidor. Escolha outro ou deixe automático.".to_string(),
            )),
        }
    };
    if !DRAFT_TIMERS.contains(&request.draft_timer_seconds) {
        return Err(ApiError::BadRequest("tempo de draft inválido".to_string()));
    }
    if !DRAFT_RESERVES.contains(&request.draft_reserve_seconds) {
        return Err(ApiError::BadRequest("tempo de reserva do draft inválido".to_string()));
    }
    if request.draft_bans_per_team > MAX_DRAFT_BANS {
        return Err(ApiError::BadRequest(format!(
            "o DeadChaps permite no máximo {MAX_DRAFT_BANS} bans por time"
        )));
    }
    let draft_view = match request.draft_view.as_deref() {
        None => DraftView::Cinematic,
        Some(name) => DraftView::parse(name)
            .ok_or_else(|| ApiError::BadRequest(format!("visualização de draft inválida: {name}")))?,
    };
    let server_region = request
        .server_region
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty());
    if server_region.as_ref().is_some_and(|r| r.chars().count() > MAX_REGION_LEN) {
        return Err(ApiError::BadRequest("região muito longa".to_string()));
    }
    Ok(GuildSettings {
        queue_voice_id: channel(request.queue_voice_id)?,
        match1_hiddenking_voice_id: channel(request.match1_hiddenking_voice_id)?,
        match1_archmother_voice_id: channel(request.match1_archmother_voice_id)?,
        match2_hiddenking_voice_id: channel(request.match2_hiddenking_voice_id)?,
        match2_archmother_voice_id: channel(request.match2_archmother_voice_id)?,
        create_lobby: request.create_lobby,
        server_region,
        use_draft_tool: request.use_draft_tool,
        draft_timer_seconds: request.draft_timer_seconds,
        draft_reserve_seconds: request.draft_reserve_seconds,
        draft_bans_per_team: request.draft_bans_per_team,
        draft_view,
        dm_captains: request.dm_captains,
    })
}

pub async fn guild_settings(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
) -> Result<Json<SettingsDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let (settings, saved) = db::guild_settings(&state.db, guild_id as i64, &state.default_settings).await?;
    Ok(Json(settings_dto(settings, saved, state.link.voice_channels(guild_id))))
}

/// Takes effect on the next action: matches already running keep their draft and lobby.
pub async fn save_guild_settings(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
    Json(request): Json<SettingsRequest>,
) -> Result<Json<SettingsDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let voice = state.link.voice_channels(guild_id);
    let settings = parse_settings(request, &voice)?;
    tracing::info!(user_id = session.user_id, guild_id, ?settings, "server settings saved from the web");
    db::save_guild_settings(&state.db, guild_id as i64, &settings).await?;
    // The camp shows whether drafts use the draft tool.
    state.link.changes().notify(guild_id);
    Ok(Json(settings_dto(settings, true, voice)))
}
