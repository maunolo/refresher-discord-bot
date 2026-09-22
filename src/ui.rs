use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateSelectMenu,
    CreateSelectMenuKind, Mentionable, UserId,
};

use crate::db::{CampState, OpenMatch, PlayerRow};

pub fn mention(user_id: i64) -> String {
    UserId::new(user_id as u64).mention().to_string()
}

pub fn list_mentions(ids: &[i64]) -> String {
    if ids.is_empty() {
        return "_vazio_".to_string();
    }
    ids.iter()
        .map(|id| mention(*id))
        .collect::<Vec<_>>()
        .join("\n")
}

fn status_label(status: &str) -> &str {
    match status {
        "signup" => "inscrição",
        "running" => "em andamento",
        "voting" => "votação",
        "live" => "ao vivo",
        "ended" => "encerrado",
        "cancelled" => "cancelado",
        "cancelled_signup" => "inscrição cancelada",
        "cancelled_running" => "cancelado em andamento",
        "completed" => "encerrada",
        _ => status,
    }
}

fn location_label(location: &str) -> &str {
    match location {
        "queued" => "fila",
        "match_1" => "partida 1",
        "match_2" => "partida 2",
        _ => location,
    }
}

pub fn trophy_lines(players: &[PlayerRow]) -> String {
    let mut sorted = players.to_vec();
    sorted.sort_by(|a, b| b.points.cmp(&a.points).then(a.user_id.cmp(&b.user_id)));
    let mut unique: Vec<i64> = sorted.iter().map(|p| p.points).collect();
    unique.dedup();
    let gold = unique.first().copied();
    let silver = unique.get(1).copied();
    let bronze = unique.get(2).copied();
    sorted
        .into_iter()
        .map(|p| {
            let medal = if Some(p.points) == gold {
                "🥇 "
            } else if Some(p.points) == silver {
                "🥈 "
            } else if Some(p.points) == bronze {
                "🥉 "
            } else {
                ""
            };
            format!("{medal}{} — **{}** pts", mention(p.user_id), p.points)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn footer(camp_id: &str) -> CreateEmbedFooter {
    CreateEmbedFooter::new(format!("Camp {camp_id}"))
}

pub fn signup_embed(state: &CampState) -> CreateEmbed {
    let mut ids: Vec<i64> = state.players.iter().map(|p| p.user_id).collect();
    ids.sort();
    CreateEmbed::new()
        .title("Elenco do camp de Deadlock")
        .description(format!(
            "{}/{} jogadores. O admin adiciona usuários do Discord e depois inicia a fila.{}",
            state.players.len(),
            state.camp.target_size,
            match state.camp.layout().match_size {
                2 => " Este é um camp de teste 1v1 (duas partidas ao mesmo tempo).",
                4 => " Este é um camp de teste 2v2.",
                _ => "",
            }
        ))
        .field("Jogadores", list_mentions(&ids), false)
        .footer(footer(&state.camp.id))
}

pub fn signup_components(state: &CampState) -> Vec<CreateActionRow> {
    let full = state.players.len() as i64 == state.camp.target_size;
    let remaining = (state.camp.target_size as i64 - state.players.len() as i64).max(0) as u8;
    let select_max = remaining.min(25).max(1);
    let mut rows = Vec::new();
    if !full {
        rows.push(CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("add_users:{}", state.camp.id),
                CreateSelectMenuKind::User {
                    default_users: None,
                },
            )
            .placeholder("Adicionar jogadores (até 25 por vez)")
            .min_values(1)
            .max_values(select_max),
        ));
    }
    rows.push(CreateActionRow::Buttons(vec![CreateButton::new(format!(
        "start_queue:{}",
        state.camp.id
    ))
    .label("Iniciar fila")
    .style(ButtonStyle::Success)
    .disabled(!full)]));
    rows
}

fn queued_players(state: &CampState) -> Vec<&PlayerRow> {
    let mut q: Vec<&PlayerRow> = state
        .players
        .iter()
        .filter(|p| p.location == "queued")
        .collect();
    q.sort_by_key(|p| p.queue_position.unwrap_or(i64::MAX));
    q
}

fn queue_line(index: usize, player: &PlayerRow, up_next: bool) -> String {
    format!(
        "`{:>2}.` {} {} ({} seg., {} pts)",
        index + 1,
        if up_next { "→" } else { " " },
        mention(player.user_id),
        player.consecutive_matches,
        player.points
    )
}

/// Discord embed fields are capped at 1024 characters.
fn pack_field_chunks(lines: &[String]) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in lines {
        let extra = if current.is_empty() { 0 } else { 1 };
        if !current.is_empty() && current.len() + extra + line.len() > 1024 {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

pub fn board_embed(state: &CampState) -> CreateEmbed {
    let queue = queued_players(state);
    let next_n = state.camp.layout().match_size;
    let mut embed = CreateEmbed::new()
        .title(format!("Camp de Deadlock {}", state.camp.id))
        .description(format!(
            "Partidas jogadas: **{}** · Elenco: **{}**\nO admin clica em **Criar partida** para montar os próximos times a partir da fila. **→** indica quem entra no próximo sorteio.",
            state.camp.match_count,
            state.players.len()
        ))
        .footer(footer(&state.camp.id));
    if queue.is_empty() {
        embed = embed.field("Fila", "_vazia_", false);
    } else {
        let lines: Vec<String> = queue
            .iter()
            .enumerate()
            .map(|(i, p)| queue_line(i, p, i < next_n))
            .collect();
        for (i, chunk) in pack_field_chunks(&lines).into_iter().enumerate() {
            let name = if i == 0 {
                "Fila".to_string()
            } else {
                format!("Fila ({})", i + 1)
            };
            embed = embed.field(name, chunk, false);
        }
    }
    for open in &state.open_matches {
        embed = embed.field(
            format!(
                "{} ({})",
                if open.row.is_final {
                    "Partida final".to_string()
                } else {
                    format!("Partida {}", open.row.slot)
                },
                status_label(&open.row.status)
            ),
            match_field(open),
            false,
        );
    }
    embed
}

pub fn board_components(state: &CampState) -> Vec<CreateActionRow> {
    if state.camp.status != "running" {
        return Vec::new();
    }
    let scored = state.camp.match_count >= 1;
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("create_match:{}", state.camp.id))
            .label("Criar partida")
            .style(ButtonStyle::Success)
            .disabled(!state.can_create_match()),
        CreateButton::new(format!("final_match:{}", state.camp.id))
            .label("Partida final")
            .style(ButtonStyle::Primary)
            .disabled(!state.can_create_final()),
        CreateButton::new(format!("end_camp:{}", state.camp.id))
            .label("Encerrar camp")
            .style(ButtonStyle::Danger)
            .disabled(!scored),
    ])]
}

fn match_field(open: &OpenMatch) -> String {
    let mut hk: Vec<i64> = open
        .players
        .iter()
        .filter(|p| p.side == "hiddenking")
        .map(|p| p.user_id)
        .collect();
    let mut am: Vec<i64> = open
        .players
        .iter()
        .filter(|p| p.side == "archmother")
        .map(|p| p.user_id)
        .collect();
    hk.sort();
    am.sort();
    let mut text = format!(
        "**Hiddenking**\n{}\n**Archmother**\n{}",
        list_mentions(&hk),
        list_mentions(&am)
    );
    if let Some(code) = &open.row.party_code {
        text.push_str(&format!("\nCódigo da party: `{code}`"));
    }
    text
}

pub fn match_embed(state: &CampState, open: &OpenMatch) -> CreateEmbed {
    let hk: Vec<i64> = open
        .players
        .iter()
        .filter(|p| p.side == "hiddenking")
        .map(|p| p.user_id)
        .collect();
    let am: Vec<i64> = open
        .players
        .iter()
        .filter(|p| p.side == "archmother")
        .map(|p| p.user_id)
        .collect();
    let refresh_ids: Vec<i64> = open
        .votes
        .iter()
        .filter(|v| v.refresh)
        .map(|v| v.user_id)
        .collect();
    let keep = open.votes.iter().filter(|v| !v.refresh).count();
    let hk_votes = refresh_ids.iter().filter(|id| hk.contains(id)).count();
    let am_votes = refresh_ids.iter().filter(|id| am.contains(id)).count();
    let needed = state.camp.layout().refresh_votes;
    let voted = open.votes.len();
    let total = open.players.len();
    let mut desc = if open.row.status == "voting" {
        if open.row.refresh_used && !open.votes.is_empty() {
            format!(
                "Votação encerrada — times mantidos ({voted}/{total}). Um admin pode iniciar a partida."
            )
        } else if open.row.refresh_used {
            "Times atualizados. Um admin pode iniciar a partida.".to_string()
        } else {
            format!(
                "Atualizar: Hiddenking **{hk_votes}/{needed}** · Archmother **{am_votes}/{needed}**\nManter times: **{keep}** · Votos: **{voted}/{total}**"
            )
        }
    } else {
        "Partida ao vivo. Entre com o código da party e escolha os heróis no Deadlock.".to_string()
    };
    if let Some(code) = &open.row.party_code {
        desc.push_str(&format!("\n\n**Código da party:** `{code}`"));
    }
    CreateEmbed::new()
        .title(if open.row.is_final {
            format!("Partida final · Camp {}", state.camp.id)
        } else {
            format!("Partida {} · Camp {}", open.row.slot, state.camp.id)
        })
        .description(desc)
        .field("Hiddenking", list_mentions(&hk), true)
        .field("Archmother", list_mentions(&am), true)
        .footer(footer(&state.camp.id))
}

pub fn match_components(state: &CampState, open: &OpenMatch) -> Vec<CreateActionRow> {
    let mut rows = Vec::new();
    if open.row.status == "voting" {
        let mut buttons = vec![CreateButton::new(format!(
            "start_match:{}:{}",
            state.camp.id, open.row.id
        ))
        .label("Iniciar partida")
        .style(ButtonStyle::Success)];
        if !open.row.refresh_used {
            buttons.insert(
                0,
                CreateButton::new(format!("no_refresh:{}:{}", state.camp.id, open.row.id))
                    .label("Manter times")
                    .style(ButtonStyle::Secondary),
            );
            buttons.insert(
                0,
                CreateButton::new(format!("refresh:{}:{}", state.camp.id, open.row.id))
                    .label("Refresh")
                    .style(ButtonStyle::Primary),
            );
        }
        rows.push(CreateActionRow::Buttons(buttons));
    }
    if open.row.status == "live" {
        rows.push(CreateActionRow::Buttons(vec![
            CreateButton::new(format!(
                "win:{}:{}:hiddenking",
                state.camp.id, open.row.id
            ))
            .label("Hiddenking venceu")
            .style(ButtonStyle::Secondary),
            CreateButton::new(format!(
                "win:{}:{}:archmother",
                state.camp.id, open.row.id
            ))
            .label("Archmother venceu")
            .style(ButtonStyle::Secondary),
        ]));
    }
    rows
}

pub fn winner_label(side: &str) -> &'static str {
    if side == "hiddenking" {
        "Hiddenking"
    } else {
        "Archmother"
    }
}

pub fn completed_match_embed(camp_id: &str, winner_side: &str, is_final: bool) -> CreateEmbed {
    let winner = winner_label(winner_side);
    let title = if is_final {
        format!("{winner} venceu · Partida final")
    } else {
        format!("{winner} venceu")
    };
    CreateEmbed::new()
        .title(title)
        .description(format!("Camp {camp_id}"))
        .footer(footer(camp_id))
}

pub fn completed_match_components(camp_id: &str, match_id: i64) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("correct_win:{camp_id}:{match_id}"))
            .label("Trocar vencedor")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("reopen_match:{camp_id}:{match_id}"))
            .label("Reabrir partida")
            .style(ButtonStyle::Primary),
    ])]
}

pub fn standings_embed(camp_id: &str, match_count: i64, players: &[PlayerRow]) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("Ranking do camp {camp_id}"))
        .description(format!("Partidas jogadas: **{match_count}**"))
        .field("Colocação", trophy_lines(players), false)
        .footer(footer(camp_id))
}

pub fn info_embed(state: &CampState) -> CreateEmbed {
    let queue = queued_players(state);
    let queue_text = if queue.is_empty() {
        "_nenhum_".into()
    } else {
        queue
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "`{:>2}.` {} · {} pts · {} seg. · {}",
                    i + 1,
                    mention(p.user_id),
                    p.points,
                    p.consecutive_matches,
                    location_label(&p.location)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut embed = CreateEmbed::new()
        .title(format!(
            "Camp {} ({})",
            state.camp.id,
            status_label(&state.camp.status)
        ))
        .description(format!(
            "Tamanho {} · tamanho da partida {} · partidas jogadas {}",
            state.camp.target_size, state.camp.match_size, state.camp.match_count
        ))
        .field("Fila", queue_text, false)
        .footer(footer(&state.camp.id));
    for open in &state.open_matches {
        embed = embed.field(
            if open.row.is_final {
                format!("Partida final ({})", status_label(&open.row.status))
            } else {
                format!(
                    "Partida {} ({})",
                    open.row.slot,
                    status_label(&open.row.status)
                )
            },
            match_field(open),
            false,
        );
    }
    embed
}
