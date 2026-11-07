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
    self, CampRow, CampState, CampSummary, MatchPlayerRow, MatchRow, PlayerRow, PlayerStats, RecentMatch,
    RefreshVote,
};
use refresher_core::draft;
use refresher_core::link::{CampAction, Channel, Profile};
use refresher_core::queue::{self, CampSize, Role};
use refresher_core::settings::{GuildSettings, DRAFT_TIMERS};
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
    has_final: bool,
    can_create_match: bool,
    can_create_final: bool,
    /// Refresh votes one team needs to re-sort a match.
    refresh_votes: usize,
    players: Vec<PlayerDto>,
    /// Who the next match would take from the queue, by the bot's seating rule.
    up_next: Vec<String>,
    open_matches: Vec<MatchDto>,
    recent_matches: Vec<RecentMatchDto>,
    /// Name and avatar per user ID, for everyone on the roster Discord could find.
    members: HashMap<String, MemberDto>,
    /// During signup: who is in the Queue voice channel right now, roster or not.
    queue_voice: Vec<String>,
    /// The camp's message in Discord (signup, then queue), else its channel.
    discord_url: String,
    /// The server creates drafts on the draft tool; off, admins share a draft link themselves.
    draft_tool: bool,
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
    consecutive_matches: i64,
    s_rank: bool,
    /// `tank`, `carry` or `support`; kept per server.
    role: Option<&'static str>,
    /// Finished matches only.
    matches_played: i64,
    wins: i64,
    /// Points admins added (or, below zero, took) by hand.
    points_adjusted: i64,
    /// Finished matches as MVP: the tie-breaker for equal points.
    mvps: i64,
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

fn up_next(state: &CampState) -> Vec<String> {
    let mut queued: Vec<&PlayerRow> = state
        .players
        .iter()
        .filter(|p| p.location == "queued")
        .collect();
    queued.sort_by_key(|p| p.queue_position.unwrap_or(i64::MAX));
    let seats: Vec<queue::Seat> = queued.iter().map(|p| p.seat()).collect();
    queue::pick_match_seats(&seats, state.camp.layout().match_size)
        .iter()
        .map(|s| s.user_id.to_string())
        .collect()
}

fn recent_dto(state: &CampState, recent: &[RecentMatch]) -> Vec<RecentMatchDto> {
    let queued = |user_id: i64| {
        state
            .players
            .iter()
            .any(|p| p.user_id == user_id && p.location == "queued")
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

fn camp_dto(
    state: &CampState,
    recent: &[RecentMatch],
    stats: &HashMap<i64, PlayerStats>,
    profiles: &HashMap<u64, Profile>,
    queue_voice: Vec<u64>,
    draft_tool: bool,
) -> CampDto {
    CampDto {
        id: state.camp.id.clone(),
        guild_id: state.camp.guild_id.to_string(),
        channel_id: state.camp.channel_id.to_string(),
        status: state.camp.status.clone(),
        target_size: state.camp.target_size,
        match_size: state.camp.match_size,
        match_count: state.camp.match_count,
        has_final: state.has_final,
        can_create_match: state.can_create_match(),
        can_create_final: state.can_create_final(),
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
                    consecutive_matches: p.consecutive_matches,
                    s_rank: p.s_rank,
                    role: p.role.map(Role::as_str),
                    matches_played: stats.played,
                    wins: stats.wins,
                    points_adjusted: stats.adjusted,
                    mvps: stats.mvps,
                }
            })
            .collect(),
        up_next: up_next(state),
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
        draft_tool,
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
async fn admin_camp(state: &AppState, session: &Session, camp_id: &str) -> Result<CampState, ApiError> {
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
    let queue_voice = if camp.camp.status == "signup" {
        state.link.queue_voice_members(camp.camp.guild_id as u64).await
    } else {
        Vec::new()
    };
    let (settings, _) = db::guild_settings(&state.db, camp.camp.guild_id, &state.default_settings).await?;
    Ok(Json(camp_dto(&camp, &recent, &stats, &profiles, queue_voice, settings.use_draft_tool)))
}

/// A server the admin manages and the bot is in, or 404.
fn managed_guild(state: &AppState, session: &Session, guild_id: u64) -> Result<(), ApiError> {
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
    Ok(Sse::new(events).keep_alive(KeepAlive::default()))
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
    tracing::info!(user_id = session.user_id, guild_id, channel_id, ?size, "camp created from the web");
    let camp_id = state
        .link
        .create_camp(guild_id, channel_id, size)
        .await
        .map_err(|err| ApiError::Conflict(err.to_string()))?;
    let camp = admin_camp(&state, &session, &camp_id).await?;
    Ok((StatusCode::CREATED, camp_response(&state, camp).await?))
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
    CreateMatch,
    CreateFinal,
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
    AddPlayers { user_ids: Vec<String> },
    RemovePlayer { user_id: String },
    SetSRank { user_id: String, s_rank: bool },
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

fn user_id(id: &str) -> Result<u64, ApiError> {
    id.parse()
        .map_err(|_| ApiError::BadRequest(format!("ID de usuário inválido: {id}")))
}

impl TryFrom<ActionRequest> for CampAction {
    type Error = ApiError;

    fn try_from(request: ActionRequest) -> Result<Self, ApiError> {
        Ok(match request {
            ActionRequest::CreateMatch => CampAction::CreateMatch,
            ActionRequest::CreateFinal => CampAction::CreateFinal,
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
            ActionRequest::AddPlayers { user_ids } => CampAction::AddPlayers {
                user_ids: user_ids.iter().map(|id| user_id(id)).collect::<Result<_, _>>()?,
            },
            ActionRequest::RemovePlayer { user_id: id } => CampAction::RemovePlayer { user_id: user_id(&id)? },
            ActionRequest::SetSRank { user_id: id, s_rank } => CampAction::SetSRank {
                user_id: user_id(&id)?,
                s_rank,
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
    /// False until an admin saves this server's settings: until then it runs on the `.env` values.
    saved: bool,
    voice_channels: Vec<ChannelDto>,
    draft_timers: Vec<u32>,
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
}

fn yes() -> bool {
    true
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
        saved,
        voice_channels: voice
            .into_iter()
            .map(|c| ChannelDto {
                id: c.id.to_string(),
                name: c.name,
            })
            .collect(),
        draft_timers: DRAFT_TIMERS.to_vec(),
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
