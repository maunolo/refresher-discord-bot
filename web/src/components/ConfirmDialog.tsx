import { useEffect, useRef } from 'react';

import type { Confirm } from '../actions';

const TONE_CLASS: Record<NonNullable<Confirm['tone']>, string> = {
  primary: 'button-primary',
  danger: 'button-danger-solid',
  hiddenking: 'button-hk',
  archmother: 'button-am',
};

/**
 * Asks before an action, in the page's own style. A native modal dialog: focus starts on
 * Voltar and stays inside, Esc or a click outside cancels, and focus goes back to the button.
 */
export function ConfirmDialog({ ask, onAnswer }: { ask: Confirm | null; onAnswer: (ok: boolean) => void }) {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    if (ask && !dialog.open) dialog.showModal();
    if (!ask && dialog.open) dialog.close();
  }, [ask]);

  return (
    <dialog
      ref={ref}
      className="modal"
      aria-labelledby="modal-title"
      aria-describedby="modal-message"
      onCancel={(e) => {
        e.preventDefault();
        onAnswer(false);
      }}
      onClick={(e) => {
        // The dialog itself only receives clicks on the backdrop; its content is the inner box.
        if (e.target === e.currentTarget) onAnswer(false);
      }}
    >
      {ask && (
        <div className="modal-box">
          <h2 id="modal-title">{ask.title}</h2>
          <p id="modal-message" className="text-2">
            {ask.message}
          </p>
          {ask.note && <p className="muted small">{ask.note}</p>}
          <div className="modal-actions">
            <button type="button" className="button button-ghost" autoFocus onClick={() => onAnswer(false)}>
              Voltar
            </button>
            <button type="button" className={`button ${TONE_CLASS[ask.tone ?? 'primary']}`} onClick={() => onAnswer(true)}>
              {ask.action}
            </button>
          </div>
        </div>
      )}
    </dialog>
  );
}
