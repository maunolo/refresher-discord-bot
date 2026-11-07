use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde::Deserialize;

/// DeadChaps' draft API (https://www.deadchaps.gg/api). Needs a key with the `drafts.write` scope.
const CREATE_URL: &str = "https://www.deadchaps.gg/api/v1/drafts";

/// Where DeadChaps draft pages live: a draft's watch link is this plus its code.
const DRAFT_PAGE: &str = "https://www.deadchaps.gg/draft/";

/// The draft's OBS overlay: its watch link with `/obs`. `None` for drafts made before DeadChaps.
pub fn obs_url(spectator_url: &str) -> Option<String> {
    spectator_url
        .starts_with(DRAFT_PAGE)
        .then(|| format!("{}/obs", spectator_url.trim_end_matches('/')))
}

/// Links for one draft. The join link is private: whoever opens it can take a slot on either
/// side, so only captains get it.
#[derive(Clone, Debug)]
pub struct CreatedDraft {
    /// Watch link, open to anyone.
    pub spectator_url: String,
    pub join_url: String,
}

#[derive(Debug, Deserialize)]
struct DraftCreated {
    url: String,
    join_url: String,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    detail: ErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

/// The draft request. Hiddenking is DeadChaps' `team0`, with 2 bans per team and one pick per
/// player. DeadChaps has no 1v1 lobby, so a 1v1 test match drafts as 2v2. `timer_seconds` is
/// per pick or ban, one of `settings::DRAFT_TIMERS`; 0 (no timer) becomes the longest
/// DeadChaps allows.
fn create_body(match_size: usize, timer_seconds: u32) -> serde_json::Value {
    let per_side = (match_size / 2).clamp(2, 6);
    let mut body = serde_json::json!({
        "team0": { "name": "Hiddenking" },
        "team1": { "name": "Archmother" },
        "lobby_size": format!("{per_side}v{per_side}"),
        "bans_per_team": 2,
    });
    if timer_seconds == 0 {
        body["time_per_pick"] = 300.into();
        body["reserve_time"] = 600.into();
    } else {
        body["time_per_pick"] = timer_seconds.into();
    }
    body
}

/// Creates a private draft on DeadChaps.
pub async fn create_draft(
    http: &Client,
    api_key: Option<&str>,
    match_size: usize,
    timer_seconds: u32,
) -> Result<CreatedDraft> {
    let Some(api_key) = api_key else {
        bail!("DEADCHAPS_API_KEY não está configurada. Configure a chave ou desligue o draft automático em Configurações.");
    };
    let response = http
        .post(CREATE_URL)
        .header("X-API-Key", api_key)
        .json(&create_body(match_size, timer_seconds))
        .send()
        .await
        .context("falha ao chamar a API de draft do DeadChaps")?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(match serde_json::from_str::<ErrorBody>(&text) {
            Ok(error) => anyhow!(
                "O DeadChaps recusou o draft ({status}, {}): {}",
                error.detail.code,
                error.detail.message
            ),
            Err(_) => anyhow!("O DeadChaps recusou o draft ({status}): {text}"),
        });
    }
    let created: DraftCreated = serde_json::from_str(&text)
        .with_context(|| format!("resposta inesperada da API de draft do DeadChaps: {text}"))?;
    Ok(CreatedDraft {
        spectator_url: created.url,
        join_url: created.join_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lobby_size_follows_the_match_and_starts_at_2v2() {
        for (match_size, lobby) in [(2_usize, "2v2"), (4, "2v2"), (12, "6v6")] {
            assert_eq!(create_body(match_size, 30)["lobby_size"], lobby, "{match_size}");
        }
    }

    #[test]
    fn obs_overlay_only_for_deadchaps_drafts() {
        assert_eq!(
            obs_url("https://www.deadchaps.gg/draft/3GQCZG98").as_deref(),
            Some("https://www.deadchaps.gg/draft/3GQCZG98/obs")
        );
        assert_eq!(obs_url("https://draft.deadlock.pro.br/abc"), None);
    }

    #[test]
    fn no_timer_becomes_the_longest_deadchaps_allows() {
        let body = create_body(12, 0);
        assert_eq!(body["time_per_pick"], 300);
        assert_eq!(body["reserve_time"], 600);
        let body = create_body(12, 45);
        assert_eq!(body["time_per_pick"], 45);
        assert!(body.get("reserve_time").is_none(), "keeps DeadChaps' default reserve");
    }
}
