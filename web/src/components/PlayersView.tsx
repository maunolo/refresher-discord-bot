import { useEffect, useState } from 'react';

import { useActions } from '../actions';
import { api, type Camp, type MemberResult, type Player, type Role } from '../api';
import { formatLabel, playerName, points, ROLE_LABEL, roleCounts, ROLES, signed, whereIs } from '../camp';
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

/** The player's main role, kept per server for later camps too. */
function RoleSelect({ camp, player }: { camp: Camp; player: Player }) {
  const { run, busy } = useActions();
  return (
    <select
      className="input role-select"
      aria-label={`Função: ${playerName(camp, player.user_id)}`}
      value={player.role ?? ''}
      disabled={busy}
      onChange={(e) => run({ type: 'set_role', user_id: player.user_id, role: (e.target.value || null) as Role | null })}
    >
      <option value="">Sem função</option>
      {ROLES.map((role) => (
        <option key={role} value={role}>
          {ROLE_LABEL[role]}
        </option>
      ))}
    </select>
  );
}

const ROLES_HELP =
  'Ao iniciar a fila, as funções são espalhadas entre os times como os S rank. Ao montar os times de uma partida, o bot tenta dar a cada time pelo menos um Tank/Bruiser, um Carry e um Utility/Support, sem mudar a divisão dos S rank. A função fica guardada no servidor e vale nos próximos camps.';

/** Lowercase without accents, so "fumaca" finds "fumaça". */
function folded(text: string): string {
  return text.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase();
}

/** The most members the list shows at once; typing narrows it. */
const SHOWN = 50;

type ServerMember = MemberResult & { folded: string };

function MemberSearch({ camp }: { camp: Camp }) {
  const { run, busy } = useActions();
  const [query, setQuery] = useState('');
  const [members, setMembers] = useState<ServerMember[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Once per server: the list is filtered here, not on Discord.
  useEffect(() => {
    let cancelled = false;
    setMembers(null);
    setError(null);
    api
      .members(camp.guild_id)
      .then((list) => {
        if (cancelled) return;
        const sorted = list
          .map((m) => ({ ...m, folded: folded(m.name) }))
          .sort((a, b) => a.name.localeCompare(b.name, 'pt-BR', { sensitivity: 'base' }));
        setMembers(sorted);
      })
      .catch((e: Error) => !cancelled && setError(e.message));
    return () => {
      cancelled = true;
    };
  }, [camp.guild_id]);

  const onRoster = new Set(camp.players.map((p) => p.user_id));
  const inVoice = new Set(camp.queue_voice);
  const full = camp.players.length >= camp.target_size;
  const q = folded(query.trim());
  const available = members?.filter((m) => !onRoster.has(m.id)) ?? [];
  const matching = q ? available.filter((m) => m.folded.includes(q)) : available;

  return (
    <div className="field">
      <label htmlFor="member-search">Adicionar membro do servidor</label>
      <input
        id="member-search"
        type="search"
        className="input"
        placeholder="Filtrar por nome"
        autoComplete="off"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      {error ? (
        <p className="error small">{error}</p>
      ) : members === null ? (
        <p className="muted small">Carregando membros do servidor…</p>
      ) : available.length === 0 ? (
        <p className="muted small">Todos os membros do servidor já estão no elenco.</p>
      ) : matching.length === 0 ? (
        <p className="muted small">Ninguém fora do elenco com esse nome.</p>
      ) : (
        <>
          <ul className="search-results" aria-label="Membros do servidor fora do elenco">
            {matching.slice(0, SHOWN).map((m) => (
              <li key={m.id} className="search-result">
                <Avatar url={m.avatar_url} name={m.name} />
                <span className="ellipsis">{m.name}</span>
                {inVoice.has(m.id) && <span className="muted small">em voz: Queue</span>}
                <button
                  type="button"
                  className="button button-primary button-small row-end"
                  disabled={busy || full}
                  title={full ? 'O elenco está completo.' : undefined}
                  onClick={() => run({ type: 'add_players', user_ids: [m.id] })}
                >
                  Adicionar
                </button>
              </li>
            ))}
          </ul>
          {matching.length > SHOWN && (
            <p className="muted small">
              Mostrando {SHOWN} de {matching.length}. Digite para filtrar.
            </p>
          )}
        </>
      )}
    </div>
  );
}

/** How the starting queue spreads S rank over the camp's teams, e.g. "3 times com 2 e 2 com 1". */
function sRankSpread(sRank: number, teams: number): string {
  const each = Math.floor(sRank / teams);
  const more = sRank % teams;
  if (more === 0) return `${each} por time`;
  return `${more} ${more === 1 ? 'time' : 'times'} com ${each + 1} e ${teams - more} com ${each}`;
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
  const teamSize = camp.match_size / 2;
  const teams = camp.target_size / teamSize;

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
                <span className="row-end">
                  <RoleSelect camp={camp} player={p} />
                </span>
                <span className="muted small">S rank</span>
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
          <p className="small">
            {`Ao iniciar a fila, o elenco é sorteado em ${teams} times de ${teamSize}, com os S rank divididos por igual${
              sRank > 0 ? `: ${sRankSpread(sRank, teams)}` : ''
            }. Depois disso, S rank não têm prioridade na fila.`}
          </p>
          <p className="muted small">
            Numa partida, os S rank ficam divididos entre os dois times, e todo S rank do time é capitão e recebe o link do
            draft. Um time sem S rank tem como capitão quem tem mais pontos.
          </p>
        </section>
        <section className="card" aria-labelledby="roles-title">
          <h2 id="roles-title">Funções</h2>
          <p className="small">{roleCounts(camp.players)}</p>
          <p className="muted small">{ROLES_HELP}</p>
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
                    message: `O elenco é embaralhado em ${teams} times de ${teamSize}, com os S rank e as funções divididos por igual, e postado no Discord. Quem estiver em voz vai para o canal Queue.`,
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
          ? 'O elenco não muda depois que a fila começa. S rank e função valem para os times montados daqui em diante, numa partida nova ou num refresh. Ajustar pontos é para correções e penalidades; o bot anuncia cada ajuste.'
          : 'Este camp não está em andamento.'}
      </p>
      <p className="muted small">
        {roleCounts(players)}. {ROLES_HELP}
      </p>
      <div className="table-scroll">
        <table className="table">
          <thead>
            <tr>
              <th scope="col">Jogador</th>
              <th scope="col">S rank</th>
              <th scope="col">Função</th>
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
                <td>{running ? <RoleSelect camp={camp} player={p} /> : p.role ? ROLE_LABEL[p.role] : '—'}</td>
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
