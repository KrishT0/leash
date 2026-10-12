//! The app state behind the commands: the config, the watchdog, and the log.
//!
//! The commands stay thin. The rules live here and in the engine.

use leash_engine::config::{Config, General, Monitored, Policy, config_path, validate};
use leash_engine::elevation;
use leash_engine::log::{self, ActivityLog, Entry};
use leash_engine::safety::{SafetyLists, Suggested, Verdict};
use leash_engine::scm::{self, DEFAULT_WAIT, ServiceInfo, StartOutcome, StopOutcome};
use leash_engine::watchdog::{RetryBook, WatchEvent, WatchSettings, Watchdog};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, MutexGuard};

use crate::task;

/// Longest log the UI may ask for in one call.
const MAX_LOG_LIMIT: usize = 1_000;

/// The sentence the UI shows when Leash cannot touch services. Section 6 words
/// the banner this way, so the text lives in one place.
const NEEDS_ADMIN: &str = "Leash needs administrator rights to stop services.";

/// One service, as the UI needs it.
#[derive(Debug, Clone, Serialize)]
pub struct ServiceView {
    #[serde(flatten)]
    pub info: ServiceInfo,
    /// What the safety check says about it.
    pub verdict: Verdict,
    /// The same verdict as a sentence, so the UI never words it itself.
    pub verdict_text: String,
    /// The policy the user chose, when the service is monitored.
    pub policy: Option<Policy>,
    /// True when Leash used up its stops for this service.
    pub gave_up: bool,
    /// The running services that depend on it. Only the single service read
    /// fills this in, because it costs a call.
    pub dependents: Vec<String>,
    /// The plain reason and the risk, for a service on the suggested list.
    pub suggested: Option<Suggested>,
}

/// A day and its catch count, for the weekly bars.
#[derive(Debug, Clone, Serialize)]
pub struct DayCount {
    pub date: String,
    pub count: u32,
}

/// Everything the UI reads on start, and again after a state change.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    /// True when Leash may stop services.
    pub elevated: bool,
    pub paused: bool,
    /// True when the watchdog is running.
    pub watching: bool,
    /// The services the user chose, with their state.
    pub monitored: Vec<ServiceView>,
    /// Names in the config that Windows will not let Leash watch, for example a
    /// service that has since been uninstalled.
    pub unwatched: Vec<String>,
    pub caught_today: u32,
    pub weekly: Vec<DayCount>,
    /// When Leash last caught something, for the quiet timer.
    pub last_catch: Option<String>,
    pub general: General,
    /// Set when the config file could not be read. The UI shows it, and no
    /// command will write over the file until it is fixed.
    pub config_error: Option<String>,
}

/// The three facts the tray colour needs, read without touching the machine.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct TrayFlags {
    pub elevated: bool,
    pub paused: bool,
    pub watching: bool,
}

/// The payload of `service_caught`.
#[derive(Debug, Clone, Serialize)]
pub struct CaughtEvent {
    pub service: String,
    pub outcome: StopOutcome,
}

/// The payload of `service_notified`. `dry_run` tells a real notice apart from
/// a service that Leash only wrote down.
#[derive(Debug, Clone, Serialize)]
pub struct NotifiedEvent {
    pub service: String,
    pub dry_run: bool,
}

/// The payload of `state_changed`.
#[derive(Debug, Clone, Serialize)]
pub struct StateChangedEvent {
    /// The service this is about, when it is about one.
    pub service: Option<String>,
    /// `gave_up` when a service used up its stops, `null` for the app itself.
    pub kind: Option<&'static str>,
}

/// The payload of `engine_error`.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorEvent {
    pub service: String,
    pub error: String,
}

/// The app state. Tauri keeps one of these and hands it to every command.
pub struct AppState {
    /// Where the config lives, and where the log and the counts go.
    config_path: PathBuf,
    log_dir: PathBuf,
    stats: PathBuf,
    /// The embedded safety lists. They never change while the app runs.
    lists: SafetyLists,
    /// The engine events, handed to every watchdog and forwarded to the UI.
    events: Sender<WatchEvent>,
    /// The log for the actions the user starts from the UI. The watchdog writes
    /// its own lines through its own handle.
    log: Mutex<ActivityLog>,
    /// True when this launch found no config file, so the app has not been run
    /// before. Such a launch always shows the window.
    first_run: bool,
    /// The parts that change as the user works.
    inner: Mutex<Inner>,
}

struct Inner {
    config: Config,
    watchdog: Option<Watchdog>,
    elevated: bool,
    /// Names the last watchdog start could not watch.
    unwatched: Vec<String>,
    /// The reason the config file could not be read, when it could not.
    config_error: Option<String>,
    /// The stop counts, kept here so a new watchdog inherits them.
    retries: RetryBook,
}

impl AppState {
    /// Build the state from the real Windows folders.
    pub fn new(events: Sender<WatchEvent>) -> Result<Self, String> {
        let config_path = config_path().ok_or("Windows gave Leash no %APPDATA% folder")?;
        let log_dir = log::log_dir().ok_or("Windows gave Leash no %LOCALAPPDATA% folder")?;
        let stats = log::stats_path().ok_or("Windows gave Leash no %LOCALAPPDATA% folder")?;

        Self::with_paths(config_path, log_dir, stats, events)
    }

    /// Build the state from the given folders. The tests keep their own, so a
    /// test never reads or writes the real config.
    fn with_paths(
        config_path: PathBuf,
        log_dir: PathBuf,
        stats: PathBuf,
        events: Sender<WatchEvent>,
    ) -> Result<Self, String> {
        // A broken build must not run: without the deny list Leash could stop a
        // service it must never touch.
        let lists = SafetyLists::embedded().map_err(|error| error.to_string())?;

        // Read before anything can create the file, because this is what tells a
        // first launch from every later one.
        let first_run = !config_path.exists();

        // A config file that cannot be read must not stop the app: the user has
        // to be able to open Settings and see what is wrong.
        let (config, config_error) = match Config::load(&config_path) {
            Ok(config) => (config, None),
            Err(error) => {
                tracing::warn!(%error, "using the default settings");
                (Config::default(), Some(error.to_string()))
            }
        };

        // A first launch writes the file, even with nothing changed yet, so the
        // next launch is not treated as a first one. Without this, someone who
        // never touches a setting would be shown the window at every logon. A
        // failure here is logged rather than fatal: the app runs on its defaults
        // either way.
        if first_run && let Err(error) = config.save(&config_path) {
            tracing::warn!(%error, "the config could not be written on the first run");
        }

        let elevated = elevation::is_elevated().unwrap_or(false);
        let state = Self {
            log: Mutex::new(ActivityLog::new(log_dir.clone())),
            config_path,
            log_dir,
            stats,
            lists,
            events,
            first_run,
            inner: Mutex::new(Inner {
                config,
                watchdog: None,
                elevated,
                unwatched: Vec::new(),
                config_error,
                retries: RetryBook::new(),
            }),
        };

        {
            let mut inner = lock(&state.inner);
            state.restart_watchdog(&mut inner)?;
        }

        Ok(state)
    }

    /// Everything the UI reads at once.
    pub fn snapshot(&self) -> Result<Snapshot, String> {
        let mut inner = lock(&self.inner);

        // The rights can change while the app runs, for example when the user
        // starts Leash again as administrator, so they are read again here.
        let elevated = elevation::is_elevated().unwrap_or(false);
        if elevated != inner.elevated {
            inner.elevated = elevated;
            if let Some(watchdog) = &inner.watchdog {
                watchdog.set_elevated(elevated);
            }
        }

        let gave_up = inner
            .watchdog
            .as_ref()
            .map(Watchdog::gave_up)
            .unwrap_or_default();

        let mut monitored = Vec::new();
        for service in &inner.config.services {
            match scm::query(&service.name) {
                Ok(info) => monitored.push(self.view(
                    info,
                    &inner.config.never_touch.names,
                    Some(service.policy),
                    &gave_up,
                    Vec::new(),
                )),
                Err(error) => {
                    tracing::warn!(service = %service.name, %error, "cannot read a monitored service");
                }
            }
        }

        let counts = log::read_stats(&self.stats).map_err(|error| error.to_string())?;
        let caught_today = counts
            .get(&leash_engine::clock::today())
            .copied()
            .unwrap_or(0);
        let weekly = counts
            .into_iter()
            .map(|(date, count)| DayCount { date, count })
            .collect();
        let last_catch =
            log::read_latest_catch(&self.log_dir).map_err(|error| error.to_string())?;

        Ok(Snapshot {
            elevated: inner.elevated,
            paused: inner.config.general.paused,
            watching: inner.watchdog.is_some(),
            monitored,
            unwatched: inner.unwatched.clone(),
            caught_today,
            weekly,
            last_catch,
            general: inner.config.general.clone(),
            config_error: inner.config_error.clone(),
        })
    }

    /// True when the user wants notices. Section 9 gates them with a setting,
    /// and the log records every event whether they are on or not.
    pub fn notifications_on(&self) -> bool {
        lock(&self.inner).config.general.notifications
    }

    /// True when the window should stay out of the way until the tray is used.
    ///
    /// The scheduled task starts Leash at logon, and a window in the middle of
    /// the screen each morning would be the wrong greeting.
    pub fn start_minimized(&self) -> bool {
        lock(&self.inner).config.general.start_minimized
    }

    /// True when this launch has never run before.
    pub fn first_run(&self) -> bool {
        self.first_run
    }
    /// The three facts the tray colour needs. This reads nothing from Windows
    /// and no files, so the tray can ask for it whenever it likes.
    pub fn tray_flags(&self) -> TrayFlags {
        let inner = lock(&self.inner);

        TrayFlags {
            elevated: inner.elevated,
            paused: inner.config.general.paused,
            watching: inner.watchdog.is_some(),
        }
    }

    /// Every service on the machine, with the safety check and the policy for
    /// each one.
    pub fn services(&self) -> Result<Vec<ServiceView>, String> {
        // The machine is read outside the lock, because listing takes a moment
        // and no command should have to wait for it.
        let (never_touch, monitored, gave_up) = {
            let inner = lock(&self.inner);
            (
                inner.config.never_touch.names.clone(),
                inner.config.services.clone(),
                inner
                    .watchdog
                    .as_ref()
                    .map(Watchdog::gave_up)
                    .unwrap_or_default(),
            )
        };

        let infos = scm::list_services().map_err(|error| error.to_string())?;

        Ok(infos
            .into_iter()
            .map(|info| {
                let policy = monitored
                    .iter()
                    .find(|entry| entry.name.eq_ignore_ascii_case(&info.name))
                    .map(|entry| entry.policy);

                self.view(info, &never_touch, policy, &gave_up, Vec::new())
            })
            .collect())
    }

    /// One service, with everything the service sheet shows.
    ///
    /// This is the only path that reads the description and the dependents,
    /// because each costs a call and only one service is on screen at a time.
    pub fn service(&self, name: &str) -> Result<ServiceView, String> {
        let info = scm::query(name).map_err(|error| error.to_string())?;
        // A service that will not list its dependents still has a sheet.
        let dependents = scm::dependents(name).unwrap_or_default();

        let inner = lock(&self.inner);
        let gave_up = inner
            .watchdog
            .as_ref()
            .map(Watchdog::gave_up)
            .unwrap_or_default();
        let policy = inner.config.monitored(&info.name).map(|entry| entry.policy);

        Ok(self.view(
            info,
            &inner.config.never_touch.names,
            policy,
            &gave_up,
            dependents,
        ))
    }

    /// One service as the UI reads it, with the safety check and the policy.
    fn view(
        &self,
        info: ServiceInfo,
        never_touch: &[String],
        policy: Option<Policy>,
        gave_up: &[String],
        dependents: Vec<String>,
    ) -> ServiceView {
        let verdict = self.lists.assess(&info.facts(), never_touch);
        let gave_up = gave_up
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&info.name));
        // The suggested list is small, so one lookup per service is cheap.
        let suggested = self.lists.suggestion(&info.name).cloned();

        ServiceView {
            info,
            verdict_text: verdict.reason.text().to_string(),
            verdict,
            policy,
            gave_up,
            dependents,
            suggested,
        }
    }
    /// The newest log lines.
    pub fn log(&self, limit: usize) -> Result<Vec<Entry>, String> {
        log::read_entries(&self.log_dir, limit.min(MAX_LOG_LIMIT))
            .map_err(|error| error.to_string())
    }

    /// Put a service on the leash. It starts on the safest policy.
    pub fn add_monitor(&self, name: &str) -> Result<(), String> {
        // Section 11 asks for the name to be checked with Windows before it is
        // saved, and the safety check runs here too.
        let info = scm::query(name).map_err(|error| error.to_string())?;

        let mut inner = lock(&self.inner);
        let verdict = self
            .lists
            .assess(&info.facts(), &inner.config.never_touch.names);
        if !verdict.stop_allowed() {
            return Err(refuse(&info.name, verdict.reason.text()));
        }
        if inner.config.monitored(&info.name).is_some() {
            return Err(format!("{} is already on the leash", info.name));
        }

        let mut config = inner.config.clone();
        config.services.push(Monitored {
            name: info.name.clone(),
            policy: Policy::Notify,
        });
        self.store(&mut inner, config)?;
        self.restart_watchdog(&mut inner)
    }

    /// Take a service off the leash.
    pub fn remove_monitor(&self, name: &str) -> Result<(), String> {
        let mut inner = lock(&self.inner);
        let mut config = inner.config.clone();
        let before = config.services.len();
        config
            .services
            .retain(|entry| !entry.name.eq_ignore_ascii_case(name));
        if config.services.len() == before {
            return Err(format!("{name} is not on the leash"));
        }

        self.store(&mut inner, config)?;
        self.restart_watchdog(&mut inner)
    }

    /// Change the policy for a service that is on the leash.
    pub fn set_policy(&self, name: &str, policy: Policy) -> Result<(), String> {
        let mut inner = lock(&self.inner);
        let mut config = inner.config.clone();
        let Some(entry) = config
            .services
            .iter_mut()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
        else {
            return Err(format!("{name} is not on the leash"));
        };
        entry.policy = policy;

        self.store(&mut inner, config)?;
        self.restart_watchdog(&mut inner)
    }

    /// Add a service to the list Leash never touches.
    pub fn add_never_touch(&self, name: &str) -> Result<(), String> {
        let info = scm::query(name).map_err(|error| error.to_string())?;

        let mut inner = lock(&self.inner);
        if inner.config.is_never_touch(&info.name) {
            return Err(format!("{} is already left alone", info.name));
        }

        let mut config = inner.config.clone();
        config.never_touch.names.push(info.name.clone());
        self.store(&mut inner, config)?;
        self.restart_watchdog(&mut inner)
    }

    /// Pause or resume. A paused watchdog takes no action at all.
    pub fn set_paused(&self, paused: bool) -> Result<(), String> {
        let mut inner = lock(&self.inner);
        let mut config = inner.config.clone();
        config.general.paused = paused;
        self.store(&mut inner, config)?;

        if let Some(watchdog) = &inner.watchdog {
            watchdog.set_paused(paused);
        }

        Ok(())
    }

    /// Change the settings.
    pub fn set_settings(&self, settings: General) -> Result<(), String> {
        // Whether the switch that starts Leash with Windows moved. The task
        // itself is touched after the lock is released.
        let with_windows = {
            let mut inner = lock(&self.inner);
            let before = inner.config.general.start_with_windows;

            let mut config = inner.config.clone();
            config.general = settings;
            self.store(&mut inner, config)?;

            if let Some(watchdog) = &inner.watchdog {
                watchdog.set_paused(inner.config.general.paused);
            }

            // Turning the watchdog on or off takes effect now.
            if inner.config.general.start_watchdog != inner.watchdog.is_some() {
                self.restart_watchdog(&mut inner)?;
            }

            let after = inner.config.general.start_with_windows;
            (after != before).then_some(after)
        };

        // Starting with Windows is the scheduled task, which the installer
        // registered. The switch only enables or disables it, and Windows takes
        // a moment to answer, so this runs outside the lock.
        if let Some(enabled) = with_windows {
            task::set_start_with_windows(enabled)?;
        }

        Ok(())
    }

    /// Let Leash try a service again after it gave up on it.
    pub fn rearm(&self, name: &str) {
        let inner = lock(&self.inner);
        if let Some(watchdog) = &inner.watchdog {
            watchdog.rearm(name);
        }
    }

    /// Stop a service because the user asked, not because it started.
    pub fn stop_now(&self, name: &str) -> Result<StopOutcome, String> {
        // The manual path runs the same checks as the automatic one.
        let facts = scm::facts(name).map_err(|error| error.to_string())?;
        let verdict = {
            let inner = lock(&self.inner);
            if !inner.elevated {
                return Err(NEEDS_ADMIN.to_string());
            }
            self.lists.assess(&facts, &inner.config.never_touch.names)
        };
        if !verdict.stop_allowed() {
            return Err(refuse(&facts.name, verdict.reason.text()));
        }

        let outcome = scm::stop(name, DEFAULT_WAIT).map_err(|error| error.to_string())?;
        let (result, error) = scm::describe(&outcome);
        self.write_log(name, "stop", result, error)?;

        Ok(outcome)
    }

    /// Start a service because the user asked. Starting is the undo of a stop,
    /// so a blocked service is left alone here too.
    pub fn start_now(&self, name: &str) -> Result<StartOutcome, String> {
        let info = scm::query(name).map_err(|error| error.to_string())?;
        let verdict = {
            let inner = lock(&self.inner);
            if !inner.elevated {
                return Err(NEEDS_ADMIN.to_string());
            }
            self.lists
                .assess(&info.facts(), &inner.config.never_touch.names)
        };
        if !verdict.stop_allowed() {
            return Err(refuse(&info.name, verdict.reason.text()));
        }

        let outcome = scm::start(name, DEFAULT_WAIT).map_err(|error| error.to_string())?;
        let (result, error) = scm::describe_start(&outcome);
        self.write_log(name, "start", result, error)?;

        Ok(outcome)
    }

    /// Write the config and keep the new copy in memory. The file on disk is
    /// never written while it could not be read, so a hand made mistake in it is
    /// never lost.
    fn store(&self, inner: &mut Inner, config: Config) -> Result<(), String> {
        if let Some(error) = &inner.config_error {
            return Err(format!(
                "the config file could not be read, so Leash will not write over it: {error}"
            ));
        }

        let config = validate(config).map_err(|error| error.to_string())?;
        config
            .save(&self.config_path)
            .map_err(|error| error.to_string())?;
        inner.config = config;

        Ok(())
    }

    /// Start the watchdog again from the current config. This is how a change to
    /// the watched list is applied, and it is also how the app starts.
    ///
    /// A service Windows will not let Leash watch is left out and reported,
    /// because one bad entry must not leave every other service unguarded.
    fn restart_watchdog(&self, inner: &mut Inner) -> Result<(), String> {
        // Drop the old one first, so its threads end before a new set starts.
        inner.watchdog = None;

        let (watchable, unwatched) = self.watchable(&inner.config.services);
        inner.unwatched = unwatched;

        if !inner.config.general.start_watchdog || watchable.is_empty() {
            return Ok(());
        }

        // A service the user removed does not keep a count that would apply to it
        // if it were added again later. The counts of the rest stay, so a service
        // one stop from the cap, or already given up, is not forgiven by adding
        // another one.
        let watched: Vec<&str> = watchable.iter().map(|info| info.name.as_str()).collect();
        inner.retries.keep_only(&watched);

        let settings = WatchSettings {
            lists: SafetyLists::embedded().map_err(|error| error.to_string())?,
            never_touch: inner.config.never_touch.names.clone(),
            paused: inner.config.general.paused,
            elevated: inner.elevated,
            log: ActivityLog::new(self.log_dir.clone()),
            stats: self.stats.clone(),
            retries: inner.retries.clone(),
            stop_wait: DEFAULT_WAIT,
        };

        let watchdog = Watchdog::start(settings, &watchable, self.events.clone())
            .map_err(|error| error.to_string())?;
        inner.watchdog = Some(watchdog);

        Ok(())
    }

    /// Split the configured services into the ones Leash can watch and the ones
    /// it cannot.
    fn watchable(&self, services: &[Monitored]) -> (Vec<Monitored>, Vec<String>) {
        let mut ok = Vec::new();
        let mut missing = Vec::new();

        for service in services {
            match scm::query(&service.name) {
                Ok(_) => ok.push(service.clone()),
                Err(error) => {
                    tracing::warn!(service = %service.name, %error, "cannot watch this service");
                    missing.push(service.name.clone());
                }
            }
        }

        (ok, missing)
    }

    /// One line for an action the user started. The watchdog writes its own
    /// lines the same way, one short append per line.
    fn write_log(
        &self,
        service: &str,
        action: &str,
        result: &str,
        error: Option<String>,
    ) -> Result<(), String> {
        let mut entry = Entry::new(service, "manual");
        entry.action = Some(action.to_string());
        entry.result = Some(result.to_string());
        entry.error = error;

        lock(&self.log)
            .append(&entry)
            .map_err(|error| error.to_string())
    }
}

/// The message for a service Leash refuses to touch.
fn refuse(name: &str, reason: &str) -> String {
    format!("Leash never touches {name}: {reason}")
}

/// Take a lock, even when a thread panicked while holding it. The config and the
/// log are still usable in that case.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use leash_engine::safety::{Level, Reason};

    // The tests below read the real service manager, so they are ignored by
    // default. Nothing in them changes a service or the real config.
    //     cargo test -p leash -- --include-ignored

    /// A state with its own folders.
    fn state(tag: &str) -> AppState {
        let dir =
            std::env::temp_dir().join(format!("leash-state-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");

        let (events, _received) = std::sync::mpsc::channel();

        AppState::with_paths(
            dir.join("config.toml"),
            dir.join("logs"),
            dir.join("stats.json"),
            events,
        )
        .expect("state")
    }

    #[test]
    #[ignore = "reads the real service manager"]
    fn should_keep_the_watched_list_in_the_config() {
        let state = state("monitor");
        assert!(state.snapshot().expect("snapshot").monitored.is_empty());
        assert!(!state.snapshot().expect("snapshot").watching);

        state.add_monitor("Spooler").expect("add");

        let snapshot = state.snapshot().expect("snapshot");
        assert_eq!(snapshot.monitored.len(), 1);
        assert_eq!(snapshot.monitored[0].info.name, "Spooler");
        assert_eq!(snapshot.monitored[0].policy, Some(Policy::Notify));
        assert!(snapshot.watching);

        // The file holds it too, so Leash remembers it after a restart.
        let saved = Config::load(&state.config_path).expect("load");
        assert_eq!(saved.services.len(), 1);

        state.set_policy("Spooler", Policy::Stop).expect("policy");
        assert_eq!(
            state.snapshot().expect("snapshot").monitored[0].policy,
            Some(Policy::Stop)
        );

        state.remove_monitor("Spooler").expect("remove");
        assert!(state.snapshot().expect("snapshot").monitored.is_empty());
        assert!(
            Config::load(&state.config_path)
                .expect("load")
                .services
                .is_empty()
        );
    }

    #[test]
    #[ignore = "reads the real service manager"]
    fn should_refuse_a_service_leash_never_touches() {
        let state = state("deny");
        let error = state.add_monitor("RpcSs").unwrap_err();

        assert!(error.contains("never touches"), "{error}");
        assert!(state.snapshot().expect("snapshot").monitored.is_empty());
    }

    #[test]
    #[ignore = "reads the real service manager"]
    fn should_report_a_service_that_does_not_exist() {
        let state = state("missing");
        let error = state.add_monitor("LeashNoSuchService1234").unwrap_err();

        assert!(error.contains("does not exist"), "{error}");
    }

    #[test]
    #[ignore = "reads the real service manager"]
    fn should_block_a_service_the_user_left_alone() {
        let state = state("never-touch");
        state.add_never_touch("Spooler").expect("never touch");

        // A protected service cannot go on the leash either.
        let error = state.add_monitor("Spooler").unwrap_err();
        assert!(error.contains("never touches"), "{error}");

        let view = state
            .services()
            .expect("services")
            .into_iter()
            .find(|view| view.info.name == "Spooler")
            .expect("Spooler");
        assert_eq!(view.verdict.level, Level::Blocked);
        assert_eq!(view.verdict.reason, Reason::NeverTouch);
    }

    #[test]
    #[ignore = "reads the real service manager"]
    fn should_pause_and_resume_without_touching_the_threads() {
        let state = state("pause");
        state.add_monitor("Spooler").expect("add");

        state.set_paused(true).expect("pause");
        assert!(state.snapshot().expect("snapshot").paused);

        state.set_paused(false).expect("resume");
        assert!(!state.snapshot().expect("snapshot").paused);
    }

    #[test]
    #[ignore = "reads the real service manager"]
    fn should_keep_the_settings() {
        let state = state("settings");
        let mut settings = state.snapshot().expect("snapshot").general;
        settings.notifications = false;
        settings.start_minimized = false;

        state.set_settings(settings.clone()).expect("settings");

        assert_eq!(state.snapshot().expect("snapshot").general, settings);
    }

    #[test]
    #[ignore = "reads the real service manager"]
    fn should_read_one_service_for_the_sheet() {
        let state = state("sheet");

        let view = state.service("Spooler").expect("service");
        assert_eq!(view.info.name, "Spooler");
        assert!(!view.info.description.is_empty());
        assert_eq!(view.policy, None);
        assert_eq!(view.verdict.level, Level::Normal);
        // A normal service needs no explanation.
        assert!(view.verdict_text.is_empty());

        // A service the user protects gets the reason in words.
        state.add_never_touch("Spooler").expect("never touch");
        let view = state.service("Spooler").expect("service");
        assert_eq!(view.verdict.reason, Reason::NeverTouch);
        assert!(
            view.verdict_text.contains("leave it alone"),
            "{}",
            view.verdict_text
        );
    }

    #[test]
    #[ignore = "reads the real service manager"]
    fn should_refuse_a_manual_stop_without_admin_rights() {
        let state = state("admin");
        if state.snapshot().expect("snapshot").elevated {
            println!("this shell is elevated, so the refusal cannot be checked here");
            return;
        }

        let error = state.stop_now("Spooler").unwrap_err();
        assert!(error.contains("administrator"), "{error}");
    }

    #[test]
    #[ignore = "reads the real service manager"]
    fn should_report_the_log_it_has_written() {
        let state = state("log");
        assert!(state.log(10).expect("log").is_empty());

        // A manual action writes one line, even when the action is refused by
        // Windows, because the user should see that they tried.
        let _ = state.stop_now("Spooler");

        assert!(state.log(1_000_000).expect("log").len() <= MAX_LOG_LIMIT);
    }
}
