// One service on a leash, as a card. Section 11 shows these on Home, as many as
// fit in one row, up to six.

import { duration } from "../lib/format";
import type { ServiceView } from "../lib/types";

interface ServiceCardProps {
  service: ServiceView;
  /** When this service was last caught, from the log. */
  lastCatch: number | null;
  /** True while the dot is out at the rim for this service. */
  justCaught: boolean;
  /** Now, so the held timer moves once a second. */
  now: number;
  onOpen: () => void;
}

export function ServiceCard({ service, lastCatch, justCaught, now, onOpen }: ServiceCardProps) {
  const held = service.status === "stopped" && lastCatch !== null;

  const status = justCaught
    ? "Tried to start"
    : held
      ? `Held ${duration(now - lastCatch)}`
      : service.status === "running"
        ? policyText(service)
        : "Stopped";

  return (
    <button
      type="button"
      onClick={onOpen}
      className={`flex flex-col gap-0.5 rounded-card border bg-surface p-3.5 text-left transition-transform active:scale-[0.97] ${
        justCaught ? "border-warn" : "border-border"
      }`}
    >
      <b className="font-semibold">{service.display_name}</b>
      <span className="font-mono text-xs text-muted">{service.name}</span>
      <span
        className={`mt-3 flex items-center gap-1.5 font-mono text-xs ${
          service.status === "running" ? "text-warn" : "text-ok"
        }`}
      >
        <i aria-hidden="true" className="inline-block size-2 rounded-full bg-current" />
        {status}
      </span>
    </button>
  );
}

/** What a running service is doing, said in three words. */
function policyText(service: ServiceView): string {
  switch (service.policy) {
    case "notify":
      return "Running, notify";
    case "dry_run":
      return "Running, dry run";
    default:
      return "Running";
  }
}
