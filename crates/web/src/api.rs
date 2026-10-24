//! Read-only JSON for the admin page. Discord IDs go out as strings: they don't fit in a
//! JavaScript number.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use refresher_core::db::{self, CampState, CampSummary, MatchPlayerRow, MatchRow, PlayerRow, RecentMatch, RefreshVote};
use refresher_core::link::{CampAction, Profile};
use refresher_core::queue::{self, CampSize};
use serde::{Deserialize, Serialize};

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
}

#[derive(Serialize)]
pub struct ChannelDto {
    id: String,
    name: String,
}

#[derive(Serialize)]
pub struct MemberSearchDto {
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
    /// The bot's rule: only the latest result, and only while its players are all back in the queue.
    correctable: bool,
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
}

#[derive(Serialize)]
pub struct MatchDto {
    id: String,
    camp_id: String,
    slot: i64,
    status: String,
    is_final: bool,
    refresh_used: bool,
    winner_side: Option<String>,
    party_code: Option<String>,
    draft: Option<DraftDto>,
    players: Vec<MatchPlayerDto>,
    votes: Vec<VoteDto>,
}

/// Admins see every link, team links included, so they can help a captain whose DM failed.
#[derive(Serialize)]
struct DraftDto {
    spectator_url: String,
    hiddenking_url: Option<String>,
    archmother_url: Option<String>,
    admin_url: Option<String>,
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

fn match_dto(camp_id: &str, row: &MatchRow, players: &[MatchPlayerRow], votes: &[RefreshVote]) -> MatchDto {
    MatchDto {
        id: row.id.clone(),
        camp_id: camp_id.to_string(),
        slot: row.slot,
        status: row.status.clone(),
        is_final: row.is_final,
        refresh_used: row.refresh_used,
        winner_side: row.winner_side.clone(),
        party_code: row.party_code.clone(),
        draft: row.draft_url.clone().map(|spectator_url| DraftDto {
            spectator_url,
            hiddenking_url: row.draft_hiddenking_url.clone(),
            archmother_url: row.draft_archmother_url.clone(),
            admin_url: row.draft_admin_url.clone(),
        }),
        players: players
            .iter()
            .map(|p| MatchPlayerDto {
                user_id: p.user_id.to_string(),
                side: p.side.clone(),
                is_captain: p.is_captain,
                points_after: p.points_after,
            })
            .collect(),
        votes: votes
            .iter()
            .map(|v| VoteDto {
                user_id: v.user_id.to_string(),
                refresh: v.refresh,
            })
            .collect(),
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
            correctable: i == 0
                && state.camp.status == "running"
                && m.players.iter().all(|p| queued(p.user_id)),
        })
        .collect()
}

fn camp_dto(
    state: &CampState,
    recent: &[RecentMatch],
    profiles: &HashMap<u64, Profile>,
    queue_voice: Vec<u64>,
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
            .map(|p| PlayerDto {
                user_id: p.user_id.to_string(),
                points: p.points,
                location: p.location.clone(),
                queue_position: p.queue_position,
                consecutive_matches: p.consecutive_matches,
                s_rank: p.s_rank,
            })
            .collect(),
        up_next: up_next(state),
        open_matches: state
            .open_matches
            .iter()
            .map(|m| match_dto(&state.camp.id, &m.row, &m.players, &m.votes))
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
    }
}

fn summary_dto(camp: CampSummary) -> CampSummaryDto {
    CampSummaryDto {
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
    Ok(Json(camps.into_iter().map(summary_dto).collect()))
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

async fn camp_response(state: &AppState, camp: CampState) -> Result<Json<CampDto>, ApiError> {
    let recent = db::recent_matches(&state.db, &camp.camp.id, 5).await?;
    let user_ids: Vec<u64> = camp.players.iter().map(|p| p.user_id as u64).collect();
    let profiles = state
        .link
        .profiles(camp.camp.guild_id as u64, &user_ids)
        .await;
    let queue_voice = if camp.camp.status == "signup" {
        state.link.queue_voice_members(camp.camp.guild_id as u64)
    } else {
        Vec::new()
    };
    Ok(Json(camp_dto(&camp, &recent, &profiles, queue_voice)))
}

/// A server the admin manages and the bot is in, or 404.
fn managed_guild(state: &AppState, session: &Session, guild_id: u64) -> Result<(), ApiError> {
    if session.is_admin_of(guild_id) && state.link.in_guild(guild_id) {
        Ok(())
    } else {
        Err(ApiError::NotFound)
    }
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

#[derive(Deserialize)]
pub struct MemberQuery {
    query: String,
}

/// Server members whose name starts with `query`, for adding players.
pub async fn search_members(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
    Query(params): Query<MemberQuery>,
) -> Result<Json<Vec<MemberSearchDto>>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let query = params.query.trim();
    if query.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let members = state.link.search_members(guild_id, query, 10).await?;
    Ok(Json(
        members
            .into_iter()
            .map(|(id, p)| MemberSearchDto {
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
    StartDraft { match_id: String },
    StartMatch { match_id: String },
    ReportWinner { match_id: String, side: SideRequest },
    CorrectWinner { match_id: String },
    ReopenMatch { match_id: String },
    AddPlayers { user_ids: Vec<String> },
    RemovePlayer { user_id: String },
    SetSRank { user_id: String, s_rank: bool },
    StartQueue,
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
            ActionRequest::StartDraft { match_id } => CampAction::StartDraft { match_id },
            ActionRequest::StartMatch { match_id } => CampAction::StartMatch { match_id },
            ActionRequest::ReportWinner { match_id, side } => CampAction::ReportWinner {
                match_id,
                side: match side {
                    SideRequest::Hiddenking => "hiddenking",
                    SideRequest::Archmother => "archmother",
                }
                .to_string(),
            },
            ActionRequest::CorrectWinner { match_id } => CampAction::CorrectWinner { match_id },
            ActionRequest::ReopenMatch { match_id } => CampAction::ReopenMatch { match_id },
            ActionRequest::AddPlayers { user_ids } => CampAction::AddPlayers {
                user_ids: user_ids.iter().map(|id| user_id(id)).collect::<Result<_, _>>()?,
            },
            ActionRequest::RemovePlayer { user_id: id } => CampAction::RemovePlayer { user_id: user_id(&id)? },
            ActionRequest::SetSRank { user_id: id, s_rank } => CampAction::SetSRank {
                user_id: user_id(&id)?,
                s_rank,
            },
            ActionRequest::StartQueue => CampAction::StartQueue,
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
    Ok(Json(match_dto(&info.camp_id, &info.row, &info.players, &[])))
}
