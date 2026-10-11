// The native dialog, as section 4 asks. The same double-bezel as a card, with a
// heavy frosted scrim behind it and the close wiring every dialog needs.

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
      className="w-[min(540px,calc(100vw-32px))] rounded-bezel border border-border bg-shell p-[7px] text-text shadow-[0_50px_120px_-30px_rgba(0,0,0,0.7)] backdrop:bg-bg/75 backdrop:backdrop-blur-xl"
    >
      {open ? <div className="rounded-core bg-surface shadow-core">{children}</div> : null}
    </dialog>
  );
}
