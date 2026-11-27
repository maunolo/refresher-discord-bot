import { useEffect, useState, type FormEvent } from 'react';

import { api, type MemberResult, type Preset, type PresetPlayer, type Presets } from '../api';
import { RANK_LABEL, ROLE_LABEL, ROLES } from '../camp';
import { ConfirmDialog } from './ConfirmDialog';
import { Avatar, MemberPicker, useMembers } from './MemberPicker';
import { NO_FILTER, passes, PlayerFilter, RankPicker, RolePicker, type Filter } from './PlayerFields';

/** The most players a selection holds: the biggest camp. */
const MAX_PLAYERS = 30;

interface Draft {
  /** `null` while creating a new one. */
  id: number | null;
  name: string;
  players: PresetPlayer[];
}

function editedOn(seconds: number): string {
  return new Date(seconds * 1000).toLocaleDateString('pt-BR', { day: '2-digit', month: 'short' });
}

/** "Tank/Bruiser 8 · Carry 10 · 2 SS · 5 S", for a selection's row. */
function counts(players: PresetPlayer[]): string {
  const parts = ROLES.map((role) => `${ROLE_LABEL[role]} ${players.filter((p) => p.role === role).length}`);
  const ss = players.filter((p) => p.rank === 'ss').length;
  const s = players.filter((p) => p.rank === 's').length;
  return [...parts, `${ss} SS`, `${s} S`].join(' · ');
}

function Editor({
  guildId,
  draft,
  roles,
  onDraft,
  onSaved,
  onClose,
}: {
  guildId: string;
  draft: Draft;
  roles: Presets['roles'];
  onDraft: (draft: Draft) => void;
  onSaved: (presets: Presets) => void;
  onClose: () => void;
}) {
  const { members } = useMembers(guildId);
  const [filter, setFilter] = useState<Filter>(NO_FILTER);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const member = (userId: string) => members?.find((m) => m.id === userId);
  const name = (userId: string) => member(userId)?.name ?? `Jogador …${userId.slice(-4)}`;
  const shown = draft.players.filter((p) => passes(filter, name(p.user_id), p.role));
  const change = (userId: string, patch: Partial<PresetPlayer>) =>
    onDraft({ ...draft, players: draft.players.map((p) => (p.user_id === userId ? { ...p, ...patch } : p)) });
  const add = (m: MemberResult) =>
    onDraft({ ...draft, players: [...draft.players, { user_id: m.id, rank: null, role: roles[m.id] ?? null }] });
  const remove = (userId: string) => onDraft({ ...draft, players: draft.players.filter((p) => p.user_id !== userId) });

  const save = (e: FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    const form = { name: draft.name, players: draft.players };
    (draft.id === null ? api.createPreset(guildId, form) : api.savePreset(guildId, draft.id, form))
      .then(onSaved)
      .catch((err: Error) => setError(err.message))
      .finally(() => setSaving(false));
  };

  return (
    <form className="card stack" aria-labelledby="preset-editor-title" onSubmit={save}>
      <div className="card-head">
        <h2 id="preset-editor-title">{draft.id === null ? 'Nova seleção' : 'Editar seleção'}</h2>
        <span className="mono muted">
          {draft.players.length}/{MAX_PLAYERS}
        </span>
      </div>
      <div className="field">
        <label htmlFor="preset-editor-name">Nome</label>
        <input
          id="preset-editor-name"
          className="input"
          maxLength={60}
          required
          placeholder="Ex.: Elenco de quinta"
          value={draft.name}
          onChange={(e) => onDraft({ ...draft, name: e.target.value })}
        />
      </div>
      <MemberPicker
        guildId={guildId}
        label="Adicionar membro do servidor"
        taken={new Set(draft.players.map((p) => p.user_id))}
        full={draft.players.length >= MAX_PLAYERS}
        onAdd={add}
      />
      {draft.players.length > 0 && (
        <PlayerFilter filter={filter} onChange={setFilter} shown={shown.length} total={draft.players.length} />
      )}
      {draft.players.length === 0 ? (
        <p className="empty">Ninguém na seleção ainda.</p>
      ) : shown.length === 0 ? (
        <p className="empty">Ninguém da seleção com esse filtro.</p>
      ) : (
        <ul className="rows roster" aria-label="Jogadores da seleção">
          {shown.map((p) => (
            <li key={p.user_id} className="row row-tall">
              <Avatar url={member(p.user_id)?.avatar_url} name={name(p.user_id)} />
              <span className="ellipsis">{name(p.user_id)}</span>
              <span className="row-end">
                <RolePicker value={p.role} label={`Função: ${name(p.user_id)}`} onChange={(role) => change(p.user_id, { role })} />
              </span>
              <RankPicker value={p.rank} label={`Categoria: ${name(p.user_id)}`} onChange={(rank) => change(p.user_id, { rank })} />
              <button
                type="button"
                className="icon-button icon-button-quiet"
                aria-label={`Tirar ${name(p.user_id)} da seleção`}
                onClick={() => remove(p.user_id)}
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
      <p className="muted small">
        Ao usar a seleção num camp, a função de cada jogador vira a função dele no servidor, e a categoria vale para o camp.
        Um jogador novo na seleção começa com a função que já tem no servidor.
      </p>
      {error && (
        <p className="error small" role="alert">
          {error}
        </p>
      )}
      <div className="form-actions">
        <button type="submit" className="button button-primary" disabled={saving || !draft.name.trim()}>
          {saving ? 'Salvando…' : 'Salvar seleção'}
        </button>
        <button type="button" className="button button-ghost" onClick={onClose}>
          Fechar
        </button>
      </div>
    </form>
  );
}

/** Elenco: selections of players saved before any camp, who plays with their category and role. */
export function PresetsView({ guildId }: { guildId: string }) {
  const [data, setData] = useState<Presets | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [deleting, setDeleting] = useState<Preset | null>(null);

  useEffect(() => {
    let cancelled = false;
    setData(null);
    setDraft(null);
    api
      .presets(guildId)
      .then((loaded) => !cancelled && setData(loaded))
      .catch((e: Error) => !cancelled && setError(e.message));
    return () => {
      cancelled = true;
    };
  }, [guildId]);

  if (error && !data) return <p className="error">{error}</p>;
  if (!data) return <p className="muted">Carregando…</p>;

  const remove = (preset: Preset) => {
    setError(null);
    api
      .deletePreset(guildId, preset.id)
      .then((result) => {
        setData(result);
        if (draft?.id === preset.id) setDraft(null);
      })
      .catch((e: Error) => setError(e.message));
  };

  return (
    <section className="stack" aria-labelledby="presets-title">
      <div className="card-head">
        <div className="title-row">
          <h1 id="presets-title" className="page-title">
            Elenco
          </h1>
          <span className="muted small">Jogadores, categorias e funções prontos para o próximo camp</span>
        </div>
        <button
          type="button"
          className="button button-primary"
          disabled={draft !== null}
          onClick={() => setDraft({ id: null, name: '', players: [] })}
        >
          Nova seleção
        </button>
      </div>
      {draft && (
        <Editor
          guildId={guildId}
          draft={draft}
          roles={data.roles}
          onDraft={setDraft}
          onSaved={(saved) => {
            setData(saved);
            setDraft(null);
          }}
          onClose={() => setDraft(null)}
        />
      )}
      {error && (
        <p className="error small" role="alert">
          {error}
        </p>
      )}
      {data.presets.length === 0 ? (
        <section className="card empty-card">
          <h2>Nenhuma seleção ainda</h2>
          <p className="muted small">
            Monte uma seleção com os jogadores do próximo camp, já com {RANK_LABEL.ss}, {RANK_LABEL.s} e funções. Ao criar
            o camp, escolha a seleção e o elenco já entra pronto.
          </p>
        </section>
      ) : (
        <div className="card table-scroll">
          <table className="table">
            <thead>
              <tr>
                <th scope="col">Seleção</th>
                <th scope="col">Jogadores</th>
                <th scope="col">Funções e categorias</th>
                <th scope="col">Editada</th>
                <th scope="col" className="align-end">
                  <span className="visually-hidden">Ações</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {data.presets.map((preset) => (
                <tr key={preset.id}>
                  <td className="strong">{preset.name}</td>
                  <td className="mono">{preset.players.length}</td>
                  <td className="muted small">{counts(preset.players)}</td>
                  <td className="muted">{editedOn(preset.updated_at)}</td>
                  <td className="align-end">
                    <div className="inline-actions">
                      <button
                        type="button"
                        className="button button-small button-ghost"
                        onClick={() => setDraft({ id: preset.id, name: preset.name, players: preset.players })}
                      >
                        Editar
                      </button>
                      <button type="button" className="button button-small button-ghost" onClick={() => setDeleting(preset)}>
                        Apagar
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="muted small">
        Use uma seleção ao criar um camp, em Camps, ou na inscrição, em Jogadores. Ela só enche o elenco: depois, dá para
        mudar o camp à vontade sem mexer na seleção.
      </p>
      <ConfirmDialog
        ask={
          deleting && {
            title: `Apagar a seleção ${deleting.name}?`,
            message: 'Os camps que já usaram a seleção não mudam.',
            action: 'Apagar seleção',
            tone: 'danger',
          }
        }
        onAnswer={(ok) => {
          if (ok && deleting) remove(deleting);
          setDeleting(null);
        }}
      />
    </section>
  );
}
