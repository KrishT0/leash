// The filter sheet. Section 11: status, monitoring, startup, and a switch for
// the blocked services, with the button saying how many rows are left.

import { type ReactNode } from "react";
import { DEFAULT_FILTERS, type Filters } from "../lib/services";
import { EYEBROW, GHOST } from "../lib/ui";
import { Cta } from "./Button";
import { Dialog } from "./Dialog";
import { Segmented } from "./Segmented";
import { Switch } from "./Switch";

const STATUS = [
  { value: "all" as const, label: "All" },
  { value: "running" as const, label: "Running" },
  { value: "stopped" as const, label: "Stopped" },
];

const MONITORING = [
  { value: "all" as const, label: "All" },
  { value: "monitored" as const, label: "Monitored" },
  { value: "ignored" as const, label: "Ignored" },
];

const STARTUP = [
  { value: "all" as const, label: "All" },
  { value: "automatic" as const, label: "Automatic" },
  { value: "manual" as const, label: "Manual" },
];

interface FilterSheetProps {
  open: boolean;
  filters: Filters;
  /** How many services the filters leave, for the primary button. */
  count: number;
  onChange: (filters: Filters) => void;
  onClose: () => void;
}

export function FilterSheet({ open, filters, count, onChange, onClose }: FilterSheetProps) {
  const set = (part: Partial<Filters>) => onChange({ ...filters, ...part });

  return (
    <Dialog open={open} onClose={onClose} label="Filter services">
      <h2 className="m-0 text-[17px] font-semibold tracking-[-0.02em]">Filter services</h2>

      <Group label="Status">
        <Segmented
          options={STATUS}
          value={filters.status}
          onChange={(status) => set({ status })}
          label="Status"
        />
      </Group>

      <Group label="Monitoring">
        <Segmented
          options={MONITORING}
          value={filters.monitoring}
          onChange={(monitoring) => set({ monitoring })}
          label="Monitoring"
        />
      </Group>

      <Group label="Startup">
        <Segmented
          options={STARTUP}
          value={filters.startup}
          onChange={(startup) => set({ startup })}
          label="Startup"
        />
      </Group>

      <label className="mt-1 flex items-center justify-between gap-3 border-t border-hair py-3.5">
        <span>Show blocked services</span>
        <Switch
          checked={filters.blocked}
          onChange={(blocked) => set({ blocked })}
          label="Show blocked services"
        />
      </label>

      <div className="mt-3.5 flex justify-end gap-2 border-t border-hair pt-4">
        <button type="button" className={GHOST} onClick={() => onChange(DEFAULT_FILTERS)}>
          Reset
        </button>
        <Cta onClick={onClose}>
          Show {count} services
        </Cta>
      </div>
    </Dialog>
  );
}

/** A muted label above a segmented control. */
function Group({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div>
      <p className={`mt-0 mb-0 ${EYEBROW}`}>{label}</p>
      {children}
    </div>
  );
}
