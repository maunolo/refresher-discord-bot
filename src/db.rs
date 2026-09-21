use anyhow::{anyhow, Result};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool, Transaction, Sqlite};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::queue::{self, CampLayout, QueuePlayer};

#[derive(Clone, Debug)]
pub struct CampRow {
    pub id: String,
    pub guild_id: i64,
    pub channel_id: i64,
    pub board_message_id: Option<i64>,
    pub target_size: i64,
    pub status: String,
    pub match_count: i64,
}

impl CampRow {
    pub fn layout(&self) -> CampLayout {
        CampLayout::from_roster_size(self.target_size)
    }
}

#[derive(Clone, Debug)]
pub struct PlayerRow {
    pub user_id: i64,
    pub points: i64,
    pub location: String,
    pub queue_position: Option<i64>,
    pub consecutive_matches: i64,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MatchRow {
    pub id: i64,
    pub slot: i64,
    pub status: String,
    pub refresh_used: bool,
    pub message_id: Option<i64>,
    pub party_id: Option<String>,
    pub party_code: Option<String>,
    pub winner_side: Option<String>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MatchPlayerRow {
    pub user_id: i64,
    pub side: String,
    pub points_after: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct OpenMatch {
    pub row: MatchRow,
    pub players: Vec<MatchPlayerRow>,
    pub votes: Vec<i64>,
}

#[derive(Clone, Debug)]
pub struct CampState {
    pub camp: CampRow,
    pub players: Vec<PlayerRow>,
    pub open_matches: Vec<OpenMatch>,
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs() as i64
}

pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

async fn generate_unique_id(tx: &mut Transaction<'_, Sqlite>) -> Result<String> {
    for _ in 0..16 {
        let id = queue::generate_camp_id(&mut rand::thread_rng());
        let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM camps WHERE id = ?")
            .bind(&id)
            .fetch_optional(&mut **tx)
            .await?;
        if exists.is_none() {
            return Ok(id);
        }
    }
    Err(anyhow!("could not allocate a unique camp id"))
}

pub async fn create_camp(
    pool: &SqlitePool,
    guild_id: i64,
    channel_id: i64,
    target_size: i64,
) -> Result<CampRow> {
    let mut tx = pool.begin().await?;
    let id = generate_unique_id(&mut tx).await?;
    let ts = now();
    sqlx::query(
        "INSERT INTO camps (id, guild_id, channel_id, target_size, status, match_count, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'signup', 0, ?, ?)",
    )
    .bind(&id)
    .bind(guild_id)
    .bind(channel_id)
    .bind(target_size)
    .bind(ts)
    .bind(ts)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(CampRow {
        id,
        guild_id,
        channel_id,
        board_message_id: None,
        target_size,
        status: "signup".into(),
        match_count: 0,
    })
}

pub async fn set_board_message_id(pool: &SqlitePool, camp_id: &str, message_id: i64) -> Result<()> {
    sqlx::query("UPDATE camps SET board_message_id = ?, updated_at = ? WHERE id = ?")
        .bind(message_id)
        .bind(now())
        .bind(camp_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_match_message_id(pool: &SqlitePool, match_id: i64, message_id: i64) -> Result<()> {
    sqlx::query("UPDATE camp_matches SET message_id = ? WHERE id = ?")
        .bind(message_id)
        .bind(match_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn load_camp(pool: &SqlitePool, camp_id: &str) -> Result<Option<CampState>> {
    let camp = sqlx::query_as::<_, (String, i64, i64, Option<i64>, i64, String, i64)>(
        "SELECT id, guild_id, channel_id, board_message_id, target_size, status, match_count FROM camps WHERE id = ?",
    )
    .bind(camp_id)
    .fetch_optional(pool)
    .await?;
    let Some(c) = camp else {
        return Ok(None);
    };
    Ok(Some(assemble_state(pool, c).await?))
}

pub async fn load_active_for_guild(pool: &SqlitePool, guild_id: i64) -> Result<Option<CampState>> {
    let camp = sqlx::query_as::<_, (String, i64, i64, Option<i64>, i64, String, i64)>(
        "SELECT id, guild_id, channel_id, board_message_id, target_size, status, match_count
         FROM camps WHERE guild_id = ? AND status IN ('signup', 'running') LIMIT 1",
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await?;
    let Some(c) = camp else {
        return Ok(None);
    };
    Ok(Some(assemble_state(pool, c).await?))
}

pub async fn load_restore_camps(pool: &SqlitePool) -> Result<Vec<CampState>> {
    let rows = sqlx::query_as::<_, (String, i64, i64, Option<i64>, i64, String, i64)>(
        "SELECT id, guild_id, channel_id, board_message_id, target_size, status, match_count
         FROM camps WHERE status IN ('signup', 'running')",
    )
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for row in rows {
        match assemble_state(pool, row).await {
            Ok(state) => out.push(state),
            Err(err) => tracing::error!(error = %err, "skipping inconsistent camp on restore"),
        }
    }
    Ok(out)
}

async fn assemble_state(
    pool: &SqlitePool,
    c: (String, i64, i64, Option<i64>, i64, String, i64),
) -> Result<CampState> {
    let camp = CampRow {
        id: c.0,
        guild_id: c.1,
        channel_id: c.2,
        board_message_id: c.3,
        target_size: c.4,
        status: c.5,
        match_count: c.6,
    };
    let player_rows = sqlx::query_as::<_, (i64, i64, String, Option<i64>, i64)>(
        "SELECT user_id, points, location, queue_position, consecutive_matches
         FROM camp_players WHERE camp_id = ?",
    )
    .bind(&camp.id)
    .fetch_all(pool)
    .await?;
    let players = player_rows
        .into_iter()
        .map(|p| PlayerRow {
            user_id: p.0,
            points: p.1,
            location: p.2,
            queue_position: p.3,
            consecutive_matches: p.4,
        })
        .collect();

    let match_rows = sqlx::query_as::<
        _,
        (
            i64,
            i64,
            String,
            i64,
            Option<i64>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT id, slot, status, refresh_used, message_id, party_id, party_code, winner_side
         FROM camp_matches WHERE camp_id = ? AND status IN ('voting', 'live') ORDER BY slot",
    )
    .bind(&camp.id)
    .fetch_all(pool)
    .await?;

    let mut open_matches = Vec::new();
    for m in match_rows {
        let row = MatchRow {
            id: m.0,
            slot: m.1,
            status: m.2,
            refresh_used: m.3 != 0,
            message_id: m.4,
            party_id: m.5,
            party_code: m.6,
            winner_side: m.7,
        };
        let mp = sqlx::query_as::<_, (i64, String, Option<i64>)>(
            "SELECT user_id, side, points_after FROM camp_match_players WHERE match_id = ?",
        )
        .bind(row.id)
        .fetch_all(pool)
        .await?;
        let match_players: Vec<MatchPlayerRow> = mp
            .into_iter()
            .map(|p| MatchPlayerRow {
                user_id: p.0,
                side: p.1,
                points_after: p.2,
            })
            .collect();
        let votes: Vec<i64> = sqlx::query_scalar("SELECT user_id FROM camp_refresh_votes WHERE match_id = ?")
            .bind(row.id)
            .fetch_all(pool)
            .await?;
        let expected = camp.layout().match_size;
        if match_players.len() != expected {
            tracing::error!(
                camp_id = %camp.id,
                match_id = row.id,
                count = match_players.len(),
                expected,
                "inconsistent match roster; skipping restore of this camp"
            );
            return Err(anyhow!(
                "inconsistent match {} has {} players (expected {})",
                row.id,
                match_players.len(),
                expected
            ));
        }
        open_matches.push(OpenMatch {
            row,
            players: match_players,
            votes,
        });
    }

    Ok(CampState {
        camp,
        players,
        open_matches,
    })
}

pub async fn add_players(pool: &SqlitePool, camp_id: &str, user_ids: &[i64]) -> Result<(usize, Vec<i64>)> {
    let state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp not found"))?;
    if state.camp.status != "signup" {
        return Err(anyhow!("players can only be added before the queue starts"));
    }
    let mut added = 0usize;
    let mut skipped = Vec::new();
    let mut seen: std::collections::HashSet<i64> =
        state.players.iter().map(|p| p.user_id).collect();
    let ts = now();
    let mut tx = pool.begin().await?;
    for user_id in user_ids {
        if !seen.insert(*user_id) {
            skipped.push(*user_id);
            continue;
        }
        let current = count_players(&mut tx, camp_id).await?;
        if current >= state.camp.target_size {
            skipped.push(*user_id);
            continue;
        }
        let pos = current;
        sqlx::query(
            "INSERT INTO camp_players (camp_id, user_id, points, location, queue_position, consecutive_matches, joined_at)
             VALUES (?, ?, 0, 'queued', ?, 0, ?)",
        )
        .bind(camp_id)
        .bind(user_id)
        .bind(pos)
        .bind(ts)
        .execute(&mut *tx)
        .await?;
        added += 1;
    }
    sqlx::query("UPDATE camps SET updated_at = ? WHERE id = ?")
        .bind(ts)
        .bind(camp_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((added, skipped))
}

async fn count_players(tx: &mut Transaction<'_, Sqlite>, camp_id: &str) -> Result<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM camp_players WHERE camp_id = ?")
        .bind(camp_id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(n)
}

pub async fn remove_player(pool: &SqlitePool, camp_id: &str, user_id: i64) -> Result<()> {
    let state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp not found"))?;
    if state.camp.status != "signup" {
        return Err(anyhow!("players can only be removed before the queue starts"));
    }
    let mut tx = pool.begin().await?;
    let deleted = sqlx::query("DELETE FROM camp_players WHERE camp_id = ? AND user_id = ?")
        .bind(camp_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if deleted == 0 {
        return Err(anyhow!("that player is not on the roster"));
    }
    reindex_signup_queue(&mut tx, camp_id).await?;
    sqlx::query("UPDATE camps SET updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(camp_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn reindex_signup_queue(tx: &mut Transaction<'_, Sqlite>, camp_id: &str) -> Result<()> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT user_id FROM camp_players WHERE camp_id = ? ORDER BY joined_at, user_id",
    )
    .bind(camp_id)
    .fetch_all(&mut **tx)
    .await?;
    for (i, user_id) in ids.into_iter().enumerate() {
        sqlx::query("UPDATE camp_players SET queue_position = ? WHERE camp_id = ? AND user_id = ?")
            .bind(i as i64)
            .bind(camp_id)
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

async fn write_queue_order(
    tx: &mut Transaction<'_, Sqlite>,
    camp_id: &str,
    ordered: &[QueuePlayer],
) -> Result<()> {
    for player in ordered {
        sqlx::query(
            "UPDATE camp_players SET location = 'queued', queue_position = ?, consecutive_matches = ?
             WHERE camp_id = ? AND user_id = ?",
        )
        .bind(player.queue_position)
        .bind(player.consecutive_matches)
        .bind(camp_id)
        .bind(player.user_id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn next_free_slot(tx: &mut Transaction<'_, Sqlite>, camp_id: &str) -> Result<Option<i64>> {
    let used: Vec<i64> = sqlx::query_scalar(
        "SELECT slot FROM camp_matches WHERE camp_id = ? AND status IN ('voting', 'live')",
    )
    .bind(camp_id)
    .fetch_all(&mut **tx)
    .await?;
    for slot in [1_i64, 2] {
        if !used.contains(&slot) {
            return Ok(Some(slot));
        }
    }
    Ok(None)
}

async fn camp_layout_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    camp_id: &str,
) -> Result<CampLayout> {
    let size: i64 = sqlx::query_scalar("SELECT target_size FROM camps WHERE id = ?")
        .bind(camp_id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(CampLayout::from_roster_size(size))
}

async fn pull_match_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    camp_id: &str,
) -> Result<Option<i64>> {
    let slot = match next_free_slot(tx, camp_id).await? {
        Some(s) => s,
        None => return Ok(None),
    };
    let layout = camp_layout_in_tx(tx, camp_id).await?;
    let queued: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT user_id, consecutive_matches FROM camp_players
         WHERE camp_id = ? AND location = 'queued'
         ORDER BY queue_position",
    )
    .bind(camp_id)
    .fetch_all(&mut **tx)
    .await?;
    if queued.len() < layout.match_size {
        return Ok(None);
    }
    let batch: Vec<i64> = queued.iter().take(layout.match_size).map(|q| q.0).collect();
    let (hiddenking, archmother) = queue::split_teams(batch, &mut rand::thread_rng());

    sqlx::query(
        "INSERT INTO camp_matches (camp_id, slot, status, refresh_used) VALUES (?, ?, 'voting', 0)",
    )
    .bind(camp_id)
    .bind(slot)
    .execute(&mut **tx)
    .await?;
    let match_id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(&mut **tx)
        .await?;

    let location = if slot == 1 { "match_1" } else { "match_2" };
    for user_id in &hiddenking {
        sqlx::query("INSERT INTO camp_match_players (match_id, user_id, side) VALUES (?, ?, 'hiddenking')")
            .bind(match_id)
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
        sqlx::query(
            "UPDATE camp_players SET location = ?, queue_position = NULL WHERE camp_id = ? AND user_id = ?",
        )
        .bind(location)
        .bind(camp_id)
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
    }
    for user_id in &archmother {
        sqlx::query("INSERT INTO camp_match_players (match_id, user_id, side) VALUES (?, ?, 'archmother')")
            .bind(match_id)
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
        sqlx::query(
            "UPDATE camp_players SET location = ?, queue_position = NULL WHERE camp_id = ? AND user_id = ?",
        )
        .bind(location)
        .bind(camp_id)
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
    }

    let remaining: Vec<i64> = sqlx::query_scalar(
        "SELECT user_id FROM camp_players WHERE camp_id = ? AND location = 'queued' ORDER BY queue_position",
    )
    .bind(camp_id)
    .fetch_all(&mut **tx)
    .await?;
    for (i, user_id) in remaining.into_iter().enumerate() {
        sqlx::query("UPDATE camp_players SET queue_position = ? WHERE camp_id = ? AND user_id = ?")
            .bind(i as i64)
            .bind(camp_id)
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(Some(match_id))
}

pub async fn start_queue(pool: &SqlitePool, camp_id: &str) -> Result<Vec<i64>> {
    let state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp not found"))?;
    if state.camp.status != "signup" {
        return Err(anyhow!("the queue has already started"));
    }
    if state.players.len() as i64 != state.camp.target_size {
        return Err(anyhow!(
            "need exactly {} players, have {}",
            state.camp.target_size,
            state.players.len()
        ));
    }

    let mut tx = pool.begin().await?;
    let mut ids: Vec<i64> = state.players.iter().map(|p| p.user_id).collect();
    queue::shuffle_ids(&mut ids, &mut rand::thread_rng());
    for (i, user_id) in ids.iter().enumerate() {
        sqlx::query(
            "UPDATE camp_players SET queue_position = ?, consecutive_matches = 0, location = 'queued'
             WHERE camp_id = ? AND user_id = ?",
        )
        .bind(i as i64)
        .bind(camp_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("UPDATE camps SET status = 'running', updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(camp_id)
        .execute(&mut *tx)
        .await?;

    let mut created = Vec::new();
    if let Some(id) = pull_match_in_tx(&mut tx, camp_id).await? {
        created.push(id);
    }
    if let Some(id) = pull_match_in_tx(&mut tx, camp_id).await? {
        created.push(id);
    }
    tx.commit().await?;
    Ok(created)
}

#[derive(Debug)]
pub enum RefreshOutcome {
    Recorded { hk: usize, am: usize, needed: usize },
    AlreadyUsed,
    NotInMatch,
    AlreadyVoted,
    Refreshed { hiddenking: Vec<i64>, archmother: Vec<i64> },
}

pub async fn vote_refresh(pool: &SqlitePool, match_id: i64, user_id: i64) -> Result<RefreshOutcome> {
    let mut tx = pool.begin().await?;
    let match_row = sqlx::query_as::<_, (String, i64, String)>(
        "SELECT status, refresh_used, camp_id FROM camp_matches WHERE id = ?",
    )
    .bind(match_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| anyhow!("match not found"))?;
    if match_row.0 != "voting" {
        return Err(anyhow!("refresh is only available before the lobby starts"));
    }
    if match_row.1 != 0 {
        return Ok(RefreshOutcome::AlreadyUsed);
    }
    let layout = camp_layout_in_tx(&mut tx, &match_row.2).await?;
    let needed = layout.refresh_votes;
    let side: Option<String> =
        sqlx::query_scalar("SELECT side FROM camp_match_players WHERE match_id = ? AND user_id = ?")
            .bind(match_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
    if side.is_none() {
        return Ok(RefreshOutcome::NotInMatch);
    }
    let already: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM camp_refresh_votes WHERE match_id = ? AND user_id = ?")
            .bind(match_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
    if already.is_some() {
        return Ok(RefreshOutcome::AlreadyVoted);
    }
    sqlx::query("INSERT INTO camp_refresh_votes (match_id, user_id) VALUES (?, ?)")
        .bind(match_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    let roster: Vec<(i64, String)> =
        sqlx::query_as("SELECT user_id, side FROM camp_match_players WHERE match_id = ?")
            .bind(match_id)
            .fetch_all(&mut *tx)
            .await?;
    let votes: Vec<i64> =
        sqlx::query_scalar("SELECT user_id FROM camp_refresh_votes WHERE match_id = ?")
            .bind(match_id)
            .fetch_all(&mut *tx)
            .await?;
    let hk: Vec<i64> = roster
        .iter()
        .filter(|p| p.1 == "hiddenking")
        .map(|p| p.0)
        .collect();
    let am: Vec<i64> = roster
        .iter()
        .filter(|p| p.1 == "archmother")
        .map(|p| p.0)
        .collect();
    let hk_votes = votes.iter().filter(|id| hk.contains(id)).count();
    let am_votes = votes.iter().filter(|id| am.contains(id)).count();
    if hk_votes < needed && am_votes < needed {
        tx.commit().await?;
        return Ok(RefreshOutcome::Recorded {
            hk: hk_votes,
            am: am_votes,
            needed,
        });
    }

    let batch: Vec<i64> = roster.iter().map(|p| p.0).collect();
    let (hiddenking, archmother) = queue::split_teams(batch, &mut rand::thread_rng());
    sqlx::query("DELETE FROM camp_match_players WHERE match_id = ?")
        .bind(match_id)
        .execute(&mut *tx)
        .await?;
    for user_id in &hiddenking {
        sqlx::query("INSERT INTO camp_match_players (match_id, user_id, side) VALUES (?, ?, 'hiddenking')")
            .bind(match_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }
    for user_id in &archmother {
        sqlx::query("INSERT INTO camp_match_players (match_id, user_id, side) VALUES (?, ?, 'archmother')")
            .bind(match_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE camp_matches SET refresh_used = 1 WHERE id = ?")
        .bind(match_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM camp_refresh_votes WHERE match_id = ?")
        .bind(match_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(RefreshOutcome::Refreshed {
        hiddenking,
        archmother,
    })
}

pub async fn start_live_match(
    pool: &SqlitePool,
    match_id: i64,
    party_id: Option<String>,
    party_code: Option<String>,
) -> Result<()> {
    let updated = sqlx::query(
        "UPDATE camp_matches SET status = 'live', party_id = ?, party_code = ? WHERE id = ? AND status = 'voting'",
    )
    .bind(party_id)
    .bind(party_code)
    .bind(match_id)
    .execute(pool)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(anyhow!("that match is not waiting to start"));
    }
    Ok(())
}

pub async fn report_winner(
    pool: &SqlitePool,
    camp_id: &str,
    match_id: i64,
    winner_side: &str,
) -> Result<Option<i64>> {
    let mut tx = pool.begin().await?;
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM camp_matches WHERE id = ? AND camp_id = ?")
            .bind(match_id)
            .bind(camp_id)
            .fetch_optional(&mut *tx)
            .await?;
    if status.as_deref() != Some("live") {
        return Err(anyhow!("that match is not live"));
    }

    let roster: Vec<(i64, String)> =
        sqlx::query_as("SELECT user_id, side FROM camp_match_players WHERE match_id = ?")
            .bind(match_id)
            .fetch_all(&mut *tx)
            .await?;
    let layout = camp_layout_in_tx(&mut tx, camp_id).await?;
    if roster.len() != layout.match_size {
        return Err(anyhow!("inconsistent match roster"));
    }

    let waiters_raw: Vec<(i64, i64, Option<i64>)> = sqlx::query_as(
        "SELECT user_id, consecutive_matches, queue_position FROM camp_players
         WHERE camp_id = ? AND location = 'queued' ORDER BY queue_position",
    )
    .bind(camp_id)
    .fetch_all(&mut *tx)
    .await?;

    let mut finished = Vec::new();
    for (user_id, side) in &roster {
        let won = side == winner_side;
        if won {
            sqlx::query("UPDATE camp_players SET points = points + 1 WHERE camp_id = ? AND user_id = ?")
                .bind(camp_id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query(
            "UPDATE camp_players SET consecutive_matches = consecutive_matches + 1
             WHERE camp_id = ? AND user_id = ?",
        )
        .bind(camp_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        let consecutive: i64 = sqlx::query_scalar(
            "SELECT consecutive_matches FROM camp_players WHERE camp_id = ? AND user_id = ?",
        )
        .bind(camp_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        let points: i64 =
            sqlx::query_scalar("SELECT points FROM camp_players WHERE camp_id = ? AND user_id = ?")
                .bind(camp_id)
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?;
        sqlx::query("UPDATE camp_match_players SET points_after = ? WHERE match_id = ? AND user_id = ?")
            .bind(points)
            .bind(match_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        finished.push(QueuePlayer {
            user_id: *user_id,
            consecutive_matches: consecutive,
            queue_position: 0,
            just_won: Some(won),
        });
    }

    let waiters: Vec<QueuePlayer> = waiters_raw
        .into_iter()
        .map(|w| QueuePlayer {
            user_id: w.0,
            consecutive_matches: w.1,
            queue_position: w.2.unwrap_or(0),
            just_won: None,
        })
        .collect();

    let ts = now();
    sqlx::query(
        "UPDATE camp_matches SET status = 'completed', winner_side = ?, completed_at = ? WHERE id = ?",
    )
    .bind(winner_side)
    .bind(ts)
    .bind(match_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE camps SET match_count = match_count + 1, updated_at = ? WHERE id = ?")
        .bind(ts)
        .bind(camp_id)
        .execute(&mut *tx)
        .await?;

    let ordered = queue::requeue(waiters, finished, &mut rand::thread_rng());
    write_queue_order(&mut tx, camp_id, &ordered).await?;

    let pulled = pull_match_in_tx(&mut tx, camp_id).await?;
    tx.commit().await?;
    Ok(pulled)
}

pub async fn cancel_camp(pool: &SqlitePool, camp_id: &str) -> Result<()> {
    sqlx::query("UPDATE camps SET status = 'cancelled', updated_at = ? WHERE id = ? AND status IN ('signup', 'running')")
        .bind(now())
        .bind(camp_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn resume_camp(pool: &SqlitePool, camp_id: &str, guild_id: i64) -> Result<CampState> {
    if load_active_for_guild(pool, guild_id).await?.is_some() {
        return Err(anyhow!("this server already has an active camp"));
    }
    let state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp not found"))?;
    if state.camp.guild_id != guild_id {
        return Err(anyhow!("that camp belongs to another server"));
    }
    if state.camp.status != "cancelled" {
        return Err(anyhow!("only a cancelled camp can be resumed"));
    }
    let restored = if state.open_matches.is_empty() && state.camp.match_count == 0 {
        "signup"
    } else {
        "running"
    };
    sqlx::query("UPDATE camps SET status = ?, updated_at = ? WHERE id = ?")
        .bind(restored)
        .bind(now())
        .bind(camp_id)
        .execute(pool)
        .await?;
    load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp vanished"))
}

pub async fn end_camp(pool: &SqlitePool, camp_id: &str) -> Result<Vec<PlayerRow>> {
    let state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp not found"))?;
    if state.camp.status != "running" {
        return Err(anyhow!("the camp is not running"));
    }
    if state.camp.match_count < 1 {
        return Err(anyhow!("end camp after at least one scored match"));
    }
    sqlx::query("UPDATE camps SET status = 'ended', updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(camp_id)
        .execute(pool)
        .await?;
    Ok(state.players)
}
