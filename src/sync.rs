use anyhow::Result;
use serenity::all::{ChannelId, Context, CreateMessage, EditMessage, GuildId, Mentionable, UserId};
use sqlx::SqlitePool;

use crate::config::Config;
use crate::db::{self, CampState, OpenMatch};
use crate::voice::{self, VoiceDest};

pub async fn redraw_board(ctx: &Context, pool: &SqlitePool, state: &CampState) -> Result<()> {
    let channel = ChannelId::new(state.camp.channel_id as u64);
    let embed = if state.camp.status == "signup" {
        crate::ui::signup_embed(state)
    } else {
        crate::ui::board_embed(state)
    };
    let components = if state.camp.status == "signup" {
        crate::ui::signup_components(state)
    } else {
        crate::ui::board_components(state)
    };
    if let Some(id) = state.camp.board_message_id {
        if channel
            .edit_message(
                &ctx.http,
                id as u64,
                EditMessage::new()
                    .embed(embed.clone())
                    .components(components.clone()),
            )
            .await
            .is_ok()
        {
            return Ok(());
        }
    }
    let msg = channel
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed).components(components),
        )
        .await?;
    db::set_board_message_id(pool, &state.camp.id, msg.id.get() as i64).await?;
    Ok(())
}

pub async fn redraw_match(
    ctx: &Context,
    pool: &SqlitePool,
    state: &CampState,
    open: &OpenMatch,
) -> Result<()> {
    let channel = ChannelId::new(state.camp.channel_id as u64);
    let embed = crate::ui::match_embed(state, open);
    let components = crate::ui::match_components(state, open);
    if let Some(id) = open.row.message_id {
        if channel
            .edit_message(
                &ctx.http,
                id as u64,
                EditMessage::new()
                    .embed(embed.clone())
                    .components(components.clone()),
            )
            .await
            .is_ok()
        {
            return Ok(());
        }
    }
    let msg = channel
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed).components(components),
        )
        .await?;
    db::set_match_message_id(pool, open.row.id, msg.id.get() as i64).await?;
    Ok(())
}

pub async fn redraw_all(ctx: &Context, pool: &SqlitePool, state: &CampState) -> Result<()> {
    redraw_board(ctx, pool, state).await?;
    let fresh = db::load_camp(pool, &state.camp.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("camp missing"))?;
    for open in &fresh.open_matches {
        redraw_match(ctx, pool, &fresh, open).await?;
    }
    Ok(())
}

pub async fn restore_messages(ctx: &Context, pool: &SqlitePool) -> Result<()> {
    let camps = match db::load_restore_camps(pool).await {
        Ok(c) => c,
        Err(err) => {
            tracing::error!(error = %err, "skipping camp restore due to inconsistent data");
            return Ok(());
        }
    };
    for state in camps {
        if let Err(err) = restore_one(ctx, pool, &state).await {
            tracing::error!(camp = %state.camp.id, error = %err, "failed to restore camp messages");
        }
    }
    Ok(())
}

async fn restore_one(ctx: &Context, pool: &SqlitePool, state: &CampState) -> Result<()> {
    redraw_board(ctx, pool, state).await?;
    let fresh = db::load_camp(pool, &state.camp.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("camp missing after restore"))?;
    for open in &fresh.open_matches {
        redraw_match(ctx, pool, &fresh, open).await?;
    }
    Ok(())
}

pub fn match_voice_moves(open: &OpenMatch) -> Vec<(UserId, VoiceDest)> {
    open.players
        .iter()
        .map(|p| {
            (
                UserId::new(p.user_id as u64),
                VoiceDest::Match {
                    slot: open.row.slot,
                    hiddenking: p.side == "hiddenking",
                },
            )
        })
        .collect()
}

pub fn queue_voice_moves(user_ids: &[i64]) -> Vec<(UserId, VoiceDest)> {
    user_ids
        .iter()
        .map(|id| (UserId::new(*id as u64), VoiceDest::Queue))
        .collect()
}

pub async fn apply_moves(
    ctx: &Context,
    guild_id: GuildId,
    config: &Config,
    channel: ChannelId,
    moves: &[(UserId, VoiceDest)],
) {
    let skipped = voice::move_players(ctx, guild_id, config, moves).await;
    if let Some(text) = voice::skipped_message(&skipped) {
        let _ = channel.say(&ctx.http, text).await;
    }
}

pub fn mention_list(ids: &[i64]) -> String {
    ids.iter()
        .map(|id| UserId::new(*id as u64).mention().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}
