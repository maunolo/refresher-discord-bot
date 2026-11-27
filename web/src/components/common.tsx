import { useState, type ReactNode } from 'react';

import type { MatchPoints, Player, Role } from '../api';
import { rankOf, ROLE_LABEL, roleShort } from '../camp';
import { ArrowRightIcon, CheckIcon, CopyIcon, CrownIcon, ExternalIcon, StarIcon } from './Icons';

/** What a match gives each player, for a win and for a loss. */
export function MatchWorth({ points }: { points: MatchPoints }) {
  return (
    <div className="inset worth" title="Pontos de cada jogador quando a partida termina">
      <span className="eyebrow">Vale</span>
      <strong className="mono">+{points.win}</strong>
      <span className="muted small">vitória</span>
      <span className="faint" aria-hidden="true">
        ·
      </span>
      <strong className="mono">+{points.loss}</strong>
      <span className="muted small">derrota</span>
    </div>
  );
}

/** An admin took the player out of the camp. */
export function RemovedChip() {
  return (
    <span className="chip chip-removed" title="Removido do camp: não entra na fila nem em partidas novas">
      Removido
    </span>
  );
}

/** The player's category: SS rank (captains) or S rank. Nothing without one. */
export function RankChip({ player }: { player: Pick<Player, 's_rank' | 'ss_rank'> | undefined }) {
  const rank = player ? rankOf(player) : null;
  if (rank === 'ss') {
    return (
      <span className="chip chip-ss" title="SS rank: capitão do time">
        SS
      </span>
    );
  }
  if (rank === 's') {
    return (
      <span className="chip chip-s" title="S rank">
        S
      </span>
    );
  }
  return null;
}

/** A player's main role, short, with the full name on hover. Nothing without a role. */
export function RoleChip({ role }: { role: Role | null }) {
  if (role === null) return null;
  return (
    <span className="chip chip-role" title={ROLE_LABEL[role]}>
      {roleShort(role)}
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

/** Copies `text`. An icon, or with `caption` a small button that says what it copies. */
export function CopyButton({ text, label, caption }: { text: string; label: string; caption?: string }) {
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
  if (caption) {
    return (
      <button type="button" className="button button-small button-ghost" aria-label={label} title={label} onClick={copy}>
        {copied ? <CheckIcon size={14} /> : <CopyIcon size={14} />}
        {copied ? 'Copiado' : caption}
      </button>
    );
  }
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
        <img src="/logo-dragon.webp" alt="" width={72} height={72} />
        <h1>{title}</h1>
        {children}
      </div>
    </div>
  );
}
