import { useState } from 'react';

import { confirmCloseVote, confirmRefreshTeams, useActions } from '../actions';
import type { Camp, Draft, Match, Side } from '../api';
import {
  SIDES,
  SIDE_LABEL,
  draftStarted,
  matchName,
  playerById,
  playerName,
  points,
  stage,
  voteSummary,
  type Stage,
} from '../camp';
import { CardLink, Captain, CopyButton, DiscordLink, SRankChip } from './common';
import { WinnerDialog } from './ResultDialogs';
import { OBS_OVERLAY_CSS } from '../obs';
import { matchHref } from '../route';

const STEPS = ['Votação', 'Draft', 'Ao vivo', 'Resultado'];
const STEP_INDEX: Record<Stage, number> = { voting: 0, draft: 1, live: 2 };

export function StagePill({ match }: { match: Match }) {
  if (match.status === 'completed') return <span className="pill pill-muted">Encerrada</span>;
  const current = stage(match);
  if (current === 'live') {
    return (
      <span className="pill pill-live">
        <span className="dot" />
        Ao vivo
      </span>
    );
  }
  if (current === 'draft') {
    const label = !draftStarted(match) ? 'Pronta para o draft' : match.draft_manual ? 'Draft manual' : 'Draft em andamento';
    return <span className="pill pill-draft">{label}</span>;
  }
  return <span className="pill pill-running">Votação aberta</span>;
}

export function Stepper({ match }: { match: Match }) {
  const current = match.status === 'completed' ? STEPS.length : STEP_INDEX[stage(match)];
  return (
    <ol className="stepper" aria-label="Etapas da partida">
      {STEPS.map((label, i) => (
        <li
          key={label}
          className={i < current ? 'step step-done' : i === current ? 'step step-current' : 'step'}
          aria-current={i === current ? 'step' : undefined}
        >
          <span className="step-bar" />
          <span className="step-label">{label}</span>
        </li>
      ))}
    </ol>
  );
}

function Team({ camp, match, side }: { camp: Camp; match: Match; side: Side }) {
  const team = match.players
    .filter((p) => p.side === side)
    .sort((a, b) => Number(b.is_captain) - Number(a.is_captain));
  return (
    <div className="team">
      <h3 className={`team-title team-${side}`}>
        <span className="swatch" />
        {SIDE_LABEL[side]}
      </h3>
      <ul className="rows">
        {team.map((p) => {
          const player = playerById(camp, p.user_id);
          return (
            <li key={p.user_id} className="row">
              <span className="ellipsis">{playerName(camp, p.user_id)}</span>
              {player?.s_rank && <SRankChip />}
              {p.is_captain && <Captain />}
              <span className="row-end mono muted small">{points(player?.points ?? 0)}</span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/** A team's captains by name: all its S-rank players, or its top scorer. */
function captainNames(camp: Camp, match: Match, side: Side): string {
  return match.players
    .filter((p) => p.side === side && p.is_captain)
    .map((p) => playerName(camp, p.user_id))
    .join(', ');
}

/** A draft started without the draft tool: who should get the link an admin shares. */
export function ManualDraftNote({ camp, match }: { camp: Camp; match: Match }) {
  const teams = SIDES.map((side) => ({ side, names: captainNames(camp, match, side) })).filter((t) => t.names);
  return (
    <div className="inset">
      <span className="eyebrow">Draft manual</span>
      <span className="small text-2">
        Mande o link do draft para os capitães
        {teams.length > 0 && `: ${teams.map((t) => `${t.names} (${SIDE_LABEL[t.side]})`).join(' e ')}`}.
      </span>
    </div>
  );
}

const OBS_CSS_HELP = 'Cole no CSS personalizado da fonte de navegador (1920×1080)';

export function DraftLinks({ camp, match, draft }: { camp: Camp; match: Match; draft: Draft }) {
  const captains = SIDES.map((side) => ({ side, names: captainNames(camp, match, side) }))
    .filter((t) => t.names)
    .map((t) => `${t.names} (${SIDE_LABEL[t.side]})`)
    .join(', ');
  return (
    <div className="inset draft-links">
      <span className="eyebrow">Draft</span>
      <div className="link-row">
        <span className="muted small">Espectador</span>
        <a href={draft.spectator_url} target="_blank" rel="noreferrer" className="mono small ellipsis">
          {draft.spectator_url.replace(/^https?:\/\//, '')}
        </a>
        <CopyButton text={draft.spectator_url} label="Copiar link do espectador" />
      </div>
      {draft.obs_url && (
        <>
          <div className="link-row">
            <span className="muted small">OBS</span>
            <a href={draft.obs_url} target="_blank" rel="noreferrer" className="mono small ellipsis" title="Overlay do draft para o OBS">
              {draft.obs_url.replace(/^https?:\/\//, '')}
            </a>
            <CopyButton text={draft.obs_url} label="Copiar link do overlay do OBS" />
          </div>
          <div className="link-row">
            <span className="muted small">CSS do OBS</span>
            <span className="small ellipsis" title={OBS_CSS_HELP}>
              {OBS_CSS_HELP}
            </span>
            <CopyButton text={OBS_OVERLAY_CSS} label="Copiar CSS do OBS" />
          </div>
        </>
      )}
      {draft.join_url && (
        <div className="link-row">
          <span className="muted small">Capitães</span>
          <span className="small ellipsis" title={captains || undefined}>
            {captains ? `link de ${captains}` : 'link dos capitães'}
          </span>
          <CopyButton text={draft.join_url} label="Copiar link dos capitães" />
        </div>
      )}
    </div>
  );
}

function Actions({ camp, match }: { camp: Camp; match: Match }) {
  const { run, busy } = useActions();
  const [winner, setWinner] = useState<Side | null>(null);
  const current = stage(match);
  if (current === 'live') {
    return (
      <div className="actions two">
        <button type="button" className="button button-hk" disabled={busy} onClick={() => setWinner('hiddenking')}>
          Hiddenking venceu
        </button>
        <button type="button" className="button button-am" disabled={busy} onClick={() => setWinner('archmother')}>
          Archmother venceu
        </button>
        <WinnerDialog camp={camp} match={match} side={winner} onClose={() => setWinner(null)} />
      </div>
    );
  }
  if (current === 'voting') {
    return (
      <div className="actions">
        <button type="button" className="button button-primary" disabled title="Espere a votação terminar no Discord.">
          Iniciar draft
        </button>
        <div className="actions two">
          <button
            type="button"
            className="button"
            disabled={busy}
            title="Encerra a votação agora sem mudar os times."
            onClick={() => run({ type: 'close_vote', match_id: match.id }, { confirm: confirmCloseVote(match) })}
          >
            Manter times
          </button>
          <button
            type="button"
            className="button"
            disabled={busy}
            title="Encerra a votação agora e sorteia os times de novo."
            onClick={() => run({ type: 'refresh_teams', match_id: match.id }, { confirm: confirmRefreshTeams(match) })}
          >
            Sortear de novo
          </button>
        </div>
        <span className="muted small">Encerrar a votação agora, sem esperar os votos.</span>
      </div>
    );
  }
  return (
    <div className="actions">
      {draftStarted(match) ? (
        <button
          type="button"
          className="button button-primary"
          disabled={busy}
          onClick={() => run({ type: 'start_match', match_id: match.id })}
        >
          Iniciar partida
        </button>
      ) : (
        <button
          type="button"
          className="button button-primary"
          disabled={busy}
          title={camp.draft_tool ? undefined : 'Sem a ferramenta de draft: o bot move os jogadores e você manda o link.'}
          onClick={() => run({ type: 'start_draft', match_id: match.id })}
        >
          {camp.draft_tool ? 'Iniciar draft' : 'Iniciar draft manual'}
        </button>
      )}
    </div>
  );
}

export function MatchCard({ camp, match }: { camp: Camp; match: Match }) {
  const titleId = `match-${match.id}`;
  return (
    <section className="card match" aria-labelledby={titleId}>
      <div className="card-head">
        <div className="title-row">
          <h2 id={titleId}>
            <a className="title-link" href={matchHref(match.id)}>
              {matchName(match)}
            </a>
          </h2>
          <span className="mono muted small">{match.id}</span>
        </div>
        <div className="inline-actions">
          <StagePill match={match} />
          <CardLink href={matchHref(match.id)} label="Detalhes" />
        </div>
      </div>
      <Stepper match={match} />
      <p className="muted small">{voteSummary(camp, match)}</p>
      <div className="teams">
        {SIDES.map((side) => (
          <Team key={side} camp={camp} match={match} side={side} />
        ))}
      </div>
      {match.draft && <DraftLinks camp={camp} match={match} draft={match.draft} />}
      {match.draft_manual && <ManualDraftNote camp={camp} match={match} />}
      {match.party_code && (
        <div className="inset link-row">
          <span className="eyebrow">Código da party</span>
          <span className="mono">{match.party_code}</span>
          <CopyButton text={match.party_code} label="Copiar código da party" />
        </div>
      )}
      <Actions camp={camp} match={match} />
      <DiscordLink url={match.discord_url} label="Abrir a mensagem no Discord" />
    </section>
  );
}
