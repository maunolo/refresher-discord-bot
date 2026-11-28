import { type Ref, useEffect, useRef, useState } from 'react';

import { useActions } from '../actions';
import type { Camp, MatchPoints } from '../api';
import { freeSlots, SLOTS, STREAM_NAMES } from '../camp';
import { Modal } from './Modal';

/** The most a match can give each player; the bot refuses more. */
const MAX_POINTS = 10;

const PRESETS: { label: string; points: MatchPoints }[] = [
  { label: 'Normal', points: { win: 1, loss: 0 } },
  { label: 'Dobrada', points: { win: 2, loss: 0 } },
  { label: 'Todos pontuam', points: { win: 2, loss: 1 } },
];

const clamp = (n: number) => Math.min(MAX_POINTS, Math.max(0, Math.trunc(n)));

function PointsField({
  id,
  label,
  value,
  onChange,
  inputRef,
}: {
  id: string;
  label: string;
  value: number;
  onChange: (n: number) => void;
  inputRef?: Ref<HTMLInputElement>;
}) {
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <div className="stepper-input">
        <button
          type="button"
          className="button"
          aria-label={`${label}: 1 a menos`}
          disabled={value <= 0}
          onClick={() => onChange(clamp(value - 1))}
        >
          −
        </button>
        <input
          ref={inputRef}
          id={id}
          className="input mono"
          type="number"
          inputMode="numeric"
          min={0}
          max={MAX_POINTS}
          step={1}
          value={value}
          onChange={(e) => onChange(e.target.value === '' ? 0 : clamp(Number(e.target.value)))}
        />
        <button
          type="button"
          className="button"
          aria-label={`${label}: 1 a mais`}
          disabled={value >= MAX_POINTS}
          onClick={() => onChange(clamp(value + 1))}
        >
          +
        </button>
      </div>
    </div>
  );
}

/**
 * Creates a match, from the queue or in the finals from the top of the ranking, with the points it
 * gives each winner and each loser. Starts with what the camp's newest match gave.
 */
export function CreateMatchDialog({ camp, open, onClose }: { camp: Camp; open: boolean; onClose: () => void }) {
  const final = camp.finals;
  const { run, busy } = useActions();
  const [points, setPoints] = useState<MatchPoints>(camp.next_points);
  const free = freeSlots(camp);
  const [slot, setSlot] = useState<number>(free[0] ?? 1);
  const input = useRef<HTMLInputElement>(null);
  // Runs after Modal has opened the dialog (a child's effects run first), so focus can land.
  useEffect(() => {
    if (!open) return;
    setPoints(camp.next_points);
    setSlot(free[0] ?? 1);
    input.current?.select();
    // Only on opening: the camp keeps reloading while the dialog is open.
  }, [open]);

  const worth = `+${points.win} / +${points.loss}`;
  const submit = () => {
    run({ type: 'create_match', win_points: points.win, loss_points: points.loss, slot });
    onClose();
  };

  return (
    <Modal open={open} onClose={onClose} labelledBy="create-title" describedBy="create-help">
      {open && (
        <form
          className="stack"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          <div>
            <h2 id="create-title">{final ? 'Criar partida da final' : 'Criar partida'}</h2>
            <p className="text-2">
              {final
                ? `Os ${camp.match_size} primeiros do ranking jogam, em times equilibrados por pontos.`
                : `Os próximos ${camp.match_size} da fila jogam.`}
            </p>
          </div>
          <fieldset className="field slot-picker">
            <legend className="field-label">Vaga e transmissão</legend>
            <div className="presets">
              {SLOTS.map((s) => {
                const taken = !free.includes(s);
                const on = s === slot;
                return (
                  <button
                    key={s}
                    type="button"
                    className={on ? 'button button-ghost button-small preset-on' : 'button button-ghost button-small'}
                    aria-pressed={on}
                    disabled={taken}
                    title={taken ? `A vaga ${s} já tem uma partida aberta.` : undefined}
                    onClick={() => setSlot(s)}
                  >
                    Vaga {s} · {STREAM_NAMES[s]}
                  </button>
                );
              })}
            </div>
          </fieldset>
          <div className="field">
            <span className="field-label">Valores comuns</span>
            <div className="presets">
              {PRESETS.map((p) => {
                const on = p.points.win === points.win && p.points.loss === points.loss;
                return (
                  <button
                    key={p.label}
                    type="button"
                    className={on ? 'button button-ghost button-small preset-on' : 'button button-ghost button-small'}
                    aria-pressed={on}
                    onClick={() => setPoints(p.points)}
                  >
                    {p.label} · +{p.points.win} / +{p.points.loss}
                  </button>
                );
              })}
            </div>
          </div>
          <div className="points-fields">
            <PointsField
              id="points-win"
              label="Quem vence ganha"
              value={points.win}
              onChange={(win) => setPoints((p) => ({ ...p, win }))}
              inputRef={input}
            />
            <PointsField
              id="points-loss"
              label="Quem perde ganha"
              value={points.loss}
              onChange={(loss) => setPoints((p) => ({ ...p, loss }))}
            />
          </div>
          <p id="create-help" className="muted small">
            De 0 a {MAX_POINTS} pontos por jogador. Aparece na mensagem da partida no Discord e no card dela. A
            próxima partida já começa com estes valores.
          </p>
          <div className="modal-actions">
            <button type="button" className="button button-ghost" onClick={onClose}>
              Voltar
            </button>
            <button type="submit" className="button button-primary" disabled={busy}>
              {final ? 'Criar partida da final' : 'Criar partida'} na vaga {slot} · {worth}
            </button>
          </div>
        </form>
      )}
    </Modal>
  );
}
