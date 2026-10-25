use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use poise::serenity_prelude as serenity;
use refresher_core::link::{BotLink, BoxFuture, CampAction, Channel, Profile};
use refresher_core::queue::CampSize;
use serenity::all::{ChannelType, GuildId, Member, UserId};

use crate::actions;
use crate::discord::Discord;
use crate::state::{with_guild_lock, Data};
use crate::voice::{self, VoiceDest};

/// Names change rarely; this spares Discord thirty member lookups on every page refresh.
const PROFILE_TTL: Duration = Duration::from_secs(10 * 60);

/// Answers the web admin through the bot's Discord connection.
pub struct SerenityLink {
    discord: Discord,
    data: Data,
    profiles: Mutex<HashMap<(u64, u64), (Profile, Instant)>>,
}

impl SerenityLink {
    pub fn new(discord: Discord, data: Data) -> Self {
        Self {
            discord,
            data,
            profiles: Mutex::new(HashMap::new()),
        }
    }

    fn remember(&self, guild_id: u64, user_id: u64, profile: &Profile) {
        self.profiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert((guild_id, user_id), (profile.clone(), Instant::now()));
    }

    async fn fetch(&self, guild_id: u64, user_id: u64) -> Option<Profile> {
        let user = UserId::new(user_id);
        match GuildId::new(guild_id)
            .member((&self.discord.cache, &*self.discord.http), user)
            .await
        {
            Ok(member) => Some(member_profile(&member)),
            // No longer in the server: fall back to the account itself.
            Err(_) => self.discord.http.get_user(user).await.ok().map(|user| Profile {
                name: user.display_name().to_string(),
                avatar_url: user.face(),
            }),
        }
    }
}

/// A server's channels of one kind, in Discord's order.
fn channels_of(discord: &Discord, guild_id: u64, kind: ChannelType) -> Vec<Channel> {
    let Some(guild) = discord.cache.guild(GuildId::new(guild_id)) else {
        return Vec::new();
    };
    let mut channels: Vec<_> = guild
        .channels
        .values()
        .filter(|c| c.kind == kind)
        .map(|c| (c.position, c.id.get(), c.name.clone()))
        .collect();
    channels.sort();
    channels
        .into_iter()
        .map(|(_, id, name)| Channel { id, name })
        .collect()
}

fn member_profile(member: &Member) -> Profile {
    Profile {
        name: member.display_name().to_string(),
        avatar_url: member.face(),
    }
}

impl BotLink for SerenityLink {
    fn in_guild(&self, guild_id: u64) -> bool {
        self.discord.cache.guild(GuildId::new(guild_id)).is_some()
    }

    fn profiles<'a>(
        &'a self,
        guild_id: u64,
        user_ids: &'a [u64],
    ) -> BoxFuture<'a, HashMap<u64, Profile>> {
        Box::pin(async move {
            let mut found = HashMap::new();
            let mut missing = Vec::new();
            {
                let cached = self.profiles.lock().unwrap_or_else(PoisonError::into_inner);
                for id in user_ids {
                    match cached.get(&(guild_id, *id)) {
                        Some((profile, at)) if at.elapsed() < PROFILE_TTL => {
                            found.insert(*id, profile.clone());
                        }
                        _ => missing.push(*id),
                    }
                }
            }
            let fetched = serenity::futures::future::join_all(
                missing.iter().map(|id| self.fetch(guild_id, *id)),
            )
            .await;
            for (id, profile) in missing.into_iter().zip(fetched) {
                if let Some(profile) = profile {
                    self.remember(guild_id, id, &profile);
                    found.insert(id, profile);
                }
            }
            found
        })
    }

    fn text_channels(&self, guild_id: u64) -> Vec<Channel> {
        channels_of(&self.discord, guild_id, ChannelType::Text)
    }

    fn voice_channels(&self, guild_id: u64) -> Vec<Channel> {
        channels_of(&self.discord, guild_id, ChannelType::Voice)
    }

    fn search_members<'a>(
        &'a self,
        guild_id: u64,
        query: &'a str,
        limit: u64,
    ) -> BoxFuture<'a, anyhow::Result<Vec<(u64, Profile)>>> {
        Box::pin(async move {
            let members = GuildId::new(guild_id)
                .search_members(&self.discord.http, query, Some(limit))
                .await?;
            Ok(members
                .iter()
                .filter(|m| !m.user.bot)
                .map(|m| {
                    let profile = member_profile(m);
                    self.remember(guild_id, m.user.id.get(), &profile);
                    (m.user.id.get(), profile)
                })
                .collect())
        })
    }

    fn queue_voice_members(&self, guild_id: u64) -> BoxFuture<'_, Vec<u64>> {
        Box::pin(async move {
            let settings = match self.data.settings(guild_id).await {
                Ok(settings) => settings,
                Err(err) => {
                    tracing::warn!(guild_id, error = %err, "failed to load server settings");
                    return Vec::new();
                }
            };
            // The same channel the bot moves waiting players to.
            let id = GuildId::new(guild_id);
            let Some(queue) = voice::find_channel(&self.discord, id, VoiceDest::Queue, &settings) else {
                return Vec::new();
            };
            let Some(guild) = self.discord.cache.guild(id) else {
                return Vec::new();
            };
            guild
                .voice_states
                .values()
                .filter(|v| v.channel_id == Some(queue))
                .map(|v| v.user_id.get())
                .collect()
        })
    }

    fn create_camp<'a>(
        &'a self,
        guild_id: u64,
        channel_id: u64,
        size: CampSize,
    ) -> BoxFuture<'a, anyhow::Result<String>> {
        Box::pin(async move {
            with_guild_lock(&self.data, guild_id, || {
                actions::create_camp(&self.discord, &self.data, GuildId::new(guild_id), channel_id, size)
            })
            .await
        })
    }

    fn act<'a>(
        &'a self,
        guild_id: u64,
        camp_id: &'a str,
        action: CampAction,
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            // The same lock the Discord buttons take, so a web click and a button can't interleave.
            with_guild_lock(&self.data, guild_id, || {
                actions::run(&self.discord, &self.data, GuildId::new(guild_id), camp_id, &action)
            })
            .await
        })
    }
}
