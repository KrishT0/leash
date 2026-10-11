// The warning dialog for a service that is critical. Section 8 asks the user to
// confirm before Leash is allowed to stop one.

import { GHOST } from "../lib/ui";
import { Cta } from "./Button";
import { Dialog } from "./Dialog";

interface WarnDialogProps {
  open: boolean;
  /** What makes this service critical, in the engine's words. */
  detail: string;
  onCancel: () => void;
  onAccept: () => void;
}

export function WarnDialog({ open, detail, onCancel, onAccept }: WarnDialogProps) {
  return (
    <Dialog open={open} onClose={onCancel} label="Potentially critical service">
      <h2 className="m-0 text-[15px] font-semibold tracking-[-0.02em]">Potentially critical service</h2>
      <p className="mt-2 mb-0 text-[12.5px] text-muted">
        Stopping this service may affect Windows functionality.{detail ? ` ${detail}` : ""}
      </p>
      <div className="mt-3 flex justify-end gap-2 border-t border-hair pt-3.5">
        <button type="button" onClick={onCancel} className={GHOST}>
          Cancel
        </button>
        <Cta onClick={onAccept}>
          I understand
        </Cta>
      </div>
    </Dialog>
  );
}
