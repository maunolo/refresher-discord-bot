import { useEffect, useState, type FormEvent } from 'react';

import { useActions } from '../actions';
import { api, type Camp, type Player, type Preset, type Rank } from '../api';
import { formatLabel, matchesWord, playerName, points, RANK_LABEL, rankOf, ROLE_LABEL, roleCounts, signed, whereIs } from '../camp';
import { routeHref } from '../route';
import { RemovedChip } from './common';
import { CheckIcon } from './Icons';
import { Avatar, MemberPicker } from './MemberPicker';
import { NO_FILTER, passes, PlayerFilter, RankPicker, RolePicker, type Filter } from './PlayerFields';
import { PointsDialog } from './PointsDialog';

/** None, S rank or SS rank. Each takes the other away. */
function RankSelect({ camp, player }: { camp: Camp; player: Player }) {
  const { run, busy } = useActions();
  const set = (rank: Rank | null) => {
    if (rank === 'ss') run({ type: 'set_ss_rank', user_id: player.user_id, ss_rank: true });
    else if (rank === 's') run({ type: 'set_s_rank', user_id: player.user_id, s_rank: true });
    else if (player.ss_rank) run({ type: 'set_ss_rank', user_id: player.user_id, ss_rank: false });
    else run({ type: 'set_s_rank', user_id: player.user_id, s_rank: false });
  };
  return (
    <RankPicker
      value={rankOf(player)}
      label={`Categoria: ${playerName(camp, player.user_id)}`}
      disabled={busy}
      onChange={set}
    />
  );
}

/** The player's main role, kept per server for later camps too. */
function RoleSelect({ camp, player }: { camp: Camp; player: Player }) {
  const { run, busy } = useActions();
  return (
    <RolePicker
      value={player.role}
      label={`Função: ${playerName(camp, player.user_id)}`}
      disabled={busy}
      onChange={(role) => run({ type: 'set_role', user_id: player.user_id, role })}
    />
  );
}

const ROLES_HELP =
  'Ao iniciar a fila, as funções são espalhadas entre os times como as categorias. Ao montar os times de uma partida, o bot tenta dar a cada time pelo menos um Tank/Bruiser, um Carry e um Utility/Support, sem mudar a divisão das categorias. A função fica guardada no servidor e vale nos próximos camps.';

function MemberSearch({ camp }: { camp: Camp }) {
  const { run, busy } = useActions();
  return (
    <MemberPicker
      guildId={camp.guild_id}
      label="Adicionar membro do servidor"
      taken={new Set(camp.players.map((p) => p.user_id))}
      inVoice={new Set(camp.queue_voice)}
      full={camp.players.length >= camp.target_size}
      busy={busy}
      onAdd={(m) => run({ type: 'add_players', user_ids: [m.id] })}
    />
  );
}

/** Fills the roster from a saved selection, or saves the roster as one. */
function Selections({ camp }: { camp: Camp }) {
  const { run, busy } = useActions();
  const [presets, setPresets] = useState<Preset[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [chosen, setChosen] = useState('');
  const [name, setName] = useState('');
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .presets(camp.guild_id)
      .then((loaded) => !cancelled && setPresets(loaded.presets))
      .catch((e: Error) => !cancelled && setError(e.message));
    return () => {
      cancelled = true;
    };
  }, [camp.guild_id]);

  const preset = presets?.find((p) => String(p.id) === chosen);
  const apply = () => {
    if (!preset) return;
    run(
      { type: 'apply_preset', preset_id: preset.id },
      { success: `Seleção ${preset.name} no elenco, com categorias e funções.` },
    );
  };
  const save = (e: FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    setSaved(null);
    const players = camp.players.map((p) => ({ user_id: p.user_id, rank: rankOf(p), role: p.role }));
    api
      .createPreset(camp.guild_id, { name, players })
      .then((result) => {
        setPresets(result.presets);
        setSaved(`Elenco salvo como ${name.trim()}.`);
        setName('');
      })
      .catch((err: Error) => setError(err.message))
      .finally(() => setSaving(false));
  };

  return (
    <section className="card" aria-labelledby="selections-title">
      <h2 id="selections-title">Seleções</h2>
      {presets === null && !error ? (
        <p className="muted small">Carregando…</p>
      ) : presets && presets.length > 0 ? (
        <>
          <div className="field">
            <label htmlFor="preset-choice">Usar uma seleção salva</label>
            <select id="preset-choice" className="input" value={chosen} onChange={(e) => setChosen(e.target.value)}>
              <option value="">Escolha uma seleção</option>
              {presets.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name} ({p.players.length})
                </option>
              ))}
            </select>
          </div>
          <div className="form-actions">
            <button type="button" className="button button-small" disabled={busy || !preset} onClick={apply}>
              Usar seleção
            </button>
          </div>
          <p className="muted small">
            Entram os jogadores da seleção até completar o elenco, com as categorias e as funções dela. Quem já está no
            elenco fica.
          </p>
        </>
      ) : (
        <p className="muted small">
          Nenhuma seleção salva. Monte uma em <a href={routeHref('presets')}>Elenco</a>, antes do camp, com jogadores,
          categorias e funções.
        </p>
      )}
      {camp.players.length > 0 && (
        <form className="field" onSubmit={save}>
          <label htmlFor="preset-name">Salvar este elenco como seleção</label>
          <div className="inline-actions">
            <input
              id="preset-name"
              className="input"
              maxLength={60}
              placeholder="Nome da seleção"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
            <button type="submit" className="button button-small" disabled={saving || !name.trim()}>
              {saving ? 'Salvando…' : 'Salvar'}
            </button>
          </div>
        </form>
      )}
      {saved && (
        <p className="small" role="status">
          {saved}
        </p>
      )}
      {error && (
        <p className="error small" role="alert">
          {error}
        </p>
      )}
    </section>
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
  const ssRank = camp.players.filter((p) => p.ss_rank).length;
  const [filter, setFilter] = useState<Filter>(NO_FILTER);
  const shown = roster.filter((p) => passes(filter, playerName(camp, p.user_id), p.role));
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
        {roster.length > 0 && <PlayerFilter filter={filter} onChange={setFilter} shown={shown.length} total={roster.length} />}
        {roster.length === 0 ? (
          <p className="empty">Ninguém no elenco ainda.</p>
        ) : shown.length === 0 ? (
          <p className="empty">Ninguém do elenco com esse filtro.</p>
        ) : (
          <ul className="rows roster" aria-label="Jogadores no elenco">
            {shown.map((p) => (
              <li key={p.user_id} className="row row-tall">
                <Avatar url={camp.members[p.user_id]?.avatar_url} name={playerName(camp, p.user_id)} />
                <span className="ellipsis">{playerName(camp, p.user_id)}</span>
                <span className="row-end">
                  <RoleSelect camp={camp} player={p} />
                </span>
                <RankSelect camp={camp} player={p} />
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
        <Selections camp={camp} />
        <section className="card" aria-labelledby="s-title">
          <div className="card-head">
            <h2 id="s-title">Categorias</h2>
            <span className="inline-actions">
              <span className="chip chip-ss">{ssRank} SS</span>
              <span className="chip chip-s">{sRank} S</span>
            </span>
          </div>
          <p className="small">
            {`Ao iniciar a fila, o elenco é sorteado em ${teams} times de ${teamSize}, com os ${RANK_LABEL.ss} e depois os ${RANK_LABEL.s} divididos por igual. Depois disso, a categoria não dá prioridade na fila.`}
          </p>
          <p className="muted small">
            Numa partida, cada time tem no máximo dois SS rank, e todo SS rank do time é capitão e recebe o link do draft.
            Os S rank também ficam divididos entre os dois times. Um time sem SS rank tem um capitão: o S rank com mais
            pontos, ou quem tem mais pontos.
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
                    message: `O elenco é embaralhado em ${teams} times de ${teamSize}, com as categorias e as funções divididas por igual, e postado no Discord. Quem estiver em voz vai para o canal Queue.`,
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

/** Takes a player out of the running camp, or brings a removed one back. */
function RemoveButton({ camp, player }: { camp: Camp; player: Player }) {
  const { run, busy } = useActions();
  const name = playerName(camp, player.user_id);
  const inMatch = player.location !== 'queued' && player.location !== 'removed';
  if (player.removed) {
    return (
      <button
        type="button"
        className="button button-small button-ghost"
        disabled={busy}
        aria-label={`Devolver ${name} ao camp`}
        onClick={() =>
          run(
            { type: 'restore_player', user_id: player.user_id },
            {
              confirm: {
                title: `Devolver ${name} ao camp?`,
                message: inMatch
                  ? 'Volta para a fila quando a partida atual terminar, como os outros jogadores.'
                  : 'Entra no fim da fila e volta a ser sorteado nas partidas novas.',
                action: 'Devolver ao camp',
              },
            },
          )
        }
      >
        Devolver
      </button>
    );
  }
  return (
    <button
      type="button"
      className="button button-small button-ghost"
      disabled={busy}
      aria-label={`Remover ${name} do camp`}
      onClick={() =>
        run(
          { type: 'remove_player', user_id: player.user_id },
          {
            confirm: {
              title: `Remover ${name} do camp?`,
              message: inMatch
                ? 'Termina a partida atual e depois não volta para a fila. Não entra em partidas novas nem na final. Para tirar o jogador da partida agora, cancele a partida depois: os outros voltam para a fila nos lugares de antes.'
                : 'Sai da fila agora e não entra em partidas novas nem na final.',
              note: 'Os pontos e as partidas jogadas continuam no ranking. O bot avisa no canal do camp, e dá para devolver o jogador depois.',
              action: 'Remover do camp',
              tone: 'danger',
            },
          },
        )
      }
    >
      Remover
    </button>
  );
}

function Roster({ camp }: { camp: Camp }) {
  const players = [...camp.players].sort(
    (a, b) =>
      Number(a.removed) - Number(b.removed) ||
      b.points - a.points ||
      playerName(camp, a.user_id).localeCompare(playerName(camp, b.user_id)),
  );
  const running = camp.status === 'running';
  const [adjusting, setAdjusting] = useState<Player | null>(null);
  const { busy } = useActions();
  const removed = players.filter((p) => p.removed).length;
  const [filter, setFilter] = useState<Filter>(NO_FILTER);
  const shown = players.filter((p) => passes(filter, playerName(camp, p.user_id), p.role));
  return (
    <section className="card" aria-labelledby="roster-table-title">
      <div className="card-head">
        <h2 id="roster-table-title">
          Elenco <span className="mono muted">{players.length - removed}</span>
          {removed > 0 && (
            <span className="muted small">
              {' '}
              · {removed} {removed === 1 ? 'removido' : 'removidos'}
            </span>
          )}
        </h2>
      </div>
      <p className="muted small">
        {running
          ? 'Depois que a fila começa, ninguém entra no elenco, mas dá para remover um jogador: ele sai da fila e não entra em partidas novas nem na final, e os pontos dele continuam no ranking. Categoria e função valem para os times montados daqui em diante, numa partida nova ou num refresh. Ajustar pontos é para correções e penalidades; o bot anuncia cada ajuste e cada remoção.'
          : 'Este camp não está em andamento.'}
      </p>
      <p className="muted small">
        {roleCounts(players)}. {ROLES_HELP}
      </p>
      <PlayerFilter filter={filter} onChange={setFilter} shown={shown.length} total={players.length} />
      <div className="table-scroll">
        <table className="table">
          <thead>
            <tr>
              <th scope="col">Jogador</th>
              <th scope="col">Categoria</th>
              <th scope="col">Função</th>
              <th scope="col" aria-sort="descending">
                Pontos
              </th>
              <th scope="col">Partidas</th>
              <th scope="col">Onde está</th>
              {running && (
                <th scope="col" className="align-end">
                  <span className="visually-hidden">Ações</span>
                </th>
              )}
            </tr>
          </thead>
          <tbody>
            {shown.length === 0 && (
              <tr>
                <td colSpan={running ? 7 : 6} className="muted">
                  Ninguém do elenco com esse filtro.
                </td>
              </tr>
            )}
            {shown.map((p) => (
              <tr key={p.user_id} className={p.removed ? 'row-removed' : undefined}>
                <td>
                  <span className="player-cell">
                    <Avatar url={camp.members[p.user_id]?.avatar_url} name={playerName(camp, p.user_id)} />
                    {playerName(camp, p.user_id)}
                    {p.removed && <RemovedChip />}
                  </span>
                </td>
                <td>{running ? <RankSelect camp={camp} player={p} /> : rankOf(p) ? RANK_LABEL[rankOf(p)!] : '—'}</td>
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
                <td>{matchesWord(p.matches_played)}</td>
                <td className="muted">{whereIs(camp, p)}</td>
                {running && (
                  <td className="align-end">
                    <div className="inline-actions">
                      <button
                        type="button"
                        className="button button-small button-ghost"
                        disabled={busy}
                        aria-label={`Ajustar pontos de ${playerName(camp, p.user_id)}`}
                        onClick={() => setAdjusting(p)}
                      >
                        Ajustar pontos
                      </button>
                      <RemoveButton camp={camp} player={p} />
                    </div>
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
