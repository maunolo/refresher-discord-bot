import { CopyButton } from './common';

/** A link without its secret, for showing on a page that may be on stream. */
export function masked(url: string): string {
  return url.replace(/^https?:\/\//, '').replace(/\/overlay\/[0-9a-f]+\//, '/overlay/•••/');
}

/** An overlay's link to copy into OBS, shown without its secret. */
export function OverlayLink({ title, url, label }: { title: string; url: string; label: string }) {
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
