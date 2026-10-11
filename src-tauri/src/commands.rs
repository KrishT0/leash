//! The Tauri commands. Each one checks its input, calls the state, and returns.
//!
//! The commands stay thin, as section 5 asks: the rules live in the state and in
//! the engine. Every input is checked in Rust, and Tauri checks the types before
//! the body runs, so the UI is never trusted.
//!
//! Commands that read the machine or wait on it are marked `async`, which runs
//! them off the main thread. A stop can wait for a service for up to a minute,
//! and that must not freeze the window.

use crate::state::{AppState, ServiceView, Snapshot};
use leash_engine::config::{General, Policy};
use leash_engine::log::Entry;
use leash_engine::scm::{StartOutcome, StopOutcome};
use tauri::State;

/// The whole state the UI shows.
#[tauri::command]
pub fn get_state(state: State<'_, AppState>) -> Result<Snapshot, String> {
    state.snapshot()
}

/// Every service on the machine, with the safety check and the policy.
#[tauri::command(async)]
pub fn list_services(state: State<'_, AppState>) -> Result<Vec<ServiceView>, String> {
    state.services()
}

/// Put a service on the leash. It starts on the safest policy.
#[tauri::command(async)]
pub fn add_monitor(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state.add_monitor(&name)
}

/// Take a service off the leash.
#[tauri::command(async)]
pub fn remove_monitor(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state.remove_monitor(&name)
}

/// Change the policy of a service that is on the leash.
#[tauri::command(async)]
pub fn set_policy(state: State<'_, AppState>, name: String, policy: Policy) -> Result<(), String> {
    state.set_policy(&name, policy)
}

/// Stop a service now, because the user asked.
#[tauri::command(async)]
pub fn stop_now(state: State<'_, AppState>, name: String) -> Result<StopOutcome, String> {
    state.stop_now(&name)
}

/// Start a service now, because the user asked.
#[tauri::command(async)]
pub fn start_now(state: State<'_, AppState>, name: String) -> Result<StartOutcome, String> {
    state.start_now(&name)
}

/// Pause or resume the watchdog.
#[tauri::command(async)]
pub fn set_paused(state: State<'_, AppState>, paused: bool) -> Result<(), String> {
    state.set_paused(paused)
}

/// Let Leash try a service again after it gave up on it.
#[tauri::command(async)]
pub fn rearm(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state.rearm(&name);
    Ok(())
}

/// The newest log lines.
#[tauri::command]
pub fn get_log(state: State<'_, AppState>, limit: usize) -> Result<Vec<Entry>, String> {
    state.log(limit)
}

/// The settings block.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<General, String> {
    Ok(state.snapshot()?.general)
}

/// Change the settings block.
#[tauri::command(async)]
pub fn set_settings(state: State<'_, AppState>, settings: General) -> Result<(), String> {
    state.set_settings(settings)
}

/// Add a service to the list Leash never touches.
#[tauri::command(async)]
pub fn add_never_touch(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state.add_never_touch(&name)
}
