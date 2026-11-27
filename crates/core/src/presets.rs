//! Selections of players saved per server before any camp: who plays, their category (S or SS
//! rank) and their main role. Picking one fills a camp in signup.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use sqlx::SqlitePool;

use crate::db;
use crate::queue::Role;

/// A player's category in a camp: S rank, or SS rank above it (the team captains).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rank {
    S,
    Ss,
}

impl Rank {
    /// `s` or `ss`, as stored and as the admin page sends it.
    pub fn as_str(self) -> &'static str {
        match self {
            Rank::S => "s",
            Rank::Ss => "ss",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        [Rank::S, Rank::Ss].into_iter().find(|rank| rank.as_str() == name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresetPlayer {
    pub user_id: i64,
    pub rank: Option<Rank>,
    pub role: Option<Role>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preset {
    pub id: i64,
    pub name: String,
    /// In the order they were added.
    pub players: Vec<PresetPlayer>,
    pub updated_at: i64,
}

/// The longest name a selection can have.
pub const MAX_NAME: usize = 60;

/// The most players a selection holds: the biggest camp.
pub const MAX_PLAYERS: usize = 30;

/// A server's selections, newest change first.
pub async fn list(pool: &SqlitePool, guild_id: i64) -> Result<Vec<Preset>> {
    let rows: Vec<(i64, String, i64)> = sqlx::query_as(
        "SELECT id, name, updated_at FROM guild_presets WHERE guild_id = ? ORDER BY updated_at DESC, id DESC",
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await?;
    let mut presets = Vec::with_capacity(rows.len());
    for (id, name, updated_at) in rows {
        presets.push(Preset { id, name, players: players(pool, id).await?, updated_at });
    }
    Ok(presets)
}

async fn players(pool: &SqlitePool, preset_id: i64) -> Result<Vec<PresetPlayer>> {
    let rows: Vec<(i64, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT user_id, rank, role FROM guild_preset_players WHERE preset_id = ? ORDER BY position",
    )
    .bind(preset_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(user_id, rank, role)| PresetPlayer {
            user_id,
            rank: rank.as_deref().and_then(Rank::parse),
            role: role.as_deref().and_then(Role::parse),
        })
        .collect())
}

/// One of the server's selections.
pub async fn load(pool: &SqlitePool, guild_id: i64, preset_id: i64) -> Result<Option<Preset>> {
    let row: Option<(String, i64)> =
        sqlx::query_as("SELECT name, updated_at FROM guild_presets WHERE id = ? AND guild_id = ?")
            .bind(preset_id)
            .bind(guild_id)
            .fetch_optional(pool)
            .await?;
    let Some((name, updated_at)) = row else {
        return Ok(None);
    };
    Ok(Some(Preset { id: preset_id, name, players: players(pool, preset_id).await?, updated_at }))
}

/// Creates a selection (`preset_id` `None`) or replaces one of the server's. A player listed twice
/// keeps the first entry. Returns the selection's ID.
pub async fn save(
    pool: &SqlitePool,
    guild_id: i64,
    preset_id: Option<i64>,
    name: &str,
    players: &[PresetPlayer],
) -> Result<i64> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("dê um nome para a seleção"));
    }
    if name.chars().count() > MAX_NAME {
        return Err(anyhow!("o nome da seleção pode ter até {MAX_NAME} caracteres"));
    }
    let mut unique: Vec<PresetPlayer> = Vec::with_capacity(players.len());
    for player in players {
        if !unique.iter().any(|p| p.user_id == player.user_id) {
            unique.push(*player);
        }
    }
    if unique.len() > MAX_PLAYERS {
        return Err(anyhow!("uma seleção tem até {MAX_PLAYERS} jogadores"));
    }

    let mut tx = pool.begin().await?;
    let ts = db::now();
    let id = match preset_id {
        Some(id) => {
            let updated = sqlx::query("UPDATE guild_presets SET name = ?, updated_at = ? WHERE id = ? AND guild_id = ?")
                .bind(name)
                .bind(ts)
                .bind(id)
                .bind(guild_id)
                .execute(&mut *tx)
                .await?
                .rows_affected();
            if updated == 0 {
                return Err(anyhow!("seleção não encontrada"));
            }
            sqlx::query("DELETE FROM guild_preset_players WHERE preset_id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            id
        }
        None => sqlx::query_scalar("INSERT INTO guild_presets (guild_id, name, updated_at) VALUES (?, ?, ?) RETURNING id")
            .bind(guild_id)
            .bind(name)
            .bind(ts)
            .fetch_one(&mut *tx)
            .await?,
    };
    for (position, player) in unique.iter().enumerate() {
        sqlx::query(
            "INSERT INTO guild_preset_players (preset_id, user_id, rank, role, position) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(player.user_id)
        .bind(player.rank.map(Rank::as_str))
        .bind(player.role.map(Role::as_str))
        .bind(position as i64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(id)
}

pub async fn delete(pool: &SqlitePool, guild_id: i64, preset_id: i64) -> Result<()> {
    let mut tx = pool.begin().await?;
    let deleted = sqlx::query("DELETE FROM guild_presets WHERE id = ? AND guild_id = ?")
        .bind(preset_id)
        .bind(guild_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if deleted == 0 {
        return Err(anyhow!("seleção não encontrada"));
    }
    sqlx::query("DELETE FROM guild_preset_players WHERE preset_id = ?")
        .bind(preset_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// The main role of every server member who has one, kept for every camp of the server.
pub async fn guild_roles(pool: &SqlitePool, guild_id: i64) -> Result<HashMap<i64, Role>> {
    let rows: Vec<(i64, String)> = sqlx::query_as("SELECT user_id, role FROM guild_player_roles WHERE guild_id = ?")
        .bind(guild_id)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(user_id, role)| Role::parse(&role).map(|role| (user_id, role)))
        .collect())
}

/// What picking a selection did to a camp's roster.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    /// New on the roster.
    pub added: usize,
    /// Left out: the roster was full.
    pub skipped: Vec<i64>,
}

/// Fills a camp in signup from one of its server's selections: adds the players (as many as the
/// camp has room for, in the selection's order) and gives everyone of the selection on the roster
/// its category and role. Players already on the roster stay; roles are the server's, so they
/// change for later camps too.
pub async fn apply(pool: &SqlitePool, camp_id: &str, preset_id: i64) -> Result<Applied> {
    let state = db::load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    if state.camp.status != "signup" {
        return Err(anyhow!("só dá para usar uma seleção na inscrição, antes de iniciar a fila"));
    }
    let preset = load(pool, state.camp.guild_id, preset_id)
        .await?
        .ok_or_else(|| anyhow!("seleção não encontrada"))?;
    let ids: Vec<i64> = preset.players.iter().map(|p| p.user_id).collect();
    let (added, _) = db::add_players(pool, camp_id, &ids).await?;
    let roster: Vec<i64> = db::load_camp(pool, camp_id)
        .await?
        .map(|state| state.players.iter().map(|p| p.user_id).collect())
        .unwrap_or_default();
    let (on_roster, left_out): (Vec<&PresetPlayer>, Vec<&PresetPlayer>) =
        preset.players.iter().partition(|p| roster.contains(&p.user_id));
    for player in on_roster {
        db::set_player_s_rank(pool, camp_id, player.user_id, player.rank == Some(Rank::S)).await?;
        if player.rank == Some(Rank::Ss) {
            db::set_player_ss_rank(pool, camp_id, player.user_id, true).await?;
        }
        db::set_player_role(pool, camp_id, player.user_id, player.role).await?;
    }
    Ok(Applied {
        added,
        skipped: left_out.into_iter().map(|p| p.user_id).collect(),
    })
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
        db::migrate(&pool).await.unwrap();
        pool
    }

    fn player(user_id: i64, rank: Option<Rank>, role: Option<Role>) -> PresetPlayer {
        PresetPlayer { user_id, rank, role }
    }

    #[tokio::test]
    async fn a_selection_fills_a_camp_in_signup() {
        let pool = memory_db().await;
        let players = [
            player(1, Some(Rank::Ss), Some(Role::Tank)),
            player(2, Some(Rank::S), None),
            player(3, None, Some(Role::Carry)),
            player(4, None, None),
            player(5, Some(Rank::S), Some(Role::Support)),
            player(1, None, None),
        ];
        let id = save(&pool, 1, None, " Quinta ", &players).await.unwrap();
        let saved = list(&pool, 1).await.unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!((saved[0].name.as_str(), saved[0].players.len()), ("Quinta", 5), "trimmed, no one twice");
        assert!(list(&pool, 2).await.unwrap().is_empty(), "per server");
        assert!(save(&pool, 2, Some(id), "Outra", &players).await.is_err(), "another server's");

        // Player 3 already signed up with S rank: the selection decides.
        let camp = db::create_camp(&pool, 1, 10, 4, 2).await.unwrap();
        db::add_players(&pool, &camp.id, &[3]).await.unwrap();
        db::set_player_s_rank(&pool, &camp.id, 3, true).await.unwrap();
        let applied = apply(&pool, &camp.id, id).await.unwrap();
        assert_eq!(applied, Applied { added: 3, skipped: vec![5] }, "the camp holds 4");

        let state = db::load_camp(&pool, &camp.id).await.unwrap().unwrap();
        let of = |id: i64| state.players.iter().find(|p| p.user_id == id).map(|p| (p.s_rank, p.ss_rank, p.role));
        assert_eq!(of(1), Some((false, true, Some(Role::Tank))));
        assert_eq!(of(2), Some((true, false, None)));
        assert_eq!(of(3), Some((false, false, Some(Role::Carry))));
        assert_eq!(of(4), Some((false, false, None)));
        assert_eq!(of(5), None);
        assert_eq!(guild_roles(&pool, 1).await.unwrap().get(&1), Some(&Role::Tank), "roles are the server's");

        db::start_queue(&pool, &camp.id).await.unwrap();
        assert!(apply(&pool, &camp.id, id).await.is_err(), "signup only");
        let other = db::create_camp(&pool, 2, 10, 4, 2).await.unwrap();
        assert!(apply(&pool, &other.id, id).await.is_err(), "another server's selection");

        delete(&pool, 1, id).await.unwrap();
        assert!(list(&pool, 1).await.unwrap().is_empty());
        assert!(delete(&pool, 1, id).await.is_err());
    }

    #[tokio::test]
    async fn a_selection_needs_a_name_and_fits_a_camp() {
        let pool = memory_db().await;
        assert!(save(&pool, 1, None, "  ", &[]).await.is_err());
        let many: Vec<PresetPlayer> = (0..31).map(|id| player(id, None, None)).collect();
        assert!(save(&pool, 1, None, "Grande", &many).await.is_err());
        assert!(save(&pool, 1, None, "Grande", &many[..30]).await.is_ok());
    }
}
