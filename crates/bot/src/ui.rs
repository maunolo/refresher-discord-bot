//! What players see in Discord: the camp's queue message and one message per match. Admin
//! actions live on the web admin page, so the only buttons here are the players' own.

use std::collections::HashMap;

use serenity::all::{ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, Mentionable, UserId};

use crate::db::{CampState, MatchPlayerRow, OpenMatch, PlayerRow};
use crate::queue;
use crate::ranking::{self, Standing};

pub const S_RANK_TAG: &str = "⭐";
pub const CAPTAIN_TAG: &str = "👑";
pub const CAPTAIN_LINK_LABEL: &str = "Link do capitão";

pub fn mention(user_id: i64) -> String {
    UserId::new(user_id as u64).mention().to_string()
}

fn player_label(player: &PlayerRow) -> String {
    if player.s_rank {
        format!("{} {S_RANK_TAG}", mention(player.user_id))
    } else {
        mention(player.user_id)
    }
}

/// One team's mentions, one per line, captain first.
fn team_list(players: &[MatchPlayerRow], side: &str) -> String {
    let mut team: Vec<&MatchPlayerRow> = players.iter().filter(|p| p.side == side).collect();
    if team.is_empty() {
        return "_vazio_".to_string();
    }
    team.sort_by_key(|p| (!p.is_captain, p.user_id));
    team.iter()
        .map(|p| {
            if p.is_captain {
                format!("{} {CAPTAIN_TAG}", mention(p.user_id))
            } else {
                mention(p.user_id)
            }
        })
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

/// The ranking by points, MVPs breaking ties; gold, silver and bronze for the top three places.
pub fn trophy_lines(players: &[PlayerRow], mvps: &HashMap<i64, i64>) -> String {
    let standings: Vec<Standing> = players
        .iter()
        .map(|p| Standing {
            user_id: p.user_id,
            points: p.points,
            mvps: mvps.get(&p.user_id).copied().unwrap_or(0),
        })
        .collect();
    ranking::rank(&standings)
        .into_iter()
        .map(|(s, place)| {
            let medal = match place {
                1 => "🥇 ",
                2 => "🥈 ",
                3 => "🥉 ",
                _ => "",
            };
            let mvp = match s.mvps {
                0 => String::new(),
                1 => " · 1 MVP".to_string(),
                n => format!(" · {n} MVPs"),
            };
            format!("{medal}{} — **{}** pts{mvp}", mention(s.user_id), s.points)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn footer(camp_id: &str) -> CreateEmbedFooter {
    CreateEmbedFooter::new(format!("Camp {camp_id}"))
}

pub fn signup_embed(state: &CampState) -> CreateEmbed {
    let mut players: Vec<&PlayerRow> = state.players.iter().collect();
    players.sort_by_key(|p| p.user_id);
    let roster = if players.is_empty() {
        "_vazio_".to_string()
    } else {
        players
            .iter()
            .map(|p| player_label(p))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let s_rank = state.players.iter().filter(|p| p.s_rank).count();
    CreateEmbed::new()
        .title("Elenco do camp de Deadlock")
        .description(format!(
            "Inscrição aberta: {}/{} jogadores. Quando o elenco fechar, a fila aparece aqui.{}\n{S_RANK_TAG} S rank: **{s_rank}** — a fila começa com eles divididos por igual entre os times, e todos são capitães.",
            state.players.len(),
            state.camp.target_size,
            match state.camp.layout().match_size {
                2 => " Este é um camp de teste 1v1 (duas partidas ao mesmo tempo).",
                4 => " Este é um camp de teste 2v2.",
                _ => "",
            }
        ))
        .field("Jogadores", roster, false)
        .footer(footer(&state.camp.id))
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
        player_label(player),
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
    let seats: Vec<queue::Seat> = queue.iter().map(|p| p.seat()).collect();
    let up_next: Vec<i64> = queue::pick_match_seats(&seats, state.camp.layout().match_size)
        .iter()
        .map(|s| s.user_id)
        .collect();
    let mut embed = CreateEmbed::new()
        .title(format!("Camp de Deadlock {}", state.camp.id))
        .description(format!(
            "Partidas jogadas: **{}** · Elenco: **{}**\nOs próximos times saem da fila quando um admin cria a partida. **→** indica quem entra no próximo sorteio.",
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
            .map(|(i, p)| queue_line(i, p, up_next.contains(&p.user_id)))
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

fn teams_text(players: &[MatchPlayerRow], party_code: Option<&str>) -> String {
    let mut text = format!(
        "**Hiddenking**\n{}\n**Archmother**\n{}",
        team_list(players, "hiddenking"),
        team_list(players, "archmother")
    );
    if let Some(code) = party_code.filter(|c| !c.is_empty()) {
        text.push_str(&format!("\nCódigo da party: `{code}`"));
    }
    text
}

fn match_field(open: &OpenMatch) -> String {
    teams_text(&open.players, open.row.party_code.as_deref())
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
        if open.row.draft_url.is_some() {
            format!("Draft iniciado — os capitães ({CAPTAIN_TAG}) receberam por DM o link do draft. Um admin pode iniciar a partida.")
        } else if open.row.draft_manual {
            format!("Draft iniciado — os capitães ({CAPTAIN_TAG}) recebem o link do draft de um admin. Depois, um admin inicia a partida.")
        } else if open.row.refreshed_by_admin {
            "Times sorteados de novo por um admin. Um admin pode iniciar o draft.".to_string()
        } else if open.row.vote_closed_by_admin {
            format!(
                "Votação encerrada por um admin — times mantidos ({voted}/{total} votaram). Um admin pode iniciar o draft."
            )
        } else if open.row.refresh_used && !open.votes.is_empty() {
            format!(
                "Votação encerrada — times mantidos ({voted}/{total}). Um admin pode iniciar o draft."
            )
        } else if open.row.refresh_used {
            "Times atualizados. Um admin pode iniciar o draft.".to_string()
        } else {
            let refresh = if open.row.is_final {
                "**Refresh** sorteia times novos com os mesmos jogadores."
            } else {
                "**Refresh** sorteia times novos e pode trazer jogadores da fila; quem estava esperando fica."
            };
            format!(
                "{refresh}\nAtualizar: Hiddenking **{hk_votes}/{needed}** · Archmother **{am_votes}/{needed}**\nManter times: **{keep}** · Votos: **{voted}/{total}**"
            )
        }
    } else {
        "Partida ao vivo. Entre com o código da party e escolha os heróis no Deadlock.".to_string()
    };
    if let Some(url) = &open.row.draft_url {
        desc.push_str(&format!("\n\n**Assistir ao draft:** {url}"));
    }
    if let Some(code) = &open.row.party_code {
        desc.push_str(&format!("\n\n**Código da party:** `{code}`"));
    }
    CreateEmbed::new()
        .title(if open.row.is_final {
            format!("Partida final · ID {} · Camp {}", open.row.id, state.camp.id)
        } else {
            format!(
                "Partida {} · ID {} · Camp {}",
                open.row.slot, open.row.id, state.camp.id
            )
        })
        .description(desc)
        .field("Hiddenking", team_list(&open.players, "hiddenking"), true)
        .field("Archmother", team_list(&open.players, "archmother"), true)
        .footer(footer(&state.camp.id))
}

fn captain_link_button(state: &CampState, open: &OpenMatch) -> CreateButton {
    CreateButton::new(format!("captain_link:{}:{}", state.camp.id, open.row.id))
        .label(CAPTAIN_LINK_LABEL)
        .emoji('👑')
        .style(ButtonStyle::Secondary)
}

/// The refresh vote while it's open, and the captains' draft link once the draft exists.
pub fn match_components(state: &CampState, open: &OpenMatch) -> Vec<CreateActionRow> {
    let mut buttons = Vec::new();
    if open.row.status == "voting" && !open.row.refresh_used {
        buttons.push(
            CreateButton::new(format!("refresh:{}:{}", state.camp.id, open.row.id))
                .label("Refresh")
                .style(ButtonStyle::Primary),
        );
        buttons.push(
            CreateButton::new(format!("no_refresh:{}:{}", state.camp.id, open.row.id))
                .label("Manter times")
                .style(ButtonStyle::Secondary),
        );
    }
    if open.row.draft_url.is_some() {
        buttons.push(captain_link_button(state, open));
    }
    if buttons.is_empty() {
        Vec::new()
    } else {
        vec![CreateActionRow::Buttons(buttons)]
    }
}

/// `1 ponto`, `3 pontos`.
pub fn points_label(points: i64) -> String {
    if points == 1 {
        "1 ponto".to_string()
    } else {
        format!("{points} pontos")
    }
}

pub fn winner_label(side: &str) -> &'static str {
    if side == "hiddenking" {
        "Hiddenking"
    } else {
        "Archmother"
    }
}

fn completed_match_description(camp_id: &str, match_id: &str, party_code: Option<&str>) -> String {
    let mut desc = format!("Camp `{camp_id}`\nID da partida: `{match_id}`");
    if let Some(code) = party_code.filter(|c| !c.is_empty()) {
        desc.push_str(&format!("\n**Código da party:** `{code}`"));
    }
    desc
}

pub fn completed_match_embed(
    camp_id: &str,
    match_id: &str,
    winner_side: &str,
    is_final: bool,
    party_code: Option<&str>,
    mvp_user_id: Option<i64>,
) -> CreateEmbed {
    let winner = winner_label(winner_side);
    let title = if is_final {
        format!("{winner} venceu · Partida final")
    } else {
        format!("{winner} venceu")
    };
    let mut desc = completed_match_description(camp_id, match_id, party_code);
    if let Some(mvp) = mvp_user_id {
        desc.push_str(&format!("\n**MVP:** {}", mention(mvp)));
    }
    CreateEmbed::new()
        .title(title)
        .description(desc)
        .footer(footer(camp_id))
}

pub fn standings_embed(
    camp_id: &str,
    match_count: i64,
    players: &[PlayerRow],
    mvps: &HashMap<i64, i64>,
) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("Ranking do camp {camp_id}"))
        .description(format!(
            "Partidas jogadas: **{match_count}**\nEmpates em pontos são desfeitos pelo número de MVPs."
        ))
        .field("Colocação", trophy_lines(players, mvps), false)
        .footer(footer(camp_id))
}
