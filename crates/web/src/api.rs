//! Read-only JSON for the admin page. Discord IDs go out as strings: they don't fit in a
//! JavaScript number.

use axum::extract::{Path, State};
use axum::Json;
use refresher_core::db::{self, CampState, CampSummary, MatchPlayerRow, MatchRow, RefreshVote};
use serde::Serialize;

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
    players: Vec<PlayerDto>,
    open_matches: Vec<MatchDto>,
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

fn camp_dto(state: &CampState) -> CampDto {
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
        open_matches: state
            .open_matches
            .iter()
            .map(|m| match_dto(&state.camp.id, &m.row, &m.players, &m.votes))
            .collect(),
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
            .filter(|g| state.bot_guilds.contains(g.id))
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

pub async fn camp(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(camp_id): Path<String>,
) -> Result<Json<CampDto>, ApiError> {
    let camp = db::load_camp(&state.db, &camp_id.trim().to_uppercase())
        .await?
        .filter(|c| session.is_admin_of(c.camp.guild_id as u64))
        .ok_or(ApiError::NotFound)?;
    Ok(Json(camp_dto(&camp)))
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
