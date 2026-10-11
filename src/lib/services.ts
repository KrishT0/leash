// Which services the Services page shows. The rules are simple enough to test,
// which is why they live away from the page.

import type { ServiceView } from "./types";

/** The three views above the list. */
export type ServiceTab = "suggested" | "third_party" | "all";

export interface Filters {
  status: "all" | "running" | "stopped";
  monitoring: "all" | "monitored" | "ignored";
  startup: "all" | "automatic" | "manual";
  /** Blocked services are hidden unless this is on. */
  blocked: boolean;
}

export const DEFAULT_FILTERS: Filters = {
  status: "all",
  monitoring: "all",
  startup: "all",
  blocked: false,
};

export const TAB_LABELS: Record<ServiceTab, string> = {
  suggested: "Suggested",
  third_party: "Third-party",
  all: "All",
};

/** How many filters are away from their default, for the Filters button. */
export function activeFilters(filters: Filters): number {
  let count = 0;
  if (filters.status !== "all") count += 1;
  if (filters.monitoring !== "all") count += 1;
  if (filters.startup !== "all") count += 1;
  if (filters.blocked) count += 1;
  return count;
}

/** The services to show, in the order the engine gave them. */
export function filterServices(
  services: ServiceView[],
  tab: ServiceTab,
  query: string,
  filters: Filters,
): ServiceView[] {
  const wanted = query.trim().toLowerCase();

  return services.filter((service) => {
    if (wanted && !`${service.display_name}${service.name}`.toLowerCase().includes(wanted)) {
      return false;
    }
    if (tab === "suggested" && !service.suggested) return false;
    if (tab === "third_party" && !service.third_party) return false;
    if (service.verdict.level === "blocked" && !filters.blocked) return false;
    if (filters.status === "running" && service.status !== "running") return false;
    if (filters.status === "stopped" && service.status !== "stopped") return false;

    const monitored = service.policy !== null;
    if (filters.monitoring === "monitored" && !monitored) return false;
    if (filters.monitoring === "ignored" && monitored) return false;

    if (filters.startup === "automatic" && service.startup !== "automatic") return false;
    if (filters.startup === "manual" && service.startup !== "manual") return false;

    return true;
  });
}

/** The words for a service's policy, as the design reference writes them. */
export function policyLabel(policy: string | null): string {
  switch (policy) {
    case "notify":
      return "notify";
    case "dry_run":
      return "dry run";
    case "stop":
      return "stop";
    default:
      return "";
  }
}
