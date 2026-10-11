// The warning dialog for a service that is critical. Section 8 asks the user to
// confirm before Leash is allowed to stop one.

import { PRIMARY, GHOST } from "../lib/ui";
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
      <b>Potentially critical service</b>
      <p className="my-2 text-[13px] text-muted">
        Stopping this service may affect Windows functionality.{detail ? ` ${detail}` : ""}
      </p>
      <div className="mt-3.5 flex justify-end gap-2">
        <button type="button" onClick={onCancel} className={GHOST}>
          Cancel
        </button>
        <button type="button" onClick={onAccept} className={PRIMARY}>
          I understand
        </button>
      </div>
    </Dialog>
  );
}
