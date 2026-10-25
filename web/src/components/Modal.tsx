import { useEffect, useRef, type ReactNode } from 'react';

interface ModalProps {
  open: boolean;
  onClose: () => void;
  labelledBy: string;
  describedBy?: string;
  /** Room for a list, like the MVP picker. */
  wide?: boolean;
  children: ReactNode;
}

/**
 * A native modal dialog in the page's style: focus starts on its first button and stays inside,
 * Esc or a click outside closes it, and focus goes back to what opened it.
 */
export function Modal({ open, onClose, labelledBy, describedBy, wide, children }: ModalProps) {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  }, [open]);

  return (
    <dialog
      ref={ref}
      className={wide ? 'modal modal-wide' : 'modal'}
      aria-labelledby={labelledBy}
      aria-describedby={describedBy}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onClick={(e) => {
        // The dialog itself only receives clicks on the backdrop; its content is the inner box.
        if (e.target === e.currentTarget) onClose();
      }}
    >
      {open && <div className="modal-box">{children}</div>}
    </dialog>
  );
}
