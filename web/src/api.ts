// Shapes of the Rust API (crates/web/src/api.rs). Discord IDs are strings.

export type Side = 'hiddenking' | 'archmother';

export interface Me {
  user: { id: string; name: string; avatar_url: string | null };
  /** Servers the user manages that the bot is in. */
  guilds: Guild[];
  /** The public league site, for admins of its server. */
  site: { guild_id: string; url: string } | null;
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
  /** The camp's message in Discord, else its channel. */
  discord_url: string;
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
  /** The camp's message in Discord, else its channel. */
  discord_url: string;
  /** The server creates drafts on the draft tool; off, admins share a draft link themselves. */
  draft_tool: boolean;
  /** Captains get the draft's invite link by DM; either way it's on their match message button. */
  dm_captains: boolean;
  /** What the newest match gives, the starting values for the next one. */
  next_points: MatchPoints;
}

/** What a match gives each player: `win` to the winning side, `loss` to the losing one. */
export interface MatchPoints {
  win: number;
  loss: number;
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

/** A player's main role, kept per server: tank or bruiser, carry, utility or support. */
export type Role = 'tank' | 'carry' | 'support';

export interface Player {
  user_id: string;
  points: number;
  location: string;
  queue_position: number | null;
  consecutive_matches: number;
  s_rank: boolean;
  role: Role | null;
  /** Finished matches only. */
  matches_played: number;
  wins: number;
  /** Points admins added (or, below zero, took) by hand. */
  points_adjusted: number;
  /** Finished matches as MVP: breaks ties in points. */
  mvps: number;
  /** An admin took them out of the camp: no queue, no new matches. In an open match, they leave once it ends. */
  removed: boolean;
}

export interface Match {
  id: string;
  camp_id: string;
  slot: number;
  status: 'voting' | 'live' | 'completed';
  is_final: boolean;
  refresh_used: boolean;
  /** An admin ended the vote early and kept the teams. */
  vote_closed_by_admin: boolean;
  /** The draft started without the draft tool: no links, an admin shares one. */
  draft_manual: boolean;
  /** An admin ended the vote by re-sorting the teams. */
  refreshed_by_admin: boolean;
  winner_side: Side | null;
  mvp_user_id: string | null;
  /** What each player earns when it ends. */
  points: MatchPoints;
  party_code: string | null;
  draft: Draft | null;
  players: MatchPlayer[];
  votes: Vote[];
  /** Finished matches, in the match list: position in the order they ended. */
  number: number | null;
  completed_at: number | null;
  /** The match's message in Discord, else the camp's channel. */
  discord_url: string;
}

export interface Draft {
  spectator_url: string;
  /** The captains' invite link: whoever opens it can take a slot on either side. */
  join_url: string | null;
  /** The draft as an OBS overlay, for streaming it. */
  obs_url: string | null;
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
  points: MatchPoints;
  captains: { user_id: string; side: Side }[];
  players: MatchPlayer[];
  mvp_user_id: string | null;
  correctable: boolean;
  /** Only the latest result, while the camp runs. */
  mvp_changeable: boolean;
}

export interface Member {
  name: string;
  avatar_url: string;
}

/** An admin action; the bot makes the change and does its Discord side. */
export type CampAction =
  | { type: 'create_match'; win_points: number; loss_points: number }
  | { type: 'create_final'; win_points: number; loss_points: number }
  | { type: 'close_vote'; match_id: string }
  | { type: 'refresh_teams'; match_id: string }
  | { type: 'start_draft'; match_id: string }
  | { type: 'start_match'; match_id: string }
  | { type: 'report_winner'; match_id: string; side: Side; mvp_user_id: string }
  | { type: 'correct_winner'; match_id: string }
  | { type: 'change_mvp'; match_id: string; user_id: string }
  | { type: 'reopen_match'; match_id: string }
  | { type: 'cancel_match'; match_id: string }
  | { type: 'add_players'; user_ids: string[] }
  | { type: 'remove_player'; user_id: string }
  | { type: 'restore_player'; user_id: string }
  | { type: 'set_s_rank'; user_id: string; s_rank: boolean }
  | { type: 'set_role'; user_id: string; role: Role | null }
  | { type: 'adjust_points'; user_id: string; delta: number }
  | { type: 'start_queue' }
  | { type: 'reorder_queue'; user_ids: string[] }
  | { type: 'remind_voters'; match_id: string }
  | { type: 'cancel_camp' }
  | { type: 'resume_camp' }
  | { type: 'end_camp' };

/** A server's settings. A voice channel left null is found by its name. */
export interface SettingsForm {
  queue_voice_id: string | null;
  match1_hiddenking_voice_id: string | null;
  match1_archmother_voice_id: string | null;
  match2_hiddenking_voice_id: string | null;
  match2_archmother_voice_id: string | null;
  create_lobby: boolean;
  server_region: string | null;
  /** Off: no drafts on the draft tool; admins share a draft link themselves. */
  use_draft_tool: boolean;
  draft_timer_seconds: number;
  /** Extra seconds per side once a turn runs out. */
  draft_reserve_seconds: number;
  /** 0 turns bans off. */
  draft_bans_per_team: number;
  /** DeadChaps' draft page layout. */
  draft_view: DraftView;
  /** DM each captain the invite link. */
  dm_captains: boolean;
}

export type DraftView = 'cinematic' | 'competitive';

export interface Settings extends SettingsForm {
  /** False until an admin saves: the server runs on the bot's .env values. */
  saved: boolean;
  voice_channels: Channel[];
  draft_timers: number[];
  draft_reserves: number[];
  max_draft_bans: number;
}

/** A server's OBS overlay links. Anyone with one sees the overlay, no login needed. */
export interface OverlayLinks {
  vote_url: string;
  ranking_url: string;
}

// The public league site (crates/web/src/site.rs): staff write its info cards; its ranking and
// matches come from the published camp.

/** One info card in each of the site's languages. */
export interface SiteTexts {
  pt: string;
  en: string;
  es: string;
}

export type SiteInfoKey = 'data' | 'formato' | 'premiacao' | 'times';

export type SiteInfo = Record<SiteInfoKey, SiteTexts>;

export interface SiteState {
  /** This camp is the one the site shows. */
  published: boolean;
  url: string;
  info: SiteInfo;
  /** Finished matches of the camp, and how many have a Deadlock scoreboard for the site. */
  finished_matches: number;
  scoreboards: number;
}

/** A finished match's scoreboard lookup on the Deadlock API (crates/web/src/scoreboard.rs). */
export interface MatchStats {
  /** The bot is looking; an admin has to give the ID; done; or the API never had it. */
  status: 'pending' | 'needs_id' | 'ready' | 'failed' | 'none';
  deadlock_match_id: string | null;
  attempts: number;
  /** Unix seconds. */
  next_attempt_at: number | null;
  error: string | null;
  scoreboard: {
    duration_s: number;
    winner_side: Side;
    players: StatsPlayer[];
  } | null;
}

export interface StatsPlayer {
  /** The Steam account. */
  account_id: string;
  side: Side;
  steam_name: string | null;
  hero: string;
  hero_image: string | null;
  kills: number;
  deaths: number;
  assists: number;
  souls: number;
  /** The member linked to the Steam account. */
  user_id: string | null;
}

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
  /** Open matches first, then finished ones newest first. */
  matches: (campId: string) => request<Match[]>(`/api/camps/${campId}/matches`),
  settings: (guildId: string) => request<Settings>(`/api/guilds/${guildId}/settings`),
  overlays: (guildId: string) => request<OverlayLinks>(`/api/guilds/${guildId}/overlays`),
  /** New links; the old ones stop working. */
  resetOverlays: (guildId: string) => send<OverlayLinks>('POST', `/api/guilds/${guildId}/overlays`, {}),
  saveSettings: (guildId: string, form: SettingsForm) =>
    send<Settings>('PUT', `/api/guilds/${guildId}/settings`, form),
  channels: (guildId: string) => request<Channel[]>(`/api/guilds/${guildId}/channels`),
  /** Every server member except bots. */
  members: (guildId: string) => request<MemberResult[]>(`/api/guilds/${guildId}/members`),
  /** Resolves with the camp once the bot has also updated Discord. */
  act: (campId: string, action: CampAction) => post<Camp>(`/api/camps/${campId}/actions`, action),
  createCamp: (guildId: string, channelId: string, size: CampSizeChoice) =>
    post<Camp>(`/api/guilds/${guildId}/camps`, { channel_id: channelId, size }),
  site: (campId: string) => request<SiteState>(`/api/camps/${campId}/site`),
  /** Saves the camp's info cards; they come back cleaned up. */
  saveSite: (campId: string, info: SiteInfo) => send<SiteState>('PUT', `/api/camps/${campId}/site`, { info }),
  /** The site shows this camp's ranking, matches and info from now on. */
  publishSite: (campId: string) => post<SiteState>(`/api/camps/${campId}/site/publish`, {}),
  matchStats: (matchId: string) => request<MatchStats>(`/api/matches/${matchId}/stats`),
  /** Looks the match up on the Deadlock API by this ID; the same ID again retries. */
  setDeadlockId: (matchId: string, deadlockMatchId: string) =>
    send<MatchStats>('PUT', `/api/matches/${matchId}/stats`, { deadlock_match_id: deadlockMatchId }),
  /** Who plays on a Steam account, or `null` to unlink it. */
  linkAccount: (guildId: string, accountId: string, userId: string | null) =>
    send<Record<string, never>>('PUT', `/api/guilds/${guildId}/deadlock-accounts/${accountId}`, { user_id: userId }),
};

function post<T>(path: string, body: unknown): Promise<T> {
  return send<T>('POST', path, body);
}

function send<T>(method: string, path: string, body: unknown): Promise<T> {
  return request<T>(path, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}
