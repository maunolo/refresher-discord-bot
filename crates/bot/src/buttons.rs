//! The buttons players press in Discord: the refresh vote and the captains' draft link.
//! Everything an admin does happens on the web admin page.

use anyhow::{anyhow, Result};
use serenity::all::{
    ChannelId, ComponentInteraction, Context, CreateInteractionResponse, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage,
};

use crate::db::{self, RefreshOutcome};
use crate::discord::Discord;
use crate::state::{with_guild_lock, Data};
use crate::sync;

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

/// A private message after `ack`.
async fn followup(ctx: &Context, interaction: &ComponentInteraction, text: impl Into<String>) {
    let _ = interaction
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .content(text.into())
                .ephemeral(true),
        )
        .await;
}

pub async fn handle(ctx: &Context, data: &Data, interaction: &ComponentInteraction) -> Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        return ephemeral(ctx, interaction, "Use isso em um servidor.").await;
    };
    let result = with_guild_lock(data, guild_id.get(), || async {
        handle_locked(ctx, data, interaction).await
    })
    .await;
    // A vote and the like: open admin pages reload the camp.
    data.changes.notify(guild_id.get());
    result
}

async fn handle_locked(ctx: &Context, data: &Data, interaction: &ComponentInteraction) -> Result<()> {
    let custom_id = &interaction.data.custom_id;
    let parts: Vec<&str> = custom_id.split(':').collect();
    match parts.as_slice() {
        ["refresh", camp_id, match_id] => {
            vote_choice(ctx, data, interaction, camp_id, match_id, true).await
        }
        ["no_refresh", camp_id, match_id] => {
            vote_choice(ctx, data, interaction, camp_id, match_id, false).await
        }
        ["captain_link", camp_id, match_id] => {
            captain_link(ctx, data, interaction, camp_id, match_id).await
        }
        // Admin buttons left on messages from before the admin page.
        _ => {
            ephemeral(
                ctx,
                interaction,
                format!(
                    "Isso agora é feito no painel admin: {}",
                    data.config.web.public_url
                ),
            )
            .await
        }
    }
}

async fn vote_choice(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    camp_id: &str,
    match_id: &str,
    want_refresh: bool,
) -> Result<()> {
    let match_id = match_id.trim().to_uppercase();
    let user_id = interaction.user.id.get() as i64;
    let discord = Discord::from(ctx);
    match db::vote_refresh_choice(&data.db, &match_id, user_id, want_refresh).await? {
        RefreshOutcome::AlreadyUsed => {
            ephemeral(ctx, interaction, "A votação desta partida já acabou.").await
        }
        RefreshOutcome::NotInMatch => {
            ephemeral(ctx, interaction, "Você não está nesta partida.").await
        }
        RefreshOutcome::AlreadyVoted => {
            ephemeral(ctx, interaction, "Você já votou.").await
        }
        RefreshOutcome::Recorded {
            hk,
            am,
            keep,
            voted,
            total,
            needed,
        } => {
            ack(ctx, interaction).await?;
            if let Some(state) = db::load_camp(&data.db, camp_id).await? {
                sync::redraw_all(&discord, &data.db, &state).await?;
                followup(
                    ctx,
                    interaction,
                    format!(
                        "Voto registrado. Atualizar Hiddenking {hk}/{needed} · Archmother {am}/{needed} · Manter times {keep} · Votos {voted}/{total}"
                    ),
                )
                .await;
            }
            Ok(())
        }
        RefreshOutcome::Kept { voted, total } => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp não encontrado"))?;
            let channel = ChannelId::new(state.camp.channel_id as u64);
            sync::redraw_all(&discord, &data.db, &state).await?;
            let _ = channel
                .say(
                    &ctx.http,
                    format!(
                        "Votação encerrada — times mantidos ({voted}/{total}). Um admin pode iniciar o draft."
                    ),
                )
                .await;
            Ok(())
        }
        RefreshOutcome::Refreshed(change) => {
            ack(ctx, interaction).await?;
            let guild_id = interaction.guild_id.ok_or_else(|| anyhow!("Use isso em um servidor."))?;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp não encontrado"))?;
            sync::redraw_all(&discord, &data.db, &state).await?;
            if let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) {
                let headline = "Refresh aprovado: times atualizados nesta partida.";
                sync::announce_refresh(&discord, data, guild_id, &state, open, &change, headline).await?;
            }
            Ok(())
        }
    }
}

/// Shows a captain the draft's invite link, in case the DM didn't arrive.
async fn captain_link(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    camp_id: &str,
    match_id: &str,
) -> Result<()> {
    let match_id = match_id.trim().to_uppercase();
    let state = db::load_camp(&data.db, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) else {
        return ephemeral(ctx, interaction, "Partida não encontrada.").await;
    };
    let user_id = interaction.user.id.get() as i64;
    let Some(captain) = open
        .players
        .iter()
        .find(|p| p.user_id == user_id && p.is_captain)
    else {
        return ephemeral(
            ctx,
            interaction,
            format!(
                "Só os capitães ({}) desta partida têm link do draft.",
                crate::ui::CAPTAIN_TAG
            ),
        )
        .await;
    };
    let Some(url) = &open.row.draft_join_url else {
        return ephemeral(ctx, interaction, "Não há link de draft nesta partida.").await;
    };
    ephemeral(
        ctx,
        interaction,
        format!(
            "Abra o link do draft e entre no lado **{}**. Não compartilhe: quem tiver o link pode ocupar uma vaga no draft.\n{url}",
            crate::ui::winner_label(&captain.side)
        ),
    )
    .await
}
