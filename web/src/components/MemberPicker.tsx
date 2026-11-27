import { useEffect, useState } from 'react';

import { api, type MemberResult } from '../api';

export function Avatar({ url, name }: { url?: string; name: string }) {
  if (url) return <img className="avatar" src={url} alt="" width={28} height={28} />;
  return (
    <span className="avatar avatar-initial" aria-hidden="true">
      {name.charAt(0).toUpperCase()}
    </span>
  );
}

/** Lowercase without accents, so "fumaca" finds "fumaça". */
export function folded(text: string): string {
  return text.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase();
}

/** The most members the list shows at once; typing narrows it. */
const SHOWN = 50;

type ServerMember = MemberResult & { folded: string };

/** The server's members, sorted by name, loaded once per server. */
export function useMembers(guildId: string): { members: ServerMember[] | null; error: string | null } {
  const [members, setMembers] = useState<ServerMember[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let cancelled = false;
    setMembers(null);
    setError(null);
    api
      .members(guildId)
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
  }, [guildId]);
  return { members, error };
}

/** Finds server members by name to add them to a list: a camp's roster or a selection. */
export function MemberPicker({
  guildId,
  taken,
  full,
  busy = false,
  inVoice,
  label,
  onAdd,
}: {
  guildId: string;
  /** Already on the list, so not offered. */
  taken: Set<string>;
  /** No room left: the buttons are off. */
  full: boolean;
  busy?: boolean;
  /** In the Queue voice channel right now. */
  inVoice?: Set<string>;
  label: string;
  onAdd: (member: MemberResult) => void;
}) {
  const [query, setQuery] = useState('');
  const { members, error } = useMembers(guildId);
  const q = folded(query.trim());
  const available = members?.filter((m) => !taken.has(m.id)) ?? [];
  const matching = q ? available.filter((m) => m.folded.includes(q)) : available;
  const id = `member-search-${label.length}`;

  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <input
        id={id}
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
        <p className="muted small">Todos os membros do servidor já estão na lista.</p>
      ) : matching.length === 0 ? (
        <p className="muted small">Ninguém fora da lista com esse nome.</p>
      ) : (
        <>
          <ul className="search-results" aria-label="Membros do servidor fora da lista">
            {matching.slice(0, SHOWN).map((m) => (
              <li key={m.id} className="search-result">
                <Avatar url={m.avatar_url} name={m.name} />
                <span className="ellipsis">{m.name}</span>
                {inVoice?.has(m.id) && <span className="muted small">em voz: Queue</span>}
                <button
                  type="button"
                  className="button button-primary button-small row-end"
                  disabled={busy || full}
                  title={full ? 'A lista está completa.' : undefined}
                  onClick={() => onAdd(m)}
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
