import { createContext, useContext, useEffect, useMemo, useRef, useState } from 'react';

export interface Live {
  /** The live updates stream is open: changes show up as they happen. */
  connected: boolean;
  /** Goes up each time the server's camp changes; pass it to `usePolling` as its signal. */
  version: number;
}

export const LiveContext = createContext<Live>({ connected: false, version: 0 });

export function useLive(): Live {
  return useContext(LiveContext);
}

/** How often to load when live updates are down, and as a safety net while they're up. */
export function pollEvery(live: Live, offlineMs: number): number {
  return live.connected ? 60_000 : offlineMs;
}

/**
 * Follows a server's live updates (`/api/guilds/{id}/events`): `version` goes up whenever its camp changes,
 * whether from this page, another admin, a Discord button or the Queue voice channel, and again after a
 * reconnect, since changes may have gone by while the stream was down. The browser reconnects by itself.
 */
export function useLiveUpdates(guildId: string | null): Live {
  const [connected, setConnected] = useState(false);
  const [version, setVersion] = useState(0);
  const opened = useRef(false);

  useEffect(() => {
    setConnected(false);
    opened.current = false;
    if (guildId === null || typeof EventSource === 'undefined') return;
    const source = new EventSource(`/api/guilds/${guildId}/events`);
    source.onopen = () => {
      setConnected(true);
      if (opened.current) setVersion((v) => v + 1);
      opened.current = true;
    };
    source.onerror = () => setConnected(false);
    source.addEventListener('change', () => setVersion((v) => v + 1));
    return () => source.close();
  }, [guildId]);

  return useMemo(() => ({ connected, version }), [connected, version]);
}
