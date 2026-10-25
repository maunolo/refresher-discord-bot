import { useEffect, useState } from 'react';

import { useActions } from '../actions';
import type { Camp, Match, MatchPlayer, RecentMatch, Side } from '../api';
import { SIDES, SIDE_LABEL, matchName, playerById, playerName } from '../camp';
import { Captain, SRankChip } from './common';
import { Modal } from './Modal';

const TONE: Record<Side, string> = { hiddenking: 'button-hk', archmother: 'button-am' };

/** The match's players as radio buttons, one team per column, `first` on the left. */
function MvpPicker({
  camp,
  players,
  first,
  value,
  onChange,
}: {
  camp: Camp;
  players: MatchPlayer[];
  first: Side;
  value: string | null;
  onChange: (userId: string) => void;
}) {
  const sides = first === 'hiddenking' ? SIDES : [...SIDES].reverse();
  return (
    <fieldset className="mvp-picker">
      <legend className="eyebrow">MVP da partida</legend>
      <div className="mvp-teams">
        {sides.map((side) => (
          <div key={side} className="mvp-team">
            <span className={`team-title team-${side}`}>
              <span className="swatch" />
              {SIDE_LABEL[side]}
            </span>
            {players
              .filter((p) => p.side === side)
              .sort((a, b) => Number(b.is_captain) - Number(a.is_captain))
              .map((p) => (
                <label key={p.user_id} className={value === p.user_id ? 'mvp-option mvp-option-on' : 'mvp-option'}>
                  <input
                    type="radio"
                    name="mvp"
                    value={p.user_id}
                    checked={value === p.user_id}
                    onChange={() => onChange(p.user_id)}
                  />
                  <span className="ellipsis">{playerName(camp, p.user_id)}</span>
                  {playerById(camp, p.user_id)?.s_rank && <SRankChip />}
                  {p.is_captain && <Captain />}
                </label>
              ))}
          </div>
        ))}
      </div>
    </fieldset>
  );
}

/** Confirms a match's winner and asks for its MVP, which breaks ties in the final ranking. */
export function WinnerDialog({
  camp,
  match,
  side,
  onClose,
}: {
  camp: Camp;
  match: Match;
  side: Side | null;
  onClose: () => void;
}) {
  const { run, busy } = useActions();
  const [mvp, setMvp] = useState<string | null>(null);
  useEffect(() => setMvp(null), [side]);

  const submit = () => {
    if (!side || !mvp) return;
    run({ type: 'report_winner', match_id: match.id, side, mvp_user_id: mvp });
    onClose();
  };

  return (
    <Modal open={side !== null} onClose={onClose} labelledBy="winner-title" describedBy="winner-help" wide>
      {side && (
        <form
          className="stack"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          <h2 id="winner-title">
            {SIDE_LABEL[side]} venceu a {matchName(match)}?
          </h2>
          <p id="winner-help" className="text-2">
            Os vencedores ganham 1 ponto e todos voltam para a fila. Escolha o MVP: no ranking final, quem empatar em pontos
            fica na frente com mais MVPs.
          </p>
          <MvpPicker camp={camp} players={match.players} first={side} value={mvp} onChange={setMvp} />
          <p className="muted small">Se errar, dá para trocar o vencedor ou o MVP desta partida enquanto ela for a última.</p>
          <div className="modal-actions">
            <button type="button" className="button button-ghost" onClick={onClose}>
              Voltar
            </button>
            <button
              type="submit"
              className={`button ${TONE[side]}`}
              disabled={busy || !mvp}
              title={mvp ? undefined : 'Escolha o MVP primeiro.'}
            >
              {SIDE_LABEL[side]} venceu
            </button>
          </div>
        </form>
      )}
    </Modal>
  );
}

/** Changes the MVP of the latest result. */
export function MvpDialog({
  camp,
  match,
  onClose,
}: {
  camp: Camp;
  match: RecentMatch | null;
  onClose: () => void;
}) {
  const { run, busy } = useActions();
  const [mvp, setMvp] = useState<string | null>(null);
  const matchId = match?.id;
  const current = match?.mvp_user_id ?? null;
  useEffect(() => setMvp(current), [matchId, current]);

  const submit = () => {
    if (!match || !mvp || mvp === current) return;
    run(
      { type: 'change_mvp', match_id: match.id, user_id: mvp },
      { success: `MVP da partida #${match.number}: ${playerName(camp, mvp)}. O Discord foi avisado.` },
    );
    onClose();
  };

  return (
    <Modal open={match !== null} onClose={onClose} labelledBy="mvp-title" describedBy="mvp-help" wide>
      {match && (
        <form
          className="stack"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          <h2 id="mvp-title">Trocar o MVP da partida #{match.number}</h2>
          <p id="mvp-help" className="text-2">
            {current
              ? `MVP agora: ${playerName(camp, current)}. Pontos e fila não mudam; só a contagem de MVPs do desempate.`
              : 'Esta partida terminou sem MVP. Escolha um; pontos e fila não mudam.'}
          </p>
          <MvpPicker camp={camp} players={match.players} first={match.winner_side} value={mvp} onChange={setMvp} />
          <div className="modal-actions">
            <button type="button" className="button button-ghost" onClick={onClose}>
              Voltar
            </button>
            <button type="submit" className="button button-primary" disabled={busy || !mvp || mvp === current}>
              Salvar MVP
            </button>
          </div>
        </form>
      )}
    </Modal>
  );
}
