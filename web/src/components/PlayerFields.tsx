import type { Rank, Role } from '../api';
import { RANK_LABEL, RANKS, ROLE_LABEL, ROLES } from '../camp';
import { folded } from './MemberPicker';

/** No category, S rank or SS rank. */
export function RankPicker({
  value,
  label,
  disabled,
  onChange,
}: {
  value: Rank | null;
  label: string;
  disabled?: boolean;
  onChange: (rank: Rank | null) => void;
}) {
  return (
    <select
      className="input rank-select"
      aria-label={label}
      value={value ?? ''}
      disabled={disabled}
      onChange={(e) => onChange((e.target.value || null) as Rank | null)}
    >
      <option value="">Sem categoria</option>
      {RANKS.map((rank) => (
        <option key={rank} value={rank}>
          {RANK_LABEL[rank]}
        </option>
      ))}
    </select>
  );
}

/** A main role, or none. */
export function RolePicker({
  value,
  label,
  disabled,
  onChange,
}: {
  value: Role | null;
  label: string;
  disabled?: boolean;
  onChange: (role: Role | null) => void;
}) {
  return (
    <select
      className="input role-select"
      aria-label={label}
      value={value ?? ''}
      disabled={disabled}
      onChange={(e) => onChange((e.target.value || null) as Role | null)}
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

/** Narrows a player list: part of the name, and a role (`none` for players without one). */
export interface Filter {
  name: string;
  role: Role | 'none' | '';
}

export const NO_FILTER: Filter = { name: '', role: '' };

export function passes(filter: Filter, name: string, role: Role | null): boolean {
  const q = folded(filter.name.trim());
  if (q && !folded(name).includes(q)) return false;
  if (filter.role === 'none') return role === null;
  return filter.role === '' || role === filter.role;
}

/** The name and role filters over a player list, with how many it shows. */
export function PlayerFilter({
  filter,
  onChange,
  shown,
  total,
}: {
  filter: Filter;
  onChange: (filter: Filter) => void;
  shown: number;
  total: number;
}) {
  const filtering = filter.name.trim() !== '' || filter.role !== '';
  return (
    <div className="filters" role="search">
      <input
        type="search"
        className="input"
        placeholder="Buscar jogador"
        aria-label="Filtrar jogadores por nome"
        value={filter.name}
        onChange={(e) => onChange({ ...filter, name: e.target.value })}
      />
      <select
        className="input"
        aria-label="Filtrar jogadores por função"
        value={filter.role}
        onChange={(e) => onChange({ ...filter, role: e.target.value as Filter['role'] })}
      >
        <option value="">Todas as funções</option>
        {ROLES.map((role) => (
          <option key={role} value={role}>
            {ROLE_LABEL[role]}
          </option>
        ))}
        <option value="none">Sem função</option>
      </select>
      {filtering && (
        <>
          <span className="muted small" role="status">
            {shown} de {total}
          </span>
          <button type="button" className="button button-small button-ghost" onClick={() => onChange(NO_FILTER)}>
            Limpar
          </button>
        </>
      )}
    </div>
  );
}
