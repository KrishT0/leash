//! The watchdog: what Leash does when a monitored service starts.
//!
//! This half of the module is the decision. It has no Windows calls, so it is
//! tested on any OS. The event loop that feeds it lives further down.

use crate::config::Policy;
use crate::safety::Verdict;
use std::collections::VecDeque;
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
#[derive(Debug, Clone, Default)]
pub struct RetryState {
    stops: VecDeque<Instant>,
    /// False after the user has been told Leash gave up. Cleared by `rearm`.
    armed: bool,
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
}
