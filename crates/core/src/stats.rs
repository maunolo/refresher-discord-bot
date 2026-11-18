//! Each finished match's scoreboard from the Deadlock API, for the public site's match history.
//!
//! When a match ends it's queued here. A background worker turns the bot's lobby (its party) into
//! the Deadlock match ID, then fetches the match's data once the API has it, which takes a few
//! minutes after the game. Matches the bot didn't make the lobby for, or whose party never
//! resolves, wait for an admin to give the match ID.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::db::now;
use crate::deadlock::{self, Catalog, Metadata};
use crate::link::{BoxFuture, Changes};

/// The bot is looking the match up.
pub const PENDING: &str = "pending";
/// An admin has to give the Deadlock match ID.
pub const NEEDS_ID: &str = "needs_id";
pub const READY: &str = "ready";
/// The API never had the match; an admin can try again.
pub const FAILED: &str = "failed";

/// The bot side each Deadlock team plays as: team 0 is the Hidden King's.
pub const TEAM_SIDES: [&str; 2] = ["hiddenking", "archmother"];

/// A finished match as Deadlock recorded it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scoreboard {
    pub deadlock_match_id: u64,
    /// Unix seconds.
    pub start_time: i64,
    pub duration_s: i64,
    /// `0` for the Hidden King, `1` for the Archmother.
    pub winning_team: usize,
    pub players: Vec<ScoreboardPlayer>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreboardPlayer {
    /// The Steam account (SteamID3).
    pub account_id: u64,
    pub team: usize,
    pub steam_name: Option<String>,
    pub hero_id: u64,
    pub hero: String,
    pub hero_image: Option<String>,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    /// Net worth at the end.
    pub souls: i64,
    pub damage: i64,
    pub objective_damage: i64,
    pub healing: i64,
    /// Items held at the end, in the order they were bought.
    pub items: Vec<ScoreboardItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreboardItem {
    pub id: u64,
    pub name: String,
    /// `weapon`, `spirit` or `vitality`.
    pub slot: String,
    pub tier: Option<i64>,
    pub active: bool,
    pub image: Option<String>,
}

/// Lowercase words joined by `-`, as the site names its icons: `Lady Geist` is `lady-geist`.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// IDs in the metadata the catalog doesn't name yet: a new patch.
fn unknown_ids(meta: &Metadata, catalog: &Catalog) -> bool {
    meta.match_info.players.iter().any(|p| !catalog.heroes.contains_key(&p.hero_id))
}

pub fn build_scoreboard(meta: &Metadata, catalog: &Catalog, names: &HashMap<u64, String>) -> Result<Scoreboard> {
    let info = &meta.match_info;
    let winning_team =
        deadlock::team_index(&info.winning_team).ok_or_else(|| anyhow!("a partida não tem vencedor nos dados"))?;
    let mut players = Vec::with_capacity(info.players.len());
    for p in &info.players {
        let team = deadlock::team_index(&p.team).ok_or_else(|| anyhow!("jogador sem time nos dados"))?;
        let hero = catalog.heroes.get(&p.hero_id);
        let last = p.stats.last().cloned().unwrap_or_default();
        let mut held: Vec<&deadlock::MetaItem> = p
            .items
            .iter()
            .filter(|i| i.sold_time_s == 0 && catalog.items.contains_key(&i.item_id))
            .collect();
        held.sort_by_key(|i| i.game_time_s);
        let mut seen = HashSet::new();
        let items = held
            .into_iter()
            .filter(|i| seen.insert(i.item_id))
            .map(|i| {
                let item = &catalog.items[&i.item_id];
                ScoreboardItem {
                    id: i.item_id,
                    name: item.name.clone(),
                    slot: item.slot.clone(),
                    tier: item.tier,
                    active: item.active,
                    image: item.image.clone(),
                }
            })
            .collect();
        players.push(ScoreboardPlayer {
            account_id: p.account_id,
            team,
            steam_name: names.get(&p.account_id).cloned(),
            hero_id: p.hero_id,
            hero: hero.map_or_else(|| format!("Herói {}", p.hero_id), |h| h.name.clone()),
            hero_image: hero.and_then(|h| h.image.clone()),
            kills: p.kills,
            deaths: p.deaths,
            assists: p.assists,
            souls: p.net_worth,
            damage: last.player_damage,
            objective_damage: last.boss_damage,
            healing: last.player_healing,
            items,
        });
    }
    Ok(Scoreboard {
        deadlock_match_id: info.match_id,
        start_time: info.start_time,
        duration_s: info.duration_s,
        winning_team,
        players,
    })
}

/// A match's lookup, as the admin page shows it.
#[derive(Clone, Debug)]
pub struct StatsRow {
    pub deadlock_match_id: Option<u64>,
    pub status: String,
    pub attempts: i64,
    pub next_attempt_at: Option<i64>,
    pub last_error: Option<String>,
    pub scoreboard: Option<Scoreboard>,
}

/// Queues a match that just ended: looked up by its party if the bot made its lobby, otherwise
/// waiting for an admin. A match that already has a lookup (ended again after a reopen) keeps it.
pub async fn queue_in_tx(tx: &mut Transaction<'_, Sqlite>, match_id: &str) -> Result<()> {
    let party: Option<String> = sqlx::query_scalar("SELECT party_id FROM camp_matches WHERE id = ?")
        .bind(match_id)
        .fetch_optional(&mut **tx)
        .await?
        .flatten();
    let status = if party.is_some() { PENDING } else { NEEDS_ID };
    let ts = now();
    sqlx::query(
        "INSERT INTO match_stats (match_id, status, next_attempt_at, updated_at) VALUES (?, ?, ?, ?)
         ON CONFLICT (match_id) DO NOTHING",
    )
    .bind(match_id)
    .bind(status)
    .bind(ts)
    .bind(ts)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

type RawRow = (Option<i64>, String, i64, Option<i64>, Option<String>, Option<String>);

fn parse_row((id, status, attempts, next_attempt_at, last_error, data): RawRow) -> Result<StatsRow> {
    Ok(StatsRow {
        deadlock_match_id: id.map(|id| id as u64),
        status,
        attempts,
        next_attempt_at,
        last_error,
        scoreboard: data.map(|d| serde_json::from_str(&d)).transpose()?,
    })
}

pub async fn load(pool: &SqlitePool, match_id: &str) -> Result<Option<StatsRow>> {
    let row: Option<RawRow> = sqlx::query_as(
        "SELECT deadlock_match_id, status, attempts, next_attempt_at, last_error, data
         FROM match_stats WHERE match_id = ?",
    )
    .bind(match_id)
    .fetch_optional(pool)
    .await?;
    row.map(parse_row).transpose()
}

/// The scoreboards of a camp's matches that have one, by match.
pub async fn camp_scoreboards(pool: &SqlitePool, camp_id: &str) -> Result<HashMap<String, Scoreboard>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT s.match_id, s.data FROM match_stats s JOIN camp_matches m ON m.id = s.match_id
         WHERE m.camp_id = ? AND s.status = 'ready' AND s.data IS NOT NULL",
    )
    .bind(camp_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|(id, data)| Ok((id, serde_json::from_str(&data)?)))
        .collect()
}

/// An admin gave the match's Deadlock ID (or asked to try again): looked up right away.
pub async fn set_deadlock_id(pool: &SqlitePool, match_id: &str, deadlock_match_id: u64) -> Result<()> {
    let ts = now();
    sqlx::query(
        "INSERT INTO match_stats (match_id, deadlock_match_id, status, attempts, next_attempt_at, updated_at)
         VALUES (?, ?, 'pending', 0, ?, ?)
         ON CONFLICT (match_id) DO UPDATE SET deadlock_match_id = excluded.deadlock_match_id,
             status = 'pending', attempts = 0, next_attempt_at = excluded.next_attempt_at,
             last_steam_at = NULL, last_error = NULL, updated_at = excluded.updated_at",
    )
    .bind(match_id)
    .bind(deadlock_match_id as i64)
    .bind(ts)
    .bind(ts)
    .execute(pool)
    .await?;
    Ok(())
}

/// Which server member plays on each Steam account.
pub async fn account_links(pool: &SqlitePool, guild_id: i64) -> Result<HashMap<u64, i64>> {
    let rows: Vec<(i64, i64)> =
        sqlx::query_as("SELECT account_id, user_id FROM deadlock_accounts WHERE guild_id = ?")
            .bind(guild_id)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(|(a, u)| (a as u64, u)).collect())
}

/// Links a Steam account to a member, or unlinks it with `None`.
pub async fn set_account_link(pool: &SqlitePool, guild_id: i64, account_id: u64, user_id: Option<i64>) -> Result<()> {
    match user_id {
        Some(user_id) => {
            sqlx::query(
                "INSERT INTO deadlock_accounts (guild_id, account_id, user_id, updated_at) VALUES (?, ?, ?, ?)
                 ON CONFLICT (guild_id, account_id) DO UPDATE SET user_id = excluded.user_id,
                     updated_at = excluded.updated_at",
            )
            .bind(guild_id)
            .bind(account_id as i64)
            .bind(user_id)
            .bind(now())
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM deadlock_accounts WHERE guild_id = ? AND account_id = ?")
                .bind(guild_id)
                .bind(account_id as i64)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

/// Links the one account left on a team to the one player left on that side, when the rest of the
/// team is already linked: in a 1v1, or once all but one player of a team are known.
async fn auto_link(pool: &SqlitePool, guild_id: i64, match_id: &str, board: &Scoreboard) -> Result<()> {
    let roster: Vec<(i64, String)> =
        sqlx::query_as("SELECT user_id, side FROM camp_match_players WHERE match_id = ?")
            .bind(match_id)
            .fetch_all(pool)
            .await?;
    let links = account_links(pool, guild_id).await?;
    for (team, side) in TEAM_SIDES.iter().enumerate() {
        let linked_users: HashSet<i64> = board
            .players
            .iter()
            .filter_map(|p| links.get(&p.account_id).copied())
            .collect();
        let accounts: Vec<u64> = board
            .players
            .iter()
            .filter(|p| p.team == team && !links.contains_key(&p.account_id))
            .map(|p| p.account_id)
            .collect();
        let users: Vec<i64> = roster
            .iter()
            .filter(|(user, s)| s == side && !linked_users.contains(user))
            .map(|(user, _)| *user)
            .collect();
        if let ([account], [user]) = (accounts.as_slice(), users.as_slice()) {
            set_account_link(pool, guild_id, *account, Some(*user)).await?;
        }
    }
    Ok(())
}

/// What the worker asks the Deadlock API; a trait so tests can stand in for it.
pub trait Api: Send + Sync {
    fn custom_match_id<'a>(&'a self, party_id: &'a str) -> BoxFuture<'a, Result<Option<u64>>>;
    fn metadata(&self, match_id: u64, from_steam: bool) -> BoxFuture<'_, Result<Option<Metadata>>>;
    fn steam_names<'a>(&'a self, account_ids: &'a [u64]) -> BoxFuture<'a, Result<HashMap<u64, String>>>;
    /// Heroes and items; `refresh` fetches them again (a new patch).
    fn catalog(&self, refresh: bool) -> BoxFuture<'_, Result<Arc<Catalog>>>;
    /// With an API key, Steam lookups are 300 an hour instead of 3.
    fn has_key(&self) -> bool;
}

/// The Deadlock API over HTTP. The catalog is kept for a day.
pub struct HttpApi {
    http: reqwest::Client,
    api_key: Option<String>,
    catalog: tokio::sync::Mutex<Option<(Instant, Arc<Catalog>)>>,
}

impl HttpApi {
    pub fn new(api_key: Option<String>) -> Result<Self> {
        Ok(HttpApi {
            // The item list is a few megabytes.
            http: reqwest::Client::builder().timeout(Duration::from_secs(90)).build()?,
            api_key: api_key.filter(|k| !k.is_empty()),
            catalog: tokio::sync::Mutex::new(None),
        })
    }
}

impl Api for HttpApi {
    fn custom_match_id<'a>(&'a self, party_id: &'a str) -> BoxFuture<'a, Result<Option<u64>>> {
        Box::pin(deadlock::custom_match_id(&self.http, self.api_key.as_deref(), party_id))
    }

    fn metadata(&self, match_id: u64, from_steam: bool) -> BoxFuture<'_, Result<Option<Metadata>>> {
        Box::pin(deadlock::match_metadata(&self.http, self.api_key.as_deref(), match_id, from_steam))
    }

    fn steam_names<'a>(&'a self, account_ids: &'a [u64]) -> BoxFuture<'a, Result<HashMap<u64, String>>> {
        Box::pin(deadlock::steam_names(&self.http, self.api_key.as_deref(), account_ids))
    }

    fn catalog(&self, refresh: bool) -> BoxFuture<'_, Result<Arc<Catalog>>> {
        Box::pin(async move {
            let mut cached = self.catalog.lock().await;
            if let Some((at, catalog)) = cached.as_ref() {
                if !refresh && at.elapsed() < Duration::from_secs(24 * 3600) {
                    return Ok(catalog.clone());
                }
            }
            let fresh = Arc::new(deadlock::catalog(&self.http, self.api_key.as_deref()).await?);
            *cached = Some((Instant::now(), fresh.clone()));
            Ok(fresh)
        })
    }

    fn has_key(&self) -> bool {
        self.api_key.is_some()
    }
}

/// Tries to turn a party into its match ID, every 2 minutes, before asking an admin.
const PARTY_TRIES: i64 = 6;
const PARTY_RETRY_S: i64 = 120;
/// Seconds before each try at the match's data: about two hours in all. The API has it a few
/// minutes after the game, or later if it has to ask Steam.
const DATA_DELAYS_S: [i64; 8] = [60, 120, 180, 300, 600, 900, 1800, 3600];

/// Valve publishes a match's data a few minutes after it ends; asking Steam sooner wastes a lookup.
const STEAM_AFTER_END_S: i64 = 300;
/// Without an API key, one match asks Steam at most this often.
const STEAM_SPACING_S: i64 = 900;
/// Steam lookups per rolling hour, across all matches. The API allows 3 per IP without a key (this
/// leaves one spare, as other apps may share the IP) and 300 per key.
const STEAM_PER_HOUR: usize = 2;
const STEAM_PER_HOUR_WITH_KEY: usize = 250;

/// The Steam lookups the worker made in the last hour, across all matches.
pub struct SteamBudget {
    per_hour: usize,
    /// When each lookup was made, oldest first.
    used: VecDeque<i64>,
}

impl SteamBudget {
    pub fn new(has_key: bool) -> Self {
        let per_hour = if has_key { STEAM_PER_HOUR_WITH_KEY } else { STEAM_PER_HOUR };
        SteamBudget { per_hour, used: VecDeque::new() }
    }

    fn forget_old(&mut self, at: i64) {
        while self.used.front().is_some_and(|t| *t <= at - 3600) {
            self.used.pop_front();
        }
    }

    /// Spends a lookup at `at`, if one is left this hour.
    fn take(&mut self, at: i64) -> bool {
        self.forget_old(at);
        if self.used.len() >= self.per_hour {
            return false;
        }
        self.used.push_back(at);
        true
    }

    /// When the next lookup frees up.
    fn next_free(&mut self, at: i64) -> i64 {
        self.forget_old(at);
        match self.used.front() {
            Some(oldest) if self.used.len() >= self.per_hour => oldest + 3600,
            _ => at,
        }
    }
}

/// Whether this try should let the API ask Steam: from 5 minutes after the match ended and, without
/// a key, at most every 15 minutes per match.
fn wants_steam(has_key: bool, job: &Job, at: i64) -> bool {
    let ended = job.completed_at.is_none_or(|t| at - t >= STEAM_AFTER_END_S);
    let spaced = has_key || job.last_steam_at.is_none_or(|t| at - t >= STEAM_SPACING_S);
    ended && spaced
}

struct Job {
    match_id: String,
    guild_id: i64,
    party_id: Option<String>,
    deadlock_match_id: Option<u64>,
    attempts: i64,
    completed_at: Option<i64>,
    last_steam_at: Option<i64>,
}

/// Match, server, party, Deadlock match ID, tries so far, when it ended, last Steam try.
type JobRow = (String, i64, Option<String>, Option<i64>, i64, Option<i64>, Option<i64>);

async fn due_jobs(pool: &SqlitePool, at: i64) -> Result<Vec<Job>> {
    let rows: Vec<JobRow> = sqlx::query_as(
        "SELECT s.match_id, c.guild_id, m.party_id, s.deadlock_match_id, s.attempts, m.completed_at,
             s.last_steam_at
         FROM match_stats s
         JOIN camp_matches m ON m.id = s.match_id
         JOIN camps c ON c.id = m.camp_id
         WHERE s.status = 'pending' AND (s.next_attempt_at IS NULL OR s.next_attempt_at <= ?)
         ORDER BY s.next_attempt_at",
    )
    .bind(at)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(match_id, guild_id, party_id, id, attempts, completed_at, last_steam_at)| Job {
            match_id,
            guild_id,
            party_id,
            deadlock_match_id: id.map(|id| id as u64),
            attempts,
            completed_at,
            last_steam_at,
        })
        .collect())
}

async fn update(
    pool: &SqlitePool,
    match_id: &str,
    status: &str,
    deadlock_match_id: Option<u64>,
    attempts: i64,
    next_attempt_at: Option<i64>,
    error: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "UPDATE match_stats SET status = ?, deadlock_match_id = COALESCE(?, deadlock_match_id),
             attempts = ?, next_attempt_at = ?, last_error = ?, updated_at = ?
         WHERE match_id = ?",
    )
    .bind(status)
    .bind(deadlock_match_id.map(|id| id as i64))
    .bind(attempts)
    .bind(next_attempt_at)
    .bind(error)
    .bind(now())
    .bind(match_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn save_ready(pool: &SqlitePool, match_id: &str, board: &Scoreboard) -> Result<()> {
    sqlx::query(
        "UPDATE match_stats SET status = 'ready', deadlock_match_id = ?, data = ?, next_attempt_at = NULL,
             last_error = NULL, updated_at = ?
         WHERE match_id = ?",
    )
    .bind(board.deadlock_match_id as i64)
    .bind(serde_json::to_string(board)?)
    .bind(now())
    .bind(match_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn fetch_scoreboard(api: &dyn Api, meta: &Metadata) -> Result<Scoreboard> {
    let mut catalog = api.catalog(false).await?;
    if unknown_ids(meta, &catalog) {
        catalog = api.catalog(true).await?;
    }
    let accounts: Vec<u64> = meta.match_info.players.iter().map(|p| p.account_id).collect();
    // Names are a nicety: a scoreboard without them is still worth saving.
    let names = match api.steam_names(&accounts).await {
        Ok(names) => names,
        Err(err) => {
            tracing::warn!(error = %err, "Steam names unavailable");
            HashMap::new()
        }
    };
    build_scoreboard(meta, &catalog, &names)
}

/// One try at a match at `at`; says whether anything an admin sees changed.
async fn process(pool: &SqlitePool, api: &dyn Api, budget: &mut SteamBudget, job: &Job, at: i64) -> Result<bool> {
    let Some(id) = job.deadlock_match_id else {
        let Some(party) = job.party_id.as_deref() else {
            update(pool, &job.match_id, NEEDS_ID, None, 0, None, None).await?;
            return Ok(true);
        };
        let found = api.custom_match_id(party).await;
        return match found {
            Ok(Some(id)) => {
                update(pool, &job.match_id, PENDING, Some(id), 0, Some(at + DATA_DELAYS_S[0]), None).await?;
                Ok(true)
            }
            Ok(None) | Err(_) => {
                let error = found.err().map(|e| e.to_string());
                let attempts = job.attempts + 1;
                if attempts >= PARTY_TRIES {
                    let message = "A API do Deadlock não achou a partida pela party do lobby. Informe o ID da partida.";
                    update(pool, &job.match_id, NEEDS_ID, None, attempts, None, Some(message)).await?;
                    Ok(true)
                } else {
                    let next = Some(at + PARTY_RETRY_S);
                    update(pool, &job.match_id, PENDING, None, attempts, next, error.as_deref()).await?;
                    Ok(false)
                }
            }
        };
    };

    let wants = wants_steam(api.has_key(), job, at);
    let from_steam = wants && budget.take(at);
    if from_steam {
        sqlx::query("UPDATE match_stats SET last_steam_at = ? WHERE match_id = ?")
            .bind(at)
            .bind(&job.match_id)
            .execute(pool)
            .await?;
    }
    let fetched = match api.metadata(id, from_steam).await {
        Ok(Some(meta)) => fetch_scoreboard(api, &meta).await.map(Some),
        Ok(None) => Ok(None),
        Err(err) => Err(err),
    };
    match fetched {
        Ok(Some(board)) => {
            save_ready(pool, &job.match_id, &board).await?;
            auto_link(pool, job.guild_id, &job.match_id, &board).await?;
            Ok(true)
        }
        // This try was due to ask Steam but the hour's lookups are spent: it doesn't count, and
        // comes back when one frees up.
        Ok(None) if wants && !from_steam => {
            let next = budget.next_free(at).max(at + 60);
            let message = "Esperando a vez de pedir a partida à Steam (limite da API do Deadlock).";
            update(pool, &job.match_id, PENDING, None, job.attempts, Some(next), Some(message)).await?;
            Ok(false)
        }
        Ok(None) | Err(_) => {
            let error = fetched.err().map(|e| e.to_string());
            let attempts = job.attempts + 1;
            match DATA_DELAYS_S.get(attempts as usize) {
                Some(delay) => {
                    let message = error.as_deref().unwrap_or("A API do Deadlock ainda não tem a partida.");
                    update(pool, &job.match_id, PENDING, None, attempts, Some(at + delay), Some(message)).await?;
                    Ok(false)
                }
                None => {
                    let message = error.unwrap_or_else(|| {
                        "A API do Deadlock não teve os dados da partida em duas horas. Confira o ID e busque de novo."
                            .to_string()
                    });
                    update(pool, &job.match_id, FAILED, None, attempts, None, Some(&message)).await?;
                    Ok(true)
                }
            }
        }
    }
}

/// Tries every lookup due at `at`; returns the servers whose admin pages should reload.
pub async fn step(pool: &SqlitePool, api: &dyn Api, budget: &mut SteamBudget, at: i64) -> Result<HashSet<i64>> {
    let mut changed = HashSet::new();
    for job in due_jobs(pool, at).await? {
        match process(pool, api, budget, &job, at).await {
            Ok(true) => {
                changed.insert(job.guild_id);
            }
            Ok(false) => {}
            Err(err) => tracing::error!(match_id = job.match_id, error = %err, "match stats lookup failed"),
        }
    }
    Ok(changed)
}

/// Looks matches up until the bot stops, every 10 seconds.
pub async fn run(pool: SqlitePool, api: Arc<dyn Api>, changes: Changes) -> Result<()> {
    let mut budget = SteamBudget::new(api.has_key());
    let mut tick = tokio::time::interval(Duration::from_secs(10));
    loop {
        tick.tick().await;
        match step(&pool, api.as_ref(), &mut budget, now()).await {
            Ok(changed) => changed.into_iter().for_each(|guild| changes.notify(guild as u64)),
            Err(err) => tracing::error!(error = %err, "match stats worker failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;
    use crate::db;
    use crate::deadlock::{CatalogHero, CatalogItem, MatchInfo, MetaItem, MetaPlayer, MetaStats};

    fn player(account_id: u64, team: u64, hero_id: u64) -> MetaPlayer {
        MetaPlayer {
            account_id,
            team: serde_json::json!(team),
            hero_id,
            kills: 22,
            deaths: 3,
            assists: 8,
            net_worth: 60_123,
            items: vec![
                MetaItem { item_id: 7, game_time_s: 300, sold_time_s: 0 },
                MetaItem { item_id: 5, game_time_s: 100, sold_time_s: 0 },
                MetaItem { item_id: 6, game_time_s: 200, sold_time_s: 900 },
                // An ability upgrade: not in the shop.
                MetaItem { item_id: 99, game_time_s: 50, sold_time_s: 0 },
            ],
            stats: vec![
                MetaStats { player_damage: 10, boss_damage: 1, player_healing: 2 },
                MetaStats { player_damage: 55_000, boss_damage: 13_000, player_healing: 16_559 },
            ],
        }
    }

    fn metadata() -> Metadata {
        Metadata {
            match_info: MatchInfo {
                match_id: 139086680,
                start_time: 1794919042,
                duration_s: 2026,
                winning_team: serde_json::json!(0),
                players: vec![player(100, 0, 1), player(200, 1, 2)],
            },
        }
    }

    fn catalog() -> Catalog {
        let item = |name: &str, slot: &str| CatalogItem {
            name: name.to_string(),
            slot: slot.to_string(),
            tier: Some(2),
            active: false,
            image: Some(format!("https://img/{name}.webp")),
        };
        Catalog {
            heroes: HashMap::from([
                (1, CatalogHero { name: "Wraith".to_string(), image: Some("https://img/wraith.webp".to_string()) }),
                (2, CatalogHero { name: "Lady Geist".to_string(), image: None }),
            ]),
            items: HashMap::from([
                (5, item("Swift Striker", "weapon")),
                (6, item("Extra Health", "vitality")),
                (7, item("Diviner's Kevlar", "spirit")),
            ]),
        }
    }

    #[test]
    fn slugs_match_the_site_icons() {
        assert_eq!(slug("Lady Geist"), "lady-geist");
        assert_eq!(slug("Mo & Krill"), "mo-krill");
        assert_eq!(slug("Diviner's Kevlar"), "diviner-s-kevlar");
        assert_eq!(slug("McGinnis"), "mcginnis");
    }

    #[test]
    fn scoreboards_keep_the_items_held_at_the_end_in_buying_order() {
        let names = HashMap::from([(100, "VioletaS2".to_string())]);
        let board = build_scoreboard(&metadata(), &catalog(), &names).unwrap();
        assert_eq!(board.winning_team, 0);
        assert_eq!(board.duration_s, 2026);
        let wraith = &board.players[0];
        assert_eq!((wraith.team, wraith.hero.as_str()), (0, "Wraith"));
        assert_eq!(wraith.steam_name.as_deref(), Some("VioletaS2"));
        assert_eq!((wraith.kills, wraith.deaths, wraith.assists, wraith.souls), (22, 3, 8, 60_123));
        assert_eq!((wraith.damage, wraith.objective_damage, wraith.healing), (55_000, 13_000, 16_559));
        let items: Vec<&str> = wraith.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(items, ["Swift Striker", "Diviner's Kevlar"]);
        assert_eq!(board.players[1].team, 1);
        assert_eq!(board.players[1].steam_name, None);
    }

    #[test]
    fn teams_written_as_names_still_map() {
        let mut meta = metadata();
        meta.match_info.winning_team = serde_json::json!("k_ECitadelLobbyTeam_Team1");
        meta.match_info.players[0].team = serde_json::json!("k_ECitadelLobbyTeam_Team1");
        let board = build_scoreboard(&meta, &catalog(), &HashMap::new()).unwrap();
        assert_eq!(board.winning_team, 1);
        assert_eq!(board.players[0].team, 1);
    }

    /// Against the real Deadlock API: `cargo test -p refresher-core real_match -- --ignored`.
    #[tokio::test]
    #[ignore = "calls the Deadlock API"]
    async fn real_match_from_the_api() {
        let api = HttpApi::new(None).unwrap();
        let meta = api.metadata(139086680, false).await.unwrap().expect("match in the API's store");
        let board = fetch_scoreboard(&api, &meta).await.unwrap();
        assert_eq!((board.winning_team, board.duration_s, board.players.len()), (0, 2026, 12));
        let wraith = board.players.iter().find(|p| p.hero == "Wraith").unwrap();
        assert_eq!((wraith.team, wraith.kills, wraith.deaths, wraith.assists), (0, 22, 3, 8));
        assert_eq!(wraith.items.len(), 12);
        assert!(board.players.iter().all(|p| p.hero_image.is_some()));
        assert!(board.players.iter().filter(|p| p.steam_name.is_some()).count() >= 6);
    }

    #[derive(Default)]
    struct FakeApi {
        match_id: Option<u64>,
        metadata: Option<Metadata>,
        /// `from_steam` of each metadata call.
        calls: Mutex<Vec<bool>>,
        key: bool,
    }

    impl Api for FakeApi {
        fn custom_match_id<'a>(&'a self, _party_id: &'a str) -> BoxFuture<'a, Result<Option<u64>>> {
            Box::pin(async move { Ok(self.match_id) })
        }
        fn metadata(&self, _match_id: u64, from_steam: bool) -> BoxFuture<'_, Result<Option<Metadata>>> {
            self.calls.lock().unwrap().push(from_steam);
            Box::pin(async move { Ok(self.metadata.clone()) })
        }
        fn steam_names<'a>(&'a self, _ids: &'a [u64]) -> BoxFuture<'a, Result<HashMap<u64, String>>> {
            Box::pin(async { Ok(HashMap::new()) })
        }
        fn catalog(&self, _refresh: bool) -> BoxFuture<'_, Result<Arc<Catalog>>> {
            Box::pin(async { Ok(Arc::new(catalog())) })
        }
        fn has_key(&self) -> bool {
            self.key
        }
    }

    async fn memory_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        db::migrate(&pool).await.unwrap();
        pool
    }

    /// A finished 1v1 in server 1; `party` if the bot made its lobby. Returns the match and its
    /// (Hidden King, Archmother) players.
    async fn finished_match(pool: &SqlitePool, party: Option<&str>) -> (String, i64, i64) {
        let camp = db::create_camp(pool, 1, 10, 4, 2).await.unwrap();
        db::add_players(pool, &camp.id, &[1, 2, 3, 4]).await.unwrap();
        db::start_queue(pool, &camp.id).await.unwrap();
        let (_, match_id) = db::create_match(pool, &camp.id, 1).await.unwrap();
        db::close_voting(pool, &camp.id, &match_id).await.unwrap();
        db::start_live_match(pool, &match_id, party.map(str::to_string), party.map(|_| "CODE".to_string()))
            .await
            .unwrap();
        let info = db::load_match_info(pool, &match_id, 1).await.unwrap();
        let side = |side: &str| info.players.iter().find(|p| p.side == side).unwrap().user_id;
        let (king, arch) = (side("hiddenking"), side("archmother"));
        db::report_winner(pool, &camp.id, &match_id, "hiddenking", None).await.unwrap();
        (match_id, king, arch)
    }

    /// When the match ended.
    async fn ended_at(pool: &SqlitePool, match_id: &str) -> i64 {
        sqlx::query_scalar("SELECT completed_at FROM camp_matches WHERE id = ?")
            .bind(match_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// Runs the worker at each try's scheduled time until the lookup settles; returns those times.
    async fn run_until_settled(pool: &SqlitePool, api: &FakeApi, budget: &mut SteamBudget, match_id: &str) -> Vec<i64> {
        let mut times = Vec::new();
        for _ in 0..30 {
            let row = load(pool, match_id).await.unwrap().unwrap();
            if row.status != PENDING {
                break;
            }
            let at = row.next_attempt_at.unwrap();
            step(pool, api, budget, at).await.unwrap();
            times.push(at);
        }
        times
    }

    #[tokio::test]
    async fn a_match_without_a_bot_lobby_waits_for_an_admin() {
        let pool = memory_db().await;
        let (match_id, _, _) = finished_match(&pool, None).await;
        assert_eq!(load(&pool, &match_id).await.unwrap().unwrap().status, NEEDS_ID);
        let mut budget = SteamBudget::new(false);
        assert!(step(&pool, &FakeApi::default(), &mut budget, now()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_bot_lobby_becomes_a_scoreboard_and_links_the_1v1_players() {
        let pool = memory_db().await;
        let (match_id, king, arch) = finished_match(&pool, Some("123")).await;
        let t0 = ended_at(&pool, &match_id).await;
        let api = FakeApi { match_id: Some(139086680), metadata: Some(metadata()), ..FakeApi::default() };
        let mut budget = SteamBudget::new(false);

        assert_eq!(step(&pool, &api, &mut budget, t0).await.unwrap(), HashSet::from([1]));
        let row = load(&pool, &match_id).await.unwrap().unwrap();
        assert_eq!((row.status.as_str(), row.deadlock_match_id), (PENDING, Some(139086680)));

        // The data is tried a minute later; that soon after the match, without asking Steam.
        assert!(step(&pool, &api, &mut budget, t0).await.unwrap().is_empty());
        step(&pool, &api, &mut budget, t0 + 60).await.unwrap();
        let row = load(&pool, &match_id).await.unwrap().unwrap();
        assert_eq!(row.status, READY);
        assert_eq!(row.scoreboard.unwrap().players.len(), 2);
        assert_eq!(*api.calls.lock().unwrap(), [false]);
        assert_eq!(account_links(&pool, 1).await.unwrap(), HashMap::from([(100, king), (200, arch)]));
    }

    #[tokio::test]
    async fn an_unknown_party_asks_for_the_id() {
        let pool = memory_db().await;
        let (match_id, _, _) = finished_match(&pool, Some("123")).await;
        let t0 = ended_at(&pool, &match_id).await;
        let mut budget = SteamBudget::new(false);
        for i in 0..PARTY_TRIES {
            step(&pool, &FakeApi::default(), &mut budget, t0 + i * PARTY_RETRY_S).await.unwrap();
        }
        let row = load(&pool, &match_id).await.unwrap().unwrap();
        assert_eq!(row.status, NEEDS_ID);
        assert!(row.last_error.unwrap().contains("Informe o ID"));
    }

    #[tokio::test]
    async fn steam_is_asked_late_sparingly_and_within_the_hours_budget() {
        let pool = memory_db().await;
        let (match_id, _, _) = finished_match(&pool, None).await;
        let t0 = ended_at(&pool, &match_id).await;
        // The admin gives the ID right as the match ends, and the API never has it.
        set_deadlock_id(&pool, &match_id, 42).await.unwrap();
        sqlx::query("UPDATE match_stats SET next_attempt_at = ? WHERE match_id = ?")
            .bind(t0)
            .bind(&match_id)
            .execute(&pool)
            .await
            .unwrap();
        let api = FakeApi::default();
        let times = run_until_settled(&pool, &api, &mut SteamBudget::new(false), &match_id).await;

        let offsets: Vec<i64> = times.iter().map(|t| t - t0).collect();
        // Not before 5 minutes; then every 15 minutes at most; the third Steam try within an hour
        // (at 35 minutes) is over the budget of 2, so it waits for a slot at 65 minutes.
        assert_eq!(offsets, [0, 120, 300, 600, 1200, 2100, 3900, 5700, 9300]);
        assert_eq!(*api.calls.lock().unwrap(), [false, false, true, false, true, false, true, true, true]);
        let row = load(&pool, &match_id).await.unwrap().unwrap();
        assert_eq!((row.status.as_str(), row.deadlock_match_id), (FAILED, Some(42)));
    }

    #[tokio::test]
    async fn with_a_key_every_try_from_5_minutes_asks_steam() {
        let pool = memory_db().await;
        let (match_id, _, _) = finished_match(&pool, None).await;
        let t0 = ended_at(&pool, &match_id).await;
        set_deadlock_id(&pool, &match_id, 42).await.unwrap();
        let api = FakeApi { key: true, ..FakeApi::default() };
        let mut budget = SteamBudget::new(true);
        for offset in [0, 300, 360] {
            sqlx::query("UPDATE match_stats SET next_attempt_at = ? WHERE match_id = ?")
                .bind(t0 + offset)
                .bind(&match_id)
                .execute(&pool)
                .await
                .unwrap();
            step(&pool, &api, &mut budget, t0 + offset).await.unwrap();
        }
        assert_eq!(*api.calls.lock().unwrap(), [false, true, true]);
    }

    #[test]
    fn the_steam_budget_is_per_rolling_hour() {
        let mut budget = SteamBudget::new(false);
        assert!(budget.take(1000));
        assert!(budget.take(2000));
        assert!(!budget.take(2500));
        assert_eq!(budget.next_free(2500), 4600);
        assert!(budget.take(4600));
        assert_eq!(budget.next_free(4600), 5600);
    }
}
