import type { Camp } from '../api';
import { SIDE_LABEL, playerName, points, queuedPlayers, ranking } from '../camp';
import { SOON, SRankChip } from './common';

export function QueuePanel({ camp }: { camp: Camp }) {
  const queue = queuedPlayers(camp);
  const upNext = new Set(camp.up_next);
  const missing = camp.match_size - queue.length;
  return (
    <aside className="card queue" aria-labelledby="queue-title">
      <div className="card-head">
        <h2 id="queue-title">Fila</h2>
        <span className="mono muted small">
          {queue.length} {queue.length === 1 ? 'jogador' : 'jogadores'}
        </span>
      </div>
      <p className="muted small">
        {missing > 0
          ? `Uma partida precisa de ${camp.match_size}. Faltam ${missing}; os próximos entram quando uma partida terminar.`
          : 'Marcados com → entram na próxima partida.'}
      </p>
      {queue.length === 0 ? (
        <p className="empty">Todo mundo está em partida.</p>
      ) : (
        <ol className="rows">
          {queue.map((p, i) => (
            <li key={p.user_id} className="row row-tall">
              <span className="mono faint small queue-pos">{String(i + 1).padStart(2, '0')}</span>
              {missing <= 0 && upNext.has(p.user_id) ? (
                <span className="up-next" title="Entra na próxima partida">
                  →
                </span>
              ) : (
                <span className="up-next" />
              )}
              <span className="ellipsis">{playerName(camp, p.user_id)}</span>
              {p.s_rank && <SRankChip />}
              <span className="row-end muted small">{p.consecutive_matches} seg.</span>
              <span className="mono small points">{points(p.points)}</span>
            </li>
          ))}
        </ol>
      )}
      <p className="muted small">
        Depois de cada partida: quem ficou de fora vai na frente; depois, quem jogou menos partidas seguidas, com
        vencedores antes dos perdedores.
      </p>
    </aside>
  );
}

export function HistoryPanel({ camp }: { camp: Camp }) {
  return (
    <section className="card history" aria-labelledby="history-title">
      <div className="card-head">
        <h2 id="history-title">Últimas partidas</h2>
      </div>
      {camp.recent_matches.length === 0 ? (
        <p className="empty">Nenhuma partida terminou ainda.</p>
      ) : (
        <div className="table-scroll">
          <table className="table">
            <thead>
              <tr>
                <th scope="col">Partida</th>
                <th scope="col">ID</th>
                <th scope="col">Vencedor</th>
                <th scope="col">Capitães</th>
                <th scope="col" className="align-end">
                  Correção
                </th>
              </tr>
            </thead>
            <tbody>
              {camp.recent_matches.map((m, i) => (
                <tr key={m.id}>
                  <td>
                    #{m.number}
                    {m.is_final && <span className="muted"> · final</span>}
                  </td>
                  <td className="mono">{m.id}</td>
                  <td>
                    <span className={`tag tag-${m.winner_side}`}>{SIDE_LABEL[m.winner_side]}</span>
                  </td>
                  <td className="muted">{m.captains.map((c) => playerName(camp, c.user_id)).join(' · ') || '—'}</td>
                  <td className="align-end">
                    {i === 0 ? (
                      <div className="inline-actions">
                        <button
                          type="button"
                          className="button button-ghost button-small"
                          disabled
                          title={m.correctable ? SOON : 'Os jogadores já estão em outra partida'}
                        >
                          Trocar vencedor
                        </button>
                        <button
                          type="button"
                          className="button button-ghost button-small"
                          disabled
                          title={m.correctable ? SOON : 'Os jogadores já estão em outra partida'}
                        >
                          Reabrir
                        </button>
                      </div>
                    ) : (
                      <span className="faint">—</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="muted small">Só a última partida pode ser corrigida, e só enquanto os jogadores dela estão na fila.</p>
    </section>
  );
}

export function RankingPanel({ camp }: { camp: Camp }) {
  const top = ranking(camp.players).slice(0, 8);
  return (
    <section className="card ranking" aria-labelledby="ranking-title">
      <div className="card-head">
        <h2 id="ranking-title">Ranking</h2>
      </div>
      <ol className="rows">
        {top.map(({ player, place }) => (
          <li key={player.user_id} className="row row-tall">
            <span className={`medal medal-${Math.min(place, 4)}`}>{place}</span>
            <span className="ellipsis">{playerName(camp, player.user_id)}</span>
            <span className="row-end mono small">{points(player.points)}</span>
          </li>
        ))}
      </ol>
    </section>
  );
}
