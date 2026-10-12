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

/// How many pings the detached shell waits through before it asks for the task.
///
/// Three pings take about two seconds. That is long enough for this process to
/// exit and release the single-instance lock, and short enough that the window
/// seems to come straight back.
const RESTART_WAIT_PINGS: u32 = 3;

/// The command a detached shell runs to start the task, once this process has
/// gone.
///
/// The task name comes from the constant above and nothing else, so the shell
/// never sees a string this program did not write.
///
/// The redirection matters more than it looks. Without it `ping` prints to the
/// console, and this command hands itself to `cmd`, which only gets a console on
/// screen when it has none of its own.
fn restart_command(pings: u32) -> String {
    format!("ping -n {pings} 127.0.0.1 >nul & schtasks /Run /TN \"{TASK_NAME}\"")
}

/// Ask for Leash to start again through the scheduled task, with rights.
///
/// The task is asked for after this process has gone. A new copy that arrives
/// while the single-instance lock is still held would either exit at once or
/// become a second engine watching the same services.
///
/// A machine with no installer has no task to run, and that is reported rather
/// than ignored, because the button that calls this does nothing until an
/// install has happened.
pub fn restart_as_admin() -> Result<(), String> {
    if !exists() {
        return Err(
            "There is no Leash scheduled task to restart from. Start Leash from a shell \
             running as administrator instead."
                .to_string(),
        );
    }

    let mut command = Command::new("cmd");
    command.args(["/C"]);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        // raw_arg passes the string through exactly as it is written. args()
        // quoted it the way Rust quotes an argument, and cmd does not understand
        // backslash escaping, so the line arrived mangled: the redirection was
        // lost and the schtasks half never ran.
        command.raw_arg(restart_command(RESTART_WAIT_PINGS));

        // A console without a window. DETACHED_PROCESS was here as well and did
        // the opposite of what it sounds like, because cmd allocates a console of
        // its own when it has none, which puts it on screen. The child outlives
        // this process without any flag; Windows does not tie them together.
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
        .spawn()
        .map_err(|error| format!("Windows could not start the restart: {error}"))?;

    Ok(())
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

    #[test]
    fn should_ask_for_the_task_after_a_wait() {
        let command = restart_command(RESTART_WAIT_PINGS);
        assert!(command.contains("/Run"), "{command}");
        assert!(command.contains("-n 3"), "{command}");
        assert!(command.ends_with("\"Leash\""), "{command}");
    }

    #[test]
    fn should_build_a_command_that_shells_out_once() {
        // Nothing from outside reaches this string, so no service name and no
        // path can arrive as a second command.
        let command = restart_command(9);
        assert_eq!(command.matches("schtasks").count(), 1);
        assert_eq!(command.matches('&').count(), 1);
        assert_eq!(command.matches("Leash").count(), 1);
    }
}
