use anyhow::Result;
use poise::serenity_prelude as serenity;
use serenity::FullEvent;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

mod buttons;
mod commands;
mod state;
mod sync;
mod ui;
mod voice;

// Camp rules and storage live in refresher-core; the bot modules reach them as `crate::db` etc.
use refresher_core::{config, db, deadlock, draft, queue};

use config::Config;
pub use state::Data;

pub type Error = anyhow::Error;

async fn event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready { data_about_bot, .. } => {
            tracing::info!("logged in as {}", data_about_bot.user.name);
            if let Err(err) = sync::restore_messages(ctx, &data.db).await {
                tracing::error!(error = %err, "camp restore failed");
            }
        }
        FullEvent::GuildCreate { guild, .. } => {
            if let Err(err) = poise::builtins::register_in_guild(
                ctx,
                &framework.options().commands,
                guild.id,
            )
            .await
            {
                tracing::error!(error = %err, guild_id = %guild.id, "failed to register slash commands");
            } else {
                tracing::info!("registered slash commands in {}", guild.name);
            }
        }
        FullEvent::VoiceStateUpdate { old, new } => {
            if let Err(err) = sync::place_late_voice_join(ctx, data, old.as_ref(), new).await {
                tracing::warn!(error = %err, "late voice placement failed");
            }
        }
        FullEvent::InteractionCreate { interaction } => {
            if let serenity::Interaction::Component(component) = interaction {
                if let Err(err) = buttons::handle(ctx, data, component).await {
                    tracing::error!(error = %err, "component handler failed");
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Connects to Discord and handles camp commands and buttons until the connection ends.
pub async fn run(config: Config, db: SqlitePool) -> Result<()> {
    let data = Data {
        config: config.clone(),
        db,
        http: reqwest::Client::new(),
        locks: Arc::new(Mutex::new(HashMap::new())),
    };

    let intents = serenity::GatewayIntents::GUILDS | serenity::GatewayIntents::GUILD_VOICE_STATES;
    let token = config.discord_token.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::camp()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            on_error: |err| Box::pin(on_error(err)),
            ..Default::default()
        })
        .setup(move |ctx, _ready, _framework| {
            let data = data.clone();
            Box::pin(async move {
                // Guild commands update immediately. A leftover global /camp
                // would keep showing the old 24/30-only size list for up to an hour.
                serenity::all::Command::set_global_commands(&ctx.http, vec![]).await?;
                Ok(data)
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;
    client.start().await?;
    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            tracing::error!(error = %error, "command failed");
            let _ = ctx
                .send(
                    poise::CreateReply::default()
                        .content(format!("Erro: {error}"))
                        .ephemeral(true),
                )
                .await;
        }
        other => {
            tracing::error!("framework error: {other}");
        }
    }
}
