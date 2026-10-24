// Shapes of the Rust API (crates/web/src/api.rs). Discord IDs are strings.

export type Side = 'hiddenking' | 'archmother';

export interface Me {
  user: { id: string; name: string; avatar_url: string | null };
  /** Servers the user manages that the bot is in. */
  guilds: Guild[];
}

export interface Guild {
  id: string;
  name: string;
  icon_url: string | null;
}

export interface CampSummary {
  id: string;
  channel_id: string;
  status: string;
  target_size: number;
  match_size: number;
  match_count: number;
  created_at: number;
}

export interface Camp {
  id: string;
  guild_id: string;
  channel_id: string;
  status: string;
  target_size: number;
  match_size: number;
  match_count: number;
  has_final: boolean;
  can_create_match: boolean;
  can_create_final: boolean;
  refresh_votes: number;
  players: Player[];
  up_next: string[];
  open_matches: Match[];
  recent_matches: RecentMatch[];
  members: Record<string, Member>;
}

export interface Player {
  user_id: string;
  points: number;
  location: string;
  queue_position: number | null;
  consecutive_matches: number;
  s_rank: boolean;
}

export interface Match {
  id: string;
  camp_id: string;
  slot: number;
  status: 'voting' | 'live' | 'completed';
  is_final: boolean;
  refresh_used: boolean;
  winner_side: Side | null;
  party_code: string | null;
  draft: Draft | null;
  players: MatchPlayer[];
  votes: Vote[];
}

export interface Draft {
  spectator_url: string;
  hiddenking_url: string | null;
  archmother_url: string | null;
  admin_url: string | null;
}

export interface MatchPlayer {
  user_id: string;
  side: Side;
  is_captain: boolean;
  points_after: number | null;
}

export interface Vote {
  user_id: string;
  refresh: boolean;
}

export interface RecentMatch {
  id: string;
  number: number;
  winner_side: Side;
  is_final: boolean;
  completed_at: number;
  captains: { user_id: string; side: Side }[];
  correctable: boolean;
}

export interface Member {
  name: string;
  avatar_url: string;
}

/** The session is missing or expired: show the login screen. */
export class Unauthorized extends Error {}

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(path, { credentials: 'same-origin' });
  if (response.status === 401) throw new Unauthorized('Faça login com o Discord');
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as { error?: string } | null;
    throw new Error(body?.error ?? `Erro ${response.status}`);
  }
  return (await response.json()) as T;
}

export const api = {
  me: () => getJson<Me>('/api/me'),
  camps: (guildId: string) => getJson<CampSummary[]>(`/api/guilds/${guildId}/camps`),
  camp: (campId: string) => getJson<Camp>(`/api/camps/${campId}`),
};
