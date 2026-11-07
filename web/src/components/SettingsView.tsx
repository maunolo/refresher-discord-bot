import { useEffect, useState, type FormEvent } from 'react';

import { api, type Settings, type SettingsForm } from '../api';

type VoiceKey =
  | 'queue_voice_id'
  | 'match1_hiddenking_voice_id'
  | 'match1_archmother_voice_id'
  | 'match2_hiddenking_voice_id'
  | 'match2_archmother_voice_id';

/** Each voice slot, with the channel name the bot looks for when none is picked. */
const VOICE: { key: VoiceKey; label: string; name: string }[] = [
  { key: 'queue_voice_id', label: 'Fila', name: 'Queue' },
  { key: 'match1_hiddenking_voice_id', label: 'Partida 1 · Hiddenking', name: 'Match 1 Hiddenking' },
  { key: 'match1_archmother_voice_id', label: 'Partida 1 · Archmother', name: 'Match 1 Archmother' },
  { key: 'match2_hiddenking_voice_id', label: 'Partida 2 · Hiddenking', name: 'Match 2 Hiddenking' },
  { key: 'match2_archmother_voice_id', label: 'Partida 2 · Archmother', name: 'Match 2 Archmother' },
];

function formOf(settings: Settings): SettingsForm {
  return {
    queue_voice_id: settings.queue_voice_id,
    match1_hiddenking_voice_id: settings.match1_hiddenking_voice_id,
    match1_archmother_voice_id: settings.match1_archmother_voice_id,
    match2_hiddenking_voice_id: settings.match2_hiddenking_voice_id,
    match2_archmother_voice_id: settings.match2_archmother_voice_id,
    create_lobby: settings.create_lobby,
    server_region: settings.server_region,
    use_draft_tool: settings.use_draft_tool,
    draft_timer_seconds: settings.draft_timer_seconds,
  };
}

function timerLabel(seconds: number): string {
  return seconds === 0 ? 'Sem tempo (5 minutos, o máximo do DeadChaps)' : `${seconds} segundos`;
}

/** How this server's camps run: voice channels, the Deadlock lobby and the draft timer. */
export function SettingsView({ guildId, guildName }: { guildId: string; guildName: string }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [form, setForm] = useState<SettingsForm | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [savedNow, setSavedNow] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setSettings(null);
    setForm(null);
    setLoadError(null);
    api
      .settings(guildId)
      .then((loaded) => {
        if (cancelled) return;
        setSettings(loaded);
        setForm(formOf(loaded));
      })
      .catch((err: Error) => !cancelled && setLoadError(err.message));
    return () => {
      cancelled = true;
    };
  }, [guildId]);

  if (loadError) return <p className="error">{loadError}</p>;
  if (!settings || !form) return <p className="muted">Carregando…</p>;

  const changed = JSON.stringify(form) !== JSON.stringify(formOf(settings));
  const update = (patch: Partial<SettingsForm>) => {
    setForm({ ...form, ...patch });
    setSavedNow(false);
  };
  const submit = (e: FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    api
      .saveSettings(guildId, form)
      .then((saved) => {
        setSettings(saved);
        setForm(formOf(saved));
        setSavedNow(true);
      })
      .catch((err: Error) => setError(err.message))
      .finally(() => setSaving(false));
  };

  return (
    <form className="split" onSubmit={submit} aria-labelledby="settings-title">
      <div className="split-main stack">
        <div className="card-head">
          <div className="title-row title-row-wrap">
            <h1 id="settings-title" className="page-title">
              Configurações
            </h1>
            <span className="muted small">{guildName}</span>
          </div>
          <span className={settings.saved ? 'pill pill-running' : 'pill pill-muted'}>
            {settings.saved ? 'Salvas para este servidor' : 'Usando os valores do .env'}
          </span>
        </div>

        <fieldset className="card settings-group">
          <legend className="settings-legend">Canais de voz</legend>
          <p className="muted small">
            Para onde o bot move os jogadores. Em automático, ele usa o canal com o nome indicado e o cria se não existir.
          </p>
          <div className="settings-grid">
            {VOICE.map(({ key, label, name }) => {
              const value = form[key] ?? '';
              const missing = value !== '' && !settings.voice_channels.some((c) => c.id === value);
              return (
                <div key={key} className="field">
                  <label htmlFor={`voice-${key}`}>{label}</label>
                  <select
                    id={`voice-${key}`}
                    className="input"
                    value={value}
                    onChange={(e) => update({ [key]: e.target.value || null })}
                  >
                    <option value="">Automático: “{name}”</option>
                    {missing && <option value={value}>Canal removido do servidor</option>}
                    {settings.voice_channels.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.name}
                      </option>
                    ))}
                  </select>
                  {missing && <span className="error small">Esse canal não existe mais. Escolha outro.</span>}
                </div>
              );
            })}
          </div>
        </fieldset>

        <fieldset className="card settings-group">
          <legend className="settings-legend">Lobby do Deadlock</legend>
          <div className="settings-switch">
            <button
              type="button"
              role="switch"
              aria-checked={form.create_lobby}
              aria-labelledby="lobby-label"
              className={form.create_lobby ? 'switch switch-on' : 'switch'}
              onClick={() => update({ create_lobby: !form.create_lobby })}
            >
              <span className="switch-knob" />
            </button>
            <span id="lobby-label">
              <strong>Criar o lobby ao iniciar a partida</strong>
              <span className="muted small"> O bot cria a partida personalizada e posta o código da party no canal.</span>
            </span>
          </div>
          <div className="field">
            <label htmlFor="region">Região do servidor</label>
            <input
              id="region"
              className="input"
              value={form.server_region ?? ''}
              maxLength={40}
              disabled={!form.create_lobby}
              placeholder="Padrão da API"
              onChange={(e) => update({ server_region: e.target.value || null })}
            />
            <span className="muted small">O mesmo valor de DEADLOCK_SERVER_REGION: nome ou número da região.</span>
          </div>
        </fieldset>

        <fieldset className="card settings-group">
          <legend className="settings-legend">Draft</legend>
          <div className="settings-switch">
            <button
              type="button"
              role="switch"
              aria-checked={form.use_draft_tool}
              aria-labelledby="draft-tool-label"
              className={form.use_draft_tool ? 'switch switch-on' : 'switch'}
              onClick={() => update({ use_draft_tool: !form.use_draft_tool })}
            >
              <span className="switch-knob" />
            </button>
            <span id="draft-tool-label">
              <strong>Criar o draft no DeadChaps</strong>
              <span className="muted small">
                {form.use_draft_tool
                  ? ' O bot cria o draft e manda aos capitães, por DM, o link para entrar no draft.'
                  : ' Desligado: o bot só move os jogadores e avisa no canal. Você cria o draft e manda o link para os capitães, como o código da partida.'}
              </span>
            </span>
          </div>
          <div className="field">
            <label htmlFor="draft-timer">Tempo por escolha e banimento</label>
            <select
              id="draft-timer"
              className="input"
              value={form.draft_timer_seconds}
              disabled={!form.use_draft_tool}
              onChange={(e) => update({ draft_timer_seconds: Number(e.target.value) })}
            >
              {settings.draft_timers.map((t) => (
                <option key={t} value={t}>
                  {timerLabel(t)}
                </option>
              ))}
            </select>
          </div>
        </fieldset>

        {error && (
          <p className="error small" role="alert">
            {error}
          </p>
        )}
        <div className="form-actions">
          <button type="submit" className="button button-primary" disabled={saving || !changed}>
            {saving ? 'Salvando…' : 'Salvar configurações'}
          </button>
          {changed && (
            <button type="button" className="button button-ghost" disabled={saving} onClick={() => setForm(formOf(settings))}>
              Descartar
            </button>
          )}
          <span className="muted small" role="status">
            {savedNow ? 'Salvo.' : changed ? 'Alterações ainda não salvas.' : ''}
          </span>
        </div>
      </div>
      <aside className="split-aside">
        <section className="card" aria-labelledby="settings-when-title">
          <h2 id="settings-when-title" className="small-title">
            Quando vale
          </h2>
          <p className="muted small">
            Na próxima ação do bot. Uma partida com draft já criado mantém o tempo dele; o lobby é decidido ao iniciar a
            partida.
          </p>
          <p className="muted small">Vale para todos os camps deste servidor, e só para ele.</p>
        </section>
      </aside>
    </form>
  );
}
