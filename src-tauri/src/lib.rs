//! The app itself: the engine behind Tauri, the tray, the notices, and the
//! events the UI listens for.

mod commands;
mod state;
mod tray;

use leash_engine::watchdog::WatchEvent;
use state::{AppState, CaughtEvent, ErrorEvent, NotifiedEvent, StateChangedEvent};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

/// Events the UI listens for, as section 10 names them.
const SERVICE_CAUGHT: &str = "service_caught";
const SERVICE_NOTIFIED: &str = "service_notified";
const STATE_CHANGED: &str = "state_changed";
const ENGINE_ERROR: &str = "engine_error";

/// How long a service is left quiet after a notice. Section 11 asks for
/// repeated events to be grouped, and a service that flaps can fire often.
const NOTICE_WINDOW: Duration = Duration::from_secs(60);

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
        .plugin(tauri_plugin_notification::init())
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
            commands::get_service,
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

/// Hand the engine events to the window, and show a notice for the events that
/// are meant to reach the user. The channel closes when the app ends, and then
/// this thread ends with it.
fn forward_events(app: AppHandle, events: Receiver<WatchEvent>) -> Result<(), std::io::Error> {
    std::thread::Builder::new()
        .name("leash-events".to_string())
        .spawn(move || {
            let mut notices = Notices::default();

            while let Ok(event) = events.recv() {
                match event {
                    WatchEvent::Caught { service, outcome } => {
                        let _ = app.emit(SERVICE_CAUGHT, CaughtEvent { service, outcome });
                    }
                    WatchEvent::Notified { service } => {
                        show_notice(&app, &mut notices, &service, false);
                        let _ = app.emit(
                            SERVICE_NOTIFIED,
                            NotifiedEvent {
                                service,
                                dry_run: false,
                            },
                        );
                    }
                    WatchEvent::WouldStop { service } => {
                        show_notice(&app, &mut notices, &service, true);
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

/// Show a Windows notice for a service that started and was only watched.
///
/// Nothing is shown when the user turned notices off, and a service that fires
/// again inside the window is held back and counted instead.
fn show_notice(app: &AppHandle, notices: &mut Notices, service: &str, dry_run: bool) {
    if !app.state::<AppState>().notifications_on() {
        return;
    }

    let Some(notice) = notices.due(service, dry_run, Instant::now()) else {
        return;
    };
    let (title, body) = notice_text(&notice);

    if let Err(error) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!(%error, "cannot show a notice");
    }
}

/// One notice that is due.
struct Notice {
    service: String,
    /// How many times the service started since the last notice, this one
    /// included.
    count: u32,
    dry_run: bool,
}

/// The notices already shown, and the ones held back since.
#[derive(Default)]
struct Notices {
    seen: HashMap<String, Seen>,
}

struct Seen {
    shown: Instant,
    held: u32,
}

impl Notices {
    /// The notice to show now, or None when this one is held back.
    fn due(&mut self, service: &str, dry_run: bool, now: Instant) -> Option<Notice> {
        match self.seen.entry(service.to_string()) {
            Entry::Vacant(slot) => {
                slot.insert(Seen {
                    shown: now,
                    held: 0,
                });
                Some(Notice {
                    service: service.to_string(),
                    count: 1,
                    dry_run,
                })
            }
            Entry::Occupied(mut slot) => {
                let seen = slot.get_mut();
                if now.duration_since(seen.shown) < NOTICE_WINDOW {
                    seen.held += 1;
                    return None;
                }

                let count = seen.held + 1;
                seen.held = 0;
                seen.shown = now;
                Some(Notice {
                    service: service.to_string(),
                    count,
                    dry_run,
                })
            }
        }
    }
}

/// The title and the body of a notice.
fn notice_text(notice: &Notice) -> (String, String) {
    let body = if notice.count > 1 {
        format!(
            "{} started again {} times in the last minute.",
            notice.service, notice.count
        )
    } else if notice.dry_run {
        format!("{} started. Leash would stop it.", notice.service)
    } else {
        format!("{} started. Leash is watching it.", notice.service)
    };

    ("Leash".to_string(), body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn due(notices: &mut Notices, service: &str, at: Instant) -> Option<Notice> {
        notices.due(service, false, at)
    }

    #[test]
    fn should_show_the_first_notice() {
        let now = Instant::now();
        let mut notices = Notices::default();

        let notice = notices.due("Spooler", false, now).expect("the first one");

        assert_eq!(notice.service, "Spooler");
        assert_eq!(notice.count, 1);
    }

    #[test]
    fn should_hold_a_repeat_back_inside_the_window() {
        let now = Instant::now();
        let mut notices = Notices::default();
        notices.due("Spooler", false, now).expect("the first one");

        assert!(notices.due("Spooler", false, now).is_none());
        assert!(
            notices
                .due(
                    "Spooler",
                    false,
                    now + NOTICE_WINDOW - Duration::from_secs(1)
                )
                .is_none()
        );
    }

    #[test]
    fn should_count_what_it_held_back() {
        let now = Instant::now();
        let mut notices = Notices::default();
        notices.due("Spooler", false, now).expect("the first one");

        // Three more starts inside the window.
        for step in 1..4 {
            assert!(
                notices
                    .due("Spooler", false, now + Duration::from_secs(step))
                    .is_none()
            );
        }

        // The next notice, after the window, says how many there were.
        let later = now + NOTICE_WINDOW;
        let notice = notices.due("Spooler", false, later).expect("the next one");
        assert_eq!(notice.count, 4);

        // And the count starts again.
        let notice = notices
            .due("Spooler", false, later + NOTICE_WINDOW)
            .expect("the one after");
        assert_eq!(notice.count, 1);
    }

    #[test]
    fn should_keep_services_apart() {
        let now = Instant::now();
        let mut notices = Notices::default();
        notices.due("Spooler", false, now).expect("Spooler");

        // Another service is not held back by the first one.
        let notice = due(&mut notices, "DiagTrack", now).expect("DiagTrack");
        assert_eq!(notice.service, "DiagTrack");
    }

    #[test]
    fn should_word_a_single_notice() {
        let notice = Notice {
            service: "Spooler".to_string(),
            count: 1,
            dry_run: false,
        };
        let (title, body) = notice_text(&notice);

        assert_eq!(title, "Leash");
        assert_eq!(body, "Spooler started. Leash is watching it.");
    }

    #[test]
    fn should_word_a_dry_run_as_the_trial_it_is() {
        let notice = Notice {
            service: "Spooler".to_string(),
            count: 1,
            dry_run: true,
        };

        assert_eq!(
            notice_text(&notice).1,
            "Spooler started. Leash would stop it."
        );
    }

    #[test]
    fn should_word_a_burst_once() {
        let notice = Notice {
            service: "Spooler".to_string(),
            count: 3,
            dry_run: false,
        };

        assert_eq!(
            notice_text(&notice).1,
            "Spooler started again 3 times in the last minute."
        );
    }
}
