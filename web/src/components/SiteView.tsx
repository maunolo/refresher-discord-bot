import { useEffect, useState, type FormEvent } from 'react';

import { api, type Camp, type SiteInfo, type SiteInfoKey, type SiteState, type SiteTexts } from '../api';
import { routeHref } from '../route';
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

/** The public site: its info cards, and which camp's ranking and matches it shows. */
export function SiteView({ camp }: { camp: Camp }) {
  const [site, setSite] = useState<SiteState | null>(null);
  const [form, setForm] = useState<SiteInfo | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [savedNow, setSavedNow] = useState(false);

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
        setForm(loaded.info);
      })
      .catch((err: Error) => !cancelled && setLoadError(err.message));
    return () => {
      cancelled = true;
    };
  }, [camp.id]);

  const changed = !!site && !!form && JSON.stringify(form) !== JSON.stringify(site.info);
  useEffect(() => {
    if (!changed) return;
    const warn = (e: BeforeUnloadEvent) => e.preventDefault();
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [changed]);

  if (loadError) return <p className="error">{loadError}</p>;
  if (!site || !form) return <p className="muted">Carregando…</p>;

  const setText = (card: SiteInfoKey, lang: keyof SiteTexts, value: string) => {
    setForm({ ...form, [card]: { ...form[card], [lang]: value } });
    setSavedNow(false);
  };

  const save = (e: FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    api
      .saveSite(camp.id, form)
      .then((saved) => {
        setSite(saved);
        setForm(saved.info);
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

  const missing = site.finished_matches - site.scoreboards;

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
                      value={form[key][lang.key]}
                      aria-label={`${label} – ${lang.label}`}
                      onChange={(e) => setText(key, lang.key, e.target.value)}
                    />
                  </label>
                ))}
              </fieldset>
            ))}
          </div>
        </fieldset>

        {error && (
          <p className="error small" role="alert">
            {error}
          </p>
        )}
        <div className="form-actions">
          <button type="submit" className="button button-primary" disabled={saving || !changed}>
            {saving ? 'Salvando…' : 'Salvar informações'}
          </button>
          {changed && (
            <button type="button" className="button button-ghost" disabled={saving} onClick={() => setForm(site.info)}>
              Descartar
            </button>
          )}
          <span className="muted small" role="status">
            {savedNow ? (site.published ? 'Salvo. Já está no site.' : 'Salvo.') : changed ? 'Alterações ainda não salvas.' : ''}
          </span>
        </div>
      </div>
      <aside className="split-aside">
        <section className="card" aria-labelledby="site-camp-title">
          <h2 id="site-camp-title" className="small-title">
            Dados do camp
          </h2>
          <a href={site.url} target="_blank" rel="noreferrer" className="link-row small">
            <span className="mono ellipsis">{site.url.replace(/^https?:\/\//, '')}</span>
            <ExternalIcon size={15} />
          </a>
          <p className="muted small">
            O ranking e o histórico de partidas do site saem do camp publicado: pontos, MVPs e o placar de cada partida
            vindo da API do Deadlock. Tudo atualiza sozinho.
          </p>
          {site.published ? (
            <p className="small">Este camp é o que o site mostra.</p>
          ) : (
            <div className="form-actions">
              <button type="button" className="button button-small" disabled={saving} onClick={publish}>
                Usar os dados deste camp no site
              </button>
            </div>
          )}
          <p className="muted small">
            {site.finished_matches === 0
              ? 'Nenhuma partida terminada ainda.'
              : `${site.scoreboards} de ${site.finished_matches} partidas terminadas já têm placar.`}
            {missing > 0 && (
              <>
                {' '}
                As outras esperam a API do Deadlock ou o ID da partida, na página de cada uma em{' '}
                <a href={routeHref('matches')}>Partidas</a>.
              </>
            )}
          </p>
        </section>
      </aside>
    </form>
  );
}
