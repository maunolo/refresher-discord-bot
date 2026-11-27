//! The public league site (`site/`). Its pages and assets are static files. Its data comes from
//! the camp the site's server publishes: the ranking and the match history (each match's
//! scoreboard from the Deadlock API, see `refresher_core::stats`), plus the four info cards staff
//! write in the dashboard. It's served as JSON under `/api/` and as the three scripts the pages
//! load (`info.js`, `jogadores.js` and `partida.js`).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path as FsPath, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use axum::extract::{Path, Request, State};
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_ORIGIN, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, WWW_AUTHENTICATE,
};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use refresher_core::config::WebConfig;
use refresher_core::db;
use refresher_core::link::Profile;
use refresher_core::ranking::{rank, Standing};
use refresher_core::stats::{self, slug, Scoreboard, ScoreboardPlayer};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tower_http::services::ServeDir;

use crate::api::admin_camp;
use crate::auth::Admin;
use crate::error::ApiError;
use crate::AppState;

/// One info card's text in each of the site's languages.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Texts {
    pub pt: String,
    pub en: String,
    pub es: String,
}

/// The four cards at the top of the home page (`window.INFO`).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Info {
    pub data: Texts,
    pub formato: Texts,
    pub premiacao: Texts,
    pub times: Texts,
}

/// A ranking row, best first (`window.JOGADORES`, `/api/ranking`).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RankedPlayer {
    /// 1 is first; players equal on points and MVPs share it.
    pub posicao: usize,
    pub nome: String,
    pub pontos: i64,
    pub mvps: i64,
    pub vitorias: i64,
    pub partidas: i64,
    pub avatar: Option<String>,
}

/// The match history, oldest first, and the name and kind of each item its icons show
/// (`window.PARTIDAS`, `/api/matches`).
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Matches {
    pub partidas: Vec<SiteMatch>,
    pub itens: BTreeMap<String, Item>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Item {
    pub nome: String,
    /// `weapon`, `spirit` or `vitality`: the tile's color.
    pub tipo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<i64>,
    pub ativo: bool,
    /// The icon on the Deadlock API, for items the site has no icon of its own for.
    #[serde(rename = "imgUrl", skip_serializing_if = "Option::is_none")]
    pub img_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SiteMatch {
    /// The admin's pick, by name; not the game's MVP.
    pub mvp: Option<String>,
    /// The Deadlock match ID.
    pub id: u64,
    /// When it started, in Unix seconds.
    pub inicio: i64,
    /// How long it lasted, in seconds.
    pub duracao: i64,
    /// The winning team's index in `times`.
    pub vencedor: usize,
    /// Hidden King, then Archmother.
    pub times: Vec<Team>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Team {
    pub nome: String,
    /// `king` or `arch`: the side's colors.
    pub classe: String,
    pub almas: String,
    pub jogadores: Vec<MatchPlayer>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct MatchPlayer {
    /// The member's Discord name if their Steam account is linked, else their Steam name.
    pub nome: String,
    pub heroi: String,
    /// The hero's icon: `assets/partida/herois/{img}.webp`.
    pub img: String,
    /// The icon on the Deadlock API, for heroes the site has no icon of its own for.
    #[serde(rename = "imgUrl", skip_serializing_if = "Option::is_none")]
    pub img_url: Option<String>,
    pub almas: String,
    pub k: i64,
    pub d: i64,
    pub a: i64,
    pub dano: String,
    pub obj: String,
    pub cura: String,
    /// Item icons: `assets/partida/itens/{slug}.webp`, named in `itens`.
    pub itens: Vec<String>,
}

/// A number as the scoreboard shows it: `979`, `4.9k`, `40k`.
pub fn short(n: i64) -> String {
    if n < 1000 {
        n.to_string()
    } else if n < 9950 {
        format!("{:.1}k", n as f64 / 1000.0).replace(".0k", "k")
    } else {
        format!("{}k", (n as f64 / 1000.0).round())
    }
}

/// Whitespace runs become one space, then the text is cut to `max` characters.
fn text(value: &str, max: usize) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(max).collect()
}

/// Empty cards say "to be decided" in their language.
fn clean_info(mut info: Info) -> Info {
    for card in [&mut info.data, &mut info.formato, &mut info.premiacao, &mut info.times] {
        for (value, tbd) in [(&mut card.pt, "A definir"), (&mut card.en, "TBD"), (&mut card.es, "Por definir")] {
            let cleaned = text(value, 80);
            *value = if cleaned.is_empty() { tbd.to_string() } else { cleaned };
        }
    }
    info
}

async fn load_info(db: &SqlitePool, camp_id: &str) -> Result<Info> {
    let info: Option<String> = sqlx::query_scalar("SELECT info FROM site_content WHERE camp_id = ?")
        .bind(camp_id)
        .fetch_optional(db)
        .await?;
    match info {
        Some(info) => Ok(serde_json::from_str(&info)?),
        None => Ok(clean_info(Info::default())),
    }
}

async fn save_info(db: &SqlitePool, camp_id: &str, info: &Info) -> Result<()> {
    sqlx::query(
        "INSERT INTO site_content (camp_id, info, updated_at) VALUES (?, ?, ?)
         ON CONFLICT (camp_id) DO UPDATE SET info = excluded.info, updated_at = excluded.updated_at",
    )
    .bind(camp_id)
    .bind(serde_json::to_string(info)?)
    .bind(db::now())
    .execute(db)
    .await?;
    Ok(())
}

async fn published_camp(db: &SqlitePool, guild_id: u64) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT camp_id FROM site_published WHERE guild_id = ?")
        .bind(guild_id as i64)
        .fetch_optional(db)
        .await?)
}

async fn publish(db: &SqlitePool, guild_id: i64, camp_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO site_published (guild_id, camp_id, updated_at) VALUES (?, ?, ?)
         ON CONFLICT (guild_id) DO UPDATE SET camp_id = excluded.camp_id, updated_at = excluded.updated_at",
    )
    .bind(guild_id)
    .bind(camp_id)
    .bind(db::now())
    .execute(db)
    .await?;
    Ok(())
}

/// Where visitors open the site.
pub fn site_url(config: &WebConfig) -> String {
    match config.site_hosts.first() {
        Some(host) => format!("https://{host}/"),
        None => format!("{}/site/", config.public_url),
    }
}

/// Everything the site shows.
#[derive(Default)]
pub struct SiteData {
    info: Info,
    ranking: Vec<RankedPlayer>,
    matches: Matches,
}

/// Whether the site ships its own icon, `site/assets/partida/{folder}/{name}.webp`.
fn has_icon(site_dir: &str, folder: &str, name: &str) -> bool {
    FsPath::new(site_dir)
        .join("assets/partida")
        .join(folder)
        .join(format!("{name}.webp"))
        .is_file()
}

/// A Deadlock scoreboard in the site's shape, with Discord names for linked accounts.
fn site_match(
    board: &Scoreboard,
    mvp: Option<String>,
    name_of: &dyn Fn(&ScoreboardPlayer) -> String,
    site_dir: &str,
    itens: &mut BTreeMap<String, Item>,
) -> SiteMatch {
    let teams = [("Hidden King", "king"), ("Archmother", "arch")];
    let times = teams
        .iter()
        .enumerate()
        .map(|(team, (nome, classe))| {
            let players: Vec<&ScoreboardPlayer> = board.players.iter().filter(|p| p.team == team).collect();
            Team {
                nome: nome.to_string(),
                classe: classe.to_string(),
                almas: short(players.iter().map(|p| p.souls).sum()),
                jogadores: players
                    .iter()
                    .map(|p| {
                        let img = slug(&p.hero);
                        let own_icon = has_icon(site_dir, "herois", &img);
                        MatchPlayer {
                            nome: name_of(p),
                            heroi: p.hero.clone(),
                            img_url: if own_icon { None } else { p.hero_image.clone() },
                            img,
                            almas: short(p.souls),
                            k: p.kills,
                            d: p.deaths,
                            a: p.assists,
                            dano: short(p.damage),
                            obj: short(p.objective_damage),
                            cura: short(p.healing),
                            itens: p
                                .items
                                .iter()
                                .map(|item| {
                                    let key = slug(&item.name);
                                    itens.entry(key.clone()).or_insert_with(|| Item {
                                        nome: item.name.clone(),
                                        tipo: item.slot.clone(),
                                        tier: item.tier,
                                        ativo: item.active,
                                        img_url: (!has_icon(site_dir, "itens", &key))
                                            .then(|| item.image.clone())
                                            .flatten(),
                                    });
                                    key
                                })
                                .collect(),
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    SiteMatch {
        mvp,
        id: board.deadlock_match_id,
        inicio: board.start_time,
        duracao: board.duration_s,
        vencedor: board.winning_team,
        times,
    }
}

/// The published camp's info, ranking and the finished matches that have a scoreboard.
async fn site_data(state: &AppState) -> Result<SiteData> {
    let Some(guild_id) = state.config.site_guild_id else {
        return Ok(SiteData { info: clean_info(Info::default()), ..SiteData::default() });
    };
    let Some(camp_id) = published_camp(&state.db, guild_id).await? else {
        return Ok(SiteData { info: clean_info(Info::default()), ..SiteData::default() });
    };
    let info = load_info(&state.db, &camp_id).await?;
    let Some(camp) = db::load_camp(&state.db, &camp_id).await? else {
        return Ok(SiteData { info, ..SiteData::default() });
    };

    let mut finished: Vec<db::MatchDetail> = db::list_matches(&state.db, &camp_id)
        .await?
        .into_iter()
        .filter(|m| m.number.is_some())
        .collect();
    finished.sort_by_key(|m| m.number);
    let boards = stats::camp_scoreboards(&state.db, &camp_id).await?;
    let links = stats::account_links(&state.db, guild_id as i64).await?;

    let mut user_ids: Vec<u64> = camp.players.iter().map(|p| p.user_id as u64).collect();
    user_ids.extend(links.values().map(|u| *u as u64));
    user_ids.extend(finished.iter().filter_map(|m| m.row.mvp_user_id).map(|u| u as u64));
    user_ids.sort_unstable();
    user_ids.dedup();
    let profiles: HashMap<u64, Profile> = state.link.profiles(guild_id, &user_ids).await;
    let discord_name = |user_id: i64| profiles.get(&(user_id as u64)).map(|p| p.name.clone());

    let player_stats = db::player_stats(&state.db, &camp_id).await?;
    let standings: Vec<Standing> = camp
        .players
        .iter()
        .map(|p| Standing {
            user_id: p.user_id,
            points: p.points,
            matches: p.matches_played,
            mvps: player_stats.get(&p.user_id).map_or(0, |s| s.mvps),
            wins: player_stats.get(&p.user_id).map_or(0, |s| s.wins),
        })
        .collect();
    let ranking = rank(&standings)
        .into_iter()
        .map(|(s, place)| {
            let played = player_stats.get(&s.user_id).copied().unwrap_or_default();
            RankedPlayer {
                posicao: place,
                nome: discord_name(s.user_id).unwrap_or_else(|| s.user_id.to_string()),
                pontos: s.points,
                mvps: s.mvps,
                vitorias: played.wins,
                partidas: played.played,
                avatar: profiles
                    .get(&(s.user_id as u64))
                    .map(|p| p.avatar_url.clone())
                    .filter(|url| !url.is_empty()),
            }
        })
        .collect();

    let name_of = |p: &ScoreboardPlayer| {
        links
            .get(&p.account_id)
            .and_then(|user| discord_name(*user))
            .or_else(|| p.steam_name.clone())
            .unwrap_or_else(|| "Jogador".to_string())
    };
    let mut matches = Matches::default();
    for m in &finished {
        if let Some(board) = boards.get(&m.row.id) {
            let mvp = m.row.mvp_user_id.and_then(discord_name);
            let site = site_match(board, mvp, &name_of, &state.config.site_dir, &mut matches.itens);
            matches.partidas.push(site);
        }
    }
    Ok(SiteData { info, ranking, matches })
}

/// The site's data, shared for a few seconds: one page load asks for it five times at once, and
/// each build looks names up on Discord.
#[derive(Default)]
pub struct SiteCache(tokio::sync::Mutex<Option<(Instant, Arc<SiteData>)>>);

const SITE_CACHE_TTL: Duration = Duration::from_secs(5);

impl SiteCache {
    /// Forgets the data, after an admin changes what the site shows.
    async fn clear(&self) {
        *self.0.lock().await = None;
    }
}

/// The site's data, built once for requests arriving together.
async fn shared_site_data(state: &AppState) -> Result<Arc<SiteData>> {
    let mut slot = state.site_cache.0.lock().await;
    if let Some((at, data)) = slot.as_ref() {
        if at.elapsed() < SITE_CACHE_TTL {
            return Ok(data.clone());
        }
    }
    let data = Arc::new(site_data(state).await?);
    *slot = Some((Instant::now(), data.clone()));
    Ok(data)
}

/// Whether admins opened the site to everyone while `SITE_LOCK` is set. Read from the database on
/// the first request, then kept here: every file of the site asks.
#[derive(Default)]
pub struct SiteAccess(tokio::sync::OnceCell<AtomicBool>);

impl SiteAccess {
    async fn unlocked(&self, state: &AppState) -> Result<&AtomicBool> {
        self.0
            .get_or_try_init(|| async {
                let unlocked = match state.config.site_guild_id {
                    Some(guild_id) => sqlx::query_scalar::<_, bool>(
                        "SELECT unlocked != 0 FROM site_access WHERE guild_id = ?",
                    )
                    .bind(guild_id as i64)
                    .fetch_optional(&state.db)
                    .await?
                    .unwrap_or(false),
                    None => false,
                };
                Ok(AtomicBool::new(unlocked))
            })
            .await
    }

    /// Whether the site asks for `SITE_LOCK`'s login now.
    async fn locked(&self, state: &AppState) -> Result<bool> {
        if state.config.site_lock.is_none() {
            return Ok(false);
        }
        Ok(!self.unlocked(state).await?.load(Ordering::Relaxed))
    }

    async fn set_locked(&self, state: &AppState, guild_id: i64, locked: bool) -> Result<()> {
        sqlx::query(
            "INSERT INTO site_access (guild_id, unlocked, updated_at) VALUES (?, ?, ?)
             ON CONFLICT (guild_id) DO UPDATE SET unlocked = excluded.unlocked, updated_at = excluded.updated_at",
        )
        .bind(guild_id)
        .bind(!locked)
        .bind(db::now())
        .execute(&state.db)
        .await?;
        // After the save, so a first read racing with it can't keep the old value.
        self.unlocked(state).await?.store(!locked, Ordering::Relaxed);
        Ok(())
    }
}

// --- Dashboard API ---

#[derive(Serialize)]
pub struct SiteDto {
    /// This camp is the one the site shows.
    published: bool,
    url: String,
    info: Info,
    /// Finished matches of the camp, and how many of them have a scoreboard for the site.
    finished_matches: usize,
    scoreboards: usize,
    /// `SITE_LOCK` is set: the site asks for its login unless admins unlocked it.
    lock_configured: bool,
    /// The site asks for the login now.
    locked: bool,
}

#[derive(Deserialize)]
pub struct SiteRequest {
    info: Info,
}

#[derive(Deserialize)]
pub struct LockRequest {
    locked: bool,
}

/// A camp of the site's server that the admin manages, or 404.
async fn site_camp(
    state: &AppState,
    session: &crate::session::Session,
    camp_id: &str,
) -> Result<(String, i64), ApiError> {
    let camp = admin_camp(state, session, camp_id).await?;
    if state.config.site_guild_id != Some(camp.camp.guild_id as u64) {
        return Err(ApiError::NotFound);
    }
    Ok((camp.camp.id, camp.camp.guild_id))
}

async fn site_dto(state: &AppState, camp_id: &str, guild_id: i64) -> Result<SiteDto, ApiError> {
    let finished = db::list_matches(&state.db, camp_id)
        .await?
        .iter()
        .filter(|m| m.number.is_some())
        .count();
    Ok(SiteDto {
        published: published_camp(&state.db, guild_id as u64).await?.as_deref() == Some(camp_id),
        url: site_url(&state.config),
        info: load_info(&state.db, camp_id).await?,
        finished_matches: finished,
        scoreboards: stats::camp_scoreboards(&state.db, camp_id).await?.len(),
        lock_configured: state.config.site_lock.is_some(),
        locked: state.site_access.locked(state).await?,
    })
}

pub async fn camp_site(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(camp_id): Path<String>,
) -> Result<Json<SiteDto>, ApiError> {
    let (camp_id, guild_id) = site_camp(&state, &session, &camp_id).await?;
    Ok(Json(site_dto(&state, &camp_id, guild_id).await?))
}

/// Saves the camp's info cards. On the site right away if the camp is the published one.
pub async fn save_camp_site(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(camp_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<SiteDto>, ApiError> {
    let (camp_id, guild_id) = site_camp(&state, &session, &camp_id).await?;
    let request: SiteRequest =
        serde_json::from_value(body).map_err(|err| ApiError::BadRequest(format!("Dados inválidos: {err}")))?;
    save_info(&state.db, &camp_id, &clean_info(request.info)).await?;
    state.site_cache.clear().await;
    tracing::info!(user_id = session.user_id, camp_id, "site info saved from the web");
    Ok(Json(site_dto(&state, &camp_id, guild_id).await?))
}

/// The site shows this camp's ranking, matches and info from now on.
pub async fn publish_camp_site(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(camp_id): Path<String>,
) -> Result<Json<SiteDto>, ApiError> {
    let (camp_id, guild_id) = site_camp(&state, &session, &camp_id).await?;
    publish(&state.db, guild_id, &camp_id).await?;
    state.site_cache.clear().await;
    tracing::info!(user_id = session.user_id, camp_id, "camp published on the site");
    Ok(Json(site_dto(&state, &camp_id, guild_id).await?))
}

/// Opens the whole site to everyone, or asks for `SITE_LOCK`'s login again. It's the site's
/// setting, whichever of its server's camps the admin has open.
pub async fn lock_site(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(camp_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<SiteDto>, ApiError> {
    let (camp_id, guild_id) = site_camp(&state, &session, &camp_id).await?;
    let request: LockRequest =
        serde_json::from_value(body).map_err(|err| ApiError::BadRequest(format!("Dados inválidos: {err}")))?;
    if state.config.site_lock.is_none() {
        return Err(ApiError::Conflict(
            "O site não tem SITE_LOCK: já está aberto para todos.".to_string(),
        ));
    }
    state.site_access.set_locked(&state, guild_id, request.locked).await?;
    tracing::info!(user_id = session.user_id, locked = request.locked, "site lock changed from the web");
    Ok(Json(site_dto(&state, &camp_id, guild_id).await?))
}

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/camps/{camp_id}/site", get(camp_site).put(save_camp_site))
        .route("/camps/{camp_id}/site/publish", post(publish_camp_site))
        .route("/camps/{camp_id}/site/lock", put(lock_site))
}

// --- The public site ---

/// `window.{name} = …;`, re-checked by browsers on every visit so changes show up right away.
fn data_script(name: &str, value: &impl Serialize) -> Result<Response, ApiError> {
    let json = serde_json::to_string_pretty(value)?;
    Ok((
        [
            (CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (CACHE_CONTROL, "no-cache, must-revalidate"),
        ],
        format!("// Gerado pelo bot com os dados do camp publicado no site.\nwindow.{name} = {json};\n"),
    )
        .into_response())
}

/// JSON any page may read, from any domain.
fn public_json(value: &impl Serialize) -> Response {
    (
        [
            (ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            (CACHE_CONTROL, "no-cache, must-revalidate"),
        ],
        Json(value),
    )
        .into_response()
}

async fn info_js(State(state): State<AppState>) -> Result<Response, ApiError> {
    data_script("INFO", &shared_site_data(&state).await?.info)
}

async fn players_js(State(state): State<AppState>) -> Result<Response, ApiError> {
    data_script("JOGADORES", &shared_site_data(&state).await?.ranking)
}

async fn matches_js(State(state): State<AppState>) -> Result<Response, ApiError> {
    data_script("PARTIDAS", &shared_site_data(&state).await?.matches)
}

async fn info_api(State(state): State<AppState>) -> Result<Response, ApiError> {
    Ok(public_json(&shared_site_data(&state).await?.info))
}

async fn ranking_api(State(state): State<AppState>) -> Result<Response, ApiError> {
    Ok(public_json(&shared_site_data(&state).await?.ranking))
}

async fn matches_api(State(state): State<AppState>) -> Result<Response, ApiError> {
    Ok(public_json(&shared_site_data(&state).await?.matches))
}

/// The footer's Staff link: the site is edited in the dashboard.
async fn staff(State(state): State<AppState>) -> Redirect {
    Redirect::to(&format!("{}/#/site", state.config.public_url))
}

/// How long browsers keep each kind of file, as the old `.htaccess` had it.
fn cache_policy(path: &str) -> &'static str {
    let extension = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match extension.as_str() {
        "woff2" => "public, max-age=31536000",
        "webp" | "png" | "jpg" | "jpeg" | "webm" => "public, max-age=2592000",
        "js" | "css" => "public, max-age=604800",
        _ => "no-cache, must-revalidate",
    }
}

async fn cache_headers(request: Request, next: Next) -> Response {
    let policy = cache_policy(request.uri().path());
    let mut response = next.run(request).await;
    if response.status().is_success() && !response.headers().contains_key(CACHE_CONTROL) {
        response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static(policy));
    }
    response
}

/// While `SITE_LOCK` is set, the whole site asks for that user and password, until admins unlock
/// it on the Site tab.
async fn require_lock(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let locked = state.site_access.locked(&state).await.unwrap_or_else(|err| {
        tracing::error!("could not read whether the site is unlocked: {err:#}");
        true
    });
    if let Some(lock) = state.config.site_lock.as_ref().filter(|_| locked) {
        let expected = format!("Basic {}", STANDARD.encode(lock));
        let given = request.headers().get(AUTHORIZATION).and_then(|v| v.to_str().ok());
        if given != Some(expected.as_str()) {
            return (
                StatusCode::UNAUTHORIZED,
                [(WWW_AUTHENTICATE, "Basic realm=\"Refresher League\", charset=\"UTF-8\"")],
                "Site em teste: entre com o usuário e a senha da staff.",
            )
                .into_response();
        }
    }
    next.run(request).await
}

/// The site's pages, assets, data scripts and JSON, from its root.
pub fn router(state: AppState) -> Router {
    let files = ServeDir::new(PathBuf::from(&state.config.site_dir));
    Router::new()
        .route("/info.js", get(info_js))
        .route("/jogadores.js", get(players_js))
        .route("/partida.js", get(matches_js))
        .route("/api/info", get(info_api))
        .route("/api/ranking", get(ranking_api))
        .route("/api/matches", get(matches_api))
        .route("/staff", get(staff))
        .route("/staff/", get(staff))
        .route("/staff/{*rest}", get(staff))
        .fallback_service(files)
        .layer(middleware::from_fn(cache_headers))
        .layer(middleware::from_fn_with_state(state.clone(), require_lock))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use refresher_core::stats::ScoreboardItem;

    use super::*;

    #[test]
    fn numbers_read_like_the_game_scoreboard() {
        assert_eq!(short(979), "979");
        assert_eq!(short(4_912), "4.9k");
        assert_eq!(short(2_000), "2k");
        assert_eq!(short(9_960), "10k");
        assert_eq!(short(60_123), "60k");
        assert_eq!(short(270_499), "270k");
    }

    #[test]
    fn empty_cards_say_to_be_decided_in_each_language() {
        let mut info = Info::default();
        info.data.pt = "  28 e 29\n de novembro ".to_string();
        let info = clean_info(info);
        assert_eq!(info.data.pt, "28 e 29 de novembro");
        assert_eq!(info.data.en, "TBD");
        assert_eq!(info.premiacao.es, "Por definir");
    }

    fn board() -> Scoreboard {
        let player = |account_id, team, hero: &str, item: &str| ScoreboardPlayer {
            account_id,
            team,
            steam_name: Some(format!("steam{account_id}")),
            hero_id: 1,
            hero: hero.to_string(),
            hero_image: Some(format!("https://api/{hero}.webp")),
            kills: 22,
            deaths: 3,
            assists: 8,
            souls: 60_123,
            damage: 55_000,
            objective_damage: 13_000,
            healing: 979,
            items: vec![ScoreboardItem {
                id: 1,
                name: item.to_string(),
                slot: "spirit".to_string(),
                tier: Some(3),
                active: true,
                image: Some(format!("https://api/{item}.webp")),
            }],
        };
        Scoreboard {
            deadlock_match_id: 139086680,
            start_time: 1794919042,
            duration_s: 2026,
            winning_team: 1,
            players: vec![
                player(1, 0, "Wraith", "Knockdown"),
                player(2, 1, "Brand New Hero", "Brand New Item"),
            ],
        }
    }

    #[test]
    fn scoreboards_take_the_site_shape_and_fall_back_to_api_icons() {
        let site_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../site");
        let mut itens = BTreeMap::new();
        let name_of = |p: &ScoreboardPlayer| if p.account_id == 1 { "kaze".to_string() } else { p.steam_name.clone().unwrap() };
        let m = site_match(&board(), Some("kaze".to_string()), &name_of, site_dir, &mut itens);
        assert_eq!((m.id, m.vencedor, m.duracao), (139086680, 1, 2026));
        assert_eq!(m.mvp.as_deref(), Some("kaze"));
        assert_eq!((m.times[0].nome.as_str(), m.times[0].classe.as_str()), ("Hidden King", "king"));
        assert_eq!(m.times[1].classe, "arch");
        let wraith = &m.times[0].jogadores[0];
        assert_eq!((wraith.nome.as_str(), wraith.img.as_str()), ("kaze", "wraith"));
        assert_eq!((wraith.almas.as_str(), wraith.dano.as_str(), wraith.cura.as_str()), ("60k", "55k", "979"));
        // The site has its own Wraith and Knockdown icons; the new ones come from the API.
        assert_eq!(wraith.img_url, None);
        assert_eq!(itens["knockdown"].img_url, None);
        let new = &m.times[1].jogadores[0];
        assert_eq!(new.nome, "steam2");
        assert_eq!(new.img_url.as_deref(), Some("https://api/Brand New Hero.webp"));
        assert_eq!(itens["brand-new-item"].img_url.as_deref(), Some("https://api/Brand New Item.webp"));
        assert_eq!((itens["knockdown"].tipo.as_str(), itens["knockdown"].ativo), ("spirit", true));
    }

    #[test]
    fn files_keep_the_old_cache_times() {
        assert_eq!(cache_policy("/"), "no-cache, must-revalidate");
        assert_eq!(cache_policy("/historico.html"), "no-cache, must-revalidate");
        assert_eq!(cache_policy("/assets/fonts/VALVEOracle-Medium.woff2"), "public, max-age=31536000");
        assert_eq!(cache_policy("/assets/partida/herois/paige.webp"), "public, max-age=2592000");
        assert_eq!(cache_policy("/style.css"), "public, max-age=604800");
    }
}
