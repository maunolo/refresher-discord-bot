import { Avatar } from './Avatar';
import { useRanking, type OverlayRank } from './data';

/** ▲ up, ▼ down, — same place since the latest result; nothing before there are two results. */
function Movement({ moved }: { moved: number | null }) {
  if (moved === null) return <span className="ov-move" />;
  if (moved > 0) return <span className="ov-move ov-up">▲{moved}</span>;
  if (moved < 0) return <span className="ov-move ov-down">▼{-moved}</span>;
  return <span className="ov-move ov-same">—</span>;
}

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
          {player.scored_last && <span className="ov-plus">+1</span>}
        </div>
        <div className="ov-rank-sub">
          {player.wins} V · {player.played} J
        </div>
      </div>
      <Movement moved={player.moved} />
      <div className="ov-pts">
        {player.points}
        <small>{player.points === 1 ? 'PT' : 'PTS'}</small>
      </div>
    </li>
  );
}

/**
 * The camp's top `top` by points, then MVPs, on a transparent background for OBS: wins and
 * matches played, who scored in the latest result (+1) and who moved since it (▲ ▼).
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
          {ranking.camp_id && (
            <span className="ov-muted">
              Camp {ranking.camp_id} · {played} {played === 1 ? 'partida' : 'partidas'}
            </span>
          )}
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
