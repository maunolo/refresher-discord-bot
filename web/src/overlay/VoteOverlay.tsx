import { useEffect, useRef, useState } from 'react';

import type { Side } from '../api';
import { Avatar } from './Avatar';
import { useVotes, type OverlayPlayer, type OverlayVote } from './data';

/** How long a vote's outcome stays up after the vote closes. */
const OUTCOME_MS = 10_000;
/** The fade at the end of it. */
const FADE_MS = 600;

const SIDE_LABEL: Record<Side, string> = { hiddenking: 'Hiddenking', archmother: 'Archmother' };
const ROLE_TAG = { tank: '🛡️', carry: '🎯', support: '✨' } as const;

/** A vote seen open that just closed, shown until `until`. */
interface Ended {
  vote: OverlayVote;
  before: OverlayVote;
  until: number;
}

function matchName(vote: OverlayVote): string {
  return vote.is_final ? 'Partida final' : `Partida ${vote.slot}`;
}

function Player({ player }: { player: OverlayPlayer }) {
  return (
    <span className="ov-who">
      <Avatar name={player.name} url={player.avatar_url} />
      {player.captain && <span className="ov-crown">👑</span>}
      <span className="ov-name">{player.name}</span>
      {player.s_rank && <span className="ov-s">S</span>}
      {player.role && <span className="ov-role">{ROLE_TAG[player.role]}</span>}
    </span>
  );
}

function Pips({ count, needed, side }: { count: number; needed: number; side: Side }) {
  return (
    <div className={`ov-pips ov-pips-${side}`} style={{ gridTemplateColumns: `repeat(${needed}, 1fr)` }}>
      {Array.from({ length: needed }, (_, i) => (
        <i key={i} className={i < count ? 'on' : undefined} />
      ))}
    </div>
  );
}

function OpenVote({ vote }: { vote: OverlayVote }) {
  return (
    <section className="ov-panel">
      <header className="ov-top">
        <div>
          <div className="ov-label ov-jade">{matchName(vote)}</div>
          <h1>Votação de refresh</h1>
        </div>
        <span className="ov-pill">
          {vote.voted}/{vote.total} votaram
        </span>
      </header>
      <div className="ov-cols">
        {vote.teams.map((team) => (
          <div key={team.side}>
            <div className="ov-team-head">
              <span className={`ov-label ov-${team.side}`}>{SIDE_LABEL[team.side]}</span>
              <span className="ov-muted">
                Refresh <b className={`ov-${team.side}`}>{team.refresh}</b>/{vote.needed}
              </span>
            </div>
            <Pips count={team.refresh} needed={vote.needed} side={team.side} />
            {team.players.map((p) => (
              <div key={p.user_id} className="ov-row">
                <Player player={p} />
                {p.voted ? <span className="ov-ok">✓ votou</span> : <span className="ov-wait">aguardando</span>}
              </div>
            ))}
          </div>
        ))}
      </div>
      <footer className="ov-foot">
        <span>
          Manter times: <b>{vote.keep}</b>
        </span>
        <span className="ov-muted">
          {vote.needed === 1
            ? '1 voto Refresh de um lado sorteia times novos'
            : `${vote.needed} votos Refresh de um lado sorteiam times novos`}
        </span>
      </footer>
    </section>
  );
}

/** The new teams after a refresh, with who came in from the queue, or the teams kept. */
function Outcome({ ended, leaving }: { ended: Ended; leaving: boolean }) {
  const { vote, before } = ended;
  const refreshed = vote.state === 'refreshed';
  const players = (v: OverlayVote) => v.teams.flatMap((t) => t.players);
  const wasIn = new Set(players(before).map((p) => p.user_id));
  const isIn = new Set(players(vote).map((p) => p.user_id));
  const joined = players(vote).filter((p) => !wasIn.has(p.user_id));
  const left = players(before).filter((p) => !isIn.has(p.user_id));
  return (
    <section className={leaving ? 'ov-panel ov-leaving' : 'ov-panel'}>
      <header className="ov-top">
        <div>
          <div className="ov-label ov-jade">
            {matchName(vote)} · {refreshed ? 'Refresh aprovado' : 'Votação encerrada'}
          </div>
          <h1>{refreshed ? 'Times sorteados de novo' : 'Times mantidos'}</h1>
        </div>
      </header>
      <div className="ov-cols">
        {vote.teams.map((team) => (
          <div key={team.side}>
            <div className="ov-team-head">
              <span className={`ov-label ov-${team.side}`}>{SIDE_LABEL[team.side]}</span>
            </div>
            {team.players.map((p) => (
              <div key={p.user_id} className={wasIn.has(p.user_id) ? 'ov-row' : 'ov-row ov-new'}>
                <Player player={p} />
                {!wasIn.has(p.user_id) && <span className="ov-ok">da fila</span>}
              </div>
            ))}
          </div>
        ))}
      </div>
      <footer className="ov-foot">
        {!refreshed ? (
          <span>
            {vote.voted}/{vote.total} votaram · Manter times: <b>{vote.keep}</b>
          </span>
        ) : joined.length > 0 ? (
          <span>
            Entraram da fila: <b>{joined.map((p) => p.name).join(', ')}</b> · Voltaram para a fila:{' '}
            <b>{left.map((p) => p.name).join(', ')}</b>
          </span>
        ) : (
          <span>Os mesmos jogadores, em times novos.</span>
        )}
      </footer>
    </section>
  );
}

/**
 * The refresh votes of the server's camp, on a transparent background for OBS. Open votes show who
 * has voted; when one closes, its outcome shows for a few seconds and fades. `slot` keeps one
 * match's vote only.
 */
export function VoteOverlay({ token, slot }: { token: string; slot: number | null }) {
  const votes = useVotes(token);
  /** Votes last seen open, so a close is noticed. Votes closed before the page loaded never show. */
  const open = useRef(new Map<string, OverlayVote>());
  const [ended, setEnded] = useState<Ended[]>([]);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (votes === null) return;
    const at = Date.now();
    const closed: Ended[] = [];
    for (const vote of votes) {
      const before = open.current.get(vote.match_id);
      if (vote.state === 'open') {
        open.current.set(vote.match_id, vote);
      } else if (before) {
        open.current.delete(vote.match_id);
        closed.push({ vote, before, until: at + OUTCOME_MS });
      }
    }
    // Gone without a close seen, e.g. the draft started right away: nothing to show.
    const listed = new Set(votes.map((v) => v.match_id));
    for (const id of [...open.current.keys()]) {
      if (!listed.has(id)) open.current.delete(id);
    }
    if (closed.length > 0) setEnded((list) => [...list, ...closed]);
    setNow(at);
  }, [votes]);

  // While an outcome is up, tick to fade it and then drop it.
  useEffect(() => {
    if (ended.length === 0) return;
    const timer = window.setInterval(() => {
      const at = Date.now();
      setNow(at);
      setEnded((list) => (list.some((e) => e.until <= at) ? list.filter((e) => e.until > at) : list));
    }, 200);
    return () => window.clearInterval(timer);
  }, [ended.length]);

  const panels = [
    ...(votes ?? [])
      .filter((v) => v.state === 'open')
      .map((v) => ({ key: v.match_id, slot: v.slot, panel: <OpenVote vote={v} /> })),
    ...ended
      .filter((e) => e.until > now)
      .map((e) => ({
        key: e.vote.match_id,
        slot: e.vote.slot,
        panel: <Outcome ended={e} leaving={e.until - now <= FADE_MS} />,
      })),
  ]
    .filter((p) => slot === null || p.slot === slot)
    .sort((a, b) => a.slot - b.slot);

  return (
    <div className="ov">
      {panels.map((p) => (
        <div key={p.key}>{p.panel}</div>
      ))}
    </div>
  );
}
