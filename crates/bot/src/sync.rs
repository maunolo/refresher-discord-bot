use anyhow::Result;
use serenity::all::{
    ChannelId, CreateMessage, EditMessage, GuildId, Mentionable, UserId, VoiceState,
};
use sqlx::SqlitePool;

use crate::db::{self, CampState, MatchPlayerRow, OpenMatch, RefreshChange};
use crate::discord::Discord;
use crate::settings::GuildSettings;
use crate::state::{with_guild_lock, Data};
use crate::voice::{self, VoiceDest};

pub async fn redraw_board(discord: &Discord, pool: &SqlitePool, state: &CampState) -> Result<()> {
    let channel = ChannelId::new(state.camp.channel_id as u64);
    let embed = if state.camp.status == "signup" {
        crate::ui::signup_embed(state)
    } else {
        crate::ui::board_embed(state)
    };
    if let Some(id) = state.camp.board_message_id {
        // No components: this also clears the admin buttons older versions put here.
        if channel
            .edit_message(
                &discord.http,
                id as u64,
                EditMessage::new().embed(embed.clone()).components(Vec::new()),
            )
            .await
            .is_ok()
        {
            return Ok(());
        }
    }
    let msg = channel
        .send_message(&discord.http, CreateMessage::new().embed(embed))
        .await?;
    db::set_board_message_id(pool, &state.camp.id, msg.id.get() as i64).await?;
    Ok(())
}

pub async fn redraw_match(
    discord: &Discord,
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
                &discord.http,
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
            &discord.http,
            CreateMessage::new().embed(embed).components(components),
        )
        .await?;
    db::set_match_message_id(pool, &open.row.id, msg.id.get() as i64).await?;
    Ok(())
}

pub async fn redraw_all(discord: &Discord, pool: &SqlitePool, state: &CampState) -> Result<()> {
    redraw_board(discord, pool, state).await?;
    let fresh = db::load_camp(pool, &state.camp.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("camp não encontrado"))?;
    for open in &fresh.open_matches {
        redraw_match(discord, pool, &fresh, open).await?;
    }
    Ok(())
}

pub async fn restore_messages(discord: &Discord, pool: &SqlitePool) -> Result<()> {
    let camps = match db::load_restore_camps(pool).await {
        Ok(c) => c,
        Err(err) => {
            tracing::error!(error = %err, "skipping camp restore due to inconsistent data");
            return Ok(());
        }
    };
    for state in camps {
        if let Err(err) = restore_one(discord, pool, &state).await {
            tracing::error!(camp = %state.camp.id, error = %err, "failed to restore camp messages");
        }
    }
    Ok(())
}

async fn restore_one(discord: &Discord, pool: &SqlitePool, state: &CampState) -> Result<()> {
    redraw_board(discord, pool, state).await?;
    let fresh = db::load_camp(pool, &state.camp.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("camp não encontrado após restaurar"))?;
    for open in &fresh.open_matches {
        redraw_match(discord, pool, &fresh, open).await?;
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
    discord: &Discord,
    guild_id: GuildId,
    settings: &GuildSettings,
    channel: ChannelId,
    moves: &[(UserId, VoiceDest)],
) {
    let skipped = voice::move_players(discord, guild_id, settings, moves).await;
    if let Some(text) = voice::skipped_message(discord, guild_id, settings, &skipped).await {
        let _ = channel.say(&discord.http, text).await;
    }
}

/// Posts a refresh's new teams, `headline` first, and who swapped places with the queue. Then
/// moves the match's players to their team channels and whoever lost their seat back to Queue.
pub async fn announce_refresh(
    discord: &Discord,
    data: &Data,
    guild_id: GuildId,
    state: &CampState,
    open: &OpenMatch,
    change: &RefreshChange,
    headline: &str,
) -> Result<()> {
    let channel = ChannelId::new(state.camp.channel_id as u64);
    let mut text = format!(
        "{headline}\nHiddenking: {}\nArchmother: {}",
        side_mentions(&open.players, "hiddenking"),
        side_mentions(&open.players, "archmother")
    );
    if !change.joined.is_empty() {
        text.push_str(&format!(
            "\nEntraram da fila: {}\nVoltaram para o começo da fila: {}",
            mention_list(&change.joined),
            mention_list(&change.left)
        ));
    }
    let _ = channel.say(&discord.http, text).await;
    let settings = data.settings(guild_id.get()).await?;
    let mut moves = match_voice_moves(open);
    moves.extend(queue_voice_moves(&change.left));
    apply_moves(discord, guild_id, &settings, channel, &moves).await;
    Ok(())
}

pub fn mention_list(ids: &[i64]) -> String {
    ids.iter()
        .map(|id| UserId::new(*id as u64).mention().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// One team's mentions on a single line, captain first.
pub fn side_mentions(players: &[MatchPlayerRow], side: &str) -> String {
    let mut team: Vec<&MatchPlayerRow> = players.iter().filter(|p| p.side == side).collect();
    team.sort_by_key(|p| !p.is_captain);
    team.iter()
        .map(|p| {
            let mention = UserId::new(p.user_id as u64).mention().to_string();
            if p.is_captain {
                format!("{mention} {}", crate::ui::CAPTAIN_TAG)
            } else {
                mention
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// DM each captain the draft's invite link, naming the side they take in the lobby. A team can
/// have several captains (all its S-rank players). The link is never posted in the channel: it
/// lets anyone take a slot, so a captain who can't be DM'd is pointed at the captain-only button
/// on the match message.
pub async fn send_draft_to_captains(
    discord: &Discord,
    channel: ChannelId,
    camp_id: &str,
    open: &OpenMatch,
) {
    let match_name = if open.row.is_final {
        "partida final".to_string()
    } else {
        format!("partida {}", open.row.slot)
    };
    let Some(join_url) = &open.row.draft_join_url else {
        return;
    };
    for captain in open.players.iter().filter(|p| p.is_captain) {
        let team = crate::ui::winner_label(&captain.side);
        let user = UserId::new(captain.user_id as u64);
        let co_captains: Vec<String> = open
            .players
            .iter()
            .filter(|p| p.is_captain && p.side == captain.side && p.user_id != captain.user_id)
            .map(|p| UserId::new(p.user_id as u64).mention().to_string())
            .collect();
        let shared = if co_captains.is_empty() {
            String::new()
        } else {
            format!(
                "\nO time tem mais capitães: {}. Combinem quem escolhe e bane.",
                co_captains.join(", ")
            )
        };
        let text = format!(
            "Você é capitão do **{team}** na {match_name} (ID `{}`) do camp `{camp_id}`.{shared}\nAbra o link do draft e entre no lado **{team}**. Não compartilhe: quem tiver o link pode ocupar uma vaga no draft.\n{join_url}",
            open.row.id
        );
        if let Err(err) = user
            .direct_message((&discord.cache, &*discord.http), CreateMessage::new().content(text))
            .await
        {
            tracing::warn!(%user, error = %err, "failed to DM draft link to captain");
            let _ = channel
                .say(
                    &discord.http,
                    format!(
                        "{} não consegui te mandar DM. Clique em **{}** na mensagem da partida para ver o link do **{team}**.",
                        user.mention(),
                        crate::ui::CAPTAIN_LINK_LABEL
                    ),
                )
                .await;
        }
    }
}

/// Whether someone joined or left the server's Queue voice channel, which the admin page lists
/// during signup.
pub async fn touches_queue_voice(
    discord: &Discord,
    data: &Data,
    old: Option<&VoiceState>,
    new: &VoiceState,
) -> bool {
    let Some(guild_id) = new.guild_id else {
        return false;
    };
    let before = old.and_then(|state| state.channel_id);
    if before == new.channel_id {
        return false;
    }
    let Ok(settings) = data.settings(guild_id.get()).await else {
        return false;
    };
    let queue = voice::find_channel(discord, guild_id, VoiceDest::Queue, &settings);
    queue.is_some() && (before == queue || new.channel_id == queue)
}

/// A player of an open match who joins voice, or switches channels, goes to their team channel:
/// teams talk there from the match's creation until its result.
pub async fn place_late_voice_join(
    discord: &Discord,
    data: &Data,
    old: Option<&VoiceState>,
    new: &VoiceState,
) -> Result<()> {
    let Some(guild_id) = new.guild_id else {
        return Ok(());
    };
    let Some(current) = new.channel_id else {
        return Ok(());
    };
    if old.and_then(|state| state.channel_id) == Some(current) {
        return Ok(());
    }

    with_guild_lock(data, guild_id.get(), || async {
        let Some(state) = db::load_active_for_guild(&data.db, guild_id.get() as i64).await? else {
            return Ok(());
        };
        let user_id = new.user_id.get() as i64;
        let Some(open) = state
            .open_matches
            .iter()
            .find(|m| m.players.iter().any(|p| p.user_id == user_id))
        else {
            return Ok(());
        };
        let Some(player) = open.players.iter().find(|p| p.user_id == user_id) else {
            return Ok(());
        };
        let settings = data.settings(guild_id.get()).await?;
        voice::move_to_dest(
            discord,
            guild_id,
            &settings,
            new.user_id,
            VoiceDest::Match {
                slot: open.row.slot,
                hiddenking: player.side == "hiddenking",
            },
            current,
        )
        .await;
        Ok(())
    })
    .await
}
