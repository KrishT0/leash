// The add sheet. Section 11: the user types a display name or an internal name,
// matches show a Found tag, a blocked match is disabled, and the name is checked
// with Windows before it is saved.

import { useEffect, useState, type ReactNode } from "react";
import { addMonitor } from "../lib/api";
import type { ServiceView } from "../lib/types";
import { GHOST } from "../lib/ui";
import { Dialog } from "./Dialog";
import { Tag } from "./Tag";
import { WarnDialog } from "./WarnDialog";

/** How many matches to show at once. */
const MAX_MATCHES = 5;

interface AddSheetProps {
  open: boolean;
  services: ServiceView[];
  onClose: () => void;
  /** Called after a service is added, so the list and the app catch up. */
  onAdded: () => void;
}

export function AddSheet({ open, services, onClose, onAdded }: AddSheetProps) {
  const [query, setQuery] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [asking, setAsking] = useState<ServiceView | null>(null);

  // Every open starts fresh.
  useEffect(() => {
    if (open) {
      setQuery("");
      setProblem(null);
    }
  }, [open]);

  const wanted = query.trim().toLowerCase();
  const matches = wanted
    ? services
        .filter((service) =>
          `${service.display_name}${service.name}`.toLowerCase().includes(wanted),
        )
        .slice(0, MAX_MATCHES)
    : [];

  async function add(service: ServiceView) {
    try {
      await addMonitor(service.name);
      setProblem(null);
      onAdded();
      onClose();
    } catch (error) {
      setProblem(String(error));
    }
  }

  return (
    <>
      <Dialog open={open} onClose={onClose} label="Add a service to stop">
        <h2 className="m-0 text-[19px] font-semibold tracking-[-0.02em]">Add a service to stop</h2>
        <p className="mt-2 mb-0 text-[13px] text-muted">
          Type a display name or an internal name. Leash checks it with Windows first.
        </p>

        <input
          type="text"
          value={query}
          onChange={(event) => setQuery(event.currentTarget.value)}
          placeholder="For example: Spooler"
          aria-label="Service name"
          autoComplete="off"
          className="w-full rounded-island border border-hair bg-bg px-4 py-3 text-sm text-text placeholder:text-muted"
        />

        {problem ? <p className="m-0 text-[13px] text-error">{problem}</p> : null}

        {wanted && matches.length === 0 ? (
          <p className="m-0 text-[13px] text-muted">
            Not found. Check the internal name in services.msc.
          </p>
        ) : null}

        {matches.map((service) => (
          <MatchRow
            key={service.name}
            service={service}
            onAdd={() => (service.verdict.level === "critical" ? setAsking(service) : void add(service))}
          />
        ))}

        <div className="mt-4 flex justify-end gap-2 border-t border-hair pt-5">
          <button type="button" className={GHOST} onClick={onClose}>
            Close
          </button>
        </div>
      </Dialog>

      <WarnDialog
        open={asking !== null}
        detail={
          asking?.dependents.length
            ? `Other services depend on it: ${asking.dependents.join(", ")}.`
            : ""
        }
        onCancel={() => setAsking(null)}
        onAccept={() => {
          const service = asking;
          setAsking(null);
          if (service) void add(service);
        }}
      />
    </>
  );
}

/** One match, with the tag that says what will happen. */
function MatchRow({ service, onAdd }: { service: ServiceView; onAdd: () => void }) {
  const blocked = service.verdict.level === "blocked";
  const monitored = service.policy !== null;

  return (
    <button
      type="button"
      disabled={blocked || monitored}
      onClick={onAdd}
      className="flex w-full cursor-pointer items-center justify-between gap-3 border-t border-hair px-0.5 py-3.5 text-left transition-colors duration-700 ease-spring hover:bg-shell disabled:cursor-default disabled:opacity-50"
    >
      <span className="min-w-0">
        <span className="block truncate text-[15px] font-medium">{service.display_name}</span>
        <span className="block truncate font-mono text-xs text-muted">{service.name}</span>
      </span>
      <span className="flex items-center font-mono text-xs text-muted">
        {blocked ? <Tag tone="error">Blocked</Tag> : monitored ? <Tag tone="ok">On a leash</Tag> : <Found />}
      </span>
    </button>
  );
}

/** The Found tag plus the word Add, as the design reference shows them. */
function Found(): ReactNode {
  return (
    <>
      <Tag tone="ok">Found</Tag>
      <span className="ml-1.5">Add</span>
    </>
  );
}
