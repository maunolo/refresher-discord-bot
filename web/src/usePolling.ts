import { useCallback, useEffect, useRef, useState } from 'react';

interface Loaded<T> {
  key: string | null;
  data: T | null;
  /** `data` as JSON: a load that brings the same thing doesn't re-render the page. */
  json: string | null;
  error: Error | null;
}

/**
 * Loads `load(key)` now, again every `everyMs` while the tab is visible, and right away when the tab comes back
 * or `signal` changes (a live update said the data changed).
 * A new key starts over; the last good data stays on screen if a later load fails.
 * Only the latest load lands: one overtaken by another load or by `replace` can't put older data back.
 * `load` must be a stable function (defined outside the component).
 * `replace` swaps in fresher data, e.g. what an action returned; `refresh` loads again now.
 */
export function usePolling<T>(
  key: string | null,
  load: (key: string) => Promise<T>,
  everyMs: number,
  signal: unknown = null,
): { data: T | null; error: Error | null; replace: (data: T) => void; refresh: () => void } {
  const [state, setState] = useState<Loaded<T>>({ key: null, data: null, json: null, error: null });
  const [nonce, setNonce] = useState(0);
  /** Taken by every load and by `replace`; a load lands only if nothing took a later one. */
  const latest = useRef(0);
  const replace = useCallback((data: T) => {
    latest.current += 1;
    setState((s) => ({ key: s.key, data, json: JSON.stringify(data), error: null }));
  }, []);
  const refresh = useCallback(() => setNonce((n) => n + 1), []);

  useEffect(() => {
    if (key === null) return;
    let cancelled = false;
    let timer: number | undefined;
    const tick = async () => {
      window.clearTimeout(timer);
      if (!document.hidden) {
        const ticket = ++latest.current;
        try {
          const data = await load(key);
          const json = JSON.stringify(data);
          if (!cancelled && ticket === latest.current) {
            setState((s) =>
              s.key === key && s.error === null && s.json === json ? s : { key, data, json, error: null },
            );
          }
        } catch (error) {
          if (!cancelled && ticket === latest.current) {
            setState((s) =>
              s.key === key ? { ...s, error: error as Error } : { key, data: null, json: null, error: error as Error },
            );
          }
        }
      }
      if (!cancelled) {
        window.clearTimeout(timer);
        timer = window.setTimeout(tick, everyMs);
      }
    };
    const onVisible = () => {
      if (!document.hidden) void tick();
    };
    document.addEventListener('visibilitychange', onVisible);
    void tick();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
      document.removeEventListener('visibilitychange', onVisible);
    };
  }, [key, load, everyMs, nonce, signal]);

  return state.key === key
    ? { data: state.data, error: state.error, replace, refresh }
    : { data: null, error: null, replace, refresh };
}

/** A value kept in localStorage, for remembering the chosen server and camp. */
export function remembered(name: string): string | null {
  try {
    return window.localStorage.getItem(`refresher.${name}`);
  } catch {
    return null;
  }
}

export function remember(name: string, value: string): void {
  try {
    window.localStorage.setItem(`refresher.${name}`, value);
  } catch {
    // Private windows can refuse storage; the choice just isn't kept.
  }
}
