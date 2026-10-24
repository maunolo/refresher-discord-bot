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
  /** During signup: who is in the Queue voice channel now. */
  queue_voice: string[];
}

export interface Channel {
  id: string;
  name: string;
}

export interface MemberResult {
  id: string;
  name: string;
  avatar_url: string;
}

export type CampSizeChoice = '24' | '30' | '4-1v1' | '4-2v2';

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

/** An admin action; the bot runs it like the matching Discord button. */
export type CampAction =
  | { type: 'create_match' }
  | { type: 'create_final' }
  | { type: 'start_draft'; match_id: string }
  | { type: 'start_match'; match_id: string }
  | { type: 'report_winner'; match_id: string; side: Side }
  | { type: 'correct_winner'; match_id: string }
  | { type: 'reopen_match'; match_id: string }
  | { type: 'add_players'; user_ids: string[] }
  | { type: 'remove_player'; user_id: string }
  | { type: 'set_s_rank'; user_id: string; s_rank: boolean }
  | { type: 'start_queue' }
  | { type: 'cancel_camp' }
  | { type: 'resume_camp' }
  | { type: 'end_camp' };

/** The session is missing or expired: show the login screen. */
export class Unauthorized extends Error {}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, { credentials: 'same-origin', ...init });
  if (response.status === 401) throw new Unauthorized('Faça login com o Discord');
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as { error?: string } | null;
    throw new Error(body?.error ?? `Erro ${response.status}`);
  }
  return (await response.json()) as T;
}

export const api = {
  me: () => request<Me>('/api/me'),
  camps: (guildId: string) => request<CampSummary[]>(`/api/guilds/${guildId}/camps`),
  camp: (campId: string) => request<Camp>(`/api/camps/${campId}`),
  channels: (guildId: string) => request<Channel[]>(`/api/guilds/${guildId}/channels`),
  searchMembers: (guildId: string, query: string) =>
    request<MemberResult[]>(`/api/guilds/${guildId}/members?query=${encodeURIComponent(query)}`),
  /** Resolves with the camp once the bot has also updated Discord. */
  act: (campId: string, action: CampAction) => post<Camp>(`/api/camps/${campId}/actions`, action),
  createCamp: (guildId: string, channelId: string, size: CampSizeChoice) =>
    post<Camp>(`/api/guilds/${guildId}/camps`, { channel_id: channelId, size }),
};

function post<T>(path: string, body: unknown): Promise<T> {
  return request<T>(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}
