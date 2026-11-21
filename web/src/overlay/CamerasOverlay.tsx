import { useCallback, useEffect, useReducer, useRef, useState } from 'react';

import { useCameras, type OverlayCamera } from './data';

/**
 * The commentators' cameras for one stream, from VDO.Ninja, in Haunter's gold frame
 * (overlay/comentaristas.html): two side by side, or one in the middle while only one is live.
 * 1920×1080 with a transparent background, so the scene's art goes below it in OBS.
 *
 * `?debug=1` logs VDO.Ninja's events on screen; `?demo=1` shows colored boxes that come and go
 * instead of the cameras. Keys (OBS: right-click the source, Interact): 1 or 2 force that many
 * cameras, A goes back to automatic, D toggles the log.
 */

const WIDTH = 1920;
/** The whole frame, in rebuttal.png's proportions (3595×2075). */
const FRAME = { w: 880, h: 508 };
const GAP = 60;
const CENTER_Y = 350;
/** A camera has to stay disconnected this long before it leaves, so a blip doesn't flash. */
const LEAVE_DELAY_MS = 2500;
const DEMO_LEAVE_DELAY_MS = 300;

// VDO.Ninja's iframe events; versions name them differently, so all of these count.
const CONNECTED = new Set(['guest-connected', 'view-connection', 'push-connection', 'new-view-connection', 'video-element-created']);
const LEFT = new Set(['guest-disconnected', 'end-view-connection', 'view-disconnected', 'push-connection-ended', 'video-element-removed']);

const DEMO_CAMERAS: OverlayCamera[] = [
  { id: 'demo1', name: 'Câmera 1' },
  { id: 'demo2', name: 'Câmera 2' },
];
const DEMO_COLORS = ['linear-gradient(135deg,#1d6b57,#0b2c25)', 'linear-gradient(135deg,#2d4f86,#121f3a)'];

interface Box {
  x: number;
  y: number;
}

/** Where `count` cameras sit, left to right. */
function boxes(count: number): Box[] {
  const y = CENTER_Y - FRAME.h / 2;
  if (count === 1) return [{ x: (WIDTH - FRAME.w) / 2, y }];
  const x0 = (WIDTH - (2 * FRAME.w + GAP)) / 2;
  return [
    { x: x0, y },
    { x: x0 + FRAME.w + GAP, y },
  ];
}

/** A camera on its own, as a VDO.Ninja scene: no menus or cursor, filling the frame. */
function viewUrl(room: string, password: string, id: string): string {
  let url = 'https://vdo.ninja/?scene';
  if (room) url += `&room=${encodeURIComponent(room)}`;
  url += `&view=${encodeURIComponent(id)}`;
  if (password) url += `&password=${encodeURIComponent(password)}`;
  return `${url}&cleanoutput&hidemenu&cover&nocursor`;
}

export function CamerasOverlay({
  token,
  stream,
  debug: debugAtStart,
  demo,
}: {
  token: string;
  stream: number;
  debug: boolean;
  demo: boolean;
}) {
  const view = useCameras(token, stream);
  const room = view?.room ?? '';
  const password = view?.password ?? '';
  const cameras = demo && view && view.cameras.length === 0 ? DEMO_CAMERAS : (view?.cameras ?? []);

  const [, rerender] = useReducer((n: number) => n + 1, 0);
  const [forced, setForced] = useState<1 | 2 | null>(null);
  const [debug, setDebug] = useState(debugAtStart);
  const [lines, setLines] = useState<string[]>([]);
  const live = useRef(new Map<string, boolean>());
  const leaving = useRef(new Map<string, number>());
  const frames = useRef(new Map<string, HTMLIFrameElement>());
  const placed = useRef(new Map<string, Box>());
  const debugOn = useRef(debug);
  debugOn.current = debug;

  const note = useCallback((line: string) => {
    if (!debugOn.current) return;
    setLines((old) => [`${new Date().toLocaleTimeString()}  ${line}`, ...old].slice(0, 40));
  }, []);

  /** Live right away; gone only once it stays disconnected for a moment. */
  const mark = useCallback(
    (id: string, on: boolean, why: string) => {
      const timers = leaving.current;
      const wasLive = live.current.get(id) ?? false;
      if (on) {
        window.clearTimeout(timers.get(id));
        timers.delete(id);
        if (!wasLive) {
          live.current.set(id, true);
          note(`${id} conectou (${why})`);
          rerender();
        }
      } else if (wasLive && !timers.has(id)) {
        const delay = demo ? DEMO_LEAVE_DELAY_MS : LEAVE_DELAY_MS;
        timers.set(
          id,
          window.setTimeout(() => {
            timers.delete(id);
            live.current.set(id, false);
            note(`${id} saiu (${why})`);
            rerender();
          }, delay),
        );
      }
    },
    [demo, note],
  );

  // A new room or password reloads every camera, so none is live until it says so again.
  useEffect(() => {
    live.current.clear();
    for (const timer of leaving.current.values()) window.clearTimeout(timer);
    leaving.current.clear();
    rerender();
  }, [room, password]);

  // What the cameras' VDO.Ninja frames say, and a check every 3 seconds in case an event is lost.
  useEffect(() => {
    if (demo) return;
    const onMessage = (event: MessageEvent) => {
      let id: string | undefined;
      for (const [key, frame] of frames.current) if (frame.contentWindow === event.source) id = key;
      if (!id || !event.data || typeof event.data !== 'object') return;
      const data = event.data as Record<string, unknown>;
      const action = typeof data.action === 'string' ? data.action : undefined;
      if (action && action !== 'loudness' && action !== 'view-stats-updated' && !('stats' in data)) {
        note(`${id} <- ${JSON.stringify(data).slice(0, 160)}`);
      }
      if (action) {
        if (LEFT.has(action) || (CONNECTED.has(action) && data.value === false)) mark(id, false, action);
        else if (CONNECTED.has(action)) mark(id, true, action);
      }
      // The answer to { getStreamIDs: true }: the IDs connected in that frame.
      if (data.streamIDs && typeof data.streamIDs === 'object') {
        const ids = Array.isArray(data.streamIDs) ? data.streamIDs : Object.keys(data.streamIDs);
        mark(id, ids.length > 0, 'getStreamIDs');
      }
    };
    window.addEventListener('message', onMessage);
    const poll = window.setInterval(() => {
      for (const frame of frames.current.values()) {
        try {
          frame.contentWindow?.postMessage({ getStreamIDs: true }, '*');
        } catch {
          // A frame still loading.
        }
      }
    }, 3000);
    return () => {
      window.removeEventListener('message', onMessage);
      window.clearInterval(poll);
    };
  }, [demo, mark, note]);

  // Demo: the cameras connect and leave in turn.
  const demoIds = cameras.map((c) => c.id).join(',');
  useEffect(() => {
    if (!demo || !demoIds) return;
    const ids = demoIds.split(',');
    const steps: [number, boolean][] = [
      [0, true],
      [1, true],
      [1, false],
      [0, false],
      [1, true],
      [0, true],
      [0, false],
      [1, false],
    ];
    let n = 0;
    const timer = window.setInterval(() => {
      const [index, on] = steps[n++ % steps.length];
      if (ids[index]) mark(ids[index], on, 'demo');
    }, 2600);
    return () => window.clearInterval(timer);
  }, [demo, demoIds, mark]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      if (key === 'd') setDebug((on) => !on);
      else if (key === '1') setForced(1);
      else if (key === '2') setForced(2);
      else if (key === 'a') setForced(null);
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, []);

  const isLive = (c: OverlayCamera) => live.current.get(c.id) ?? false;
  const liveCameras = cameras.filter(isLive);
  let visible: OverlayCamera[];
  if (forced === 1) visible = [liveCameras[0] ?? cameras[0]].filter(Boolean);
  else if (forced === 2) visible = cameras;
  // Nobody live: the first camera's frame, waiting.
  else visible = liveCameras.length > 0 ? liveCameras : cameras.slice(0, 1);
  const shown = boxes(visible.length);
  const home = boxes(2);

  return (
    <div className="cams-stage">
      {cameras.map((camera, index) => {
        const slot = visible.indexOf(camera);
        const on = slot >= 0;
        // A hidden camera stays where it was, shrinking in place; one never shown waits on its side.
        const box = on ? shown[slot] : (placed.current.get(camera.id) ?? home[Math.min(index, 1)]);
        if (on) placed.current.set(camera.id, box);
        return (
          <div
            key={camera.id}
            className={['cam', on && 'on', isLive(camera) && 'live'].filter(Boolean).join(' ')}
            style={{ left: box.x, top: box.y, width: FRAME.w, height: FRAME.h }}
          >
            <div className="cam-frame">
              <div className="cam-area">
                {demo ? (
                  <div className="cam-demo" style={{ background: DEMO_COLORS[index % DEMO_COLORS.length] }}>
                    {camera.name || camera.id}
                  </div>
                ) : (
                  <iframe
                    key={`${room}|${password}|${camera.id}`}
                    ref={(frame) => {
                      if (frame) frames.current.set(camera.id, frame);
                      else frames.current.delete(camera.id);
                    }}
                    title={camera.name || camera.id}
                    src={viewUrl(room, password, camera.id)}
                    allow="autoplay;camera *;microphone *;fullscreen;picture-in-picture"
                  />
                )}
                <div className="cam-wait">
                  <div className="cam-pulse" />
                  AGUARDANDO
                </div>
              </div>
            </div>
            <img className="cam-border" src="/cameras/rebuttal.png" alt="" />
            {camera.name && <div className="cam-name">{camera.name}</div>}
          </div>
        );
      })}
      {forced && <div className="cam-mode">MODO FORÇADO: {forced} CAM (A = automático)</div>}
      {debug && (
        <div className="cam-debug">
          {view === null
            ? 'Carregando as câmeras…'
            : cameras.length === 0
              ? `Nenhuma câmera escolhida para a transmissão ${stream}.`
              : lines.join('\n') || 'Esperando eventos do VDO.Ninja…'}
        </div>
      )}
    </div>
  );
}
