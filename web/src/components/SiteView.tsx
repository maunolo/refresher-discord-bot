import { useEffect, useState, type FormEvent } from 'react';

import {
  api,
  type Camp,
  type SiteContent,
  type SiteInfoKey,
  type SiteItem,
  type SiteMatch,
  type SiteState,
  type SiteTexts,
} from '../api';
import { ExternalIcon } from './Icons';

const CARDS: { key: SiteInfoKey; label: string }[] = [
  { key: 'data', label: 'Data' },
  { key: 'formato', label: 'Formato' },
  { key: 'premiacao', label: 'Premiação' },
  { key: 'times', label: 'Times' },
];

const LANGUAGES: { key: keyof SiteTexts; label: string }[] = [
  { key: 'pt', label: 'Português' },
  { key: 'en', label: 'English' },
  { key: 'es', label: 'Español' },
];

function duration(seconds: number): string {
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}

/**
 * Matches to add, from JSON in the old partida.js shape: one match, a list of them, or the whole
 * `{ partidas, itens }`. The server checks every field when the page is saved.
 */
function parseMatches(text: string): { partidas: SiteMatch[]; itens: Record<string, SiteItem> } {
  const trimmed = text.trim().replace(/^window\.PARTIDAS\s*=\s*/, '').replace(/;\s*$/, '');
  const value: unknown = JSON.parse(trimmed);
  const isMatch = (m: unknown): m is SiteMatch =>
    typeof m === 'object' && m !== null && Array.isArray((m as SiteMatch).times);
  if (Array.isArray(value)) {
    if (!value.every(isMatch)) throw new Error('Cada item da lista precisa ser uma partida com "times".');
    return { partidas: value, itens: {} };
  }
  if (isMatch(value)) return { partidas: [value], itens: {} };
  if (typeof value === 'object' && value !== null && Array.isArray((value as SiteContent['matches']).partidas)) {
    const all = value as SiteContent['matches'];
    if (!all.partidas.every(isMatch)) throw new Error('Cada partida precisa de "times".');
    return { partidas: all.partidas, itens: all.itens ?? {} };
  }
  throw new Error('Cole uma partida, uma lista de partidas ou o conteúdo de partida.js.');
}

/** Everything the public site shows for this camp, in place of the old PHP staff panel. */
export function SiteView({ camp }: { camp: Camp }) {
  const [site, setSite] = useState<SiteState | null>(null);
  const [form, setForm] = useState<SiteContent | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [savedNow, setSavedNow] = useState(false);
  const [importText, setImportText] = useState('');
  const [importError, setImportError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setSite(null);
    setForm(null);
    setLoadError(null);
    api
      .site(camp.id)
      .then((loaded) => {
        if (cancelled) return;
        setSite(loaded);
        setForm(loaded.content);
      })
      .catch((err: Error) => !cancelled && setLoadError(err.message));
    return () => {
      cancelled = true;
    };
  }, [camp.id]);

  const changed = !!site && !!form && JSON.stringify(form) !== JSON.stringify(site.content);
  useEffect(() => {
    if (!changed) return;
    const warn = (e: BeforeUnloadEvent) => e.preventDefault();
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [changed]);

  if (loadError) return <p className="error">{loadError}</p>;
  if (!site || !form) return <p className="muted">Carregando…</p>;

  const update = (next: SiteContent) => {
    setForm(next);
    setSavedNow(false);
  };
  const setText = (card: SiteInfoKey, lang: keyof SiteTexts, value: string) =>
    update({ ...form, info: { ...form.info, [card]: { ...form.info[card], [lang]: value } } });
  const setPlayer = (index: number, patch: Partial<SiteContent['players'][number]>) =>
    update({ ...form, players: form.players.map((p, i) => (i === index ? { ...p, ...patch } : p)) });
  const setMatches = (partidas: SiteMatch[], itens = form.matches.itens) =>
    update({ ...form, matches: { partidas, itens } });

  /** The camp's own ranking, by points, with each player's Discord name. */
  const fillFromCamp = () => {
    const players = [...camp.players]
      .sort((a, b) => b.points - a.points)
      .map((p) => ({ nome: camp.members[p.user_id]?.name ?? p.user_id, pontos: p.points }));
    update({ ...form, players });
  };

  const addMatches = () => {
    setImportError(null);
    try {
      const added = parseMatches(importText);
      setMatches([...form.matches.partidas, ...added.partidas], { ...form.matches.itens, ...added.itens });
      setImportText('');
    } catch (err) {
      setImportError(err instanceof SyntaxError ? `JSON inválido: ${err.message}` : (err as Error).message);
    }
  };

  const save = (e: FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    api
      .saveSite(camp.id, form)
      .then((saved) => {
        setSite(saved);
        setForm(saved.content);
        setSavedNow(true);
      })
      .catch((err: Error) => setError(err.message))
      .finally(() => setSaving(false));
  };

  const publish = () => {
    setSaving(true);
    setError(null);
    api
      .publishSite(camp.id)
      .then(setSite)
      .catch((err: Error) => setError(err.message))
      .finally(() => setSaving(false));
  };

  return (
    <form className="split" onSubmit={save} aria-labelledby="site-title">
      <div className="split-main stack">
        <div className="card-head">
          <div className="title-row title-row-wrap">
            <h1 id="site-title" className="page-title">
              Site
            </h1>
            <span className="muted small">Camp {camp.id}</span>
          </div>
          <span className={site.published ? 'pill pill-running' : 'pill pill-muted'}>
            {site.published ? 'Este camp está no site' : 'Outro camp está no site'}
          </span>
        </div>

        <fieldset className="card settings-group">
          <legend className="settings-legend">Informações do campeonato</legend>
          <p className="muted small">Os quatro cartões do topo do site. Vazio aparece como “A definir”.</p>
          <div className="settings-pairs">
            {CARDS.map(({ key, label }) => (
              <fieldset key={key} className="field site-texts">
                <legend>{label}</legend>
                {LANGUAGES.map((lang) => (
                  <label key={lang.key} className="site-lang">
                    <span className="muted small">{lang.key.toUpperCase()}</span>
                    <input
                      className="input"
                      maxLength={80}
                      value={form.info[key][lang.key]}
                      aria-label={`${label} – ${lang.label}`}
                      onChange={(e) => setText(key, lang.key, e.target.value)}
                    />
                  </label>
                ))}
              </fieldset>
            ))}
          </div>
        </fieldset>

        <fieldset className="card settings-group">
          <legend className="settings-legend">Ranking de jogadores</legend>
          <p className="muted small">O site ordena pela pontuação. Linhas sem nome são ignoradas.</p>
          {form.players.length > 0 && (
            <div className="table-scroll">
              <table className="table site-rank">
                <thead>
                  <tr>
                    <th scope="col">Jogador</th>
                    <th scope="col">Pontos</th>
                    <th scope="col">
                      <span className="sr-only">Remover</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {form.players.map((p, i) => (
                    <tr key={i}>
                      <td>
                        <input
                          className="input"
                          maxLength={40}
                          value={p.nome}
                          aria-label="Nome do jogador"
                          onChange={(e) => setPlayer(i, { nome: e.target.value })}
                        />
                      </td>
                      <td>
                        <input
                          className="input site-points"
                          type="number"
                          step={1}
                          min={-99999}
                          max={999999}
                          value={p.pontos}
                          aria-label={`Pontos de ${p.nome || 'jogador'}`}
                          onChange={(e) => setPlayer(i, { pontos: Math.trunc(Number(e.target.value)) || 0 })}
                        />
                      </td>
                      <td className="align-end">
                        <button
                          type="button"
                          className="icon-button"
                          aria-label={`Remover ${p.nome || 'jogador'}`}
                          onClick={() => update({ ...form, players: form.players.filter((_, j) => j !== i) })}
                        >
                          ×
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          <div className="form-actions">
            <button
              type="button"
              className="button button-small button-ghost"
              onClick={() => update({ ...form, players: [...form.players, { nome: '', pontos: 0 }] })}
            >
              + Adicionar jogador
            </button>
            <button
              type="button"
              className="button button-small button-ghost"
              disabled={camp.players.length === 0}
              onClick={fillFromCamp}
            >
              Preencher com o ranking do camp
            </button>
          </div>
        </fieldset>

        <fieldset className="card settings-group">
          <legend className="settings-legend">Partidas</legend>
          <p className="muted small">
            O MVP é escolhido pela staff (não é o MVP do jogo) e aparece no resumo da página inicial e no histórico.
          </p>
          {form.matches.partidas.length === 0 && <p className="muted small">Nenhuma partida no site.</p>}
          <ol className="site-matches">
            {form.matches.partidas.map((m, i) => {
              const winner = m.times[m.vencedor]?.nome ?? '?';
              return (
                <li key={i} className="site-match">
                  <div className="site-match-head">
                    <strong>Partida {i + 1}</strong>
                    <span className="muted small">
                      {winner} venceu · {new Date(m.inicio * 1000).toLocaleDateString('pt-BR')} · ⏱ {duration(m.duracao)}
                      {m.id ? ` · #${m.id}` : ''}
                    </span>
                    <button
                      type="button"
                      className="icon-button"
                      aria-label={`Remover a partida ${i + 1}`}
                      onClick={() => setMatches(form.matches.partidas.filter((_, j) => j !== i))}
                    >
                      ×
                    </button>
                  </div>
                  <label className="field">
                    <span className="small">MVP</span>
                    <select
                      className="input"
                      value={m.mvp ?? ''}
                      onChange={(e) =>
                        setMatches(
                          form.matches.partidas.map((x, j) => (j === i ? { ...x, mvp: e.target.value || null } : x)),
                        )
                      }
                    >
                      <option value="">A definir</option>
                      {m.times.map((t, ti) => (
                        <optgroup key={ti} label={t.nome}>
                          {t.jogadores.map((p) => (
                            <option key={p.nome} value={p.nome}>
                              {p.nome} ({p.heroi} · {p.k}/{p.d}/{p.a})
                            </option>
                          ))}
                        </optgroup>
                      ))}
                    </select>
                  </label>
                </li>
              );
            })}
          </ol>
          <div className="field">
            <label htmlFor="site-import">Adicionar partidas (JSON)</label>
            <textarea
              id="site-import"
              className="input site-import"
              rows={5}
              spellCheck={false}
              value={importText}
              placeholder='{"inicio": 1794919042, "duracao": 2026, "vencedor": 0, "mvp": null, "times": [...]}'
              onChange={(e) => setImportText(e.target.value)}
            />
            <span className="muted small">
              No formato do antigo partida.js: uma partida, uma lista delas ou o arquivo inteiro. Entram no fim da lista.
            </span>
            {importError && (
              <span className="error small" role="alert">
                {importError}
              </span>
            )}
          </div>
          <div className="form-actions">
            <button
              type="button"
              className="button button-small button-ghost"
              disabled={!importText.trim()}
              onClick={addMatches}
            >
              Adicionar
            </button>
          </div>
        </fieldset>

        {error && (
          <p className="error small" role="alert">
            {error}
          </p>
        )}
        <div className="form-actions">
          <button type="submit" className="button button-primary" disabled={saving || !changed}>
            {saving ? 'Salvando…' : 'Salvar alterações'}
          </button>
          {changed && (
            <button type="button" className="button button-ghost" disabled={saving} onClick={() => setForm(site.content)}>
              Descartar
            </button>
          )}
          <span className="muted small" role="status">
            {savedNow ? (site.published ? 'Salvo. Já está no site.' : 'Salvo.') : changed ? 'Alterações ainda não salvas.' : ''}
          </span>
        </div>
      </div>
      <aside className="split-aside">
        <section className="card" aria-labelledby="site-published-title">
          <h2 id="site-published-title" className="small-title">
            Site público
          </h2>
          <a href={site.url} target="_blank" rel="noreferrer" className="link-row small">
            <span className="mono ellipsis">{site.url.replace(/^https?:\/\//, '')}</span>
            <ExternalIcon size={15} />
          </a>
          {site.published ? (
            <p className="muted small">Este camp é o que o site mostra. Ao salvar, a mudança aparece no site na hora.</p>
          ) : (
            <>
              <p className="muted small">
                O site mostra outro camp (ou nenhum). Salvar aqui não muda o site até este camp ser o publicado.
              </p>
              <div className="form-actions">
                <button type="button" className="button button-small" disabled={saving || changed} onClick={publish}>
                  Mostrar este camp no site
                </button>
              </div>
              {changed && <p className="muted small">Salve ou descarte as alterações antes.</p>}
            </>
          )}
        </section>
      </aside>
    </form>
  );
}
