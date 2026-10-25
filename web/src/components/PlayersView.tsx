import { useEffect, useState } from 'react';

import { useActions } from '../actions';
import { api, type Camp, type MemberResult, type Player } from '../api';
import { formatLabel, playerName, points, signed, whereIs } from '../camp';
import { CheckIcon } from './Icons';
import { PointsDialog } from './PointsDialog';

function Avatar({ url, name }: { url?: string; name: string }) {
  if (url) return <img className="avatar" src={url} alt="" width={28} height={28} />;
  return (
    <span className="avatar avatar-initial" aria-hidden="true">
      {name.charAt(0).toUpperCase()}
    </span>
  );
}

function SRankSwitch({ camp, player }: { camp: Camp; player: Player }) {
  const { run, busy } = useActions();
  return (
    <button
      type="button"
      role="switch"
      aria-checked={player.s_rank}
      aria-label={`S rank: ${playerName(camp, player.user_id)}`}
      className={player.s_rank ? 'switch switch-on' : 'switch'}
      disabled={busy}
      onClick={() => run({ type: 'set_s_rank', user_id: player.user_id, s_rank: !player.s_rank })}
    >
      <span className="switch-knob" />
    </button>
  );
}

function MemberSearch({ camp }: { camp: Camp }) {
  const { run, busy } = useActions();
  const [query, setQuery] = useState('');
  const [found, setFound] = useState<{ query: string; members: MemberResult[] } | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setFound(null);
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      api
        .searchMembers(camp.guild_id, q)
        .then((members) => {
          if (!cancelled) {
            setFound({ query: q, members });
            setError(null);
          }
        })
        .catch((e: Error) => !cancelled && setError(e.message));
    }, 300);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [query, camp.guild_id]);

  const onRoster = new Set(camp.players.map((p) => p.user_id));
  const inVoice = new Set(camp.queue_voice);
  const full = camp.players.length >= camp.target_size;

  return (
    <div className="field">
      <label htmlFor="member-search">Adicionar membro do servidor</label>
      <input
        id="member-search"
        type="search"
        className="input"
        placeholder="Comece a digitar o nome"
        autoComplete="off"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      {error && <p className="error small">{error}</p>}
      {found && found.query === query.trim() && (
        found.members.length === 0 ? (
          <p className="muted small">Ninguém com esse começo de nome no servidor.</p>
        ) : (
          <ul className="search-results" aria-label="Resultados da busca">
            {found.members.map((m) => (
              <li key={m.id} className="search-result">
                <Avatar url={m.avatar_url} name={m.name} />
                <span className="ellipsis">{m.name}</span>
                {inVoice.has(m.id) && <span className="muted small">em voz: Queue</span>}
                {onRoster.has(m.id) ? (
                  <span className="row-end muted small">Já está no elenco</span>
                ) : (
                  <button
                    type="button"
                    className="button button-primary button-small row-end"
                    disabled={busy || full}
                    title={full ? 'O elenco está completo.' : undefined}
                    onClick={() => run({ type: 'add_players', user_ids: [m.id] })}
                  >
                    Adicionar
                  </button>
                )}
              </li>
            ))}
          </ul>
        )
      )}
    </div>
  );
}

function Signup({ camp }: { camp: Camp }) {
  const { run, busy } = useActions();
  const roster = [...camp.players].sort((a, b) => playerName(camp, a.user_id).localeCompare(playerName(camp, b.user_id)));
  const count = camp.players.length;
  const full = count === camp.target_size;
  const onRoster = new Set(camp.players.map((p) => p.user_id));
  const voiceToAdd = camp.queue_voice.filter((id) => !onRoster.has(id));
  const sRank = camp.players.filter((p) => p.s_rank).length;
  const isTest = camp.target_size === 4;

  return (
    <div className="split">
      <section className="card split-main" aria-labelledby="roster-title">
        <div className="card-head">
          <h2 id="roster-title">
            Elenco{' '}
            <span className="mono muted">
              {count}/{camp.target_size}
            </span>
          </h2>
          <button
            type="button"
            className="button"
            disabled={busy || voiceToAdd.length === 0 || full}
            title={voiceToAdd.length === 0 ? 'Ninguém novo no canal de voz Queue.' : undefined}
            onClick={() => run({ type: 'add_players', user_ids: voiceToAdd })}
          >
            Adicionar quem está no canal Queue ({voiceToAdd.length})
          </button>
        </div>
        <div className="progress" role="img" aria-label={`${count} de ${camp.target_size} jogadores`}>
          <div className="progress-bar" style={{ width: `${(count / camp.target_size) * 100}%` }} />
        </div>
        <MemberSearch camp={camp} />
        {roster.length === 0 ? (
          <p className="empty">Ninguém no elenco ainda.</p>
        ) : (
          <ul className="rows roster" aria-label="Jogadores no elenco">
            {roster.map((p) => (
              <li key={p.user_id} className="row row-tall">
                <Avatar url={camp.members[p.user_id]?.avatar_url} name={playerName(camp, p.user_id)} />
                <span className="ellipsis">{playerName(camp, p.user_id)}</span>
                <span className="row-end muted small">S rank</span>
                <SRankSwitch camp={camp} player={p} />
                <button
                  type="button"
                  className="icon-button icon-button-quiet"
                  aria-label={`Remover ${playerName(camp, p.user_id)}`}
                  disabled={busy}
                  onClick={() => run({ type: 'remove_player', user_id: p.user_id })}
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <aside className="split-aside" aria-label="Antes de iniciar">
        <section className="card" aria-labelledby="s-title">
          <div className="card-head">
            <h2 id="s-title">S rank</h2>
            <span className="chip chip-s">{sRank} marcados</span>
          </div>
          <p className="small">Cada partida puxa os dois primeiros S rank da fila, um em cada time, e eles viram capitães.</p>
          {!isTest && (
            <p className="muted small">
              Com duas partidas ao mesmo tempo, o ideal são 4.{sRank > 4 ? ` Com ${sRank}, alguns esperam na fila.` : ''}
            </p>
          )}
        </section>
        <section className="card" aria-labelledby="check-title">
          <h2 id="check-title">Antes de iniciar a fila</h2>
          <ul className="checklist">
            <li className={full ? 'done' : ''}>
              {full ? <CheckIcon size={16} label="Pronto" /> : <span className="pending" aria-label="Pendente" role="img" />}
              Elenco completo: {count} de {camp.target_size}
              {full ? '' : `, faltam ${camp.target_size - count}`}
            </li>
            <li className="done">
              <CheckIcon size={16} label="Pronto" />
              Formato: {isTest ? `teste ${formatLabel(camp.match_size)}` : `${camp.target_size} jogadores, ${formatLabel(camp.match_size)}`}
            </li>
          </ul>
          <button
            type="button"
            className="button button-primary"
            disabled={busy || !full}
            title={full ? undefined : `Precisa de exatamente ${camp.target_size} jogadores.`}
            onClick={() =>
              run(
                { type: 'start_queue' },
                {
                  confirm: {
                    title: 'Iniciar a fila?',
                    message: 'O elenco é embaralhado e postado no Discord, e quem estiver em voz vai para o canal Queue.',
                    note: 'Depois disso não dá para adicionar nem remover jogadores.',
                    action: 'Iniciar fila',
                  },
                },
              )
            }
          >
            Iniciar fila
          </button>
          <p className="muted small">O bot posta a fila no canal do camp e move para Queue quem estiver em voz.</p>
        </section>
      </aside>
    </div>
  );
}

function Roster({ camp }: { camp: Camp }) {
  const players = [...camp.players].sort(
    (a, b) => b.points - a.points || playerName(camp, a.user_id).localeCompare(playerName(camp, b.user_id)),
  );
  const running = camp.status === 'running';
  const [adjusting, setAdjusting] = useState<Player | null>(null);
  const { busy } = useActions();
  return (
    <section className="card" aria-labelledby="roster-table-title">
      <div className="card-head">
        <h2 id="roster-table-title">
          Elenco <span className="mono muted">{players.length}</span>
        </h2>
      </div>
      <p className="muted small">
        {running
          ? 'O elenco não muda depois que a fila começa. Marcar S rank vale para as próximas partidas criadas. Ajustar pontos é para correções e penalidades; o bot anuncia cada ajuste.'
          : 'Este camp não está em andamento.'}
      </p>
      <div className="table-scroll">
        <table className="table">
          <thead>
            <tr>
              <th scope="col">Jogador</th>
              <th scope="col">S rank</th>
              <th scope="col" aria-sort="descending">
                Pontos
              </th>
              <th scope="col">Seguidas</th>
              <th scope="col">Onde está</th>
              {running && (
                <th scope="col" className="align-end">
                  <span className="visually-hidden">Ajustar pontos</span>
                </th>
              )}
            </tr>
          </thead>
          <tbody>
            {players.map((p) => (
              <tr key={p.user_id}>
                <td>
                  <span className="player-cell">
                    <Avatar url={camp.members[p.user_id]?.avatar_url} name={playerName(camp, p.user_id)} />
                    {playerName(camp, p.user_id)}
                  </span>
                </td>
                <td>{running ? <SRankSwitch camp={camp} player={p} /> : p.s_rank ? 'Sim' : '—'}</td>
                <td className="mono">
                  {points(p.points)}
                  {p.points_adjusted !== 0 && (
                    <span className="muted small" title="Ajustes manuais">
                      {' '}
                      ({signed(p.points_adjusted)})
                    </span>
                  )}
                </td>
                <td>{p.consecutive_matches}</td>
                <td className="muted">{whereIs(camp, p)}</td>
                {running && (
                  <td className="align-end">
                    <button
                      type="button"
                      className="button button-small button-ghost"
                      disabled={busy}
                      aria-label={`Ajustar pontos de ${playerName(camp, p.user_id)}`}
                      onClick={() => setAdjusting(p)}
                    >
                      Ajustar pontos
                    </button>
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <PointsDialog camp={camp} player={adjusting} onClose={() => setAdjusting(null)} />
    </section>
  );
}

export function PlayersView({ camp }: { camp: Camp }) {
  return camp.status === 'signup' ? <Signup camp={camp} /> : <Roster camp={camp} />;
}
