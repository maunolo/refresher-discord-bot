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
        return "_empty_".to_string();
    }
    ids.iter()
        .map(|id| mention(*id))
        .collect::<Vec<_>>()
        .join("\n")
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
        .title("Deadlock camp roster")
        .description(format!(
            "{}/{} players. Admin adds Discord users, then starts the queue.{}",
            state.players.len(),
            state.camp.target_size,
            if state.camp.target_size == 4 {
                " This is a 2v2 test camp."
            } else {
                ""
            }
        ))
        .field("Players", list_mentions(&ids), false)
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
            .placeholder("Add players (up to 25 at a time)")
            .min_values(1)
            .max_values(select_max),
        ));
    }
    rows.push(CreateActionRow::Buttons(vec![CreateButton::new(format!(
        "start_queue:{}",
        state.camp.id
    ))
    .label("Start queue")
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

pub fn board_embed(state: &CampState) -> CreateEmbed {
    let queue = queued_players(state);
    let queue_preview: String = if queue.is_empty() {
        "_empty_".to_string()
    } else {
        let preview = state.camp.layout().match_size;
        queue
            .iter()
            .take(preview)
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "`{:>2}.` {} ({} consec, {} pts)",
                    i + 1,
                    mention(p.user_id),
                    p.consecutive_matches,
                    p.points
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut embed = CreateEmbed::new()
        .title(format!("Deadlock camp {}", state.camp.id))
        .description(format!(
            "Matches played: **{}** · Roster: **{}**\nAdmin clicks **Create match** to sort the next teams from the queue.",
            state.camp.match_count,
            state.players.len()
        ))
        .field("Queue", queue_preview, false)
        .footer(footer(&state.camp.id));
    for open in &state.open_matches {
        embed = embed.field(
            format!("Match {} ({})", open.row.slot, open.row.status),
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
            .label("Create match")
            .style(ButtonStyle::Success)
            .disabled(!state.can_create_match()),
        CreateButton::new(format!("end_camp:{}", state.camp.id))
            .label("End camp")
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
        text.push_str(&format!("\nParty code: `{code}`"));
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
            format!("Voting complete — teams kept ({voted}/{total}). An admin can start the match.")
        } else if open.row.refresh_used {
            "Teams refreshed. An admin can start the match.".to_string()
        } else {
            format!(
                "Refresh: Hiddenking **{hk_votes}/{needed}** · Archmother **{am_votes}/{needed}**\nNo refresh: **{keep}** · Voted: **{voted}/{total}**"
            )
        }
    } else {
        "Match is live. Join with the party code and pick heroes in Deadlock.".to_string()
    };
    if let Some(code) = &open.row.party_code {
        desc.push_str(&format!("\n\n**Party code:** `{code}`"));
    }
    CreateEmbed::new()
        .title(format!("Match {} · Camp {}", open.row.slot, state.camp.id))
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
        .label("Start match")
        .style(ButtonStyle::Success)];
        if !open.row.refresh_used {
            buttons.insert(
                0,
                CreateButton::new(format!("no_refresh:{}:{}", state.camp.id, open.row.id))
                    .label("No refresh")
                    .style(ButtonStyle::Secondary),
            );
            buttons.insert(
                0,
                CreateButton::new(format!("refresh:{}:{}", state.camp.id, open.row.id))
                    .label("Refresh teams")
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
            .label("Hiddenking won")
            .style(ButtonStyle::Secondary),
            CreateButton::new(format!(
                "win:{}:{}:archmother",
                state.camp.id, open.row.id
            ))
            .label("Archmother won")
            .style(ButtonStyle::Secondary),
        ]));
    }
    rows
}

pub fn standings_embed(camp_id: &str, match_count: i64, players: &[PlayerRow]) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("Camp {camp_id} standings"))
        .description(format!("Matches played: **{match_count}**"))
        .field("Ranks", trophy_lines(players), false)
        .footer(footer(camp_id))
}

pub fn info_embed(state: &CampState) -> CreateEmbed {
    let queue = queued_players(state);
    let queue_text = if queue.is_empty() {
        "_none_".into()
    } else {
        queue
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "`{:>2}.` {} · {} pts · {} consec · {}",
                    i + 1,
                    mention(p.user_id),
                    p.points,
                    p.consecutive_matches,
                    p.location
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut embed = CreateEmbed::new()
        .title(format!("Camp {} ({})", state.camp.id, state.camp.status))
        .description(format!(
            "Size {} · matches played {}",
            state.camp.target_size, state.camp.match_count
        ))
        .field("Queue", queue_text, false)
        .footer(footer(&state.camp.id));
    for open in &state.open_matches {
        embed = embed.field(
            format!("Live/voting match {}", open.row.slot),
            match_field(open),
            false,
        );
    }
    embed
}
