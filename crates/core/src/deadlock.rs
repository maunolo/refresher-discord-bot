use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;

const CREATE_URL: &str = "https://api.deadlock-api.com/v1/matches/custom/create";
const API_URL: &str = "https://api.deadlock-api.com/v1";

#[derive(Debug, Deserialize)]
pub struct CreateCustomResponse {
    pub party_id: String,
    pub party_code: String,
}

pub async fn create_custom_match(
    http: &Client,
    api_key: Option<&str>,
    server_region: Option<&str>,
) -> Result<(String, String)> {
    let mut body = serde_json::json!({
        "game_mode": "normal",
        "min_roster_size": 12,
        "is_publicly_visible": false,
        "duplicate_heroes_enabled": false
    });
    if let Some(region) = server_region {
        if let Ok(number) = region.parse::<u64>() {
            body["server_region"] = serde_json::json!(number);
        } else {
            body["server_region"] = serde_json::json!(region);
        }
    }

    let mut request = http.post(CREATE_URL).json(&body);
    if let Some(key) = api_key.filter(|k| !k.is_empty()) {
        request = request.header("X-API-Key", key);
    }
    let response = request
        .send()
        .await
        .context("falha ao chamar a API de partida custom do Deadlock")?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!(
            "A API do Deadlock recusou a criação da partida custom ({status}): {text}"
        ));
    }

    let parsed: CreateCustomResponse = serde_json::from_str(&text)
        .with_context(|| format!("resposta inesperada da API do Deadlock: {text}"))?;
    Ok((parsed.party_id, parsed.party_code))
}

fn get(http: &Client, api_key: Option<&str>, url: &str) -> reqwest::RequestBuilder {
    let request = http.get(url);
    match api_key.filter(|k| !k.is_empty()) {
        Some(key) => request.header("X-API-Key", key),
        None => request,
    }
}

/// The body of a 200, `None` for a 404 (not there yet), an error for anything else.
async fn fetch(request: reqwest::RequestBuilder, what: &str) -> Result<Option<String>> {
    let response = request
        .send()
        .await
        .with_context(|| format!("falha ao chamar a API do Deadlock ({what})"))?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        let text: String = text.chars().take(200).collect();
        return Err(anyhow!("a API do Deadlock respondeu {status} ({what}): {text}"));
    }
    Ok(Some(text))
}

/// The Deadlock match a custom lobby became, once it started. `None` while the API doesn't know.
pub async fn custom_match_id(http: &Client, api_key: Option<&str>, party_id: &str) -> Result<Option<u64>> {
    if party_id.is_empty() || !party_id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(anyhow!("party inválida: {party_id}"));
    }
    #[derive(Deserialize)]
    struct Found {
        match_id: u64,
    }
    let url = format!("{API_URL}/matches/custom/{party_id}/match-id");
    let text = fetch(get(http, api_key, &url), "ID da partida").await?;
    Ok(text
        .and_then(|text| serde_json::from_str::<Found>(&text).ok())
        .map(|found| found.match_id)
        .filter(|id| *id > 0))
}

/// What the bot reads of a match's metadata (`CMsgMatchMetaDataContents`).
#[derive(Clone, Debug, Deserialize)]
pub struct Metadata {
    pub match_info: MatchInfo,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MatchInfo {
    pub match_id: u64,
    pub start_time: i64,
    pub duration_s: i64,
    /// `0` is the Hidden King's team, `1` the Archmother's.
    pub winning_team: serde_json::Value,
    pub players: Vec<MetaPlayer>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MetaPlayer {
    pub account_id: u64,
    pub team: serde_json::Value,
    pub hero_id: u64,
    #[serde(default)]
    pub kills: i64,
    #[serde(default)]
    pub deaths: i64,
    #[serde(default)]
    pub assists: i64,
    #[serde(default)]
    pub net_worth: i64,
    /// Everything bought, abilities included, in order.
    #[serde(default)]
    pub items: Vec<MetaItem>,
    /// Running totals over the match; the last entry is the end of the match.
    #[serde(default)]
    pub stats: Vec<MetaStats>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MetaItem {
    pub item_id: u64,
    #[serde(default)]
    pub game_time_s: i64,
    /// `0` if the player still had it at the end.
    #[serde(default)]
    pub sold_time_s: i64,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct MetaStats {
    #[serde(default)]
    pub player_damage: i64,
    /// Damage to objectives: guardians, walkers, shrines and the patron.
    #[serde(default)]
    pub boss_damage: i64,
    #[serde(default)]
    pub player_healing: i64,
}

/// `0` or `1` from a team as the API writes it: a number, or a name ending in `0`/`1`.
pub fn team_index(team: &serde_json::Value) -> Option<usize> {
    match team {
        serde_json::Value::Number(n) => n.as_u64().filter(|n| *n <= 1).map(|n| n as usize),
        serde_json::Value::String(s) if s.ends_with('0') => Some(0),
        serde_json::Value::String(s) if s.ends_with('1') => Some(1),
        _ => None,
    }
}

/// A match's metadata, `None` while the API doesn't have it. Without `from_steam` the API only
/// looks in its own store; asking Steam is rate limited hard (3 an hour per IP without a key).
pub async fn match_metadata(
    http: &Client,
    api_key: Option<&str>,
    match_id: u64,
    from_steam: bool,
) -> Result<Option<Metadata>> {
    let url = format!(
        "{API_URL}/matches/{match_id}/metadata?is_custom=true&disable_steam={}",
        !from_steam
    );
    match fetch(get(http, api_key, &url), "dados da partida").await? {
        Some(text) => Ok(Some(
            serde_json::from_str(&text).context("dados da partida em formato inesperado")?,
        )),
        None => Ok(None),
    }
}

/// Steam names of the given accounts; accounts the API doesn't know are left out.
pub async fn steam_names(
    http: &Client,
    api_key: Option<&str>,
    account_ids: &[u64],
) -> Result<HashMap<u64, String>> {
    if account_ids.is_empty() {
        return Ok(HashMap::new());
    }
    #[derive(Deserialize)]
    struct Profile {
        account_id: u64,
        #[serde(default)]
        personaname: Option<String>,
    }
    let ids: Vec<String> = account_ids.iter().map(u64::to_string).collect();
    let url = format!("{API_URL}/players/steam?account_ids={}", ids.join(","));
    let Some(text) = fetch(get(http, api_key, &url), "nomes da Steam").await? else {
        return Ok(HashMap::new());
    };
    let profiles: Vec<Profile> =
        serde_json::from_str(&text).context("nomes da Steam em formato inesperado")?;
    Ok(profiles
        .into_iter()
        .filter_map(|p| {
            p.personaname
                .filter(|name| !name.trim().is_empty())
                .map(|name| (p.account_id, name))
        })
        .collect())
}

/// A hero, as a scoreboard shows it.
#[derive(Clone, Debug)]
pub struct CatalogHero {
    pub name: String,
    pub image: Option<String>,
}

/// A shop item (an "upgrade"), as a scoreboard shows it.
#[derive(Clone, Debug)]
pub struct CatalogItem {
    pub name: String,
    /// `weapon`, `spirit` or `vitality`.
    pub slot: String,
    pub tier: Option<i64>,
    pub active: bool,
    pub image: Option<String>,
}

/// Heroes and shop items by ID, from the API's assets.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub heroes: HashMap<u64, CatalogHero>,
    pub items: HashMap<u64, CatalogItem>,
}

pub async fn catalog(http: &Client, api_key: Option<&str>) -> Result<Catalog> {
    #[derive(Deserialize, Default)]
    struct HeroImages {
        icon_image_small_webp: Option<String>,
        icon_image_small: Option<String>,
    }
    #[derive(Deserialize)]
    struct Hero {
        id: u64,
        name: String,
        #[serde(default)]
        images: HeroImages,
    }
    #[derive(Deserialize)]
    struct Item {
        id: u64,
        name: String,
        #[serde(rename = "type")]
        kind: String,
        #[serde(default)]
        item_slot_type: Option<String>,
        #[serde(default)]
        item_tier: Option<i64>,
        #[serde(default)]
        is_active_item: bool,
        #[serde(default)]
        shop_image_webp: Option<String>,
        #[serde(default)]
        shop_image: Option<String>,
    }
    let heroes = fetch(get(http, api_key, &format!("{API_URL}/assets/heroes")), "heróis")
        .await?
        .ok_or_else(|| anyhow!("a API do Deadlock não tem a lista de heróis"))?;
    let items = fetch(get(http, api_key, &format!("{API_URL}/assets/items")), "itens")
        .await?
        .ok_or_else(|| anyhow!("a API do Deadlock não tem a lista de itens"))?;
    let heroes: Vec<Hero> =
        serde_json::from_str(&heroes).context("lista de heróis em formato inesperado")?;
    let items: Vec<Item> = serde_json::from_str(&items).context("lista de itens em formato inesperado")?;
    Ok(Catalog {
        heroes: heroes
            .into_iter()
            .map(|h| {
                let image = h.images.icon_image_small_webp.or(h.images.icon_image_small);
                (h.id, CatalogHero { name: h.name, image })
            })
            .collect(),
        items: items
            .into_iter()
            .filter(|i| i.kind == "upgrade")
            .map(|i| {
                let item = CatalogItem {
                    name: i.name,
                    slot: i.item_slot_type.unwrap_or_else(|| "weapon".to_string()),
                    tier: i.item_tier,
                    active: i.is_active_item,
                    image: i.shop_image_webp.or(i.shop_image),
                };
                (i.id, item)
            })
            .collect(),
    })
}
