use anyhow::{anyhow, Result};
use refresher_core::link::CampAction;
use serenity::all::{
    ChannelId, ComponentInteraction, ComponentInteractionDataKind, Context, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, GuildId,
};

use crate::actions;
use crate::db::{self, RefreshOutcome};
use crate::discord::Discord;
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

/// A private message after `ack`, for errors and notes.
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
    let admin = |action: CampAction, camp_id: &str, what: &'static str| {
        admin_action(ctx, data, interaction, guild_id, camp_id.to_string(), action, what)
    };
    match parts.as_slice() {
        ["add_users", camp_id] => add_users(ctx, data, interaction, camp_id).await,
        ["s_rank", camp_id] => set_s_rank(ctx, data, interaction, camp_id).await,
        ["start_queue", camp_id] => admin(CampAction::StartQueue, camp_id, "iniciar a fila").await,
        ["create_match", camp_id] => admin(CampAction::CreateMatch, camp_id, "criar uma partida").await,
        ["final_match", camp_id] => admin(CampAction::CreateFinal, camp_id, "iniciar a final").await,
        ["refresh", camp_id, match_id] => {
            vote_choice(ctx, data, interaction, camp_id, match_id, true).await
        }
        ["no_refresh", camp_id, match_id] => {
            vote_choice(ctx, data, interaction, camp_id, match_id, false).await
        }
        ["start_draft", camp_id, match_id] => {
            let action = CampAction::StartDraft { match_id: match_id.to_string() };
            admin(action, camp_id, "iniciar o draft").await
        }
        ["start_match", camp_id, match_id] => {
            let action = CampAction::StartMatch { match_id: match_id.to_string() };
            admin(action, camp_id, "iniciar uma partida").await
        }
        ["captain_link", camp_id, match_id] => {
            captain_link(ctx, data, interaction, camp_id, match_id).await
        }
        ["win", camp_id, match_id, side] => {
            let action = CampAction::ReportWinner {
                match_id: match_id.to_string(),
                side: side.to_string(),
            };
            admin(action, camp_id, "informar o vencedor").await
        }
        ["correct_win", camp_id, match_id] => {
            let action = CampAction::CorrectWinner { match_id: match_id.to_string() };
            admin(action, camp_id, "corrigir o resultado").await
        }
        ["reopen_match", camp_id, match_id] => {
            let action = CampAction::ReopenMatch { match_id: match_id.to_string() };
            admin(action, camp_id, "reabrir a partida").await
        }
        ["end_camp", camp_id] => admin(CampAction::EndCamp, camp_id, "encerrar o camp").await,
        _ => Ok(()),
    }
}

/// Answers Discord right away (it allows 3 seconds), then runs the action the web admin also
/// uses, and tells the admin privately if it failed.
async fn admin_action(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    guild_id: GuildId,
    camp_id: String,
    action: CampAction,
    what: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(ctx, interaction, format!("Apenas quem gerencia o servidor pode {what}.")).await;
    }
    ack(ctx, interaction).await?;
    if let Err(err) = actions::run(&Discord::from(ctx), data, guild_id, &camp_id, &action).await {
        followup(ctx, interaction, err.to_string()).await;
    }
    Ok(())
}

async fn add_users(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    camp_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(ctx, interaction, "Apenas quem gerencia o servidor pode adicionar jogadores.").await;
    }
    let ComponentInteractionDataKind::UserSelect { values } = &interaction.data.kind else {
        return Ok(());
    };
    let ids: Vec<i64> = values.iter().map(|u| u.get() as i64).collect();
    match db::add_players(&data.db, camp_id, &ids).await {
        Ok((added, skipped)) => {
            ack(ctx, interaction).await?;
            if let Some(state) = db::load_camp(&data.db, camp_id).await? {
                sync::redraw_board(&Discord::from(ctx), &data.db, &state).await?;
            }
            if added == 0 && !skipped.is_empty() {
                followup(ctx, interaction, "Nenhum jogador novo adicionado (duplicados ou elenco cheio).").await;
            }
            Ok(())
        }
        Err(err) => ephemeral(ctx, interaction, err.to_string()).await,
    }
}

async fn set_s_rank(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    camp_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(ctx, interaction, "Apenas quem gerencia o servidor pode escolher os S rank.").await;
    }
    let ComponentInteractionDataKind::UserSelect { values } = &interaction.data.kind else {
        return Ok(());
    };
    let ids: Vec<i64> = values.iter().map(|u| u.get() as i64).collect();
    match db::set_s_rank(&data.db, camp_id, &ids).await {
        Ok(not_on_roster) => {
            ack(ctx, interaction).await?;
            if let Some(state) = db::load_camp(&data.db, camp_id).await? {
                sync::redraw_board(&Discord::from(ctx), &data.db, &state).await?;
            }
            if !not_on_roster.is_empty() {
                followup(
                    ctx,
                    interaction,
                    format!("Ignorado(s), fora do elenco: {}", sync::mention_list(&not_on_roster)),
                )
                .await;
            }
            Ok(())
        }
        Err(err) => ephemeral(ctx, interaction, err.to_string()).await,
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
        RefreshOutcome::Refreshed => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp não encontrado"))?;
            let channel = ChannelId::new(state.camp.channel_id as u64);
            sync::redraw_all(&discord, &data.db, &state).await?;
            if let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) {
                let _ = channel
                    .say(
                        &ctx.http,
                        format!(
                            "Times atualizados nesta partida.\nHiddenking: {}\nArchmother: {}",
                            sync::side_mentions(&open.players, "hiddenking"),
                            sync::side_mentions(&open.players, "archmother")
                        ),
                    )
                    .await;
            }
            Ok(())
        }
    }
}

/// Shows a captain their team's private draft link, in case the DM didn't arrive.
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
    let Some(url) = open.row.team_draft_url(&captain.side) else {
        return ephemeral(ctx, interaction, "Não há link de draft do seu time nesta partida.").await;
    };
    ephemeral(
        ctx,
        interaction,
        format!(
            "Seu link do draft (**{}**). Não compartilhe — quem abrir escolhe e bane pelo seu time:\n{url}",
            crate::ui::winner_label(&captain.side)
        ),
    )
    .await
}
