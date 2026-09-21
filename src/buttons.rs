use anyhow::{anyhow, Result};
use serenity::all::{
    ChannelId, ComponentInteraction, ComponentInteractionDataKind, Context, CreateInteractionResponse,
    CreateInteractionResponseMessage, GuildId,
};

use crate::db::{self, RefreshOutcome};
use crate::deadlock;
use crate::state::{with_guild_lock, Data};
use crate::sync;

fn is_admin(interaction: &ComponentInteraction) -> bool {
    interaction
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.manage_guild())
        .unwrap_or(false)
}

async fn ephemeral(
    ctx: &Context,
    interaction: &ComponentInteraction,
    text: impl Into<String>,
) -> Result<()> {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(text.into())
                    .ephemeral(true),
            ),
        )
        .await?;
    Ok(())
}

async fn ack(ctx: &Context, interaction: &ComponentInteraction) -> Result<()> {
    interaction
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await?;
    Ok(())
}

pub async fn handle(ctx: &Context, data: &Data, interaction: &ComponentInteraction) -> Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        return ephemeral(ctx, interaction, "Use this in a server.").await;
    };
    with_guild_lock(data, guild_id.get(), || async {
        handle_locked(ctx, data, interaction, guild_id).await
    })
    .await
}

async fn handle_locked(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    guild_id: GuildId,
) -> Result<()> {
    let custom_id = &interaction.data.custom_id;
    let parts: Vec<&str> = custom_id.split(':').collect();
    match parts.as_slice() {
        ["add_users", camp_id] => add_users(ctx, data, interaction, camp_id).await,
        ["start_queue", camp_id] => start_queue(ctx, data, interaction, guild_id, camp_id).await,
        ["create_match", camp_id] => {
            create_match(ctx, data, interaction, guild_id, camp_id).await
        }
        ["refresh", camp_id, match_id] => {
            refresh(ctx, data, interaction, guild_id, camp_id, match_id).await
        }
        ["start_match", camp_id, match_id] => {
            start_match(ctx, data, interaction, guild_id, camp_id, match_id).await
        }
        ["win", camp_id, match_id, side] => {
            report_win(ctx, data, interaction, guild_id, camp_id, match_id, side).await
        }
        ["end_camp", camp_id] => end_camp(ctx, data, interaction, camp_id).await,
        _ => Ok(()),
    }
}

async fn add_users(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    camp_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(ctx, interaction, "Only server managers can add players.").await;
    }
    let ComponentInteractionDataKind::UserSelect { values } = &interaction.data.kind else {
        return Ok(());
    };
    let ids: Vec<i64> = values.iter().map(|u| u.get() as i64).collect();
    match db::add_players(&data.db, camp_id, &ids).await {
        Ok((added, skipped)) => {
            ack(ctx, interaction).await?;
            if let Some(state) = db::load_camp(&data.db, camp_id).await? {
                sync::redraw_board(ctx, &data.db, &state).await?;
            }
            if added == 0 && !skipped.is_empty() {
                let _ = interaction
                    .create_followup(
                        &ctx.http,
                        serenity::all::CreateInteractionResponseFollowup::new()
                            .content("No new players added (duplicates or roster full).")
                            .ephemeral(true),
                    )
                    .await;
            }
            Ok(())
        }
        Err(err) => ephemeral(ctx, interaction, err.to_string()).await,
    }
}

async fn start_queue(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    guild_id: GuildId,
    camp_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(ctx, interaction, "Only server managers can start the queue.").await;
    }
    match db::start_queue(&data.db, camp_id).await {
        Ok(()) => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp missing"))?;
            let channel = ChannelId::new(state.camp.channel_id as u64);
            let queued_ids: Vec<i64> = state.players.iter().map(|p| p.user_id).collect();
            sync::redraw_all(ctx, &data.db, &state).await?;
            sync::apply_moves(
                ctx,
                guild_id,
                &data.config,
                channel,
                &sync::queue_voice_moves(&queued_ids),
            )
            .await;
            let _ = channel
                .say(
                    &ctx.http,
                    format!(
                        "Queue shuffled for camp `{}`. An admin clicks **Create match** to sort the next teams.",
                        state.camp.id
                    ),
                )
                .await;
            Ok(())
        }
        Err(err) => ephemeral(ctx, interaction, err.to_string()).await,
    }
}

async fn create_match(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    guild_id: GuildId,
    camp_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(ctx, interaction, "Only server managers can create a match.").await;
    }
    match db::create_match(&data.db, camp_id).await {
        Ok(match_id) => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp missing"))?;
            let channel = ChannelId::new(state.camp.channel_id as u64);
            sync::redraw_all(ctx, &data.db, &state).await?;
            if let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) {
                sync::apply_moves(
                    ctx,
                    guild_id,
                    &data.config,
                    channel,
                    &sync::match_voice_moves(open),
                )
                .await;
                let hk: Vec<i64> = open
                    .players
                    .iter()
                    .filter(|p| p.side == "hiddenking")
                    .map(|p| p.user_id)
                    .collect();
                let am: Vec<i64> = open
                    .players
                    .iter()
                    .filter(|p| p.side == "archmother")
                    .map(|p| p.user_id)
                    .collect();
                let _ = channel
                    .say(
                        &ctx.http,
                        format!(
                            "Match {} created. Vote to refresh teams, then an admin starts the match.\nHiddenking: {}\nArchmother: {}",
                            open.row.slot,
                            sync::mention_list(&hk),
                            sync::mention_list(&am)
                        ),
                    )
                    .await;
            }
            Ok(())
        }
        Err(err) => ephemeral(ctx, interaction, err.to_string()).await,
    }
}

async fn refresh(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    guild_id: GuildId,
    camp_id: &str,
    match_id: &str,
) -> Result<()> {
    let match_id: i64 = match_id.parse().unwrap_or(0);
    let user_id = interaction.user.id.get() as i64;
    match db::vote_refresh(&data.db, match_id, user_id).await? {
        RefreshOutcome::AlreadyUsed => {
            ephemeral(ctx, interaction, "This match already used its refresh.").await
        }
        RefreshOutcome::NotInMatch => {
            ephemeral(ctx, interaction, "You're not in this match.").await
        }
        RefreshOutcome::AlreadyVoted => {
            ephemeral(ctx, interaction, "You already voted to refresh.").await
        }
        RefreshOutcome::Recorded { hk, am, needed } => {
            ack(ctx, interaction).await?;
            if let Some(state) = db::load_camp(&data.db, camp_id).await? {
                sync::redraw_all(ctx, &data.db, &state).await?;
                let _ = interaction
                    .create_followup(
                        &ctx.http,
                        serenity::all::CreateInteractionResponseFollowup::new()
                            .content(format!(
                                "Vote recorded. Hiddenking {hk}/{needed} · Archmother {am}/{needed}"
                            ))
                            .ephemeral(true),
                    )
                    .await;
            }
            Ok(())
        }
        RefreshOutcome::Refreshed {
            hiddenking,
            archmother,
        } => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp missing"))?;
            let channel = ChannelId::new(state.camp.channel_id as u64);
            sync::redraw_all(ctx, &data.db, &state).await?;
            if let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) {
                sync::apply_moves(
                    ctx,
                    guild_id,
                    &data.config,
                    channel,
                    &sync::match_voice_moves(open),
                )
                .await;
            }
            let _ = channel
                .say(
                    &ctx.http,
                    format!(
                        "Teams refreshed for match.\nHiddenking: {}\nArchmother: {}",
                        sync::mention_list(&hiddenking),
                        sync::mention_list(&archmother)
                    ),
                )
                .await;
            Ok(())
        }
    }
}

async fn start_match(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    guild_id: GuildId,
    camp_id: &str,
    match_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(ctx, interaction, "Only server managers can start a match.").await;
    }
    let match_id: i64 = match_id.parse().unwrap_or(0);
    ack(ctx, interaction).await?;

    let (party_id, party_code) = if data.config.enable_custom_match_create {
        match deadlock::create_custom_match(
            &data.http,
            data.config.deadlock_api_key.as_deref(),
            data.config.deadlock_server_region.as_deref(),
        )
        .await
        {
            Ok(created) => (Some(created.0), Some(created.1)),
            Err(err) => {
                let _ = interaction
                    .create_followup(
                        &ctx.http,
                        serenity::all::CreateInteractionResponseFollowup::new()
                            .content(format!("Failed to create Deadlock lobby: {err}"))
                            .ephemeral(true),
                    )
                    .await;
                return Ok(());
            }
        }
    } else {
        (None, None)
    };

    db::start_live_match(&data.db, match_id, party_id, party_code.clone()).await?;
    let state = db::load_camp(&data.db, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp missing"))?;
    let channel = ChannelId::new(state.camp.channel_id as u64);
    sync::redraw_all(ctx, &data.db, &state).await?;
    if let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) {
        sync::apply_moves(
            ctx,
            guild_id,
            &data.config,
            channel,
            &sync::match_voice_moves(open),
        )
        .await;
    }
    let message = if let Some(code) = party_code {
        format!("Match lobby created. **Party code:** `{code}` — pick heroes in Deadlock.")
    } else {
        "Match is live in Discord. Custom lobby creation is disabled (`ENABLE_CUSTOM_MATCH_CREATE`).".to_string()
    };
    let _ = channel.say(&ctx.http, message).await;
    Ok(())
}

async fn report_win(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    guild_id: GuildId,
    camp_id: &str,
    match_id: &str,
    side: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(ctx, interaction, "Only server managers can report a winner.").await;
    }
    let match_id: i64 = match_id.parse().unwrap_or(0);
    let before = db::load_camp(&data.db, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp missing"))?;
    let finished_ids: Vec<i64> = before
        .open_matches
        .iter()
        .find(|m| m.row.id == match_id)
        .map(|m| m.players.iter().map(|p| p.user_id).collect())
        .unwrap_or_default();
    let old_message = before
        .open_matches
        .iter()
        .find(|m| m.row.id == match_id)
        .and_then(|m| m.row.message_id);

    match db::report_winner(&data.db, camp_id, match_id, side).await {
        Ok(()) => {
            ack(ctx, interaction).await?;
            let channel = ChannelId::new(before.camp.channel_id as u64);
            if let Some(msg_id) = old_message {
                let winner = if side == "hiddenking" {
                    "Hiddenking"
                } else {
                    "Archmother"
                };
                let _ = channel
                    .edit_message(
                        &ctx.http,
                        msg_id as u64,
                        serenity::all::EditMessage::new()
                            .embed(
                                serenity::all::CreateEmbed::new()
                                    .title(format!("{winner} won"))
                                    .description(format!("Camp {}", before.camp.id))
                                    .footer(serenity::all::CreateEmbedFooter::new(format!(
                                        "Camp {}",
                                        before.camp.id
                                    ))),
                            )
                            .components(Vec::new()),
                    )
                    .await;
            }
            sync::apply_moves(
                ctx,
                guild_id,
                &data.config,
                channel,
                &sync::queue_voice_moves(&finished_ids),
            )
            .await;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp missing"))?;
            sync::redraw_all(ctx, &data.db, &state).await?;
            let winner = if side == "hiddenking" {
                "Hiddenking"
            } else {
                "Archmother"
            };
            let _ = channel
                .say(
                    &ctx.http,
                    format!(
                        "{winner} wins. +1 point to that side. Players are back in queue — an admin clicks **Create match** when ready."
                    ),
                )
                .await;
            Ok(())
        }
        Err(err) => ephemeral(ctx, interaction, err.to_string()).await,
    }
}

async fn end_camp(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    camp_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(ctx, interaction, "Only server managers can end the camp.").await;
    }
    match db::end_camp(&data.db, camp_id).await {
        Ok(players) => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp missing"))?;
            let channel = ChannelId::new(state.camp.channel_id as u64);
            let _ = channel
                .send_message(
                    &ctx.http,
                    serenity::all::CreateMessage::new().embed(crate::ui::standings_embed(
                        camp_id,
                        state.camp.match_count,
                        &players,
                    )),
                )
                .await;
            if let Some(board_id) = state.camp.board_message_id {
                let _ = channel
                    .edit_message(
                        &ctx.http,
                        board_id as u64,
                        serenity::all::EditMessage::new()
                            .embed(crate::ui::standings_embed(
                                camp_id,
                                state.camp.match_count,
                                &players,
                            ))
                            .components(Vec::new()),
                    )
                    .await;
            }
            Ok(())
        }
        Err(err) => ephemeral(ctx, interaction, err.to_string()).await,
    }
}
