// Settings. Section 11: the elevation status, the platform, and the switches for
// notices, starting with Windows, starting minimized, and starting the watchdog.

import { useState } from "react";
import { Bezel } from "../components/Bezel";
import { Switch } from "../components/Switch";
import { Tag } from "../components/Tag";
import { setSettings } from "../lib/api";
import type { General, Snapshot } from "../lib/types";
import { EYEBROW } from "../lib/ui";

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
    <section className="grid gap-4">
      <Bezel coreClassName="gap-4">
        <p className={`m-0 ${EYEBROW}`}>This machine</p>

        <div className="flex items-center justify-between gap-3">
          <span className="flex items-center gap-2.5 text-[15px]">
            <i
              aria-hidden="true"
              className={`block size-2 rounded-island ${state.elevated ? "bg-ok" : "bg-error"}`}
            />
            {state.elevated ? "Running elevated" : "Not elevated"}
          </span>
          <Tag tone={state.elevated ? "ok" : "error"}>
            {state.elevated ? "Stop actions on" : "Stop actions off"}
          </Tag>
        </div>

        <div className="flex items-center justify-between gap-3 border-t border-hair pt-4 text-[15px]">
          <span>Platform</span>
          <span className="text-muted">Windows 11 only</span>
        </div>
      </Bezel>

      <Bezel coreClassName="gap-4">
        <p className={`m-0 ${EYEBROW}`}>Preferences</p>

        <div>
          {switches.map(({ key, label }) => (
            <label
              key={key}
              className="flex items-center justify-between gap-4 border-t border-hair py-4 text-[15px] first:border-t-0 first:pt-1"
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

        {problem ? <p className="m-0 text-[13px] text-error">{problem}</p> : null}

        {state.config_error ? (
          <p className="m-0 text-[13px] text-warn">
            Leash is using its default settings, because the config file could not be read. It will
            not write over that file until you fix it.
          </p>
        ) : null}
      </Bezel>
    </section>
  );
}
