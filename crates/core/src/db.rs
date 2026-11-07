use anyhow::{anyhow, Result};
use rand::rngs::StdRng;
use rand::SeedableRng;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool, Transaction, Sqlite};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use std::collections::HashMap;

use crate::draft::CreatedDraft;
use crate::queue::{self, CampLayout, Candidate, QueuePlayer, Seat, TeamSplit};
use crate::settings::GuildSettings;

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

#[derive(Clone, Debug, sqlx::FromRow)]
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
    /// The captains' invite link: whoever opens it can take a slot on either side.
    pub draft_join_url: Option<String>,
    /// An admin ended the refresh vote early and kept the teams.
    pub vote_closed_by_admin: bool,
    /// The draft started without the draft tool: the admin shares the draft link themselves.
    pub draft_manual: bool,
    /// An admin ended the refresh vote early by re-sorting the teams.
    pub refreshed_by_admin: bool,
    /// Picked by the admin with the result; breaks ties in points in the final ranking.
    pub mvp_user_id: Option<i64>,
}

/// The columns a `MatchRow` reads, for `SELECT {MATCH_COLUMNS} FROM camp_matches m`.
const MATCH_COLUMNS: &str = "m.id, m.slot, m.status, m.refresh_used, m.message_id, m.party_id, m.party_code,
     m.winner_side, m.is_final, m.draft_url, m.draft_join_url, m.vote_closed_by_admin,
     m.draft_manual, m.refreshed_by_admin, m.mvp_user_id";

impl MatchRow {
    /// The draft step happened, on the draft tool or by hand.
    pub fn draft_started(&self) -> bool {
        self.draft_url.is_some() || self.draft_manual
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
         SET draft_url = ?, draft_join_url = ?
         WHERE id = ? AND status = 'voting' AND refresh_used != 0 AND draft_url IS NULL AND draft_manual = 0",
    )
    .bind(&draft.spectator_url)
    .bind(&draft.join_url)
    .bind(match_id)
    .execute(pool)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(anyhow!("não é possível iniciar o draft desta partida"));
    }
    Ok(())
}

/// Starts the draft step without the draft tool: no links, the admin shares one themselves.
pub async fn set_manual_draft(pool: &SqlitePool, match_id: &str) -> Result<()> {
    let updated = sqlx::query(
        "UPDATE camp_matches SET draft_manual = 1
         WHERE id = ? AND status = 'voting' AND refresh_used != 0 AND draft_url IS NULL AND draft_manual = 0",
    )
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
    pub board_message_id: Option<i64>,
    pub target_size: i64,
    pub match_size: i64,
    pub status: String,
    pub match_count: i64,
    pub created_at: i64,
}

/// Every camp of a server, newest first.
pub async fn list_camps_for_guild(pool: &SqlitePool, guild_id: i64) -> Result<Vec<CampSummary>> {
    let rows = sqlx::query_as::<_, (String, i64, i64, i64, String, i64, i64, Option<i64>)>(
        "SELECT id, channel_id, target_size, match_size, status, match_count, created_at, board_message_id
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
            board_message_id: r.7,
        })
        .collect())
}

#[derive(Clone, Debug)]
pub struct RecentMatch {
    pub id: String,
    pub winner_side: String,
    pub is_final: bool,
    pub completed_at: i64,
    pub mvp_user_id: Option<i64>,
    pub players: Vec<MatchPlayerRow>,
}

/// A camp's finished matches, newest first.
pub async fn recent_matches(pool: &SqlitePool, camp_id: &str, limit: i64) -> Result<Vec<RecentMatch>> {
    let rows = sqlx::query_as::<_, (String, String, i64, i64, Option<i64>)>(
        "SELECT id, winner_side, is_final, completed_at, mvp_user_id FROM camp_matches
         WHERE camp_id = ? AND status = 'completed' AND winner_side IS NOT NULL AND completed_at IS NOT NULL
         ORDER BY completed_at DESC, id DESC LIMIT ?",
    )
    .bind(camp_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for (id, winner_side, is_final, completed_at, mvp_user_id) in rows {
        let players = sqlx::query_as::<_, (i64, String, Option<i64>, i64)>(
            "SELECT user_id, side, points_after, is_captain FROM camp_match_players WHERE match_id = ?",
        )
        .bind(&id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|p| MatchPlayerRow {
            user_id: p.0,
            side: p.1,
            points_after: p.2,
            is_captain: p.3 != 0,
        })
        .collect();
        out.push(RecentMatch {
            id,
            winner_side,
            is_final: is_final != 0,
            completed_at,
            mvp_user_id,
            players,
        });
    }
    Ok(out)
}

/// A match with everything the admin page shows about it.
#[derive(Clone, Debug)]
pub struct MatchDetail {
    pub row: MatchRow,
    /// Finished matches only: position in the order they ended, 1 being the first.
    pub number: Option<i64>,
    pub completed_at: Option<i64>,
    pub players: Vec<MatchPlayerRow>,
    pub votes: Vec<RefreshVote>,
}

/// Every match of a camp: open ones by slot, then finished ones newest first.
pub async fn list_matches(pool: &SqlitePool, camp_id: &str) -> Result<Vec<MatchDetail>> {
    #[derive(sqlx::FromRow)]
    struct Listed {
        #[sqlx(flatten)]
        row: MatchRow,
        completed_at: Option<i64>,
    }
    let rows: Vec<Listed> = sqlx::query_as(&format!(
        "SELECT {MATCH_COLUMNS}, m.completed_at FROM camp_matches m WHERE m.camp_id = ?"
    ))
    .bind(camp_id)
    .fetch_all(pool)
    .await?;

    let mut matches = Vec::with_capacity(rows.len());
    for m in rows {
        let players = sqlx::query_as::<_, (i64, String, Option<i64>, i64)>(
            "SELECT user_id, side, points_after, is_captain FROM camp_match_players WHERE match_id = ?",
        )
        .bind(&m.row.id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|p| MatchPlayerRow {
            user_id: p.0,
            side: p.1,
            points_after: p.2,
            is_captain: p.3 != 0,
        })
        .collect();
        let votes = sqlx::query_as::<_, (i64, String)>(
            "SELECT user_id, choice FROM camp_refresh_votes WHERE match_id = ?",
        )
        .bind(&m.row.id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|(user_id, choice)| RefreshVote {
            user_id,
            refresh: choice != "keep",
        })
        .collect();
        matches.push(MatchDetail {
            row: m.row,
            number: None,
            completed_at: m.completed_at,
            players,
            votes,
        });
    }

    let finished = |m: &MatchDetail| m.row.status == "completed" && m.completed_at.is_some();
    let mut order: Vec<(i64, String)> = matches
        .iter()
        .filter(|m| finished(m))
        .map(|m| (m.completed_at.unwrap_or(0), m.row.id.clone()))
        .collect();
    order.sort();
    for m in matches.iter_mut().filter(|m| finished(m)) {
        m.number = order
            .iter()
            .position(|(_, id)| *id == m.row.id)
            .map(|i| i as i64 + 1);
    }
    matches.sort_by(|a, b| match (finished(a), finished(b)) {
        (false, false) => a.row.slot.cmp(&b.row.slot),
        (false, true) => std::cmp::Ordering::Less,
        (true, false) => std::cmp::Ordering::Greater,
        (true, true) => b.number.cmp(&a.number),
    });
    Ok(matches)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerStats {
    /// Finished matches only.
    pub played: i64,
    pub wins: i64,
    /// Points added (or, below zero, taken) by hand by admins.
    pub adjusted: i64,
    /// Finished matches where this player was the MVP.
    pub mvps: i64,
}

/// Matches played and won, and manual point changes, per player of a camp.
pub async fn player_stats(pool: &SqlitePool, camp_id: &str) -> Result<HashMap<i64, PlayerStats>> {
    let rows: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT mp.user_id, COUNT(*), SUM(CASE WHEN mp.side = m.winner_side THEN 1 ELSE 0 END)
         FROM camp_match_players mp
         JOIN camp_matches m ON m.id = mp.match_id
         WHERE m.camp_id = ? AND m.status = 'completed'
         GROUP BY mp.user_id",
    )
    .bind(camp_id)
    .fetch_all(pool)
    .await?;
    let mut stats: HashMap<i64, PlayerStats> = rows
        .into_iter()
        .map(|(user_id, played, wins)| (user_id, PlayerStats { played, wins, ..PlayerStats::default() }))
        .collect();
    let adjustments: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT user_id, SUM(delta) FROM camp_point_adjustments WHERE camp_id = ? GROUP BY user_id",
    )
    .bind(camp_id)
    .fetch_all(pool)
    .await?;
    for (user_id, adjusted) in adjustments {
        stats.entry(user_id).or_default().adjusted = adjusted;
    }
    let mvps: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT mvp_user_id, COUNT(*) FROM camp_matches
         WHERE camp_id = ? AND status = 'completed' AND mvp_user_id IS NOT NULL
         GROUP BY mvp_user_id",
    )
    .bind(camp_id)
    .fetch_all(pool)
    .await?;
    for (user_id, count) in mvps {
        stats.entry(user_id).or_default().mvps = count;
    }
    Ok(stats)
}

/// The most points one manual change can add or take.
pub const MAX_POINT_ADJUSTMENT: i64 = 50;

/// Adds `delta` points to a player by hand (negative takes them away), while the camp runs.
/// Returns the player's new total.
pub async fn adjust_points(pool: &SqlitePool, camp_id: &str, user_id: i64, delta: i64) -> Result<i64> {
    if delta == 0 || delta.abs() > MAX_POINT_ADJUSTMENT {
        return Err(anyhow!("o ajuste precisa ser de 1 a {MAX_POINT_ADJUSTMENT} pontos"));
    }
    let mut tx = pool.begin().await?;
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM camps WHERE id = ?")
        .bind(camp_id)
        .fetch_optional(&mut *tx)
        .await?;
    if status.as_deref() != Some("running") {
        return Err(anyhow!("os pontos só podem ser ajustados com o camp em andamento"));
    }
    let points: Option<i64> =
        sqlx::query_scalar("SELECT points FROM camp_players WHERE camp_id = ? AND user_id = ?")
            .bind(camp_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some(points) = points else {
        return Err(anyhow!("esse jogador não está no camp"));
    };
    let total = points + delta;
    if total < 0 {
        return Err(anyhow!("o jogador tem {points} pontos; não dá para tirar {}", -delta));
    }
    sqlx::query("UPDATE camp_players SET points = ? WHERE camp_id = ? AND user_id = ?")
        .bind(total)
        .bind(camp_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO camp_point_adjustments (camp_id, user_id, delta, created_at) VALUES (?, ?, ?, ?)")
        .bind(camp_id)
        .bind(user_id)
        .bind(delta)
        .bind(now())
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE camps SET updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(camp_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(total)
}

/// Puts the queue in this order. `user_ids` must be exactly the players queued right now, so a
/// page showing an outdated queue can't drop or duplicate anyone.
pub async fn reorder_queue(pool: &SqlitePool, camp_id: &str, user_ids: &[i64]) -> Result<()> {
    let mut tx = pool.begin().await?;
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM camps WHERE id = ?")
        .bind(camp_id)
        .fetch_optional(&mut *tx)
        .await?;
    if status.as_deref() != Some("running") {
        return Err(anyhow!("a fila só existe com o camp em andamento"));
    }
    let mut queued: Vec<i64> = sqlx::query_scalar(
        "SELECT user_id FROM camp_players WHERE camp_id = ? AND location = 'queued'",
    )
    .bind(camp_id)
    .fetch_all(&mut *tx)
    .await?;
    let mut wanted = user_ids.to_vec();
    queued.sort_unstable();
    wanted.sort_unstable();
    if queued != wanted {
        return Err(anyhow!("a fila mudou enquanto você reordenava; confira e tente de novo"));
    }
    for (position, user_id) in user_ids.iter().enumerate() {
        sqlx::query("UPDATE camp_players SET queue_position = ? WHERE camp_id = ? AND user_id = ?")
            .bind(position as i64)
            .bind(camp_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE camps SET updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(camp_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// A server's settings, and whether they were saved on the admin page. A server that never
/// saved any runs on `defaults`, the `.env` values (`GuildSettings::from_config`).
pub async fn guild_settings(
    pool: &SqlitePool,
    guild_id: i64,
    defaults: &GuildSettings,
) -> Result<(GuildSettings, bool)> {
    let row = sqlx::query_as::<
        _,
        (Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>, i64, Option<String>, i64, i64),
    >(
        "SELECT queue_voice_id, match1_hiddenking_voice_id, match1_archmother_voice_id,
                match2_hiddenking_voice_id, match2_archmother_voice_id, create_lobby, server_region,
                draft_timer_seconds, use_draft_tool
         FROM guild_settings WHERE guild_id = ?",
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await?;
    let Some(r) = row else {
        return Ok((defaults.clone(), false));
    };
    let id = |v: Option<i64>| v.map(|v| v as u64);
    Ok((
        GuildSettings {
            queue_voice_id: id(r.0),
            match1_hiddenking_voice_id: id(r.1),
            match1_archmother_voice_id: id(r.2),
            match2_hiddenking_voice_id: id(r.3),
            match2_archmother_voice_id: id(r.4),
            create_lobby: r.5 != 0,
            server_region: r.6,
            use_draft_tool: r.8 != 0,
            draft_timer_seconds: r.7 as u32,
        },
        true,
    ))
}

pub async fn save_guild_settings(pool: &SqlitePool, guild_id: i64, settings: &GuildSettings) -> Result<()> {
    let id = |v: Option<u64>| v.map(|v| v as i64);
    sqlx::query(
        "INSERT INTO guild_settings (guild_id, queue_voice_id, match1_hiddenking_voice_id,
             match1_archmother_voice_id, match2_hiddenking_voice_id, match2_archmother_voice_id,
             create_lobby, server_region, draft_timer_seconds, use_draft_tool, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(guild_id) DO UPDATE SET
             queue_voice_id = excluded.queue_voice_id,
             match1_hiddenking_voice_id = excluded.match1_hiddenking_voice_id,
             match1_archmother_voice_id = excluded.match1_archmother_voice_id,
             match2_hiddenking_voice_id = excluded.match2_hiddenking_voice_id,
             match2_archmother_voice_id = excluded.match2_archmother_voice_id,
             create_lobby = excluded.create_lobby,
             server_region = excluded.server_region,
             draft_timer_seconds = excluded.draft_timer_seconds,
             use_draft_tool = excluded.use_draft_tool,
             updated_at = excluded.updated_at",
    )
    .bind(guild_id)
    .bind(id(settings.queue_voice_id))
    .bind(id(settings.match1_hiddenking_voice_id))
    .bind(id(settings.match1_archmother_voice_id))
    .bind(id(settings.match2_hiddenking_voice_id))
    .bind(id(settings.match2_archmother_voice_id))
    .bind(settings.create_lobby)
    .bind(&settings.server_region)
    .bind(settings.draft_timer_seconds as i64)
    .bind(settings.use_draft_tool)
    .bind(now())
    .execute(pool)
    .await?;
    Ok(())
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
    #[derive(sqlx::FromRow)]
    struct Found {
        #[sqlx(flatten)]
        row: MatchRow,
        camp_id: String,
        guild_id: i64,
    }
    let found: Found = sqlx::query_as(&format!(
        "SELECT {MATCH_COLUMNS}, m.camp_id, c.guild_id
         FROM camp_matches m
         JOIN camps c ON c.id = m.camp_id
         WHERE m.id = ?"
    ))
    .bind(match_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("partida não encontrada"))?;
    if found.guild_id != guild_id {
        return Err(anyhow!("essa partida pertence a outro servidor"));
    }
    let match_row = found.row;
    let camp_id = found.camp_id;
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

    let match_rows: Vec<MatchRow> = sqlx::query_as(&format!(
        "SELECT {MATCH_COLUMNS} FROM camp_matches m
         WHERE m.camp_id = ? AND m.status IN ('voting', 'live') ORDER BY m.slot"
    ))
    .bind(&camp.id)
    .fetch_all(pool)
    .await?;

    let mut open_matches = Vec::new();
    for row in match_rows {
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

/// Set one player's S-rank flag. Works during signup and while running; matches created
/// afterwards use it.
pub async fn set_player_s_rank(pool: &SqlitePool, camp_id: &str, user_id: i64, s_rank: bool) -> Result<()> {
    let updated = sqlx::query(
        "UPDATE camp_players SET is_s_rank = ?
         WHERE camp_id = ? AND user_id = ?
           AND camp_id IN (SELECT id FROM camps WHERE status IN ('signup', 'running'))",
    )
    .bind(s_rank)
    .bind(camp_id)
    .bind(user_id)
    .execute(pool)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(anyhow!("esse jogador não está no elenco de um camp ativo"));
    }
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

/// Where a match's players are, by its slot.
fn slot_location(slot: i64) -> &'static str {
    if slot == 1 {
        "match_1"
    } else {
        "match_2"
    }
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
    for (side, team, captains) in [
        ("hiddenking", &split.hiddenking, &split.hiddenking_captains),
        ("archmother", &split.archmother, &split.archmother_captains),
    ] {
        for user_id in team {
            sqlx::query(
                "INSERT INTO camp_match_players (match_id, user_id, side, is_captain) VALUES (?, ?, ?, ?)",
            )
            .bind(match_id)
            .bind(user_id)
            .bind(side)
            .bind(captains.contains(user_id))
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

    let location = slot_location(slot);
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
    let roster: Vec<(i64, bool)> = sqlx::query_as::<_, (i64, i64)>(
        "SELECT user_id, is_s_rank FROM camp_players WHERE camp_id = ? ORDER BY joined_at, user_id",
    )
    .bind(camp_id)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|(user_id, s_rank)| (user_id, s_rank != 0))
    .collect();
    let original: Vec<i64> = roster.iter().map(|p| p.0).collect();
    let team_size = state.camp.layout().match_size / 2;
    let mut rng = StdRng::from_entropy();
    let mut ids = queue::starting_queue(&roster, team_size, &mut rng);
    // Shuffle again if it happened to keep the signup order.
    for _ in 0..16 {
        if ids != original || ids.len() < 2 {
            break;
        }
        ids = queue::starting_queue(&roster, team_size, &mut rng);
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

    let location = slot_location(slot);
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
    Refreshed(RefreshChange),
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
        let change = resort_teams(&mut tx, &match_row.2, match_id, match_row.3 != 0).await?;
        tx.commit().await?;
        return Ok(RefreshOutcome::Refreshed(change));
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

/// Who a refresh brought in from the queue, and who it sent back there (to the front, in this order).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RefreshChange {
    pub joined: Vec<i64>,
    pub left: Vec<i64>,
}

/// The refresh itself: new teams (S rank apart, new captains), the match's refresh used up and
/// its votes cleared. A normal match also draws from the queue (`queue::refresh_roster`), and
/// whoever loses their seat goes back to the front of the queue. The final keeps its players.
async fn resort_teams(
    tx: &mut Transaction<'_, Sqlite>,
    camp_id: &str,
    match_id: &str,
    is_final: bool,
) -> Result<RefreshChange> {
    let slot: i64 = sqlx::query_scalar("SELECT slot FROM camp_matches WHERE id = ?")
        .bind(match_id)
        .fetch_one(&mut **tx)
        .await?;
    let roster: Vec<i64> = sqlx::query_scalar("SELECT user_id FROM camp_match_players WHERE match_id = ?")
        .bind(match_id)
        .fetch_all(&mut **tx)
        .await?;
    let players: Vec<(i64, i64, i64, i64, String)> = sqlx::query_as(
        "SELECT user_id, points, is_s_rank, consecutive_matches, location FROM camp_players
         WHERE camp_id = ? ORDER BY queue_position",
    )
    .bind(camp_id)
    .fetch_all(&mut **tx)
    .await?;
    let candidate = |user_id: i64| {
        let (points, s_rank, consecutive_matches) = players
            .iter()
            .find(|p| p.0 == user_id)
            .map(|p| (p.1, p.2 != 0, p.3))
            .unwrap_or((0, false, 0));
        Candidate {
            seat: Seat { user_id, points, s_rank },
            consecutive_matches,
        }
    };
    let playing: Vec<Candidate> = roster.iter().map(|id| candidate(*id)).collect();
    let queued: Vec<Candidate> = players
        .iter()
        .filter(|p| p.4 == "queued")
        .map(|p| candidate(p.0))
        .collect();
    let (seats, split) = {
        let mut rng = rand::thread_rng();
        let seats: Vec<Seat> = if is_final {
            playing.iter().map(|c| c.seat).collect()
        } else {
            queue::refresh_roster(&playing, &queued, &mut rng)
        };
        let split = queue::split_match(&seats, is_final, &mut rng);
        (seats, split)
    };
    let change = RefreshChange {
        joined: seats
            .iter()
            .map(|s| s.user_id)
            .filter(|id| !roster.contains(id))
            .collect(),
        left: roster
            .iter()
            .copied()
            .filter(|id| !seats.iter().any(|s| s.user_id == *id))
            .collect(),
    };

    sqlx::query("DELETE FROM camp_match_players WHERE match_id = ?")
        .bind(match_id)
        .execute(&mut **tx)
        .await?;
    insert_match_players(tx, match_id, &split).await?;
    for user_id in &change.joined {
        sqlx::query(
            "UPDATE camp_players SET location = ?, queue_position = NULL WHERE camp_id = ? AND user_id = ?",
        )
        .bind(slot_location(slot))
        .bind(camp_id)
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
    }
    if !change.left.is_empty() {
        let waiting = queued
            .iter()
            .map(|c| c.seat.user_id)
            .filter(|id| !change.joined.contains(id));
        for (i, user_id) in change.left.iter().copied().chain(waiting).enumerate() {
            sqlx::query(
                "UPDATE camp_players SET location = 'queued', queue_position = ? WHERE camp_id = ? AND user_id = ?",
            )
            .bind(i as i64)
            .bind(camp_id)
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
        }
    }
    sqlx::query("UPDATE camp_matches SET refresh_used = 1 WHERE id = ?")
        .bind(match_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM camp_refresh_votes WHERE match_id = ?")
        .bind(match_id)
        .execute(&mut **tx)
        .await?;
    Ok(change)
}

/// An admin ends a match's refresh vote early by re-sorting the teams, as if the refresh had
/// passed. Votes cast so far are dropped; the draft can start right after.
pub async fn refresh_by_admin(pool: &SqlitePool, camp_id: &str, match_id: &str) -> Result<RefreshChange> {
    let mut tx = pool.begin().await?;
    let is_final: Option<i64> = sqlx::query_scalar(
        "SELECT is_final FROM camp_matches
         WHERE id = ? AND camp_id = ? AND status = 'voting' AND refresh_used = 0",
    )
    .bind(match_id)
    .bind(camp_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(is_final) = is_final else {
        return Err(anyhow!("a votação desta partida não está aberta"));
    };
    let change = resort_teams(&mut tx, camp_id, match_id, is_final != 0).await?;
    sqlx::query("UPDATE camp_matches SET refreshed_by_admin = 1 WHERE id = ?")
        .bind(match_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(change)
}

/// An admin ends a match's refresh vote early and keeps the teams, e.g. when someone can't vote.
/// Votes already cast stay; the draft can start right after.
pub async fn close_voting(pool: &SqlitePool, camp_id: &str, match_id: &str) -> Result<()> {
    let updated = sqlx::query(
        "UPDATE camp_matches SET refresh_used = 1, vote_closed_by_admin = 1
         WHERE id = ? AND camp_id = ? AND status = 'voting' AND refresh_used = 0",
    )
    .bind(match_id)
    .bind(camp_id)
    .execute(pool)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(anyhow!("a votação desta partida já terminou"));
    }
    Ok(())
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

/// Ends a live match. `mvp_user_id`, one of its players, breaks ties in the final ranking.
pub async fn report_winner(
    pool: &SqlitePool,
    camp_id: &str,
    match_id: &str,
    winner_side: &str,
    mvp_user_id: Option<i64>,
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
    if let Some(mvp) = mvp_user_id {
        if !roster.iter().any(|(user_id, _)| *user_id == mvp) {
            return Err(anyhow!("o MVP precisa ser um jogador da partida"));
        }
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
        "UPDATE camp_matches SET status = 'completed', winner_side = ?, completed_at = ?, mvp_user_id = ?
         WHERE id = ?",
    )
    .bind(winner_side)
    .bind(ts)
    .bind(mvp_user_id)
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

/// Changes the MVP of the camp's latest result while the camp runs. Unlike the winner it moves no
/// points or queue places, so its players may already be in another match.
pub async fn change_mvp(
    pool: &SqlitePool,
    camp_id: &str,
    match_id: &str,
    mvp_user_id: i64,
) -> Result<CompletedMatchView> {
    let mut tx = pool.begin().await?;
    let camp_status: Option<String> = sqlx::query_scalar("SELECT status FROM camps WHERE id = ?")
        .bind(camp_id)
        .fetch_optional(&mut *tx)
        .await?;
    if camp_status.as_deref() != Some("running") {
        return Err(anyhow!("o MVP só pode ser trocado com o camp em andamento"));
    }
    let latest: Option<(String, String, i64, Option<String>)> = sqlx::query_as(
        "SELECT id, winner_side, is_final, party_code FROM camp_matches
         WHERE camp_id = ? AND status = 'completed' AND completed_at IS NOT NULL AND winner_side IS NOT NULL
         ORDER BY completed_at DESC, id DESC LIMIT 1",
    )
    .bind(camp_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((_, winner_side, is_final, party_code)) = latest.filter(|l| l.0 == match_id) else {
        return Err(anyhow!("só dá para trocar o MVP da última partida"));
    };
    let in_match: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM camp_match_players WHERE match_id = ? AND user_id = ?")
            .bind(match_id)
            .bind(mvp_user_id)
            .fetch_optional(&mut *tx)
            .await?;
    if in_match.is_none() {
        return Err(anyhow!("o MVP precisa ser um jogador da partida"));
    }
    sqlx::query("UPDATE camp_matches SET mvp_user_id = ? WHERE id = ?")
        .bind(mvp_user_id)
        .bind(match_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE camps SET updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(camp_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(CompletedMatchView {
        is_final: is_final != 0,
        winner_side,
        party_code,
    })
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
    let location = slot_location(current.slot);

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
        "UPDATE camp_matches SET status = 'live', winner_side = NULL, completed_at = NULL, mvp_user_id = NULL
         WHERE id = ?",
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
    if !state.open_matches.is_empty() {
        return Err(anyhow!("termine as partidas abertas antes de encerrar o camp"));
    }
    sqlx::query("UPDATE camps SET status = 'ended', updated_at = ? WHERE id = ?")
        .bind(now())
        .bind(camp_id)
        .execute(pool)
        .await?;
    Ok(state.players)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    async fn memory_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        migrate(&pool).await.unwrap();
        pool
    }

    /// A 2v2 test camp with its first match open for voting.
    async fn camp_with_match(pool: &SqlitePool) -> (String, String) {
        let camp = create_camp(pool, 1, 10, 4, 4).await.unwrap();
        add_players(pool, &camp.id, &[1, 2, 3, 4]).await.unwrap();
        start_queue(pool, &camp.id).await.unwrap();
        let (_, match_id) = create_match(pool, &camp.id, 1).await.unwrap();
        (camp.id, match_id)
    }

    fn sides(state: &CampState) -> HashMap<i64, String> {
        state.open_matches[0]
            .players
            .iter()
            .map(|p| (p.user_id, p.side.clone()))
            .collect()
    }

    #[tokio::test]
    async fn starting_the_queue_spreads_s_rank_over_the_teams() {
        let pool = memory_db().await;
        let camp = create_camp(&pool, 1, 10, 30, 12).await.unwrap();
        let roster: Vec<i64> = (1..=30).collect();
        add_players(&pool, &camp.id, &roster).await.unwrap();
        let s_rank: Vec<i64> = (1..=8).collect();
        set_s_rank(&pool, &camp.id, &s_rank).await.unwrap();

        let order = start_queue(&pool, &camp.id).await.unwrap();

        let per_team: Vec<usize> = order
            .chunks(6)
            .map(|team| team.iter().filter(|id| s_rank.contains(id)).count())
            .collect();
        assert_eq!(per_team.len(), 5);
        assert!(per_team.iter().all(|n| (1..=2).contains(n)), "{per_team:?}");
        let state = load_camp(&pool, &camp.id).await.unwrap().unwrap();
        let mut queued = state.players.clone();
        queued.sort_by_key(|p| p.queue_position);
        assert_eq!(queued.iter().map(|p| p.user_id).collect::<Vec<_>>(), order, "saved in that order");
    }

    #[tokio::test]
    async fn every_s_rank_player_in_a_match_is_captain() {
        let pool = memory_db().await;
        let camp = create_camp(&pool, 1, 10, 4, 4).await.unwrap();
        add_players(&pool, &camp.id, &[1, 2, 3, 4]).await.unwrap();
        set_s_rank(&pool, &camp.id, &[1, 2, 3]).await.unwrap();
        start_queue(&pool, &camp.id).await.unwrap();
        create_match(&pool, &camp.id, 1).await.unwrap();

        let state = load_camp(&pool, &camp.id).await.unwrap().unwrap();
        let players = &state.open_matches[0].players;
        let mut captains: Vec<i64> = players.iter().filter(|p| p.is_captain).map(|p| p.user_id).collect();
        captains.sort();
        assert_eq!(captains, [1, 2, 3], "three S rank in one 2v2: all captains, one team has two");
        let side_of = |id: i64| players.iter().find(|p| p.user_id == id).unwrap().side.clone();
        let regular_side = side_of(4);
        assert_eq!([1, 2, 3].iter().filter(|id| side_of(**id) == regular_side).count(), 1);
    }

    #[tokio::test]
    async fn admin_closes_an_open_vote_and_keeps_the_teams() {
        let pool = memory_db().await;
        let (camp_id, match_id) = camp_with_match(&pool).await;
        vote_refresh_choice(&pool, &match_id, 1, true).await.unwrap();
        let before = load_camp(&pool, &camp_id).await.unwrap().unwrap();

        close_voting(&pool, &camp_id, &match_id).await.unwrap();

        let after = load_camp(&pool, &camp_id).await.unwrap().unwrap();
        let open = &after.open_matches[0];
        assert!(open.row.refresh_used);
        assert!(open.row.vote_closed_by_admin);
        assert_eq!(open.votes.len(), 1, "votes already cast stay");
        assert_eq!(sides(&before), sides(&after), "teams are kept");

        // The vote is over for players too, it can't be closed twice, and the draft can start.
        assert!(matches!(
            vote_refresh_choice(&pool, &match_id, 2, true).await.unwrap(),
            RefreshOutcome::AlreadyUsed
        ));
        assert!(close_voting(&pool, &camp_id, &match_id).await.is_err());
        let draft = CreatedDraft {
            spectator_url: "https://draft.example/x".to_string(),
            join_url: "https://draft.example/x?key=k".to_string(),
        };
        set_draft(&pool, &match_id, &draft).await.unwrap();
    }

    #[tokio::test]
    async fn closing_needs_an_open_vote_in_that_camp() {
        let pool = memory_db().await;
        let (camp_id, match_id) = camp_with_match(&pool).await;
        let other = create_camp(&pool, 2, 20, 24, 12).await.unwrap();

        assert!(close_voting(&pool, &other.id, &match_id).await.is_err());

        close_voting(&pool, &camp_id, &match_id).await.unwrap();
        start_live_match(&pool, &match_id, None, None).await.unwrap();
        assert!(close_voting(&pool, &camp_id, &match_id).await.is_err());
        let state = load_camp(&pool, &camp_id).await.unwrap().unwrap();
        assert_eq!(state.open_matches[0].row.status, "live");
    }

    fn queue_order(state: &CampState) -> Vec<i64> {
        let mut queued: Vec<&PlayerRow> = state
            .players
            .iter()
            .filter(|p| p.location == "queued")
            .collect();
        queued.sort_by_key(|p| p.queue_position);
        queued.iter().map(|p| p.user_id).collect()
    }

    #[tokio::test]
    async fn reorder_takes_exactly_the_queued_players() {
        let pool = memory_db().await;
        // A 1v1 camp of 4: two play, two wait.
        let camp = create_camp(&pool, 1, 10, 4, 2).await.unwrap();
        add_players(&pool, &camp.id, &[1, 2, 3, 4]).await.unwrap();
        start_queue(&pool, &camp.id).await.unwrap();
        create_match(&pool, &camp.id, 1).await.unwrap();
        let state = load_camp(&pool, &camp.id).await.unwrap().unwrap();
        let before = queue_order(&state);
        assert_eq!(before.len(), 2);

        let reversed: Vec<i64> = before.iter().rev().copied().collect();
        reorder_queue(&pool, &camp.id, &reversed).await.unwrap();
        let state = load_camp(&pool, &camp.id).await.unwrap().unwrap();
        assert_eq!(queue_order(&state), reversed);

        assert!(reorder_queue(&pool, &camp.id, &[before[0]]).await.is_err(), "someone left out");
        assert!(reorder_queue(&pool, &camp.id, &[before[0], before[0]]).await.is_err(), "duplicate");
        let playing = state.players.iter().find(|p| p.location != "queued").unwrap().user_id;
        assert!(reorder_queue(&pool, &camp.id, &[before[0], playing]).await.is_err(), "not queued");
    }

    #[tokio::test]
    async fn match_list_and_stats_count_finished_matches() {
        let pool = memory_db().await;
        let (camp_id, first) = camp_with_match(&pool).await;
        close_voting(&pool, &camp_id, &first).await.unwrap();
        start_live_match(&pool, &first, None, None).await.unwrap();
        report_winner(&pool, &camp_id, &first, "hiddenking", None).await.unwrap();
        let (_, second) = create_match(&pool, &camp_id, 1).await.unwrap();

        let matches = list_matches(&pool, &camp_id).await.unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].row.id, second, "open matches come first");
        assert_eq!(matches[0].number, None);
        assert_eq!(matches[1].row.id, first);
        assert_eq!(matches[1].number, Some(1));
        assert_eq!(matches[1].players.len(), 4);

        let stats = player_stats(&pool, &camp_id).await.unwrap();
        let winners: Vec<i64> = matches[1]
            .players
            .iter()
            .filter(|p| p.side == "hiddenking")
            .map(|p| p.user_id)
            .collect();
        for p in &matches[1].players {
            let wins = i64::from(winners.contains(&p.user_id));
            assert_eq!(stats[&p.user_id], PlayerStats { played: 1, wins, ..PlayerStats::default() });
        }
    }

    #[tokio::test]
    async fn settings_fall_back_to_env_until_saved() {
        let pool = memory_db().await;
        let config = crate::config::Config {
            discord_token: String::new(),
            deadlock_api_key: None,
            database_url: String::new(),
            deadlock_server_region: Some("SouthAmerica".to_string()),
            enable_custom_match_create: true,
            deadchaps_api_key: None,
            queue_voice_channel_id: Some(5),
            match1_hiddenking_voice_id: None,
            match1_archmother_voice_id: None,
            match2_hiddenking_voice_id: None,
            match2_archmother_voice_id: None,
            web: crate::config::WebConfig {
                bind: String::new(),
                public_url: String::new(),
                discord_client_id: String::new(),
                discord_client_secret: String::new(),
                static_dir: String::new(),
            },
        };
        let env = GuildSettings::from_config(&config);
        let (defaults, saved) = guild_settings(&pool, 1, &env).await.unwrap();
        assert!(!saved);
        assert_eq!(defaults, env);

        let mine = GuildSettings {
            queue_voice_id: None,
            match1_hiddenking_voice_id: Some(11),
            create_lobby: false,
            server_region: None,
            use_draft_tool: false,
            draft_timer_seconds: 60,
            ..defaults.clone()
        };
        save_guild_settings(&pool, 1, &mine).await.unwrap();
        assert_eq!(guild_settings(&pool, 1, &env).await.unwrap(), (mine.clone(), true));
        assert_eq!(guild_settings(&pool, 2, &env).await.unwrap(), (defaults, false), "per server");

        let changed = GuildSettings { draft_timer_seconds: 0, ..mine };
        save_guild_settings(&pool, 1, &changed).await.unwrap();
        assert_eq!(guild_settings(&pool, 1, &env).await.unwrap().0, changed);
    }

    #[tokio::test]
    async fn admins_adjust_points_by_hand() {
        let pool = memory_db().await;
        let (camp_id, _) = camp_with_match(&pool).await;

        assert_eq!(adjust_points(&pool, &camp_id, 1, 3).await.unwrap(), 3);
        assert_eq!(adjust_points(&pool, &camp_id, 1, -1).await.unwrap(), 2);
        assert!(adjust_points(&pool, &camp_id, 1, -3).await.is_err(), "can't go below zero");
        assert!(adjust_points(&pool, &camp_id, 1, 0).await.is_err());
        assert!(adjust_points(&pool, &camp_id, 1, MAX_POINT_ADJUSTMENT + 1).await.is_err());
        assert!(adjust_points(&pool, &camp_id, 99, 1).await.is_err(), "not on the roster");

        let state = load_camp(&pool, &camp_id).await.unwrap().unwrap();
        assert_eq!(state.players.iter().find(|p| p.user_id == 1).unwrap().points, 2);
        assert_eq!(player_stats(&pool, &camp_id).await.unwrap()[&1].adjusted, 2);

        let signup = create_camp(&pool, 2, 20, 4, 4).await.unwrap();
        add_players(&pool, &signup.id, &[5]).await.unwrap();
        assert!(adjust_points(&pool, &signup.id, 5, 1).await.is_err(), "only while running");
    }

    #[tokio::test]
    async fn manual_draft_blocks_the_tool_draft_and_ending_waits_for_matches() {
        let pool = memory_db().await;
        let (camp_id, match_id) = camp_with_match(&pool).await;
        assert!(set_manual_draft(&pool, &match_id).await.is_err(), "the vote is still open");

        close_voting(&pool, &camp_id, &match_id).await.unwrap();
        set_manual_draft(&pool, &match_id).await.unwrap();
        let state = load_camp(&pool, &camp_id).await.unwrap().unwrap();
        let open = &state.open_matches[0].row;
        assert!(open.draft_manual && open.draft_started() && open.draft_url.is_none());
        assert!(set_manual_draft(&pool, &match_id).await.is_err());
        let draft = CreatedDraft {
            spectator_url: "https://draft.example/x".to_string(),
            join_url: "https://draft.example/x?key=k".to_string(),
        };
        assert!(set_draft(&pool, &match_id, &draft).await.is_err(), "already drafted by hand");

        start_live_match(&pool, &match_id, None, None).await.unwrap();
        assert!(end_camp(&pool, &camp_id).await.is_err(), "no scored match yet");
        report_winner(&pool, &camp_id, &match_id, "hiddenking", None).await.unwrap();
        create_match(&pool, &camp_id, 1).await.unwrap();
        assert!(end_camp(&pool, &camp_id).await.is_err(), "a match is open");
        let info = load_match_info(&pool, &match_id, 1).await.unwrap();
        assert!(info.row.draft_manual);
        assert_eq!(list_matches(&pool, &camp_id).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn admin_refreshes_an_open_vote() {
        let pool = memory_db().await;
        let (camp_id, match_id) = camp_with_match(&pool).await;
        set_s_rank(&pool, &camp_id, &[1, 2]).await.unwrap();
        vote_refresh_choice(&pool, &match_id, 3, false).await.unwrap();

        refresh_by_admin(&pool, &camp_id, &match_id).await.unwrap();

        let state = load_camp(&pool, &camp_id).await.unwrap().unwrap();
        let open = &state.open_matches[0];
        assert!(open.row.refresh_used && open.row.refreshed_by_admin && !open.row.vote_closed_by_admin);
        assert!(open.votes.is_empty(), "votes for the old teams are dropped");
        assert_eq!(open.players.len(), 4, "same players");
        assert_eq!(open.players.iter().filter(|p| p.side == "hiddenking").count(), 2);
        assert_eq!(open.players.iter().filter(|p| p.is_captain).count(), 2);

        assert!(refresh_by_admin(&pool, &camp_id, &match_id).await.is_err(), "only once");
        assert!(close_voting(&pool, &camp_id, &match_id).await.is_err());
        assert!(matches!(
            vote_refresh_choice(&pool, &match_id, 4, true).await.unwrap(),
            RefreshOutcome::AlreadyUsed
        ));
        let other = create_camp(&pool, 2, 20, 24, 12).await.unwrap();
        assert!(refresh_by_admin(&pool, &other.id, &match_id).await.is_err(), "another camp's match");
    }

    #[tokio::test]
    async fn refresh_draws_from_the_queue_but_waiters_keep_their_seat() {
        let mut outcomes = std::collections::HashSet::new();
        for _ in 0..30 {
            let pool = memory_db().await;
            // A 1v1 camp of 4: two play, two wait.
            let camp = create_camp(&pool, 1, 10, 4, 2).await.unwrap();
            add_players(&pool, &camp.id, &[1, 2, 3, 4]).await.unwrap();
            start_queue(&pool, &camp.id).await.unwrap();
            let (_, match_id) = create_match(&pool, &camp.id, 1).await.unwrap();
            let state = load_camp(&pool, &camp.id).await.unwrap().unwrap();
            let playing: Vec<i64> = state.open_matches[0].players.iter().map(|p| p.user_id).collect();
            let waiting = queue_order(&state);
            // In the match, one waited (0 in a row) and one played once. In the queue, one played
            // twice in a row and one once.
            for (user_id, consecutive) in [(playing[0], 0), (playing[1], 1), (waiting[0], 2), (waiting[1], 1)] {
                sqlx::query("UPDATE camp_players SET consecutive_matches = ? WHERE camp_id = ? AND user_id = ?")
                    .bind(consecutive)
                    .bind(&camp.id)
                    .bind(user_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }

            let change = refresh_by_admin(&pool, &camp.id, &match_id).await.unwrap();

            let state = load_camp(&pool, &camp.id).await.unwrap().unwrap();
            let seated: Vec<i64> = state.open_matches[0].players.iter().map(|p| p.user_id).collect();
            assert_eq!(seated.len(), 2);
            assert!(seated.contains(&playing[0]), "the waiter keeps the seat");
            assert!(!seated.contains(&waiting[0]), "nobody who played more in a row than the match");
            let location = |id: i64| state.players.iter().find(|p| p.user_id == id).unwrap().location.clone();
            if change.joined.is_empty() {
                assert_eq!(change, RefreshChange::default());
                assert_eq!(queue_order(&state), waiting, "the queue is untouched");
            } else {
                assert_eq!(change, RefreshChange { joined: vec![waiting[1]], left: vec![playing[1]] });
                assert_eq!(queue_order(&state), [playing[1], waiting[0]], "who lost the seat goes first");
                assert_eq!(location(waiting[1]), "match_1");
            }
            outcomes.insert(change.joined.is_empty());
        }
        assert_eq!(outcomes.len(), 2, "the open seat goes to the match player or the queued one");
    }

    #[tokio::test]
    async fn mvp_comes_with_the_result_and_only_the_latest_changes() {
        let pool = memory_db().await;
        let (camp_id, first) = camp_with_match(&pool).await;
        close_voting(&pool, &camp_id, &first).await.unwrap();
        start_live_match(&pool, &first, None, None).await.unwrap();
        assert!(
            report_winner(&pool, &camp_id, &first, "hiddenking", Some(99)).await.is_err(),
            "the MVP plays in the match"
        );
        report_winner(&pool, &camp_id, &first, "hiddenking", Some(1)).await.unwrap();
        assert_eq!(player_stats(&pool, &camp_id).await.unwrap()[&1].mvps, 1);

        let view = change_mvp(&pool, &camp_id, &first, 2).await.unwrap();
        assert_eq!(view.winner_side, "hiddenking");
        let stats = player_stats(&pool, &camp_id).await.unwrap();
        assert_eq!((stats[&1].mvps, stats[&2].mvps), (0, 1));
        assert!(change_mvp(&pool, &camp_id, &first, 99).await.is_err());
        assert_eq!(recent_matches(&pool, &camp_id, 1).await.unwrap()[0].mvp_user_id, Some(2));

        // A newer result locks the older MVP; reopening clears the MVP. Results are timed to the
        // second, so the first one is moved back to keep "latest" unambiguous.
        sqlx::query("UPDATE camp_matches SET completed_at = completed_at - 60 WHERE id = ?")
            .bind(&first)
            .execute(&pool)
            .await
            .unwrap();
        let (_, second) = create_match(&pool, &camp_id, 1).await.unwrap();
        close_voting(&pool, &camp_id, &second).await.unwrap();
        start_live_match(&pool, &second, None, None).await.unwrap();
        report_winner(&pool, &camp_id, &second, "archmother", Some(3)).await.unwrap();
        assert!(change_mvp(&pool, &camp_id, &first, 1).await.is_err(), "not the latest");
        change_mvp(&pool, &camp_id, &second, 4).await.unwrap();
        reopen_match(&pool, &camp_id, &second).await.unwrap();
        let info = load_match_info(&pool, &second, 1).await.unwrap();
        assert_eq!(info.row.mvp_user_id, None);
        assert_eq!(player_stats(&pool, &camp_id).await.unwrap()[&4].mvps, 0);
    }
}
