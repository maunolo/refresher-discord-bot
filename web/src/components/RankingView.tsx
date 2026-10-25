import { useState } from 'react';

import { useActions } from '../actions';
import type { Camp, Player } from '../api';
import { playerName, ranking, signed, whereIs, winRate } from '../camp';
import { DiscordLink, SRankChip } from './common';
import { PointsDialog } from './PointsDialog';

/** Every player by points, with wins and matches; ties share a place, like the bot's medals. */
export function RankingView({ camp }: { camp: Camp }) {
  const rows = ranking(camp.players);
  const ended = camp.status === 'ended';
  const running = camp.status === 'running';
  const [adjusting, setAdjusting] = useState<Player | null>(null);
  const { busy } = useActions();
  return (
    <section className="stack" aria-labelledby="ranking-page-title">
      <div className="card-head">
        <div className="title-row">
          <h1 id="ranking-page-title" className="page-title">
            {ended ? 'Ranking final' : 'Ranking'}
          </h1>
          <span className="muted small">
            {camp.match_count} {camp.match_count === 1 ? 'partida jogada' : 'partidas jogadas'}
          </span>
        </div>
        {ended && <DiscordLink url={camp.discord_url} label="Ver no Discord" />}
      </div>
      {rows.length === 0 ? (
        <section className="card empty-card">
          <h2>Ninguém no elenco ainda</h2>
          <p className="muted small">O ranking aparece quando o camp tiver jogadores.</p>
        </section>
      ) : (
        <div className="card table-scroll">
          <table className="table">
            <thead>
              <tr>
                <th scope="col">Lugar</th>
                <th scope="col">Jogador</th>
                <th scope="col" className="align-end">
                  Pontos
                </th>
                <th scope="col" className="align-end">
                  MVPs
                </th>
                <th scope="col" className="align-end">
                  Vitórias
                </th>
                <th scope="col" className="align-end">
                  Partidas
                </th>
                <th scope="col" className="align-end">
                  Aproveitamento
                </th>
                {!ended && <th scope="col">Agora</th>}
                {running && (
                  <th scope="col" className="align-end">
                    <span className="visually-hidden">Ajustar pontos</span>
                  </th>
                )}
              </tr>
            </thead>
            <tbody>
              {rows.map(({ player, place }) => (
                <tr key={player.user_id}>
                  <td>
                    <span className={`medal medal-${Math.min(place, 4)}`}>{place}</span>
                  </td>
                  <td>
                    <span className="player-cell">
                      {playerName(camp, player.user_id)}
                      {player.s_rank && <SRankChip />}
                    </span>
                  </td>
                  <td className="align-end mono">
                    {player.points_adjusted !== 0 && (
                      <span className="muted small" title="Ajustes manuais">
                        ({signed(player.points_adjusted)}){' '}
                      </span>
                    )}
                    <span className="strong">{player.points}</span>
                  </td>
                  <td className="align-end mono">{player.mvps || <span className="faint">0</span>}</td>
                  <td className="align-end mono">{player.wins}</td>
                  <td className="align-end mono">{player.matches_played}</td>
                  <td className="align-end mono muted">{winRate(player)}</td>
                  {!ended && <td className="muted">{whereIs(camp, player)}</td>}
                  {running && (
                    <td className="align-end">
                      <button
                        type="button"
                        className="button button-small button-ghost"
                        disabled={busy}
                        aria-label={`Ajustar pontos de ${playerName(camp, player.user_id)}`}
                        onClick={() => setAdjusting(player)}
                      >
                        Ajustar
                      </button>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="muted small">
        Cada vitória vale 1 ponto; ajustes manuais aparecem entre parênteses. Empate em pontos é desfeito pelo número de
        MVPs; empatados nos dois dividem o lugar. Aproveitamento conta só partidas terminadas; correções de resultado entram
        na hora.
      </p>
      <PointsDialog camp={camp} player={adjusting} onClose={() => setAdjusting(null)} />
    </section>
  );
}
