// One service on a leash, as a card. Section 11 shows these on Home, as many as
// fit in one row, up to six.

import { duration } from "../lib/format";
import type { ServiceView } from "../lib/types";
import { TAG, TAG_TONES } from "../lib/ui";

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
  const running = service.status === "running";
  const tone = justCaught ? "warn" : running ? "dim" : "ok";

  const state = justCaught
    ? "Tried to start"
    : held
      ? `Held ${duration(now - lastCatch)}`
      : running
        ? "Running"
        : "Stopped";

  return (
    <button
      type="button"
      onClick={onOpen}
      className={`flex h-full flex-col gap-1 rounded-item border p-3 text-left transition-[transform,border-color,background] duration-700 ease-spring hover:bg-control active:scale-[0.97] ${
        justCaught
          ? "border-warn/40 bg-warn/8"
          : "border-hair bg-control"
      }`}
    >
      {/* Two lines are reserved, so the state tag on every card sits at the
          same height however long the name is. */}
      <b className="line-clamp-2 min-h-[2.75em] text-[14px] leading-snug font-medium">
        {service.display_name}
      </b>
      <span className="font-mono text-[11px] text-muted">{service.name}</span>
      <div className="mt-auto pt-3">
        <span className={`${TAG} ${TAG_TONES[tone]}`}>
          {state} · {policyText(service)}
        </span>
      </div>
    </button>
  );
}

/** What the policy is, in two words at most. */
function policyText(service: ServiceView): string {
  switch (service.policy) {
    case "notify":
      return "Notify";
    case "dry_run":
      return "Dry run";
    default:
      return "Stop";
  }
}
