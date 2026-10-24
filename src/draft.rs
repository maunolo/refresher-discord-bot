use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;

pub const DEFAULT_BASE_URL: &str = "https://draft.deadlock.pro.br";

/// Links for one draft. Team links are private: whoever opens one picks and bans for that team.
#[derive(Clone, Debug)]
pub struct CreatedDraft {
    pub spectator_url: String,
    pub hiddenking_url: String,
    pub archmother_url: String,
    pub admin_url: String,
}

#[derive(Debug, Deserialize)]
struct CreateDraftResponse {
    data: DraftCodes,
}

#[derive(Debug, Deserialize)]
struct DraftCodes {
    code_url: String,
    code_a: String,
    code_b: String,
    code_admin: String,
}

/// Pick/ban order for the draft tool: `a#` is one Team A ban, `b2` is two Team B picks in a row.
/// Every size gives each team one pick per player.
fn draft_order(match_size: usize) -> &'static str {
    match match_size {
        2 => "a#-b#-a1-b1",
        4 => "a#-b#-a1-b2-a1",
        _ => "a#-b#-a1-b2-a2-b1-b#-a#-b1-a2-b2-a1",
    }
}

/// Creates a draft on draft.deadlock.pro.br (or a self-hosted copy). Team A is Hiddenking.
pub async fn create_draft(http: &Client, base_url: &str, match_size: usize) -> Result<CreatedDraft> {
    let base = base_url.trim_end_matches('/');
    let body = serde_json::json!({
        "name_a": "Hiddenking",
        "name_b": "Archmother",
        "sort": draft_order(match_size),
        "timer_seconds": 30
    });
    let response = http
        .post(format!("{base}/api/draft"))
        .json(&body)
        .send()
        .await
        .context("falha ao chamar a API de draft")?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("A API de draft recusou a criação ({status}): {text}"));
    }
    let parsed: CreateDraftResponse = serde_json::from_str(&text)
        .with_context(|| format!("resposta inesperada da API de draft: {text}"))?;
    let codes = parsed.data;
    Ok(CreatedDraft {
        spectator_url: format!("{base}/{}", codes.code_url),
        hiddenking_url: format!("{base}/{}/{}", codes.code_url, codes.code_a),
        archmother_url: format!("{base}/{}/{}", codes.code_url, codes.code_b),
        admin_url: format!("{base}/links/{}", codes.code_admin),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_order_gives_one_pick_per_player() {
        for match_size in [2_usize, 4, 12] {
            let order = draft_order(match_size);
            let mut picks = [0_usize; 2];
            for block in order.split('-') {
                let (team, count) = block.split_at(1);
                assert!(team == "a" || team == "b", "bad team in {order}");
                if count != "#" {
                    let n: usize = count.parse().expect("pick count");
                    assert!((1..=9).contains(&n));
                    picks[usize::from(team == "b")] += n;
                }
            }
            assert_eq!(picks, [match_size / 2, match_size / 2], "{order}");
        }
    }
}
