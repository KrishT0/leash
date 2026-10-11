// The four pages, and the title each one shows in the top bar.

export type Page = "home" | "services" | "activity" | "settings";

export const PAGES: Page[] = ["home", "services", "activity", "settings"];

export const PAGE_TITLES: Record<Page, string> = {
  home: "Home",
  services: "Services",
  activity: "Activity",
  settings: "Settings",
};

/** The four events the engine sends, as section 10 names them. */
export const ENGINE_EVENTS = [
  "service_caught",
  "service_notified",
  "state_changed",
  "engine_error",
];
