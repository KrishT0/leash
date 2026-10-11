// The page title and the Watching or Paused button, as section 11 describes the
// top bar.

import type { Page } from "../lib/pages";
import { PAGE_TITLES } from "../lib/pages";

interface TopBarProps {
  page: Page;
  /** null until the state has been read once. */
  paused: boolean | null;
  onTogglePause: () => void;
}

export function TopBar({ page, paused, onTogglePause }: TopBarProps) {
  const dot = paused === null ? "text-muted" : paused ? "text-warn" : "text-ok";
  const label = paused === null ? "Reading" : paused ? "Paused" : "Watching";

  return (
    <header className="mb-2 flex items-center justify-between">
      <h1 className="m-0 text-[22px] font-bold tracking-[-0.02em]">{PAGE_TITLES[page]}</h1>

      <button
        type="button"
        aria-pressed={paused === true}
        disabled={paused === null}
        onClick={onTogglePause}
        className="flex items-center gap-2 rounded-pill border border-border bg-surface px-3.5 py-[7px] text-[13px] font-medium text-text"
      >
        <i aria-hidden="true" className={`inline-block size-2 rounded-full bg-current ${dot}`} />
        <span>{label}</span>
      </button>
    </header>
  );
}
