use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::response::Html;
use axum::routing::{get, post};
use axum::Router;
use refresher_core::config::WebConfig;
use refresher_core::link::BotLink;
use sqlx::SqlitePool;
use tower_http::services::{ServeDir, ServeFile};

mod api;
mod auth;
#[cfg(feature = "demo")]
pub mod demo;
mod error;
mod session;

use error::ApiError;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<WebConfig>,
    pub http: reqwest::Client,
    pub link: Arc<dyn BotLink>,
}

pub fn router(state: AppState) -> Router {
    let static_dir = PathBuf::from(&state.config.static_dir);
    let api = Router::new()
        .route("/me", get(api::me))
        .route("/guilds/{guild_id}/camps", get(api::guild_camps).post(api::create_camp))
        .route("/guilds/{guild_id}/channels", get(api::guild_channels))
        .route("/guilds/{guild_id}/members", get(api::search_members))
        .route("/camps/{camp_id}", get(api::camp))
        .route("/camps/{camp_id}/actions", post(api::act))
        .route("/matches/{match_id}", get(api::match_info))
        .fallback(|| async { ApiError::NotFound });
    let app = Router::new()
        .nest("/api", api)
        .route("/auth/login", get(auth::login))
        .route("/auth/callback", get(auth::callback))
        .route("/auth/logout", post(auth::logout))
        .with_state(state);
    if static_dir.join("index.html").is_file() {
        // Paths the server doesn't know get index.html, so the page can have its own routes.
        app.fallback_service(
            ServeDir::new(&static_dir).fallback(ServeFile::new(static_dir.join("index.html"))),
        )
    } else {
        app.route("/", get(placeholder_page))
    }
}

/// Serves the admin web page and its API until the server stops.
pub async fn run(config: WebConfig, db: SqlitePool, link: Arc<dyn BotLink>) -> Result<()> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let listener = tokio::net::TcpListener::bind(&config.bind)
        .await
        .with_context(|| format!("could not listen on {}", config.bind))?;
    tracing::info!("web admin on {} (open {})", config.bind, config.public_url);
    if !PathBuf::from(&config.static_dir).join("index.html").is_file() {
        tracing::warn!(
            "no admin page in {}: run `npm install` and `npm run build` in web/",
            config.static_dir
        );
    }
    let state = AppState {
        db,
        config: Arc::new(config),
        http,
        link,
    };
    axum::serve(listener, router(state)).await?;
    Ok(())
}

/// Shown when the React page hasn't been built yet.
async fn placeholder_page() -> Html<&'static str> {
    Html(
        r##"<!doctype html>
<html lang="pt-BR">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Refresher Admin</title></head>
<body style="margin: 0; padding: 40px 24px; font-family: system-ui, sans-serif; background: #0A1615; color: #E4F2EF;">
<h1>Refresher Admin</h1>
<p>A página ainda não foi compilada. Rode <code>npm install</code> e <code>npm run build</code> na pasta <code>web/</code>.</p>
<p><a href="/auth/login" style="color: #8CF5D2;">Entrar com Discord</a> · <a href="/api/me" style="color: #8CF5D2;">Ver minha sessão</a></p>
<form method="post" action="/auth/logout"><button type="submit">Sair</button></form>
</body>
</html>"##,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use refresher_core::link::{BoxFuture, CampAction, Profile, TextChannel};
    use refresher_core::queue::CampSize;
    use sqlx::sqlite::SqlitePoolOptions;
    use tower::ServiceExt;

    use super::*;
    use crate::session::{AdminGuild, Session};

    #[derive(Default)]
    struct FakeLink {
        guilds: Vec<u64>,
        names: HashMap<u64, String>,
        /// What `act` was asked to do: (server, camp, action).
        actions: Mutex<Vec<(u64, String, CampAction)>>,
        /// When set, `act` fails with this message, like the bot refusing an action.
        refuse: Option<String>,
        /// The test database, so `create_camp` can create one for real.
        db: OnceLock<SqlitePool>,
    }

    fn profile(name: &str) -> Profile {
        Profile {
            name: name.to_string(),
            avatar_url: String::new(),
        }
    }

    impl BotLink for FakeLink {
        fn in_guild(&self, guild_id: u64) -> bool {
            self.guilds.contains(&guild_id)
        }

        fn profiles<'a>(
            &'a self,
            _guild_id: u64,
            user_ids: &'a [u64],
        ) -> BoxFuture<'a, HashMap<u64, Profile>> {
            Box::pin(async move {
                user_ids
                    .iter()
                    .filter_map(|id| self.names.get(id).map(|name| (*id, profile(name))))
                    .collect()
            })
        }

        fn text_channels(&self, guild_id: u64) -> Vec<TextChannel> {
            vec![TextChannel {
                id: guild_id * 10,
                name: "camp-deadlock".to_string(),
            }]
        }

        fn search_members<'a>(
            &'a self,
            _guild_id: u64,
            query: &'a str,
            _limit: u64,
        ) -> BoxFuture<'a, anyhow::Result<Vec<(u64, Profile)>>> {
            Box::pin(async move {
                let mut found: Vec<(u64, Profile)> = self
                    .names
                    .iter()
                    .filter(|(_, name)| name.starts_with(query))
                    .map(|(id, name)| (*id, profile(name)))
                    .collect();
                found.sort_by_key(|(id, _)| *id);
                Ok(found)
            })
        }

        fn queue_voice_members(&self, _guild_id: u64) -> Vec<u64> {
            vec![5]
        }

        fn create_camp<'a>(
            &'a self,
            guild_id: u64,
            channel_id: u64,
            size: CampSize,
        ) -> BoxFuture<'a, anyhow::Result<String>> {
            Box::pin(async move {
                let db = self.db.get().expect("test database");
                let camp = refresher_core::db::create_camp(
                    db,
                    guild_id as i64,
                    channel_id as i64,
                    size.target_size(),
                    size.match_size(),
                )
                .await?;
                Ok(camp.id)
            })
        }

        fn act<'a>(
            &'a self,
            guild_id: u64,
            camp_id: &'a str,
            action: CampAction,
        ) -> BoxFuture<'a, anyhow::Result<()>> {
            Box::pin(async move {
                self.actions
                    .lock()
                    .unwrap()
                    .push((guild_id, camp_id.to_string(), action));
                match &self.refuse {
                    Some(message) => Err(anyhow::anyhow!(message.clone())),
                    None => Ok(()),
                }
            })
        }
    }

    async fn test_state(link: FakeLink) -> AppState {
        test_state_with(Arc::new(link)).await
    }

    async fn test_state_with(link: Arc<FakeLink>) -> AppState {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        refresher_core::db::migrate(&db).await.unwrap();
        let _ = link.db.set(db.clone());
        AppState {
            db,
            config: Arc::new(WebConfig {
                bind: "127.0.0.1:0".to_string(),
                public_url: "http://localhost:8080".to_string(),
                discord_client_id: "1".to_string(),
                discord_client_secret: "secret".to_string(),
                static_dir: "does-not-exist".to_string(),
            }),
            http: reqwest::Client::new(),
            link,
        }
    }

    async fn post(state: &AppState, path: &str, token: Option<&str>, body: &str) -> (StatusCode, serde_json::Value) {
        let mut request = Request::post(path).header("content-type", "application/json");
        if let Some(token) = token {
            request = request.header("cookie", format!("{}={token}", session::COOKIE_NAME));
        }
        let response = router(state.clone())
            .oneshot(request.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null))
    }

    async fn sign_in(state: &AppState, admin_of: &[u64]) -> String {
        session::create(
            &state.db,
            &Session {
                user_id: 42,
                display_name: "admin".to_string(),
                avatar: None,
                admin_guilds: admin_of
                    .iter()
                    .map(|id| AdminGuild {
                        id: *id,
                        name: format!("server {id}"),
                        icon: None,
                    })
                    .collect(),
            },
        )
        .await
        .unwrap()
    }

    async fn get(state: &AppState, path: &str, token: Option<&str>) -> (StatusCode, serde_json::Value) {
        let mut request = Request::get(path);
        if let Some(token) = token {
            request = request.header("cookie", format!("{}={token}", session::COOKIE_NAME));
        }
        let response = router(state.clone())
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null))
    }

    #[tokio::test]
    async fn api_needs_a_session() {
        let state = test_state(FakeLink::default()).await;
        assert_eq!(get(&state, "/api/me", None).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(get(&state, "/api/me", Some("not-a-token")).await.0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn unknown_api_paths_are_json_404s() {
        let state = test_state(FakeLink::default()).await;
        let (status, body) = get(&state, "/api/nope", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "não encontrado");
    }

    #[tokio::test]
    async fn expired_session_is_rejected() {
        let state = test_state(FakeLink::default()).await;
        let token = sign_in(&state, &[111]).await;
        sqlx::query("UPDATE web_sessions SET expires_at = 0")
            .execute(&state.db)
            .await
            .unwrap();
        assert_eq!(get(&state, "/api/me", Some(&token)).await.0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn me_lists_managed_servers_the_bot_is_in() {
        let state = test_state(FakeLink {
            guilds: vec![111],
            ..FakeLink::default()
        })
        .await;
        let token = sign_in(&state, &[111, 222]).await;
        let (status, body) = get(&state, "/api/me", Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["user"]["id"], "42");
        let ids: Vec<&str> = body["guilds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["111"]);
    }

    #[tokio::test]
    async fn admins_only_see_camps_of_their_servers() {
        let state = test_state(FakeLink {
            guilds: vec![111, 222],
            names: HashMap::from([(5, "kaze".to_string())]),
            ..FakeLink::default()
        })
        .await;
        let mine = refresher_core::db::create_camp(&state.db, 111, 900, 24, 12).await.unwrap();
        let other = refresher_core::db::create_camp(&state.db, 222, 901, 24, 12).await.unwrap();
        refresher_core::db::add_players(&state.db, &mine.id, &[5, 6]).await.unwrap();
        let token = sign_in(&state, &[111]).await;

        let (status, body) = get(&state, "/api/guilds/111/camps", Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body[0]["id"], mine.id.as_str());
        assert_eq!(body[0]["channel_id"], "900");

        let (status, body) = get(&state, &format!("/api/camps/{}", mine.id.to_lowercase()), Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["guild_id"], "111");
        assert_eq!(body["status"], "signup");
        assert_eq!(body["refresh_votes"], 4);
        assert_eq!(body["members"]["5"]["name"], "kaze");
        assert!(body["members"].get("6").is_none());

        assert_eq!(get(&state, "/api/guilds/222/camps", Some(&token)).await.0, StatusCode::NOT_FOUND);
        assert_eq!(
            get(&state, &format!("/api/camps/{}", other.id), Some(&token)).await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(get(&state, "/api/matches/ZZZZZZ", Some(&token)).await.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn actions_run_on_the_bot_and_return_the_camp() {
        let link = Arc::new(FakeLink::default());
        let state = test_state_with(link.clone()).await;
        let camp = refresher_core::db::create_camp(&state.db, 111, 900, 24, 12).await.unwrap();
        let token = sign_in(&state, &[111]).await;

        let path = format!("/api/camps/{}/actions", camp.id.to_lowercase());
        let body = r#"{"type": "report_winner", "match_id": "R4T8WN", "side": "archmother"}"#;
        let (status, response) = post(&state, &path, Some(&token), body).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response["id"], camp.id.as_str());

        let (status, _) = post(&state, &path, Some(&token), r#"{"type": "create_match"}"#).await;
        assert_eq!(status, StatusCode::OK);

        let actions = link.actions.lock().unwrap();
        assert_eq!(
            *actions,
            [
                (
                    111,
                    camp.id.clone(),
                    CampAction::ReportWinner {
                        match_id: "R4T8WN".to_string(),
                        side: "archmother".to_string()
                    }
                ),
                (111, camp.id.clone(), CampAction::CreateMatch),
            ]
        );
    }

    #[tokio::test]
    async fn actions_are_limited_to_managed_servers() {
        let link = Arc::new(FakeLink::default());
        let state = test_state_with(link.clone()).await;
        let other = refresher_core::db::create_camp(&state.db, 222, 901, 24, 12).await.unwrap();
        let token = sign_in(&state, &[111]).await;
        let path = format!("/api/camps/{}/actions", other.id);

        assert_eq!(post(&state, &path, None, r#"{"type": "create_match"}"#).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(
            post(&state, &path, Some(&token), r#"{"type": "create_match"}"#).await.0,
            StatusCode::NOT_FOUND
        );
        assert!(link.actions.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn bad_actions_never_reach_the_bot() {
        let link = Arc::new(FakeLink::default());
        let state = test_state_with(link.clone()).await;
        let camp = refresher_core::db::create_camp(&state.db, 111, 900, 24, 12).await.unwrap();
        let token = sign_in(&state, &[111]).await;
        let path = format!("/api/camps/{}/actions", camp.id);

        let bad_side = r#"{"type": "report_winner", "match_id": "R4T8WN", "side": "everyone"}"#;
        assert!(post(&state, &path, Some(&token), bad_side).await.0.is_client_error());
        assert!(post(&state, &path, Some(&token), r#"{"type": "delete_everything"}"#)
            .await
            .0
            .is_client_error());
        assert!(link.actions.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn refused_actions_come_back_as_conflicts() {
        let state = test_state(FakeLink {
            refuse: Some("Espere a votação de Refresh terminar antes do draft.".to_string()),
            ..FakeLink::default()
        })
        .await;
        let camp = refresher_core::db::create_camp(&state.db, 111, 900, 24, 12).await.unwrap();
        let token = sign_in(&state, &[111]).await;
        let (status, body) = post(
            &state,
            &format!("/api/camps/{}/actions", camp.id),
            Some(&token),
            r#"{"type": "start_draft", "match_id": "R4T8WN"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"], "Espere a votação de Refresh terminar antes do draft.");
    }

    #[tokio::test]
    async fn camps_are_created_in_managed_servers_only() {
        let state = test_state(FakeLink {
            guilds: vec![111, 222],
            ..FakeLink::default()
        })
        .await;
        let token = sign_in(&state, &[111]).await;

        let (status, body) = post(
            &state,
            "/api/guilds/111/camps",
            Some(&token),
            r#"{"channel_id": "1110", "size": "30"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["status"], "signup");
        assert_eq!(body["target_size"], 30);
        assert_eq!(body["match_size"], 12);
        assert_eq!(body["channel_id"], "1110");
        assert_eq!(body["queue_voice"], serde_json::json!(["5"]));

        let bad_size = r#"{"channel_id": "1110", "size": "7"}"#;
        assert_eq!(post(&state, "/api/guilds/111/camps", Some(&token), bad_size).await.0, StatusCode::BAD_REQUEST);
        let other = r#"{"channel_id": "2220", "size": "24"}"#;
        assert_eq!(post(&state, "/api/guilds/222/camps", Some(&token), other).await.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn channels_and_member_search_need_a_managed_server() {
        let state = test_state(FakeLink {
            guilds: vec![111, 222],
            names: HashMap::from([(5, "kaze".to_string()), (6, "kami".to_string()), (7, "zeca".to_string())]),
            ..FakeLink::default()
        })
        .await;
        let token = sign_in(&state, &[111]).await;

        let (status, body) = get(&state, "/api/guilds/111/channels", Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body[0]["id"], "1110");

        let (status, body) = get(&state, "/api/guilds/111/members?query=ka", Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
        let names: Vec<&str> = body.as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["kaze", "kami"]);
        assert_eq!(body[0]["id"], "5");

        assert_eq!(get(&state, "/api/guilds/222/channels", Some(&token)).await.0, StatusCode::NOT_FOUND);
        assert_eq!(get(&state, "/api/guilds/222/members?query=ka", Some(&token)).await.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn roster_actions_carry_user_ids() {
        let link = Arc::new(FakeLink::default());
        let state = test_state_with(link.clone()).await;
        let camp = refresher_core::db::create_camp(&state.db, 111, 900, 4, 2).await.unwrap();
        let token = sign_in(&state, &[111]).await;
        let path = format!("/api/camps/{}/actions", camp.id);

        let add = r#"{"type": "add_players", "user_ids": ["307688453406064641", "5"]}"#;
        assert_eq!(post(&state, &path, Some(&token), add).await.0, StatusCode::OK);
        let s_rank = r#"{"type": "set_s_rank", "user_id": "5", "s_rank": true}"#;
        assert_eq!(post(&state, &path, Some(&token), s_rank).await.0, StatusCode::OK);
        let bad = r#"{"type": "remove_player", "user_id": "not-an-id"}"#;
        assert_eq!(post(&state, &path, Some(&token), bad).await.0, StatusCode::BAD_REQUEST);

        let actions = link.actions.lock().unwrap();
        assert_eq!(
            actions.iter().map(|a| a.2.clone()).collect::<Vec<_>>(),
            [
                CampAction::AddPlayers {
                    user_ids: vec![307688453406064641, 5]
                },
                CampAction::SetSRank { user_id: 5, s_rank: true },
            ]
        );
    }

    #[tokio::test]
    async fn login_sends_the_browser_to_discord() {
        let state = test_state(FakeLink::default()).await;
        let response = router(state)
            .oneshot(Request::get("/auth/login").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = response.headers()["location"].to_str().unwrap();
        assert!(location.starts_with("https://discord.com/oauth2/authorize?client_id=1&"));
        assert!(location.contains("redirect_uri=http%3A%2F%2Flocalhost%3A8080%2Fauth%2Fcallback"));
        assert!(location.contains("scope=identify+guilds"));
        let cookie = response.headers()["set-cookie"].to_str().unwrap();
        assert!(cookie.starts_with("refresher_oauth_state="));
        assert!(cookie.contains("HttpOnly"));
    }

    #[tokio::test]
    async fn callback_rejects_a_forged_state() {
        let state = test_state(FakeLink::default()).await;
        let response = router(state)
            .oneshot(
                Request::get("/auth/callback?code=abc&state=forged")
                    .header("cookie", "refresher_oauth_state=real")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn manage_server_rule_matches_the_bot() {
        assert!(auth::can_manage(true, "0"));
        assert!(auth::can_manage(false, "32"));
        assert!(auth::can_manage(false, "8"));
        assert!(!auth::can_manage(false, "2048"));
        assert!(!auth::can_manage(false, "not a number"));
    }
}
