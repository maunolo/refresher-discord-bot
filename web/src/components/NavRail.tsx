import type { ReactNode } from 'react';

import { routeHref, type Route } from '../route';
import { BookIcon, CampsIcon, LiveIcon, MatchesIcon, PlayersIcon, QueueIcon, SettingsIcon, TrophyIcon } from './Icons';

const SECTIONS: { label: string; icon: ReactNode; route: Route }[] = [
  { label: 'Ao vivo', icon: <LiveIcon />, route: 'live' },
  { label: 'Fila', icon: <QueueIcon />, route: 'queue' },
  { label: 'Jogadores', icon: <PlayersIcon />, route: 'players' },
  { label: 'Partidas', icon: <MatchesIcon />, route: 'matches' },
  { label: 'Ranking', icon: <TrophyIcon />, route: 'ranking' },
  { label: 'Camps', icon: <CampsIcon />, route: 'camps' },
  { label: 'Configurações', icon: <SettingsIcon />, route: 'settings' },
  { label: 'Regras', icon: <BookIcon />, route: 'rules' },
];

export function NavRail({ route }: { route: Route }) {
  return (
    <nav className="nav" aria-label="Seções">
      {SECTIONS.map((s) => (
        <a
          key={s.label}
          href={routeHref(s.route)}
          className={s.route === route ? 'nav-item nav-item-active' : 'nav-item'}
          aria-current={s.route === route ? 'page' : undefined}
        >
          {s.icon}
          {s.label}
        </a>
      ))}
    </nav>
  );
}
