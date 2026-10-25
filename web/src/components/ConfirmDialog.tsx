import type { Confirm } from '../actions';
import { Modal } from './Modal';

const TONE_CLASS: Record<NonNullable<Confirm['tone']>, string> = {
  primary: 'button-primary',
  danger: 'button-danger-solid',
  hiddenking: 'button-hk',
  archmother: 'button-am',
};

/** Asks before an action, in the page's own style. Focus starts on Voltar. */
export function ConfirmDialog({ ask, onAnswer }: { ask: Confirm | null; onAnswer: (ok: boolean) => void }) {
  return (
    <Modal open={ask !== null} onClose={() => onAnswer(false)} labelledBy="modal-title" describedBy="modal-message">
      {ask && (
        <>
          <h2 id="modal-title">{ask.title}</h2>
          <p id="modal-message" className="text-2">
            {ask.message}
          </p>
          {ask.note && <p className="muted small">{ask.note}</p>}
          <div className="modal-actions">
            <button type="button" className="button button-ghost" onClick={() => onAnswer(false)}>
              Voltar
            </button>
            <button type="button" className={`button ${TONE_CLASS[ask.tone ?? 'primary']}`} onClick={() => onAnswer(true)}>
              {ask.action}
            </button>
          </div>
        </>
      )}
    </Modal>
  );
}
