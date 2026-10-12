//! The watchdog: what Leash does when a monitored service starts.
//!
//! Leash acts on changes only. A monitored service that is already running when
//! the watchdog starts is left alone. Section 7 of the instructions reads that
//! way, and it means Leash never stops something just for being open.
//!
//! This half of the module is the decision. It has no Windows calls, so it is
//! tested on any OS. The event loop that feeds it lives further down.

use crate::clock::today;
use crate::config::{Monitored, Policy};
use crate::log::{ActivityLog, Entry, LogError, bump_catch};
use crate::safety::{SafetyLists, Verdict};
use crate::scm::{self, ScmError, Status, StopOutcome, WatchState, Watched};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Stops allowed for one service inside [`STOP_WINDOW`].
pub const MAX_STOPS: usize = 5;
/// The window that the stop count is measured over.
pub const STOP_WINDOW: Duration = Duration::from_secs(600);
/// The first backoff step. It doubles with each stop in the window.
pub const BACKOFF_STEP: Duration = Duration::from_secs(1);
/// The longest backoff.
pub const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// What Leash does about one service that started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Log the event and tell the user. Do not stop.
    Notify,
    /// Log what a stop would do. Do not stop.
    DryRun,
    /// Stop the service, after waiting `after`.
    Stop { after: Duration },
    /// Do nothing.
    Skip(SkipReason),
}

/// Why Leash took no action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The watchdog is paused.
    Paused,
    /// The safety check blocks this service.
    Blocked,
    /// The service reached the stop cap. The user must re-arm it.
    GaveUp,
    /// The process is not elevated, so it cannot stop services.
    NotElevated,
}

/// The stop count for one service, and whether Leash has given up on it.
#[derive(Debug, Clone)]
pub struct RetryState {
    stops: VecDeque<Instant>,
    /// False after the user has been told Leash gave up. Cleared by `rearm`.
    armed: bool,
}

impl Default for RetryState {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryState {
    pub fn new() -> Self {
        Self {
            stops: VecDeque::new(),
            armed: true,
        }
    }

    /// Note that Leash stopped this service now.
    pub fn record_stop(&mut self, now: Instant) {
        self.stops.push_back(now);
        self.forget_old(now);
    }

    /// Stops inside the window.
    pub fn stops_in_window(&self, now: Instant) -> usize {
        self.stops
            .iter()
            .filter(|time| now.duration_since(**time) < STOP_WINDOW)
            .count()
    }

    /// True when the service used up its stops and Leash stopped trying.
    /// The user clears this with [`RetryState::rearm`].
    pub fn has_given_up(&self, now: Instant) -> bool {
        self.armed && self.stops_in_window(now) >= MAX_STOPS
    }

    /// Let Leash try this service again, after the user asked for it.
    pub fn rearm(&mut self) {
        self.armed = true;
        self.stops.clear();
    }

    /// How long to wait before the next stop of this service. Zero for the
    /// first stop.
    pub fn backoff(&self, now: Instant) -> Duration {
        match self.stops_in_window(now) {
            0 => Duration::ZERO,
            stops => {
                // One step per earlier stop: 1s, 2s, 4s, and so on.
                let step = BACKOFF_STEP.saturating_mul(1 << (stops.min(16) as u32 - 1));
                step.min(MAX_BACKOFF)
            }
        }
    }

    fn forget_old(&mut self, now: Instant) {
        while let Some(first) = self.stops.front() {
            if now.duration_since(*first) < STOP_WINDOW {
                break;
            }
            self.stops.pop_front();
        }
    }
}

/// Decide what to do about one service that started. The first match wins, so
/// the safer answer always comes first.
pub fn decide(
    policy: Policy,
    paused: bool,
    elevated: bool,
    verdict: &Verdict,
    retry: &RetryState,
    now: Instant,
) -> Decision {
    if paused {
        return Decision::Skip(SkipReason::Paused);
    }
    // A blocked service is never touched, whatever the policy says.
    if !verdict.stop_allowed() {
        return Decision::Skip(SkipReason::Blocked);
    }

    match policy {
        Policy::Notify => Decision::Notify,
        Policy::DryRun => Decision::DryRun,
        Policy::Stop => {
            if retry.has_given_up(now) {
                return Decision::Skip(SkipReason::GaveUp);
            }
            if !elevated {
                return Decision::Skip(SkipReason::NotElevated);
            }
            Decision::Stop {
                after: retry.backoff(now),
            }
        }
    }
}

/// An event that the watchdog sends to the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// A monitored service started and Leash stopped it.
    Caught {
        service: String,
        outcome: StopOutcome,
    },
    /// A monitored service started and Leash told the user.
    Notified { service: String },
    /// A monitored service started and Leash wrote down what a stop would do.
    WouldStop { service: String },
    /// A service used up its stops inside the window.
    GaveUp { service: String },
    /// One service failed, for example because Windows denied the read.
    Failed { service: String, error: String },
}

/// What the watchdog needs before it starts.
#[derive(Debug)]
pub struct WatchSettings {
    /// The embedded safety lists.
    pub lists: SafetyLists,
    /// The user list that Leash must never touch.
    pub never_touch: Vec<String>,
    pub paused: bool,
    /// True when this process may stop services.
    pub elevated: bool,
    pub log: ActivityLog,
    /// Where the daily catch counts go.
    pub stats: PathBuf,
    /// The counts to use. Pass the same one to keep them across a rebuild.
    pub retries: RetryBook,
    pub stop_wait: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error(transparent)]
    Scm(#[from] ScmError),
    #[error(transparent)]
    Log(#[from] LogError),
    #[error("cannot start a watch thread for {service}: {source}")]
    Thread {
        service: String,
        #[source]
        source: std::io::Error,
    },
}

/// Everything the watch threads share.
#[derive(Debug)]
struct Shared {
    lists: SafetyLists,
    never_touch: Vec<String>,
    paused: AtomicBool,
    elevated: AtomicBool,
    sink: Sender<WatchEvent>,
    log: Mutex<ActivityLog>,
    stats: PathBuf,
    retries: RetryBook,
    stop_wait: Duration,
}

/// The stop counts and backoffs, one per service, kept outside the watchdog.
///
/// The app owns one of these and hands the same one to every watchdog it builds.
/// Changing the watch list, or a policy, builds a new watchdog, and without this
/// the record of a service that is close to the cap, or has already reached it,
/// would go with the old one.
///
/// Cloning gives another handle on the same counts, not a copy of them.
#[derive(Debug, Clone, Default)]
pub struct RetryBook(Arc<Mutex<HashMap<String, RetryState>>>);

impl RetryBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// The counts, for the engine's own use. The app only carries the book.
    pub(crate) fn lock(&self) -> MutexGuard<'_, HashMap<String, RetryState>> {
        match self.0.lock() {
            Ok(counts) => counts,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

/// The watchdog. One thread watches each monitored service.
#[derive(Debug)]
pub struct Watchdog {
    shared: Arc<Shared>,
    quit: scm::StopEvent,
    threads: Vec<JoinHandle<()>>,
}

impl Watchdog {
    /// Start one watch thread for each monitored service.
    pub fn start(
        settings: WatchSettings,
        services: &[Monitored],
        sink: Sender<WatchEvent>,
    ) -> Result<Self, WatchError> {
        let shared = Arc::new(Shared {
            lists: settings.lists,
            never_touch: settings.never_touch,
            paused: AtomicBool::new(settings.paused),
            elevated: AtomicBool::new(settings.elevated),
            sink,
            log: Mutex::new(settings.log),
            stats: settings.stats,
            retries: settings.retries,
            stop_wait: settings.stop_wait,
        });
        let quit = scm::StopEvent::new()?;
        let mut threads = Vec::with_capacity(services.len());

        for service in services {
            // A service that cannot be opened stops the whole start. The
            // threads that are already running are told to stop first.
            let watched = match scm::watch(&service.name, &quit) {
                Ok(watched) => watched,
                Err(error) => {
                    join_all(&mut threads, &quit);
                    return Err(error.into());
                }
            };

            let name = service.name.clone();
            let policy = service.policy;
            let shared = Arc::clone(&shared);
            let started = std::thread::Builder::new()
                .name(format!("leash-watch-{}", service.name))
                .spawn(move || watch_loop(watched, name, policy, shared));

            match started {
                Ok(thread) => threads.push(thread),
                Err(source) => {
                    join_all(&mut threads, &quit);
                    return Err(WatchError::Thread {
                        service: service.name.clone(),
                        source,
                    });
                }
            }
        }

        Ok(Self {
            shared,
            quit,
            threads,
        })
    }

    /// Stop every watch thread and wait for them to end.
    pub fn stop(&mut self) {
        join_all(&mut self.threads, &self.quit);
    }

    /// Pause or resume. While paused the watchdog takes no action at all.
    pub fn set_paused(&self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.shared.paused.load(Ordering::Relaxed)
    }

    /// Tell the watchdog whether this process may stop services.
    pub fn set_elevated(&self, elevated: bool) {
        self.shared.elevated.store(elevated, Ordering::Relaxed);
    }

    /// Let Leash try a service again after the stop cap. The user asks for it.
    pub fn rearm(&self, service: &str) {
        if let Some(retry) = self.shared.retries.lock().get_mut(service) {
            retry.rearm();
        }
    }

    /// The services that used up their stops inside the window and are waiting
    /// for the user to try again. The UI shows a badge for these.
    pub fn gave_up(&self) -> Vec<String> {
        let retries = self.shared.retries.lock();
        let now = Instant::now();

        retries
            .iter()
            .filter(|(_, retry)| retry.has_given_up(now))
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.stop();
    }
}

fn join_all(threads: &mut Vec<JoinHandle<()>>, quit: &scm::StopEvent) {
    quit.set();
    for thread in threads.drain(..) {
        let _ = thread.join();
    }
}

/// One thread per service. Windows reports one change per request, so the
/// watch is armed again on every round.
fn watch_loop(mut watched: Watched, name: String, policy: Policy, shared: Arc<Shared>) {
    if let Err(error) = watched.arm() {
        report(&shared, &name, &error.to_string());
        return;
    }

    loop {
        match watched.wait() {
            Ok(WatchState::Quit) => return,
            Ok(WatchState::Changed) => {}
            Err(error) => {
                report(&shared, &name, &error.to_string());
                return;
            }
        }

        // Arm again straight away, before the work below. Stopping a service
        // can take seconds, and a service that comes back in that time must
        // not be missed.
        if let Err(error) = watched.arm() {
            report(&shared, &name, &error.to_string());
            return;
        }

        // A change can be a stop or a pending state. Only a running service
        // gets the policy.
        match watched.status() {
            Ok(Status::Running) => {}
            Ok(_) => continue,
            Err(error) => {
                report(&shared, &name, &error.to_string());
                return;
            }
        }

        if let Err(error) = act(&name, policy, &shared) {
            report(&shared, &name, &error.to_string());
        }
    }
}

/// Apply the policy to one service that is running now.
fn act(name: &str, policy: Policy, shared: &Shared) -> Result<(), WatchError> {
    let now = Instant::now();
    let paused = shared.paused.load(Ordering::Relaxed);
    let elevated = shared.elevated.load(Ordering::Relaxed);

    // The safety check runs again here, right before any action. Without the
    // facts Leash cannot tell whether a stop is safe, so it does not stop.
    let verdict = match scm::facts(name) {
        Ok(facts) => shared.lists.assess(&facts, &shared.never_touch),
        Err(error) => {
            let text = error.to_string();
            write_log(
                shared,
                name,
                Some("unknown"),
                Some("no_facts"),
                Some(text.clone()),
            )?;
            report(shared, name, &text);
            return Ok(());
        }
    };

    let decision = {
        let retries = shared.retries.lock();
        let unseen = RetryState::new();
        let retry = retries.get(name).unwrap_or(&unseen);
        decide(policy, paused, elevated, &verdict, retry, now)
    };

    match decision {
        // While paused Leash stays quiet, and writes nothing.
        Decision::Skip(SkipReason::Paused) => {}
        Decision::Skip(SkipReason::Blocked) => {
            write_log(shared, name, None, Some("blocked"), None)?;
        }
        Decision::Skip(SkipReason::GaveUp) => {
            write_log(shared, name, None, Some("gave_up"), None)?;
            send(
                shared,
                WatchEvent::GaveUp {
                    service: name.to_string(),
                },
            );
        }
        Decision::Skip(SkipReason::NotElevated) => {
            write_log(shared, name, Some("stop"), Some("not_elevated"), None)?;
        }
        Decision::Notify => {
            write_log(shared, name, Some("notify"), Some("notified"), None)?;
            send(
                shared,
                WatchEvent::Notified {
                    service: name.to_string(),
                },
            );
        }
        Decision::DryRun => {
            write_log(shared, name, Some("dry_run"), Some("would_stop"), None)?;
            send(
                shared,
                WatchEvent::WouldStop {
                    service: name.to_string(),
                },
            );
        }
        Decision::Stop { after } => {
            // Wait the backoff, so a service that comes straight back is not
            // hammered.
            if !after.is_zero() {
                std::thread::sleep(after);
            }

            let outcome = scm::stop(name, shared.stop_wait)?;
            // Every attempt counts, so a service that refuses to stop also
            // reaches the cap and the backoff.
            shared
                .retries
                .lock()
                .entry(name.to_string())
                .or_default()
                .record_stop(now);

            let (result, error) = scm::describe(&outcome);
            write_log(shared, name, Some("stop"), Some(result), error)?;

            if matches!(outcome, StopOutcome::Stopped) {
                // A catch, for the weekly bars.
                if let Err(error) = bump_catch(&shared.stats, &today()) {
                    tracing::warn!(%error, "cannot count a catch");
                }
            }

            send(
                shared,
                WatchEvent::Caught {
                    service: name.to_string(),
                    outcome,
                },
            );
        }
    }

    Ok(())
}

fn write_log(
    shared: &Shared,
    service: &str,
    action: Option<&str>,
    result: Option<&str>,
    error: Option<String>,
) -> Result<(), WatchError> {
    let mut entry = Entry::new(service, "started");
    entry.action = action.map(str::to_string);
    entry.result = result.map(str::to_string);
    entry.error = error;

    lock(&shared.log).append(&entry).map_err(WatchError::Log)
}

fn report(shared: &Shared, service: &str, error: &str) {
    send(
        shared,
        WatchEvent::Failed {
            service: service.to_string(),
            error: error.to_string(),
        },
    );
}

fn send(shared: &Shared, event: WatchEvent) {
    // The app may have closed the channel while it shuts down.
    let _ = shared.sink.send(event);
}

/// Take a lock, even when a thread panicked while holding it. The log and the
/// stop counts still work in that case.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety::{Level, Reason};

    fn verdict(level: Level) -> Verdict {
        Verdict {
            level,
            reason: match level {
                Level::Blocked => Reason::DenyList,
                Level::Critical => Reason::SensitiveList,
                Level::Normal => Reason::None,
            },
        }
    }

    fn now() -> Instant {
        Instant::now()
    }

    #[test]
    fn should_notify_when_the_policy_is_notify() {
        let decision = decide(
            Policy::Notify,
            false,
            true,
            &verdict(Level::Normal),
            &RetryState::new(),
            now(),
        );
        assert_eq!(decision, Decision::Notify);
    }

    #[test]
    fn should_report_a_dry_run_when_the_policy_is_dry_run() {
        let decision = decide(
            Policy::DryRun,
            false,
            true,
            &verdict(Level::Normal),
            &RetryState::new(),
            now(),
        );
        assert_eq!(decision, Decision::DryRun);
    }

    #[test]
    fn should_stop_when_the_policy_is_stop_and_the_process_is_elevated() {
        let decision = decide(
            Policy::Stop,
            false,
            true,
            &verdict(Level::Normal),
            &RetryState::new(),
            now(),
        );
        assert_eq!(
            decision,
            Decision::Stop {
                after: Duration::ZERO
            }
        );
    }

    #[test]
    fn should_skip_everything_while_paused() {
        for policy in [Policy::Notify, Policy::DryRun, Policy::Stop] {
            let decision = decide(
                policy,
                true,
                true,
                &verdict(Level::Normal),
                &RetryState::new(),
                now(),
            );
            assert_eq!(decision, Decision::Skip(SkipReason::Paused), "{policy:?}");
        }
    }

    #[test]
    fn should_skip_a_blocked_service_whatever_the_policy_is() {
        for policy in [Policy::Notify, Policy::DryRun, Policy::Stop] {
            let decision = decide(
                policy,
                false,
                true,
                &verdict(Level::Blocked),
                &RetryState::new(),
                now(),
            );
            assert_eq!(decision, Decision::Skip(SkipReason::Blocked), "{policy:?}");
        }
    }

    #[test]
    fn should_let_a_critical_service_through_after_a_warning() {
        let decision = decide(
            Policy::Stop,
            false,
            true,
            &verdict(Level::Critical),
            &RetryState::new(),
            now(),
        );
        assert_eq!(
            decision,
            Decision::Stop {
                after: Duration::ZERO
            }
        );
    }

    #[test]
    fn should_skip_a_stop_without_elevation() {
        let decision = decide(
            Policy::Stop,
            false,
            false,
            &verdict(Level::Normal),
            &RetryState::new(),
            now(),
        );
        assert_eq!(decision, Decision::Skip(SkipReason::NotElevated));
    }

    #[test]
    fn should_still_notify_and_dry_run_without_elevation() {
        let notify = decide(
            Policy::Notify,
            false,
            false,
            &verdict(Level::Normal),
            &RetryState::new(),
            now(),
        );
        assert_eq!(notify, Decision::Notify);

        let dry_run = decide(
            Policy::DryRun,
            false,
            false,
            &verdict(Level::Normal),
            &RetryState::new(),
            now(),
        );
        assert_eq!(dry_run, Decision::DryRun);
    }

    #[test]
    fn should_give_up_after_the_stop_cap() {
        let start = now();
        let mut retry = RetryState::new();

        for stop in 0..MAX_STOPS {
            assert!(!retry.has_given_up(start), "stop {stop}");
            retry.record_stop(start);
        }

        assert!(retry.has_given_up(start));
        let decision = decide(
            Policy::Stop,
            false,
            true,
            &verdict(Level::Normal),
            &retry,
            start,
        );
        assert_eq!(decision, Decision::Skip(SkipReason::GaveUp));
    }

    #[test]
    fn should_count_only_the_stops_inside_the_window() {
        let start = now();
        let mut retry = RetryState::new();

        for _ in 0..MAX_STOPS {
            retry.record_stop(start);
        }
        assert_eq!(retry.stops_in_window(start), MAX_STOPS);

        let later = start + STOP_WINDOW + Duration::from_secs(1);
        assert_eq!(retry.stops_in_window(later), 0);
        assert!(!retry.has_given_up(later));
    }

    #[test]
    fn should_back_off_between_stops() {
        let start = now();
        let mut retry = RetryState::new();

        assert_eq!(retry.backoff(start), Duration::ZERO);
        retry.record_stop(start);
        assert_eq!(retry.backoff(start), Duration::from_secs(1));
        retry.record_stop(start);
        assert_eq!(retry.backoff(start), Duration::from_secs(2));
        retry.record_stop(start);
        assert_eq!(retry.backoff(start), Duration::from_secs(4));
        retry.record_stop(start);
        assert_eq!(retry.backoff(start), Duration::from_secs(8));
    }

    #[test]
    fn should_cap_the_backoff() {
        let start = now();
        let mut retry = RetryState::new();
        for _ in 0..MAX_STOPS + 4 {
            retry.record_stop(start);
        }
        assert_eq!(retry.backoff(start), MAX_BACKOFF);
    }

    #[test]
    fn should_carry_the_backoff_into_the_decision() {
        let start = now();
        let mut retry = RetryState::new();
        retry.record_stop(start);
        retry.record_stop(start);

        let decision = decide(
            Policy::Stop,
            false,
            true,
            &verdict(Level::Normal),
            &retry,
            start,
        );
        assert_eq!(
            decision,
            Decision::Stop {
                after: Duration::from_secs(2),
            }
        );
    }

    #[test]
    fn should_try_again_after_the_user_rearms() {
        let start = now();
        let mut retry = RetryState::new();
        for _ in 0..MAX_STOPS {
            retry.record_stop(start);
        }
        assert!(retry.has_given_up(start));

        retry.rearm();
        assert!(!retry.has_given_up(start));
        assert_eq!(retry.stops_in_window(start), 0);
        assert_eq!(retry.backoff(start), Duration::ZERO);
    }

    #[test]
    fn should_give_up_again_after_a_rearm_and_more_stops() {
        let start = now();
        let mut retry = RetryState::new();
        for _ in 0..MAX_STOPS {
            retry.record_stop(start);
        }
        retry.rearm();
        for _ in 0..MAX_STOPS {
            retry.record_stop(start);
        }
        assert!(retry.has_given_up(start));
    }

    // The tests below start real watch threads.

    fn settings(tag: &str) -> WatchSettings {
        let dir =
            std::env::temp_dir().join(format!("leash-watch-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        // A count left over from an earlier run would break the catch check.
        let stats = std::env::temp_dir().join(format!("leash-watch-stats-{tag}.json"));
        let _ = std::fs::remove_file(&stats);

        WatchSettings {
            lists: SafetyLists::embedded().expect("lists"),
            never_touch: Vec::new(),
            paused: false,
            elevated: false,
            log: ActivityLog::new(dir),
            stats,
            retries: RetryBook::new(),
            stop_wait: Duration::from_secs(1),
        }
    }

    #[test]
    #[ignore = "needs an elevated shell and LeashTestSvc"]
    fn should_give_up_after_the_cap_on_a_service_that_keeps_coming_back() {
        if scm::query(TEST_SERVICE).is_err() {
            println!("{TEST_SERVICE} is not installed. Run scripts/elevated-test.ps1.");
            return;
        }

        let (sink, events) = std::sync::mpsc::channel();
        let services = vec![Monitored {
            name: TEST_SERVICE.to_string(),
            policy: Policy::Stop,
        }];
        // Named `first`, because a local called `settings` would hide the helper
        // function of the same name from the rest of this test.
        let mut first = settings("cap");
        first.elevated = true;
        first.stop_wait = Duration::from_secs(5);
        // Both paths are checked at the end, so they are taken before the
        // settings move into the watchdog.
        let stats = first.stats.clone();
        let log_dir = first.log.dir().to_path_buf();
        let book = first.retries.clone();

        let mut watchdog = Watchdog::start(first, &services, sink).expect("start");
        start_test_service();
        show_start_timeline();

        let mut caught = 0;
        let deadline = Instant::now() + Duration::from_secs(600);
        loop {
            assert!(
                Instant::now() < deadline,
                "timed out after {caught} catches"
            );

            let event = match events.recv_timeout(Duration::from_secs(120)) {
                Ok(event) => event,
                Err(error) => panic!(
                    "no event in 120 s ({error:?}). {TEST_SERVICE} is now {:?}",
                    scm::query(TEST_SERVICE).map(|info| info.status)
                ),
            };

            match event {
                WatchEvent::Caught {
                    outcome: StopOutcome::Stopped,
                    ..
                } => {
                    caught += 1;
                    println!("catch {caught} of {MAX_STOPS}");
                    // Bring the service back, so the watchdog gets another turn.
                    start_test_service();
                    show_start_timeline();
                }
                WatchEvent::GaveUp { .. } => break,
                other => panic!("unexpected event: {other:?}"),
            }
        }

        assert_eq!(caught, MAX_STOPS, "the cap must allow {MAX_STOPS} stops");

        // The same run must have counted the catches and written the log.
        let counted = crate::log::read_stats(&stats).expect("stats");
        assert_eq!(
            counted.get(&today()),
            Some(&(MAX_STOPS as u32)),
            "the daily catch count"
        );
        assert_eq!(logged_catches(&log_dir), MAX_STOPS, "the logged catches");

        watchdog.stop();

        // Anything else the app does, such as adding a service, builds a new
        // watchdog. The counts live outside it, so this one must still know that
        // the service gave up.
        let (sink_again, _) = std::sync::mpsc::channel();
        // Not called `settings`: that name is already the one built for the
        // first watchdog, and this line would call the value instead.
        let mut again = settings("cap-again");
        again.elevated = true;
        again.retries = book;
        let mut rebuilt = Watchdog::start(again, &services, sink_again).expect("start again");

        assert_eq!(
            rebuilt.gave_up(),
            vec![TEST_SERVICE.to_string()],
            "a rebuild must not forgive a service that used up its stops"
        );

        rebuilt.stop();
    }

    /// The dummy service that `scripts/elevated-test.ps1` creates.
    const TEST_SERVICE: &str = "LeashTestSvc";

    fn start_test_service() {
        let started = std::process::Command::new("sc.exe")
            .args(["start", TEST_SERVICE])
            .status()
            .expect("sc start");
        assert!(started.success(), "sc start {TEST_SERVICE} failed");
    }

    /// Print how the service moves through its states after a start. This never
    /// fails the test. The watchdog normally answers within a few hundred
    /// milliseconds, faster than a poll can sample the running state, so a move
    /// to StopPending is the proof that it worked.
    fn show_start_timeline() {
        let start = Instant::now();
        let mut last = None;
        let mut left_stopped = false;

        while start.elapsed() < Duration::from_secs(10) {
            if let Ok(info) = scm::query(TEST_SERVICE) {
                if last != Some(info.status) {
                    println!("  {:>6} ms: {:?}", start.elapsed().as_millis(), info.status);
                    last = Some(info.status);
                }
                if info.status != Status::Stopped {
                    left_stopped = true;
                }
                // The test never stops the service here, so a move to StopPending
                // can only come from the watchdog.
                if info.status == Status::Running
                    || (left_stopped && info.status == Status::StopPending)
                {
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        println!("  {TEST_SERVICE} did not start within 10 s");
    }

    /// How many catches the activity log holds.
    fn logged_catches(dir: &std::path::Path) -> usize {
        let mut count = 0;
        for entry in std::fs::read_dir(dir).expect("log dir") {
            let path = entry.expect("entry").path();
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            count += text
                .lines()
                .filter(|line| line.contains("\"action\":\"stop\""))
                .filter(|line| line.contains("\"result\":\"success\""))
                .count();
        }
        count
    }

    #[test]
    fn should_share_the_counts_with_every_clone_of_the_book() {
        let book = RetryBook::new();
        let handed_on = book.clone();
        let now = Instant::now();

        book.lock()
            .entry("Spooler".to_string())
            .or_default()
            .record_stop(now);

        let counts = handed_on.lock();
        let retry = counts.get("Spooler").expect("the count this test recorded");
        assert_eq!(retry.stops_in_window(now), 1);
    }

    #[test]
    #[ignore = "creates a real event object"]
    fn should_report_no_service_as_given_up_before_anything_happens() {
        // An empty watch list starts no threads, so this is cheap. Windows is
        // still called for the event that tells the threads to stop.
        let (sink, _events) = std::sync::mpsc::channel();
        let mut watchdog = Watchdog::start(settings("no-give-up"), &[], sink).expect("start empty");

        assert!(!watchdog.is_paused());
        assert!(watchdog.gave_up().is_empty());

        watchdog.rearm("Nothing");
        watchdog.stop();
    }

    #[test]
    #[ignore = "starts real watch threads"]
    fn should_watch_a_service_and_stop_cleanly() {
        let (sink, events) = std::sync::mpsc::channel();
        let services = vec![Monitored {
            name: "Spooler".to_string(),
            policy: Policy::Notify,
        }];

        let mut watchdog = Watchdog::start(settings("clean"), &services, sink).expect("start");
        assert!(!watchdog.is_paused());

        watchdog.set_paused(true);
        assert!(watchdog.is_paused());
        watchdog.set_elevated(true);
        watchdog.rearm("Spooler");

        std::thread::sleep(Duration::from_millis(300));
        watchdog.stop();

        // Nothing changed on the machine, so no event arrived.
        assert!(events.try_recv().is_err(), "an event arrived on its own");
    }

    #[test]
    #[ignore = "starts real watch threads"]
    fn should_refuse_to_watch_a_service_that_does_not_exist() {
        let (sink, _events) = std::sync::mpsc::channel();
        let services = vec![Monitored {
            name: "LeashNoSuchService1234".to_string(),
            policy: Policy::Notify,
        }];

        match Watchdog::start(settings("missing"), &services, sink) {
            Err(WatchError::Scm(ScmError::NotFound { .. })) => {}
            Err(error) => panic!("unexpected error: {error:?}"),
            Ok(_) => panic!("a missing service must not start a watch"),
        }
    }
}
