# Installer tests

Three cases that cannot be covered by a unit test or by the checks in
`README.md`, because each one needs a real install, a real account, or both.

Run them in order on a Windows 11 machine that already has Leash installed, and
record the result in the table at the end. Anything marked "not run" is exactly
that.

## What the installer does

The NSIS installer asks for administrator rights once, at install time. It
copies the program to `C:\Program Files\Leash`, registers a scheduled task named
`Leash`, creates the Start menu shortcut, and writes the uninstall entry. The
uninstaller ends Leash, deletes the task, and removes the files.

The task is what makes the logon start work without a prompt. It holds the
highest rights available and runs at logon for the account that installed Leash.

`src-tauri/installer/hooks.nsh` is where the task is written and registered, and
where Leash is ended before an install or an uninstall touches its files.

## 1. A standard user who enters an admin password

Sign in as a standard user, run the installer, and choose the option that takes
an administrator's name and password rather than switching user.

What to expect: the install succeeds. The task belongs to the **administrator
account whose credentials were entered**, because that is the account the
installer runs as, and Windows gives a task the rights of the account it runs
under.

That has a consequence worth knowing. Leash can only stop a service with
administrator rights, and a standard user's logon cannot start a process with
rights the user does not have. So for a standard user the logon start either does
nothing or starts Leash without the rights it needs, and the honest behaviour is
the second: the app runs, the banner says it needs rights, and the tray is red.
Automatic stopping needs a logon by an administrator.

Check with:

```
schtasks /Query /TN Leash /V /FO LIST
```

The `Run As User` line names the owner of the task. It should be the account
whose credentials were entered during the install.

## 2. An upgrade

Install an older build, leave the program where it is, then run the installer for
a newer one.

What to expect: one UAC prompt, the same as a first install. The task is
registered again with `/F`, so it is replaced rather than refused, and its
command points at the folder the program is now in. Settings and the activity log
are untouched, because they live in `%APPDATA%` and `%LOCALAPPDATA%`.

Check with:

```
schtasks /Query /TN Leash /XML | findstr Command
```

That line must name the folder the program is installed in now. If an older
install went somewhere else, the task must not still point there.

## 3. An uninstall while Leash is running

Start Leash, leave it running in the tray, then remove it from Installed apps.

What to expect: Leash ends without being asked, the task is deleted, and the
folder is removed. A window close would not be enough, because Leash hides
instead of exiting when its window is closed, so the hooks end the process before
the installer's own running-program check gets a chance to fail.

Check with:

```
schtasks /Query /TN Leash
```

That must report that the task does not exist.

## Results

| Case | Expected | Result |
| --- | --- | --- |
| Standard user with admin credentials | install succeeds, task owned by the administrator whose credentials were entered | not run, this machine has no standard account |
| Upgrade | one prompt, task registered again, command points at the new folder, settings kept | passed, with the two differences noted below |
| Uninstall while running | Leash ends, task deleted, folder removed | passed |

## What was run, and how

A fresh install, 12 October 2026, on a machine with no Leash left on it. The
program, the task definition file, the Start menu shortcut and the uninstall
entry all appeared. The task runs `C:\Program Files\Leash\leash.exe
--autostart`, at the highest level available, with no time limit, and its logon
trigger fires for the account that installed it.

The upgrade case, 12 October. The same build was installed over itself, into the
same folder, with Leash running. The installer ended the running Leash rather than
stalling on it, replaced the task, and left the settings byte for byte the same.
Two things a true upgrade would add are not covered: a different version, and an
install into a different folder.

The uninstall case, 12 October. Leash was running, then the uninstaller ran with
`/S`. Leash ended, and the folder, the task, the shortcut and the uninstall entry
all went. The settings and the activity log stayed. The `/S` flag means the
uninstall wizard was never shown, so its own question about deleting app data was
never seen.

The restart with rights, 12 October, after it had been broken and fixed. The button
was asked for on an unelevated launch. The app left, and two seconds later the task
started an elevated copy, which came back with its window. Checked afterwards: the
task's status was Running with result 267009, the code for a task that is running;
the new process held a higher integrity level than the shell reading it; the window
titled Leash was visible rather than hidden; no console or `ping` process was left
behind; and the activity log recorded `success` for stops, which it could not do
without the rights. Before the fix the same button left the task never run, with
result 267011, and put a `ping` window on screen.

What is still not run: the standard user case, the logon start itself, which needs a
log off and back on, and the restart case above with Leash started by the task at
logon rather than by hand.
