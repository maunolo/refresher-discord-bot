import { useState, type FormEvent } from 'react';

import { api, type Camp, type Match, type MatchStats, type StatsPlayer } from '../api';
import { SIDES, SIDE_LABEL, playerName } from '../camp';
import { useLive } from '../live';
import { usePolling } from '../usePolling';

const loadStats = (matchId: string) => api.matchStats(matchId);

function at(seconds: number): string {
  return new Date(seconds * 1000).toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' });
}

function duration(seconds: number): string {
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}

function souls(n: number): string {
  return n < 1000 ? String(n) : `${(n / 1000).toFixed(n < 9950 ? 1 : 0).replace('.0', '')}k`;
}

/** What the bot is doing about the match's scoreboard, in a sentence. */
function statusText(stats: MatchStats): string {
  switch (stats.status) {
    case 'pending':
      if (!stats.deadlock_match_id) return 'Procurando a partida pelo lobby do bot na API do Deadlock…';
      return stats.next_attempt_at
        ? `Esperando a API do Deadlock ter os dados da partida. Próxima tentativa às ${at(stats.next_attempt_at)}.`
        : 'Buscando os dados da partida na API do Deadlock…';
    case 'needs_id':
      return 'Informe o ID da partida no Deadlock para o placar ir para o site.';
    case 'failed':
      return stats.error ?? 'A API do Deadlock não teve os dados da partida.';
    default:
      return '';
  }
}

/** Who plays on a Steam account: the match's players on that side, or nobody. */
function AccountSelect({
  camp,
  match,
  player,
  busy,
  onLink,
}: {
  camp: Camp;
  match: Match;
  player: StatsPlayer;
  busy: boolean;
  onLink: (userId: string | null) => void;
}) {
  const side = match.players.filter((p) => p.side === player.side).map((p) => p.user_id);
  const options = player.user_id && !side.includes(player.user_id) ? [...side, player.user_id] : side;
  return (
    <select
      className="input stats-select"
      value={player.user_id ?? ''}
      disabled={busy}
      aria-label={`Quem jogou de ${player.hero}`}
      onChange={(e) => onLink(e.target.value || null)}
    >
      <option value="">{player.steam_name ? `Steam: ${player.steam_name}` : 'Sem vínculo'}</option>
      {options.map((id) => (
        <option key={id} value={id}>
          {playerName(camp, id)}
        </option>
      ))}
    </select>
  );
}

/**
 * A finished match's scoreboard from the Deadlock API, which the public site's match history shows.
 * The bot looks it up by its lobby; without one, an admin gives the match ID.
 */
export function MatchStatsCard({ camp, match }: { camp: Camp; match: Match }) {
  const live = useLive();
  const stats = usePolling<MatchStats>(match.id, loadStats, 15_000, live.version);
  const [idInput, setIdInput] = useState('');
  const [changingId, setChangingId] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const data = stats.data;

  if (!data) {
    return (
      <section className="card">
        <h2 className="small-title">Placar do Deadlock</h2>
        <p className={stats.error ? 'error small' : 'muted small'}>{stats.error?.message ?? 'Carregando…'}</p>
      </section>
    );
  }

  const submit = (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    api
      .setDeadlockId(match.id, idInput || data.deadlock_match_id || '')
      .then((updated) => {
        stats.replace(updated);
        setIdInput('');
        setChangingId(false);
      })
      .catch((err: Error) => setError(err.message))
      .finally(() => setBusy(false));
  };

  const link = (player: StatsPlayer, userId: string | null) => {
    setBusy(true);
    setError(null);
    api
      .linkAccount(camp.guild_id, player.account_id, userId)
      .then(() => stats.refresh())
      .catch((err: Error) => setError(err.message))
      .finally(() => setBusy(false));
  };

  const askId = data.status === 'needs_id' || data.status === 'failed' || changingId;
  const board = data.scoreboard;

  return (
    <section className="card stack" aria-labelledby="stats-title">
      <div className="card-head">
        <h2 id="stats-title" className="small-title">
          Placar do Deadlock
        </h2>
        {data.deadlock_match_id && <span className="mono muted small">#{data.deadlock_match_id}</span>}
      </div>
      {data.status !== 'ready' && <p className={data.status === 'failed' ? 'error small' : 'small'}>{statusText(data)}</p>}
      {data.status === 'needs_id' && data.error && <p className="muted small">{data.error}</p>}

      {board && (
        <>
          <p className="muted small">
            {SIDE_LABEL[board.winner_side]} venceu em {duration(board.duration_s)}. Vincule cada conta da Steam a um
            jogador para o site mostrar o nome do Discord; o vínculo vale para as próximas partidas.
          </p>
          {SIDES.map((side) => (
            <div key={side} className="table-scroll">
              <table className="table stats-table">
                <caption className={`tag tag-${side}`}>{SIDE_LABEL[side]}</caption>
                <thead>
                  <tr>
                    <th scope="col">Herói</th>
                    <th scope="col">K / D / A</th>
                    <th scope="col">Almas</th>
                    <th scope="col">Jogador</th>
                  </tr>
                </thead>
                <tbody>
                  {board.players
                    .filter((p) => p.side === side)
                    .map((p) => (
                      <tr key={p.account_id}>
                        <td>
                          <span className="stats-hero">
                            {p.hero_image && <img src={p.hero_image} alt="" width={28} height={28} loading="lazy" />}
                            {p.hero}
                          </span>
                        </td>
                        <td className="mono">
                          {p.kills} / {p.deaths} / {p.assists}
                        </td>
                        <td className="mono">{souls(p.souls)}</td>
                        <td>
                          <AccountSelect camp={camp} match={match} player={p} busy={busy} onLink={(id) => link(p, id)} />
                        </td>
                      </tr>
                    ))}
                </tbody>
              </table>
            </div>
          ))}
        </>
      )}

      {askId ? (
        <form className="stats-id" onSubmit={submit}>
          <label className="field">
            <span className="small">ID da partida no Deadlock</span>
            <input
              className="input mono"
              inputMode="numeric"
              placeholder={data.deadlock_match_id ?? '139086680'}
              value={idInput}
              onChange={(e) => setIdInput(e.target.value)}
            />
          </label>
          <div className="form-actions">
            <button
              type="submit"
              className="button button-small"
              disabled={busy || !(idInput.trim() || data.deadlock_match_id)}
            >
              {data.status === 'failed' && !idInput.trim() ? 'Buscar de novo' : 'Buscar'}
            </button>
            {changingId && (
              <button type="button" className="button button-small button-ghost" onClick={() => setChangingId(false)}>
                Cancelar
              </button>
            )}
          </div>
          <span className="muted small">
            O ID aparece no histórico de partidas do Deadlock (em Partidas, no perfil) e na tela do fim da partida.
          </span>
        </form>
      ) : (
        data.status !== 'none' && (
          <div className="form-actions">
            <button type="button" className="button button-small button-ghost" onClick={() => setChangingId(true)}>
              {data.deadlock_match_id ? 'Trocar o ID da partida' : 'Informar o ID da partida'}
            </button>
          </div>
        )
      )}
      {error && (
        <p className="error small" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
