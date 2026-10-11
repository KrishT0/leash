// The service sheet: everything Leash knows about one service, and the only
// place the user changes what Leash does with it.

import { useCallback, useEffect, useState, type ReactNode } from "react";
import { addMonitor, getService, removeMonitor, setPolicy, startNow, stopNow } from "../lib/api";
import type { Policy, ServiceView } from "../lib/types";
import { GHOST } from "../lib/ui";
import { Cta } from "./Button";
import { Dialog } from "./Dialog";
import { Segmented } from "./Segmented";
import { Switch } from "./Switch";
import { Tag } from "./Tag";
import { WarnDialog } from "./WarnDialog";

/** The policy ladder, in the order the user climbs it. Section 7. */
const POLICIES = [
  { value: "notify" as Policy, label: "Notify only" },
  { value: "dry_run" as Policy, label: "Dry run" },
  { value: "stop" as Policy, label: "Stop" },
];

interface ServiceSheetProps {
  /** The service to show, or null when the sheet is closed. */
  name: string | null;
  onClose: () => void;
  /** Called after anything that changes the config, so the list can catch up. */
  onChanged: () => void;
}

export function ServiceSheet({ name, onClose, onChanged }: ServiceSheetProps) {
  const [service, setService] = useState<ServiceView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [asking, setAsking] = useState(false);

  const load = useCallback(async (wanted: string) => {
    try {
      setService(await getService(wanted));
      setProblem(null);
    } catch (error) {
      setProblem(String(error));
    }
  }, []);

  useEffect(() => {
    if (name === null) {
      setService(null);
      setProblem(null);
      return;
    }
    void load(name);
  }, [name, load]);

  /** Run a command, then bring the sheet and the list up to date. */
  async function act(command: () => Promise<unknown>) {
    try {
      await command();
      setProblem(null);
      if (name !== null) await load(name);
      onChanged();
    } catch (error) {
      setProblem(String(error));
    }
  }

  const blocked = service?.verdict.level === "blocked";
  const critical = service?.verdict.level === "critical";
  const monitored = service?.policy != null;

  /** Turning Monitor on for a critical service asks first. */
  function toggleMonitor(on: boolean) {
    if (!service) return;
    if (on && critical) {
      setAsking(true);
      return;
    }
    void act(() => (on ? addMonitor(service.name) : removeMonitor(service.name)));
  }

  return (
    <>
      <Dialog open={name !== null} onClose={onClose} label="Service">
        {service ? (
          <div>
            <div className="flex items-start justify-between gap-3">
              <div>
                <h2 className="m-0 text-[28px] leading-tight font-semibold tracking-[-0.03em]">
                  {service.display_name}
                </h2>
                <div className="mt-1 font-mono text-xs text-muted">{service.name}</div>
              </div>
              <div className="flex flex-none items-center gap-1.5 pt-1.5">
                {blocked ? <Tag tone="error">Blocked</Tag> : null}
                {critical ? <Tag tone="warn">Critical</Tag> : null}
                <Tag tone="dim">{service.third_party ? "Third party" : "Windows"}</Tag>
              </div>
            </div>

            <dl className="my-4 grid grid-cols-[86px_1fr] gap-x-3 gap-y-2 border-t border-hair pt-5">
              <dt className="text-muted">Status</dt>
              <dd className="m-0 font-mono text-[13px]">{statusWord(service)}</dd>
              <dt className="text-muted">Startup</dt>
              <dd className="m-0 font-mono text-[13px]">{startupWord(service.startup)}</dd>
              <dt className="text-muted">PID</dt>
              <dd className="m-0 font-mono text-[13px]">{service.pid ?? "none"}</dd>
            </dl>

            {service.description ? (
              <p className="my-2 text-[13px] text-muted">{service.description}</p>
            ) : null}

            {service.suggested ? (
              <p className="my-2 text-[13px] text-muted">
                <Note>Why stop it:</Note> {service.suggested.reason}
                <br />
                <Note>If you stop it:</Note> {service.suggested.effect}
              </p>
            ) : null}

            {critical && service.dependents.length > 0 ? (
              <p className="my-2 text-[13px] text-muted">
                <Note>Depends on it:</Note> {service.dependents.join(", ")}
              </p>
            ) : null}

            {service.gave_up ? (
              <p className="my-2 text-[13px] text-warn">
                Leash stopped trying after {service.name} kept coming back.
              </p>
            ) : null}

            {blocked ? (
              <p className="my-2 text-[13px] text-muted">
                <Note>Leash cannot stop this service.</Note> {service.verdict_text}
              </p>
            ) : (
              <>
                <label className="mt-2 flex items-center justify-between gap-3 border-t border-hair py-4">
                  <span>Monitor this service</span>
                  <Switch
                    checked={monitored}
                    onChange={toggleMonitor}
                    label="Monitor this service"
                  />
                </label>

                <Segmented
                  options={POLICIES}
                  value={service.policy ?? "notify"}
                  onChange={(policy) => void act(() => setPolicy(service.name, policy))}
                  label="Policy"
                  disabled={!monitored}
                />

                <p className="my-2 text-[13px] text-muted">
                  Try it first: notify, then dry run, then stop.
                </p>
              </>
            )}

            {problem ? <p className="my-2 text-[13px] text-error">{problem}</p> : null}

            <div className="mt-4 flex justify-end gap-2 border-t border-hair pt-5">
              <button type="button" onClick={onClose} className={GHOST}>
                Close
              </button>
              {blocked ? null : (
                <Cta
                  onClick={() =>
                    void act(() =>
                      service.status === "running" ? stopNow(service.name) : startNow(service.name),
                    )
                  }
                >
                  {service.status === "running" ? "Stop now" : "Start now"}
                </Cta>
              )}
            </div>
          </div>
        ) : (
          <p className="text-muted">{problem ?? "Reading the service."}</p>
        )}
      </Dialog>

      <WarnDialog
        open={asking}
        detail={service?.dependents.length ? `Other services depend on it: ${service.dependents.join(", ")}.` : ""}
        onCancel={() => setAsking(false)}
        onAccept={() => {
          setAsking(false);
          if (service) void act(() => addMonitor(service.name));
        }}
      />
    </>
  );
}

/** The bold lead in to a note. */
function Note({ children }: { children: ReactNode }) {
  return <b className="font-medium text-text">{children}</b>;
}

/** The service state in one word. */
function statusWord(service: ServiceView): string {
  switch (service.status) {
    case "running":
      return "Running";
    case "stopped":
      return "Stopped";
    case "start_pending":
      return "Starting";
    case "stop_pending":
      return "Stopping";
    case "paused":
      return "Paused";
    case "pause_pending":
      return "Pausing";
    case "continue_pending":
      return "Resuming";
    default:
      return "Unknown";
  }
}

/** The startup type in the words Windows uses. */
function startupWord(startup: ServiceView["startup"]): string {
  switch (startup) {
    case "automatic":
      return "Automatic";
    case "manual":
      return "Manual";
    case "disabled":
      return "Disabled";
    case "boot":
      return "Boot";
    case "system":
      return "System";
    default:
      return "Unknown";
  }
}
