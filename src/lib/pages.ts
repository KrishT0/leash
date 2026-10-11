// The four pages, and the title each one shows in the top bar.

export type Page = "home" | "services" | "activity" | "settings";

export const PAGES: Page[] = ["home", "services", "activity", "settings"];

export const PAGE_TITLES: Record<Page, string> = {
  home: "Home",
  services: "Services",
  activity: "Activity",
  settings: "Settings",
};

/** The event that means a service was caught, which the orbit animates. */
export const SERVICE_CAUGHT = "service_caught";

/** The events that only mean the numbers on screen are out of date. */
export const REFRESH_EVENTS = ["service_notified", "state_changed", "engine_error"];

/** How long a caught service stays out at the rim. */
export const CAUGHT_HOLD_MS = 900;
