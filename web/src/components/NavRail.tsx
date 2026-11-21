import type { ReactNode } from 'react';

import { routeHref, type Route } from '../route';
import {
  BookIcon,
  CampsIcon,
  GlobeIcon,
  LiveIcon,
  MatchesIcon,
  MonitorIcon,
  PlayersIcon,
  QueueIcon,
  SettingsIcon,
  TrophyIcon,
} from './Icons';

const SECTIONS: { label: string; icon: ReactNode; route: Route }[] = [
  { label: 'Ao vivo', icon: <LiveIcon />, route: 'live' },
  { label: 'Fila', icon: <QueueIcon />, route: 'queue' },
  { label: 'Jogadores', icon: <PlayersIcon />, route: 'players' },
  { label: 'Partidas', icon: <MatchesIcon />, route: 'matches' },
  { label: 'Ranking', icon: <TrophyIcon />, route: 'ranking' },
  { label: 'Camps', icon: <CampsIcon />, route: 'camps' },
  { label: 'Overlays', icon: <MonitorIcon />, route: 'overlays' },
  { label: 'Configurações', icon: <SettingsIcon />, route: 'settings' },
  { label: 'Site', icon: <GlobeIcon />, route: 'site' },
  { label: 'Regras', icon: <BookIcon />, route: 'rules' },
];

/** `site` shows Site, for the server that has the public site. */
export function NavRail({ route, site }: { route: Route; site: boolean }) {
  return (
    <nav className="nav" aria-label="Seções">
      {SECTIONS.filter((s) => site || s.route !== 'site').map((s) => (
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
