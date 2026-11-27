// Labels and derived values for camps and matches, matching what the bot shows in Discord.

import type { Camp, Match, Player, Rank, Role, Side } from './api';

export const SIDES: Side[] = ['hiddenking', 'archmother'];

export const SIDE_LABEL: Record<Side, string> = {
  hiddenking: 'Hiddenking',
  archmother: 'Archmother',
};

/** In the order teams list them. */
export const ROLES: Role[] = ['tank', 'carry', 'support'];

export const ROLE_LABEL: Record<Role, string> = {
  tank: 'Tank/Bruiser',
  carry: 'Carry',
  support: 'Utility/Support',
};

const ROLE_SHORT: Record<Role, string> = {
  tank: 'Tank',
  carry: 'Carry',
  support: 'Support',
};

export function roleShort(role: Role): string {
  return ROLE_SHORT[role];
}

/** How many players have each role, e.g. "Tank/Bruiser 8 · Carry 10 · Utility/Support 6 · sem função 6". */
export function roleCounts(players: Player[]): string {
  const parts = ROLES.map((role) => `${ROLE_LABEL[role]} ${players.filter((p) => p.role === role).length}`);
  const none = players.filter((p) => p.role === null).length;
  if (none > 0) parts.push(`sem função ${none}`);
  return parts.join(' · ');
}

export const RANKS: Rank[] = ['ss', 's'];

export const RANK_LABEL: Record<Rank, string> = {
  ss: 'SS rank',
  s: 'S rank',
};

/** The player's category, if any. */
export function rankOf(player: Pick<Player, 's_rank' | 'ss_rank'>): Rank | null {
  if (player.ss_rank) return 'ss';
  return player.s_rank ? 's' : null;
}

/** `1 partida`, `3 partidas`. */
export function matchesWord(n: number): string {
  return `${n} ${n === 1 ? 'partida' : 'partidas'}`;
}

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
    if (seat) {
      const where = `${match.is_final ? 'Final' : `Partida ${match.slot}`} · ${SIDE_LABEL[seat.side]}`;
      return player.removed ? `${where} · sai depois` : where;
    }
  }
  return player.removed ? 'Fora do camp' : '—';
}

/** The draft step happened, on the draft tool or by hand. */
export function draftStarted(match: Match): boolean {
  return match.draft !== null || match.draft_manual;
}

/** `+2`, `-1`: a manual points change. */
/** `1 ponto`, `3 pontos`. */
export function pointsWord(n: number): string {
  return `${n} ${n === 1 ? 'ponto' : 'pontos'}`;
}

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
  /** Refresh votes per team. */
  refresh: Record<Side, number>;
  /** Refresh votes from both teams; reaching `needed` re-sorts the match. */
  total_refresh: number;
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
    total_refresh: match.votes.filter((v) => v.refresh).length,
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
  return `Refresh: ${count.total_refresh}/${needed} · Manter ${count.keep} · Votos ${voted}/${total}`;
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
 * Players by points, then MVPs, then fewer matches played, then wins, like the bot's final
 * ranking: players equal on all four share the place.
 */
export function ranking(players: Player[]): { player: Player; place: number }[] {
  const sorted = [...players].sort(
    (a, b) => b.points - a.points || b.mvps - a.mvps || a.matches_played - b.matches_played || b.wins - a.wins,
  );
  const key = (p: Player) => `${p.points}:${p.mvps}:${p.matches_played}:${p.wins}`;
  const keys = [...new Set(sorted.map(key))];
  return sorted.map((player) => ({ player, place: keys.indexOf(key(player)) + 1 }));
}
