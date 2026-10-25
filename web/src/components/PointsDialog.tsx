import { useEffect, useRef, useState } from 'react';

import { useActions } from '../actions';
import type { Camp, Player } from '../api';
import { playerName, points, signed } from '../camp';
import { Modal } from './Modal';

/** The most one change can add or take; the bot refuses more. */
const MAX_CHANGE = 50;

/**
 * Adds or takes points from one player by hand. The bot announces it in the camp's channel,
 * and it counts for the ranking and the final match like any other point.
 */
export function PointsDialog({ camp, player: picked, onClose }: { camp: Camp; player: Player | null; onClose: () => void }) {
  const { run, busy } = useActions();
  // The latest numbers: the camp keeps reloading while the dialog is open.
  const player = picked && (camp.players.find((p) => p.user_id === picked.user_id) ?? picked);
  const [delta, setDelta] = useState(1);
  const input = useRef<HTMLInputElement>(null);
  const userId = player?.user_id;
  // Runs after Modal has opened the dialog (a child's effects run first), so focus can land.
  useEffect(() => {
    setDelta(1);
    if (userId) input.current?.select();
  }, [userId]);

  const name = player ? playerName(camp, player.user_id) : '';
  const total = (player?.points ?? 0) + delta;
  const problem =
    delta === 0
      ? 'Escolha quantos pontos somar ou tirar.'
      : Math.abs(delta) > MAX_CHANGE
        ? `No máximo ${MAX_CHANGE} pontos por ajuste.`
        : total < 0
          ? `${name} tem ${points(player?.points ?? 0)}; os pontos não ficam negativos.`
          : null;
  const apply = () => {
    if (!player || problem) return;
    run(
      { type: 'adjust_points', user_id: player.user_id, delta },
      { success: `Pontos de ${name}: ${signed(delta)}. O ajuste foi anunciado no Discord.` },
    );
    onClose();
  };

  return (
    <Modal open={player !== null} onClose={onClose} labelledBy="points-title" describedBy="points-help">
      {player && (
        <form
          className="stack"
          onSubmit={(e) => {
            e.preventDefault();
            apply();
          }}
        >
          <div>
            <h2 id="points-title">Ajustar pontos</h2>
            <p className="text-2">
              {name} tem <strong>{points(player.points)}</strong>
              {player.points_adjusted !== 0 && (
                <span className="muted"> ({signed(player.points_adjusted)} em ajustes manuais)</span>
              )}
              .
            </p>
          </div>
          <div className="field">
            <label htmlFor="points-delta">Pontos a somar (negativo tira)</label>
            <div className="stepper-input">
              <button
                type="button"
                className="button"
                aria-label="Tirar 1 ponto"
                onClick={() => setDelta((d) => d - 1)}
              >
                −
              </button>
              <input
                ref={input}
                id="points-delta"
                className="input mono"
                type="number"
                inputMode="numeric"
                min={-MAX_CHANGE}
                max={MAX_CHANGE}
                step={1}
                value={delta}
                onChange={(e) => setDelta(e.target.value === '' ? 0 : Math.trunc(Number(e.target.value)))}
              />
              <button type="button" className="button" aria-label="Somar 1 ponto" onClick={() => setDelta((d) => d + 1)}>
                +
              </button>
            </div>
          </div>
          <p role="status" className={problem ? 'error small' : 'small'}>
            {problem ?? (
              <>
                Fica com <strong>{points(total)}</strong> ({signed(delta)}).
              </>
            )}
          </p>
          <p id="points-help" className="muted small">
            O bot anuncia o ajuste no canal do camp. Conta no ranking e na escolha da partida final.
          </p>
          <div className="modal-actions">
            <button type="button" className="button button-ghost" onClick={onClose}>
              Voltar
            </button>
            <button type="submit" className="button button-primary" disabled={busy || problem !== null}>
              Aplicar ajuste
            </button>
          </div>
        </form>
      )}
    </Modal>
  );
}
