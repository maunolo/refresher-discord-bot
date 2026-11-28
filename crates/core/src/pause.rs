//! The pause screen: each stream's replay overlay (`clips.rs`) can show a countdown with a
//! message ("Já voltamos") while the stream is on a break. Admins start it from the Overlays
//! page with a length, add time to it, and take it off.

use anyhow::{anyhow, Result};
use sqlx::SqlitePool;

use crate::clips::{check_streams, now_ms};

pub const DEFAULT_MESSAGE: &str = "Já voltamos";
const MAX_MESSAGE: usize = 60;
/// Up to 3 hours.
pub const MAX_MS: i64 = 3 * 60 * 60 * 1000;

/// A stream's pause, as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pause {
    pub stream: i64,
    pub message: String,
    /// The length it was started with, plus any time added.
    pub duration_ms: i64,
    pub ends_at_ms: i64,
}

impl Pause {
    /// Left on the countdown, by the server's clock; 0 once it ran out.
    pub fn remaining_ms(&self) -> i64 {
        (self.ends_at_ms - now_ms()).max(0)
    }
}

fn clean_message(message: &str) -> Result<String> {
    let message = message.trim();
    if message.chars().count() > MAX_MESSAGE {
        return Err(anyhow!("a mensagem pode ter até {MAX_MESSAGE} caracteres"));
    }
    Ok(if message.is_empty() { DEFAULT_MESSAGE.to_string() } else { message.to_string() })
}

/// Shows the pause on `streams`, counting down `duration_ms` from now; replaces one under way.
pub async fn start(pool: &SqlitePool, guild_id: i64, streams: &[i64], duration_ms: i64, message: &str) -> Result<()> {
    check_streams(streams)?;
    if !(1_000..=MAX_MS).contains(&duration_ms) {
        return Err(anyhow!("o contador vai de 1 segundo a 3 horas"));
    }
    let message = clean_message(message)?;
    let ends = now_ms() + duration_ms;
    for stream in streams {
        sqlx::query(
            "INSERT INTO pause_screens (guild_id, stream, message, duration_ms, ends_at_ms) VALUES (?, ?, ?, ?, ?)
             ON CONFLICT (guild_id, stream) DO UPDATE SET
               message = excluded.message, duration_ms = excluded.duration_ms, ends_at_ms = excluded.ends_at_ms",
        )
        .bind(guild_id)
        .bind(stream)
        .bind(&message)
        .bind(duration_ms)
        .bind(ends)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Adds (or, negative, takes) time on the pauses under way on `streams`. A countdown that already
/// ran out counts from now again.
pub async fn add(pool: &SqlitePool, guild_id: i64, streams: &[i64], delta_ms: i64) -> Result<()> {
    check_streams(streams)?;
    let now = now_ms();
    for pause in list(pool, guild_id).await?.into_iter().filter(|p| streams.contains(&p.stream)) {
        let ends = (pause.ends_at_ms.max(now) + delta_ms).clamp(now, now + MAX_MS);
        let duration = (pause.duration_ms + (ends - pause.ends_at_ms.max(now))).clamp(1_000, MAX_MS);
        sqlx::query("UPDATE pause_screens SET ends_at_ms = ?, duration_ms = ? WHERE guild_id = ? AND stream = ?")
            .bind(ends)
            .bind(duration)
            .bind(guild_id)
            .bind(pause.stream)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Takes the pause off `streams`.
pub async fn stop(pool: &SqlitePool, guild_id: i64, streams: &[i64]) -> Result<()> {
    check_streams(streams)?;
    for stream in streams {
        sqlx::query("DELETE FROM pause_screens WHERE guild_id = ? AND stream = ?")
            .bind(guild_id)
            .bind(stream)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// The pauses on screen, by stream.
pub async fn list(pool: &SqlitePool, guild_id: i64) -> Result<Vec<Pause>> {
    let rows: Vec<(i64, String, i64, i64)> = sqlx::query_as(
        "SELECT stream, message, duration_ms, ends_at_ms FROM pause_screens WHERE guild_id = ? ORDER BY stream",
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(stream, message, duration_ms, ends_at_ms)| Pause { stream, message, duration_ms, ends_at_ms })
        .collect())
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
    async fn pauses_count_down_per_stream() {
        let pool = memory_db().await;
        assert!(start(&pool, 1, &[3], 60_000, "").await.is_err());
        assert!(start(&pool, 1, &[1], 0, "").await.is_err());
        start(&pool, 1, &[1, 2], 5 * 60_000, "  ").await.unwrap();
        let pauses = list(&pool, 1).await.unwrap();
        assert_eq!(pauses.len(), 2);
        assert_eq!(pauses[0].message, DEFAULT_MESSAGE);
        assert!(pauses[0].remaining_ms() > 4 * 60_000);

        add(&pool, 1, &[2], 60_000).await.unwrap();
        let pauses = list(&pool, 1).await.unwrap();
        assert_eq!(pauses[1].duration_ms, 6 * 60_000);
        assert!(pauses[1].remaining_ms() > 5 * 60_000);
        add(&pool, 1, &[2], -60 * 60_000).await.unwrap();
        assert_eq!(list(&pool, 1).await.unwrap()[1].remaining_ms(), 0, "never below zero");

        stop(&pool, 1, &[1]).await.unwrap();
        let pauses = list(&pool, 1).await.unwrap();
        assert_eq!(pauses.iter().map(|p| p.stream).collect::<Vec<_>>(), vec![2]);
        assert!(list(&pool, 2).await.unwrap().is_empty(), "per server");
    }
}
