import type { Camp } from '../api';
import { playerName, ranking, whereIs, winRate } from '../camp';
import { DiscordLink, SRankChip } from './common';

/** Every player by points, with wins and matches; ties share a place, like the bot's medals. */
export function RankingView({ camp }: { camp: Camp }) {
  const rows = ranking(camp.players);
  const ended = camp.status === 'ended';
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
                  Vitórias
                </th>
                <th scope="col" className="align-end">
                  Partidas
                </th>
                <th scope="col" className="align-end">
                  Aproveitamento
                </th>
                {!ended && <th scope="col">Agora</th>}
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
                  <td className="align-end mono strong">{player.points}</td>
                  <td className="align-end mono">{player.wins}</td>
                  <td className="align-end mono">{player.matches_played}</td>
                  <td className="align-end mono muted">{winRate(player)}</td>
                  {!ended && <td className="muted">{whereIs(camp, player)}</td>}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="muted small">
        Cada vitória vale 1 ponto. Empatados dividem o lugar. Aproveitamento conta só partidas terminadas; correções de
        resultado entram na hora.
      </p>
    </section>
  );
}
