import { useActions } from '../actions';
import type { Camp, RecentMatch } from '../api';
import { SIDE_LABEL, playerName, points, queuedPlayers, ranking } from '../camp';
import { routeHref } from '../route';
import { useState } from 'react';

import { CardLink, DiscordLink, MvpTag, SRankChip } from './common';
import { MvpDialog } from './ResultDialogs';

const PLAYERS_MOVED_ON = 'Os jogadores já estão em outra partida.';

export function Corrections({ camp, match }: { camp: Camp; match: RecentMatch }) {
  const { run, busy } = useActions();
  const [changingMvp, setChangingMvp] = useState(false);
  const other = match.winner_side === 'hiddenking' ? 'archmother' : 'hiddenking';
  const off = busy || !match.correctable;
  const why = match.correctable ? undefined : PLAYERS_MOVED_ON;
  return (
    <div className="inline-actions">
      <button
        type="button"
        className="button button-ghost button-small"
        disabled={off}
        title={why}
        onClick={() =>
          run(
            { type: 'correct_winner', match_id: match.id },
            {
              confirm: {
                title: `Trocar o vencedor da partida #${match.number}?`,
                message: `${SIDE_LABEL[other]} passa a ser o vencedor. Os pontos e a fila são refeitos.`,
                action: `${SIDE_LABEL[other]} venceu`,
                tone: other,
              },
            },
          )
        }
      >
        Trocar vencedor
      </button>
      <button
        type="button"
        className="button button-ghost button-small"
        disabled={off}
        title={why}
        onClick={() =>
          run(
            { type: 'reopen_match', match_id: match.id },
            {
              confirm: {
                title: `Reabrir a partida #${match.number}?`,
                message: 'O ponto dos vencedores é desfeito e a partida volta a ficar ao vivo.',
                note: 'Os jogadores voltam para os canais de voz dos times.',
                action: 'Reabrir partida',
              },
            },
          )
        }
      >
        Reabrir
      </button>
      <button
        type="button"
        className="button button-ghost button-small"
        disabled={busy || !match.mvp_changeable}
        title={match.mvp_changeable ? undefined : 'Só a última partida, com o camp em andamento.'}
        onClick={() => setChangingMvp(true)}
      >
        Trocar MVP
      </button>
      <MvpDialog camp={camp} match={changingMvp ? match : null} onClose={() => setChangingMvp(false)} />
    </div>
  );
}

export function QueuePanel({ camp }: { camp: Camp }) {
  const queue = queuedPlayers(camp);
  const upNext = new Set(camp.up_next);
  const missing = camp.match_size - queue.length;
  return (
    <aside className="card queue" aria-labelledby="queue-title">
      <div className="card-head">
        <div className="title-row">
          <h2 id="queue-title">Fila</h2>
          <span className="mono muted small">
            {queue.length} {queue.length === 1 ? 'jogador' : 'jogadores'}
          </span>
        </div>
        <CardLink href={routeHref('queue')} label="Abrir fila" />
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
      <DiscordLink url={camp.discord_url} />
    </aside>
  );
}

export function HistoryPanel({ camp }: { camp: Camp }) {
  return (
    <section className="card history" aria-labelledby="history-title">
      <div className="card-head">
        <h2 id="history-title">Últimas partidas</h2>
        <CardLink href={routeHref('matches')} label="Ver todas" />
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
                <th scope="col">MVP</th>
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
                  <td>{m.mvp_user_id ? playerName(camp, m.mvp_user_id) : <span className="faint">—</span>}</td>
                  <td className="align-end">
                    {i === 0 ? (
                      <Corrections camp={camp} match={m} />
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
      <p className="muted small">
        Só a última partida pode ser corrigida: vencedor e reabrir enquanto os jogadores dela estão na fila, o MVP enquanto o
        camp estiver em andamento.
      </p>
    </section>
  );
}

export function RankingPanel({ camp }: { camp: Camp }) {
  const top = ranking(camp.players).slice(0, 8);
  return (
    <section className="card ranking" aria-labelledby="ranking-title">
      <div className="card-head">
        <h2 id="ranking-title">Ranking</h2>
        <CardLink href={routeHref('ranking')} label="Ver completo" />
      </div>
      <ol className="rows">
        {top.map(({ player, place }) => (
          <li key={player.user_id} className="row row-tall">
            <span className={`medal medal-${Math.min(place, 4)}`}>{place}</span>
            <span className="ellipsis">{playerName(camp, player.user_id)}</span>
            {player.mvps > 0 && <MvpTag />}
            <span className="row-end mono small">
              {points(player.points)}
              {player.mvps > 0 && <span className="muted"> · {player.mvps}</span>}
            </span>
          </li>
        ))}
      </ol>
    </section>
  );
}
