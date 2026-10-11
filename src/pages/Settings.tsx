// Settings. Section 11: the elevation status, the platform, and the switches for
// notices, starting with Windows, starting minimized, and starting the watchdog.

import { useState } from "react";
import { Switch } from "../components/Switch";
import { setSettings } from "../lib/api";
import type { General, Snapshot } from "../lib/types";

interface SettingsProps {
  state: Snapshot;
  /** Called after a change, so the app reads the state again. */
  onChanged: () => void;
}

export function Settings({ state, onChanged }: SettingsProps) {
  const [problem, setProblem] = useState<string | null>(null);

  async function change(part: Partial<General>) {
    try {
      await setSettings({ ...state.general, ...part });
      setProblem(null);
      onChanged();
    } catch (error) {
      setProblem(String(error));
    }
  }

  const switches: { key: keyof General; label: string }[] = [
    { key: "notifications", label: "Notifications (grouped)" },
    { key: "start_with_windows", label: "Start Leash with Windows" },
    { key: "start_minimized", label: "Start minimized to tray" },
    { key: "start_watchdog", label: "Start the watchdog automatically" },
  ];

  return (
    <section>
      <h2 className="mt-0 mb-2.5 text-sm font-medium text-muted">Preferences</h2>

      <div className="mt-2.5 rounded-card border border-border bg-surface px-4 py-1">
        <div className="flex items-center justify-between gap-3 py-3">
          <span className="flex items-center gap-2">
            <i
              aria-hidden="true"
              className={`inline-block size-2 rounded-full ${
                state.elevated ? "bg-ok" : "bg-error"
              }`}
            />
            {state.elevated ? "Running elevated" : "Not elevated"}
          </span>
          <span className="text-muted">
            {state.elevated ? "Stop actions on" : "Stop actions off"}
          </span>
        </div>

        <div className="flex items-center justify-between gap-3 border-t border-border py-3">
          <span>Platform</span>
          <span className="text-muted">Windows 11 only</span>
        </div>
      </div>

      <div className="mt-2.5 rounded-card border border-border bg-surface px-4 py-1">
        {switches.map(({ key, label }) => (
          <label
            key={key}
            className="flex items-center justify-between gap-3 border-t border-border py-2.5 first:border-t-0"
          >
            <span>{label}</span>
            <Switch
              checked={state.general[key]}
              onChange={(checked) => void change({ [key]: checked })}
              label={label}
            />
          </label>
        ))}
      </div>

      {problem ? <p className="my-2 text-[13px] text-error">{problem}</p> : null}

      {state.config_error ? (
        <p className="my-2 text-[13px] text-warn">
          Leash is using its default settings, because the config file could not be read. It will
          not write over that file until you fix it.
        </p>
      ) : null}
    </section>
  );
}
