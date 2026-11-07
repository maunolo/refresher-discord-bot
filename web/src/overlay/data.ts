import { useEffect, useState } from 'react';

import type { Role, Side } from '../api';

/** What the stream sees of a player: whether they voted, never how. */
export interface OverlayPlayer {
  user_id: string;
  name: string;
  avatar_url: string | null;
  captain: boolean;
  s_rank: boolean;
  role: Role | null;
  voted: boolean;
}

export interface OverlayTeam {
  side: Side;
  /** The side's Refresh votes. */
  refresh: number;
  /** Captains first, then by role. */
  players: OverlayPlayer[];
}

/** A match's refresh vote, from its creation until the draft starts. */
export interface OverlayVote {
  match_id: string;
  slot: number;
  is_final: boolean;
  /** `open` while players vote, then `refreshed` (new teams) or `kept`. */
  state: 'open' | 'refreshed' | 'kept';
  /** Refresh votes one side needs for new teams. */
  needed: number;
  keep: number;
  voted: number;
  total: number;
  /** Hiddenking, then Archmother. */
  teams: OverlayTeam[];
}

/** The secret and the overlay from the page's path: `/overlay/{secret}/{overlay}`. */
export function overlayPath(): { token: string; overlay: string } | null {
  const match = /^\/overlay\/([0-9a-f]+)\/([a-z]+)\/?$/.exec(window.location.pathname);
  return match ? { token: match[1], overlay: match[2] } : null;
}

/** A player in the ranking overlay. */
export interface OverlayRank {
  user_id: string;
  name: string;
  avatar_url: string | null;
  /** 1 is gold. Points first, then MVPs; equal on both shares the place. */
  place: number;
  points: number;
  mvps: number;
  wins: number;
  played: number;
  /** Places moved with the latest result, up positive; `null` until there's an earlier one. */
  moved: number | null;
  /** On the winning side of the latest result. */
  scored_last: boolean;
}

export interface OverlayRanking {
  camp_id: string | null;
  /** Finished matches. */
  match_count: number;
  /** The camp ended: its final ranking. */
  ended: boolean;
  /** Best first. */
  players: OverlayRank[];
}

/**
 * `/api/overlay/{secret}/{endpoint}`, reloaded whenever the server's camp changes (the same live
 * updates as the admin page) and every 30 seconds in case the stream drops unnoticed. `null`
 * until the first load; a failed load keeps what's on screen.
 */
function useOverlayData<T>(token: string, endpoint: string): T | null {
  const [data, setData] = useState<T | null>(null);

  useEffect(() => {
    let cancelled = false;
    let latest = 0;
    const load = async () => {
      const ticket = ++latest;
      try {
        const response = await fetch(`/api/overlay/${token}/${endpoint}`);
        if (!response.ok) throw new Error(`Erro ${response.status}`);
        const body = (await response.json()) as T;
        if (!cancelled && ticket === latest) setData(body);
      } catch (error) {
        // A wrong or replaced link, or the server restarting.
        console.warn(`overlay: could not load ${endpoint}`, error);
      }
    };
    const source = new EventSource(`/api/overlay/${token}/events`);
    source.onopen = () => void load();
    source.addEventListener('change', () => void load());
    const timer = window.setInterval(() => void load(), 30_000);
    void load();
    return () => {
      cancelled = true;
      source.close();
      window.clearInterval(timer);
    };
  }, [token, endpoint]);

  return data;
}

/** The camp's refresh votes, from each match's creation until its draft starts. */
export function useVotes(token: string): OverlayVote[] | null {
  return useOverlayData<{ votes: OverlayVote[] }>(token, 'votes')?.votes ?? null;
}

/** The camp's ranking; after a camp ends, its final ranking until another starts. */
export function useRanking(token: string): OverlayRanking | null {
  return useOverlayData<OverlayRanking>(token, 'ranking');
}
