import type { Camp, CampSummary, Guild, Me } from '../api';
import { campStatusLabel, formatLabel } from '../camp';
import { SOON } from './common';

interface HeaderProps {
  me: Me;
  guildId: string;
  onGuild: (id: string) => void;
  camps: CampSummary[];
  campId: string | null;
  onCamp: (id: string) => void;
  camp: Camp | null;
}

function StatusPill({ status }: { status: string }) {
  const tone = status === 'running' ? 'pill-running' : status === 'signup' ? 'pill-neutral' : 'pill-muted';
  return (
    <span className={`pill ${tone}`}>
      <span className="dot" />
      {campStatusLabel(status)}
    </span>
  );
}

function GuildPicker({ guilds, value, onChange }: { guilds: Guild[]; value: string; onChange: (id: string) => void }) {
  if (guilds.length < 2) return null;
  return (
    <label className="picker">
      <span className="visually-hidden">Servidor</span>
      <select value={value} onChange={(e) => onChange(e.target.value)}>
        {guilds.map((g) => (
          <option key={g.id} value={g.id}>
            {g.name}
          </option>
        ))}
      </select>
    </label>
  );
}

export function Header({ me, guildId, onGuild, camps, campId, onCamp, camp }: HeaderProps) {
  const sRank = camp?.players.filter((p) => p.s_rank).length ?? 0;
  return (
    <header className="header">
      <div className="header-group">
        <div className="brand">
          <img src="/logo.png" alt="" width={40} height={40} />
          <span className="brand-name">Refresher</span>
          <span className="muted small">Admin</span>
        </div>
        <GuildPicker guilds={me.guilds} value={guildId} onChange={onGuild} />
        {camps.length > 0 && campId && (
          <label className="picker">
            <span className="visually-hidden">Camp</span>
            <select value={campId} onChange={(e) => onCamp(e.target.value)}>
              {camps.map((c) => (
                <option key={c.id} value={c.id}>
                  Camp {c.id} · {campStatusLabel(c.status)}
                </option>
              ))}
            </select>
          </label>
        )}
        {camp && <StatusPill status={camp.status} />}
        {camp && (
          <span className="muted small">
            {camp.players.length} jogadores · {formatLabel(camp.match_size)} · {camp.match_count}{' '}
            {camp.match_count === 1 ? 'partida jogada' : 'partidas jogadas'} · {sRank} S rank
          </span>
        )}
      </div>
      <div className="header-group">
        {camp?.status === 'running' && (
          <>
            <button type="button" className="button" disabled title={SOON}>
              Criar partida
            </button>
            <button type="button" className="button" disabled title={SOON}>
              Partida final
            </button>
          </>
        )}
        <div className="account">
          {me.user.avatar_url && <img src={me.user.avatar_url} alt="" width={32} height={32} className="avatar" />}
          <span className="small">{me.user.name}</span>
          <form method="post" action="/auth/logout">
            <button type="submit" className="button button-ghost button-small">
              Sair
            </button>
          </form>
        </div>
      </div>
    </header>
  );
}
