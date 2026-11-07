import { useState } from 'react';

/** The Discord avatar over the name's initial, which shows while it loads or if it can't. */
export function Avatar({ name, url, size }: { name: string; url: string | null; size?: number }) {
  const [failed, setFailed] = useState(false);
  const style = size ? { width: size, height: size, fontSize: Math.round(size * 0.42) } : undefined;
  return (
    <span className="ov-avatar" style={style}>
      {name.charAt(0).toUpperCase()}
      {url && !failed && <img src={url} alt="" onError={() => setFailed(true)} />}
    </span>
  );
}
