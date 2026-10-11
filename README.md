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

The window opens on Home. The left rail tells you whether the app holds
administrator rights, and that is the first thing to look at. Without them,
stops will be refused.

To test real stops, run the same command from an elevated PowerShell.

## Building it

    pnpm tauri build

That writes an NSIS installer for one machine under
`src-tauri/target/release/bundle/nsis/`.

The step has not run on this machine. The NSIS download times out on the network
here, so no installer exists yet. Two things wait on it: a scheduled task that
starts Leash elevated at logon, and Windows notifications, which may need the
Start menu shortcut an installer creates.

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
single-instance lock, and the notices. A command that touches the machine is
asynchronous, because waiting on Windows must not freeze the window.

`src` is the interface, in React 19, Tailwind 4, Vite 8 and TypeScript 6. The
two fonts, Bricolage Grotesque and JetBrains Mono, are served from this
repository along with their OFL licences.

## Checks

All of these pass:

    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    pnpm lint
    pnpm test
    npx tsc --noEmit
    npx vite build

The counts are 102 tests in the engine, 21 in the app crate and 47 over the
interface and the shared code. Of those, the 22 engine tests and 9 app tests
that need a real machine are ignored.

A commit cannot carry a new lint finding. lefthook runs Biome over the staged
frontend files first, and a warning fails the hook as well as an error.

Two scripts reach the parts a unit test cannot. `scripts/elevated-test.ps1`
builds a throwaway service from a Windows binary, stops it five times, and
checks that Leash gives up at the cap. Run it from an administrator shell.
Nothing in this repository stops a service that Windows needs.

`scripts/preview.mjs` puts the built interface in a browser with the engine
stubbed, which is how the layout is checked without opening the app.
`pnpm preview:serve` serves it at http://127.0.0.1:4599, and the address hash
picks the page. `#services-sheet` opens a service sheet, `#home-tip` holds the
pointer over a dot, and `#home-scroll` scrolls the work area to the bottom.

## What is not finished

- The installer and the scheduled task, as above. Until they exist, Leash wants
  an elevated shell.
- The tray, the single-instance lock, and close-to-tray have run by hand only.
  No test covers them.
- Stop counts live in memory. Changing the watch list rebuilds the watchdog and
  clears them, so a `gave_up` badge can vanish when you add another service.
- The daily counts are keyed by UTC date. A catch late in the evening in India
  lands on the previous day's bar.
- There is no licence file yet. The two fonts are the exception, and theirs sit
  beside them in `public/fonts/`.
