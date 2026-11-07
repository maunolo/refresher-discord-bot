import { useCallback, useEffect, useState } from 'react';

/** Screens of the admin page, kept in the URL hash so the server only serves index.html. */
export type Route = 'live' | 'queue' | 'players' | 'matches' | 'ranking' | 'camps' | 'settings' | 'rules';

const HASHES: Record<Route, string> = {
  live: '#/',
  queue: '#/fila',
  players: '#/jogadores',
  matches: '#/partidas',
  ranking: '#/ranking',
  camps: '#/camps',
  settings: '#/configuracoes',
  rules: '#/regras',
};

/** Where the page is: a screen, and on Partidas the match open in it. */
export interface Place {
  route: Route;
  matchId: string | null;
}

function current(): Place {
  const hash = window.location.hash || '#/';
  const match = /^#\/partidas\/([A-Za-z0-9]+)$/.exec(hash);
  if (match) return { route: 'matches', matchId: match[1].toUpperCase() };
  const found = (Object.keys(HASHES) as Route[]).find((r) => HASHES[r] === hash);
  return { route: found ?? 'live', matchId: null };
}

export function routeHref(route: Route): string {
  return HASHES[route];
}

export function matchHref(matchId: string): string {
  return `${HASHES.matches}/${matchId}`;
}

export function useRoute(): [Place, (route: Route) => void] {
  const [place, setPlace] = useState<Place>(current);
  useEffect(() => {
    const onChange = () => setPlace(current());
    window.addEventListener('hashchange', onChange);
    return () => window.removeEventListener('hashchange', onChange);
  }, []);
  const go = useCallback((next: Route) => {
    window.location.hash = HASHES[next];
  }, []);
  return [place, go];
}
