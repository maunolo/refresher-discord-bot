use anyhow::Result;
use serenity::all::{
    ChannelId, ChannelType, CreateChannel, GuildId, Mentionable, UserId,
};

use crate::discord::Discord;
use crate::settings::GuildSettings;

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

    fn configured_id(self, settings: &GuildSettings) -> Option<ChannelId> {
        let id = match self {
            VoiceDest::Queue => settings.queue_voice_id,
            VoiceDest::Match {
                slot: 1,
                hiddenking: true,
            } => settings.match1_hiddenking_voice_id,
            VoiceDest::Match {
                slot: 1,
                hiddenking: false,
            } => settings.match1_archmother_voice_id,
            VoiceDest::Match {
                slot: 2,
                hiddenking: true,
            } => settings.match2_hiddenking_voice_id,
            VoiceDest::Match {
                slot: 2,
                hiddenking: false,
            } => settings.match2_archmother_voice_id,
            VoiceDest::Match { .. } => None,
        };
        id.map(ChannelId::new)
    }
}

/// The voice channel for `dest`: the one picked in the server's settings while it still exists,
/// else the first channel with one of its names. `None` when the server has neither.
pub fn find_channel(
    discord: &Discord,
    guild_id: GuildId,
    dest: VoiceDest,
    settings: &GuildSettings,
) -> Option<ChannelId> {
    let guild = discord.cache.guild(guild_id)?;
    if let Some(id) = dest.configured_id(settings) {
        if guild.channels.contains_key(&id) {
            return Some(id);
        }
    }
    dest.names().iter().find_map(|name| {
        guild
            .channels
            .values()
            .find(|ch| ch.kind == ChannelType::Voice && ch.name.eq_ignore_ascii_case(name))
            .map(|ch| ch.id)
    })
}

/// Like `find_channel`, creating the channel when the server has none for `dest`.
async fn resolve_channel(
    discord: &Discord,
    guild_id: GuildId,
    dest: VoiceDest,
    settings: &GuildSettings,
) -> Result<ChannelId> {
    if let Some(id) = find_channel(discord, guild_id, dest, settings) {
        return Ok(id);
    }
    let created = guild_id
        .create_channel(
            &discord.http,
            CreateChannel::new(dest.names()[0]).kind(ChannelType::Voice),
        )
        .await?;
    Ok(created.id)
}

/// Move players who are already in a voice channel. Returns user IDs that could not be moved.
pub async fn move_players(
    discord: &Discord,
    guild_id: GuildId,
    settings: &GuildSettings,
    moves: &[(UserId, VoiceDest)],
) -> Vec<UserId> {
    let mut skipped = Vec::new();
    for (user_id, dest) in moves {
        let in_voice = discord
            .cache
            .guild(guild_id)
            .map(|g| g.voice_states.contains_key(user_id))
            .unwrap_or(false);
        if !in_voice {
            skipped.push(*user_id);
            continue;
        }
        let Ok(channel_id) = resolve_channel(discord, guild_id, *dest, settings).await else {
            skipped.push(*user_id);
            continue;
        };
        if let Err(err) = guild_id.move_member(&discord.http, *user_id, channel_id).await {
            tracing::warn!(%user_id, error = %err, "failed to move member");
            skipped.push(*user_id);
        }
    }
    skipped
}

pub async fn skipped_message(
    discord: &Discord,
    guild_id: GuildId,
    settings: &GuildSettings,
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
    let queue = match resolve_channel(discord, guild_id, VoiceDest::Queue, settings).await {
        Ok(id) => id.mention().to_string(),
        Err(_) => "#Fila".to_string(),
    };
    Some(format!(
        "Não foi possível mover {mentions} — entre no canal de voz {queue} e o bot te posiciona."
    ))
}

/// Move one member if they are not already in the destination channel.
pub async fn move_to_dest(
    discord: &Discord,
    guild_id: GuildId,
    settings: &GuildSettings,
    user_id: UserId,
    dest: VoiceDest,
    current: ChannelId,
) {
    let Ok(dest_id) = resolve_channel(discord, guild_id, dest, settings).await else {
        return;
    };
    if dest_id == current {
        return;
    }
    if let Err(err) = guild_id.move_member(&discord.http, user_id, dest_id).await {
        tracing::warn!(%user_id, error = %err, "failed to move late-joining member");
    }
}
