use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use poise::serenity_prelude as serenity;
use refresher_core::link::{BotLink, BoxFuture, CampAction, Changes, Channel, Profile};
use refresher_core::queue::CampSize;
use serenity::all::{ChannelType, GuildId, Member, UserId};
use serenity::http::StatusCode;

use crate::actions;
use crate::discord::Discord;
use crate::state::{with_guild_lock, Data};
use crate::voice::{self, VoiceDest};

/// Names change rarely; this spares Discord thirty member lookups on every page refresh.
const PROFILE_TTL: Duration = Duration::from_secs(10 * 60);

/// With this many names to look up, one list of the whole server (a request per 1000 members)
/// beats a request each, which Discord rate limits per route.
const LIST_FROM: usize = 3;

/// A page never waits longer than this on Discord for names; the ones still missing fall back.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(8);

/// The most members Discord returns per request.
const MEMBER_PAGE: u64 = 1000;

/// Answers the web admin through the bot's Discord connection.
pub struct SerenityLink {
    discord: Discord,
    data: Data,
    profiles: Mutex<HashMap<(u64, u64), (Profile, Instant)>>,
    /// Users Discord had no account for, so they aren't asked for again on every page.
    unknown: Mutex<HashMap<(u64, u64), Instant>>,
    /// One lookup of missing names at a time: pages asking at once wait for it, then read the cache.
    lookup: tokio::sync::Mutex<()>,
}

impl SerenityLink {
    pub fn new(discord: Discord, data: Data) -> Self {
        Self {
            discord,
            data,
            profiles: Mutex::new(HashMap::new()),
            unknown: Mutex::new(HashMap::new()),
            lookup: tokio::sync::Mutex::new(()),
        }
    }

    /// The fresh cached profiles of `user_ids`, and the ids still to look up.
    fn cached(&self, guild_id: u64, user_ids: &[u64]) -> (HashMap<u64, Profile>, Vec<u64>) {
        let cached = self.profiles.lock().unwrap_or_else(PoisonError::into_inner);
        let unknown = self.unknown.lock().unwrap_or_else(PoisonError::into_inner);
        let mut found = HashMap::new();
        let mut missing = Vec::new();
        for id in user_ids {
            match cached.get(&(guild_id, *id)) {
                Some((profile, at)) if at.elapsed() < PROFILE_TTL => {
                    found.insert(*id, profile.clone());
                }
                _ if unknown.get(&(guild_id, *id)).is_some_and(|at| at.elapsed() < PROFILE_TTL) => {}
                _ => missing.push(*id),
            }
        }
        (found, missing)
    }

    /// Looks the missing names up: the whole member list if there are several, then one by one
    /// for those not in the server (anymore).
    async fn look_up(&self, guild_id: u64, user_ids: &[u64]) {
        let _turn = self.lookup.lock().await;
        // Whoever looked up just before may have found them.
        let (_, missing) = self.cached(guild_id, user_ids);
        if missing.len() >= LIST_FROM {
            if let Err(err) = self.members(guild_id).await {
                tracing::warn!(guild_id, error = %err, "member list unavailable; looking names up one by one");
            }
        }
        let (_, missing) = self.cached(guild_id, &missing);
        for id in missing {
            match self.fetch(guild_id, id).await {
                Some(profile) => self.remember(guild_id, id, &profile),
                None => {
                    self.unknown
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .insert((guild_id, id), Instant::now());
                }
            }
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

    fn changes(&self) -> &Changes {
        &self.data.changes
    }

    fn profiles<'a>(
        &'a self,
        guild_id: u64,
        user_ids: &'a [u64],
    ) -> BoxFuture<'a, HashMap<u64, Profile>> {
        Box::pin(async move {
            let (found, missing) = self.cached(guild_id, user_ids);
            if missing.is_empty() {
                return found;
            }
            if tokio::time::timeout(LOOKUP_TIMEOUT, self.look_up(guild_id, &missing)).await.is_err() {
                tracing::warn!(guild_id, missing = missing.len(), "Discord name lookup timed out");
            }
            self.cached(guild_id, user_ids).0
        })
    }

    fn text_channels(&self, guild_id: u64) -> Vec<Channel> {
        channels_of(&self.discord, guild_id, ChannelType::Text)
    }

    fn voice_channels(&self, guild_id: u64) -> Vec<Channel> {
        channels_of(&self.discord, guild_id, ChannelType::Voice)
    }

    fn members(&self, guild_id: u64) -> BoxFuture<'_, anyhow::Result<Vec<(u64, Profile)>>> {
        Box::pin(async move {
            let mut found = Vec::new();
            let mut after = None;
            loop {
                // Discord pages members by user ID, at most 1000 at a time.
                let page = match GuildId::new(guild_id)
                    .members(&self.discord.http, Some(MEMBER_PAGE), after)
                    .await
                {
                    Ok(page) => page,
                    Err(serenity::Error::Http(err)) if err.status_code() == Some(StatusCode::FORBIDDEN) => {
                        anyhow::bail!(
                            "O Discord não deixou o bot listar os membros do servidor. Ative o Server Members Intent do bot no Discord Developer Portal."
                        )
                    }
                    Err(err) => return Err(err.into()),
                };
                after = page.last().map(|m| m.user.id);
                for m in page.iter().filter(|m| !m.user.bot) {
                    let profile = member_profile(m);
                    self.remember(guild_id, m.user.id.get(), &profile);
                    found.push((m.user.id.get(), profile));
                }
                if page.len() < MEMBER_PAGE as usize {
                    return Ok(found);
                }
            }
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
            let created = with_guild_lock(&self.data, guild_id, || {
                actions::create_camp(&self.discord, &self.data, GuildId::new(guild_id), channel_id, size)
            })
            .await;
            self.data.changes.notify(guild_id);
            created
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
            let result = with_guild_lock(&self.data, guild_id, || {
                actions::run(&self.discord, &self.data, GuildId::new(guild_id), camp_id, &action)
            })
            .await;
            // Also on errors: a failed Discord step can follow a database change.
            self.data.changes.notify(guild_id);
            result
        })
    }
}
