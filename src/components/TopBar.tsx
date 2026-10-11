// The page title and the Watching or Paused island. The rule under this bar is
// the only horizontal divider at the top of the work area, so nothing needs a
// frame around it.

import type { Page } from "../lib/pages";
import { PAGE_TITLES } from "../lib/pages";

interface TopBarProps {
  page: Page;
  /** null until the state has been read once. */
  paused: boolean | null;
  onTogglePause: () => void;
}

export function TopBar({ page, paused, onTogglePause }: TopBarProps) {
  const dot = paused === null ? "bg-muted" : paused ? "bg-warn" : "bg-ok";
  const glow =
    paused === null
      ? ""
      : paused
        ? "shadow-[0_0_12px_var(--color-warn)]"
        : "shadow-[0_0_12px_var(--color-ok)]";
  const label = paused === null ? "Reading" : paused ? "Paused" : "Watching";

  return (
    <header className="flex items-center justify-between gap-4 border-b border-hair pb-4">
      <h1 className="m-0 text-2xl font-semibold tracking-[-0.03em]">{PAGE_TITLES[page]}</h1>

      <button
        type="button"
        aria-pressed={paused === true}
        disabled={paused === null}
        onClick={onTogglePause}
        className="inline-flex items-center gap-2.5 rounded-island border border-hair bg-control px-3.5 py-1.5 text-[12px] font-medium text-muted transition-[background,transform] duration-700 ease-spring hover:bg-border disabled:opacity-60"
      >
        <i aria-hidden="true" className={`block size-[7px] rounded-island ${dot} ${glow}`} />
        {label}
      </button>
    </header>
  );
}
