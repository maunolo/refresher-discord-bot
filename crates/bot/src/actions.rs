//! Admin actions shared by the Discord buttons and the web admin. Callers check that the user
//! manages the server and hold its lock. Errors are messages for the admin.

use anyhow::{anyhow, bail, Result};
use refresher_core::link::CampAction;
use refresher_core::queue::CampSize;
use serenity::all::{ChannelId, CreateMessage, EditMessage, GuildId};

use crate::db::{self, CampState};
use crate::discord::Discord;
use crate::state::Data;
use crate::{deadlock, sync, ui};

pub async fn run(
    discord: &Discord,
    data: &Data,
    guild_id: GuildId,
    camp_id: &str,
    action: &CampAction,
) -> Result<()> {
    let camp = load(data, camp_id).await?;
    if camp.camp.guild_id != guild_id.get() as i64 {
        bail!("camp não encontrado");
    }
    let camp_id = camp.camp.id.as_str();
    match action {
        CampAction::CreateMatch => create_match(discord, data, guild_id, camp_id).await,
        CampAction::CreateFinal => create_final(discord, data, guild_id, camp_id).await,
        CampAction::CloseVote { match_id } => close_vote(discord, data, camp_id, &normalize(match_id)).await,
        CampAction::StartDraft { match_id } => {
            start_draft(discord, data, guild_id, camp_id, &normalize(match_id)).await
        }
        CampAction::StartMatch { match_id } => {
            start_match(discord, data, camp_id, &normalize(match_id)).await
        }
        CampAction::ReportWinner { match_id, side } => {
            report_winner(discord, data, guild_id, camp_id, &normalize(match_id), side).await
        }
        CampAction::CorrectWinner { match_id } => {
            correct_winner(discord, data, guild_id, camp_id, &normalize(match_id)).await
        }
        CampAction::ReopenMatch { match_id } => {
            reopen_match(discord, data, guild_id, camp_id, &normalize(match_id)).await
        }
        CampAction::AddPlayers { user_ids } => {
            let ids: Vec<i64> = user_ids.iter().map(|id| *id as i64).collect();
            db::add_players(&data.db, camp_id, &ids).await?;
            redraw_board(discord, data, camp_id).await
        }
        CampAction::RemovePlayer { user_id } => {
            db::remove_player(&data.db, camp_id, *user_id as i64).await?;
            redraw_board(discord, data, camp_id).await
        }
        CampAction::SetSRank { user_id, s_rank } => {
            db::set_player_s_rank(&data.db, camp_id, *user_id as i64, *s_rank).await?;
            redraw_board(discord, data, camp_id).await
        }
        CampAction::StartQueue => start_queue(discord, data, guild_id, camp_id).await,
        CampAction::CancelCamp => cancel_camp(discord, data, camp_id).await,
        CampAction::ResumeCamp => resume_camp(discord, data, guild_id, camp_id).await,
        CampAction::EndCamp => end_camp(discord, data, camp_id).await,
    }
}

/// Opens signup for a new camp and posts its message in `channel_id`. Returns the camp ID.
pub async fn create_camp(
    discord: &Discord,
    data: &Data,
    guild_id: GuildId,
    channel_id: u64,
    size: CampSize,
) -> Result<String> {
    let channel_here = discord
        .cache
        .guild(guild_id)
        .map(|g| g.channels.contains_key(&ChannelId::new(channel_id)))
        .unwrap_or(false);
    if !channel_here {
        bail!("Esse canal não é deste servidor.");
    }
    if db::load_active_for_guild(&data.db, guild_id.get() as i64).await?.is_some() {
        bail!("Este servidor já tem um camp ativo. Cancele ou encerre o atual primeiro.");
    }
    let camp = db::create_camp(
        &data.db,
        guild_id.get() as i64,
        channel_id as i64,
        size.target_size(),
        size.match_size(),
    )
    .await?;
    redraw_board(discord, data, &camp.id).await?;
    Ok(camp.id)
}

async fn redraw_board(discord: &Discord, data: &Data, camp_id: &str) -> Result<()> {
    let state = load(data, camp_id).await?;
    sync::redraw_board(discord, &data.db, &state).await
}

async fn start_queue(discord: &Discord, data: &Data, guild_id: GuildId, camp_id: &str) -> Result<()> {
    let order = db::start_queue(&data.db, camp_id).await?;
    let state = load(data, camp_id).await?;
    let channel = channel(&state);
    let queued_ids: Vec<i64> = state.players.iter().map(|p| p.user_id).collect();
    sync::redraw_all(discord, &data.db, &state).await?;
    sync::apply_moves(discord, guild_id, &data.config, channel, &sync::queue_voice_moves(&queued_ids)).await;
    let _ = channel
        .say(
            &discord.http,
            format!(
                "Fila embaralhada do camp `{}`:\n{}\nOs times saem da fila quando um admin cria a partida.",
                state.camp.id,
                sync::mention_list(&order)
            ),
        )
        .await;
    Ok(())
}

async fn cancel_camp(discord: &Discord, data: &Data, camp_id: &str) -> Result<()> {
    db::cancel_camp(&data.db, camp_id).await?;
    let state = load(data, camp_id).await?;
    let _ = sync::redraw_board(discord, &data.db, &state).await;
    let _ = channel(&state)
        .say(
            &discord.http,
            format!("Camp `{camp_id}` cancelado. Elenco, fila e pontos ficam guardados para retomar depois."),
        )
        .await;
    Ok(())
}

async fn resume_camp(discord: &Discord, data: &Data, guild_id: GuildId, camp_id: &str) -> Result<()> {
    let state = db::resume_camp(&data.db, camp_id, guild_id.get() as i64).await?;
    sync::redraw_all(discord, &data.db, &state).await?;
    let _ = channel(&state)
        .say(&discord.http, format!("Camp `{camp_id}` retomado."))
        .await;
    Ok(())
}

async fn end_camp(discord: &Discord, data: &Data, camp_id: &str) -> Result<()> {
    let players = db::end_camp(&data.db, camp_id).await?;
    let state = load(data, camp_id).await?;
    let channel = channel(&state);
    let standings = || ui::standings_embed(camp_id, state.camp.match_count, &players);
    let _ = channel
        .send_message(&discord.http, CreateMessage::new().embed(standings()))
        .await;
    if let Some(board_id) = state.camp.board_message_id {
        let _ = channel
            .edit_message(
                &discord.http,
                board_id as u64,
                EditMessage::new().embed(standings()).components(Vec::new()),
            )
            .await;
    }
    Ok(())
}

fn normalize(match_id: &str) -> String {
    match_id.trim().to_uppercase()
}

async fn load(data: &Data, camp_id: &str) -> Result<CampState> {
    db::load_camp(&data.db, camp_id)
        .await?
        .ok_or_else(|| anyhow!("camp não encontrado"))
}

fn channel(state: &CampState) -> ChannelId {
    ChannelId::new(state.camp.channel_id as u64)
}

async fn create_match(discord: &Discord, data: &Data, guild_id: GuildId, camp_id: &str) -> Result<()> {
    let (camp_id, match_id) = db::create_match(&data.db, camp_id, guild_id.get() as i64).await?;
    let state = load(data, &camp_id).await?;
    sync::redraw_all(discord, &data.db, &state).await?;
    if let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) {
        let _ = channel(&state)
            .say(
                &discord.http,
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

async fn create_final(discord: &Discord, data: &Data, guild_id: GuildId, camp_id: &str) -> Result<()> {
    let (camp_id, match_id) = db::create_final_match(&data.db, camp_id, guild_id.get() as i64).await?;
    let state = load(data, &camp_id).await?;
    sync::redraw_all(discord, &data.db, &state).await?;
    if let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) {
        let _ = channel(&state)
            .say(
                &discord.http,
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

async fn close_vote(discord: &Discord, data: &Data, camp_id: &str, match_id: &str) -> Result<()> {
    db::close_voting(&data.db, camp_id, match_id).await?;
    let state = load(data, camp_id).await?;
    sync::redraw_all(discord, &data.db, &state).await?;
    let name = state
        .open_matches
        .iter()
        .find(|m| m.row.id == match_id)
        .map(|m| if m.row.is_final { "da partida final".to_string() } else { format!("da partida {}", m.row.slot) })
        .unwrap_or_else(|| "da partida".to_string());
    let _ = channel(&state)
        .say(
            &discord.http,
            format!("Votação {name} encerrada por um admin — times mantidos. O draft vem a seguir."),
        )
        .await;
    Ok(())
}

async fn start_draft(
    discord: &Discord,
    data: &Data,
    guild_id: GuildId,
    camp_id: &str,
    match_id: &str,
) -> Result<()> {
    let before = load(data, camp_id).await?;
    let Some(open) = before.open_matches.iter().find(|m| m.row.id == match_id) else {
        bail!("Partida não encontrada.");
    };
    if !open.row.refresh_used {
        bail!("Espere a votação de Refresh terminar antes do draft.");
    }
    if open.row.draft_url.is_some() {
        bail!("O draft desta partida já foi iniciado.");
    }

    let draft = crate::draft::create_draft(
        &data.http,
        &data.config.draft_base_url,
        before.camp.layout().match_size,
    )
    .await?;
    db::set_draft(&data.db, match_id, &draft).await?;

    let state = load(data, camp_id).await?;
    let channel = channel(&state);
    sync::redraw_all(discord, &data.db, &state).await?;
    if let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) {
        sync::apply_moves(discord, guild_id, &data.config, channel, &sync::match_voice_moves(open)).await;
        let _ = channel
            .say(
                &discord.http,
                format!(
                    "Draft criado. Cada capitão ({}) recebeu por DM o link privado do seu time.\nHiddenking: {}\nArchmother: {}\nAssistir ao draft: {}",
                    ui::CAPTAIN_TAG,
                    sync::side_mentions(&open.players, "hiddenking"),
                    sync::side_mentions(&open.players, "archmother"),
                    draft.spectator_url
                ),
            )
            .await;
        sync::send_draft_to_captains(discord, channel, &state.camp.id, open).await;
    }
    Ok(())
}

async fn start_match(discord: &Discord, data: &Data, camp_id: &str, match_id: &str) -> Result<()> {
    let before = load(data, camp_id).await?;
    let Some(open) = before.open_matches.iter().find(|m| m.row.id == match_id) else {
        bail!("Partida não encontrada.");
    };
    if open.row.draft_url.is_none() {
        bail!("Inicie o draft antes de começar a partida.");
    }

    let (party_id, party_code) = if data.config.enable_custom_match_create {
        let (id, code) = deadlock::create_custom_match(
            &data.http,
            data.config.deadlock_api_key.as_deref(),
            data.config.deadlock_server_region.as_deref(),
        )
        .await
        .map_err(|err| anyhow!("Falha ao criar o lobby do Deadlock: {err}"))?;
        (Some(id), Some(code))
    } else {
        (None, None)
    };

    db::start_live_match(&data.db, match_id, party_id, party_code.clone()).await?;
    let state = load(data, camp_id).await?;
    sync::redraw_all(discord, &data.db, &state).await?;
    let message = if let Some(code) = party_code {
        format!("Lobby da partida criado. **Código da party:** `{code}` — escolham os heróis no Deadlock.")
    } else {
        "A partida está ao vivo. Espere o admin enviar o código da partida.".to_string()
    };
    let _ = channel(&state).say(&discord.http, message).await;
    Ok(())
}

async fn report_winner(
    discord: &Discord,
    data: &Data,
    guild_id: GuildId,
    camp_id: &str,
    match_id: &str,
    side: &str,
) -> Result<()> {
    if side != "hiddenking" && side != "archmother" {
        bail!("vencedor inválido");
    }
    let before = load(data, camp_id).await?;
    let finished = before.open_matches.iter().find(|m| m.row.id == match_id);
    let finished_ids: Vec<i64> = finished
        .map(|m| m.players.iter().map(|p| p.user_id).collect())
        .unwrap_or_default();
    let old_message = finished.and_then(|m| m.row.message_id);
    let is_final = finished.map(|m| m.row.is_final).unwrap_or(false);
    let party_code = finished.and_then(|m| m.row.party_code.clone());

    db::report_winner(&data.db, camp_id, match_id, side).await?;

    let channel = channel(&before);
    if let Some(msg_id) = old_message {
        let _ = channel
            .edit_message(
                &discord.http,
                msg_id as u64,
                EditMessage::new()
                    .embed(ui::completed_match_embed(camp_id, match_id, side, is_final, party_code.as_deref()))
                    .components(Vec::new()),
            )
            .await;
    }
    // Results have no buttons; this clears the correction buttons older versions left on them.
    if let Ok(previous) = db::other_completed_message_ids(&data.db, camp_id, match_id).await {
        for msg_id in previous {
            let _ = channel
                .edit_message(&discord.http, msg_id as u64, EditMessage::new().components(Vec::new()))
                .await;
        }
    }
    sync::apply_moves(discord, guild_id, &data.config, channel, &sync::queue_voice_moves(&finished_ids)).await;
    let state = load(data, camp_id).await?;
    sync::redraw_all(discord, &data.db, &state).await?;
    let winner = ui::winner_label(side);
    let _ = channel
        .say(
            &discord.http,
            format!(
                "{winner} vence. +1 ponto para esse time. Os jogadores voltaram para a fila."
            ),
        )
        .await;
    Ok(())
}

async fn correct_winner(
    discord: &Discord,
    data: &Data,
    guild_id: GuildId,
    camp_id: &str,
    match_id: &str,
) -> Result<()> {
    let view = db::correct_winner(&data.db, camp_id, match_id).await?;
    let message_id = db::load_match_info(&data.db, match_id, guild_id.get() as i64)
        .await?
        .row
        .message_id;
    let state = load(data, camp_id).await?;
    let channel = channel(&state);
    if let Some(msg_id) = message_id {
        let _ = channel
            .edit_message(
                &discord.http,
                msg_id as u64,
                EditMessage::new()
                    .embed(ui::completed_match_embed(
                        camp_id,
                        match_id,
                        &view.winner_side,
                        view.is_final,
                        view.party_code.as_deref(),
                    ))
                    .components(Vec::new()),
            )
            .await;
    }
    sync::redraw_all(discord, &data.db, &state).await?;
    let winner = ui::winner_label(&view.winner_side);
    let _ = channel
        .say(
            &discord.http,
            format!("Resultado corrigido: {winner} vence. Pontos e fila atualizados."),
        )
        .await;
    Ok(())
}

async fn reopen_match(
    discord: &Discord,
    data: &Data,
    guild_id: GuildId,
    camp_id: &str,
    match_id: &str,
) -> Result<()> {
    db::reopen_match(&data.db, camp_id, match_id).await?;
    let state = load(data, camp_id).await?;
    let channel = channel(&state);
    sync::redraw_all(discord, &data.db, &state).await?;
    if let Some(open) = state.open_matches.iter().find(|m| m.row.id == match_id) {
        sync::apply_moves(discord, guild_id, &data.config, channel, &sync::match_voice_moves(open)).await;
    }
    let _ = channel
        .say(&discord.http, "Partida reaberta. Jogadores voltam aos canais de time.")
        .await;
    Ok(())
}
