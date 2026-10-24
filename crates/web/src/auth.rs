use anyhow::{bail, Context};
use axum::extract::{FromRequestParts, Query, State};
use axum::http::request::Parts;
use axum::response::Redirect;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::error::ApiError;
use crate::session::{self, AdminGuild, Session};
use crate::AppState;

const AUTHORIZE_URL: &str = "https://discord.com/oauth2/authorize";
// Unversioned API URLs fall back to an old version where `permissions` is a number, not a string.
const TOKEN_URL: &str = "https://discord.com/api/v10/oauth2/token";
const API_URL: &str = "https://discord.com/api/v10";
const STATE_COOKIE: &str = "refresher_oauth_state";

const ADMINISTRATOR: u64 = 1 << 3;
const MANAGE_GUILD: u64 = 1 << 5;

/// The same rule as the bot's commands: Manage Server. Owners and administrators have it too.
pub fn can_manage(owner: bool, permissions: &str) -> bool {
    let bits = permissions.parse::<u64>().unwrap_or(0);
    owner || bits & (ADMINISTRATOR | MANAGE_GUILD) != 0
}

/// The signed-in admin. Handlers that take this answer 401 without a valid session.
pub struct Admin(pub Session);

impl FromRequestParts<AppState> for Admin {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(session::COOKIE_NAME).ok_or(ApiError::Unauthorized)?;
        session::load(&state.db, token.value())
            .await?
            .map(Admin)
            .ok_or(ApiError::Unauthorized)
    }
}

fn redirect_uri(state: &AppState) -> String {
    format!("{}/auth/callback", state.config.public_url)
}

fn cookie(state: &AppState, name: &'static str, value: String) -> Cookie<'static> {
    Cookie::build((name, value))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(state.config.public_url.starts_with("https://"))
        .build()
}

/// Sends the browser to Discord's consent page.
pub async fn login(State(state): State<AppState>, jar: CookieJar) -> Result<(CookieJar, Redirect), ApiError> {
    let csrf = session::random_token();
    let redirect = redirect_uri(&state);
    let url = reqwest::Url::parse_with_params(
        AUTHORIZE_URL,
        [
            ("client_id", state.config.discord_client_id.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("response_type", "code"),
            ("scope", "identify guilds"),
            ("state", csrf.as_str()),
            ("prompt", "none"),
        ],
    )?;
    Ok((jar.add(cookie(&state, STATE_COOKIE, csrf)), Redirect::to(url.as_str())))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// Discord sends the browser back here with a one-time code after the admin agrees.
pub async fn callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> Result<(CookieJar, Redirect), ApiError> {
    let expected = jar.get(STATE_COOKIE).map(|c| c.value().to_string());
    let jar = jar.remove(Cookie::build(STATE_COOKIE).path("/"));
    if let Some(error) = query.error {
        return Err(ApiError::BadRequest(format!("o Discord não liberou o login: {error}")));
    }
    let (Some(code), Some(returned)) = (query.code, query.state) else {
        return Err(ApiError::BadRequest("resposta do Discord incompleta".to_string()));
    };
    if expected.as_deref() != Some(returned.as_str()) {
        return Err(ApiError::BadRequest("o login expirou, tente de novo".to_string()));
    }

    let access_token = exchange_code(&state, &code).await?;
    let user: DiscordUser = discord_get(&state, "/users/@me", &access_token).await?;
    let guilds: Vec<DiscordGuild> = discord_get(&state, "/users/@me/guilds", &access_token).await?;
    let session = Session {
        user_id: user.id.parse().context("ID de usuário inválido")?,
        display_name: user.global_name.unwrap_or(user.username),
        avatar: user.avatar,
        admin_guilds: guilds
            .into_iter()
            .filter(|g| can_manage(g.owner, &g.permissions))
            .filter_map(|g| {
                Some(AdminGuild {
                    id: g.id.parse().ok()?,
                    name: g.name,
                    icon: g.icon,
                })
            })
            .collect(),
    };
    let token = session::create(&state.db, &session).await?;
    tracing::info!(user_id = session.user_id, servers = session.admin_guilds.len(), "admin signed in");
    Ok((jar.add(cookie(&state, session::COOKIE_NAME, token)), Redirect::to("/")))
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> Result<(CookieJar, Redirect), ApiError> {
    if let Some(token) = jar.get(session::COOKIE_NAME) {
        session::delete(&state.db, token.value()).await?;
    }
    Ok((jar.remove(Cookie::build(session::COOKIE_NAME).path("/")), Redirect::to("/")))
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
}

#[derive(Deserialize)]
struct DiscordGuild {
    id: String,
    name: String,
    icon: Option<String>,
    owner: bool,
    permissions: String,
}

async fn exchange_code(state: &AppState, code: &str) -> anyhow::Result<String> {
    let redirect = redirect_uri(state);
    let response = state
        .http
        .post(TOKEN_URL)
        .basic_auth(&state.config.discord_client_id, Some(&state.config.discord_client_secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect.as_str()),
        ])
        .send()
        .await
        .context("falha ao falar com o Discord")?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("o Discord recusou o código de login ({status}): {text}");
    }
    Ok(serde_json::from_str::<TokenResponse>(&text)?.access_token)
}

async fn discord_get<T: DeserializeOwned>(state: &AppState, path: &str, access_token: &str) -> anyhow::Result<T> {
    let response = state
        .http
        .get(format!("{API_URL}{path}"))
        .bearer_auth(access_token)
        .send()
        .await
        .context("falha ao falar com o Discord")?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("o Discord recusou {path} ({status}): {text}");
    }
    serde_json::from_str(&text).with_context(|| format!("resposta inesperada do Discord em {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_v10_server_list() {
        let body = r#"[
            {"id": "1230000000000000001", "name": "Xatuba Nation", "icon": null, "banner": null,
             "owner": false, "permissions": "2251799813685247", "features": []},
            {"id": "1230000000000000002", "name": "Outro", "icon": "a_abc", "banner": null,
             "owner": false, "permissions": "104324673", "features": ["COMMUNITY"]}
        ]"#;
        let guilds: Vec<DiscordGuild> = serde_json::from_str(body).unwrap();
        assert_eq!(guilds.len(), 2);
        // All permissions vs. an ordinary member's set without Manage Server.
        assert!(can_manage(guilds[0].owner, &guilds[0].permissions));
        assert!(!can_manage(guilds[1].owner, &guilds[1].permissions));
    }
}
