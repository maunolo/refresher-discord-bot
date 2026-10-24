import type { ReactNode } from 'react';

import { CampsIcon, LiveIcon, MatchesIcon, PlayersIcon, QueueIcon, SettingsIcon, TrophyIcon } from './Icons';

const SECTIONS: { label: string; icon: ReactNode; ready: boolean }[] = [
  { label: 'Ao vivo', icon: <LiveIcon />, ready: true },
  { label: 'Fila', icon: <QueueIcon />, ready: false },
  { label: 'Jogadores', icon: <PlayersIcon />, ready: false },
  { label: 'Partidas', icon: <MatchesIcon />, ready: false },
  { label: 'Ranking', icon: <TrophyIcon />, ready: false },
  { label: 'Camps', icon: <CampsIcon />, ready: false },
  { label: 'Configurações', icon: <SettingsIcon />, ready: false },
];

/** Only "Ao vivo" exists so far; the other sections come with the next steps. */
export function NavRail() {
  return (
    <nav className="nav" aria-label="Seções">
      {SECTIONS.map((s) =>
        s.ready ? (
          <a key={s.label} href="/" className="nav-item nav-item-active" aria-current="page">
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
