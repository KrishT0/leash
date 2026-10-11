// Turning engine values into the words and the numbers the UI shows.

import type { Tone } from "./ui";
import type { DayCount, Entry } from "./types";

/** "H:MM:SS", the way the design reference writes a duration. */
export function duration(milliseconds: number): string {
  const total = Math.max(0, Math.floor(milliseconds / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total / 60) % 60);
  const seconds = total % 60;

  return `${hours}:${two(minutes)}:${two(seconds)}`;
}

/** The local clock time of a timestamp from the log, as "HH:MM". */
export function clockTime(timestamp: string): string {
  const at = new Date(timestamp);
  return Number.isNaN(at.getTime()) ? "--:--" : `${two(at.getHours())}:${two(at.getMinutes())}`;
}

/**
 * The colour of a log line, as one of the tag tones.
 *
 * A catch wears the accent, because it is the one thing Leash did itself. A
 * service that stayed down, which is to say a block, wears green. A notice is
 * grey, and anything that failed is red or amber. This keeps held, caught and
 * notified apart at a glance, which green for all three does not.
 */
export function entryTone(entry: Entry): Tone {
  if (entry.action === "notify") {
    return "dim";
  }
  if (entry.result === "failure") {
    return "error";
  }
  if (entry.action === "stop" && entry.result === "success" && entry.event !== "manual") {
    return "accent";
  }
  if (
    entry.result === "not_elevated" ||
    entry.result === "gave_up" ||
    entry.result === "no_facts"
  ) {
    return "warn";
  }
  return "ok";
}

/** What happened, in one short line, for the feed and the log. */
export function entryText(entry: Entry): string {
  switch (entry.action) {
    case "stop":
      return entry.event === "manual"
        ? `${entry.service} was stopped by hand`
        : `${entry.service} tried to start`;
    case "start":
      return `${entry.service} was started by hand`;
    case "notify":
      return `${entry.service} started`;
    case "dry_run":
      return `${entry.service} started`;
    default:
      return `${entry.service} changed`;
  }
}

/** The result word for a log line, the way the design reference words them. */
export function entryResult(entry: Entry): string {
  if (entry.action === "stop") {
    if (entry.event === "manual") {
      return entry.result === "success" ? "Done" : "Failed";
    }
    return entry.result === "success" ? "Caught" : "Failed";
  }
  if (entry.action === "start") {
    return entry.result === "success" ? "Started" : "Failed";
  }
  if (entry.action === "dry_run") {
    return "Would stop";
  }
  if (entry.action === "notify") {
    return "Notified";
  }

  switch (entry.result) {
    case "blocked":
      return "Held";
    case "gave_up":
      return "Gave up";
    case "not_elevated":
      return "No rights";
    case "no_facts":
      return "Unknown";
    default:
      return "Logged";
  }
}

/**
 * The last seven days, ending today, with the counts the engine recorded.
 *
 * A day with no catches still gets a bar, which is why the days are walked
 * rather than taken from the counts alone.
 */
export function lastSevenDays(weekly: DayCount[]): DayCount[] {
  const counts = new Map(weekly.map((day) => [day.date, day.count]));
  const today = new Date();
  const days: DayCount[] = [];

  for (let back = 6; back >= 0; back--) {
    const at = new Date(today.getFullYear(), today.getMonth(), today.getDate() - back);
    const date = dateKey(at);
    days.push({ date, count: counts.get(date) ?? 0 });
  }

  return days;
}

/** The single letter for a day, the way the design reference writes them. */
export function weekdayLetter(date: string): string {
  const at = new Date(`${date}T12:00:00`);
  return Number.isNaN(at.getTime()) ? "?" : ("SMTWTFS"[at.getDay()] ?? "?");
}

/** A local day as `YYYY-MM-DD`, which is how the engine keys its counts. */
function dateKey(at: Date): string {
  return `${at.getFullYear()}-${two(at.getMonth() + 1)}-${two(at.getDate())}`;
}

/** Two digits, for times. */
function two(value: number): string {
  return value.toString().padStart(2, "0");
}
