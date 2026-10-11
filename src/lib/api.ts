// The only file that calls invoke, as section 5 asks.

import { invoke } from "@tauri-apps/api/core";

/** One service the user put on the leash. */
export interface MonitoredService {
  name: string;
  display_name: string;
  /** `notify`, `dry_run`, `stop`, or null when the service is not monitored. */
  policy: string | null;
}

/** Everything the UI reads at once. */
export interface Snapshot {
  elevated: boolean;
  paused: boolean;
  watching: boolean;
  monitored: MonitoredService[];
  /** Names in the config that Windows will not let Leash watch. */
  unwatched: string[];
  caught_today: number;
  weekly: { date: string; count: number }[];
  /** When Leash last caught something, for the quiet timer. */
  last_catch: string | null;
  general: Record<string, unknown>;
  /** Set when the config file could not be read. */
  config_error: string | null;
}

export function getState(): Promise<Snapshot> {
  return invoke<Snapshot>("get_state");
}

export function setPaused(paused: boolean): Promise<void> {
  return invoke("set_paused", { paused });
}

export function rearm(name: string): Promise<void> {
  return invoke("rearm", { name });
}
