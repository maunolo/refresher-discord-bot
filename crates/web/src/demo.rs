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
use refresher_core::link::{BotLink, BoxFuture, CampAction, Channel, Profile};
use refresher_core::queue::CampSize;
use refresher_core::settings::GuildSettings;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

use crate::session::{self, AdminGuild, Session};
use crate::{auth, router, AppState};

const GUILD: u64 = 1;
const FIRST_USER: u64 = 1001;
/// The first 30 start on the seeded camp's roster; the rest are members to add.
const NAMES: [&str; 40] = [
    "kaze", "mibr_ana", "duda.fps", "dani", "bento", "tchelo", "zeca", "lulu", "nando", "gui_k",
    "rafinha", "bruxo", "pietra", "soneca", "vini", "caju", "tati", "maya", "joca", "leo", "ravi",
    "isa", "teco", "fumaça", "nina", "lara", "otto", "cris", "yuri", "miguel", "luana", "lucas.dl",
    "beto", "carol", "dudu", "fefe", "gabi", "hugo", "ingrid", "juca",
];

fn demo_profile(user_id: u64) -> Option<Profile> {
    let name = NAMES.get(user_id.checked_sub(FIRST_USER)? as usize)?;
    Some(Profile {
        name: name.to_string(),
        avatar_url: format!("https://cdn.discordapp.com/embed/avatars/{}.png", user_id % 6),
    })
}

/// Runs actions on the in-memory database only; nothing goes to Discord.
struct DemoLink {
    db: SqlitePool,
}

impl BotLink for DemoLink {
    fn act<'a>(&'a self, _guild_id: u64, camp_id: &'a str, action: CampAction) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let pool = &self.db;
            let guild = GUILD as i64;
            match action {
                CampAction::CreateMatch => {
                    db::create_match(pool, camp_id, guild).await?;
                }
                CampAction::CreateFinal => {
                    db::create_final_match(pool, camp_id, guild).await?;
                }
                CampAction::CloseVote { match_id } => {
                    db::close_voting(pool, camp_id, &match_id).await?;
                }
                CampAction::RefreshTeams { match_id } => {
                    db::refresh_by_admin(pool, camp_id, &match_id).await?;
                }
                CampAction::StartDraft { match_id } => {
                    let (settings, _) = db::guild_settings(pool, guild, &demo_settings()).await?;
                    if settings.use_draft_tool {
                        db::set_draft(pool, &match_id, &fake_draft(&match_id)).await?;
                    } else {
                        db::set_manual_draft(pool, &match_id).await?;
                    }
                }
                CampAction::StartMatch { match_id } => {
                    db::start_live_match(pool, &match_id, None, None).await?;
                }
                CampAction::ReportWinner { match_id, side, mvp_user_id } => {
                    db::report_winner(pool, camp_id, &match_id, &side, mvp_user_id.map(|id| id as i64)).await?;
                }
                CampAction::ChangeMvp { match_id, user_id } => {
                    db::change_mvp(pool, camp_id, &match_id, user_id as i64).await?;
                }
                CampAction::CorrectWinner { match_id } => {
                    db::correct_winner(pool, camp_id, &match_id).await?;
                }
                CampAction::ReopenMatch { match_id } => {
                    db::reopen_match(pool, camp_id, &match_id).await?;
                }
                CampAction::AddPlayers { user_ids } => {
                    let ids: Vec<i64> = user_ids.iter().map(|id| *id as i64).collect();
                    db::add_players(pool, camp_id, &ids).await?;
                }
                CampAction::RemovePlayer { user_id } => {
                    db::remove_player(pool, camp_id, user_id as i64).await?;
                }
                CampAction::SetSRank { user_id, s_rank } => {
                    db::set_player_s_rank(pool, camp_id, user_id as i64, s_rank).await?;
                }
                CampAction::AdjustPoints { user_id, delta } => {
                    db::adjust_points(pool, camp_id, user_id as i64, delta).await?;
                }
                CampAction::StartQueue => {
                    db::start_queue(pool, camp_id).await?;
                }
                CampAction::ReorderQueue { user_ids } => {
                    let ids: Vec<i64> = user_ids.iter().map(|id| *id as i64).collect();
                    db::reorder_queue(pool, camp_id, &ids).await?;
                }
                CampAction::RemindVoters { match_id } => {
                    // The bot would mention them in the camp's channel; here it only checks.
                    let state = db::load_camp(pool, camp_id).await?.ok_or_else(|| anyhow::anyhow!("camp não encontrado"))?;
                    let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) else {
                        anyhow::bail!("Partida não encontrada.");
                    };
                    if open.row.status != "voting" || open.row.refresh_used {
                        anyhow::bail!("A votação desta partida já terminou.");
                    }
                    if open.players.len() == open.votes.len() {
                        anyhow::bail!("Todos já votaram.");
                    }
                }
                CampAction::CancelCamp => db::cancel_camp(pool, camp_id).await?,
                CampAction::ResumeCamp => {
                    db::resume_camp(pool, camp_id, guild).await?;
                }
                CampAction::EndCamp => {
                    db::end_camp(pool, camp_id).await?;
                }
            }
            Ok(())
        })
    }

    fn text_channels(&self, _guild_id: u64) -> Vec<Channel> {
        vec![
            Channel { id: 10, name: "camp-deadlock".to_string() },
            Channel { id: 11, name: "testes-bot".to_string() },
        ]
    }

    fn voice_channels(&self, _guild_id: u64) -> Vec<Channel> {
        ["Queue", "Match 1 Hiddenking", "Match 1 Archmother", "Match 2 Hiddenking", "Match 2 Archmother", "Geral"]
            .iter()
            .zip(20..)
            .map(|(name, id)| Channel { id, name: name.to_string() })
            .collect()
    }

    fn search_members<'a>(
        &'a self,
        _guild_id: u64,
        query: &'a str,
        limit: u64,
    ) -> BoxFuture<'a, Result<Vec<(u64, Profile)>>> {
        Box::pin(async move {
            let query = query.to_lowercase();
            Ok((0..NAMES.len() as u64)
                .map(|i| FIRST_USER + i)
                .filter_map(|id| demo_profile(id).map(|p| (id, p)))
                .filter(|(_, p)| p.name.starts_with(&query))
                .take(limit as usize)
                .collect())
        })
    }

    fn queue_voice_members(&self, _guild_id: u64) -> BoxFuture<'_, Vec<u64>> {
        Box::pin(async { (30..36).map(|i| FIRST_USER + i).collect() })
    }

    fn create_camp<'a>(&'a self, _guild_id: u64, channel_id: u64, size: CampSize) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            if db::load_active_for_guild(&self.db, GUILD as i64).await?.is_some() {
                anyhow::bail!("Este servidor já tem um camp ativo. Cancele ou encerre o atual primeiro.");
            }
            let camp = db::create_camp(&self.db, GUILD as i64, channel_id as i64, size.target_size(), size.match_size()).await?;
            Ok(camp.id)
        })
    }

    fn in_guild(&self, guild_id: u64) -> bool {
        guild_id == GUILD
    }

    fn profiles<'a>(&'a self, _guild_id: u64, user_ids: &'a [u64]) -> BoxFuture<'a, HashMap<u64, Profile>> {
        Box::pin(async move {
            user_ids
                .iter()
                .filter_map(|id| demo_profile(*id).map(|p| (*id, p)))
                .collect()
        })
    }
}

/// The demo server's settings until changed on Configurações.
fn demo_settings() -> GuildSettings {
    GuildSettings {
        queue_voice_id: None,
        match1_hiddenking_voice_id: None,
        match1_archmother_voice_id: None,
        match2_hiddenking_voice_id: None,
        match2_archmother_voice_id: None,
        create_lobby: false,
        server_region: None,
        use_draft_tool: true,
        draft_timer_seconds: 30,
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
        // MVPs spread over a few players, so some ties in points are broken.
        let mvp = roster(pool, &match_id)
            .await?
            .into_iter()
            .filter(|(_, side)| side == winner)
            .nth(round % 2)
            .map(|(user_id, _)| user_id);
        db::report_winner(pool, &camp.id, &match_id, winner, mvp).await?;
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
    // The camp lives only as long as this one connection: never let the pool close it.
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect("sqlite::memory:")
        .await?;
    db::migrate(&db).await?;
    seed(&db).await?;
    let link = Arc::new(DemoLink { db: db.clone() });
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
        link,
        default_settings: demo_settings(),
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
