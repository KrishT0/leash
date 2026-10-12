//! The Tauri commands. Each one checks its input, calls the state, and returns.
//!
//! The commands stay thin: the rules live in the state and in the engine. Input is
//! checked in Rust, and Tauri checks the types before the body runs, so the interface
//! is never trusted.
//!
//! A command that reads the machine or waits on it is `async`, which keeps it off the
//! main thread. A stop can wait a minute, and that must not freeze the window. A
//! command that changes what Leash is doing refreshes the tray at the end.

use crate::state::{AppState, ServiceView, Snapshot, request_restart_window};
use crate::task;
use crate::tray;
use leash_engine::config::{General, Policy};
use leash_engine::log::Entry;
use leash_engine::scm::{StartOutcome, StopOutcome};
use tauri::{AppHandle, State};

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

/// One service, with the description and the dependents the sheet shows.
#[tauri::command(async)]
pub fn get_service(state: State<'_, AppState>, name: String) -> Result<ServiceView, String> {
    state.service(&name)
}

/// Put a service on the leash. It starts on the safest policy.
#[tauri::command(async)]
pub fn add_monitor(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<(), String> {
    state.add_monitor(&name)?;
    tray::refresh(&app);
    Ok(())
}

/// Take a service off the leash.
#[tauri::command(async)]
pub fn remove_monitor(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    state.remove_monitor(&name)?;
    tray::refresh(&app);
    Ok(())
}

/// Change the policy of a service that is on the leash.
#[tauri::command(async)]
pub fn set_policy(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    policy: Policy,
) -> Result<(), String> {
    state.set_policy(&name, policy)?;
    tray::refresh(&app);
    Ok(())
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
pub fn set_paused(app: AppHandle, state: State<'_, AppState>, paused: bool) -> Result<(), String> {
    state.set_paused(paused)?;
    tray::refresh(&app);
    Ok(())
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
pub fn set_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: General,
) -> Result<(), String> {
    state.set_settings(settings)?;
    tray::refresh(&app);
    Ok(())
}

/// Add a service to the list Leash never touches.
#[tauri::command(async)]
pub fn add_never_touch(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    state.add_never_touch(&name)?;
    tray::refresh(&app);
    Ok(())
}

/// Start Leash again with administrator rights, and step aside.
///
/// Section 6 gives the banner this way out. The scheduled task is asked for two
/// seconds from now, so this process has gone and released the single-instance
/// lock by the time the new copy starts.
#[tauri::command(async)]
pub fn restart_as_admin(app: AppHandle) -> Result<(), String> {
    task::restart_as_admin()?;

    // Written only once the task is on its way, so a restart that could not be
    // asked for leaves nothing behind. The task starts Leash with --autostart,
    // which hides the window, and this is the request that brings it back.
    request_restart_window();

    app.exit(0);
    Ok(())
}
