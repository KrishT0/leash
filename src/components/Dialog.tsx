// The native dialog. A dialog is the one legitimate layer, so it floats above
// the surface as a single card with a heavy frosted scrim behind it.

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
      // Tailwind's reset drops the margin that centres a dialog, so it is set
      // again here. The height cap keeps a long sheet inside the window.
      className="m-auto max-h-[calc(100vh-2rem)] w-[min(540px,calc(100vw-32px))] overflow-x-hidden overflow-y-auto rounded-panel border border-border bg-panel text-text shadow-[0_50px_120px_-30px_rgba(0,0,0,0.72)] backdrop:bg-bg/75 backdrop:backdrop-blur-xl"
    >
      {open ? <div className="p-6">{children}</div> : null}
    </dialog>
  );
}
