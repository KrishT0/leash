// The icon rail. Wide window: a 68px column of icons with tooltips. Narrow
// window: a slim top bar with the names showing, as section 11 asks.

import type { Page } from "../lib/pages";
import { PAGES, PAGE_TITLES } from "../lib/pages";

/** The four icons, drawn the way the design reference draws them. */
const ICONS: Record<Page, React.ReactNode> = {
  home: <path d="M3 11l9-8 9 8v9a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z" />,
  services: <path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01" />,
  activity: <path d="M3 12h4l3-8 4 16 3-8h4" />,
  settings: (
    <>
      <path d="M4 6h9m4 0h3M4 12h3m4 0h9M4 18h11m4 0h1" />
      <circle cx="15" cy="6" r="2" />
      <circle cx="9" cy="12" r="2" />
      <circle cx="17" cy="18" r="2" />
    </>
  ),
};

interface RailProps {
  page: Page;
  onPick: (page: Page) => void;
}

export function Rail({ page, onPick }: RailProps) {
  return (
    <aside className="sticky top-0 flex h-dvh flex-col gap-[18px] self-start border-r border-border px-3 py-5 max-[719px]:static max-[719px]:h-auto max-[719px]:flex-row max-[719px]:items-center max-[719px]:gap-1.5 max-[719px]:overflow-x-auto max-[719px]:border-r-0 max-[719px]:border-b max-[719px]:px-3 max-[719px]:py-2.5">
      <nav className="flex flex-col gap-0.5 max-[719px]:mx-auto max-[719px]:flex-row">
        {PAGES.map((item) => (
          <button
            key={item}
            type="button"
            title={PAGE_TITLES[item]}
            aria-label={PAGE_TITLES[item]}
            aria-current={page === item ? "page" : undefined}
            onClick={() => onPick(item)}
            className="flex items-center justify-center gap-2.5 rounded-nav px-0 py-[11px] text-left text-sm font-medium text-muted hover:bg-surface hover:text-text aria-[current=page]:bg-surface aria-[current=page]:text-text max-[719px]:px-3 max-[719px]:py-2 max-[719px]:whitespace-nowrap"
          >
            <svg
              viewBox="0 0 24 24"
              aria-hidden="true"
              className="size-4 flex-none"
              fill="none"
              stroke="currentColor"
              strokeWidth={2}
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              {ICONS[item]}
            </svg>
            <span className="hidden max-[719px]:inline">{PAGE_TITLES[item]}</span>
          </button>
        ))}
      </nav>
    </aside>
  );
}
