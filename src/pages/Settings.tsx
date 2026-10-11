// Settings. Section 11 gives this the elevation status, the platform, and the
// switches. This is the plain first step, and the values are real.

import type { Snapshot } from "../lib/types";

interface SettingsProps {
  state: Snapshot;
}

export function Settings({ state }: SettingsProps) {
  return (
    <section>
      <dl className="grid grid-cols-[110px_1fr] gap-x-2 gap-y-1 text-[13px]">
        <dt className="text-muted">Rights</dt>
        <dd className="m-0 font-mono">
          {state.elevated ? "Running elevated" : "Normal user"}
        </dd>
        <dt className="text-muted">Platform</dt>
        <dd className="m-0 font-mono">Windows 11</dd>
        <dt className="text-muted">Watchdog</dt>
        <dd className="m-0 font-mono">{state.watching ? "Running" : "Stopped"}</dd>
        <dt className="text-muted">Notices</dt>
        <dd className="m-0 font-mono">{state.general.notifications ? "On" : "Off"}</dd>
      </dl>
    </section>
  );
}
