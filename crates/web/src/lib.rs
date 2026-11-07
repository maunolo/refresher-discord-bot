use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::response::Html;
use axum::routing::{get, post};
use axum::Router;
use refresher_core::config::WebConfig;
use refresher_core::link::BotLink;
use refresher_core::settings::GuildSettings;
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
    /// Settings of a server that never saved its own: the `.env` values.
    pub default_settings: GuildSettings,
}

pub fn router(state: AppState) -> Router {
    let static_dir = PathBuf::from(&state.config.static_dir);
    let api = Router::new()
        .route("/me", get(api::me))
        .route("/guilds/{guild_id}/camps", get(api::guild_camps).post(api::create_camp))
        .route("/guilds/{guild_id}/channels", get(api::guild_channels))
        .route("/guilds/{guild_id}/members", get(api::guild_members))
        .route("/guilds/{guild_id}/settings", get(api::guild_settings).put(api::save_guild_settings))
        .route("/guilds/{guild_id}/events", get(api::guild_events))
        .route("/guilds/{guild_id}/overlays", get(api::guild_overlays).post(api::reset_guild_overlays))
        .route("/overlay/{token}/votes", get(api::overlay_votes))
        .route("/overlay/{token}/ranking", get(api::overlay_ranking))
        .route("/overlay/{token}/events", get(api::overlay_events))
        .route("/camps/{camp_id}", get(api::camp))
        .route("/camps/{camp_id}/matches", get(api::camp_matches))
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
        // OBS overlays are their own page; the secret and the overlay are read from the path.
        // Other paths the server doesn't know get index.html, so the page can have its own routes.
        app.route_service("/overlay/{*rest}", ServeFile::new(static_dir.join("overlay.html")))
            .fallback_service(
                ServeDir::new(&static_dir).fallback(ServeFile::new(static_dir.join("index.html"))),
            )
    } else {
        app.route("/", get(placeholder_page))
    }
}

/// Serves the admin web page and its API until the server stops.
pub async fn run(
    config: WebConfig,
    default_settings: GuildSettings,
    db: SqlitePool,
    link: Arc<dyn BotLink>,
) -> Result<()> {
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
        default_settings,
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
    use futures_util::StreamExt;
    use refresher_core::link::{BoxFuture, CampAction, Changes, Channel, Profile};
    use refresher_core::queue::{CampSize, Role};
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
        changes: Changes,
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

        fn changes(&self) -> &Changes {
            &self.changes
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

        fn text_channels(&self, guild_id: u64) -> Vec<Channel> {
            vec![Channel {
                id: guild_id * 10,
                name: "camp-deadlock".to_string(),
            }]
        }

        fn voice_channels(&self, guild_id: u64) -> Vec<Channel> {
            vec![Channel {
                id: guild_id * 10 + 1,
                name: "Queue".to_string(),
            }]
        }

        fn members(&self, _guild_id: u64) -> BoxFuture<'_, anyhow::Result<Vec<(u64, Profile)>>> {
            Box::pin(async move {
                let mut found: Vec<(u64, Profile)> =
                    self.names.iter().map(|(id, name)| (*id, profile(name))).collect();
                found.sort_by_key(|(id, _)| *id);
                Ok(found)
            })
        }

        fn queue_voice_members(&self, _guild_id: u64) -> BoxFuture<'_, Vec<u64>> {
            Box::pin(async { vec![5] })
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
            default_settings: GuildSettings {
                queue_voice_id: None,
                match1_hiddenking_voice_id: None,
                match1_archmother_voice_id: None,
                match2_hiddenking_voice_id: None,
                match2_archmother_voice_id: None,
                create_lobby: true,
                server_region: Some("SouthAmerica".to_string()),
                use_draft_tool: true,
                draft_timer_seconds: 30,
            },
        }
    }

    async fn post(state: &AppState, path: &str, token: Option<&str>, body: &str) -> (StatusCode, serde_json::Value) {
        send(state, Request::post(path), token, body).await
    }

    async fn put(state: &AppState, path: &str, token: Option<&str>, body: &str) -> (StatusCode, serde_json::Value) {
        send(state, Request::put(path), token, body).await
    }

    async fn send(
        state: &AppState,
        request: axum::http::request::Builder,
        token: Option<&str>,
        body: &str,
    ) -> (StatusCode, serde_json::Value) {
        let mut request = request.header("content-type", "application/json");
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
    async fn events_stream_changes_of_the_admins_server_only() {
        let link = Arc::new(FakeLink {
            guilds: vec![111, 222],
            ..FakeLink::default()
        });
        let state = test_state_with(link.clone()).await;
        let token = sign_in(&state, &[111]).await;
        let events = |guild_id: u64| {
            Request::get(format!("/api/guilds/{guild_id}/events"))
                .header("cookie", format!("{}={token}", session::COOKIE_NAME))
                .body(Body::empty())
                .unwrap()
        };
        let not_mine = router(state.clone()).oneshot(events(222)).await.unwrap();
        assert_eq!(not_mine.status(), StatusCode::NOT_FOUND);

        let response = router(state.clone()).oneshot(events(111)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "text/event-stream");
        let mut body = response.into_body().into_data_stream();
        link.changes.notify(222);
        link.changes.notify(111);
        let chunk = tokio::time::timeout(Duration::from_secs(5), body.next())
            .await
            .expect("a change event")
            .expect("an open stream")
            .unwrap();
        let text = String::from_utf8(chunk.to_vec()).unwrap();
        assert_eq!(text, "event: change
data: 111

", "the other server's change isn't sent");
    }

    #[tokio::test]
    async fn overlay_links_come_from_admins_and_can_be_replaced() {
        let state = test_state(FakeLink {
            guilds: vec![111, 222],
            ..FakeLink::default()
        })
        .await;
        let token = sign_in(&state, &[111]).await;
        assert_eq!(get(&state, "/api/guilds/111/overlays", None).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(get(&state, "/api/guilds/222/overlays", Some(&token)).await.0, StatusCode::NOT_FOUND);

        let (status, body) = get(&state, "/api/guilds/111/overlays", Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
        let url = body["vote_url"].as_str().unwrap().to_string();
        let secret = url
            .strip_prefix("http://localhost:8080/overlay/")
            .and_then(|rest| rest.strip_suffix("/votacao"))
            .unwrap()
            .to_string();
        assert_eq!(get(&state, "/api/guilds/111/overlays", Some(&token)).await.1["vote_url"], url);
        let (status, body) = get(&state, &format!("/api/overlay/{secret}/votes"), None).await;
        assert_eq!(status, StatusCode::OK, "no login needed");
        assert_eq!(body["votes"], serde_json::json!([]), "no camp yet");
        let events = |secret: &str| Request::get(format!("/api/overlay/{secret}/events")).body(Body::empty()).unwrap();
        let response = router(state.clone()).oneshot(events(&secret)).await.unwrap();
        assert_eq!(response.headers()["content-type"], "text/event-stream");

        let (status, body) = post(&state, "/api/guilds/111/overlays", Some(&token), "").await;
        assert_eq!(status, StatusCode::OK);
        assert_ne!(body["vote_url"], url);
        assert_eq!(
            get(&state, &format!("/api/overlay/{secret}/votes"), None).await.0,
            StatusCode::NOT_FOUND,
            "the old link stops working"
        );
        let response = router(state.clone()).oneshot(events(&secret)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn vote_overlay_shows_who_voted_but_not_how() {
        use refresher_core::db;
        let state = test_state(FakeLink {
            guilds: vec![111],
            names: HashMap::from([(1, "kaze".to_string()), (2, "kami".to_string())]),
            ..FakeLink::default()
        })
        .await;
        let pool = &state.db;
        let camp = db::create_camp(pool, 111, 900, 4, 4).await.unwrap();
        db::add_players(pool, &camp.id, &[1, 2, 3, 4]).await.unwrap();
        db::set_s_rank(pool, &camp.id, &[1]).await.unwrap();
        db::set_player_role(pool, &camp.id, 1, Some(Role::Tank)).await.unwrap();
        db::start_queue(pool, &camp.id).await.unwrap();
        let (_, match_id) = db::create_match(pool, &camp.id, 111).await.unwrap();
        db::vote_refresh_choice(pool, &match_id, 1, true).await.unwrap();
        let path = format!("/api/overlay/{}/votes", db::overlay_token(pool, 111).await.unwrap());

        let (status, body) = get(&state, &path, None).await;
        assert_eq!(status, StatusCode::OK);
        let vote = &body["votes"][0];
        assert_eq!(vote["match_id"], match_id);
        assert_eq!(vote["state"], "open");
        assert_eq!((vote["needed"].as_u64(), vote["voted"].as_u64(), vote["total"].as_u64()), (Some(2), Some(1), Some(4)));
        let teams = vote["teams"].as_array().unwrap();
        assert_eq!((teams[0]["side"].as_str(), teams[1]["side"].as_str()), (Some("hiddenking"), Some("archmother")));
        let players: Vec<&serde_json::Value> = teams.iter().flat_map(|t| t["players"].as_array().unwrap()).collect();
        assert_eq!(players.len(), 4);
        let kaze = players.iter().find(|p| p["user_id"] == "1").unwrap();
        assert_eq!((kaze["name"].as_str(), kaze["voted"].as_bool()), (Some("kaze"), Some(true)));
        assert_eq!((kaze["s_rank"].as_bool(), kaze["role"].as_str(), kaze["captain"].as_bool()), (Some(true), Some("tank"), Some(true)));
        assert_eq!(players.iter().filter(|p| p["voted"] == true).count(), 1);
        let kazes_team = teams.iter().find(|t| t["players"].as_array().unwrap().iter().any(|p| p["user_id"] == "1")).unwrap();
        assert_eq!(kazes_team["refresh"], 1, "the side's Refresh votes count");
        for player in &players {
            let mut keys: Vec<&String> = player.as_object().unwrap().keys().collect();
            keys.sort();
            assert_eq!(keys, ["avatar_url", "captain", "name", "role", "s_rank", "user_id", "voted"], "no vote choice");
        }
        let unnamed = players.iter().find(|p| p["user_id"] == "3").unwrap();
        assert_eq!(unnamed["name"], "3", "Discord couldn't find them");

        db::close_voting(pool, &camp.id, &match_id).await.unwrap();
        assert_eq!(get(&state, &path, None).await.1["votes"][0]["state"], "kept");
        db::set_manual_draft(pool, &match_id).await.unwrap();
        assert_eq!(get(&state, &path, None).await.1["votes"], serde_json::json!([]), "gone once the draft starts");
    }

    /// Plays the next match of a 1v1 test camp, Hiddenking winning. Returns (match, winner, loser).
    async fn play_1v1(pool: &SqlitePool, camp_id: &str, winner_mvp: bool) -> (String, i64, i64) {
        use refresher_core::db;
        let (_, match_id) = db::create_match(pool, camp_id, 111).await.unwrap();
        db::close_voting(pool, camp_id, &match_id).await.unwrap();
        db::start_live_match(pool, &match_id, None, None).await.unwrap();
        let info = db::load_match_info(pool, &match_id, 111).await.unwrap();
        let side = |side: &str| info.players.iter().find(|p| p.side == side).unwrap().user_id;
        let (winner, loser) = (side("hiddenking"), side("archmother"));
        db::report_winner(pool, camp_id, &match_id, "hiddenking", winner_mvp.then_some(winner))
            .await
            .unwrap();
        (match_id, winner, loser)
    }

    #[tokio::test]
    async fn ranking_overlay_ranks_and_tracks_the_latest_result() {
        use refresher_core::db;
        let state = test_state(FakeLink {
            guilds: vec![111],
            ..FakeLink::default()
        })
        .await;
        let pool = &state.db;
        let path = format!("/api/overlay/{}/ranking", db::overlay_token(pool, 111).await.unwrap());
        let (status, body) = get(&state, &path, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!((body["camp_id"].is_null(), body["players"].as_array().unwrap().len()), (true, 0));

        // A 1v1 test camp: two matches, so all four play once.
        let camp = db::create_camp(pool, 111, 900, 4, 2).await.unwrap();
        db::add_players(pool, &camp.id, &[1, 2, 3, 4]).await.unwrap();
        db::start_queue(pool, &camp.id).await.unwrap();
        let (first, w1, l1) = play_1v1(pool, &camp.id, false).await;
        let row = |body: &serde_json::Value, id: i64| {
            body["players"].as_array().unwrap().iter().find(|p| p["user_id"] == id.to_string()).unwrap().clone()
        };

        let (_, body) = get(&state, &path, None).await;
        assert_eq!(body["match_count"], 1);
        let leader = row(&body, w1);
        assert_eq!((leader["place"].as_u64(), leader["points"].as_i64()), (Some(1), Some(1)));
        assert_eq!((leader["wins"].as_i64(), leader["played"].as_i64()), (Some(1), Some(1)));
        assert_eq!(leader["scored_last"], true);
        assert!(body["players"].as_array().unwrap().iter().all(|p| p["moved"].is_null()), "nothing to compare yet");
        assert_eq!(row(&body, l1)["place"], 2);

        // Results are timed to the second: move the first back so the order is unambiguous.
        sqlx::query("UPDATE camp_matches SET completed_at = completed_at - 60 WHERE id = ?")
            .bind(&first)
            .execute(pool)
            .await
            .unwrap();
        let (_, w2, l2) = play_1v1(pool, &camp.id, true).await;
        assert!(![w1, l1].contains(&w2), "the two who waited play the second match");

        let (_, body) = get(&state, &path, None).await;
        let places: Vec<(i64, u64, i64, bool)> = [w2, w1, l1, l2]
            .iter()
            .map(|id| {
                let p = row(&body, *id);
                (*id, p["place"].as_u64().unwrap(), p["moved"].as_i64().unwrap(), p["scored_last"].as_bool().unwrap())
            })
            .collect();
        // The MVP breaks the tie at one point; the first winner drops to second, the losers to third.
        assert_eq!(places, [(w2, 1, 1, true), (w1, 2, -1, false), (l1, 3, -1, false), (l2, 3, -1, false)]);
        assert_eq!(body["players"][0]["user_id"], w2.to_string(), "best first");
        assert_eq!(row(&body, w2)["mvps"], 1);

        db::end_camp(pool, &camp.id).await.unwrap();
        let (_, body) = get(&state, &path, None).await;
        assert_eq!((body["ended"].as_bool(), body["camp_id"].as_str()), (Some(true), Some(camp.id.as_str())));
        assert_eq!(body["players"].as_array().unwrap().len(), 4, "the final ranking stays up");
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
        let close = r#"{"type": "close_vote", "match_id": "R4T8WN"}"#;
        assert_eq!(post(&state, &path, Some(&token), close).await.0, StatusCode::OK);
        let refresh = r#"{"type": "refresh_teams", "match_id": "r4t8wn"}"#;
        assert_eq!(post(&state, &path, Some(&token), refresh).await.0, StatusCode::OK);

        let actions = link.actions.lock().unwrap();
        assert_eq!(
            *actions,
            [
                (
                    111,
                    camp.id.clone(),
                    CampAction::ReportWinner {
                        match_id: "R4T8WN".to_string(),
                        side: "archmother".to_string(),
                        mvp_user_id: None,
                    }
                ),
                (111, camp.id.clone(), CampAction::CreateMatch),
                (
                    111,
                    camp.id.clone(),
                    CampAction::CloseVote {
                        match_id: "R4T8WN".to_string()
                    }
                ),
                (
                    111,
                    camp.id.clone(),
                    CampAction::RefreshTeams {
                        match_id: "r4t8wn".to_string()
                    }
                ),
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
    async fn channels_and_members_need_a_managed_server() {
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

        let (status, body) = get(&state, "/api/guilds/111/members", Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
        let names: Vec<&str> = body.as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["kaze", "kami", "zeca"]);
        assert_eq!(body[0]["id"], "5");

        assert_eq!(get(&state, "/api/guilds/222/channels", Some(&token)).await.0, StatusCode::NOT_FOUND);
        assert_eq!(get(&state, "/api/guilds/222/members", Some(&token)).await.0, StatusCode::NOT_FOUND);
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
        let role = r#"{"type": "set_role", "user_id": "5", "role": "support"}"#;
        assert_eq!(post(&state, &path, Some(&token), role).await.0, StatusCode::OK);
        let no_role = r#"{"type": "set_role", "user_id": "5", "role": null}"#;
        assert_eq!(post(&state, &path, Some(&token), no_role).await.0, StatusCode::OK);
        let bad_role = r#"{"type": "set_role", "user_id": "5", "role": "jungle"}"#;
        assert_eq!(post(&state, &path, Some(&token), bad_role).await.0, StatusCode::BAD_REQUEST);
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
                CampAction::SetRole { user_id: 5, role: Some(Role::Support) },
                CampAction::SetRole { user_id: 5, role: None },
            ]
        );
    }

    #[tokio::test]
    async fn queue_order_and_reminders_go_to_the_bot() {
        let link = Arc::new(FakeLink::default());
        let state = test_state_with(link.clone()).await;
        let camp = refresher_core::db::create_camp(&state.db, 111, 900, 4, 2).await.unwrap();
        let token = sign_in(&state, &[111]).await;
        let path = format!("/api/camps/{}/actions", camp.id);

        let reorder = r#"{"type": "reorder_queue", "user_ids": ["7", "5", "6"]}"#;
        assert_eq!(post(&state, &path, Some(&token), reorder).await.0, StatusCode::OK);
        let remind = r#"{"type": "remind_voters", "match_id": "R4T8WN"}"#;
        assert_eq!(post(&state, &path, Some(&token), remind).await.0, StatusCode::OK);
        let points = r#"{"type": "adjust_points", "user_id": "5", "delta": -2}"#;
        assert_eq!(post(&state, &path, Some(&token), points).await.0, StatusCode::OK);
        let winner = r#"{"type": "report_winner", "match_id": "R4T8WN", "side": "hiddenking", "mvp_user_id": "6"}"#;
        assert_eq!(post(&state, &path, Some(&token), winner).await.0, StatusCode::OK);
        let mvp = r#"{"type": "change_mvp", "match_id": "R4T8WN", "user_id": "7"}"#;
        assert_eq!(post(&state, &path, Some(&token), mvp).await.0, StatusCode::OK);
        let bad_mvp = r#"{"type": "change_mvp", "match_id": "R4T8WN", "user_id": "seven"}"#;
        assert_eq!(post(&state, &path, Some(&token), bad_mvp).await.0, StatusCode::BAD_REQUEST);
        let bad = r#"{"type": "reorder_queue", "user_ids": ["7", "x"]}"#;
        assert_eq!(post(&state, &path, Some(&token), bad).await.0, StatusCode::BAD_REQUEST);

        let actions = link.actions.lock().unwrap();
        assert_eq!(
            actions.iter().map(|a| a.2.clone()).collect::<Vec<_>>(),
            [
                CampAction::ReorderQueue { user_ids: vec![7, 5, 6] },
                CampAction::RemindVoters {
                    match_id: "R4T8WN".to_string()
                },
                CampAction::AdjustPoints { user_id: 5, delta: -2 },
                CampAction::ReportWinner {
                    match_id: "R4T8WN".to_string(),
                    side: "hiddenking".to_string(),
                    mvp_user_id: Some(6),
                },
                CampAction::ChangeMvp {
                    match_id: "R4T8WN".to_string(),
                    user_id: 7,
                },
            ]
        );
    }

    #[tokio::test]
    async fn match_list_stats_and_discord_links() {
        let state = test_state(FakeLink {
            guilds: vec![111],
            ..FakeLink::default()
        })
        .await;
        let db = &state.db;
        let camp = refresher_core::db::create_camp(db, 111, 900, 4, 4).await.unwrap();
        refresher_core::db::add_players(db, &camp.id, &[1, 2, 3, 4]).await.unwrap();
        refresher_core::db::start_queue(db, &camp.id).await.unwrap();
        let (_, first) = refresher_core::db::create_match(db, &camp.id, 111).await.unwrap();
        refresher_core::db::set_match_message_id(db, &first, 555).await.unwrap();
        refresher_core::db::close_voting(db, &camp.id, &first).await.unwrap();
        refresher_core::db::start_live_match(db, &first, None, None).await.unwrap();
        let mvp = refresher_core::db::load_match_info(db, &first, 111)
            .await
            .unwrap()
            .players
            .iter()
            .find(|p| p.side == "archmother")
            .map(|p| p.user_id);
        refresher_core::db::report_winner(db, &camp.id, &first, "archmother", mvp).await.unwrap();
        let (_, second) = refresher_core::db::create_match(db, &camp.id, 111).await.unwrap();
        let token = sign_in(&state, &[111]).await;

        let (status, body) = get(&state, &format!("/api/camps/{}/matches", camp.id), Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body[0]["id"], second.as_str());
        assert_eq!(body[0]["number"], serde_json::Value::Null);
        assert_eq!(body[0]["discord_url"], "https://discord.com/channels/111/900");
        assert_eq!(body[1]["id"], first.as_str());
        assert_eq!(body[1]["number"], 1);
        assert_eq!(body[1]["winner_side"], "archmother");
        assert_eq!(body[1]["discord_url"], "https://discord.com/channels/111/900/555");

        let (_, camp_body) = get(&state, &format!("/api/camps/{}", camp.id), Some(&token)).await;
        let players = camp_body["players"].as_array().unwrap();
        assert!(players.iter().all(|p| p["matches_played"] == 1));
        assert_eq!(players.iter().filter(|p| p["wins"] == 1).count(), 2);
        assert_eq!(camp_body["discord_url"], "https://discord.com/channels/111/900");
        assert_eq!(camp_body["draft_tool"], true);
        let mvp = mvp.unwrap().to_string();
        assert_eq!(body[1]["mvp_user_id"], mvp.as_str());
        assert_eq!(camp_body["recent_matches"][0]["mvp_user_id"], mvp.as_str());
        assert_eq!(camp_body["recent_matches"][0]["mvp_changeable"], true);
        assert_eq!(players.iter().filter(|p| p["mvps"] == 1).count(), 1);
        assert!(players.iter().all(|p| p["points_adjusted"] == 0));

        let other = sign_in(&state, &[222]).await;
        assert_eq!(
            get(&state, &format!("/api/camps/{}/matches", camp.id), Some(&other)).await.0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn settings_are_per_server_and_checked() {
        let state = test_state(FakeLink {
            guilds: vec![111, 222],
            ..FakeLink::default()
        })
        .await;
        let token = sign_in(&state, &[111]).await;

        let (status, body) = get(&state, "/api/guilds/111/settings", Some(&token)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["saved"], false);
        assert_eq!(body["create_lobby"], true, "the .env values until saved");
        assert_eq!(body["server_region"], "SouthAmerica");
        assert_eq!(body["voice_channels"][0]["id"], "1111");
        assert_eq!(body["draft_timers"], serde_json::json!([0, 15, 30, 45, 60, 90]));

        let form = r#"{"queue_voice_id": "1111", "match1_hiddenking_voice_id": null,
            "match1_archmother_voice_id": "", "match2_hiddenking_voice_id": null,
            "match2_archmother_voice_id": null, "create_lobby": false,
            "server_region": "  Europe ", "draft_timer_seconds": 45}"#;
        let (status, body) = put(&state, "/api/guilds/111/settings", Some(&token), form).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["saved"], true);
        let (_, body) = get(&state, "/api/guilds/111/settings", Some(&token)).await;
        assert_eq!(body["queue_voice_id"], "1111");
        assert_eq!(body["match1_archmother_voice_id"], serde_json::Value::Null);
        assert_eq!(body["create_lobby"], false);
        assert_eq!(body["server_region"], "Europe");
        assert_eq!(body["draft_timer_seconds"], 45);
        assert_eq!(body["use_draft_tool"], true, "left out of the form: stays on");

        let manual = form.replace("\"create_lobby\": false", "\"create_lobby\": false, \"use_draft_tool\": false");
        assert_eq!(put(&state, "/api/guilds/111/settings", Some(&token), &manual).await.0, StatusCode::OK);
        assert_eq!(get(&state, "/api/guilds/111/settings", Some(&token)).await.1["use_draft_tool"], false);

        let elsewhere = form.replace("\"1111\"", "\"2221\"");
        assert_eq!(put(&state, "/api/guilds/111/settings", Some(&token), &elsewhere).await.0, StatusCode::BAD_REQUEST);
        let odd_timer = form.replace("45", "50");
        assert_eq!(put(&state, "/api/guilds/111/settings", Some(&token), &odd_timer).await.0, StatusCode::BAD_REQUEST);
        assert_eq!(get(&state, "/api/guilds/111/settings", Some(&token)).await.1["draft_timer_seconds"], 45);
        assert_eq!(get(&state, "/api/guilds/222/settings", Some(&token)).await.0, StatusCode::NOT_FOUND);
        assert_eq!(put(&state, "/api/guilds/222/settings", Some(&token), form).await.0, StatusCode::NOT_FOUND);
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
