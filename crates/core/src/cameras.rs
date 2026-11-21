//! The commentators' cameras on a server's two stream overlays: VDO.Ninja sources in one room,
//! and which of them each stream shows, left and right.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// One overlay per streamer.
pub const STREAMS: usize = 2;
pub const MAX_SOURCES: usize = 16;
const MAX_NAME: usize = 40;
const MAX_ROOM: usize = 60;
const MAX_PASSWORD: usize = 100;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraConfig {
    /// The VDO.Ninja room; empty for none.
    #[serde(default)]
    pub room: String,
    /// The room's password; empty for none. Overlay links carry it to the stream's OBS.
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub sources: Vec<CameraSource>,
    /// Stream 1, then stream 2.
    #[serde(default)]
    pub streams: [StreamCameras; STREAMS],
}

/// A commentator's camera: what they push to VDO.Ninja (`&push=ID`) and the name shown under it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraSource {
    pub id: String,
    #[serde(default)]
    pub name: String,
}

/// The sources one stream shows, by ID. With only one live, it moves to the middle.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamCameras {
    /// Who streams it, for the admin page.
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub left: Option<String>,
    #[serde(default)]
    pub right: Option<String>,
}

/// What one stream's overlay needs: the room and its cameras, left then right.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StreamView {
    pub room: String,
    pub password: String,
    pub cameras: Vec<CameraSource>,
}

/// VDO.Ninja turns anything but letters, digits and `_` in IDs and rooms into `_`.
pub fn vdo_id(text: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in text.trim().chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
            gap = false;
        } else if !gap {
            out.push('_');
            gap = true;
        }
    }
    out
}

fn short(text: &str, max: usize, what: &str) -> Result<String> {
    let text = text.trim();
    if text.chars().count() > max {
        return Err(anyhow!("{what} pode ter até {max} caracteres"));
    }
    Ok(text.to_string())
}

impl CameraConfig {
    /// Trimmed and checked: IDs as VDO.Ninja writes them, each once, and streams that only use
    /// the list's sources, never the same one twice.
    pub fn check(self) -> Result<Self> {
        if self.sources.len() > MAX_SOURCES {
            return Err(anyhow!("até {MAX_SOURCES} câmeras"));
        }
        let room = vdo_id(&short(&self.room, MAX_ROOM, "o nome da sala")?);
        if self.password.trim().chars().count() > MAX_PASSWORD {
            return Err(anyhow!("a senha pode ter até {MAX_PASSWORD} caracteres"));
        }
        let password = self.password.trim().to_string();
        let mut sources: Vec<CameraSource> = Vec::with_capacity(self.sources.len());
        for source in self.sources {
            let id = vdo_id(&source.id);
            if id.is_empty() {
                return Err(anyhow!("toda câmera precisa de um ID do VDO.Ninja"));
            }
            if id.len() > MAX_NAME {
                return Err(anyhow!("o ID da câmera pode ter até {MAX_NAME} caracteres"));
            }
            if sources.iter().any(|s| s.id == id) {
                return Err(anyhow!("o ID {id} aparece em duas câmeras"));
            }
            let name = short(&source.name, MAX_NAME, "o nome da câmera")?;
            sources.push(CameraSource { id, name });
        }
        let mut streams: [StreamCameras; STREAMS] = Default::default();
        for (n, (stream, checked)) in self.streams.into_iter().zip(streams.iter_mut()).enumerate() {
            let pick = |id: Option<String>| -> Result<Option<String>> {
                let Some(id) = id.map(|id| vdo_id(&id)).filter(|id| !id.is_empty()) else {
                    return Ok(None);
                };
                if !sources.iter().any(|s| s.id == id) {
                    return Err(anyhow!("a transmissão {} usa uma câmera que não está na lista", n + 1));
                }
                Ok(Some(id))
            };
            let left = pick(stream.left)?;
            let right = pick(stream.right)?;
            if left.is_some() && left == right {
                return Err(anyhow!("a transmissão {} mostra a mesma câmera dos dois lados", n + 1));
            }
            *checked = StreamCameras {
                label: short(&stream.label, MAX_NAME, "o nome da transmissão")?,
                left,
                right,
            };
        }
        Ok(Self { room, password, sources, streams })
    }

    /// Stream `n` (1 or 2), or `None` for any other number.
    pub fn stream(&self, n: usize) -> Option<StreamView> {
        let stream = self.streams.get(n.checked_sub(1)?)?;
        let source = |id: &Option<String>| {
            id.as_ref()
                .and_then(|id| self.sources.iter().find(|s| &s.id == id))
                .cloned()
        };
        Some(StreamView {
            room: self.room.clone(),
            password: self.password.clone(),
            cameras: [source(&stream.left), source(&stream.right)].into_iter().flatten().collect(),
        })
    }
}

/// The server's cameras; nothing set up yet is an empty list.
pub async fn load(pool: &SqlitePool, guild_id: i64) -> Result<CameraConfig> {
    let saved: Option<String> = sqlx::query_scalar("SELECT config FROM camera_overlays WHERE guild_id = ?")
        .bind(guild_id)
        .fetch_optional(pool)
        .await?;
    Ok(match saved {
        Some(json) => serde_json::from_str(&json)?,
        None => CameraConfig::default(),
    })
}

/// Checks and saves the server's cameras; returns them as saved.
pub async fn save(pool: &SqlitePool, guild_id: i64, config: CameraConfig) -> Result<CameraConfig> {
    let config = config.check()?;
    sqlx::query(
        "INSERT INTO camera_overlays (guild_id, config, updated_at) VALUES (?, ?, ?)
         ON CONFLICT (guild_id) DO UPDATE SET config = excluded.config, updated_at = excluded.updated_at",
    )
    .bind(guild_id)
    .bind(serde_json::to_string(&config)?)
    .bind(crate::db::now())
    .execute(pool)
    .await?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, name: &str) -> CameraSource {
        CameraSource { id: id.to_string(), name: name.to_string() }
    }

    fn stream(left: Option<&str>, right: Option<&str>) -> StreamCameras {
        StreamCameras {
            label: String::new(),
            left: left.map(str::to_string),
            right: right.map(str::to_string),
        }
    }

    #[test]
    fn ids_are_written_as_vdo_ninja_does() {
        assert_eq!(vdo_id(" camp farinha! "), "camp_farinha_");
        assert_eq!(vdo_id("QJSe8uW"), "QJSe8uW");
        assert_eq!(vdo_id("a--b"), "a_b");
    }

    #[test]
    fn streams_pick_from_the_list() {
        let config = CameraConfig {
            room: "camp farinha".to_string(),
            password: " segredo ".to_string(),
            sources: vec![source("QJSe8uW", " haunter "), source("8hqFUc5", "Farinha"), source("x1", "")],
            streams: [stream(Some("QJSe8uW"), Some("8hqFUc5")), stream(None, Some("x1"))],
        }
        .check()
        .unwrap();
        assert_eq!((config.room.as_str(), config.password.as_str()), ("camp_farinha", "segredo"));
        assert_eq!(config.sources[0].name, "haunter");
        let one = config.stream(1).unwrap();
        assert_eq!(one.cameras.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["QJSe8uW", "8hqFUc5"]);
        assert_eq!(config.stream(2).unwrap().cameras, [source("x1", "")]);
        assert!(config.stream(0).is_none() && config.stream(3).is_none());
    }

    #[test]
    fn bad_configs_are_refused() {
        let base = || CameraConfig {
            sources: vec![source("a", "A"), source("b", "B")],
            ..CameraConfig::default()
        };
        let mut twice = base();
        twice.sources.push(source("a", "again"));
        assert!(twice.check().is_err(), "the same ID twice");
        let mut unknown = base();
        unknown.streams[0] = stream(Some("zzz"), None);
        assert!(unknown.check().is_err(), "a camera not on the list");
        let mut same = base();
        same.streams[1] = stream(Some("a"), Some("a"));
        assert!(same.check().is_err(), "one camera on both sides");
        let mut empty = base();
        empty.sources.push(source(" ", "nobody"));
        assert!(empty.check().is_err(), "a camera without an ID");
        assert!(base().check().is_ok());
    }

    #[tokio::test]
    async fn saved_per_server() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::db::migrate(&pool).await.unwrap();
        assert_eq!(load(&pool, 1).await.unwrap(), CameraConfig::default());
        let config = CameraConfig {
            room: "sala".to_string(),
            sources: vec![source("a", "A")],
            streams: [stream(Some("a"), None), StreamCameras::default()],
            ..CameraConfig::default()
        };
        let saved = save(&pool, 1, config.clone()).await.unwrap();
        assert_eq!(load(&pool, 1).await.unwrap(), saved);
        assert_eq!(load(&pool, 2).await.unwrap(), CameraConfig::default(), "other servers have their own");
        let mut changed = saved.clone();
        changed.room = "outra".to_string();
        save(&pool, 1, changed).await.unwrap();
        assert_eq!(load(&pool, 1).await.unwrap().room, "outra");
    }
}
