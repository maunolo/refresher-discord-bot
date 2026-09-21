use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;

const CREATE_URL: &str = "https://api.deadlock-api.com/v1/matches/custom/create";

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
        .context("failed to call Deadlock custom match API")?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!(
            "Deadlock API rejected custom match create ({status}): {text}"
        ));
    }

    let parsed: CreateCustomResponse = serde_json::from_str(&text)
        .with_context(|| format!("unexpected Deadlock API body: {text}"))?;
    Ok((parsed.party_id, parsed.party_code))
}
