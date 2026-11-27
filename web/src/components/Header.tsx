import { useState } from 'react';

import { confirmEnterFinals, confirmLeaveFinals, createMatchBlocked, enterFinalsBlocked, useActions } from '../actions';
import type { Camp, CampSummary, Guild, Me } from '../api';
import { campStatusLabel, formatLabel } from '../camp';
import { CreateMatchDialog } from './CreateMatchDialog';

interface HeaderProps {
  me: Me;
  guildId: string;
  onGuild: (id: string) => void;
  camps: CampSummary[];
  campId: string | null;
  onCamp: (id: string) => void;
  camp: Camp | null;
  /** Live updates are connected. */
  live: boolean;
}

/** Whether changes show up as they happen, or only on the next reload while the live updates reconnect. */
function LiveStatus({ live }: { live: boolean }) {
  return live ? (
    <span className="live-status live-status-on" title="A página se atualiza sozinha quando algo muda no camp, aqui ou no Discord.">
      <span className="dot" />
      Tempo real
    </span>
  ) : (
    <span className="live-status" title="Sem conexão em tempo real: a página recarrega a cada 4 segundos enquanto reconecta.">
      <span className="dot" />
      Reconectando…
    </span>
  );
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

function FinalsButton({ camp }: { camp: Camp }) {
  const { run, busy } = useActions();
  if (camp.finals) {
    return (
      <button
        type="button"
        className="button button-ghost"
        disabled={busy || !camp.can_leave_finals}
        title={camp.can_leave_finals ? 'Voltar a criar as partidas pela fila.' : 'Termine ou cancele a partida da final antes.'}
        onClick={() => run({ type: 'leave_finals' }, { confirm: confirmLeaveFinals() })}
      >
        Sair da final
      </button>
    );
  }
  const blocked = enterFinalsBlocked(camp);
  return (
    <button
      type="button"
      className="button"
      disabled={busy || blocked !== undefined}
      title={blocked ?? `Daqui em diante, as partidas são entre os ${camp.match_size} primeiros do ranking.`}
      onClick={() => run({ type: 'enter_finals' }, { confirm: confirmEnterFinals(camp) })}
    >
      Entrar na final
    </button>
  );
}

function CampButtons({ camp }: { camp: Camp }) {
  const { busy } = useActions();
  const [creating, setCreating] = useState(false);
  const matchBlocked = createMatchBlocked(camp);
  return (
    <>
      {camp.finals && <span className="pill pill-final">Final</span>}
      <button
        type="button"
        className="button"
        disabled={busy || matchBlocked !== undefined}
        title={matchBlocked}
        onClick={() => setCreating(true)}
      >
        {camp.finals ? 'Criar partida da final' : 'Criar partida'}
      </button>
      <FinalsButton camp={camp} />
      <CreateMatchDialog camp={camp} open={creating} onClose={() => setCreating(false)} />
    </>
  );
}

function summary(camp: Camp, sRank: number, ssRank: number): string {
  const played = camp.match_count === 1 ? 'partida jogada' : 'partidas jogadas';
  return `${camp.players.length} jogadores · ${formatLabel(camp.match_size)} · ${camp.match_count} ${played} · ${ssRank} SS rank · ${sRank} S rank`;
}

export function Header({ me, guildId, onGuild, camps, campId, onCamp, camp, live }: HeaderProps) {
  const sRank = camp?.players.filter((p) => p.s_rank).length ?? 0;
  const ssRank = camp?.players.filter((p) => p.ss_rank).length ?? 0;
  return (
    <header className="header">
      <div className="header-group">
        <div className="brand">
          <img src="/logo-dragon.webp" alt="" width={34} height={34} />
          <span className="brand-name">
            Refresher <span>League</span>
          </span>
          <span className="brand-tag">Admin</span>
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
          <span className="muted small header-summary" title={summary(camp, sRank, ssRank)}>
            {summary(camp, sRank, ssRank)}
          </span>
        )}
      </div>
      <div className="header-group">
        {camp && <LiveStatus live={live} />}
        {camp?.status === 'running' && <CampButtons camp={camp} />}
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
