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

/// How DeadChaps lays out the draft page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftView {
    /// DeadChaps' default.
    Cinematic,
    Competitive,
}

impl DraftView {
    pub const ALL: [DraftView; 2] = [DraftView::Cinematic, DraftView::Competitive];

    /// `cinematic` or `competitive`, as DeadChaps and the admin page name them.
    pub fn as_str(self) -> &'static str {
        match self {
            DraftView::Cinematic => "cinematic",
            DraftView::Competitive => "competitive",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        DraftView::ALL.into_iter().find(|view| view.as_str() == name)
    }
}

/// A draft's rules, from the server's settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DraftOptions {
    /// Seconds per pick or ban, one of `settings::DRAFT_TIMERS`. 0 is no timer: the longest
    /// turn and reserve DeadChaps allows.
    pub timer_seconds: u32,
    /// Extra seconds each side can spend over its turns, one of `settings::DRAFT_RESERVES`.
    pub reserve_seconds: u32,
    /// 0 turns bans off; at most `settings::MAX_DRAFT_BANS`.
    pub bans_per_team: u32,
    pub view: DraftView,
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

/// The draft request: private, Hiddenking as DeadChaps' `team0`, one pick per player. DeadChaps
/// has no 1v1 lobby, so a 1v1 test match drafts as 2v2. No timer becomes the longest turn and
/// reserve DeadChaps allows, whatever reserve the server picked.
fn create_body(match_size: usize, options: &DraftOptions) -> serde_json::Value {
    let per_side = (match_size / 2).clamp(2, 6);
    let (time_per_pick, reserve_time) = match options.timer_seconds {
        0 => (300, 600),
        seconds => (seconds, options.reserve_seconds),
    };
    serde_json::json!({
        "team0": { "name": "Hiddenking" },
        "team1": { "name": "Archmother" },
        "lobby_size": format!("{per_side}v{per_side}"),
        "bans_per_team": options.bans_per_team,
        "time_per_pick": time_per_pick,
        "reserve_time": reserve_time,
        "draft_view": options.view.as_str(),
        "private": true,
    })
}

/// Creates a private draft on DeadChaps.
pub async fn create_draft(
    http: &Client,
    api_key: Option<&str>,
    match_size: usize,
    options: &DraftOptions,
) -> Result<CreatedDraft> {
    let Some(api_key) = api_key else {
        bail!("DEADCHAPS_API_KEY não está configurada. Configure a chave ou desligue o draft automático em Configurações.");
    };
    let response = http
        .post(CREATE_URL)
        .header("X-API-Key", api_key)
        .json(&create_body(match_size, options))
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

    const DEFAULTS: DraftOptions = DraftOptions {
        timer_seconds: 30,
        reserve_seconds: 90,
        bans_per_team: 2,
        view: DraftView::Cinematic,
    };

    #[test]
    fn lobby_size_follows_the_match_and_starts_at_2v2() {
        for (match_size, lobby) in [(2_usize, "2v2"), (4, "2v2"), (12, "6v6")] {
            assert_eq!(create_body(match_size, &DEFAULTS)["lobby_size"], lobby, "{match_size}");
        }
    }

    #[test]
    fn the_servers_options_go_to_deadchaps() {
        let options = DraftOptions {
            timer_seconds: 45,
            reserve_seconds: 120,
            bans_per_team: 0,
            view: DraftView::Competitive,
        };
        let body = create_body(12, &options);
        assert_eq!(body["time_per_pick"], 45);
        assert_eq!(body["reserve_time"], 120);
        assert_eq!(body["bans_per_team"], 0, "no bans");
        assert_eq!(body["draft_view"], "competitive");
        assert_eq!(body["private"], true, "only captains get the invite link");
        assert_eq!(create_body(12, &DEFAULTS)["draft_view"], "cinematic");
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
        let options = DraftOptions { timer_seconds: 0, reserve_seconds: 30, ..DEFAULTS };
        let body = create_body(12, &options);
        assert_eq!(body["time_per_pick"], 300);
        assert_eq!(body["reserve_time"], 600, "whatever reserve was picked");
    }
}
