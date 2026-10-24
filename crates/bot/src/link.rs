use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use poise::serenity_prelude as serenity;
use refresher_core::link::{BotLink, BoxFuture, CampAction, Profile};
use serenity::all::{GuildId, UserId};

use crate::actions;
use crate::discord::Discord;
use crate::state::{with_guild_lock, Data};

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

    async fn fetch(&self, guild_id: u64, user_id: u64) -> Option<Profile> {
        let user = UserId::new(user_id);
        match GuildId::new(guild_id)
            .member((&self.discord.cache, &*self.discord.http), user)
            .await
        {
            Ok(member) => Some(Profile {
                name: member.display_name().to_string(),
                avatar_url: member.face(),
            }),
            // No longer in the server: fall back to the account itself.
            Err(_) => self.discord.http.get_user(user).await.ok().map(|user| Profile {
                name: user.display_name().to_string(),
                avatar_url: user.face(),
            }),
        }
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
            let mut cached = self.profiles.lock().unwrap_or_else(PoisonError::into_inner);
            for (id, profile) in missing.into_iter().zip(fetched) {
                if let Some(profile) = profile {
                    cached.insert((guild_id, id), (profile.clone(), Instant::now()));
                    found.insert(id, profile);
                }
            }
            found
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
