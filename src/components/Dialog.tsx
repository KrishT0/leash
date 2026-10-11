// The native dialog, as section 4 asks, with the small amount of wiring that
// every dialog needs: open it, close it on Escape, and close it on a click on
// the backdrop.

import { useEffect, useRef, type ReactNode } from "react";

interface DialogProps {
  open: boolean;
  onClose: () => void;
  /** For screen readers, because these dialogs have no visible title element. */
  label: string;
  children: ReactNode;
}

export function Dialog({ open, onClose, label, children }: DialogProps) {
  const ref = useRef<HTMLDialogElement | null>(null);

  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;

    if (open && !dialog.open) {
      dialog.showModal();
    }
    if (!open && dialog.open) {
      dialog.close();
    }
  }, [open]);

  return (
    <dialog
      ref={ref}
      aria-label={label}
      // Fires for Escape as well as for our own close calls.
      onClose={onClose}
      onClick={(event) => {
        if (event.target === ref.current) onClose();
      }}
      className="w-[min(380px,calc(100vw-32px))] rounded-card border border-border bg-surface p-5 text-text backdrop:bg-black/60"
    >
      {open ? children : null}
    </dialog>
  );
}
