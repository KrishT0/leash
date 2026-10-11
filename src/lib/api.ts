// The only file that calls invoke, as section 5 asks.

import { invoke } from "@tauri-apps/api/core";
import type {
  Entry,
  General,
  Policy,
  ServiceView,
  Snapshot,
  StartOutcome,
  StopOutcome,
} from "./types";

export function getState(): Promise<Snapshot> {
  return invoke<Snapshot>("get_state");
}

export function listServices(): Promise<ServiceView[]> {
  return invoke<ServiceView[]>("list_services");
}

export function addMonitor(name: string): Promise<void> {
  return invoke("add_monitor", { name });
}

export function removeMonitor(name: string): Promise<void> {
  return invoke("remove_monitor", { name });
}

export function setPolicy(name: string, policy: Policy): Promise<void> {
  return invoke("set_policy", { name, policy });
}

export function stopNow(name: string): Promise<StopOutcome> {
  return invoke<StopOutcome>("stop_now", { name });
}

export function startNow(name: string): Promise<StartOutcome> {
  return invoke<StartOutcome>("start_now", { name });
}

export function setPaused(paused: boolean): Promise<void> {
  return invoke("set_paused", { paused });
}

export function rearm(name: string): Promise<void> {
  return invoke("rearm", { name });
}

export function getLog(limit: number): Promise<Entry[]> {
  return invoke<Entry[]>("get_log", { limit });
}

export function getSettings(): Promise<General> {
  return invoke<General>("get_settings");
}

export function setSettings(settings: General): Promise<void> {
  return invoke("set_settings", { settings });
}

export function addNeverTouch(name: string): Promise<void> {
  return invoke("add_never_touch", { name });
}
