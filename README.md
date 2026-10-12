# Leash

Leash keeps chosen Windows services stopped. It watches them. When one of them
starts, Leash applies the policy you set for that service.

Windows ships background services that most people never need: Print Spooler,
Windows Search, SysMain, Bluetooth Support, Fax, two Xbox services, the
telemetry collector. Something asks for one of them, Windows starts it, and it
stays up all day. The usual fix is to set the service to Disabled. The usual
price is an app or a device that stops working, and an evening spent working out
why.

Leash takes another route. The service may start. Leash stops it once it is up.
A printer driver can still wake the spooler to print, and the spooler does not
stay resident afterwards.

It is not a process killer, and it does not disable anything.

## What happens when a service starts

1. One watchdog thread holds a Windows status notification for each monitored
   service. It re-arms that notification before doing any work, because a stop
   can take seconds and a service that returns in that time must not be missed.
2. Windows reports a change. Leash reads the service. Only a running service
   gets the policy. A stop, or a service on its way down, passes through.
3. The safety check runs again, right before the action. Without the facts,
   Leash does not act.
4. The decision brings together the safety level, your policy, the pause switch,
   the administrator token, and the stops already spent on this service inside
   ten minutes.
5. Under the `stop` policy, Leash waits out a backoff, asks Windows to stop the
   service, and waits up to 30 seconds for it.
6. Five stops inside ten minutes and Leash gives up on that service. The log
   records it, the interface shows it, and Leash stands aside until you press
   Try again.

The backoff starts at one second and doubles, up to a minute, so a service that
returns at once is not hammered.

## Safety levels

| Level | What it means |
| --- | --- |
| Blocked | Leash never stops it. The interface does not offer the monitor switch. |
| Critical | Stopping it asks first, and lists the services that depend on it. |
| Normal | It may be monitored. |

The lists that drive this are compiled into the executable.
`crates/engine/data/deny.json` holds five names Leash never stops: RpcSs,
DcomLaunch, EventLog, WinDefend and mpssvc. `crates/engine/data/sensitive.json`
holds two that ask first: wuauserv and Audiosrv.

A service is blocked as well when it refuses stop commands, when it is a driver
rather than a service, or when it is not a Win32 service at all. A service that
other running services depend on comes back critical rather than blocked, so you
can see what you are about to disturb.

`crates/engine/data/suggested.json` holds eight services worth stopping, each
with a reason and a plain sentence about what breaks. Those are the ones offered
when you add a service.

The never-touch list is yours, and it lives in the config file. It can only add
to a refusal. It cannot unlock a name on the deny list.

## Policies

A service you add starts on `notify`. That policy never touches the service. It
writes a line to the log, and raises a Windows notification when notices are on.
`dry_run` writes down what a stop would have done. `stop` does it.

## What it never does

- It never stops a service from the deny list, and no setting unlocks one.
- It never changes how a service starts. Windows still starts these services.
  Leash takes them down again.
- It never stops a service that was already running when the watchdog started.
  It acts on changes.
- It never starts a service on its own. The Start now button in the interface is
  the only path to that.
- It never passes a service name to a shell. Names are checked when the config
  is read, and the safety check runs again before any action.
- No Windows call in the engine sits outside `scm.rs` and `elevation.rs`.

## Requirements

Windows 11. Windows 10 has not been tried, and WebView2 is already present on
Windows 11.

Administrator rights are needed to stop a service. Without them the app still
runs, still logs, and still shows everything. A stop is refused, and the log
says `not_elevated`.

Building needs Rust, Node and pnpm.

## Running it

    pnpm install
    pnpm tauri dev

Leash starts without a window. The same program is what the scheduled task
starts at logon, and Start minimized to tray is on by default, so look for the
tray icon and then Open. The left rail tells you whether the app holds
administrator rights, which is the first thing to check. Without them, stops are
refused.

To have the window appear at once, turn off Start minimized to tray in Settings,
or set `start_minimized = false` in the config file.

To test real stops, run the same command from an elevated PowerShell.

## Building it

    pnpm tauri build

That writes an NSIS installer for one machine under
`src-tauri/target/release/bundle/nsis/`.

The installer now does three things beyond copying the program. It asks for
administrator rights once, at install time. It registers the scheduled task that
starts Leash at logon with the highest available rights, so no prompt appears at
logon, and it removes that task again on uninstall. It creates the Start menu
shortcut, which is also what lets a Windows notification carry the app id.

The installer has been run on this machine. The hook inside it registered the
task against `C:\Program Files\Leash\leash.exe`, and
`scripts/install-check.ps1` is what proves that. The logon start itself has not
been watched yet, because it needs a log off and back on.

## Where it keeps things

| Path | What lives there |
| --- | --- |
| `%APPDATA%\Leash\config.toml` | your switches, your never-touch list, and the services you monitor |
| `%LOCALAPPDATA%\Leash\logs` | one file a day, one JSON object a line, another part after 1 MB, kept for 30 days |
| `%LOCALAPPDATA%\Leash\stats.json` | catches a day, which the seven-day bars on Home read |

The config file is untrusted input. Any process can edit it, so every entry is
validated when it is read.

## The window

Four pages. Home carries the orbit: one dot per monitored service, a dot flies
out to the rim when Leash catches that service, and the middle holds the count
for today. The dots answer the pointer and the keyboard, and a click opens the
service sheet.

Services holds the search, the three views, and the sheets for adding,
filtering and inspecting. Activity is every line the engine has written,
including the moments Leash chose to do nothing. Settings holds the elevation
status, the platform, and the switches.

The window is one surface. A card never holds another card.

Close the window and Leash stays in the tray. Start it a second time and the
first window comes forward instead of a second one opening. The tray dot carries
the state: green while watching, amber while paused, red without administrator
rights, grey when the watchdog is off.

## How the code is arranged

`crates/engine` holds the rules and the Windows calls. It contains no Tauri
code. The safety lists, the config file, the activity log, the service calls and
the watchdog are separate modules, and the decision half of the watchdog makes
no Windows calls at all. That is why most tests run anywhere: 80 of the 102
engine tests need no service to watch. The other 22 talk to the real service
manager, start a real watch thread, or want an elevated shell and the test
service. They are marked ignored.

`src-tauri` is the app: 14 commands, the state behind them, the tray, the
single-instance lock, the notices, and the scheduled task. A command that
touches the machine is asynchronous, because waiting on Windows must not freeze
the window. `installer/hooks.nsh` is what the installer runs to register the
task and to remove it, and `installer/leash-task.xml` holds the same task in a
form a person can read.

`src` is the interface, in React 19, Tailwind 4, Vite 8 and TypeScript 6. The
two fonts, Bricolage Grotesque and JetBrains Mono, are served from this
repository along with their OFL licences.

## Checks

All of these pass:

    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo test --workspace
    pnpm lint
    pnpm test
    npx tsc --noEmit
    npx vite build

The counts are 102 tests in the engine, 23 in the app crate and 47 over the
interface and the shared code. Of those, the 22 engine tests and 9 app tests
that need a real machine are ignored.

A commit cannot carry a new lint finding. lefthook runs Biome over the staged
frontend files first, and a warning fails the hook as well as an error.

Five scripts reach the parts a unit test cannot. `scripts/elevated-test.ps1`
builds a throwaway service from a Windows binary, stops it five times, and
checks that Leash gives up at the cap. `scripts/task-test.ps1` registers the
scheduled task, prints what Task Scheduler stored, and starts Leash through it,
which is how the no-prompt start is checked. `scripts/install-check.ps1` reads
what an install left behind, including whether the installer's task hook ran.
The first two want an administrator shell. Nothing in this repository stops a
service that Windows needs.

`scripts/tray-marks.py` draws the tray icon at the four sizes Windows asks for,
and writes the two pages that show them: one for the four that ship, one for the
weights they were chosen over.

`scripts/preview.mjs` puts the built interface in a browser with the engine
stubbed, which is how the layout is checked without opening the app.
`pnpm preview:serve` serves it at http://127.0.0.1:4599, and the address hash
picks the page. `#services-sheet` opens a service sheet, `#home-tip` holds the
pointer over a dot, and `#home-scroll` scrolls the work area to the bottom.

## What is not finished

- One installation has been made, and its task hook verified. The hook that runs
  during the install wrote the task against the installed program, which is the
  part no script can reach from a build folder. The uninstaller's half of the
  hook, which deletes the task, has not been run.
- The logon start has not been watched yet. Everything in front of it checks
  out, and the trigger fires at logon, but that step needs a log off.
- The scheduled task is verified. Task Scheduler keeps the highest available
  rights, a logon trigger for the installing account, no time limit, one
  instance at a time, and the battery settings off. Starting it brings up Leash
  with no prompt, and the process runs at a higher integrity level than a normal
  program, which is what administrator rights look like from outside.
- The tray, the single-instance lock, and close-to-tray have run by hand only.
  No test covers them.
- Stop counts live in memory. Changing the watch list rebuilds the watchdog and
  clears them, so a `gave_up` badge can vanish when you add another service.
- The daily counts are keyed by UTC date. A catch late in the evening in India
  lands on the previous day's bar.
- There is no licence file yet. The two fonts are the exception, and theirs sit
  beside them in `public/fonts/`.
