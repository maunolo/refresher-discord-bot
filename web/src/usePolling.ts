import { useCallback, useEffect, useState } from 'react';

interface Loaded<T> {
  key: string | null;
  data: T | null;
  error: Error | null;
}

/**
 * Loads `load(key)` now and again every `everyMs` while the tab is visible.
 * A new key starts over; the last good data stays on screen if a later load fails.
 * `load` must be a stable function (defined outside the component).
 * `replace` swaps in fresher data, e.g. what an action returned; `refresh` loads again now.
 */
export function usePolling<T>(
  key: string | null,
  load: (key: string) => Promise<T>,
  everyMs: number,
): { data: T | null; error: Error | null; replace: (data: T) => void; refresh: () => void } {
  const [state, setState] = useState<Loaded<T>>({ key: null, data: null, error: null });
  const [nonce, setNonce] = useState(0);
  const replace = useCallback((data: T) => setState((s) => ({ key: s.key, data, error: null })), []);
  const refresh = useCallback(() => setNonce((n) => n + 1), []);

  useEffect(() => {
    if (key === null) return;
    let cancelled = false;
    let timer: number | undefined;
    const tick = async () => {
      if (!document.hidden) {
        try {
          const data = await load(key);
          if (!cancelled) setState({ key, data, error: null });
        } catch (error) {
          if (!cancelled) {
            setState((s) => ({ key, data: s.key === key ? s.data : null, error: error as Error }));
          }
        }
      }
      if (!cancelled) timer = window.setTimeout(tick, everyMs);
    };
    void tick();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [key, load, everyMs, nonce]);

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
