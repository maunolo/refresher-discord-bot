import { type Ref, useEffect, useRef, useState } from 'react';

import { useActions } from '../actions';
import type { Camp, MatchPoints } from '../api';
import { freeSlots, SLOTS, STREAM_NAMES } from '../camp';
import { Modal } from './Modal';

/** The most a match can give each player; the bot refuses more. */
const MAX_POINTS = 10;

type Choice = 'qualifier' | 'finals' | 'custom';

/** What a match gives each winner and loser, in the qualifiers and in the finals. */
const PRESETS: { choice: Exclude<Choice, 'custom'>; label: string; points: MatchPoints }[] = [
  { choice: 'qualifier', label: 'Classificatórias', points: { win: 2, loss: 1 } },
  { choice: 'finals', label: 'Finais', points: { win: 3, loss: 1 } },
];

const presetPoints = (choice: Choice) => PRESETS.find((p) => p.choice === choice)?.points;

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
 * gives each winner and each loser: the qualifiers' or the finals' (whichever the camp is in), or
 * custom ones.
 */
export function CreateMatchDialog({ camp, open, onClose }: { camp: Camp; open: boolean; onClose: () => void }) {
  const final = camp.finals;
  const { run, busy } = useActions();
  const start: Choice = final ? 'finals' : 'qualifier';
  const [choice, setChoice] = useState<Choice>(start);
  const [custom, setCustom] = useState<MatchPoints>(camp.next_points);
  const points = presetPoints(choice) ?? custom;
  const free = freeSlots(camp);
  const [slot, setSlot] = useState<number>(free[0] ?? 1);
  const input = useRef<HTMLInputElement>(null);
  // Runs after Modal has opened the dialog (a child's effects run first), so focus can land.
  useEffect(() => {
    if (!open) return;
    setChoice(start);
    setCustom(camp.next_points);
    setSlot(free[0] ?? 1);
    // Only on opening: the camp keeps reloading while the dialog is open.
  }, [open]);
  useEffect(() => {
    if (choice === 'custom') input.current?.select();
  }, [choice]);

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
          <fieldset className="field slot-picker">
            <legend className="field-label">Pontos</legend>
            <div className="presets">
              {PRESETS.map((p) => (
                <button
                  key={p.choice}
                  type="button"
                  className={choice === p.choice ? 'button button-ghost button-small preset-on' : 'button button-ghost button-small'}
                  aria-pressed={choice === p.choice}
                  onClick={() => setChoice(p.choice)}
                >
                  {p.label} · +{p.points.win} / +{p.points.loss}
                </button>
              ))}
              <button
                type="button"
                className={choice === 'custom' ? 'button button-ghost button-small preset-on' : 'button button-ghost button-small'}
                aria-pressed={choice === 'custom'}
                onClick={() => {
                  setCustom(points);
                  setChoice('custom');
                }}
              >
                Personalizado
              </button>
            </div>
          </fieldset>
          {choice === 'custom' && (
            <div className="points-fields">
              <PointsField
                id="points-win"
                label="Quem vence ganha"
                value={custom.win}
                onChange={(win) => setCustom((p) => ({ ...p, win }))}
                inputRef={input}
              />
              <PointsField
                id="points-loss"
                label="Quem perde ganha"
                value={custom.loss}
                onChange={(loss) => setCustom((p) => ({ ...p, loss }))}
              />
            </div>
          )}
          <p id="create-help" className="muted small">
            {choice === 'custom'
              ? `De 0 a ${MAX_POINTS} pontos por jogador. `
              : `Cada vencedor ganha ${points.win} e cada perdedor ${points.loss}. `}
            Aparece na mensagem da partida no Discord e no card dela.
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
