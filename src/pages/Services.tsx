// Services. Section 11 gives this the search box, the three views, the filter
// sheet, the add sheet, and the service sheet. This is the plain first step.

import type { Snapshot } from "../lib/types";

interface ServicesProps {
  state: Snapshot;
}

export function Services({ state }: ServicesProps) {
  return (
    <section>
      <p className="text-[13px] text-muted">
        {state.monitored.length === 0
          ? "No services are being watched."
          : `${state.monitored.length} on a leash.`}
      </p>
    </section>
  );
}
