//! Replay clips (refresher_core::clips): admins upload videos straight from the browser to the
//! R2 bucket with a presigned link, then play them on a stream's replay overlay. The videos never
//! pass through this server; it only hands out links that expire.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use refresher_core::clips::{self, Clip, Play};
use refresher_core::r2::R2Config;
use serde::{Deserialize, Serialize};

use crate::api::{managed_guild, overlay_guild};
use crate::auth::Admin;
use crate::error::ApiError;
use crate::AppState;

/// How long an upload link works.
const UPLOAD_LINK_SECS: u64 = 60 * 60;
/// How long a playback link works: longer than a camp night, so a paused overlay never loses it.
const PLAY_LINK_SECS: u64 = 12 * 60 * 60;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/guilds/{guild_id}/clips", get(list_clips).post(new_clip))
        .route("/guilds/{guild_id}/clips/{clip_id}/ready", post(clip_ready))
        .route("/guilds/{guild_id}/clips/{clip_id}/play", post(play_clip))
        .route("/guilds/{guild_id}/clips/{clip_id}/delete", post(delete_clip))
        .route("/guilds/{guild_id}/replay/stop", post(stop_replay))
        .route("/overlay/{token}/replay/{stream}", get(overlay_replay))
}

fn bucket(state: &AppState) -> Result<&R2Config, ApiError> {
    state.config.r2.as_ref().ok_or_else(|| {
        ApiError::Conflict("os clipes estão desligados: faltam as variáveis R2_* no servidor".to_string())
    })
}

#[derive(Serialize)]
struct ClipDto {
    id: String,
    title: String,
    content_type: String,
    size_bytes: i64,
    duration_ms: Option<i64>,
    created_at: i64,
    /// Plays the video for a while, for the page's preview.
    url: String,
}

fn clip_dto(r2: &R2Config, clip: &Clip) -> ClipDto {
    ClipDto {
        id: clip.id.clone(),
        title: clip.title.clone(),
        content_type: clip.content_type.clone(),
        size_bytes: clip.size_bytes,
        duration_ms: clip.duration_ms,
        created_at: clip.created_at,
        url: r2.presign("GET", &clip.object_key, PLAY_LINK_SECS),
    }
}

#[derive(Serialize)]
struct PlayDto {
    stream: i64,
    play_id: String,
    clip_id: String,
    title: String,
    duration_ms: Option<i64>,
    /// Since the admin pressed play, by the server's clock.
    elapsed_ms: i64,
}

fn play_dto(play: &Play) -> PlayDto {
    PlayDto {
        stream: play.stream,
        play_id: play.play_id.clone(),
        clip_id: play.clip.id.clone(),
        title: play.clip.title.clone(),
        duration_ms: play.clip.duration_ms,
        elapsed_ms: clips::elapsed_ms(play),
    }
}

#[derive(Serialize)]
pub struct ClipsDto {
    /// The R2 bucket is set up.
    enabled: bool,
    max_bytes: i64,
    /// Newest first.
    clips: Vec<ClipDto>,
    /// What each stream was last told to play.
    playing: Vec<PlayDto>,
}

async fn clips_dto(state: &AppState, guild_id: u64) -> Result<ClipsDto, ApiError> {
    let Some(r2) = state.config.r2.as_ref() else {
        return Ok(ClipsDto {
            enabled: false,
            max_bytes: clips::MAX_BYTES,
            clips: Vec::new(),
            playing: Vec::new(),
        });
    };
    let guild = guild_id as i64;
    Ok(ClipsDto {
        enabled: true,
        max_bytes: clips::MAX_BYTES,
        clips: clips::list(&state.db, guild).await?.iter().map(|c| clip_dto(r2, c)).collect(),
        playing: clips::plays(&state.db, guild).await?.iter().map(play_dto).collect(),
    })
}

/// The server's clips and what each stream plays.
async fn list_clips(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
) -> Result<Json<ClipsDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    Ok(Json(clips_dto(&state, guild_id).await?))
}

#[derive(Deserialize)]
struct NewClipRequest {
    title: String,
    content_type: String,
    size_bytes: i64,
}

#[derive(Serialize)]
struct NewClipDto {
    clip_id: String,
    /// `PUT` the file here, with the same `Content-Type`.
    upload_url: String,
}

/// A clip waiting for its file, and where the browser sends it.
async fn new_clip(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
    Json(request): Json<NewClipRequest>,
) -> Result<Json<NewClipDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let r2 = bucket(&state)?;
    let clip = clips::create(&state.db, guild_id as i64, &request.title, &request.content_type, request.size_bytes)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    tracing::info!(user_id = session.user_id, guild_id, clip_id = clip.id, size = clip.size_bytes, "clip upload started");
    Ok(Json(NewClipDto {
        upload_url: r2.presign("PUT", &clip.object_key, UPLOAD_LINK_SECS),
        clip_id: clip.id,
    }))
}

#[derive(Deserialize)]
struct ReadyRequest {
    /// As the browser read it from the file.
    #[serde(default)]
    duration_ms: Option<i64>,
}

/// The browser finished the upload: checked in the bucket, then listed.
async fn clip_ready(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path((guild_id, clip_id)): Path<(u64, String)>,
    Json(request): Json<ReadyRequest>,
) -> Result<Json<ClipsDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let r2 = bucket(&state)?;
    let clip = clips::get(&state.db, guild_id as i64, &clip_id).await?.ok_or(ApiError::NotFound)?;
    let head = state.http.head(r2.presign("HEAD", &clip.object_key, 60)).send().await?;
    if !head.status().is_success() {
        return Err(ApiError::Conflict("o arquivo não chegou no bucket; envie de novo".to_string()));
    }
    let size = head
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(clip.size_bytes);
    if size > clips::MAX_BYTES {
        delete_object(&state, r2, &clip).await?;
        clips::delete(&state.db, guild_id as i64, &clip.id).await?;
        return Err(ApiError::BadRequest("o clipe pode ter até 1 GB".to_string()));
    }
    clips::mark_ready(&state.db, guild_id as i64, &clip.id, size, request.duration_ms).await?;
    Ok(Json(clips_dto(&state, guild_id).await?))
}

async fn delete_object(state: &AppState, r2: &R2Config, clip: &Clip) -> Result<(), ApiError> {
    let response = state.http.delete(r2.presign("DELETE", &clip.object_key, 60)).send().await?;
    // Already gone is fine.
    if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND {
        return Err(ApiError::Internal(anyhow::anyhow!("R2 refused to delete {}: {}", clip.object_key, response.status())));
    }
    Ok(())
}

/// Deletes the clip's file and the clip; a stream playing it stops.
async fn delete_clip(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path((guild_id, clip_id)): Path<(u64, String)>,
) -> Result<Json<ClipsDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    let r2 = bucket(&state)?;
    let clip = clips::get(&state.db, guild_id as i64, &clip_id).await?.ok_or(ApiError::NotFound)?;
    delete_object(&state, r2, &clip).await?;
    clips::delete(&state.db, guild_id as i64, &clip.id).await?;
    tracing::info!(user_id = session.user_id, guild_id, clip_id, "clip deleted");
    state.link.changes().notify(guild_id);
    Ok(Json(clips_dto(&state, guild_id).await?))
}

#[derive(Deserialize)]
struct StreamsRequest {
    /// `[1]`, `[2]` or `[1, 2]`.
    streams: Vec<i64>,
}

/// Plays the clip on the streams' replay overlays now.
async fn play_clip(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path((guild_id, clip_id)): Path<(u64, String)>,
    Json(request): Json<StreamsRequest>,
) -> Result<Json<ClipsDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    bucket(&state)?;
    clips::play(&state.db, guild_id as i64, &clip_id, &request.streams)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    state.link.changes().notify(guild_id);
    Ok(Json(clips_dto(&state, guild_id).await?))
}

/// Takes the replay off the streams' overlays.
async fn stop_replay(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(guild_id): Path<u64>,
    Json(request): Json<StreamsRequest>,
) -> Result<Json<ClipsDto>, ApiError> {
    managed_guild(&state, &session, guild_id)?;
    clips::stop(&state.db, guild_id as i64, &request.streams)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    state.link.changes().notify(guild_id);
    Ok(Json(clips_dto(&state, guild_id).await?))
}

#[derive(Serialize)]
struct OverlayPlayDto {
    play_id: String,
    title: String,
    url: String,
    duration_ms: Option<i64>,
    elapsed_ms: i64,
}

#[derive(Serialize)]
pub struct OverlayReplayDto {
    /// The stream's last replay, finished or not; the overlay skips one that already ended.
    playing: Option<OverlayPlayDto>,
}

/// What stream 1's or 2's replay overlay plays. Public: the link's secret is the only key.
async fn overlay_replay(
    State(state): State<AppState>,
    Path((token, stream)): Path<(String, i64)>,
) -> Result<Json<OverlayReplayDto>, ApiError> {
    let guild_id = overlay_guild(&state, &token).await?;
    if !clips::STREAMS.contains(&stream) {
        return Err(ApiError::NotFound);
    }
    let playing = match state.config.r2.as_ref() {
        Some(r2) => clips::plays(&state.db, guild_id as i64)
            .await?
            .into_iter()
            .find(|p| p.stream == stream)
            .map(|p| OverlayPlayDto {
                url: r2.presign("GET", &p.clip.object_key, PLAY_LINK_SECS),
                elapsed_ms: clips::elapsed_ms(&p),
                play_id: p.play_id,
                title: p.clip.title,
                duration_ms: p.clip.duration_ms,
            }),
        None => None,
    };
    Ok(Json(OverlayReplayDto { playing }))
}
