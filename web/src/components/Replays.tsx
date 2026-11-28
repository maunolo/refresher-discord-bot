import { type DragEvent, useEffect, useRef, useState } from 'react';

import { api, type Clip, type ClipPlay, type ClipsState, type OverlayLinks, type PauseScreen } from '../api';
import { FilmIcon, UploadIcon } from './Icons';
import { OverlayLink } from './OverlayLink';

const TYPES = ['video/mp4', 'video/webm'];
const STREAMS = [1, 2];
/** Whose channel each stream is, as the pause screen's controls name them. */
const STREAM_NAMES: Record<number, string> = { 1: 'FARINHAGM', 2: 'NOKEIMEE' };

/** `1:05`. */
function clock(ms: number): string {
  const seconds = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}

function megabytes(bytes: number): string {
  return `${(bytes / 1024 / 1024).toLocaleString('pt-BR', { maximumFractionDigits: 1 })} MB`;
}

/** The file's length, read by the browser; `null` when it can't show the video. */
function readDuration(file: File): Promise<number | null> {
  return new Promise((resolve) => {
    const url = URL.createObjectURL(file);
    const element = document.createElement('video');
    element.preload = 'metadata';
    element.muted = true;
    const done = (ms: number | null) => {
      URL.revokeObjectURL(url);
      resolve(ms);
    };
    element.onloadedmetadata = () =>
      done(element.videoWidth > 0 && Number.isFinite(element.duration) ? Math.round(element.duration * 1000) : null);
    element.onerror = () => done(null);
    element.src = url;
  });
}

/** A failed upload; `retry` when trying again may work (R2's own errors, a dropped connection). */
class UploadError extends Error {
  constructor(
    message: string,
    readonly retry: boolean,
  ) {
    super(message);
  }
}

/** `PUT`s the file to the bucket once, reporting progress from 0 to 1. */
function putOnce(url: string, file: File, onProgress: (share: number) => void): Promise<void> {
  return new Promise((resolve, reject) => {
    const request = new XMLHttpRequest();
    request.open('PUT', url);
    request.setRequestHeader('Content-Type', file.type);
    request.upload.onprogress = (e) => e.lengthComputable && onProgress(e.loaded / e.total);
    request.onload = () =>
      request.status >= 200 && request.status < 300
        ? resolve()
        : reject(new UploadError(`o armazenamento respondeu ${request.status}`, request.status >= 500 || request.status === 429));
    request.onerror = () => reject(new UploadError('a conexão caiu durante o envio', true));
    request.send(file);
  });
}

/** Waits before each retry: R2 sometimes fails for a moment ("InternalError") and then works. */
const RETRY_DELAYS_MS = [1_000, 3_000, 8_000];

/** `PUT`s the file, trying again after R2's passing errors; `onRetry` says which attempt is next. */
async function putFile(
  url: string,
  file: File,
  onProgress: (share: number) => void,
  onRetry: (attempt: number) => void,
): Promise<void> {
  for (let attempt = 0; ; attempt++) {
    try {
      await putOnce(url, file, onProgress);
      return;
    } catch (err) {
      const delay = RETRY_DELAYS_MS[attempt];
      if (!(err instanceof UploadError) || !err.retry || delay === undefined) throw err;
      onRetry(attempt + 2);
      onProgress(0);
      await new Promise((resume) => window.setTimeout(resume, delay));
    }
  }
}

function Upload({ guildId, maxBytes, onDone }: { guildId: string; maxBytes: number; onDone: (state: ClipsState) => void }) {
  const [file, setFile] = useState<File | null>(null);
  const [title, setTitle] = useState('');
  const [duration, setDuration] = useState<number | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  /** The attempt under way, from the second one. */
  const [attempt, setAttempt] = useState<number | null>(null);
  const input = useRef<HTMLInputElement>(null);

  const choose = async (picked: File | null) => {
    setError(null);
    setFile(picked);
    setDuration(null);
    if (!picked) return;
    setTitle(picked.name.replace(/\.[^.]+$/, '').slice(0, 80));
    if (!TYPES.includes(picked.type)) {
      setError('Envie um vídeo MP4 (H.264 + AAC) ou WebM.');
      return;
    }
    if (picked.size > maxBytes) {
      setError('O clipe pode ter até 1 GB.');
      return;
    }
    const ms = await readDuration(picked);
    if (ms === null) {
      setError('O navegador não consegue mostrar este vídeo, então o OBS também não vai. Exporte em MP4 com H.264.');
      return;
    }
    setDuration(ms);
  };

  const send = async () => {
    if (!file || duration === null) return;
    setError(null);
    setProgress(0);
    try {
      const { clip_id, upload_url } = await api.newClip(guildId, {
        title,
        content_type: file.type,
        size_bytes: file.size,
      });
      await putFile(upload_url, file, setProgress, setAttempt);
      onDone(await api.clipReady(guildId, clip_id, duration));
      setFile(null);
      setTitle('');
      setDuration(null);
      if (input.current) input.current.value = '';
    } catch (err) {
      setError(`Não deu para enviar: ${(err as Error).message}.`);
    } finally {
      setProgress(null);
      setAttempt(null);
    }
  };

  const sending = progress !== null;
  const drop = (e: DragEvent) => {
    e.preventDefault();
    setDragging(false);
    if (!sending) void choose(e.dataTransfer.files[0] ?? null);
  };
  return (
    <div className="stack">
      {/* The real input is hidden; the zone is its label, so a click (or Enter on it) opens the picker. */}
      <input
        id="clip-file"
        ref={input}
        className="visually-hidden"
        type="file"
        accept="video/mp4,video/webm"
        disabled={sending}
        onChange={(e) => void choose(e.target.files?.[0] ?? null)}
      />
      <label
        htmlFor="clip-file"
        className={['dropzone', dragging && 'dropzone-over', file && 'dropzone-filled', sending && 'dropzone-busy']
          .filter(Boolean)
          .join(' ')}
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            input.current?.click();
          }
        }}
        onDragOver={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={drop}
      >
        {file ? (
          <>
            <FilmIcon size={28} />
            <span className="dropzone-text">
              <strong className="ellipsis">{file.name}</strong>
              <span className="muted small">
                {duration !== null ? `${clock(duration)} · ` : ''}
                {megabytes(file.size)}
              </span>
            </span>
            {!sending && <span className="button button-small button-ghost dropzone-change">Trocar</span>}
          </>
        ) : (
          <>
            <UploadIcon size={28} />
            <span className="dropzone-text">
              <strong>Arraste o vídeo aqui ou clique para escolher</strong>
              <span className="muted small">MP4 (H.264 + AAC) ou WebM, até 1 GB</span>
            </span>
          </>
        )}
      </label>
      <div className="field">
        <label htmlFor="clip-title">Nome do replay</label>
        <input
          id="clip-title"
          className="input"
          maxLength={80}
          placeholder="Ex.: Kashi 1v3 na base"
          disabled={sending}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
      </div>
      {error && (
        <p className="error small" role="alert">
          {error}
        </p>
      )}
      <div className="form-actions">
        <button
          type="button"
          className="button button-primary"
          disabled={sending || !file || duration === null || !title.trim() || !!error}
          onClick={() => void send()}
        >
          {sending
            ? `${attempt ? `Tentativa ${attempt} · ` : ''}Enviando… ${Math.round((progress ?? 0) * 100)}%`
            : 'Enviar replay'}
        </button>
        {sending && (
          <div className="progress clip-progress" role="progressbar" aria-valuenow={Math.round((progress ?? 0) * 100)}>
            <div className="progress-bar" style={{ width: `${(progress ?? 0) * 100}%` }} />
          </div>
        )}
      </div>
      <p className="muted small">
        H.264 com áudio AAC é o padrão do OBS, do ShadowPlay e da maioria dos editores. O vídeo vai direto do seu navegador
        para o armazenamento, sem passar pelo bot.
      </p>
    </div>
  );
}

/** What a stream plays now, counting from when the list loaded. */
function NowPlaying({
  play,
  loadedAt,
  now,
  busy,
  onStop,
}: {
  play: ClipPlay | undefined;
  loadedAt: number;
  now: number;
  busy: boolean;
  onStop: () => void;
}) {
  const elapsed = play ? play.elapsed_ms + (now - loadedAt) : 0;
  const on = play && (play.duration_ms === null ? elapsed < 15_000 : elapsed < play.duration_ms);
  return (
    <span className="replay-now">
      {on && play ? (
        <>
          <span className="pill pill-live">
            <span className="dot" />
            Tocando
          </span>
          <span className="ellipsis">{play.title}</span>
          {play.duration_ms !== null && (
            <span className="mono muted small">
              {clock(elapsed)} / {clock(play.duration_ms)}
            </span>
          )}
          <button type="button" className="button button-small button-ghost" disabled={busy} onClick={onStop}>
            Parar
          </button>
        </>
      ) : (
        <span className="muted small">Nada tocando.</span>
      )}
    </span>
  );
}

function ClipRow({
  clip,
  busy,
  onPlay,
  onDelete,
}: {
  clip: Clip;
  busy: boolean;
  onPlay: (streams: number[]) => void;
  onDelete: () => void;
}) {
  const [preview, setPreview] = useState(false);
  const [confirming, setConfirming] = useState(false);
  return (
    <li className="clip-row">
      <div className="clip-info">
        <strong className="ellipsis">{clip.title}</strong>
        <span className="muted small">
          {clip.duration_ms !== null ? `${clock(clip.duration_ms)} · ` : ''}
          {megabytes(clip.size_bytes)} · {new Date(clip.created_at * 1000).toLocaleDateString('pt-BR')}
        </span>
      </div>
      <div className="inline-actions clip-actions">
        <button type="button" className="button button-small" disabled={busy} onClick={() => onPlay([1])}>
          Tocar na 1
        </button>
        <button type="button" className="button button-small" disabled={busy} onClick={() => onPlay([2])}>
          Tocar na 2
        </button>
        <button type="button" className="button button-small" disabled={busy} onClick={() => onPlay([1, 2])}>
          Nas duas
        </button>
        <button type="button" className="button button-small button-ghost" onClick={() => setPreview(!preview)}>
          {preview ? 'Fechar' : 'Prévia'}
        </button>
        {confirming ? (
          <>
            <button type="button" className="button button-small button-danger" disabled={busy} onClick={onDelete}>
              Apagar mesmo
            </button>
            <button type="button" className="button button-small button-ghost" onClick={() => setConfirming(false)}>
              Cancelar
            </button>
          </>
        ) : (
          <button type="button" className="button button-small button-ghost" onClick={() => setConfirming(true)}>
            Apagar
          </button>
        )}
      </div>
      {preview && <video className="clip-preview" src={clip.url} controls preload="metadata" />}
    </li>
  );
}

const PRESET_MINUTES = [3, 5, 10, 15];

/**
 * The pause screen on the replay overlays: a countdown under a message ("Já voltamos"), started
 * here with a length, given more time or taken off.
 */
function PauseControls({ guildId }: { guildId: string }) {
  const [pauses, setPauses] = useState<PauseScreen[] | null>(null);
  const [loadedAt, setLoadedAt] = useState(Date.now());
  const [now, setNow] = useState(Date.now());
  const [minutes, setMinutes] = useState('5');
  const [message, setMessage] = useState('Já voltamos');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const show = (fresh: { pauses: PauseScreen[] }) => {
    setPauses(fresh.pauses);
    setLoadedAt(Date.now());
  };

  useEffect(() => {
    let cancelled = false;
    setPauses(null);
    api
      .pauses(guildId)
      .then((loaded) => !cancelled && show(loaded))
      .catch((err: Error) => !cancelled && setError(err.message));
    return () => {
      cancelled = true;
    };
  }, [guildId]);

  const anyOn = (pauses?.length ?? 0) > 0;
  useEffect(() => {
    if (!anyOn) return;
    const timer = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(timer);
  }, [anyOn]);

  const act = (action: () => Promise<{ pauses: PauseScreen[] }>) => {
    setBusy(true);
    setError(null);
    action()
      .then(show)
      .catch((err: Error) => setError(err.message))
      .finally(() => setBusy(false));
  };

  const durationMs = Math.round(Number(minutes.replace(',', '.')) * 60_000);
  const valid = Number.isFinite(durationMs) && durationMs >= 1_000 && durationMs <= 3 * 60 * 60_000;
  const start = (streams: number[]) => act(() => api.startPause(guildId, streams, durationMs, message));

  return (
    <section className="card stack" aria-labelledby="pause-title">
      <h2 id="pause-title" className="small-title">
        Pausa
      </h2>
      <p className="muted small">
        O overlay sempre mostra o fundo da liga com “Já voltamos”, o logo e o título no centro. A pausa troca a mensagem
        e põe o contador; um replay entra à direita e esse bloco vai para a esquerda.
      </p>
      <div className="pause-form">
        <div className="field pause-minutes">
          <label htmlFor="pause-minutes">Minutos</label>
          <input
            id="pause-minutes"
            className="input"
            inputMode="decimal"
            value={minutes}
            disabled={busy}
            onChange={(e) => setMinutes(e.target.value)}
          />
        </div>
        <div className="inline-actions">
          {PRESET_MINUTES.map((m) => (
            <button
              key={m}
              type="button"
              className={String(m) === minutes ? 'button button-small' : 'button button-small button-ghost'}
              disabled={busy}
              onClick={() => setMinutes(String(m))}
            >
              {m} min
            </button>
          ))}
        </div>
        <div className="field pause-message-field">
          <label htmlFor="pause-message">Mensagem</label>
          <input
            id="pause-message"
            className="input"
            maxLength={60}
            placeholder="Já voltamos"
            value={message}
            disabled={busy}
            onChange={(e) => setMessage(e.target.value)}
          />
        </div>
      </div>
      <div className="inline-actions">
        <button type="button" className="button button-primary" disabled={busy || !valid} onClick={() => start([1])}>
          Iniciar na {STREAM_NAMES[1]}
        </button>
        <button type="button" className="button button-primary" disabled={busy || !valid} onClick={() => start([2])}>
          Iniciar na {STREAM_NAMES[2]}
        </button>
        <button type="button" className="button button-primary" disabled={busy || !valid} onClick={() => start([1, 2])}>
          Nas duas
        </button>
      </div>
      {!valid && <p className="error small">O contador vai de 1 segundo a 3 horas.</p>}
      {error && (
        <p className="error small" role="alert">
          {error}
        </p>
      )}
      {STREAMS.map((stream) => {
        const pause = pauses?.find((p) => p.stream === stream);
        const left = pause ? Math.max(0, pause.remaining_ms - (now - loadedAt)) : 0;
        return (
          <div key={stream} className="replay-now">
            <strong className="small">{STREAM_NAMES[stream]}</strong>
            {pause ? (
              <>
                <span className="pill pill-live">
                  <span className="dot" />
                  Pausa
                </span>
                <span className="ellipsis">{pause.message}</span>
                <span className="mono small">{left > 0 ? clock(left) : 'tempo esgotado'}</span>
                <button
                  type="button"
                  className="button button-small button-ghost"
                  disabled={busy}
                  onClick={() => act(() => api.addPause(guildId, [stream], -60_000))}
                >
                  −1 min
                </button>
                <button
                  type="button"
                  className="button button-small button-ghost"
                  disabled={busy}
                  onClick={() => act(() => api.addPause(guildId, [stream], 60_000))}
                >
                  +1 min
                </button>
                <button
                  type="button"
                  className="button button-small"
                  disabled={busy}
                  onClick={() => act(() => api.stopPause(guildId, [stream]))}
                >
                  Tirar pausa
                </button>
              </>
            ) : (
              <span className="muted small">Sem pausa.</span>
            )}
          </div>
        );
      })}
    </section>
  );
}

/** Replay clips: upload them, then play one on a stream's replay overlay. */
export function Replays({ guildId, links }: { guildId: string; links: OverlayLinks | null }) {
  const [state, setState] = useState<ClipsState | null>(null);
  const [loadedAt, setLoadedAt] = useState(Date.now());
  const [now, setNow] = useState(Date.now());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const show = (fresh: ClipsState) => {
    setState(fresh);
    setLoadedAt(Date.now());
  };

  useEffect(() => {
    let cancelled = false;
    setState(null);
    api
      .clips(guildId)
      .then((loaded) => !cancelled && show(loaded))
      .catch((err: Error) => !cancelled && setError(err.message));
    return () => {
      cancelled = true;
    };
  }, [guildId]);

  // The "playing" timers tick while something plays.
  const anyPlaying = (state?.playing.length ?? 0) > 0;
  useEffect(() => {
    if (!anyPlaying) return;
    const timer = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(timer);
  }, [anyPlaying]);

  const act = (action: () => Promise<ClipsState>) => {
    setBusy(true);
    setError(null);
    action()
      .then(show)
      .catch((err: Error) => setError(err.message))
      .finally(() => setBusy(false));
  };

  return (
    <section className="stack" aria-labelledby="replays-title">
      <div className="title-row">
        <h2 id="replays-title" className="section-title">
          Pausa e replays
        </h2>
      </div>
      <p className="muted small">
        Cada transmissão tem um overlay de pausa e replay: a tela de pausa com contador, e os clipes de lances, que aparecem
        com o som e somem sozinhos no fim.
      </p>
      {links && (
        <section className="card stack" aria-labelledby="pause-links-title">
          <h2 id="pause-links-title" className="small-title">
            Overlays de pausa e replay
          </h2>
          {STREAMS.map((stream) => (
            <OverlayLink
              key={stream}
              title={STREAM_NAMES[stream]}
              url={links.replay_urls[stream - 1]}
              label={`Copiar o link do overlay de pausa e replay da ${STREAM_NAMES[stream]}`}
            />
          ))}
          <p className="muted small">
            Fonte de navegador de 1920 × 1080 acima das outras, com “Controlar áudio pelo OBS” ligado para o som do clipe
            entrar no mixer. Sem pausa e sem replay, fica a tela de “Já voltamos”.
          </p>
        </section>
      )}
      <PauseControls guildId={guildId} />
      {error && (
        <p className="error small" role="alert">
          {error}
        </p>
      )}
      {!state ? (
        !error && <p className="muted small">Carregando…</p>
      ) : !state.enabled ? (
        <section className="card">
          <p className="small">
            Os replays estão desligados: o servidor precisa das variáveis <span className="mono">R2_*</span> do bucket da
            Cloudflare.
          </p>
        </section>
      ) : (
        <>
          <section className="card stack" aria-labelledby="replay-links-title">
            <h2 id="replay-links-title" className="small-title">
              Replay tocando
            </h2>
            {STREAMS.map((stream) => (
              <div key={stream} className="stack replay-stream">
                <strong className="small">Transmissão {stream}</strong>
                <NowPlaying
                  play={state.playing.find((p) => p.stream === stream)}
                  loadedAt={loadedAt}
                  now={now}
                  busy={busy}
                  onStop={() => act(() => api.stopReplay(guildId, [stream]))}
                />
              </div>
            ))}
          </section>

          <section className="card stack" aria-labelledby="replay-upload-title">
            <h2 id="replay-upload-title" className="small-title">
              Enviar replay
            </h2>
            <Upload guildId={guildId} maxBytes={state.max_bytes} onDone={show} />
          </section>

          <section className="card stack" aria-labelledby="replay-list-title">
            <h2 id="replay-list-title" className="small-title">
              Clipes <span className="mono muted">{state.clips.length}</span>
            </h2>
            {state.clips.length === 0 ? (
              <p className="empty">Nenhum replay enviado ainda.</p>
            ) : (
              <ul className="clip-list" aria-label="Clipes de replay">
                {state.clips.map((clip) => (
                  <ClipRow
                    key={clip.id}
                    clip={clip}
                    busy={busy}
                    onPlay={(streams) => act(() => api.playClip(guildId, clip.id, streams))}
                    onDelete={() => act(() => api.deleteClip(guildId, clip.id))}
                  />
                ))}
              </ul>
            )}
          </section>
        </>
      )}
    </section>
  );
}
