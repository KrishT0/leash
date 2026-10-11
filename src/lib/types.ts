// The shapes the commands send back. They mirror the Rust types.

/** What Leash does when a monitored service starts. */
export type Policy = "notify" | "dry_run" | "stop";

/** How careful Leash has to be with a service. Section 8. */
export type Level = "blocked" | "critical" | "normal";

/** Why the safety check answered that. The engine owns the wording. */
export type Reason =
  | "deny_list"
  | "never_touch"
  | "stop_not_accepted"
  | "driver"
  | "not_win32_service"
  | "sensitive_list"
  | "running_dependents"
  | "none";

export type Status =
  | "running"
  | "stopped"
  | "start_pending"
  | "stop_pending"
  | "paused"
  | "pause_pending"
  | "continue_pending"
  | "unknown";

export type Startup = "automatic" | "manual" | "disabled" | "boot" | "system" | "unknown";

/** One service, as the service manager reports it. */
export interface ServiceInfo {
  name: string;
  display_name: string;
  /** What it does. Empty in the list, filled by getService. */
  description: string;
  status: Status;
  startup: Startup;
  accepts_stop: boolean;
  is_driver: boolean;
  is_win32: boolean;
  third_party: boolean;
  pid: number | null;
}

export interface Verdict {
  level: Level;
  reason: Reason;
}

/** One service, with the safety check and the policy the user chose. */
export interface ServiceView extends ServiceInfo {
  verdict: Verdict;
  /** The verdict as a sentence. The engine owns the wording. */
  verdict_text: string;
  policy: Policy | null;
  gave_up: boolean;
  /** Only getService fills this in. */
  dependents: string[];
}

/** A day and the catches on it. */
export interface DayCount {
  date: string;
  count: number;
}

export interface General {
  paused: boolean;
  notifications: boolean;
  start_with_windows: boolean;
  start_minimized: boolean;
  start_watchdog: boolean;
}

/** Everything the UI reads at once. */
export interface Snapshot {
  elevated: boolean;
  paused: boolean;
  watching: boolean;
  monitored: ServiceView[];
  /** Names in the config that Windows will not let Leash watch. */
  unwatched: string[];
  caught_today: number;
  weekly: DayCount[];
  /** When Leash last caught something, for the quiet timer. */
  last_catch: string | null;
  general: General;
  /** Set when the config file could not be read. */
  config_error: string | null;
}

/** One line of the activity log. */
export interface Entry {
  ts: string;
  service: string;
  event: string;
  action: string | null;
  result: string | null;
  error: string | null;
}

/** What happened when Leash asked a service to stop. */
export type StopOutcome =
  | "stopped"
  | "not_accepted"
  | "not_stopped"
  | { dependents_running: { dependents: string[] } };

/** What happened when Leash asked a service to start. */
export type StartOutcome = "started" | "disabled" | "not_started";
