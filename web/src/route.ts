import { useCallback, useEffect, useState } from 'react';

/** Screens of the admin page, kept in the URL hash so the server only serves index.html. */
export type Route = 'live' | 'players' | 'camps';

const HASHES: Record<Route, string> = {
  live: '#/',
  players: '#/jogadores',
  camps: '#/camps',
};

function current(): Route {
  const found = (Object.keys(HASHES) as Route[]).find((r) => HASHES[r] === window.location.hash);
  return found ?? 'live';
}

export function routeHref(route: Route): string {
  return HASHES[route];
}

export function useRoute(): [Route, (route: Route) => void] {
  const [route, setRoute] = useState<Route>(current);
  useEffect(() => {
    const onChange = () => setRoute(current());
    window.addEventListener('hashchange', onChange);
    return () => window.removeEventListener('hashchange', onChange);
  }, []);
  const go = useCallback((next: Route) => {
    window.location.hash = HASHES[next];
  }, []);
  return [route, go];
}
