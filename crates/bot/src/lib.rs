use anyhow::Result;
use poise::serenity_prelude as serenity;
use serenity::FullEvent;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

mod actions;
mod buttons;
mod discord;
mod link;
mod state;
mod sync;
mod ui;
mod voice;

// Camp rules and storage live in refresher-core; the bot modules reach them as `crate::db` etc.
use refresher_core::{config, db, deadlock, draft, queue, settings};

use config::Config;
use discord::Discord;
use refresher_core::link::BotLink;
pub use state::Data;

pub type Error = anyhow::Error;

async fn event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready { data_about_bot, .. } => {
            tracing::info!("logged in as {}", data_about_bot.user.name);
            // Redrawing also strips the admin buttons older versions left on these messages.
            if let Err(err) = sync::restore_messages(&Discord::from(ctx), &data.db).await {
                tracing::error!(error = %err, "camp restore failed");
            }
        }
        FullEvent::GuildCreate { guild, .. } => {
            // Camps are run from the admin page now: clear the old /camp command from the server.
            if let Err(err) = guild.id.set_commands(&ctx.http, Vec::new()).await {
                tracing::warn!(error = %err, guild_id = %guild.id, "failed to clear slash commands");
            }
        }
        FullEvent::VoiceStateUpdate { old, new } => {
            let discord = Discord::from(ctx);
            if let Err(err) = sync::place_late_voice_join(&discord, data, old.as_ref(), new).await {
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

/// The Discord client, built but not yet connected.
pub struct Bot {
    client: serenity::Client,
    /// Shared with the event handlers, so web actions use the same per-server locks.
    data: Data,
}

impl Bot {
    pub async fn new(config: Config, db: SqlitePool) -> Result<Self> {
        let data = Data {
            config: config.clone(),
            db,
            http: reqwest::Client::new(),
            locks: Arc::new(Mutex::new(HashMap::new())),
        };
        Ok(Self {
            client: build_client(config, data.clone()).await?,
            data,
        })
    }

    /// Lets the web admin look things up and run actions through this bot's connection.
    pub fn link(&self) -> Arc<dyn BotLink> {
        Arc::new(link::SerenityLink::new(
            Discord {
                http: self.client.http.clone(),
                cache: self.client.cache.clone(),
            },
            self.data.clone(),
        ))
    }

    /// Connects to Discord, keeps the camp messages up to date and handles the players' buttons
    /// until the connection ends.
    pub async fn run(mut self) -> Result<()> {
        self.client.start().await?;
        Ok(())
    }
}

async fn build_client(config: Config, data: Data) -> Result<serenity::Client> {
    let intents = serenity::GatewayIntents::GUILDS | serenity::GatewayIntents::GUILD_VOICE_STATES;
    let token = config.discord_token.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            on_error: |err| Box::pin(on_error(err)),
            ..Default::default()
        })
        .setup(move |ctx, _ready, _framework| {
            let data = data.clone();
            Box::pin(async move {
                // The bot has no slash commands; make sure no global ones linger either.
                serenity::all::Command::set_global_commands(&ctx.http, vec![]).await?;
                Ok(data)
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;
    Ok(client)
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    tracing::error!("framework error: {error}");
}
