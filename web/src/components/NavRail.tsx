import type { ReactNode } from 'react';

import { routeHref, type Route } from '../route';
import { CampsIcon, LiveIcon, MatchesIcon, PlayersIcon, QueueIcon, SettingsIcon, TrophyIcon } from './Icons';

const SECTIONS: { label: string; icon: ReactNode; route?: Route }[] = [
  { label: 'Ao vivo', icon: <LiveIcon />, route: 'live' },
  { label: 'Jogadores', icon: <PlayersIcon />, route: 'players' },
  { label: 'Camps', icon: <CampsIcon />, route: 'camps' },
  { label: 'Fila', icon: <QueueIcon /> },
  { label: 'Partidas', icon: <MatchesIcon /> },
  { label: 'Ranking', icon: <TrophyIcon /> },
  { label: 'Configurações', icon: <SettingsIcon /> },
];

/** Sections without a route aren't built yet; the live view covers queue, matches and ranking. */
export function NavRail({ route }: { route: Route }) {
  return (
    <nav className="nav" aria-label="Seções">
      {SECTIONS.map((s) =>
        s.route ? (
          <a
            key={s.label}
            href={routeHref(s.route)}
            className={s.route === route ? 'nav-item nav-item-active' : 'nav-item'}
            aria-current={s.route === route ? 'page' : undefined}
          >
            {s.icon}
            {s.label}
          </a>
        ) : (
          <button key={s.label} type="button" className="nav-item" disabled title="Em breve">
            {s.icon}
            {s.label}
          </button>
        ),
      )}
    </nav>
  );
}
