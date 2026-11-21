import { Avatar } from './Avatar';
import { useRanking, type OverlayRank } from './data';

function Row({ player }: { player: OverlayRank }) {
  const medal = player.place <= 3 ? ` ov-place-${player.place}` : '';
  return (
    <li className={player.place === 1 ? 'ov-rank-row ov-rank-first' : 'ov-rank-row'}>
      <span className={`ov-place${medal}`}>{player.place}</span>
      <Avatar name={player.name} url={player.avatar_url} size={34} />
      <div className="ov-rank-who">
        <div className="ov-rank-name">
          <span className="ov-name">{player.name}</span>
          {player.mvps > 0 && <span className="ov-mvp">{player.mvps} MVP</span>}
        </div>
        <div className="ov-rank-sub">
          {player.wins} V · {player.played - player.wins} D
        </div>
      </div>
      <div className="ov-pts">
        {player.points}
        <small>{player.points === 1 ? 'PT' : 'PTS'}</small>
      </div>
    </li>
  );
}

/**
 * The camp's top `top` by points, then MVPs, on a transparent background for OBS, with wins and
 * losses.
 */
export function RankingOverlay({ token, top }: { token: string; top: number }) {
  const ranking = useRanking(token);
  if (!ranking) return null;
  const played = ranking.match_count;
  return (
    <div className="ov">
      <section className="ov-panel ov-rank">
        <header className="ov-rank-head">
          <h1>{ranking.ended ? 'RANKING FINAL' : 'RANKING'}</h1>
          <span className="ov-muted">
            Refresher League · {played} {played === 1 ? 'partida' : 'partidas'}
          </span>
        </header>
        {played === 0 ? (
          <p className="ov-empty">O ranking aparece depois da primeira partida.</p>
        ) : (
          <>
            <ol className="ov-rank-list">
              {ranking.players.slice(0, top).map((p) => (
                <Row key={p.user_id} player={p} />
              ))}
            </ol>
            <footer className="ov-rank-foot">Empate em pontos: desempata quem tem mais MVPs.</footer>
          </>
        )}
      </section>
    </div>
  );
}
