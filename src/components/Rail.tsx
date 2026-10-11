// The icon rail, as a floating glass plate. Wide window: a column beside the
// work area. Narrow window: a slim bar above it, as section 11 asks.

import type { Page } from "../lib/pages";
import { PAGES, PAGE_TITLES } from "../lib/pages";

/** The four icons, drawn in the light stroke the design skill asks for. */
const ICONS: Record<Page, React.ReactNode> = {
  home: <path d="M4 11.6 12 5.2l8 6.4V19a1 1 0 0 1-1 1h-4v-6H9v6H5a1 1 0 0 1-1-1z" />,
  services: <path d="M4 7.5h16M4 12h16M4 16.5h10" />,
  activity: <path d="M4 13.5h3.2l2.1-6.2 3 11.4 2.6-6.2H20" />,
  settings: (
    <>
      <circle cx="12" cy="12" r="3.1" />
      <path d="M12 4.2v1.9M12 17.9v1.9M4.2 12h1.9M17.9 12h1.9M6.4 6.4l1.3 1.3M16.3 16.3l1.3 1.3M17.6 6.4l-1.3 1.3M7.7 16.3l-1.3 1.3" />
    </>
  ),
};

interface RailProps {
  page: Page;
  onPick: (page: Page) => void;
  /** null until the state has been read once. */
  elevated: boolean | null;
  paused: boolean | null;
}

export function Rail({ page, onPick, elevated, paused }: RailProps) {
  return (
    <aside className="m-4 flex flex-col gap-6 rounded-rail border border-hair bg-shell p-4 backdrop-blur-xl max-[719px]:m-0 max-[719px]:flex-row max-[719px]:items-center max-[719px]:gap-2 max-[719px]:overflow-x-auto max-[719px]:p-2">
      {/* No glow here. Section 11 keeps the orb as the one bold element. */}
      <div className="flex items-center gap-2.5 px-1 text-[17px] font-semibold tracking-[-0.02em] max-[719px]:hidden">
        <i aria-hidden="true" className="block size-2.5 rounded-island bg-accent" />
        Leash
      </div>

      <nav className="grid gap-1 max-[719px]:grid-flow-col max-[719px]:gap-1">
        {PAGES.map((item) => (
          <button
            key={item}
            type="button"
            title={PAGE_TITLES[item]}
            aria-label={PAGE_TITLES[item]}
            aria-current={page === item ? "page" : undefined}
            onClick={() => onPick(item)}
            className="flex items-center gap-3 rounded-island px-3 py-2.5 text-left text-sm font-medium text-muted transition-[color,background,transform] duration-700 ease-spring hover:translate-x-0.5 hover:bg-border hover:text-text aria-[current=page]:border aria-[current=page]:border-hair aria-[current=page]:bg-border aria-[current=page]:text-text max-[719px]:px-3 max-[719px]:py-2 max-[719px]:whitespace-nowrap"
          >
            <svg
              viewBox="0 0 24 24"
              aria-hidden="true"
              className="size-[18px] flex-none"
              fill="none"
              stroke="currentColor"
              strokeWidth={1.6}
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              {ICONS[item]}
            </svg>
            <span className="max-[719px]:inline">{PAGE_TITLES[item]}</span>
          </button>
        ))}
      </nav>

      {/* The technical block that fills the bottom of the rail. */}
      <div className="mt-auto grid gap-2.5 pt-6 font-mono text-[11px] leading-relaxed text-muted max-[719px]:hidden">
        <span className="flex items-center gap-2">
          <i
            aria-hidden="true"
            className={`block size-[7px] rounded-island ${elevated === false ? "bg-warn" : "bg-ok"}`}
          />
          {elevated === null
            ? "Reading rights"
            : elevated
              ? "Running elevated"
              : "No administrator rights"}
        </span>
        <span className="flex items-center gap-2">
          <i
            aria-hidden="true"
            className={`block size-[7px] rounded-island ${paused ? "bg-warn" : "bg-ok"}`}
          />
          {paused ? "Stop actions off" : "Stop actions on"}
        </span>
        {/* Leash is Windows 11 only, as section 1 says, so this is a constant. */}
        <span>Windows 11</span>
      </div>
    </aside>
  );
}
