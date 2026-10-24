use anyhow::{anyhow, Result};
use rand::rngs::StdRng;
use rand::SeedableRng;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool, Transaction, Sqlite};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::draft::CreatedDraft;
use crate::queue::{self, CampLayout, QueuePlayer, Seat, TeamSplit};

#[derive(Clone, Debug)]
pub struct CampRow {
    pub id: String,
    pub guild_id: i64,
    pub channel_id: i64,
    pub board_message_id: Option<i64>,
    pub target_size: i64,
    pub match_size: i64,
    pub status: String,
    pub match_count: i64,
}

impl CampRow {
    pub fn layout(&self) -> CampLayout {
        CampLayout::from_match_size(self.match_size)
    }
}

#[derive(Clone, Debug)]
pub struct PlayerRow {
    pub user_id: i64,
    pub points: i64,
    pub location: String,
    pub queue_position: Option<i64>,
    pub consecutive_matches: i64,
    pub s_rank: bool,
}

impl PlayerRow {
    pub fn seat(&self) -> Seat {
        Seat {
            user_id: self.user_id,
            points: self.points,
            s_rank: self.s_rank,
        }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MatchRow {
    pub id: String,
    pub slot: i64,
    pub status: String,
    pub refresh_used: bool,
    pub message_id: Option<i64>,
    pub party_id: Option<String>,
    pub party_code: Option<String>,
    pub winner_side: Option<String>,
    pub is_final: bool,
    /// Public spectator link. Set once the draft starts.
    pub draft_url: Option<String>,
    pub draft_hiddenking_url: Option<String>,
    pub draft_archmother_url: Option<String>,
    pub draft_admin_url: Option<String>,
}

impl MatchRow {
    /// The private draft link for one side.
    pub fn team_draft_url(&self, side: &str) -> Option<&str> {
        match side {
            "hiddenking" => self.draft_hiddenking_url.as_deref(),
            "archmother" => self.draft_archmother_url.as_deref(),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MatchPlayerRow {
    pub user_id: i64,
    pub side: String,
    pub points_after: Option<i64>,
    pub is_captain: bool,
}

#[derive(Clone, Debug)]
pub struct RefreshVote {
    pub user_id: i64,
    pub refresh: bool,
}

#[derive(Clone, Debug)]
pub struct OpenMatch {
    pub row: MatchRow,
    pub players: Vec<MatchPlayerRow>,
    pub votes: Vec<RefreshVote>,
}

#[derive(Clone, Debug)]
pub struct CampState {
    pub camp: CampRow,
    pub players: Vec<PlayerRow>,
    pub open_matches: Vec<OpenMatch>,
    pub has_final: bool,
}

impl CampState {
    pub fn queued_count(&self) -> usize {
        self.players.iter().filter(|p| p.location == "queued").count()
    }

    pub fn can_create_match(&self) -> bool {
        self.camp.status == "running"
            && !self.has_final
            && self.open_matches.len() < 2
            && self.queued_count() >= self.camp.layout().match_size
    }

    pub fn can_create_final(&self) -> bool {
        self.camp.status == "running"
            && !self.has_final
            && self.open_matches.is_empty()
            && self.camp.match_count >= 1
            && self.players.len() >= self.camp.layout().match_size
    }
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
    migrate(&pool).await?;
    Ok(pool)
}

pub async fn migrate(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
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
    Err(anyhow!("não foi possível gerar um ID único para o camp"))
}

async fn generate_unique_match_id(tx: &mut Transaction<'_, Sqlite>) -> Result<String> {
    for _ in 0..16 {
        let id = queue::generate_camp_id(&mut rand::thread_rng());
        let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM camp_matches WHERE id = ?")
            .bind(&id)
            .fetch_optional(&mut **tx)
            .await?;
        if exists.is_none() {
            return Ok(id);
        }
    }
    Err(anyhow!("não foi possível gerar um ID único para a partida"))
}

pub async fn create_camp(
    pool: &SqlitePool,
    guild_id: i64,
    channel_id: i64,
    target_size: i64,
    match_size: i64,
) -> Result<CampRow> {
    let mut tx = pool.begin().await?;
    let id = generate_unique_id(&mut tx).await?;
    let ts = now();
    sqlx::query(
        "INSERT INTO camps (id, guild_id, channel_id, target_size, match_size, status, match_count, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 'signup', 0, ?, ?)",
    )
    .bind(&id)
    .bind(guild_id)
    .bind(channel_id)
    .bind(target_size)
    .bind(match_size)
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
        match_size,
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

pub async fn set_match_message_id(pool: &SqlitePool, match_id: &str, message_id: i64) -> Result<()> {
    sqlx::query("UPDATE camp_matches SET message_id = ? WHERE id = ?")
        .bind(message_id)
        .bind(match_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_draft(pool: &SqlitePool, match_id: &str, draft: &CreatedDraft) -> Result<()> {
    let updated = sqlx::query(
        "UPDATE camp_matches
         SET draft_url = ?, draft_hiddenking_url = ?, draft_archmother_url = ?, draft_admin_url = ?
         WHERE id = ? AND status = 'voting' AND refresh_used != 0 AND draft_url IS NULL",
    )
    .bind(&draft.spectator_url)
    .bind(&draft.hiddenking_url)
    .bind(&draft.archmother_url)
    .bind(&draft.admin_url)
    .bind(match_id)
    .execute(pool)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(anyhow!("não é possível iniciar o draft desta partida"));
    }
    Ok(())
}

pub async fn load_camp(pool: &SqlitePool, camp_id: &str) -> Result<Option<CampState>> {
    let camp = sqlx::query_as::<_, (String, i64, i64, Option<i64>, i64, i64, String, i64)>(
        "SELECT id, guild_id, channel_id, board_message_id, target_size, match_size, status, match_count FROM camps WHERE id = ?",
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
    let camp = sqlx::query_as::<_, (String, i64, i64, Option<i64>, i64, i64, String, i64)>(
        "SELECT id, guild_id, channel_id, board_message_id, target_size, match_size, status, match_count
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

#[derive(Clone, Debug)]
pub struct CampSummary {
    pub id: String,
    pub channel_id: i64,
    pub target_size: i64,
    pub match_size: i64,
    pub status: String,
    pub match_count: i64,
    pub created_at: i64,
}

/// Every camp of a server, newest first.
pub async fn list_camps_for_guild(pool: &SqlitePool, guild_id: i64) -> Result<Vec<CampSummary>> {
    let rows = sqlx::query_as::<_, (String, i64, i64, i64, String, i64, i64)>(
        "SELECT id, channel_id, target_size, match_size, status, match_count, created_at
         FROM camps WHERE guild_id = ? ORDER BY created_at DESC, id",
    )
    .bind(guild_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| CampSummary {
            id: r.0,
            channel_id: r.1,
            target_size: r.2,
            match_size: r.3,
            status: r.4,
            match_count: r.5,
            created_at: r.6,
        })
        .collect())
}

/// The server a match belongs to, through its camp.
pub async fn match_guild_id(pool: &SqlitePool, match_id: &str) -> Result<Option<i64>> {
    let guild_id = sqlx::query_scalar(
        "SELECT c.guild_id FROM camp_matches m JOIN camps c ON c.id = m.camp_id WHERE m.id = ?",
    )
    .bind(match_id)
    .fetch_optional(pool)
    .await?;
    Ok(guild_id)
}

#[derive(Clone, Debug)]
pub struct HistoricMatch {
    pub camp_id: String,
    pub row: MatchRow,
    pub players: Vec<MatchPlayerRow>,
}

pub async fn load_match_info(
    pool: &SqlitePool,
    match_id: &str,
    guild_id: i64,
) -> Result<HistoricMatch> {
    let row = sqlx::query_as::<
        _,
        (
            String,
            i64,
            String,
            i64,
            Option<i64>,
            Option<String>,
            Option<String>,
            Option<String>,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
            i64,
        ),
    >(
        "SELECT m.id, m.slot, m.status, m.refresh_used, m.message_id, m.party_id, m.party_code,
                m.winner_side, m.is_final, m.draft_url, m.draft_hiddenking_url,
                m.draft_archmother_url, m.draft_admin_url, m.camp_id, c.guild_id
         FROM camp_matches m
         JOIN camps c ON c.id = m.camp_id
         WHERE m.id = ?",
    )
    .bind(match_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("partida não encontrada"))?;
    if row.14 != guild_id {
        return Err(anyhow!("essa partida pertence a outro servidor"));
    }
    let match_row = MatchRow {
        id: row.0,
        slot: row.1,
        status: row.2,
        refresh_used: row.3 != 0,
        message_id: row.4,
        party_id: row.5,
        party_code: row.6,
        winner_side: row.7,
        is_final: row.8 != 0,
        draft_url: row.9,
        draft_hiddenking_url: row.10,
        draft_archmother_url: row.11,
        draft_admin_url: row.12,
    };
    let camp_id = row.13;
    let mp = sqlx::query_as::<_, (i64, String, Option<i64>, i64)>(
        "SELECT user_id, side, points_after, is_captain FROM camp_match_players WHERE match_id = ?",
    )
    .bind(match_id)
    .fetch_all(pool)
    .await?;
    let players = mp
        .into_iter()
        .map(|p| MatchPlayerRow {
            user_id: p.0,
            side: p.1,
            points_after: p.2,
            is_captain: p.3 != 0,
        })
        .collect();
    Ok(HistoricMatch {
        camp_id,
        row: match_row,
        players,
    })
}

pub async fn load_restore_camps(pool: &SqlitePool) -> Result<Vec<CampState>> {
    let rows = sqlx::query_as::<_, (String, i64, i64, Option<i64>, i64, i64, String, i64)>(
        "SELECT id, guild_id, channel_id, board_message_id, target_size, match_size, status, match_count
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
    c: (String, i64, i64, Option<i64>, i64, i64, String, i64),
) -> Result<CampState> {
    let camp = CampRow {
        id: c.0,
        guild_id: c.1,
        channel_id: c.2,
        board_message_id: c.3,
        target_size: c.4,
        match_size: c.5,
        status: c.6,
        match_count: c.7,
    };
    let player_rows = sqlx::query_as::<_, (i64, i64, String, Option<i64>, i64, i64)>(
        "SELECT user_id, points, location, queue_position, consecutive_matches, is_s_rank
         FROM camp_players WHERE camp_id = ?
         ORDER BY queue_position IS NULL, queue_position, joined_at, user_id",
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
            s_rank: p.5 != 0,
        })
        .collect();

    let match_rows = sqlx::query_as::<
        _,
        (
            String,
            i64,
            String,
            i64,
            Option<i64>,
            Option<String>,
            Option<String>,
            Option<String>,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT id, slot, status, refresh_used, message_id, party_id, party_code, winner_side, is_final,
                draft_url, draft_hiddenking_url, draft_archmother_url, draft_admin_url
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
            is_final: m.8 != 0,
            draft_url: m.9,
            draft_hiddenking_url: m.10,
            draft_archmother_url: m.11,
            draft_admin_url: m.12,
        };
        let mp = sqlx::query_as::<_, (i64, String, Option<i64>, i64)>(
            "SELECT user_id, side, points_after, is_captain FROM camp_match_players WHERE match_id = ?",
        )
        .bind(&row.id)
        .fetch_all(pool)
        .await?;
        let match_players: Vec<MatchPlayerRow> = mp
            .into_iter()
            .map(|p| MatchPlayerRow {
                user_id: p.0,
                side: p.1,
                points_after: p.2,
                is_captain: p.3 != 0,
            })
            .collect();
        let votes: Vec<RefreshVote> = sqlx::query_as::<_, (i64, String)>(
            "SELECT user_id, choice FROM camp_refresh_votes WHERE match_id = ?",
        )
        .bind(&row.id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|(user_id, choice)| RefreshVote {
            user_id,
            refresh: choice != "keep",
        })
        .collect();
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
                "partida inconsistente {}: tem {} jogadores (esperado {})",
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

    let has_final: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM camp_matches WHERE camp_id = ? AND is_final != 0 LIMIT 1",
    )
    .bind(&camp.id)
    .fetch_optional(pool)
    .await?;

    Ok(CampState {
        camp,
        players,
        open_matches,
        has_final: has_final.is_some(),
    })
}

pub async fn add_players(pool: &SqlitePool, camp_id: &str, user_ids: &[i64]) -> Result<(usize, Vec<i64>)> {
    let state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    if state.camp.status != "signup" {
        return Err(anyhow!("jogadores só podem ser adicionados antes de iniciar a fila"));
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
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    if state.camp.status != "signup" {
        return Err(anyhow!("jogadores só podem ser removidos antes de iniciar a fila"));
    }
    let mut tx = pool.begin().await?;
    let deleted = sqlx::query("DELETE FROM camp_players WHERE camp_id = ? AND user_id = ?")
        .bind(camp_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if deleted == 0 {
        return Err(anyhow!("esse jogador não está no elenco"));
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

/// Replace the camp's S-rank list. Returns the selected users who are not on the roster.
pub async fn set_s_rank(pool: &SqlitePool, camp_id: &str, user_ids: &[i64]) -> Result<Vec<i64>> {
    let state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    if !matches!(state.camp.status.as_str(), "signup" | "running") {
        return Err(anyhow!("o camp não está ativo"));
    }
    let not_on_roster: Vec<i64> = user_ids
        .iter()
        .copied()
        .filter(|id| !state.players.iter().any(|p| p.user_id == *id))
        .collect();
    let mut tx = pool.begin().await?;
    for player in &state.players {
        sqlx::query("UPDATE camp_players SET is_s_rank = ? WHERE camp_id = ? AND user_id = ?")
            .bind(user_ids.contains(&player.user_id))
            .bind(camp_id)
            .bind(player.user_id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE camps SET updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(camp_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(not_on_roster)
}

/// Flip one player's S-rank flag. Returns the new value.
pub async fn toggle_s_rank(pool: &SqlitePool, camp_id: &str, user_id: i64) -> Result<bool> {
    let updated: Option<i64> = sqlx::query_scalar(
        "UPDATE camp_players SET is_s_rank = 1 - is_s_rank
         WHERE camp_id = ? AND user_id = ?
         RETURNING is_s_rank",
    )
    .bind(camp_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    updated
        .map(|flag| flag != 0)
        .ok_or_else(|| anyhow!("esse jogador não está no elenco"))
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
    let match_size: i64 = sqlx::query_scalar("SELECT match_size FROM camps WHERE id = ?")
        .bind(camp_id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(CampLayout::from_match_size(match_size))
}

async fn insert_match_players(
    tx: &mut Transaction<'_, Sqlite>,
    match_id: &str,
    split: &TeamSplit,
) -> Result<()> {
    for (side, team, captain) in [
        ("hiddenking", &split.hiddenking, split.hiddenking_captain),
        ("archmother", &split.archmother, split.archmother_captain),
    ] {
        for user_id in team {
            sqlx::query(
                "INSERT INTO camp_match_players (match_id, user_id, side, is_captain) VALUES (?, ?, ?, ?)",
            )
            .bind(match_id)
            .bind(user_id)
            .bind(side)
            .bind(captain == Some(*user_id))
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

async fn pull_match_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    camp_id: &str,
) -> Result<Option<String>> {
    let slot = match next_free_slot(tx, camp_id).await? {
        Some(s) => s,
        None => return Ok(None),
    };
    let layout = camp_layout_in_tx(tx, camp_id).await?;
    let queued: Vec<Seat> = sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT user_id, points, is_s_rank FROM camp_players
         WHERE camp_id = ? AND location = 'queued'
         ORDER BY queue_position",
    )
    .bind(camp_id)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|(user_id, points, s_rank)| Seat {
        user_id,
        points,
        s_rank: s_rank != 0,
    })
    .collect();
    if queued.len() < layout.match_size {
        return Ok(None);
    }
    let seats = queue::pick_match_seats(&queued, layout.match_size);
    let split = queue::split_match(&seats, false, &mut rand::thread_rng());
    let match_id = generate_unique_match_id(tx).await?;

    sqlx::query(
        "INSERT INTO camp_matches (id, camp_id, slot, status, refresh_used, is_final) VALUES (?, ?, ?, 'voting', 0, 0)",
    )
    .bind(&match_id)
    .bind(camp_id)
    .bind(slot)
    .execute(&mut **tx)
    .await?;
    insert_match_players(tx, &match_id, &split).await?;

    let location = if slot == 1 { "match_1" } else { "match_2" };
    for user_id in split.hiddenking.iter().chain(&split.archmother) {
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
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    if state.camp.status != "signup" {
        return Err(anyhow!("a fila já foi iniciada"));
    }
    if state.players.len() as i64 != state.camp.target_size {
        return Err(anyhow!(
            "precisa de exatamente {} jogadores, tem {}",
            state.camp.target_size,
            state.players.len()
        ));
    }

    let mut tx = pool.begin().await?;
    let original: Vec<i64> = sqlx::query_scalar(
        "SELECT user_id FROM camp_players WHERE camp_id = ? ORDER BY joined_at, user_id",
    )
    .bind(camp_id)
    .fetch_all(&mut *tx)
    .await?;
    let mut ids = original.clone();
    let mut rng = StdRng::from_entropy();
    if ids.len() > 1 {
        for _ in 0..16 {
            queue::shuffle_ids(&mut ids, &mut rng);
            if ids != original {
                break;
            }
        }
    }
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
    sqlx::query("UPDATE camps SET status = 'running', updated_at = ? WHERE id = ? AND status = 'signup'")
        .bind(now())
        .bind(camp_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(ids)
}

pub async fn create_match(pool: &SqlitePool, camp_id: &str, guild_id: i64) -> Result<(String, String)> {
    let mut state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    if state.camp.guild_id != guild_id || state.camp.status != "running" {
        state = load_active_for_guild(pool, guild_id)
            .await?
            .ok_or_else(|| anyhow!("inicie a fila primeiro (clique em **Iniciar fila** no elenco)"))?;
    }
    if state.camp.status == "signup" {
        start_queue(pool, &state.camp.id).await?;
        state = load_camp(pool, &state.camp.id)
            .await?
            .ok_or_else(|| anyhow!("camp não encontrado após iniciar a fila"))?;
    }
    if state.camp.status != "running" {
        let status = match state.camp.status.as_str() {
            "signup" => "em inscrição",
            "ended" => "encerrado",
            "cancelled_signup" => "inscrição cancelada",
            "cancelled_running" => "cancelado em andamento",
            other => other,
        };
        return Err(anyhow!(
            "o camp `{}` está {}, não em andamento. Clique em **Iniciar fila** no elenco primeiro.",
            state.camp.id,
            status
        ));
    }
    let camp_id = state.camp.id.clone();
    if state.has_final {
        return Err(anyhow!("a partida final já foi criada"));
    }
    if state.open_matches.len() >= 2 {
        return Err(anyhow!("as duas vagas de partida já estão ocupadas"));
    }
    let need = state.camp.layout().match_size;
    let queued = state.queued_count();
    if queued < need {
        return Err(anyhow!(
            "precisa de {need} jogadores na fila para criar uma partida, tem {queued}"
        ));
    }
    let mut tx = pool.begin().await?;
    let match_id = pull_match_in_tx(&mut tx, &camp_id)
        .await?
        .ok_or_else(|| anyhow!("não foi possível criar uma partida a partir da fila"))?;
    tx.commit().await?;
    Ok((camp_id, match_id))
}

pub async fn create_final_match(
    pool: &SqlitePool,
    camp_id: &str,
    guild_id: i64,
) -> Result<(String, String)> {
    let mut state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    if state.camp.guild_id != guild_id || state.camp.status != "running" {
        state = load_active_for_guild(pool, guild_id)
            .await?
            .ok_or_else(|| anyhow!("nenhum camp ativo"))?;
    }
    if !state.can_create_final() {
        if state.has_final {
            return Err(anyhow!("a partida final já foi criada"));
        }
        if !state.open_matches.is_empty() {
            return Err(anyhow!("termine todas as partidas ao vivo antes da final"));
        }
        if state.camp.match_count < 1 {
            return Err(anyhow!("jogue pelo menos uma partida pontuada antes da final"));
        }
        return Err(anyhow!("ainda não é possível iniciar a partida final"));
    }

    let camp_id = state.camp.id.clone();
    let need = state.camp.layout().match_size;
    let scored: Vec<(i64, i64)> = state.players.iter().map(|p| (p.user_id, p.points)).collect();
    let mut rng = StdRng::from_entropy();
    let batch = queue::select_finalists(&scored, need, &mut rng);
    if batch.len() < need {
        return Err(anyhow!(
            "precisa de {need} jogadores na final, tem {}",
            batch.len()
        ));
    }

    let seats: Vec<Seat> = batch
        .iter()
        .filter_map(|user_id| state.players.iter().find(|p| p.user_id == *user_id))
        .map(PlayerRow::seat)
        .collect();
    let split = queue::split_match(&seats, true, &mut rng);

    let mut tx = pool.begin().await?;
    let slot = next_free_slot(&mut tx, &camp_id)
        .await?
        .ok_or_else(|| anyhow!("não há vaga livre para partida"))?;

    let match_id = generate_unique_match_id(&mut tx).await?;

    sqlx::query(
        "INSERT INTO camp_matches (id, camp_id, slot, status, refresh_used, is_final) VALUES (?, ?, ?, 'voting', 0, 1)",
    )
    .bind(&match_id)
    .bind(&camp_id)
    .bind(slot)
    .execute(&mut *tx)
    .await?;
    insert_match_players(&mut tx, &match_id, &split).await?;

    let location = if slot == 1 { "match_1" } else { "match_2" };
    for user_id in split.hiddenking.iter().chain(&split.archmother) {
        sqlx::query(
            "UPDATE camp_players SET location = ?, queue_position = NULL WHERE camp_id = ? AND user_id = ?",
        )
        .bind(location)
        .bind(&camp_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    }

    let remaining: Vec<i64> = sqlx::query_scalar(
        "SELECT user_id FROM camp_players WHERE camp_id = ? AND location = 'queued' ORDER BY queue_position",
    )
    .bind(&camp_id)
    .fetch_all(&mut *tx)
    .await?;
    for (i, user_id) in remaining.into_iter().enumerate() {
        sqlx::query("UPDATE camp_players SET queue_position = ? WHERE camp_id = ? AND user_id = ?")
            .bind(i as i64)
            .bind(&camp_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok((camp_id, match_id))
}

#[derive(Debug)]
pub enum RefreshOutcome {
    Recorded {
        hk: usize,
        am: usize,
        keep: usize,
        voted: usize,
        total: usize,
        needed: usize,
    },
    AlreadyUsed,
    NotInMatch,
    AlreadyVoted,
    Refreshed,
    Kept { voted: usize, total: usize },
}

pub async fn vote_refresh_choice(
    pool: &SqlitePool,
    match_id: &str,
    user_id: i64,
    want_refresh: bool,
) -> Result<RefreshOutcome> {
    let mut tx = pool.begin().await?;
    let match_row = sqlx::query_as::<_, (String, i64, String, i64)>(
        "SELECT status, refresh_used, camp_id, is_final FROM camp_matches WHERE id = ?",
    )
    .bind(match_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| anyhow!("partida não encontrada"))?;
    if match_row.0 != "voting" {
        return Err(anyhow!("atualizar os times só é possível antes de iniciar o lobby"));
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
    sqlx::query("INSERT INTO camp_refresh_votes (match_id, user_id, choice) VALUES (?, ?, ?)")
        .bind(match_id)
        .bind(user_id)
        .bind(if want_refresh { "refresh" } else { "keep" })
        .execute(&mut *tx)
        .await?;

    let roster: Vec<(i64, String)> =
        sqlx::query_as("SELECT user_id, side FROM camp_match_players WHERE match_id = ?")
            .bind(match_id)
            .fetch_all(&mut *tx)
            .await?;
    let votes: Vec<(i64, String)> =
        sqlx::query_as("SELECT user_id, choice FROM camp_refresh_votes WHERE match_id = ?")
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
    let refresh_ids: Vec<i64> = votes
        .iter()
        .filter(|v| v.1 != "keep")
        .map(|v| v.0)
        .collect();
    let keep = votes.iter().filter(|v| v.1 == "keep").count();
    let hk_votes = refresh_ids.iter().filter(|id| hk.contains(id)).count();
    let am_votes = refresh_ids.iter().filter(|id| am.contains(id)).count();
    let voted = votes.len();
    let total = roster.len();
    let voted_ids: Vec<i64> = votes.iter().map(|v| v.0).collect();
    let hk_unvoted = hk.iter().filter(|id| !voted_ids.contains(id)).count();
    let am_unvoted = am.iter().filter(|id| !voted_ids.contains(id)).count();

    if hk_votes >= needed || am_votes >= needed {
        let players: Vec<(i64, i64, i64)> = sqlx::query_as(
            "SELECT user_id, points, is_s_rank FROM camp_players WHERE camp_id = ?",
        )
        .bind(&match_row.2)
        .fetch_all(&mut *tx)
        .await?;
        let seats: Vec<Seat> = roster
            .iter()
            .map(|(user_id, _)| {
                let (points, s_rank) = players
                    .iter()
                    .find(|p| p.0 == *user_id)
                    .map(|p| (p.1, p.2 != 0))
                    .unwrap_or((0, false));
                Seat {
                    user_id: *user_id,
                    points,
                    s_rank,
                }
            })
            .collect();
        let split = queue::split_match(&seats, match_row.3 != 0, &mut rand::thread_rng());
        sqlx::query("DELETE FROM camp_match_players WHERE match_id = ?")
            .bind(match_id)
            .execute(&mut *tx)
            .await?;
        insert_match_players(&mut tx, match_id, &split).await?;
        sqlx::query("UPDATE camp_matches SET refresh_used = 1 WHERE id = ?")
            .bind(match_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM camp_refresh_votes WHERE match_id = ?")
            .bind(match_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(RefreshOutcome::Refreshed);
    }

    let refresh_impossible =
        hk_votes + hk_unvoted < needed && am_votes + am_unvoted < needed;
    if refresh_impossible || voted == total {
        sqlx::query("UPDATE camp_matches SET refresh_used = 1 WHERE id = ?")
            .bind(match_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(RefreshOutcome::Kept { voted, total });
    }

    tx.commit().await?;
    Ok(RefreshOutcome::Recorded {
        hk: hk_votes,
        am: am_votes,
        keep,
        voted,
        total,
        needed,
    })
}

pub async fn start_live_match(
    pool: &SqlitePool,
    match_id: &str,
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
        return Err(anyhow!("essa partida não está esperando para começar"));
    }
    Ok(())
}

pub async fn report_winner(
    pool: &SqlitePool,
    camp_id: &str,
    match_id: &str,
    winner_side: &str,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM camp_matches WHERE id = ? AND camp_id = ?")
            .bind(match_id)
            .bind(camp_id)
            .fetch_optional(&mut *tx)
            .await?;
    if status.as_deref() != Some("live") {
        return Err(anyhow!("essa partida não está ao vivo"));
    }

    let roster: Vec<(i64, String)> =
        sqlx::query_as("SELECT user_id, side FROM camp_match_players WHERE match_id = ?")
            .bind(match_id)
            .fetch_all(&mut *tx)
            .await?;
    let layout = camp_layout_in_tx(&mut tx, camp_id).await?;
    if roster.len() != layout.match_size {
        return Err(anyhow!("elenco da partida inconsistente"));
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

    tx.commit().await?;
    Ok(())
}

struct CorrectableMatch {
    slot: i64,
    is_final: bool,
    winner_side: String,
    party_code: Option<String>,
    roster: Vec<(i64, String)>,
}

fn other_side(side: &str) -> Result<&'static str> {
    match side {
        "hiddenking" => Ok("archmother"),
        "archmother" => Ok("hiddenking"),
        _ => Err(anyhow!("vencedor inválido")),
    }
}

async fn require_correctable_match(
    tx: &mut Transaction<'_, Sqlite>,
    camp_id: &str,
    match_id: &str,
    require_free_slot: bool,
) -> Result<CorrectableMatch> {
    let camp_status: Option<String> = sqlx::query_scalar("SELECT status FROM camps WHERE id = ?")
        .bind(camp_id)
        .fetch_optional(&mut **tx)
        .await?;
    match camp_status.as_deref() {
        Some("running") => {}
        Some(_) => return Err(anyhow!("o camp não está em andamento")),
        None => return Err(anyhow!("camp não encontrado")),
    }

    let row: Option<(String, i64, Option<String>, Option<i64>, i64, Option<String>)> =
        sqlx::query_as(
            "SELECT status, slot, winner_side, completed_at, is_final, party_code
             FROM camp_matches WHERE id = ? AND camp_id = ?",
        )
        .bind(match_id)
        .bind(camp_id)
        .fetch_optional(&mut **tx)
        .await?;
    let Some((status, slot, winner_side, _completed_at, is_final, party_code)) = row else {
        return Err(anyhow!("partida não encontrada"));
    };
    let Some(winner_side) = winner_side else {
        return Err(anyhow!("não é mais possível corrigir esta partida"));
    };
    if status != "completed" {
        return Err(anyhow!("não é mais possível corrigir esta partida"));
    }

    let latest: Option<String> = sqlx::query_scalar(
        "SELECT id FROM camp_matches
         WHERE camp_id = ? AND status = 'completed' AND completed_at IS NOT NULL
         ORDER BY completed_at DESC, id DESC LIMIT 1",
    )
    .bind(camp_id)
    .fetch_optional(&mut **tx)
    .await?;
    if latest.as_deref() != Some(match_id) {
        return Err(anyhow!("não é mais possível corrigir esta partida"));
    }

    let roster: Vec<(i64, String)> =
        sqlx::query_as("SELECT user_id, side FROM camp_match_players WHERE match_id = ?")
            .bind(match_id)
            .fetch_all(&mut **tx)
            .await?;
    if roster.is_empty() {
        return Err(anyhow!("elenco da partida inconsistente"));
    }
    for (user_id, _) in &roster {
        let location: Option<String> = sqlx::query_scalar(
            "SELECT location FROM camp_players WHERE camp_id = ? AND user_id = ?",
        )
        .bind(camp_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?;
        if location.as_deref() != Some("queued") {
            return Err(anyhow!(
                "não é mais possível corrigir esta partida (os jogadores já foram para outra partida)"
            ));
        }
    }

    if require_free_slot {
        let occupied: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM camp_matches
             WHERE camp_id = ? AND slot = ? AND status IN ('voting', 'live') LIMIT 1",
        )
        .bind(camp_id)
        .bind(slot)
        .fetch_optional(&mut **tx)
        .await?;
        if occupied.is_some() {
            return Err(anyhow!(
                "não é mais possível reabrir esta partida (o slot já está em uso)"
            ));
        }
    }

    Ok(CorrectableMatch {
        slot,
        is_final: is_final != 0,
        winner_side,
        party_code,
        roster,
    })
}

async fn reindex_queued(tx: &mut Transaction<'_, Sqlite>, camp_id: &str) -> Result<()> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT user_id, consecutive_matches FROM camp_players
         WHERE camp_id = ? AND location = 'queued' ORDER BY queue_position",
    )
    .bind(camp_id)
    .fetch_all(&mut **tx)
    .await?;
    let ordered: Vec<QueuePlayer> = rows
        .into_iter()
        .enumerate()
        .map(|(i, (user_id, consecutive_matches))| QueuePlayer {
            user_id,
            consecutive_matches,
            queue_position: i as i64,
            just_won: None,
        })
        .collect();
    write_queue_order(tx, camp_id, &ordered).await
}

pub struct CompletedMatchView {
    pub is_final: bool,
    pub winner_side: String,
    pub party_code: Option<String>,
}

pub async fn correct_winner(
    pool: &SqlitePool,
    camp_id: &str,
    match_id: &str,
) -> Result<CompletedMatchView> {
    let mut tx = pool.begin().await?;
    let current = require_correctable_match(&mut tx, camp_id, match_id, false).await?;
    let new_side = other_side(&current.winner_side)?;
    let old_side = current.winner_side.clone();

    for (user_id, side) in &current.roster {
        if side == &old_side {
            sqlx::query(
                "UPDATE camp_players SET points = MAX(0, points - 1) WHERE camp_id = ? AND user_id = ?",
            )
            .bind(camp_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        }
        if side == new_side {
            sqlx::query("UPDATE camp_players SET points = points + 1 WHERE camp_id = ? AND user_id = ?")
                .bind(camp_id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }
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
    }

    sqlx::query("UPDATE camp_matches SET winner_side = ? WHERE id = ?")
        .bind(new_side)
        .bind(match_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE camps SET updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(camp_id)
        .execute(&mut *tx)
        .await?;

    let roster_ids: Vec<i64> = current.roster.iter().map(|(id, _)| *id).collect();
    let waiters_raw: Vec<(i64, i64, Option<i64>)> = sqlx::query_as(
        "SELECT user_id, consecutive_matches, queue_position FROM camp_players
         WHERE camp_id = ? AND location = 'queued' ORDER BY queue_position",
    )
    .bind(camp_id)
    .fetch_all(&mut *tx)
    .await?;
    let waiters: Vec<QueuePlayer> = waiters_raw
        .into_iter()
        .filter(|(user_id, _, _)| !roster_ids.contains(user_id))
        .map(|w| QueuePlayer {
            user_id: w.0,
            consecutive_matches: w.1,
            queue_position: w.2.unwrap_or(0),
            just_won: None,
        })
        .collect();

    let mut finished = Vec::new();
    for (user_id, side) in &current.roster {
        let consecutive: i64 = sqlx::query_scalar(
            "SELECT consecutive_matches FROM camp_players WHERE camp_id = ? AND user_id = ?",
        )
        .bind(camp_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        finished.push(QueuePlayer {
            user_id: *user_id,
            consecutive_matches: consecutive,
            queue_position: 0,
            just_won: Some(side == new_side),
        });
    }

    let ordered = queue::requeue(waiters, finished, &mut rand::thread_rng());
    write_queue_order(&mut tx, camp_id, &ordered).await?;
    tx.commit().await?;
    Ok(CompletedMatchView {
        is_final: current.is_final,
        winner_side: new_side.to_string(),
        party_code: current.party_code,
    })
}

pub async fn reopen_match(pool: &SqlitePool, camp_id: &str, match_id: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    let current = require_correctable_match(&mut tx, camp_id, match_id, true).await?;
    let location = if current.slot == 1 {
        "match_1"
    } else {
        "match_2"
    };

    for (user_id, side) in &current.roster {
        if side == &current.winner_side {
            sqlx::query(
                "UPDATE camp_players SET points = MAX(0, points - 1) WHERE camp_id = ? AND user_id = ?",
            )
            .bind(camp_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "UPDATE camp_players SET consecutive_matches = MAX(0, consecutive_matches - 1),
             location = ?, queue_position = NULL
             WHERE camp_id = ? AND user_id = ?",
        )
        .bind(location)
        .bind(camp_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "UPDATE camp_matches SET status = 'live', winner_side = NULL, completed_at = NULL WHERE id = ?",
    )
    .bind(match_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE camps SET match_count = MAX(0, match_count - 1), updated_at = ? WHERE id = ?",
    )
    .bind(now())
    .bind(camp_id)
    .execute(&mut *tx)
    .await?;

    reindex_queued(&mut tx, camp_id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn other_completed_message_ids(
    pool: &SqlitePool,
    camp_id: &str,
    except_match_id: &str,
) -> Result<Vec<i64>> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT message_id FROM camp_matches
         WHERE camp_id = ? AND status = 'completed' AND id != ? AND message_id IS NOT NULL",
    )
    .bind(camp_id)
    .bind(except_match_id)
    .fetch_all(pool)
    .await?;
    Ok(ids)
}

pub async fn cancel_camp(pool: &SqlitePool, camp_id: &str) -> Result<()> {
    let state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    let status = if state.camp.status == "signup" {
        "cancelled_signup"
    } else {
        "cancelled_running"
    };
    sqlx::query("UPDATE camps SET status = ?, updated_at = ? WHERE id = ? AND status IN ('signup', 'running')")
        .bind(status)
        .bind(now())
        .bind(camp_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn resume_camp(pool: &SqlitePool, camp_id: &str, guild_id: i64) -> Result<CampState> {
    if load_active_for_guild(pool, guild_id).await?.is_some() {
        return Err(anyhow!("este servidor já tem um camp ativo"));
    }
    let state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    if state.camp.guild_id != guild_id {
        return Err(anyhow!("esse camp pertence a outro servidor"));
    }
    if !matches!(
        state.camp.status.as_str(),
        "cancelled" | "cancelled_signup" | "cancelled_running"
    ) {
        return Err(anyhow!("só um camp cancelado pode ser retomado"));
    }
    let restored = match state.camp.status.as_str() {
        "cancelled_signup" => "signup",
        "cancelled_running" => "running",
        _ if state.open_matches.is_empty() && state.camp.match_count == 0 => "signup",
        _ => "running",
    };
    sqlx::query("UPDATE camps SET status = ?, updated_at = ? WHERE id = ?")
        .bind(restored)
        .bind(now())
        .bind(camp_id)
        .execute(pool)
        .await?;
    load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("o camp desapareceu"))
}

pub async fn end_camp(pool: &SqlitePool, camp_id: &str) -> Result<Vec<PlayerRow>> {
    let state = load_camp(pool, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    if state.camp.status != "running" {
        return Err(anyhow!("o camp não está em andamento"));
    }
    if state.camp.match_count < 1 {
        return Err(anyhow!("encerre o camp depois de pelo menos uma partida pontuada"));
    }
    sqlx::query("UPDATE camps SET status = 'ended', updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(camp_id)
        .execute(pool)
        .await?;
    Ok(state.players)
}
