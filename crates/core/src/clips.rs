//! Replay clips: videos admins upload to R2 (`r2.rs`) and play on a stream's replay overlay.
//! A clip is a row here plus its file in the bucket; it's listed once its upload is confirmed.

use anyhow::{anyhow, Result};
use rand::distributions::{Alphanumeric, DistString};
use sqlx::SqlitePool;

use crate::db::now;

/// One stream's replay overlay each, like the cameras.
pub const STREAMS: [i64; 2] = [1, 2];
/// A clip can be up to 1 GB, minutes of 1080p.
pub const MAX_BYTES: i64 = 1024 * 1024 * 1024;
const MAX_TITLE: usize = 80;

#[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct Clip {
    pub id: String,
    pub guild_id: i64,
    pub title: String,
    /// Where the file is in the bucket: `clips/{guild_id}/{id}.mp4`.
    pub object_key: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub duration_ms: Option<i64>,
    pub ready: bool,
    pub created_at: i64,
}

/// A clip a stream was told to play, and when (Unix milliseconds).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Play {
    pub stream: i64,
    /// New each time, so the overlay knows a replay of the same clip is a new one.
    pub play_id: String,
    pub started_at_ms: i64,
    pub clip: Clip,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_millis() as i64
}

fn random_id() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 12)
}

/// The file's extension, for a type OBS's browser plays: MP4 (H.264 + AAC) or WebM.
fn extension(content_type: &str) -> Option<&'static str> {
    match content_type {
        "video/mp4" => Some("mp4"),
        "video/webm" => Some("webm"),
        _ => None,
    }
}

/// A new clip waiting for its upload. Its title is trimmed; MP4 or WebM, up to `MAX_BYTES`.
pub async fn create(pool: &SqlitePool, guild_id: i64, title: &str, content_type: &str, size_bytes: i64) -> Result<Clip> {
    let title = title.trim();
    if title.is_empty() {
        return Err(anyhow!("dê um nome para o clipe"));
    }
    if title.chars().count() > MAX_TITLE {
        return Err(anyhow!("o nome do clipe pode ter até {MAX_TITLE} caracteres"));
    }
    let Some(ext) = extension(content_type) else {
        return Err(anyhow!("envie um vídeo MP4 (H.264) ou WebM"));
    };
    if size_bytes <= 0 || size_bytes > MAX_BYTES {
        return Err(anyhow!("o clipe pode ter até 1 GB"));
    }
    let id = random_id();
    let clip = Clip {
        object_key: format!("clips/{guild_id}/{id}.{ext}"),
        id,
        guild_id,
        title: title.to_string(),
        content_type: content_type.to_string(),
        size_bytes,
        duration_ms: None,
        ready: false,
        created_at: now(),
    };
    sqlx::query(
        "INSERT INTO clips (id, guild_id, title, object_key, content_type, size_bytes, duration_ms, ready, created_at)
         VALUES (?, ?, ?, ?, ?, ?, NULL, 0, ?)",
    )
    .bind(&clip.id)
    .bind(guild_id)
    .bind(&clip.title)
    .bind(&clip.object_key)
    .bind(&clip.content_type)
    .bind(size_bytes)
    .bind(clip.created_at)
    .execute(pool)
    .await?;
    Ok(clip)
}

/// The server's clip, uploaded or not.
pub async fn get(pool: &SqlitePool, guild_id: i64, id: &str) -> Result<Option<Clip>> {
    Ok(sqlx::query_as("SELECT * FROM clips WHERE id = ? AND guild_id = ?")
        .bind(id)
        .bind(guild_id)
        .fetch_optional(pool)
        .await?)
}

/// The upload arrived: `size_bytes` as the bucket has it, `duration_ms` as the browser read it.
pub async fn mark_ready(pool: &SqlitePool, guild_id: i64, id: &str, size_bytes: i64, duration_ms: Option<i64>) -> Result<()> {
    sqlx::query("UPDATE clips SET ready = 1, size_bytes = ?, duration_ms = ? WHERE id = ? AND guild_id = ?")
        .bind(size_bytes)
        .bind(duration_ms.filter(|ms| *ms > 0))
        .bind(id)
        .bind(guild_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// The server's uploaded clips, newest first.
pub async fn list(pool: &SqlitePool, guild_id: i64) -> Result<Vec<Clip>> {
    Ok(sqlx::query_as("SELECT * FROM clips WHERE guild_id = ? AND ready = 1 ORDER BY created_at DESC, id")
        .bind(guild_id)
        .fetch_all(pool)
        .await?)
}

/// Forgets the clip, and stops it wherever it plays. The caller deletes its file.
pub async fn delete(pool: &SqlitePool, guild_id: i64, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM clip_plays WHERE guild_id = ? AND clip_id = ?")
        .bind(guild_id)
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM clips WHERE id = ? AND guild_id = ?")
        .bind(id)
        .bind(guild_id)
        .execute(pool)
        .await?;
    Ok(())
}

fn check_streams(streams: &[i64]) -> Result<()> {
    if streams.is_empty() || streams.iter().any(|s| !STREAMS.contains(s)) {
        return Err(anyhow!("escolha a transmissão 1, a 2 ou as duas"));
    }
    Ok(())
}

/// Plays the clip on `streams` from now; whatever played there before stops.
pub async fn play(pool: &SqlitePool, guild_id: i64, id: &str, streams: &[i64]) -> Result<()> {
    check_streams(streams)?;
    match get(pool, guild_id, id).await? {
        Some(clip) if clip.ready => {}
        _ => return Err(anyhow!("clipe não encontrado")),
    }
    let started = now_ms();
    for stream in streams {
        sqlx::query(
            "INSERT INTO clip_plays (guild_id, stream, play_id, clip_id, started_at_ms) VALUES (?, ?, ?, ?, ?)
             ON CONFLICT (guild_id, stream) DO UPDATE SET
               play_id = excluded.play_id, clip_id = excluded.clip_id, started_at_ms = excluded.started_at_ms",
        )
        .bind(guild_id)
        .bind(stream)
        .bind(random_id())
        .bind(id)
        .bind(started)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Takes whatever plays on `streams` off the screen.
pub async fn stop(pool: &SqlitePool, guild_id: i64, streams: &[i64]) -> Result<()> {
    check_streams(streams)?;
    for stream in streams {
        sqlx::query("DELETE FROM clip_plays WHERE guild_id = ? AND stream = ?")
            .bind(guild_id)
            .bind(stream)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// The last clip each stream was told to play, finished or not.
pub async fn plays(pool: &SqlitePool, guild_id: i64) -> Result<Vec<Play>> {
    let rows: Vec<(i64, String, i64, String)> = sqlx::query_as(
        "SELECT stream, play_id, started_at_ms, clip_id FROM clip_plays WHERE guild_id = ? ORDER BY stream",
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await?;
    let mut plays = Vec::with_capacity(rows.len());
    for (stream, play_id, started_at_ms, clip_id) in rows {
        if let Some(clip) = get(pool, guild_id, &clip_id).await? {
            plays.push(Play { stream, play_id, started_at_ms, clip });
        }
    }
    Ok(plays)
}

/// How long ago a play started, by the server's clock, so overlays on other computers agree.
pub fn elapsed_ms(play: &Play) -> i64 {
    (now_ms() - play.started_at_ms).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn memory_db() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::db::migrate(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn clips_are_listed_once_uploaded_and_play_per_stream() {
        let pool = memory_db().await;
        assert!(create(&pool, 1, " ", "video/mp4", 10).await.is_err(), "no title");
        assert!(create(&pool, 1, "a", "video/quicktime", 10).await.is_err(), "MOV");
        assert!(create(&pool, 1, "a", "video/mp4", MAX_BYTES + 1).await.is_err(), "too big");

        let clip = create(&pool, 1, " Kashi 1v3 ", "video/mp4", 1_000).await.unwrap();
        assert_eq!(clip.title, "Kashi 1v3");
        assert_eq!(clip.object_key, format!("clips/1/{}.mp4", clip.id));
        assert!(list(&pool, 1).await.unwrap().is_empty(), "not uploaded yet");
        assert!(play(&pool, 1, &clip.id, &[1]).await.is_err(), "can't play before the upload");

        mark_ready(&pool, 1, &clip.id, 1_234, Some(31_500)).await.unwrap();
        let listed = list(&pool, 1).await.unwrap();
        assert_eq!((listed[0].size_bytes, listed[0].duration_ms), (1_234, Some(31_500)));
        assert!(list(&pool, 2).await.unwrap().is_empty(), "other servers don't see it");
        assert!(get(&pool, 2, &clip.id).await.unwrap().is_none());

        assert!(play(&pool, 1, &clip.id, &[3]).await.is_err());
        play(&pool, 1, &clip.id, &[1, 2]).await.unwrap();
        let first = plays(&pool, 1).await.unwrap();
        assert_eq!(first.iter().map(|p| p.stream).collect::<Vec<_>>(), [1, 2]);
        play(&pool, 1, &clip.id, &[1]).await.unwrap();
        let again = plays(&pool, 1).await.unwrap();
        assert_ne!(again[0].play_id, first[0].play_id, "playing it again is a new play");
        assert_eq!(again[1].play_id, first[1].play_id);

        stop(&pool, 1, &[2]).await.unwrap();
        assert_eq!(plays(&pool, 1).await.unwrap().len(), 1);
        delete(&pool, 1, &clip.id).await.unwrap();
        assert!(plays(&pool, 1).await.unwrap().is_empty(), "deleting stops it");
        assert!(list(&pool, 1).await.unwrap().is_empty());
    }
}
