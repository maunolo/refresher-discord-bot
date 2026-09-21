use anyhow::{anyhow, Result};
use poise::CreateReply;
use serenity::all::{GuildId, Mentionable, User};

use crate::db;
use crate::state::with_guild_lock;
use crate::sync;
use crate::Error;

type Context<'a> = poise::Context<'a, crate::Data, Error>;

fn is_admin(ctx: Context<'_>) -> bool {
    match ctx {
        poise::Context::Application(app) => app
            .interaction
            .member
            .as_ref()
            .and_then(|m| m.permissions)
            .map(|p| p.manage_guild())
            .unwrap_or(false),
        poise::Context::Prefix(_) => false,
    }
}

async fn deny_if_not_admin(ctx: Context<'_>) -> Result<bool> {
    if is_admin(ctx) {
        return Ok(true);
    }
    ctx.send(
        CreateReply::default()
            .content("Only server managers can do that.")
            .ephemeral(true),
    )
    .await?;
    Ok(false)
}

fn guild_id(ctx: Context<'_>) -> Result<GuildId> {
    ctx.guild_id()
        .ok_or_else(|| anyhow!("use this in a server"))
}

/// Deadlock camp queue, matches, and standings.
#[poise::command(
    slash_command,
    guild_only,
    subcommands("start", "add", "remove", "cancel", "end", "resume", "info")
)]
pub async fn camp(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Create a new camp roster (24, 30, or 4 for a 2v2 test).
#[poise::command(slash_command, guild_only)]
pub async fn start(
    ctx: Context<'_>,
    #[description = "Roster size (4 is a 2v2 test camp)"] size: CampSize,
) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    with_guild_lock(ctx.data(), guild_id.get(), || async {
        if db::load_active_for_guild(&ctx.data().db, guild_id.get() as i64)
            .await?
            .is_some()
        {
            ctx.send(
                CreateReply::default()
                    .content("This server already has an active camp. Cancel or end it first.")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
        let camp = db::create_camp(
            &ctx.data().db,
            guild_id.get() as i64,
            ctx.channel_id().get() as i64,
            size.as_i64(),
        )
        .await?;
        let state = db::load_camp(&ctx.data().db, &camp.id)
            .await?
            .ok_or_else(|| anyhow!("camp missing"))?;
        sync::redraw_board(ctx.serenity_context(), &ctx.data().db, &state).await?;
        ctx.send(
            CreateReply::default().content(format!(
                "Camp `{}` created for {} players. Add users, then Start queue.",
                camp.id,
                size.as_i64()
            )),
        )
        .await?;
        Ok(())
    })
    .await
}

/// Add Discord users to the camp roster.
#[poise::command(slash_command, guild_only)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "Player"] user: User,
    #[description = "Player"] user2: Option<User>,
    #[description = "Player"] user3: Option<User>,
    #[description = "Player"] user4: Option<User>,
    #[description = "Player"] user5: Option<User>,
) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    with_guild_lock(ctx.data(), guild_id.get(), || async {
        let camp = db::load_active_for_guild(&ctx.data().db, guild_id.get() as i64)
            .await?
            .ok_or_else(|| anyhow!("no active camp"))?;
        let mut ids = vec![user.id.get() as i64];
        for extra in [user2, user3, user4, user5].into_iter().flatten() {
            ids.push(extra.id.get() as i64);
        }
        let (added, skipped) = db::add_players(&ctx.data().db, &camp.camp.id, &ids).await?;
        let state = db::load_camp(&ctx.data().db, &camp.camp.id)
            .await?
            .ok_or_else(|| anyhow!("camp missing"))?;
        sync::redraw_board(ctx.serenity_context(), &ctx.data().db, &state).await?;
        ctx.send(
            CreateReply::default()
                .content(format!("Added {added}. Skipped {}.", skipped.len()))
                .ephemeral(true),
        )
        .await?;
        Ok(())
    })
    .await
}

/// Remove a Discord user from the camp roster.
#[poise::command(slash_command, guild_only)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Player"] user: User,
) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    with_guild_lock(ctx.data(), guild_id.get(), || async {
        let camp = db::load_active_for_guild(&ctx.data().db, guild_id.get() as i64)
            .await?
            .ok_or_else(|| anyhow!("no active camp"))?;
        db::remove_player(&ctx.data().db, &camp.camp.id, user.id.get() as i64).await?;
        let state = db::load_camp(&ctx.data().db, &camp.camp.id)
            .await?
            .ok_or_else(|| anyhow!("camp missing"))?;
        sync::redraw_board(ctx.serenity_context(), &ctx.data().db, &state).await?;
        ctx.send(
            CreateReply::default()
                .content(format!("Removed {}.", user.mention()))
                .ephemeral(true),
        )
        .await?;
        Ok(())
    })
    .await
}

/// Soft-cancel the active camp (can be resumed).
#[poise::command(slash_command, guild_only)]
pub async fn cancel(ctx: Context<'_>) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    with_guild_lock(ctx.data(), guild_id.get(), || async {
        let camp = db::load_active_for_guild(&ctx.data().db, guild_id.get() as i64)
            .await?
            .ok_or_else(|| anyhow!("no active camp"))?;
        db::cancel_camp(&ctx.data().db, &camp.camp.id).await?;
        ctx.send(
            CreateReply::default().content(format!(
                "Camp cancelled. Resume with `/camp resume {}`",
                camp.camp.id
            )),
        )
        .await?;
        Ok(())
    })
    .await
}

/// End the camp and post standings.
#[poise::command(slash_command, guild_only)]
pub async fn end(ctx: Context<'_>) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    with_guild_lock(ctx.data(), guild_id.get(), || async {
        let camp = db::load_active_for_guild(&ctx.data().db, guild_id.get() as i64)
            .await?
            .ok_or_else(|| anyhow!("no active camp"))?;
        let players = db::end_camp(&ctx.data().db, &camp.camp.id).await?;
        ctx.send(
            CreateReply::default().embed(crate::ui::standings_embed(
                &camp.camp.id,
                camp.camp.match_count,
                &players,
            )),
        )
        .await?;
        Ok(())
    })
    .await
}

/// Resume a cancelled camp by ID.
#[poise::command(slash_command, guild_only)]
pub async fn resume(
    ctx: Context<'_>,
    #[description = "Camp ID"] id: String,
) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    with_guild_lock(ctx.data(), guild_id.get(), || async {
        let id = id.trim().to_uppercase();
        let state = db::resume_camp(&ctx.data().db, &id, guild_id.get() as i64).await?;
        sync::redraw_all(ctx.serenity_context(), &ctx.data().db, &state).await?;
        ctx.send(CreateReply::default().content(format!("Resumed camp `{}`.", state.camp.id)))
            .await?;
        Ok(())
    })
    .await
}

/// Show roster, queue, points, and live matches.
#[poise::command(slash_command, guild_only)]
pub async fn info(
    ctx: Context<'_>,
    #[description = "Camp ID (defaults to the active camp)"] id: Option<String>,
) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    let state = if let Some(id) = id {
        db::load_camp(&ctx.data().db, &id.trim().to_uppercase())
            .await?
            .ok_or_else(|| anyhow!("camp not found"))?
    } else {
        db::load_active_for_guild(&ctx.data().db, guild_id.get() as i64)
            .await?
            .ok_or_else(|| anyhow!("no active camp"))?
    };
    if state.camp.guild_id != guild_id.get() as i64 {
        return Err(anyhow!("that camp belongs to another server"));
    }
    ctx.send(
        CreateReply::default()
            .embed(crate::ui::info_embed(&state))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

#[derive(poise::ChoiceParameter, Clone, Copy)]
pub enum CampSize {
    #[name = "4"]
    Four,
    #[name = "24"]
    TwentyFour,
    #[name = "30"]
    Thirty,
}

impl CampSize {
    fn as_i64(self) -> i64 {
        match self {
            CampSize::Four => 4,
            CampSize::TwentyFour => 24,
            CampSize::Thirty => 30,
        }
    }
}
