import { useEffect, useRef, useState, type FormEvent } from 'react';

import { api, type CameraConfig, type CameraSource, type OverlayLinks, type StreamCameras } from '../api';
import { CopyButton } from './common';

/** A link without its secret, for showing on a page that may be on stream. */
function masked(url: string): string {
  return url.replace(/^https?:\/\//, '').replace(/\/overlay\/[0-9a-f]+\//, '/overlay/•••/');
}

function OverlayLink({ title, url, label }: { title: string; url: string; label: string }) {
  return (
    <div className="link-row">
      <span className="small">
        <strong>{title}</strong>
      </span>
      <a href={url} target="_blank" rel="noreferrer" className="mono small ellipsis">
        {masked(url)}
      </a>
      <CopyButton text={url} label={label} />
    </div>
  );
}

/** The camp overlays' links, and a new secret for all of them if one leaks. */
function CampOverlays({ links, onReset }: { links: OverlayLinks; onReset: () => Promise<void> }) {
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const reset = () => {
    setBusy(true);
    onReset()
      .then(() => setConfirming(false))
      .finally(() => setBusy(false));
  };
  return (
    <section className="card" aria-labelledby="camp-overlays-title">
      <h2 id="camp-overlays-title" className="small-title">
        Votação e ranking
      </h2>
      <OverlayLink title="Votação" url={links.vote_url} label="Copiar o link do overlay da votação" />
      <p className="muted small">
        Aparece quando uma partida abre a votação de refresh e mostra quem já votou, não o voto. Quando a votação acaba,
        mostra o resultado por 10 segundos e some. Tamanho sugerido: 700 × 480, ou 700 × 960 para as duas partidas. Para
        uma partida só, termine o link com <span className="mono">?partida=1</span> ou <span className="mono">?partida=2</span>.
      </p>
      <OverlayLink title="Ranking" url={links.ranking_url} label="Copiar o link do overlay do ranking" />
      <p className="muted small">
        O top 10 por pontos e MVPs, com vitórias e derrotas.
        Quando o camp acaba, mostra o ranking final até o próximo começar. Tamanho sugerido: 440 × 700; para mostrar mais
        jogadores, termine o link com <span className="mono">?top=15</span>.
      </p>
      {confirming ? (
        <div className="form-actions">
          <p className="small">
            Todos os links desta página param de funcionar, os das câmeras também; atualize as fontes no OBS com os novos.
          </p>
          <button type="button" className="button button-small" disabled={busy} onClick={reset}>
            {busy ? 'Gerando…' : 'Gerar links novos'}
          </button>
          <button type="button" className="button button-small button-ghost" disabled={busy} onClick={() => setConfirming(false)}>
            Cancelar
          </button>
        </div>
      ) : (
        <div className="form-actions">
          <button type="button" className="button button-small button-ghost" onClick={() => setConfirming(true)}>
            Gerar links novos
          </button>
          <span className="muted small">se um link vazar</span>
        </div>
      )}
    </section>
  );
}

const MAX_CAMERAS = 16;

/** VDO.Ninja turns anything but letters, digits and `_` into `_`. */
const vdoId = (text: string) => text.trim().replace(/\W+/g, '_');

/** The ID in a pasted VDO.Ninja link (`push`, `view`, …), or the text as it is. */
function idFrom(text: string): string {
  const value = text.trim();
  if (!/^https?:\/\//i.test(value) && !value.includes('?')) return value;
  try {
    const url = new URL(value, 'https://vdo.ninja/');
    for (const key of ['push', 'view', 'id', 'permaid']) {
      const id = url.searchParams.get(key);
      if (id) return id;
    }
  } catch {
    // Not a link after all.
  }
  return value;
}

function randomId(): string {
  const letters = 'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789';
  return Array.from(crypto.getRandomValues(new Uint8Array(7)), (b) => letters[b % letters.length]).join('');
}

/** VDO.Ninja's `&hash=`: lets a guest into a room with a password without knowing it. */
async function roomHash(password: string): Promise<string> {
  if (!password || !crypto.subtle) return '';
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(`${password.trim()}vdo.ninja`));
  return [...new Uint8Array(digest)]
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
    .slice(0, 4);
}

/** The link a commentator opens to send their camera to the room. */
function pushLink(room: string, hash: string, source: CameraSource): string {
  const id = vdoId(source.id);
  if (!room || !id) return '';
  let link = `https://vdo.ninja/?room=${encodeURIComponent(vdoId(room))}`;
  if (hash) link += `&hash=${hash}`;
  link += `&push=${encodeURIComponent(id)}`;
  if (source.name.trim()) link += `&label=${encodeURIComponent(source.name.trim())}`;
  return link;
}

const EMPTY_STREAM: StreamCameras = { label: '', left: null, right: null };

/** A stream's OBS source at 1920×1080, scaled to the card. */
function Preview({ url }: { url: string }) {
  const box = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(0.25);
  useEffect(() => {
    const element = box.current;
    if (!element) return;
    const fit = () => setScale(element.clientWidth / 1920);
    fit();
    const observer = new ResizeObserver(fit);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return (
    <div ref={box} className="overlay-preview">
      <iframe title="Prévia do overlay" src={url} style={{ transform: `scale(${scale})` }} allow="autoplay" />
    </div>
  );
}

function StreamCard({
  n,
  stream,
  sources,
  url,
  onChange,
}: {
  n: number;
  stream: StreamCameras;
  sources: CameraSource[];
  url: string | undefined;
  onChange: (stream: StreamCameras) => void;
}) {
  const [preview, setPreview] = useState<'off' | 'live' | 'demo'>('off');
  const choices = sources.filter((s) => vdoId(s.id));
  const picker = (side: 'left' | 'right', label: string) => {
    const other = side === 'left' ? stream.right : stream.left;
    return (
      <div className="field">
        <label htmlFor={`stream-${n}-${side}`}>{label}</label>
        <select
          id={`stream-${n}-${side}`}
          className="input"
          value={stream[side] ?? ''}
          onChange={(e) => onChange({ ...stream, [side]: e.target.value || null })}
        >
          <option value="">Nenhuma</option>
          {choices.map((s) => {
            const id = vdoId(s.id);
            return (
              <option key={id} value={id} disabled={id === other}>
                {s.name.trim() || id}
              </option>
            );
          })}
        </select>
      </div>
    );
  };
  return (
    <section className="card stack" aria-labelledby={`stream-${n}-title`}>
      <h2 id={`stream-${n}-title`} className="small-title">
        Transmissão {n}
        {stream.label.trim() ? ` · ${stream.label.trim()}` : ''}
      </h2>
      <div className="field">
        <label htmlFor={`stream-${n}-label`}>Quem transmite</label>
        <input
          id={`stream-${n}-label`}
          className="input"
          maxLength={40}
          placeholder="Ex.: Farinha"
          value={stream.label}
          onChange={(e) => onChange({ ...stream, label: e.target.value })}
        />
      </div>
      <div className="points-fields">
        {picker('left', 'Câmera da esquerda')}
        {picker('right', 'Câmera da direita')}
      </div>
      <p className="muted small">
        Com só uma câmera ao vivo, ela vai para o meio; sem nenhuma, o quadro da esquerda fica em “aguardando”.
      </p>
      {url && (
        <>
          <OverlayLink title="OBS" url={url} label={`Copiar o link do overlay da transmissão ${n}`} />
          <p className="muted small">Fonte de navegador de 1920 × 1080, sem “Arquivo local”. A arte do fundo vai abaixo dela.</p>
          <div className="form-actions">
            <button
              type="button"
              className="button button-small button-ghost"
              onClick={() => setPreview(preview === 'live' ? 'off' : 'live')}
            >
              {preview === 'live' ? 'Esconder prévia' : 'Prévia'}
            </button>
            <button
              type="button"
              className="button button-small button-ghost"
              title="Caixas coloridas no lugar das câmeras, entrando e saindo, para ver o layout."
              onClick={() => setPreview(preview === 'demo' ? 'off' : 'demo')}
            >
              {preview === 'demo' ? 'Esconder teste' : 'Teste do layout'}
            </button>
          </div>
          {preview !== 'off' && (
            <>
              <Preview key={preview} url={preview === 'demo' ? `${url}?demo=1` : url} />
              <p className="muted small">A prévia mostra o que está salvo.</p>
            </>
          )}
        </>
      )}
    </section>
  );
}

/** The VDO.Ninja room, its cameras, and which ones each stream shows. */
function Cameras({ guildId, links }: { guildId: string; links: OverlayLinks | null }) {
  const [config, setConfig] = useState<CameraConfig | null>(null);
  const [saved, setSaved] = useState('');
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [savedNow, setSavedNow] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  const [expectedHash, setExpectedHash] = useState('');
  const [hash, setHash] = useState('');

  useEffect(() => {
    let cancelled = false;
    setConfig(null);
    setLoadError(null);
    api
      .cameras(guildId)
      .then((loaded) => {
        if (cancelled) return;
        setConfig(loaded);
        setSaved(JSON.stringify(loaded));
      })
      .catch((err: Error) => !cancelled && setLoadError(err.message));
    return () => {
      cancelled = true;
    };
  }, [guildId]);

  const password = config?.password ?? '';
  useEffect(() => {
    let cancelled = false;
    void roomHash(password).then((h) => !cancelled && setHash(h));
    return () => {
      cancelled = true;
    };
  }, [password]);

  if (loadError) return <p className="error">{loadError}</p>;
  if (!config) return <p className="muted">Carregando as câmeras…</p>;

  const changed = JSON.stringify(config) !== saved;
  const update = (patch: Partial<CameraConfig>) => {
    setConfig({ ...config, ...patch });
    setSavedNow(false);
  };
  const setStream = (index: number, stream: StreamCameras) => {
    const streams: [StreamCameras, StreamCameras] = [...config.streams];
    streams[index] = stream;
    update({ streams });
  };
  /** A camera's ID changing, or the camera going, carries over to the streams that show it. */
  const replaceInStreams = (from: string, to: string | null) =>
    config.streams.map((s) => ({
      ...s,
      left: s.left === from ? to : s.left,
      right: s.right === from ? to : s.right,
    })) as [StreamCameras, StreamCameras];
  const setSource = (index: number, source: CameraSource) => {
    const sources = config.sources.map((s, i) => (i === index ? source : s));
    const before = vdoId(config.sources[index].id);
    const after = vdoId(source.id);
    update({ sources, streams: before && before !== after ? replaceInStreams(before, after || null) : config.streams });
  };
  const removeSource = (index: number) => {
    const id = vdoId(config.sources[index].id);
    update({
      sources: config.sources.filter((_, i) => i !== index),
      streams: id ? replaceInStreams(id, null) : config.streams,
    });
  };
  const pasteRoomLink = (text: string) => {
    try {
      const url = new URL(text.trim());
      const room = url.searchParams.get('room') ?? url.searchParams.get('r') ?? url.searchParams.get('director') ?? url.searchParams.get('dir');
      const pw = url.searchParams.get('password') ?? url.searchParams.get('pw') ?? url.searchParams.get('p');
      update({ room: room ?? config.room, password: pw ?? config.password });
      setExpectedHash((url.searchParams.get('hash') ?? '').slice(0, 4).toLowerCase());
    } catch {
      // Still typing.
    }
  };
  const submit = (e: FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    api
      .saveCameras(guildId, config)
      .then((fresh) => {
        setConfig(fresh);
        setSaved(JSON.stringify(fresh));
        setSavedNow(true);
      })
      .catch((err: Error) => setError(err.message))
      .finally(() => setSaving(false));
  };

  return (
    <form className="stack" onSubmit={submit} aria-labelledby="cameras-title">
      <div className="title-row">
        <h2 id="cameras-title" className="section-title">
          Câmeras dos comentaristas
        </h2>
      </div>
      <p className="muted small">
        Cada comentarista manda a câmera para uma sala do VDO.Ninja com o link dele. Cada transmissão tem o seu overlay e
        escolhe quais câmeras mostra. Ao salvar, os overlays abertos no OBS trocam sozinhos em alguns segundos.
      </p>

      <section className="card stack" aria-labelledby="room-title">
        <h2 id="room-title" className="small-title">
          Sala do VDO.Ninja
        </h2>
        <div className="field">
          <label htmlFor="room-link">Colar o link da sala (opcional, preenche os campos)</label>
          <input
            id="room-link"
            className="input mono"
            placeholder="https://vdo.ninja/?room=minha_sala&hash=1a2b"
            onChange={(e) => pasteRoomLink(e.target.value)}
          />
        </div>
        <div className="points-fields">
          <div className="field">
            <label htmlFor="room-name">Nome da sala</label>
            <input
              id="room-name"
              className="input mono"
              autoComplete="off"
              spellCheck={false}
              maxLength={60}
              value={config.room}
              onChange={(e) => update({ room: e.target.value })}
            />
          </div>
          <div className="field">
            <label htmlFor="room-password">Senha da sala</label>
            <div className="input-with-button">
              <input
                id="room-password"
                className="input mono"
                type={showPassword ? 'text' : 'password'}
                autoComplete="off"
                spellCheck={false}
                maxLength={100}
                value={config.password}
                onChange={(e) => update({ password: e.target.value })}
              />
              <button type="button" className="button button-small button-ghost" onClick={() => setShowPassword(!showPassword)}>
                {showPassword ? 'Esconder' : 'Mostrar'}
              </button>
            </div>
            {expectedHash && config.password ? (
              <span className={hash === expectedHash ? 'small ok-text' : 'small error'}>
                {hash === expectedHash
                  ? `A senha confere com o link da sala (hash ${hash}).`
                  : `A senha não confere com o link da sala: ele espera o hash ${expectedHash}, esta senha dá ${hash}.`}
              </span>
            ) : (
              <span className="muted small">Fica nos links dos overlays, então não mostre esses links na live.</span>
            )}
          </div>
        </div>
      </section>

      <section className="card stack" aria-labelledby="sources-title">
        <div className="card-head">
          <h2 id="sources-title" className="small-title">
            Câmeras <span className="mono muted">{config.sources.length}</span>
          </h2>
          <button
            type="button"
            className="button button-small"
            disabled={config.sources.length >= MAX_CAMERAS}
            onClick={() => update({ sources: [...config.sources, { id: randomId(), name: '' }] })}
          >
            Adicionar câmera
          </button>
        </div>
        {config.sources.length === 0 ? (
          <p className="empty">Nenhuma câmera ainda. Adicione uma para cada comentarista.</p>
        ) : (
          <ul className="camera-list" aria-label="Câmeras">
            {config.sources.map((source, index) => {
              const link = pushLink(config.room, hash, source);
              return (
                <li key={index} className="camera-row">
                  <div className="field">
                    <label htmlFor={`camera-${index}-name`}>Nome na tela</label>
                    <input
                      id={`camera-${index}-name`}
                      className="input"
                      maxLength={40}
                      value={source.name}
                      onChange={(e) => setSource(index, { ...source, name: e.target.value })}
                    />
                  </div>
                  <div className="field">
                    <label htmlFor={`camera-${index}-id`}>ID no VDO.Ninja (ou o link do comentarista)</label>
                    <div className="input-with-button">
                      <input
                        id={`camera-${index}-id`}
                        className="input mono"
                        autoComplete="off"
                        spellCheck={false}
                        value={source.id}
                        onChange={(e) => setSource(index, { ...source, id: idFrom(e.target.value) })}
                      />
                      <button
                        type="button"
                        className="button button-small button-ghost"
                        onClick={() => setSource(index, { ...source, id: randomId() })}
                      >
                        Gerar ID
                      </button>
                    </div>
                  </div>
                  <div className="camera-row-end">
                    {link ? (
                      <CopyButton
                        text={link}
                        label={`Copiar o link de ${source.name.trim() || 'quem manda esta câmera'}`}
                        caption="Link do comentarista"
                      />
                    ) : (
                      <span className="muted small" title="Preencha a sala e o ID.">
                        sem link
                      </span>
                    )}
                    <button
                      type="button"
                      className="icon-button icon-button-quiet"
                      aria-label={`Remover a câmera ${source.name.trim() || index + 1}`}
                      onClick={() => removeSource(index)}
                    >
                      ×
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
        <p className="muted small">
          Mande o link do comentarista para ele: abre, permite a câmera e entra na sala. O ID fica fixo, então o mesmo link
          serve para todos os camps.
        </p>
      </section>

      <div className="streams-grid">
        {config.streams.map((stream, index) => (
          <StreamCard
            key={index}
            n={index + 1}
            stream={stream ?? EMPTY_STREAM}
            sources={config.sources}
            url={links?.camera_urls[index]}
            onChange={(s) => setStream(index, s)}
          />
        ))}
      </div>

      {error && (
        <p className="error small" role="alert">
          {error}
        </p>
      )}
      <div className="form-actions sticky-actions">
        <button type="submit" className="button button-primary" disabled={saving || !changed}>
          {saving ? 'Salvando…' : 'Salvar câmeras'}
        </button>
        {changed && (
          <button type="button" className="button button-ghost" disabled={saving} onClick={() => setConfig(JSON.parse(saved))}>
            Descartar
          </button>
        )}
        <span className="muted small" role="status">
          {savedNow ? 'Salvo. Os overlays atualizam em alguns segundos.' : changed ? 'Alterações ainda não salvas.' : ''}
        </span>
      </div>
    </form>
  );
}

/** The server's OBS browser sources: the camp's vote and ranking, and the commentators' cameras per stream. */
export function OverlaysView({ guildId }: { guildId: string }) {
  const [links, setLinks] = useState<OverlayLinks | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLinks(null);
    setError(null);
    api
      .overlays(guildId)
      .then((loaded) => !cancelled && setLinks(loaded))
      .catch((err: Error) => !cancelled && setError(err.message));
    return () => {
      cancelled = true;
    };
  }, [guildId]);

  const reset = () =>
    api
      .resetOverlays(guildId)
      .then(setLinks)
      .catch((err: Error) => setError(err.message));

  return (
    <div className="stack overlays-page">
      <div className="title-row title-row-wrap">
        <h1 className="page-title">Overlays</h1>
        <span className="muted small">
          Fontes de navegador para o OBS: fundo transparente, atualizam sozinhas e não pedem login. Quem tem o link vê o
          overlay.
        </span>
      </div>
      {error && (
        <p className="error small" role="alert">
          {error}
        </p>
      )}
      {links ? <CampOverlays links={links} onReset={reset} /> : !error && <p className="muted small">Carregando…</p>}
      <Cameras guildId={guildId} links={links} />
    </div>
  );
}
