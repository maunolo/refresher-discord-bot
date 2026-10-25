import { useState, type ReactNode } from 'react';

import { ArrowRightIcon, CheckIcon, CopyIcon, CrownIcon, ExternalIcon, StarIcon } from './Icons';

export function SRankChip() {
  return (
    <span className="chip chip-s" title="S rank">
      S
    </span>
  );
}

/** Marks a match's MVP. */
export function MvpTag() {
  return (
    <span className="mvp-tag" title="MVP da partida">
      <StarIcon size={13} />
      MVP
    </span>
  );
}

export function Captain() {
  return (
    <span className="captain">
      <CrownIcon size={16} label="Capitão" />
    </span>
  );
}

export function CopyButton({ text, label }: { text: string; label: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard can be blocked (http, permissions); the link is still on screen.
    }
  };
  return (
    <button type="button" className="icon-button" aria-label={label} title={copied ? 'Copiado' : label} onClick={copy}>
      {copied ? <CheckIcon size={15} /> : <CopyIcon size={15} />}
    </button>
  );
}

/** From a summary card on Ao vivo to the page with all of it. */
export function CardLink({ href, label }: { href: string; label: string }) {
  return (
    <a className="card-link" href={href}>
      {label}
      <ArrowRightIcon size={14} />
    </a>
  );
}

/** Opens a camp or match message in Discord (the app if installed, else the browser). */
export function DiscordLink({ url, label = 'Abrir no Discord' }: { url: string; label?: string }) {
  return (
    <a className="discord-link" href={url} target="_blank" rel="noreferrer">
      {label}
      <ExternalIcon size={14} />
    </a>
  );
}

export function FullPage({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <div className="full-page">
      <div className="full-page-card">
        <img src="/logo.png" alt="" width={72} height={72} />
        <h1>{title}</h1>
        {children}
      </div>
    </div>
  );
}
