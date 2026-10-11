//! The app itself: the engine behind Tauri, the tray, and the events it sends to
//! the UI.

mod commands;
mod state;
mod tray;

use leash_engine::watchdog::WatchEvent;
use state::{AppState, CaughtEvent, ErrorEvent, NotifiedEvent, StateChangedEvent};
use std::sync::mpsc::Receiver;
use tauri::{AppHandle, Emitter, WindowEvent};

/// Events the UI listens for, as section 10 names them.
const SERVICE_CAUGHT: &str = "service_caught";
const SERVICE_NOTIFIED: &str = "service_notified";
const STATE_CHANGED: &str = "state_changed";
const ENGINE_ERROR: &str = "engine_error";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (events, received) = std::sync::mpsc::channel::<WatchEvent>();

    let app_state = match AppState::new(events) {
        Ok(app_state) => app_state,
        Err(error) => {
            // The safety lists are built into the exe, so this means the build
            // is broken. Leash must not run without its deny list.
            eprintln!("Leash cannot start: {error}");
            std::process::exit(1);
        }
    };

    tauri::Builder::default()
        // This runs first, so a second launch only shows the window of the one
        // that is already running, and then ends.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(move |app| {
            forward_events(app.handle().clone(), received)?;
            tray::create(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window hides it. The tray and the engine keep running,
            // and Exit in the tray menu is the way out.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::list_services,
            commands::add_monitor,
            commands::remove_monitor,
            commands::set_policy,
            commands::stop_now,
            commands::start_now,
            commands::set_paused,
            commands::rearm,
            commands::get_log,
            commands::get_settings,
            commands::set_settings,
            commands::add_never_touch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Hand the engine events to the window. The channel closes when the app ends,
/// and then this thread ends with it.
fn forward_events(app: AppHandle, events: Receiver<WatchEvent>) -> Result<(), std::io::Error> {
    std::thread::Builder::new()
        .name("leash-events".to_string())
        .spawn(move || {
            while let Ok(event) = events.recv() {
                match event {
                    WatchEvent::Caught { service, outcome } => {
                        let _ = app.emit(SERVICE_CAUGHT, CaughtEvent { service, outcome });
                    }
                    WatchEvent::Notified { service } => {
                        let _ = app.emit(
                            SERVICE_NOTIFIED,
                            NotifiedEvent {
                                service,
                                dry_run: false,
                            },
                        );
                    }
                    WatchEvent::WouldStop { service } => {
                        let _ = app.emit(
                            SERVICE_NOTIFIED,
                            NotifiedEvent {
                                service,
                                dry_run: true,
                            },
                        );
                    }
                    WatchEvent::GaveUp { service } => {
                        let _ = app.emit(
                            STATE_CHANGED,
                            StateChangedEvent {
                                service: Some(service),
                                kind: Some("gave_up"),
                            },
                        );
                    }
                    WatchEvent::Failed { service, error } => {
                        let _ = app.emit(ENGINE_ERROR, ErrorEvent { service, error });
                    }
                }
            }
        })
        .map(|_| ())
}
