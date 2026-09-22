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
            .content("Apenas quem gerencia o servidor pode fazer isso.")
            .ephemeral(true),
    )
    .await?;
    Ok(false)
}

fn guild_id(ctx: Context<'_>) -> Result<GuildId> {
    ctx.guild_id()
        .ok_or_else(|| anyhow!("use este comando em um servidor"))
}

/// Fila, partidas e ranking do camp de Deadlock.
#[poise::command(
    slash_command,
    guild_only,
    subcommands("start", "add", "remove", "cancel", "end", "resume", "info")
)]
pub async fn camp(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Cria um novo camp (24, 30 ou teste de 4 jogadores).
#[poise::command(slash_command, guild_only, rename = "iniciar")]
pub async fn start(
    ctx: Context<'_>,
    #[rename = "tamanho"]
    #[description = "Tamanho do elenco. 4-1v1 roda duas partidas ao mesmo tempo."]
    size: CampSize,
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
                    .content("Este servidor já tem um camp ativo. Cancele ou encerre o atual primeiro.")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
        let camp = db::create_camp(
            &ctx.data().db,
            guild_id.get() as i64,
            ctx.channel_id().get() as i64,
            size.target_size(),
            size.match_size(),
        )
        .await?;
        let state = db::load_camp(&ctx.data().db, &camp.id)
            .await?
            .ok_or_else(|| anyhow!("camp não encontrado"))?;
        sync::redraw_board(ctx.serenity_context(), &ctx.data().db, &state).await?;
        ctx.send(
            CreateReply::default().content(format!(
                "Camp `{}` criado para {} jogadores{}. Adicione os usuários e inicie a fila.",
                camp.id,
                size.target_size(),
                size.format_note()
            )),
        )
        .await?;
        Ok(())
    })
    .await
}

/// Adiciona usuários do Discord ao elenco do camp.
#[poise::command(slash_command, guild_only, rename = "adicionar")]
pub async fn add(
    ctx: Context<'_>,
    #[rename = "jogador"]
    #[description = "Jogador"]
    user: User,
    #[rename = "jogador2"]
    #[description = "Jogador"]
    user2: Option<User>,
    #[rename = "jogador3"]
    #[description = "Jogador"]
    user3: Option<User>,
    #[rename = "jogador4"]
    #[description = "Jogador"]
    user4: Option<User>,
    #[rename = "jogador5"]
    #[description = "Jogador"]
    user5: Option<User>,
) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    with_guild_lock(ctx.data(), guild_id.get(), || async {
        let camp = db::load_active_for_guild(&ctx.data().db, guild_id.get() as i64)
            .await?
            .ok_or_else(|| anyhow!("nenhum camp ativo"))?;
        let mut ids = vec![user.id.get() as i64];
        for extra in [user2, user3, user4, user5].into_iter().flatten() {
            ids.push(extra.id.get() as i64);
        }
        let (added, skipped) = db::add_players(&ctx.data().db, &camp.camp.id, &ids).await?;
        let state = db::load_camp(&ctx.data().db, &camp.camp.id)
            .await?
            .ok_or_else(|| anyhow!("camp não encontrado"))?;
        sync::redraw_board(ctx.serenity_context(), &ctx.data().db, &state).await?;
        ctx.send(
            CreateReply::default()
                .content(format!(
                    "Adicionado(s): {added}. Ignorado(s): {}.",
                    skipped.len()
                ))
                .ephemeral(true),
        )
        .await?;
        Ok(())
    })
    .await
}

/// Remove um usuário do Discord do elenco.
#[poise::command(slash_command, guild_only, rename = "remover")]
pub async fn remove(
    ctx: Context<'_>,
    #[rename = "jogador"]
    #[description = "Jogador"]
    user: User,
) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    with_guild_lock(ctx.data(), guild_id.get(), || async {
        let camp = db::load_active_for_guild(&ctx.data().db, guild_id.get() as i64)
            .await?
            .ok_or_else(|| anyhow!("nenhum camp ativo"))?;
        db::remove_player(&ctx.data().db, &camp.camp.id, user.id.get() as i64).await?;
        let state = db::load_camp(&ctx.data().db, &camp.camp.id)
            .await?
            .ok_or_else(|| anyhow!("camp não encontrado"))?;
        sync::redraw_board(ctx.serenity_context(), &ctx.data().db, &state).await?;
        ctx.send(
            CreateReply::default()
                .content(format!("Removido {}.", user.mention()))
                .ephemeral(true),
        )
        .await?;
        Ok(())
    })
    .await
}

/// Cancela o camp ativo (pode ser retomado).
#[poise::command(slash_command, guild_only, rename = "cancelar")]
pub async fn cancel(ctx: Context<'_>) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    with_guild_lock(ctx.data(), guild_id.get(), || async {
        let camp = db::load_active_for_guild(&ctx.data().db, guild_id.get() as i64)
            .await?
            .ok_or_else(|| anyhow!("nenhum camp ativo"))?;
        db::cancel_camp(&ctx.data().db, &camp.camp.id).await?;
        if let Some(state) = db::load_camp(&ctx.data().db, &camp.camp.id).await? {
            let _ = sync::redraw_board(ctx.serenity_context(), &ctx.data().db, &state).await;
        }
        ctx.send(
            CreateReply::default().content(format!(
                "Camp cancelado. Retome com `/camp retomar {}`",
                camp.camp.id
            )),
        )
        .await?;
        Ok(())
    })
    .await
}

/// Encerra o camp e publica o ranking.
#[poise::command(slash_command, guild_only, rename = "encerrar")]
pub async fn end(ctx: Context<'_>) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    with_guild_lock(ctx.data(), guild_id.get(), || async {
        let camp = db::load_active_for_guild(&ctx.data().db, guild_id.get() as i64)
            .await?
            .ok_or_else(|| anyhow!("nenhum camp ativo"))?;
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

/// Retoma um camp cancelado pelo ID.
#[poise::command(slash_command, guild_only, rename = "retomar")]
pub async fn resume(
    ctx: Context<'_>,
    #[description = "ID do camp"]
    id: String,
) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    with_guild_lock(ctx.data(), guild_id.get(), || async {
        let id = id.trim().to_uppercase();
        let state = db::resume_camp(&ctx.data().db, &id, guild_id.get() as i64).await?;
        sync::redraw_all(ctx.serenity_context(), &ctx.data().db, &state).await?;
        ctx.send(CreateReply::default().content(format!("Camp `{}` retomado.", state.camp.id)))
            .await?;
        Ok(())
    })
    .await
}

/// Mostra elenco, fila, pontos e partidas ao vivo.
#[poise::command(slash_command, guild_only)]
pub async fn info(
    ctx: Context<'_>,
    #[description = "ID do camp (padrão: o camp ativo)"]
    id: Option<String>,
) -> Result<(), Error> {
    if !deny_if_not_admin(ctx).await? {
        return Ok(());
    }
    let guild_id = guild_id(ctx)?;
    let state = if let Some(id) = id {
        db::load_camp(&ctx.data().db, &id.trim().to_uppercase())
            .await?
            .ok_or_else(|| anyhow!("camp não encontrado"))?
    } else {
        db::load_active_for_guild(&ctx.data().db, guild_id.get() as i64)
            .await?
            .ok_or_else(|| anyhow!("nenhum camp ativo"))?
    };
    if state.camp.guild_id != guild_id.get() as i64 {
        return Err(anyhow!("esse camp pertence a outro servidor"));
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
    #[name = "4-1v1"]
    FourOneVOne,
    #[name = "4-2v2"]
    FourTwoVTwo,
    #[name = "24"]
    TwentyFour,
    #[name = "30"]
    Thirty,
}

impl CampSize {
    fn target_size(self) -> i64 {
        match self {
            CampSize::FourOneVOne | CampSize::FourTwoVTwo => 4,
            CampSize::TwentyFour => 24,
            CampSize::Thirty => 30,
        }
    }

    fn match_size(self) -> i64 {
        match self {
            CampSize::FourOneVOne => 2,
            CampSize::FourTwoVTwo => 4,
            CampSize::TwentyFour | CampSize::Thirty => 12,
        }
    }

    fn format_note(self) -> &'static str {
        match self {
            CampSize::FourOneVOne => " (1v1, duas partidas ao mesmo tempo)",
            CampSize::FourTwoVTwo => " (teste 2v2)",
            _ => "",
        }
    }
}
