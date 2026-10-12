//! The scheduled task that starts Leash at logon with administrator rights.
//!
//! The installer registers the task and the uninstaller removes it, both from
//! `installer/hooks.nsh`, using the same name this module uses. This is the
//! app's side of it: the Settings switch that starts Leash with Windows enables
//! the task, and turning the switch off disables it.
//!
//! A task with the highest available rights starts a program without a prompt.
//! A Run registry key cannot elevate at all, and a manifest asking for
//! administrator rights would show a prompt at every launch.

use std::process::Command;

/// The task name in Task Scheduler. `installer/hooks.nsh` uses the same one, and
/// registers and deletes by it.
pub const TASK_NAME: &str = "Leash";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The flag that turns the task on or off.
fn change_flag(enabled: bool) -> &'static str {
    if enabled { "/ENABLE" } else { "/DISABLE" }
}

/// Run schtasks, and give back its exit code.
fn run(args: &[&str]) -> Result<i32, String> {
    let mut command = Command::new("schtasks");
    command.args(args);

    // Without this a console window flashes over the app, because Leash has no
    // console of its own.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command
        .output()
        .map_err(|error| format!("schtasks could not be run: {error}"))?;

    // A program killed by a signal has no code. That cannot happen here, and -1
    // is not a success code either way.
    Ok(output.status.code().unwrap_or(-1))
}

/// True when the task exists, however it is set up.
pub fn exists() -> bool {
    matches!(run(&["/Query", "/TN", TASK_NAME]), Ok(0))
}

/// Ask Windows to start Leash at logon, or to stop doing so.
///
/// A task that is not there is not a failure. During development the app runs
/// from a build folder, where no installer has registered anything.
pub fn set_start_with_windows(enabled: bool) -> Result<(), String> {
    if !exists() {
        return Ok(());
    }

    let code = run(&["/Change", "/TN", TASK_NAME, change_flag(enabled)])?;
    if code != 0 {
        let verb = if enabled { "enable" } else { "disable" };
        return Err(format!(
            "Windows would not {verb} the Leash scheduled task (schtasks returned {code})."
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_ask_windows_for_the_change_the_switch_wants() {
        assert_eq!(change_flag(true), "/ENABLE");
        assert_eq!(change_flag(false), "/DISABLE");
    }

    #[test]
    fn should_name_the_task_that_the_installer_registers() {
        // installer/hooks.nsh registers and deletes this name. A rename here
        // without one there would leave an orphan task behind on uninstall.
        assert_eq!(TASK_NAME, "Leash");
    }
}
