// The page title and the Watching or Paused island, as section 11 describes the
// top bar. An island, not a bar glued to the top: it floats.

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
    <header className="flex items-center justify-between gap-4">
      <h1 className="m-0 text-3xl font-semibold tracking-[-0.03em]">{PAGE_TITLES[page]}</h1>

      <button
        type="button"
        aria-pressed={paused === true}
        disabled={paused === null}
        onClick={onTogglePause}
        className="inline-flex items-center gap-2.5 rounded-island border border-hair bg-shell px-4 py-2 text-xs font-medium text-muted backdrop-blur-xl transition-[background,transform] duration-700 ease-spring hover:bg-border disabled:opacity-60"
      >
        <i aria-hidden="true" className={`block size-[7px] rounded-island ${dot} ${glow}`} />
        {label}
      </button>
    </header>
  );
}
