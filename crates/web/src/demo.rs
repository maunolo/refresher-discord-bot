//! A camp held in memory, with made-up Discord names, for working on the admin page
//! without the bot or a Discord login:
//! `cargo run -p refresher-web --example demo --features demo`, then open `/demo/login`.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use axum::response::Redirect;
use axum::routing::get;
use axum_extra::extract::cookie::CookieJar;
use refresher_core::config::WebConfig;
use refresher_core::db::{self, RefreshOutcome};
use refresher_core::draft::CreatedDraft;
use refresher_core::link::{BotLink, BoxFuture, Profile};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

use crate::session::{self, AdminGuild, Session};
use crate::{auth, router, AppState};

const GUILD: u64 = 1;
const FIRST_USER: u64 = 1001;
const NAMES: [&str; 30] = [
    "kaze", "mibr_ana", "duda.fps", "dani", "bento", "tchelo", "zeca", "lulu", "nando", "gui_k",
    "rafinha", "bruxo", "pietra", "soneca", "vini", "caju", "tati", "maya", "joca", "leo", "ravi",
    "isa", "teco", "fumaça", "nina", "lara", "otto", "cris", "yuri", "miguel",
];

struct DemoLink;

impl BotLink for DemoLink {
    fn in_guild(&self, guild_id: u64) -> bool {
        guild_id == GUILD
    }

    fn profiles<'a>(&'a self, _guild_id: u64, user_ids: &'a [u64]) -> BoxFuture<'a, HashMap<u64, Profile>> {
        Box::pin(async move {
            user_ids
                .iter()
                .filter_map(|id| {
                    let name = NAMES.get(id.checked_sub(FIRST_USER)? as usize)?;
                    Some((
                        *id,
                        Profile {
                            name: name.to_string(),
                            avatar_url: format!("https://cdn.discordapp.com/embed/avatars/{}.png", id % 6),
                        },
                    ))
                })
                .collect()
        })
    }
}

fn fake_draft(match_id: &str) -> CreatedDraft {
    let base = format!("https://draft.deadlock.pro.br/demo{match_id}");
    CreatedDraft {
        spectator_url: base.clone(),
        hiddenking_url: format!("{base}/hk"),
        archmother_url: format!("{base}/am"),
        admin_url: format!("{base}/admin"),
    }
}

async fn roster(pool: &SqlitePool, match_id: &str) -> Result<Vec<(i64, String)>> {
    let info = db::load_match_info(pool, match_id, GUILD as i64).await?;
    Ok(info.players.into_iter().map(|p| (p.user_id, p.side)).collect())
}

/// Everyone votes to keep the teams until the vote closes.
async fn keep_teams(pool: &SqlitePool, match_id: &str) -> Result<()> {
    for (user_id, _) in roster(pool, match_id).await? {
        if let RefreshOutcome::Kept { .. } = db::vote_refresh_choice(pool, match_id, user_id, false).await? {
            break;
        }
    }
    Ok(())
}

async fn seed(pool: &SqlitePool) -> Result<()> {
    let camp = db::create_camp(pool, GUILD as i64, 10, 30, 12).await?;
    let ids: Vec<i64> = (0..30).map(|i| (FIRST_USER + i) as i64).collect();
    db::add_players(pool, &camp.id, &ids).await?;
    db::set_s_rank(pool, &camp.id, &ids[..5]).await?;
    db::start_queue(pool, &camp.id).await?;

    // Six finished matches, for points and history.
    for round in 0..6 {
        let (_, match_id) = db::create_match(pool, &camp.id, GUILD as i64).await?;
        keep_teams(pool, &match_id).await?;
        db::set_draft(pool, &match_id, &fake_draft(&match_id)).await?;
        db::start_live_match(pool, &match_id, None, None).await?;
        let winner = if round % 3 == 1 { "archmother" } else { "hiddenking" };
        db::report_winner(pool, &camp.id, &match_id, winner).await?;
    }

    // Match 1: voting, half the players in. Match 2: drafted and live.
    let (_, voting) = db::create_match(pool, &camp.id, GUILD as i64).await?;
    let players = roster(pool, &voting).await?;
    let hk: Vec<i64> = players.iter().filter(|p| p.1 == "hiddenking").map(|p| p.0).collect();
    let am: Vec<i64> = players.iter().filter(|p| p.1 == "archmother").map(|p| p.0).collect();
    for (user_id, refresh) in [(hk[0], true), (hk[1], true), (am[0], true), (hk[2], false), (am[1], false), (am[2], false)] {
        db::vote_refresh_choice(pool, &voting, user_id, refresh).await?;
    }

    let (_, live) = db::create_match(pool, &camp.id, GUILD as i64).await?;
    keep_teams(pool, &live).await?;
    db::set_draft(pool, &live, &fake_draft(&live)).await?;
    db::start_live_match(pool, &live, None, None).await?;
    Ok(())
}

async fn demo_login(state: AppState, jar: CookieJar) -> Result<(CookieJar, Redirect), crate::error::ApiError> {
    let token = session::create(
        &state.db,
        &Session {
            user_id: 1,
            display_name: "Admin de teste".to_string(),
            avatar: None,
            admin_guilds: vec![AdminGuild {
                id: GUILD,
                name: "Servidor de teste".to_string(),
                icon: None,
            }],
        },
    )
    .await?;
    Ok((jar.add(auth::session_cookie(&state, token)), Redirect::to("/")))
}

pub async fn run(bind: &str, static_dir: &str) -> Result<()> {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    db::migrate(&db).await?;
    seed(&db).await?;
    let state = AppState {
        db,
        config: Arc::new(WebConfig {
            bind: bind.to_string(),
            public_url: format!("http://{bind}"),
            discord_client_id: "demo".to_string(),
            discord_client_secret: "demo".to_string(),
            static_dir: static_dir.to_string(),
        }),
        http: reqwest::Client::new(),
        link: Arc::new(DemoLink),
    };
    let login_state = state.clone();
    let app = router(state).route(
        "/demo/login",
        get(move |jar: CookieJar| demo_login(login_state.clone(), jar)),
    );
    let listener = tokio::net::TcpListener::bind(bind).await?;
    println!("demo admin ready: open http://{bind}/demo/login");
    axum::serve(listener, app).await?;
    Ok(())
}
