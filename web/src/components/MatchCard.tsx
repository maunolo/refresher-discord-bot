import { useActions } from '../actions';
import type { Camp, Draft, Match, Side } from '../api';
import { SIDES, SIDE_LABEL, playerById, playerName, points, stage, voteSummary, type Stage } from '../camp';
import { Captain, CopyButton, SRankChip } from './common';

function matchName(match: Match): string {
  return match.is_final ? 'Partida final' : `Partida ${match.slot}`;
}

const STEPS = ['Votação', 'Draft', 'Ao vivo', 'Resultado'];
const STEP_INDEX: Record<Stage, number> = { voting: 0, draft: 1, live: 2 };

function StagePill({ match }: { match: Match }) {
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
    return <span className="pill pill-draft">{match.draft ? 'Draft em andamento' : 'Pronta para o draft'}</span>;
  }
  return <span className="pill pill-running">Votação aberta</span>;
}

function Stepper({ match }: { match: Match }) {
  const current = STEP_INDEX[stage(match)];
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

function DraftLinks({ camp, match, draft }: { camp: Camp; match: Match; draft: Draft }) {
  const captain = (side: Side) => match.players.find((p) => p.side === side && p.is_captain);
  const teamUrl = (side: Side) => (side === 'hiddenking' ? draft.hiddenking_url : draft.archmother_url);
  return (
    <div className="inset">
      <span className="eyebrow">Draft</span>
      <div className="link-row">
        <span className="muted small">Espectador</span>
        <a href={draft.spectator_url} target="_blank" rel="noreferrer" className="mono small ellipsis">
          {draft.spectator_url.replace(/^https?:\/\//, '')}
        </a>
        <CopyButton text={draft.spectator_url} label="Copiar link do espectador" />
      </div>
      {SIDES.map((side) => {
        const url = teamUrl(side);
        const cap = captain(side);
        if (!url) return null;
        return (
          <div key={side} className="link-row">
            <span className="muted small">{SIDE_LABEL[side]}</span>
            <span className="small ellipsis">{cap ? `link de ${playerName(camp, cap.user_id)}` : 'link do capitão'}</span>
            <CopyButton text={url} label={`Copiar link do capitão (${SIDE_LABEL[side]})`} />
          </div>
        );
      })}
      {draft.admin_url && (
        <a href={draft.admin_url} target="_blank" rel="noreferrer" className="small">
          Página do organizador
        </a>
      )}
    </div>
  );
}

function Actions({ match }: { match: Match }) {
  const { run, busy } = useActions();
  const current = stage(match);
  if (current === 'live') {
    const win = (side: Side) =>
      run(
        { type: 'report_winner', match_id: match.id, side },
        { confirm: `Confirmar: ${SIDE_LABEL[side]} venceu a ${matchName(match)}?` },
      );
    return (
      <div className="actions two">
        <button type="button" className="button button-hk" disabled={busy} onClick={() => win('hiddenking')}>
          Hiddenking venceu
        </button>
        <button type="button" className="button button-am" disabled={busy} onClick={() => win('archmother')}>
          Archmother venceu
        </button>
      </div>
    );
  }
  if (current === 'voting') {
    const missing = match.players.length - match.votes.length;
    return (
      <div className="actions">
        <button type="button" className="button button-primary" disabled title="Espere a votação terminar no Discord.">
          Iniciar draft
        </button>
        <button
          type="button"
          className="button"
          disabled={busy}
          onClick={() =>
            run(
              { type: 'close_vote', match_id: match.id },
              {
                confirm: `Encerrar a votação da ${matchName(match)} agora e manter os times? ${
                  missing === 1 ? 'Falta 1 voto.' : `Faltam ${missing} votos.`
                } Os botões de voto somem do Discord.`,
              },
            )
          }
        >
          Encerrar votação e manter times
        </button>
      </div>
    );
  }
  return (
    <div className="actions">
      {match.draft ? (
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
          onClick={() => run({ type: 'start_draft', match_id: match.id })}
        >
          Iniciar draft
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
          <h2 id={titleId}>{matchName(match)}</h2>
          <span className="mono muted small">{match.id}</span>
        </div>
        <StagePill match={match} />
      </div>
      <Stepper match={match} />
      <p className="muted small">{voteSummary(camp, match)}</p>
      <div className="teams">
        {SIDES.map((side) => (
          <Team key={side} camp={camp} match={match} side={side} />
        ))}
      </div>
      {match.draft && <DraftLinks camp={camp} match={match} draft={match.draft} />}
      {match.party_code && (
        <div className="inset link-row">
          <span className="eyebrow">Código da party</span>
          <span className="mono">{match.party_code}</span>
          <CopyButton text={match.party_code} label="Copiar código da party" />
        </div>
      )}
      <Actions match={match} />
    </section>
  );
}
