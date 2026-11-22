import { useEffect, useRef, useState } from 'react';

import { useReplay, type OverlayPlay } from './data';

/** A replay that started this long ago (with no known length) is old news when the overlay loads. */
const STALE_MS = 15_000;
/** Late by more than this, the clip starts where the other stream is, not from the top. */
const CATCH_UP_MS = 2_000;
const FADE_MS = 400;

/**
 * Stream `stream`'s replay clips, full screen at 1920×1080: when an admin presses play on the
 * Overlays page, the clip fades in, plays with its sound (OBS's browser source allows that) and
 * fades out at its end. Transparent the rest of the time.
 */
export function ReplayOverlay({ token, stream }: { token: string; stream: number }) {
  const data = useReplay(token, stream);
  const playing = data?.playing ?? null;
  // The play on screen, as first loaded: later loads sign a new link for the same play.
  const [current, setCurrent] = useState<OverlayPlay | null>(null);
  const [visible, setVisible] = useState(false);
  const seen = useRef(new Set<string>());
  const video = useRef<HTMLVideoElement>(null);
  /** The current clip began playing, so taking it away fades it out first. */
  const started = useRef(false);

  /** Off the screen: faded out if it was playing, dropped at once if it never started. */
  const finish = () => {
    setVisible(false);
    if (!started.current) setCurrent(null);
  };

  useEffect(() => {
    if (!data) return;
    if (!playing) {
      // Stopped by an admin.
      finish();
      return;
    }
    if (seen.current.has(playing.play_id)) return;
    seen.current.add(playing.play_id);
    const over =
      playing.duration_ms !== null ? playing.elapsed_ms >= playing.duration_ms : playing.elapsed_ms > STALE_MS;
    // A replay that ended before this overlay loaded stays off.
    if (over) return;
    started.current = false;
    setCurrent(playing);
  }, [data, playing]);

  // Once faded out, the video goes away.
  useEffect(() => {
    if (visible || !current || !started.current) return;
    const timer = window.setTimeout(() => {
      video.current?.pause();
      setCurrent(null);
    }, FADE_MS);
    return () => window.clearTimeout(timer);
  }, [visible, current]);

  const start = () => {
    const element = video.current;
    if (!element || !current || started.current) return;
    started.current = true;
    if (current.elapsed_ms > CATCH_UP_MS) element.currentTime = current.elapsed_ms / 1000;
    // OBS plays with sound; a normal browser (the admin page's preview) only allows it muted.
    element.play().catch(() => {
      element.muted = true;
      void element.play();
    });
    setVisible(true);
  };

  return (
    <div className={visible ? 'replay on' : 'replay'}>
      {current && (
        <>
          <video
            key={current.play_id}
            ref={video}
            src={current.url}
            preload="auto"
            playsInline
            onCanPlay={start}
            onEnded={finish}
            onError={finish}
          />
          <div className="replay-tag">
            <strong>Replay</strong>
            <span>{current.title}</span>
          </div>
        </>
      )}
    </div>
  );
}
