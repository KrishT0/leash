import { describe, expect, it } from "vitest";
import { DEFAULT_FILTERS, activeFilters, filterServices, policyLabel } from "./services";
import type { ServiceView } from "./types";

function service(part: Partial<ServiceView>): ServiceView {
  return {
    name: "Spooler",
    display_name: "Print Spooler",
    description: "",
    status: "running",
    startup: "automatic",
    accepts_stop: true,
    is_driver: false,
    is_win32: true,
    third_party: false,
    pid: 100,
    verdict: { level: "normal", reason: "none" },
    verdict_text: "",
    policy: null,
    gave_up: false,
    dependents: [],
    suggested: null,
    ...part,
  };
}

const suggested = service({
  name: "DiagTrack",
  display_name: "Connected User Experiences and Telemetry",
  suggested: {
    name: "DiagTrack",
    display: "Connected User Experiences and Telemetry",
    reason: "Sends diagnostic data to Microsoft",
    effect: "No effect on daily use",
    risk: "low",
  },
});

const thirdParty = service({ name: "Steam", display_name: "Steam Client", third_party: true });
const blocked = service({
  name: "RpcSs",
  display_name: "Remote Procedure Call",
  verdict: { level: "blocked", reason: "deny_list" },
});
const stopped = service({ name: "Fax", display_name: "Fax", status: "stopped", startup: "manual" });
const monitored = service({ name: "WSearch", display_name: "Windows Search", policy: "stop" });

const all = [suggested, thirdParty, blocked, stopped, monitored];

describe("filterServices", () => {
  it("should show only suggested services on the suggested view", () => {
    const rows = filterServices(all, "suggested", "", DEFAULT_FILTERS);
    expect(rows.map((row) => row.name)).toEqual(["DiagTrack"]);
  });

  it("should show only third party services on that view", () => {
    const rows = filterServices(all, "third_party", "", DEFAULT_FILTERS);
    expect(rows.map((row) => row.name)).toEqual(["Steam"]);
  });

  it("should hide blocked services unless the switch is on", () => {
    expect(filterServices(all, "all", "", DEFAULT_FILTERS).some((row) => row.name === "RpcSs")).toBe(
      false,
    );
    expect(
      filterServices(all, "all", "", { ...DEFAULT_FILTERS, blocked: true }).some(
        (row) => row.name === "RpcSs",
      ),
    ).toBe(true);
  });

  it("should search both names", () => {
    // The display name, the internal name, and trailing space.
    expect(
      filterServices(all, "all", "telemetry", DEFAULT_FILTERS).map((row) => row.name),
    ).toEqual(["DiagTrack"]);
    expect(filterServices(all, "all", "wsearch", DEFAULT_FILTERS).map((row) => row.name)).toEqual([
      "WSearch",
    ]);
    expect(filterServices(all, "all", "  FAX  ", DEFAULT_FILTERS).map((row) => row.name)).toEqual([
      "Fax",
    ]);
  });

  it("should filter by status", () => {
    expect(
      filterServices(all, "all", "", { ...DEFAULT_FILTERS, status: "stopped" }).map(
        (row) => row.name,
      ),
    ).toEqual(["Fax"]);
    expect(
      filterServices(all, "all", "", { ...DEFAULT_FILTERS, status: "running" }).map(
        (row) => row.name,
      ),
    ).toEqual(["DiagTrack", "Steam", "WSearch"]);
  });

  it("should filter by monitoring", () => {
    expect(
      filterServices(all, "all", "", { ...DEFAULT_FILTERS, monitoring: "monitored" }).map(
        (row) => row.name,
      ),
    ).toEqual(["WSearch"]);
    expect(
      filterServices(all, "all", "", { ...DEFAULT_FILTERS, monitoring: "ignored" }).map(
        (row) => row.name,
      ),
    ).toEqual(["DiagTrack", "Steam", "Fax"]);
  });

  it("should filter by startup type", () => {
    expect(
      filterServices(all, "all", "", { ...DEFAULT_FILTERS, startup: "manual" }).map(
        (row) => row.name,
      ),
    ).toEqual(["Fax"]);
  });

  it("should keep the order the engine gave", () => {
    const rows = filterServices(all, "all", "", { ...DEFAULT_FILTERS, blocked: true });
    expect(rows.map((row) => row.name)).toEqual([
      "DiagTrack",
      "Steam",
      "RpcSs",
      "Fax",
      "WSearch",
    ]);
  });
});

describe("activeFilters", () => {
  it("should count nothing at the defaults", () => {
    expect(activeFilters(DEFAULT_FILTERS)).toBe(0);
  });

  it("should count each filter that is set", () => {
    expect(activeFilters({ ...DEFAULT_FILTERS, status: "running" })).toBe(1);
    expect(activeFilters({ ...DEFAULT_FILTERS, blocked: true })).toBe(1);
    expect(
      activeFilters({ status: "running", monitoring: "monitored", startup: "manual", blocked: true }),
    ).toBe(4);
  });
});

describe("policyLabel", () => {
  it("should write the policies the way the design does", () => {
    expect(policyLabel("notify")).toBe("notify");
    expect(policyLabel("dry_run")).toBe("dry run");
    expect(policyLabel("stop")).toBe("stop");
    expect(policyLabel(null)).toBe("");
  });
});
