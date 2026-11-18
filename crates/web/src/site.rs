//! The public league site (`site/`), which replaces the old PHP staff panel. Its pages and assets
//! are static files; the three scripts they read their data from (`info.js`, `jogadores.js` and
//! `partida.js`) are built here from what staff typed in the dashboard for the published camp.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::Result;
use axum::extract::{Path, Request, State};
use axum::http::header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, WWW_AUTHENTICATE};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use refresher_core::config::WebConfig;
use refresher_core::db;
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

/// A ranking row (`window.JOGADORES`); the page sorts them by points.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct RankedPlayer {
    pub nome: String,
    #[serde(default)]
    pub pontos: i64,
}

/// Match scoreboards, and the name and kind of each item their icons show (`window.PARTIDAS`).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Matches {
    #[serde(default)]
    pub partidas: Vec<SiteMatch>,
    #[serde(default)]
    pub itens: BTreeMap<String, Item>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Item {
    pub nome: String,
    /// `weapon`, `spirit` or `vitality`: the tile's color.
    pub tipo: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<i64>,
    #[serde(default)]
    pub ativo: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SiteMatch {
    /// Picked by staff by a player's name in this match; not the game's MVP.
    #[serde(default)]
    pub mvp: Option<String>,
    /// The Deadlock match ID.
    #[serde(default)]
    pub id: Option<u64>,
    /// When it started, in Unix seconds.
    pub inicio: i64,
    /// How long it lasted, in seconds.
    pub duracao: i64,
    /// The winning team's index in `times`.
    pub vencedor: usize,
    pub times: Vec<Team>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Team {
    pub nome: String,
    /// `king` or `arch`: the side's colors.
    pub classe: String,
    pub almas: Stat,
    pub jogadores: Vec<MatchPlayer>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MatchPlayer {
    pub nome: String,
    pub heroi: String,
    /// The hero's icon: `assets/partida/herois/{img}.webp`.
    pub img: String,
    pub almas: Stat,
    pub k: i64,
    pub d: i64,
    pub a: i64,
    pub dano: Stat,
    pub obj: Stat,
    pub cura: Stat,
    /// Item icons: `assets/partida/itens/{slug}.webp`.
    #[serde(default)]
    pub itens: Vec<String>,
}

/// A number as the scoreboard shows it, e.g. `40k` or `979`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Stat {
    Number(i64),
    Text(String),
}

/// Everything the site shows for a camp.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SiteContent {
    pub info: Info,
    pub players: Vec<RankedPlayer>,
    pub matches: Matches,
}

/// Names and kinds of the items the site has icons for, from the old `partida.js`.
static ITEM_CATALOG: LazyLock<BTreeMap<String, Item>> =
    LazyLock::new(|| serde_json::from_str(include_str!("site_items.json")).expect("site_items.json is valid"));

impl SiteContent {
    /// A camp nobody has written anything for yet.
    pub fn empty() -> Self {
        SiteContent {
            info: clean_info(Info::default()),
            players: Vec::new(),
            matches: Matches {
                partidas: Vec::new(),
                itens: ITEM_CATALOG.clone(),
            },
        }
    }
}

const MAX_PLAYERS: usize = 200;
const MAX_MATCHES: usize = 200;
const MAX_ITEMS: usize = 1000;

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

/// Rows without a name are dropped.
fn clean_players(players: Vec<RankedPlayer>) -> Vec<RankedPlayer> {
    players
        .into_iter()
        .filter_map(|p| {
            let nome = text(&p.nome, 40);
            (!nome.is_empty()).then(|| RankedPlayer {
                nome,
                pontos: p.pontos.clamp(-99_999, 999_999),
            })
        })
        .take(MAX_PLAYERS)
        .collect()
}

/// Icon names end up in URLs, so they stay plain.
fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 60
        && value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn clean_stat(stat: Stat) -> Stat {
    match stat {
        Stat::Number(n) => Stat::Number(n.clamp(0, 1_000_000_000)),
        Stat::Text(s) => Stat::Text(text(&s, 12)),
    }
}

/// Refuses scoreboards the pages can't draw; an MVP who didn't play in the match is cleared.
fn clean_matches(matches: Matches) -> Result<Matches, String> {
    if matches.partidas.len() > MAX_MATCHES {
        return Err(format!("No máximo {MAX_MATCHES} partidas."));
    }
    if matches.itens.len() > MAX_ITEMS {
        return Err(format!("No máximo {MAX_ITEMS} itens."));
    }
    let mut partidas = Vec::with_capacity(matches.partidas.len());
    for (i, mut m) in matches.partidas.into_iter().enumerate() {
        let n = i + 1;
        if m.times.len() != 2 {
            return Err(format!("Partida {n}: precisa de exatamente dois times."));
        }
        if m.vencedor > 1 {
            return Err(format!("Partida {n}: \"vencedor\" é 0 (primeiro time) ou 1 (segundo)."));
        }
        if m.inicio < 0 || !(0..=24 * 3600).contains(&m.duracao) {
            return Err(format!("Partida {n}: início ou duração inválidos."));
        }
        for team in &mut m.times {
            team.nome = text(&team.nome, 40);
            if team.nome.is_empty() {
                return Err(format!("Partida {n}: um time está sem nome."));
            }
            if team.classe != "king" && team.classe != "arch" {
                return Err(format!("Partida {n}: \"classe\" do time é \"king\" ou \"arch\"."));
            }
            if team.jogadores.is_empty() || team.jogadores.len() > 12 {
                return Err(format!("Partida {n}: cada time tem de 1 a 12 jogadores."));
            }
            team.almas = clean_stat(team.almas.clone());
            for p in &mut team.jogadores {
                p.nome = text(&p.nome, 40);
                p.heroi = text(&p.heroi, 40);
                if p.nome.is_empty() || p.heroi.is_empty() {
                    return Err(format!("Partida {n}: um jogador está sem nome ou sem herói."));
                }
                if !is_slug(&p.img) {
                    return Err(format!("Partida {n}: ícone de herói inválido para {}.", p.nome));
                }
                if p.itens.len() > 30 || !p.itens.iter().all(|s| is_slug(s)) {
                    return Err(format!("Partida {n}: itens inválidos para {}.", p.nome));
                }
                for kda in [&mut p.k, &mut p.d, &mut p.a] {
                    *kda = (*kda).clamp(0, 9999);
                }
                for stat in [&mut p.almas, &mut p.dano, &mut p.obj, &mut p.cura] {
                    *stat = clean_stat(stat.clone());
                }
            }
        }
        let played = |name: &String| m.times.iter().flat_map(|t| &t.jogadores).any(|p| &p.nome == name);
        m.mvp = m.mvp.map(|name| text(&name, 40)).filter(played);
        partidas.push(m);
    }
    let mut itens = BTreeMap::new();
    for (slug, mut item) in matches.itens {
        if !is_slug(&slug) {
            return Err(format!("Item com nome de ícone inválido: {slug}."));
        }
        item.nome = text(&item.nome, 60);
        if !["weapon", "spirit", "vitality"].contains(&item.tipo.as_str()) {
            return Err(format!("Item {slug}: \"tipo\" é weapon, spirit ou vitality."));
        }
        itens.insert(slug, item);
    }
    Ok(Matches { partidas, itens })
}

pub fn clean(content: SiteContent) -> Result<SiteContent, String> {
    Ok(SiteContent {
        info: clean_info(content.info),
        players: clean_players(content.players),
        matches: clean_matches(content.matches)?,
    })
}

async fn load(db: &SqlitePool, camp_id: &str) -> Result<SiteContent> {
    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT info, players, matches FROM site_content WHERE camp_id = ?")
            .bind(camp_id)
            .fetch_optional(db)
            .await?;
    let Some((info, players, matches)) = row else {
        return Ok(SiteContent::empty());
    };
    Ok(SiteContent {
        info: serde_json::from_str(&info)?,
        players: serde_json::from_str(&players)?,
        matches: serde_json::from_str(&matches)?,
    })
}

async fn save(db: &SqlitePool, camp_id: &str, content: &SiteContent) -> Result<()> {
    sqlx::query(
        "INSERT INTO site_content (camp_id, info, players, matches, updated_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (camp_id) DO UPDATE SET info = excluded.info, players = excluded.players,
             matches = excluded.matches, updated_at = excluded.updated_at",
    )
    .bind(camp_id)
    .bind(serde_json::to_string(&content.info)?)
    .bind(serde_json::to_string(&content.players)?)
    .bind(serde_json::to_string(&content.matches)?)
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

// --- Dashboard API ---

#[derive(Serialize)]
pub struct SiteDto {
    /// This camp is the one the site shows.
    published: bool,
    url: String,
    content: SiteContent,
}

/// A camp of the site's server that the admin manages, or 404.
async fn site_camp(state: &AppState, session: &crate::session::Session, camp_id: &str) -> Result<(String, i64), ApiError> {
    let camp = admin_camp(state, session, camp_id).await?;
    if state.config.site_guild_id != Some(camp.camp.guild_id as u64) {
        return Err(ApiError::NotFound);
    }
    Ok((camp.camp.id, camp.camp.guild_id))
}

async fn site_dto(state: &AppState, camp_id: &str, guild_id: i64) -> Result<SiteDto, ApiError> {
    Ok(SiteDto {
        published: published_camp(&state.db, guild_id as u64).await?.as_deref() == Some(camp_id),
        url: site_url(&state.config),
        content: load(&state.db, camp_id).await?,
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

/// Replaces everything the site shows for the camp. Shows on the site right away if it's published.
pub async fn save_camp_site(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(camp_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<SiteDto>, ApiError> {
    let (camp_id, guild_id) = site_camp(&state, &session, &camp_id).await?;
    let content: SiteContent =
        serde_json::from_value(body).map_err(|err| ApiError::BadRequest(format!("Dados inválidos: {err}")))?;
    let content = clean(content).map_err(ApiError::BadRequest)?;
    save(&state.db, &camp_id, &content).await?;
    tracing::info!(user_id = session.user_id, camp_id, "site content saved from the web");
    Ok(Json(site_dto(&state, &camp_id, guild_id).await?))
}

/// Shows this camp on the site instead of the one before.
pub async fn publish_camp_site(
    State(state): State<AppState>,
    Admin(session): Admin,
    Path(camp_id): Path<String>,
) -> Result<Json<SiteDto>, ApiError> {
    let (camp_id, guild_id) = site_camp(&state, &session, &camp_id).await?;
    publish(&state.db, guild_id, &camp_id).await?;
    tracing::info!(user_id = session.user_id, camp_id, "camp published on the site");
    Ok(Json(site_dto(&state, &camp_id, guild_id).await?))
}

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/camps/{camp_id}/site", get(camp_site).put(save_camp_site))
        .route("/camps/{camp_id}/site/publish", post(publish_camp_site))
}

// --- The public site ---

/// What the published camp shows; a site with none shows "to be decided" everywhere.
async fn published_content(state: &AppState) -> Result<SiteContent> {
    let Some(guild_id) = state.config.site_guild_id else {
        return Ok(SiteContent::empty());
    };
    match published_camp(&state.db, guild_id).await? {
        Some(camp_id) => load(&state.db, &camp_id).await,
        None => Ok(SiteContent::empty()),
    }
}

/// `window.{name} = …;`, re-checked by browsers on every visit so saves show up right away.
fn data_script(name: &str, value: &impl Serialize) -> Result<Response, ApiError> {
    let json = serde_json::to_string_pretty(value)?;
    Ok((
        [
            (CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (CACHE_CONTROL, "no-cache, must-revalidate"),
        ],
        format!("// Gerado pelo bot com o que a staff salvou no painel (Site).\nwindow.{name} = {json};\n"),
    )
        .into_response())
}

async fn info_js(State(state): State<AppState>) -> Result<Response, ApiError> {
    data_script("INFO", &published_content(&state).await?.info)
}

async fn players_js(State(state): State<AppState>) -> Result<Response, ApiError> {
    data_script("JOGADORES", &published_content(&state).await?.players)
}

async fn matches_js(State(state): State<AppState>) -> Result<Response, ApiError> {
    data_script("PARTIDAS", &published_content(&state).await?.matches)
}

/// The footer's Staff link: the site is edited in the dashboard now.
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

/// While `SITE_LOCK` is set, the whole site asks for that user and password.
async fn require_lock(State(expected): State<Option<String>>, request: Request, next: Next) -> Response {
    if let Some(expected) = expected {
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

/// The site's pages, assets and data scripts, from its root.
pub fn router(state: AppState) -> Router {
    let lock = state.config.site_lock.as_ref().map(|l| format!("Basic {}", STANDARD.encode(l)));
    let files = ServeDir::new(PathBuf::from(&state.config.site_dir));
    Router::new()
        .route("/info.js", get(info_js))
        .route("/jogadores.js", get(players_js))
        .route("/partida.js", get(matches_js))
        .route("/staff", get(staff))
        .route("/staff/", get(staff))
        .route("/staff/{*rest}", get(staff))
        .fallback_service(files)
        .layer(middleware::from_fn(cache_headers))
        .layer(middleware::from_fn_with_state(lock, require_lock))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(nome: &str) -> MatchPlayer {
        MatchPlayer {
            nome: nome.to_string(),
            heroi: "Paige".to_string(),
            img: "paige".to_string(),
            almas: Stat::Text("40k".to_string()),
            k: 1,
            d: 2,
            a: 3,
            dano: Stat::Text("19k".to_string()),
            obj: Stat::Number(979),
            cura: Stat::Text("25k".to_string()),
            itens: vec!["knockdown".to_string()],
        }
    }

    pub(crate) fn sample_match() -> SiteMatch {
        let team = |nome: &str, classe: &str, who: &str| Team {
            nome: nome.to_string(),
            classe: classe.to_string(),
            almas: Stat::Text("270k".to_string()),
            jogadores: vec![player(who)],
        };
        SiteMatch {
            mvp: Some("VioletaS2".to_string()),
            id: Some(139086680),
            inicio: 1794919042,
            duracao: 2026,
            vencedor: 0,
            times: vec![team("Hidden King", "king", "VioletaS2"), team("Archmother", "arch", "Zibra234")],
        }
    }

    #[test]
    fn empty_cards_say_to_be_decided_in_each_language() {
        let mut info = Info::default();
        info.data.pt = "  28 e 29\n de novembro ".to_string();
        let info = clean_info(info);
        assert_eq!(info.data.pt, "28 e 29 de novembro");
        assert_eq!(info.data.en, "TBD");
        assert_eq!(info.premiacao.es, "Por definir");
        assert_eq!(clean_info(Info { formato: Texts { pt: "x".repeat(100), ..Texts::default() }, ..Info::default() }).formato.pt.len(), 80);
    }

    #[test]
    fn ranking_drops_nameless_rows_and_bounds_points() {
        let players = clean_players(vec![
            RankedPlayer { nome: " Kaiser77 ".to_string(), pontos: 5_000_000 },
            RankedPlayer { nome: "   ".to_string(), pontos: 10 },
            RankedPlayer { nome: "hydan".to_string(), pontos: -200_000 },
        ]);
        assert_eq!(
            players,
            vec![
                RankedPlayer { nome: "Kaiser77".to_string(), pontos: 999_999 },
                RankedPlayer { nome: "hydan".to_string(), pontos: -99_999 },
            ]
        );
        let many = (0..300).map(|i| RankedPlayer { nome: format!("p{i}"), pontos: i }).collect();
        assert_eq!(clean_players(many).len(), MAX_PLAYERS);
    }

    #[test]
    fn an_mvp_who_did_not_play_is_cleared() {
        let mut m = sample_match();
        m.mvp = Some("someone else".to_string());
        let cleaned = clean_matches(Matches { partidas: vec![sample_match(), m], itens: BTreeMap::new() }).unwrap();
        assert_eq!(cleaned.partidas[0].mvp.as_deref(), Some("VioletaS2"));
        assert_eq!(cleaned.partidas[1].mvp, None);
    }

    #[test]
    fn scoreboards_the_pages_cannot_draw_are_refused() {
        let check = |change: fn(&mut SiteMatch)| {
            let mut m = sample_match();
            change(&mut m);
            clean_matches(Matches { partidas: vec![m], itens: BTreeMap::new() }).unwrap_err()
        };
        assert!(check(|m| m.vencedor = 2).contains("vencedor"));
        assert!(check(|m| m.times.truncate(1)).contains("dois times"));
        assert!(check(|m| m.times[0].classe = "blue".to_string()).contains("classe"));
        assert!(check(|m| m.times[1].jogadores[0].img = "../x".to_string()).contains("herói"));
        assert!(check(|m| m.times[1].jogadores[0].itens.push("<b>".to_string())).contains("itens"));
    }

    #[test]
    fn the_old_item_catalog_loads() {
        let empty = SiteContent::empty();
        assert_eq!(empty.matches.itens["refresher"].nome, "Refresher");
        assert_eq!(clean(empty.clone()).unwrap(), empty);
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
