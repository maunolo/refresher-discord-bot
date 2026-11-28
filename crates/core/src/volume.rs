//! How loud each stream's pause and replay overlay (`clips.rs`, `pause.rs`) plays its sound, set
//! from the Overlays page so the stream's mix can change without touching OBS.

use anyhow::{anyhow, Result};
use sqlx::SqlitePool;

use crate::clips::{check_streams, STREAMS};

/// In percent; a stream nobody set plays at full volume.
pub const DEFAULT_VOLUME: i64 = 100;
pub const MAX_VOLUME: i64 = 100;

/// Sets the volume (0 to 100) of `streams`' overlays.
pub async fn set(pool: &SqlitePool, guild_id: i64, streams: &[i64], volume: i64) -> Result<()> {
    check_streams(streams)?;
    if !(0..=MAX_VOLUME).contains(&volume) {
        return Err(anyhow!("o volume vai de 0 a {MAX_VOLUME}%"));
    }
    for stream in streams {
        sqlx::query(
            "INSERT INTO overlay_volumes (guild_id, stream, volume) VALUES (?, ?, ?)
             ON CONFLICT (guild_id, stream) DO UPDATE SET volume = excluded.volume",
        )
        .bind(guild_id)
        .bind(stream)
        .bind(volume)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Every stream's volume, by stream: `(stream, volume)`.
pub async fn list(pool: &SqlitePool, guild_id: i64) -> Result<Vec<(i64, i64)>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as("SELECT stream, volume FROM overlay_volumes WHERE guild_id = ?")
        .bind(guild_id)
        .fetch_all(pool)
        .await?;
    Ok(STREAMS
        .iter()
        .map(|&stream| {
            let volume = rows.iter().find(|(s, _)| *s == stream).map_or(DEFAULT_VOLUME, |(_, v)| *v);
            (stream, volume)
        })
        .collect())
}

/// One stream's volume.
pub async fn get(pool: &SqlitePool, guild_id: i64, stream: i64) -> Result<i64> {
    Ok(list(pool, guild_id)
        .await?
        .into_iter()
        .find(|(s, _)| *s == stream)
        .map_or(DEFAULT_VOLUME, |(_, v)| v))
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
    async fn each_stream_keeps_its_own_volume() {
        let pool = memory_db().await;
        assert_eq!(list(&pool, 1).await.unwrap(), vec![(1, 100), (2, 100)], "full volume until set");
        assert!(set(&pool, 1, &[3], 50).await.is_err());
        assert!(set(&pool, 1, &[1], 101).await.is_err());
        assert!(set(&pool, 1, &[1], -1).await.is_err());

        set(&pool, 1, &[2], 40).await.unwrap();
        assert_eq!(get(&pool, 1, 1).await.unwrap(), 100);
        assert_eq!(get(&pool, 1, 2).await.unwrap(), 40);
        set(&pool, 1, &[1, 2], 0).await.unwrap();
        assert_eq!(list(&pool, 1).await.unwrap(), vec![(1, 0), (2, 0)]);
        assert_eq!(get(&pool, 2, 2).await.unwrap(), 100, "per server");
    }
}
