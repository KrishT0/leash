// One service in the list. On the Suggested view the plain reason replaces the
// internal name, as section 11 describes.

import { policyLabel } from "../lib/services";
import type { ServiceView } from "../lib/types";
import { Tag } from "./Tag";

interface ServiceRowProps {
  service: ServiceView;
  /** True on the Suggested view. */
  showReason: boolean;
  onOpen: () => void;
}

export function ServiceRow({ service, showReason, onOpen }: ServiceRowProps) {
  const suggested = showReason ? service.suggested : null;

  return (
    <button
      type="button"
      onClick={onOpen}
      className="flex w-full cursor-pointer items-center justify-between gap-3 border-t border-hair px-0.5 py-3.5 text-left transition-colors duration-700 ease-spring hover:bg-shell"
    >
      <span className="min-w-0">
        <span className="block truncate text-[15px] font-medium">{service.display_name}</span>
        {suggested ? (
          <span className="mt-0.5 block max-w-[46ch] text-xs text-muted">{suggested.reason}</span>
        ) : (
          <span className="block truncate font-mono text-xs text-muted">{service.name}</span>
        )}
      </span>

      <span className="flex flex-none items-center gap-1.5">
        {rightHandTag(service, showReason)}
        {service.policy ? <Tag tone="ok">{policyLabel(service.policy)}</Tag> : null}
      </span>
    </button>
  );
}

/** What the row says on the right: why it is blocked, its risk, or its state. */
function rightHandTag(service: ServiceView, showReason: boolean) {
  if (service.verdict.level === "blocked") {
    return <Tag tone="error">blocked</Tag>;
  }
  if (showReason && service.suggested) {
    return (
      <Tag tone={service.suggested.risk === "low" ? "ok" : "warn"}>
        {service.suggested.risk} risk
      </Tag>
    );
  }
  return <span className="font-mono text-xs text-muted">{statusWord(service.status)}</span>;
}

/** The service state in one word. */
function statusWord(status: ServiceView["status"]): string {
  switch (status) {
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
