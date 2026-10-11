import { describe, expect, it } from "vitest";
import { clockTime, duration, entryIsOk, entryResult, entryText, lastSevenDays, weekdayLetter } from "./format";
import type { Entry } from "./types";

function entry(part: Partial<Entry>): Entry {
  return {
    ts: "2026-10-11T18:58:02Z",
    service: "Spooler",
    event: "started",
    action: null,
    result: null,
    error: null,
    ...part,
  };
}

describe("duration", () => {
  it("should write hours, minutes and seconds", () => {
    expect(duration(0)).toBe("0:00:00");
    expect(duration(59_000)).toBe("0:00:59");
    expect(duration(60_000)).toBe("0:01:00");
    expect(duration(3_661_000)).toBe("1:01:01");
    expect(duration(45_296_000)).toBe("12:34:56");
  });

  it("should not go backwards on a clock that jumped", () => {
    expect(duration(-5000)).toBe("0:00:00");
  });
});

describe("clockTime", () => {
  it("should show the local time of a log line", () => {
    // Built from local parts, so the test does not depend on the time zone.
    const at = new Date(2026, 9, 11, 18, 58, 2);
    expect(clockTime(at.toISOString())).toBe("18:58");
  });

  it("should say nothing rather than nonsense for a broken timestamp", () => {
    expect(clockTime("not a time")).toBe("--:--");
  });
});

describe("lastSevenDays", () => {
  it("should give seven days ending today", () => {
    const days = lastSevenDays([]);
    expect(days).toHaveLength(7);

    const today = new Date();
    const key = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(
      today.getDate(),
    ).padStart(2, "0")}`;
    expect(days[6].date).toBe(key);
  });

  it("should use the counts the engine recorded", () => {
    const today = new Date();
    const key = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(
      today.getDate(),
    ).padStart(2, "0")}`;

    const days = lastSevenDays([{ date: key, count: 4 }]);
    expect(days[6]).toEqual({ date: key, count: 4 });
  });

  it("should give a bar with no catches a count of zero", () => {
    expect(lastSevenDays([]).every((day) => day.count === 0)).toBe(true);
  });
});

describe("weekdayLetter", () => {
  it("should give the letter the design reference uses", () => {
    // 2026-10-11 is a Sunday.
    expect(weekdayLetter("2026-10-11")).toBe("S");
    expect(weekdayLetter("2026-10-12")).toBe("M");
    expect(weekdayLetter("2026-10-17")).toBe("S");
  });

  it("should not fall over on a broken date", () => {
    expect(weekdayLetter("nothing")).toBe("?");
  });
});

describe("entryText", () => {
  it("should describe an automatic catch", () => {
    expect(entryText(entry({ action: "stop", result: "success" }))).toBe(
      "Spooler tried to start",
    );
  });

  it("should describe a notice and a dry run", () => {
    expect(entryText(entry({ action: "notify", result: "notified" }))).toBe("Spooler started");
    expect(entryText(entry({ action: "dry_run", result: "would_stop" }))).toBe(
      "Spooler started",
    );
  });

  it("should say when the user did it by hand", () => {
    expect(entryText(entry({ event: "manual", action: "stop", result: "success" }))).toBe(
      "Spooler was stopped by hand",
    );
    expect(entryText(entry({ event: "manual", action: "start", result: "success" }))).toBe(
      "Spooler was started by hand",
    );
  });
});

describe("entryResult", () => {
  it("should call a caught service caught", () => {
    expect(entryResult(entry({ action: "stop", result: "success" }))).toBe("Caught");
  });

  it("should call a hand made stop done", () => {
    expect(entryResult(entry({ event: "manual", action: "stop", result: "success" }))).toBe(
      "Done",
    );
  });

  it("should name the two policies that do not stop", () => {
    expect(entryResult(entry({ action: "notify" }))).toBe("Notified");
    expect(entryResult(entry({ action: "dry_run" }))).toBe("Would stop");
  });

  it("should name the reasons Leash took no action", () => {
    expect(entryResult(entry({ action: null, result: "blocked" }))).toBe("Held");
    expect(entryResult(entry({ action: null, result: "gave_up" }))).toBe("Gave up");
    expect(entryResult(entry({ action: null, result: "not_elevated" }))).toBe("No rights");
    expect(entryResult(entry({ action: null, result: "no_facts" }))).toBe("Unknown");
  });

  it("should mark a failed action as failed", () => {
    expect(entryResult(entry({ action: "stop", result: "failure" }))).toBe("Failed");
    expect(entryResult(entry({ action: "start", result: "failure" }))).toBe("Failed");
  });
});

describe("entryIsOk", () => {
  it("should colour a catch and a blocked service green", () => {
    expect(entryIsOk(entry({ action: "stop", result: "success" }))).toBe(true);
    expect(entryIsOk(entry({ action: null, result: "blocked" }))).toBe(true);
  });

  it("should colour a notice, a dry run and a failure amber", () => {
    expect(entryIsOk(entry({ action: "notify" }))).toBe(false);
    expect(entryIsOk(entry({ action: "dry_run" }))).toBe(false);
    expect(entryIsOk(entry({ action: "stop", result: "failure" }))).toBe(false);
  });
});
