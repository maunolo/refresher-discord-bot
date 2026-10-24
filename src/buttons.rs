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
    match parts.as_slice() {
        ["add_users", camp_id] => add_users(ctx, data, interaction, camp_id).await,
        ["s_rank", camp_id] => set_s_rank(ctx, data, interaction, camp_id).await,
        ["start_queue", camp_id] => start_queue(ctx, data, interaction, guild_id, camp_id).await,
        ["create_match", camp_id] => {
            create_match(ctx, data, interaction, guild_id, camp_id).await
        }
        ["final_match", camp_id] => {
            create_final_match(ctx, data, interaction, guild_id, camp_id).await
        }
        ["refresh", camp_id, match_id] => {
            vote_choice(ctx, data, interaction, camp_id, match_id, true).await
        }
        ["no_refresh", camp_id, match_id] => {
            vote_choice(ctx, data, interaction, camp_id, match_id, false).await
        }
        ["start_match", camp_id, match_id] => {
            start_match(ctx, data, interaction, camp_id, match_id).await
        }
        ["start_draft", camp_id, match_id] => {
            start_draft(ctx, data, interaction, guild_id, camp_id, match_id).await
        }
        ["captain_link", camp_id, match_id] => {
            captain_link(ctx, data, interaction, camp_id, match_id).await
        }
        ["win", camp_id, match_id, side] => {
            report_win(ctx, data, interaction, guild_id, camp_id, match_id, side).await
        }
        ["correct_win", camp_id, match_id] => {
            correct_win(ctx, data, interaction, camp_id, match_id).await
        }
        ["reopen_match", camp_id, match_id] => {
            reopen_match(ctx, data, interaction, guild_id, camp_id, match_id).await
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
                sync::redraw_board(ctx, &data.db, &state).await?;
            }
            if added == 0 && !skipped.is_empty() {
                let _ = interaction
                    .create_followup(
                        &ctx.http,
                        serenity::all::CreateInteractionResponseFollowup::new()
                            .content("Nenhum jogador novo adicionado (duplicados ou elenco cheio).")
                            .ephemeral(true),
                    )
                    .await;
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
                sync::redraw_board(ctx, &data.db, &state).await?;
            }
            if !not_on_roster.is_empty() {
                let _ = interaction
                    .create_followup(
                        &ctx.http,
                        serenity::all::CreateInteractionResponseFollowup::new()
                            .content(format!(
                                "Ignorado(s), fora do elenco: {}",
                                sync::mention_list(&not_on_roster)
                            ))
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
        return ephemeral(ctx, interaction, "Apenas quem gerencia o servidor pode iniciar a fila.").await;
    }
    match db::start_queue(&data.db, camp_id).await {
        Ok(order) => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp não encontrado"))?;
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
                        "Fila embaralhada do camp `{}`:\n{}\nUm admin clica em **Criar partida** para montar os próximos times.",
                        state.camp.id,
                        sync::mention_list(&order)
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
        return ephemeral(ctx, interaction, "Apenas quem gerencia o servidor pode criar uma partida.").await;
    }
    match db::create_match(&data.db, camp_id, guild_id.get() as i64).await {
        Ok((camp_id, match_id)) => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, &camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp não encontrado"))?;
            let channel = ChannelId::new(state.camp.channel_id as u64);
            sync::redraw_all(ctx, &data.db, &state).await?;
            if let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) {
                let _ = channel
                    .say(
                        &ctx.http,
                        format!(
                            "Partida {} criada. Votem para atualizar os times; depois um admin inicia o draft.\nHiddenking: {}\nArchmother: {}",
                            open.row.slot,
                            sync::side_mentions(&open.players, "hiddenking"),
                            sync::side_mentions(&open.players, "archmother")
                        ),
                    )
                    .await;
            }
            Ok(())
        }
        Err(err) => ephemeral(ctx, interaction, err.to_string()).await,
    }
}

async fn create_final_match(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    guild_id: GuildId,
    camp_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(ctx, interaction, "Apenas quem gerencia o servidor pode iniciar a final.").await;
    }
    match db::create_final_match(&data.db, camp_id, guild_id.get() as i64).await {
        Ok((camp_id, match_id)) => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, &camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp não encontrado"))?;
            let channel = ChannelId::new(state.camp.channel_id as u64);
            sync::redraw_all(ctx, &data.db, &state).await?;
            if let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) {
                let _ = channel
                    .say(
                        &ctx.http,
                        format!(
                            "Partida final criada com os jogadores de maior pontuação. Votem para atualizar os times; depois um admin inicia o draft.\nHiddenking: {}\nArchmother: {}",
                            sync::side_mentions(&open.players, "hiddenking"),
                            sync::side_mentions(&open.players, "archmother")
                        ),
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
                sync::redraw_all(ctx, &data.db, &state).await?;
                let _ = interaction
                    .create_followup(
                        &ctx.http,
                        serenity::all::CreateInteractionResponseFollowup::new()
                            .content(format!(
                                "Voto registrado. Atualizar Hiddenking {hk}/{needed} · Archmother {am}/{needed} · Manter times {keep} · Votos {voted}/{total}"
                            ))
                            .ephemeral(true),
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
            sync::redraw_all(ctx, &data.db, &state).await?;
            let _ = channel
                .say(
                    &ctx.http,
                    format!(
                        "Votação encerrada — times mantidos ({voted}/{total}). Um admin pode iniciar a partida."
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
            sync::redraw_all(ctx, &data.db, &state).await?;
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

async fn start_draft(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    guild_id: GuildId,
    camp_id: &str,
    match_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(
            ctx,
            interaction,
            "Apenas quem gerencia o servidor pode iniciar o draft.",
        )
        .await;
    }
    let match_id = match_id.trim().to_uppercase();
    let before = db::load_camp(&data.db, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    let Some(open) = before.open_matches.iter().find(|m| m.row.id == match_id) else {
        return ephemeral(ctx, interaction, "Partida não encontrada.").await;
    };
    if !open.row.refresh_used {
        return ephemeral(
            ctx,
            interaction,
            "Espere a votação de Refresh terminar antes do draft.",
        )
        .await;
    }
    if open.row.draft_url.is_some() {
        return ephemeral(ctx, interaction, "O draft desta partida já foi iniciado.").await;
    }

    let draft = match crate::draft::create_draft(
        &data.http,
        &data.config.draft_base_url,
        before.camp.layout().match_size,
    )
    .await
    {
        Ok(draft) => draft,
        Err(err) => return ephemeral(ctx, interaction, err.to_string()).await,
    };
    if let Err(err) = db::set_draft(&data.db, &match_id, &draft).await {
        return ephemeral(ctx, interaction, err.to_string()).await;
    }

    ack(ctx, interaction).await?;
    let state = db::load_camp(&data.db, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
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
        let _ = channel
            .say(
                &ctx.http,
                format!(
                    "Draft criado. Cada capitão ({}) recebeu por DM o link privado do seu time.\nHiddenking: {}\nArchmother: {}\nAssistir ao draft: {}",
                    crate::ui::CAPTAIN_TAG,
                    sync::side_mentions(&open.players, "hiddenking"),
                    sync::side_mentions(&open.players, "archmother"),
                    draft.spectator_url
                ),
            )
            .await;
        sync::send_draft_to_captains(ctx, channel, &state.camp.id, open).await;
    }
    Ok(())
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

async fn start_match(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    camp_id: &str,
    match_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(ctx, interaction, "Apenas quem gerencia o servidor pode iniciar uma partida.").await;
    }
    let match_id = match_id.trim().to_uppercase();
    let before = db::load_camp(&data.db, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    let Some(open) = before.open_matches.iter().find(|m| m.row.id == match_id) else {
        return ephemeral(ctx, interaction, "Partida não encontrada.").await;
    };
    if open.row.draft_url.is_none() {
        return ephemeral(
            ctx,
            interaction,
            "Inicie o draft antes de começar a partida.",
        )
        .await;
    }
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
                            .content(format!("Falha ao criar o lobby do Deadlock: {err}"))
                            .ephemeral(true),
                    )
                    .await;
                return Ok(());
            }
        }
    } else {
        (None, None)
    };

    db::start_live_match(&data.db, &match_id, party_id, party_code.clone()).await?;
    let state = db::load_camp(&data.db, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
    let channel = ChannelId::new(state.camp.channel_id as u64);
    sync::redraw_all(ctx, &data.db, &state).await?;
    let message = if let Some(code) = party_code {
        format!("Lobby da partida criado. **Código da party:** `{code}` — escolham os heróis no Deadlock.")
    } else {
        "A partida está ao vivo. Espere o admin enviar o código da partida.".to_string()
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
        return ephemeral(ctx, interaction, "Apenas quem gerencia o servidor pode informar o vencedor.").await;
    }
    let match_id = match_id.trim().to_uppercase();
    let before = db::load_camp(&data.db, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))?;
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

    match db::report_winner(&data.db, camp_id, &match_id, side).await {
        Ok(()) => {
            ack(ctx, interaction).await?;
            let channel = ChannelId::new(before.camp.channel_id as u64);
            let finished = before
                .open_matches
                .iter()
                .find(|m| m.row.id == match_id);
            let is_final = finished.map(|m| m.row.is_final).unwrap_or(false);
            let party_code = finished.and_then(|m| m.row.party_code.clone());
            if let Some(msg_id) = old_message {
                let _ = channel
                    .edit_message(
                        &ctx.http,
                        msg_id as u64,
                        serenity::all::EditMessage::new()
                            .embed(crate::ui::completed_match_embed(
                                &before.camp.id,
                                &match_id,
                                side,
                                is_final,
                                party_code.as_deref(),
                            ))
                            .components(crate::ui::completed_match_components(
                                &before.camp.id,
                                &match_id,
                            )),
                    )
                    .await;
            }
            if let Ok(previous) =
                db::other_completed_message_ids(&data.db, camp_id, &match_id).await
            {
                for msg_id in previous {
                    let _ = channel
                        .edit_message(
                            &ctx.http,
                            msg_id as u64,
                            serenity::all::EditMessage::new().components(Vec::new()),
                        )
                        .await;
                }
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
                .ok_or_else(|| anyhow!("camp não encontrado"))?;
            sync::redraw_all(ctx, &data.db, &state).await?;
            let winner = crate::ui::winner_label(side);
            let _ = channel
                .say(
                    &ctx.http,
                    format!(
                        "{winner} vence. +1 ponto para esse time. Os jogadores voltaram para a fila — um admin clica em **Criar partida** quando quiser."
                    ),
                )
                .await;
            Ok(())
        }
        Err(err) => ephemeral(ctx, interaction, err.to_string()).await,
    }
}

async fn correct_win(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    camp_id: &str,
    match_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(
            ctx,
            interaction,
            "Apenas quem gerencia o servidor pode corrigir o resultado.",
        )
        .await;
    }
    let match_id = match_id.trim().to_uppercase();
    match db::correct_winner(&data.db, camp_id, &match_id).await {
        Ok(view) => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp não encontrado"))?;
            let channel = ChannelId::new(state.camp.channel_id as u64);
            let msg_id = interaction.message.id.get();
            let _ = channel
                .edit_message(
                    &ctx.http,
                    msg_id,
                    serenity::all::EditMessage::new()
                        .embed(crate::ui::completed_match_embed(
                            camp_id,
                            &match_id,
                            &view.winner_side,
                            view.is_final,
                            view.party_code.as_deref(),
                        ))
                        .components(crate::ui::completed_match_components(camp_id, &match_id)),
                )
                .await;
            sync::redraw_all(ctx, &data.db, &state).await?;
            let winner = crate::ui::winner_label(&view.winner_side);
            let _ = channel
                .say(
                    &ctx.http,
                    format!(
                        "Resultado corrigido: {winner} vence. Pontos e fila atualizados."
                    ),
                )
                .await;
            Ok(())
        }
        Err(err) => ephemeral(ctx, interaction, err.to_string()).await,
    }
}

async fn reopen_match(
    ctx: &Context,
    data: &Data,
    interaction: &ComponentInteraction,
    guild_id: GuildId,
    camp_id: &str,
    match_id: &str,
) -> Result<()> {
    if !is_admin(interaction) {
        return ephemeral(
            ctx,
            interaction,
            "Apenas quem gerencia o servidor pode reabrir a partida.",
        )
        .await;
    }
    let match_id = match_id.trim().to_uppercase();
    match db::reopen_match(&data.db, camp_id, &match_id).await {
        Ok(()) => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp não encontrado"))?;
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
                    "Partida reaberta. Jogadores voltam aos canais de time.",
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
        return ephemeral(ctx, interaction, "Apenas quem gerencia o servidor pode encerrar o camp.").await;
    }
    match db::end_camp(&data.db, camp_id).await {
        Ok(players) => {
            ack(ctx, interaction).await?;
            let state = db::load_camp(&data.db, camp_id)
                .await?
                .ok_or_else(|| anyhow!("camp não encontrado"))?;
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
