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

    let _ = tray.set_icon(icon(app, colour));

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

/// The tray mark, drawn once for every size Windows asks for.
///
/// A tray icon is 16 pixels across on a normal display and larger on a scaled
/// one. Handing Windows a single large image lets it do the shrinking, and a
/// ring that survives a hand drawn 32 pixel copy turns into a smudge. Each size
/// is drawn on its own by `scripts/tray-marks.py`.
const TRAY_MARKS: [(f64, &[u8]); 4] = [
    (1.0, include_bytes!("../icons/tray-16.png")),
    (1.25, include_bytes!("../icons/tray-20.png")),
    (1.5, include_bytes!("../icons/tray-24.png")),
    (2.0, include_bytes!("../icons/tray-32.png")),
];

/// Which mark to use for a display scale factor.
///
/// The largest size at or below the asking scale wins, and the smallest is the
/// floor, so a display between two sizes gets the sharper mark rather than a
/// stretched one.
fn tray_mark_index(scale: f64) -> usize {
    TRAY_MARKS
        .iter()
        .rposition(|(at, _)| scale + 0.001 >= *at)
        .unwrap_or(0)
}

/// The tray icon in one colour.
///
/// The mark is drawn on transparency, with no tile of its own, because every
/// visible pixel takes the colour of the state.
fn icon<R: Runtime>(app: &AppHandle<R>, colour: (u8, u8, u8)) -> Option<Image<'static>> {
    let scale = app
        .get_webview_window(MAIN_WINDOW)
        .and_then(|window| window.scale_factor().ok())
        .unwrap_or(1.0);

    let base = Image::from_bytes(TRAY_MARKS[tray_mark_index(scale)].1).ok()?;

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
    fn should_read_every_tray_mark() {
        // include_bytes! does not look inside the files, so a mark that is
        // corrupt, or in the wrong format, would only show up in the tray.
        for (scale, bytes) in TRAY_MARKS {
            let image = Image::from_bytes(bytes).expect("a tray mark decodes");
            assert!(image.width() > 0, "at scale {scale}");
            assert_eq!(image.width(), image.height(), "at scale {scale}");
        }
    }

    #[test]
    fn should_draw_the_tray_at_the_size_the_display_asks_for() {
        assert_eq!(tray_mark_index(1.0), 0);
        assert_eq!(tray_mark_index(1.25), 1);
        assert_eq!(tray_mark_index(1.5), 2);
        assert_eq!(tray_mark_index(2.0), 3);
    }

    #[test]
    fn should_fall_back_to_the_nearest_mark_it_has() {
        // Below the smallest and above the largest, and between two. A display
        // between sizes takes the smaller, sharper mark over a stretched one.
        assert_eq!(tray_mark_index(0.9), 0);
        assert_eq!(tray_mark_index(1.1), 0);
        assert_eq!(tray_mark_index(1.4), 1);
        assert_eq!(tray_mark_index(3.0), 3);
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
