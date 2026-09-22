use anyhow::Result;
use serenity::all::{
    ChannelId, ChannelType, Context, CreateChannel, GuildId, Mentionable, UserId,
};

use crate::config::Config;

#[derive(Clone, Copy)]
pub enum VoiceDest {
    Queue,
    Match { slot: i64, hiddenking: bool },
}

impl VoiceDest {
    fn names(self) -> &'static [&'static str] {
        match self {
            VoiceDest::Queue => &["Queue"],
            VoiceDest::Match {
                slot: 1,
                hiddenking: true,
            } => &["Match 1 Hiddenking", "Match1 Hiddenking"],
            VoiceDest::Match {
                slot: 1,
                hiddenking: false,
            } => &["Match 1 Archmother", "Match1 Archmother"],
            VoiceDest::Match {
                slot: 2,
                hiddenking: true,
            } => &["Match 2 Hiddenking", "Match2 Hiddenking"],
            VoiceDest::Match {
                slot: 2,
                hiddenking: false,
            } => &["Match 2 Archmother", "Match2 Archmother"],
            VoiceDest::Match { .. } => &["Queue"],
        }
    }

    fn configured_id(self, config: &Config) -> Option<ChannelId> {
        match self {
            VoiceDest::Queue => config.queue_voice_channel_id,
            VoiceDest::Match {
                slot: 1,
                hiddenking: true,
            } => config.match1_hiddenking_voice_id,
            VoiceDest::Match {
                slot: 1,
                hiddenking: false,
            } => config.match1_archmother_voice_id,
            VoiceDest::Match {
                slot: 2,
                hiddenking: true,
            } => config.match2_hiddenking_voice_id,
            VoiceDest::Match {
                slot: 2,
                hiddenking: false,
            } => config.match2_archmother_voice_id,
            VoiceDest::Match { .. } => None,
        }
    }
}

async fn resolve_channel(
    ctx: &Context,
    guild_id: GuildId,
    dest: VoiceDest,
    config: &Config,
) -> Result<ChannelId> {
    if let Some(id) = dest.configured_id(config) {
        return Ok(id);
    }
    let channels = guild_id.channels(&ctx.http).await?;
    for name in dest.names() {
        if let Some((id, _)) = channels.iter().find(|(_, ch)| ch.name.eq_ignore_ascii_case(name))
        {
            return Ok(*id);
        }
    }
    let created = guild_id
        .create_channel(
            &ctx.http,
            CreateChannel::new(dest.names()[0]).kind(ChannelType::Voice),
        )
        .await?;
    Ok(created.id)
}

/// Move players who are already in a voice channel. Returns user IDs that could not be moved.
pub async fn move_players(
    ctx: &Context,
    guild_id: GuildId,
    config: &Config,
    moves: &[(UserId, VoiceDest)],
) -> Vec<UserId> {
    let mut skipped = Vec::new();
    for (user_id, dest) in moves {
        let in_voice = ctx
            .cache
            .guild(guild_id)
            .map(|g| g.voice_states.contains_key(user_id))
            .unwrap_or(false);
        if !in_voice {
            skipped.push(*user_id);
            continue;
        }
        let Ok(channel_id) = resolve_channel(ctx, guild_id, *dest, config).await else {
            skipped.push(*user_id);
            continue;
        };
        if let Err(err) = guild_id.move_member(&ctx.http, *user_id, channel_id).await {
            tracing::warn!(%user_id, error = %err, "failed to move member");
            skipped.push(*user_id);
        }
    }
    skipped
}

pub async fn skipped_message(
    ctx: &Context,
    guild_id: GuildId,
    config: &Config,
    skipped: &[UserId],
) -> Option<String> {
    if skipped.is_empty() {
        return None;
    }
    let mentions = skipped
        .iter()
        .map(|id| id.mention().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let queue = match resolve_channel(ctx, guild_id, VoiceDest::Queue, config).await {
        Ok(id) => id.mention().to_string(),
        Err(_) => "#Fila".to_string(),
    };
    Some(format!(
        "Não foi possível mover {mentions} — entre no canal de voz {queue} e o bot te posiciona."
    ))
}

/// Move one member if they are not already in the destination channel.
pub async fn move_to_dest(
    ctx: &Context,
    guild_id: GuildId,
    config: &Config,
    user_id: UserId,
    dest: VoiceDest,
    current: ChannelId,
) {
    let Ok(dest_id) = resolve_channel(ctx, guild_id, dest, config).await else {
        return;
    };
    if dest_id == current {
        return;
    }
    if let Err(err) = guild_id.move_member(&ctx.http, user_id, dest_id).await {
        tracing::warn!(%user_id, error = %err, "failed to move late-joining member");
    }
}
