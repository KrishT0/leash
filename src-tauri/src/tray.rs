//! The tray icon: Open, Pause or Resume, Exit, and the colour that carries the
//! state.
//!
//! Section 10 gives the colours: green is active, yellow is paused, red is an
//! error or no administrator rights, and gray is disabled.

use crate::state::{AppState, TrayFlags};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

/// The tray id, used to find the tray again when the state changes.
const TRAY_ID: &str = "leash";

/// The window the tray opens.
const MAIN_WINDOW: &str = "main";

const MENU_OPEN: &str = "open";
const MENU_PAUSE: &str = "pause";
const MENU_EXIT: &str = "exit";

/// The colours from section 11. The tray uses the status tokens.
const OK: (u8, u8, u8) = (0x4A, 0xDE, 0x80);
const WARN: (u8, u8, u8) = (0xFB, 0xBF, 0x24);
const ERROR: (u8, u8, u8) = (0xF8, 0x71, 0x71);
const DISABLED: (u8, u8, u8) = (0x7C, 0x84, 0x93);

/// Build the tray icon and set its first colour.
pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu(app, false)?)
        .tooltip("Leash")
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_OPEN => show_window(app),
            MENU_PAUSE => toggle_pause(app),
            MENU_EXIT => app.exit(0),
            _ => {}
        })
        .build(app)?;

    refresh(app);

    Ok(())
}

/// Set the colour and the menu label from the state. Called after anything that
/// changes whether Leash is watching, paused, or allowed to stop a service.
pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let flags = app.state::<AppState>().tray_flags();
    let (colour, tooltip) = status(flags);

    if let Some(image) = icon(app, colour) {
        let _ = tray.set_icon(Some(image));
    }

    if let Ok(menu) = menu(app, flags.paused) {
        let _ = tray.set_menu(Some(menu));
    }

    let _ = tray.set_tooltip(Some(tooltip));
}

/// Show the window, and put it in front. Used by the tray and by a second
/// launch of the app.
pub fn show_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// The tray menu. The pause item carries the word for the state the app is in.
fn menu<R: Runtime>(app: &AppHandle<R>, paused: bool) -> tauri::Result<Menu<R>> {
    let open = MenuItem::with_id(app, MENU_OPEN, "Open", true, None::<&str>)?;
    let pause = MenuItem::with_id(
        app,
        MENU_PAUSE,
        if paused { "Resume" } else { "Pause" },
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let exit = MenuItem::with_id(app, MENU_EXIT, "Exit", true, None::<&str>)?;

    Menu::with_items(app, &[&open, &pause, &separator, &exit])
}

/// Pause or resume from the tray.
fn toggle_pause<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let paused = !state.tray_flags().paused;

    if let Err(error) = state.set_paused(paused) {
        tracing::warn!(%error, "the tray could not change the pause state");
    }

    refresh(app);
}

/// The tray icon in one colour.
///
/// The app icon is the placeholder until the branding work lands, so the shape
/// stays and the colour carries the meaning. The alpha channel keeps the shape,
/// and every visible pixel takes the colour.
fn icon<R: Runtime>(app: &AppHandle<R>, colour: (u8, u8, u8)) -> Option<Image<'static>> {
    let base = app.default_window_icon()?;

    let mut rgba = base.rgba().to_vec();
    for pixel in rgba.chunks_exact_mut(4) {
        if pixel[3] > 0 {
            pixel[0] = colour.0;
            pixel[1] = colour.1;
            pixel[2] = colour.2;
        }
    }

    Some(Image::new_owned(rgba, base.width(), base.height()))
}

/// The colour and the tooltip for a set of flags.
///
/// The order is section 10's: red wins over the rest, because losing the rights
/// to stop a service is the state the user has to act on.
fn status(flags: TrayFlags) -> ((u8, u8, u8), &'static str) {
    if !flags.elevated {
        (ERROR, "Leash: no administrator rights")
    } else if flags.paused {
        (WARN, "Leash: paused")
    } else if !flags.watching {
        (DISABLED, "Leash: off")
    } else {
        (OK, "Leash: watching")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(elevated: bool, paused: bool, watching: bool) -> TrayFlags {
        TrayFlags {
            elevated,
            paused,
            watching,
        }
    }

    #[test]
    fn should_colour_the_tray_green_while_watching() {
        assert_eq!(status(flags(true, false, true)), (OK, "Leash: watching"));
    }

    #[test]
    fn should_colour_the_tray_yellow_when_paused() {
        assert_eq!(status(flags(true, true, true)), (WARN, "Leash: paused"));
    }

    #[test]
    fn should_colour_the_tray_gray_when_the_watchdog_is_off() {
        assert_eq!(status(flags(true, false, false)), (DISABLED, "Leash: off"));
    }

    #[test]
    fn should_colour_the_tray_red_without_administrator_rights() {
        assert_eq!(
            status(flags(false, false, true)),
            (ERROR, "Leash: no administrator rights")
        );
    }

    #[test]
    fn should_let_losing_the_rights_outrank_the_rest() {
        // Paused, or off, does not hide the fact that stops no longer work.
        assert_eq!(status(flags(false, true, true)).0, ERROR);
        assert_eq!(status(flags(false, false, false)).0, ERROR);
    }
}
