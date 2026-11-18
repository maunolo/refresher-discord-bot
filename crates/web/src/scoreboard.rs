//! Each finished match's Deadlock scoreboard on the admin page: how its lookup is going, the
//! match ID an admin gives when the bot can't find it, and which member plays on each Steam
//! account, so the public site shows Discord names.

use axum::extract::{Path, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use refresher_core::db;
use refresher_core::stats::{self, TEAM_SIDES};
use serde::{Deserialize, Serialize};

use crate::api::managed_guild;
use crate::auth::Admin;
use crate::error::ApiError;
use crate::session::Session;
use crate::AppState;

#[derive(Serialize)]
pub struct MatchStatsDto {
    /// `pending` (the bot is looking), `needs_id`, `ready`, `failed`, or `none` before the match ends.
    status: String,
    deadlock_match_id: Option<String>,
    /// Tries so far at the current step.
    attempts: i64,
    /// When the bot tries again, in Unix seconds.
    next_attempt_at: Option<i64>,
    error: Option<String>,
    scoreboard: Option<ScoreboardDto>,
}

#[derive(Serialize)]
struct ScoreboardDto {
    duration_s: i64,
    /// `hiddenking` or `archmother`, as Deadlock recorded it.
    winner_side: &'static str,
    players: Vec<StatsPlayerDto>,
}

#[derive(Serialize)]
struct StatsPlayerDto {
    account_id: String,
    side: &'static str,
    steam_name: Option<String>,
    hero: String,
    hero_image: Option<String>,
    kills: i64,
    deaths: i64,
    assists: i64,
    souls: i64,
    /// The member linked to the Steam account, if any.
    user_id: Option<String>,
}

/// A match in a server the admin manages: its ID, server and whether it's finished. Else 404.
async fn admin_match(state: &AppState, session: &Session, match_id: &str) -> Result<(String, i64, bool), ApiError> {
    let match_id = match_id.trim().to_uppercase();
    let guild_id = db::match_guild_id(&state.db, &match_id)
        .await?
        .filter(|g| session.is_admin_of(*g as u64))
        .ok_or(ApiError::NotFound)?;
    let info = db::load_match_info(&state.db, &match_id, guild_id).await?;
    Ok((match_id, guild_id, info.row.status == "completed"))
}

async fn stats_dto(state: &AppState, match_id: &str, guild_id: i64, finished: bool) -> Result<MatchStatsDto, ApiError> {
    let row = stats::load(&state.db, match_id).await?;
    let Some(row) = row else {
        // Matches that ended before the bot looked scoreboards up wait for an admin too.
        let status = if finished { stats::NEEDS_ID } else { "none" };
        return Ok(MatchStatsDto {
            status: status.to_string(),
            deadlock_match_id: None,
            attempts: 0,
            next_attempt_at: None,
            error: None,
            scoreboard: None,
        });
    };
    let links = stats::account_links(&state.db, guild_id).await?;
    let scoreboard = row.scoreboard.map(|board| ScoreboardDto {
        duration_s: board.duration_s,
        winner_side: TEAM_SIDES[board.winning_team.min(1)],
        players: board
            .players
            .into_iter()
            .map(|p| StatsPlayerDto {
                account_id: p.account_id.to_string(),
                side: TEAM_SIDES[p.team.min(1)],
                user_id: links.get(&p.account_id).map(|u| u.to_string()),
                steam_name: p.steam_name,
                hero: p.hero,
                hero_image: p.hero_image,
                kills: p.kills,
                deaths: p.deaths,
                assists: p.assists,
                souls: p.souls,
            })
            .collect(),
    });
    Ok(MatchStatsDto {
        status: row.status,
        deadlock_match_id: row.deadlock_match_id.map(|id| id.to_string()),
        attempts: row.attempts,
        next_attempt_at: row.next_attempt_at,
        error: row.last_error,
        scoreboard,
    })
}

pub async fn match_stats(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(match_id): Path<String>,
) -> Result<Json<MatchStatsDto>, ApiError> {
    let (match_id, guild_id, finished) = admin_match(&state, &session, &match_id).await?;
    Ok(Json(stats_dto(&state, &match_id, guild_id, finished).await?))
}

#[derive(Deserialize)]
pub struct DeadlockIdRequest {
    /// As a string: Deadlock IDs don't all fit in a JavaScript number.
    deadlock_match_id: String,
}

/// An admin gives (or corrects) the match's Deadlock ID; the bot looks it up within seconds.
/// Sending the same ID again retries a lookup that failed.
pub async fn set_deadlock_id(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(match_id): Path<String>,
    Json(request): Json<DeadlockIdRequest>,
) -> Result<Json<MatchStatsDto>, ApiError> {
    let (match_id, guild_id, finished) = admin_match(&state, &session, &match_id).await?;
    if !finished {
        return Err(ApiError::Conflict("A partida ainda não terminou.".to_string()));
    }
    let id = request
        .deadlock_match_id
        .trim()
        .trim_start_matches('#')
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| ApiError::BadRequest("O ID da partida é um número, como 139086680.".to_string()))?;
    stats::set_deadlock_id(&state.db, &match_id, id).await?;
    tracing::info!(user_id = session.user_id, match_id, deadlock_match_id = id, "Deadlock match ID set from the web");
    state.link.changes().notify(guild_id as u64);
    Ok(Json(stats_dto(&state, &match_id, guild_id, finished).await?))
}

#[derive(Deserialize)]
pub struct AccountLinkRequest {
    /// The member who plays on the account, or `null` to unlink it.
    user_id: Option<String>,
}

/// Says which member plays on a Steam account, for every match of the server.
pub async fn set_account_link(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path((guild_id, account_id)): Path<(u64, u64)>,
    Json(request): Json<AccountLinkRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let user_id = match request.user_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(id) => Some(
            id.parse::<i64>()
                .map_err(|_| ApiError::BadRequest("membro inválido".to_string()))?,
        ),
        None => None,
    };
    stats::set_account_link(&state.db, guild_id as i64, account_id, user_id).await?;
    state.link.changes().notify(guild_id);
    Ok(Json(serde_json::json!({})))
}

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/matches/{match_id}/stats", get(match_stats).put(set_deadlock_id))
        .route("/guilds/{guild_id}/deadlock-accounts/{account_id}", put(set_account_link))
}
