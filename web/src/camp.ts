// Labels and derived values for camps and matches, matching what the bot shows in Discord.

import type { Camp, Match, Player, Side } from './api';

export const SIDES: Side[] = ['hiddenking', 'archmother'];

export const SIDE_LABEL: Record<Side, string> = {
  hiddenking: 'Hiddenking',
  archmother: 'Archmother',
};

const CAMP_STATUS: Record<string, string> = {
  signup: 'Inscrição',
  running: 'Em andamento',
  ended: 'Encerrado',
  cancelled: 'Cancelado',
  cancelled_signup: 'Inscrição cancelada',
  cancelled_running: 'Cancelado em andamento',
};

export function campStatusLabel(status: string): string {
  return CAMP_STATUS[status] ?? status;
}

export function isActive(status: string): boolean {
  return status === 'signup' || status === 'running';
}

/** `12` players → "6v6". */
export function formatLabel(matchSize: number): string {
  const side = matchSize / 2;
  return `${side}v${side}`;
}

export function points(n: number): string {
  return `${n} ${n === 1 ? 'pt' : 'pts'}`;
}

export function playerName(camp: Camp, userId: string): string {
  return camp.members[userId]?.name ?? `Jogador …${userId.slice(-4)}`;
}

export function playerById(camp: Camp, userId: string): Player | undefined {
  return camp.players.find((p) => p.user_id === userId);
}

export function matchName(match: Match): string {
  return match.is_final ? 'Partida final' : `Partida ${match.slot}`;
}

/** Where a player is right now: their place in the queue or their match and team. */
export function whereIs(camp: Camp, player: Player): string {
  if (player.location === 'queued') {
    const position = queuedPlayers(camp).findIndex((p) => p.user_id === player.user_id) + 1;
    return `Fila · ${position}º`;
  }
  for (const match of camp.open_matches) {
    const seat = match.players.find((p) => p.user_id === player.user_id);
    if (seat) return `${match.is_final ? 'Final' : `Partida ${match.slot}`} · ${SIDE_LABEL[seat.side]}`;
  }
  return '—';
}

/** The draft step happened, on the draft tool or by hand. */
export function draftStarted(match: Match): boolean {
  return match.draft !== null || match.draft_manual;
}

/** `+2`, `-1`: a manual points change. */
export function signed(n: number): string {
  return n > 0 ? `+${n}` : `${n}`;
}

/** Where a match is: voting, waiting for or running its draft, or live. */
export type Stage = 'voting' | 'draft' | 'live';

export function stage(match: Match): Stage {
  if (match.status === 'live') return 'live';
  return match.refresh_used ? 'draft' : 'voting';
}

export interface VoteCount {
  /** Refresh votes per team; one team reaching `needed` re-sorts the match. */
  refresh: Record<Side, number>;
  keep: number;
  voted: number;
  total: number;
  needed: number;
}

export function voteCount(camp: Camp, match: Match): VoteCount {
  const sideOf = new Map(match.players.map((p) => [p.user_id, p.side]));
  const refresh = (side: Side) => match.votes.filter((v) => v.refresh && sideOf.get(v.user_id) === side).length;
  return {
    refresh: { hiddenking: refresh('hiddenking'), archmother: refresh('archmother') },
    keep: match.votes.filter((v) => !v.refresh).length,
    voted: match.votes.length,
    total: match.players.length,
    needed: camp.refresh_votes,
  };
}

/** The same summary line the bot puts on the match message in Discord. */
export function voteSummary(camp: Camp, match: Match): string {
  const voted = match.votes.length;
  const total = match.players.length;
  if (match.refreshed_by_admin) return 'Times sorteados de novo por um admin.';
  if (match.vote_closed_by_admin) {
    return `Votação encerrada por um admin — times mantidos (${voted}/${total} votaram).`;
  }
  if (match.refresh_used) {
    return voted > 0
      ? `Votação encerrada — times mantidos (${voted}/${total} votos).`
      : 'Times sorteados de novo pelo refresh.';
  }
  const count = voteCount(camp, match);
  const needed = count.needed;
  return `Refresh: Hiddenking ${count.refresh.hiddenking}/${needed} · Archmother ${count.refresh.archmother}/${needed} · Manter ${count.keep} · Votos ${voted}/${total}`;
}

export function queuedPlayers(camp: Camp): Player[] {
  return camp.players
    .filter((p) => p.location === 'queued')
    .sort((a, b) => (a.queue_position ?? Infinity) - (b.queue_position ?? Infinity));
}

/** Share of finished matches won, or "—" before the first. */
export function winRate(player: Player): string {
  return player.matches_played > 0 ? `${Math.round((player.wins * 100) / player.matches_played)}%` : '—';
}

/**
 * Players by points, MVPs breaking ties, like the bot's final ranking: `place` counts distinct
 * (points, MVPs) pairs. Players equal on both share the place; fewer matches played lists first.
 */
export function ranking(players: Player[]): { player: Player; place: number }[] {
  const sorted = [...players].sort(
    (a, b) => b.points - a.points || b.mvps - a.mvps || a.matches_played - b.matches_played,
  );
  const keys = [...new Set(sorted.map((p) => `${p.points}:${p.mvps}`))];
  return sorted.map((player) => ({ player, place: keys.indexOf(`${player.points}:${player.mvps}`) + 1 }));
}
