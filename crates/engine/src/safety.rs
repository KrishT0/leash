//! Safety checks. Decides if a service may be stopped.
//!
//! The deny, sensitive, and suggested lists are embedded in the exe. The
//! never-touch list comes from `config.toml`, which is untrusted input, so the
//! caller passes it in on every check.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const DENY_JSON: &str = include_str!("../data/deny.json");
const SENSITIVE_JSON: &str = include_str!("../data/sensitive.json");
const SUGGESTED_JSON: &str = include_str!("../data/suggested.json");

/// Facts about one service. The SCM layer supplies these. No Windows types
/// here, so the checks stay testable on any OS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceFacts {
    /// Internal service name, for example `Spooler`.
    pub name: String,
    /// The service accepts `SERVICE_CONTROL_STOP`.
    pub accepts_stop: bool,
    /// The service is a kernel driver.
    pub is_driver: bool,
    /// The service is a Win32 service, not a driver or a legacy service.
    pub is_win32: bool,
    /// Number of running services that depend on this one.
    pub running_dependents: usize,
}

/// The result of a safety check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Never stop. The UI hides the Monitor switch.
    Blocked,
    /// Warn first. The user must confirm. Shows the dependents.
    Critical,
    /// May be monitored.
    Normal,
}

/// Why a check returned this level. `None` for a normal service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    DenyList,
    NeverTouch,
    StopNotAccepted,
    Driver,
    NotWin32Service,
    SensitiveList,
    RunningDependents,
    None,
}

/// The verdict for one service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub level: Level,
    pub reason: Reason,
}

impl Verdict {
    fn blocked(reason: Reason) -> Self {
        Self {
            level: Level::Blocked,
            reason,
        }
    }

    fn critical(reason: Reason) -> Self {
        Self {
            level: Level::Critical,
            reason,
        }
    }

    /// True when the user may stop this service.
    pub fn stop_allowed(&self) -> bool {
        self.level != Level::Blocked
    }
}

/// How bad it is to stop a suggested service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    Low,
    Medium,
}

/// A known optional service with a plain reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggested {
    /// Internal service name.
    pub name: String,
    /// Name shown to the user.
    pub display: String,
    /// Why the service is usually not needed.
    pub reason: String,
    /// What breaks when the service is stopped.
    pub effect: String,
    pub risk: Risk,
}

/// A malformed embedded list. The app must not start when this happens,
/// because a missing deny list could allow a stop that is never allowed.
#[derive(Debug, thiserror::Error)]
pub enum SafetyError {
    #[error("the embedded list {list} is not valid JSON: {source}")]
    List {
        list: &'static str,
        #[source]
        source: serde_json::Error,
    },
}

/// The embedded safety lists, parsed and normalized.
#[derive(Debug)]
pub struct SafetyLists {
    deny: HashSet<String>,
    sensitive: HashSet<String>,
    suggested: Vec<Suggested>,
}

impl SafetyLists {
    /// Parse the three embedded lists. Call this once at startup. An error
    /// means the build is broken.
    pub fn embedded() -> Result<Self, SafetyError> {
        Self::parse(DENY_JSON, SENSITIVE_JSON, SUGGESTED_JSON)
    }

    fn parse(deny: &str, sensitive: &str, suggested: &str) -> Result<Self, SafetyError> {
        Ok(Self {
            deny: parse_names(deny, "deny.json")?,
            sensitive: parse_names(sensitive, "sensitive.json")?,
            suggested: serde_json::from_str(suggested).map_err(|source| SafetyError::List {
                list: "suggested.json",
                source,
            })?,
        })
    }

    /// True when the name is on the deny list. The name may carry the
    /// per-user suffix.
    pub fn is_denied(&self, name: &str) -> bool {
        self.deny.contains(&normalize(name))
    }

    /// True when the name is on the sensitive list.
    pub fn is_sensitive(&self, name: &str) -> bool {
        self.sensitive.contains(&normalize(name))
    }

    /// The plain reason for a suggested service, if it is on that list.
    pub fn suggestion(&self, name: &str) -> Option<&Suggested> {
        let key = normalize(name);
        self.suggested.iter().find(|s| normalize(&s.name) == key)
    }

    /// All suggested services, in file order.
    pub fn suggested(&self) -> &[Suggested] {
        &self.suggested
    }

    /// Run the check list. The first match wins.
    ///
    /// `never_touch` is the user list from `config.toml`. It can only add
    /// blocks. It can never remove one.
    pub fn assess(&self, facts: &ServiceFacts, never_touch: &[String]) -> Verdict {
        let key = normalize(&facts.name);

        if self.deny.contains(&key) {
            return Verdict::blocked(Reason::DenyList);
        }
        if never_touch.iter().any(|name| normalize(name) == key) {
            return Verdict::blocked(Reason::NeverTouch);
        }
        if !facts.is_win32 {
            return Verdict::blocked(Reason::NotWin32Service);
        }
        if facts.is_driver {
            return Verdict::blocked(Reason::Driver);
        }
        if !facts.accepts_stop {
            return Verdict::blocked(Reason::StopNotAccepted);
        }
        if self.sensitive.contains(&key) {
            return Verdict::critical(Reason::SensitiveList);
        }
        if facts.running_dependents > 0 {
            return Verdict::critical(Reason::RunningDependents);
        }

        Verdict {
            level: Level::Normal,
            reason: Reason::None,
        }
    }
}

fn parse_names(json: &str, list: &'static str) -> Result<HashSet<String>, SafetyError> {
    let names: Vec<String> =
        serde_json::from_str(json).map_err(|source| SafetyError::List { list, source })?;
    Ok(names.iter().map(|name| normalize(name)).collect())
}

/// Lowercase the name and drop the per-user suffix, for example
/// `CDPUserSvc_3f2a1` becomes `cdpusersvc`.
///
/// Windows 11 makes one copy of some services for each user. The suffix is a
/// hex run after the last underscore. The length is not fixed.
pub(crate) fn normalize(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match lower.rsplit_once('_') {
        Some((head, tail))
            if !head.is_empty()
                && !tail.is_empty()
                && tail.len() <= 8
                && tail.chars().all(|c| c.is_ascii_hexdigit()) =>
        {
            head.to_string()
        }
        _ => lower,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lists() -> SafetyLists {
        SafetyLists::embedded().expect("the embedded lists must parse")
    }

    fn facts(name: &str) -> ServiceFacts {
        ServiceFacts {
            name: name.to_string(),
            accepts_stop: true,
            is_driver: false,
            is_win32: true,
            running_dependents: 0,
        }
    }

    fn assert_blocked(name: &str, reason: Reason) {
        let verdict = lists().assess(&facts(name), &[]);
        assert_eq!(verdict.level, Level::Blocked, "{name}");
        assert_eq!(verdict.reason, reason, "{name}");
    }

    #[test]
    fn should_parse_the_embedded_lists() {
        let lists = lists();
        assert!(lists.is_denied("RpcSs"));
        assert!(lists.is_sensitive("wuauserv"));
        assert!(lists.suggestion("Spooler").is_some());
    }

    #[test]
    fn should_fail_on_a_malformed_embedded_list() {
        let error = SafetyLists::parse("{ not json", "[]", "[]").unwrap_err();
        assert!(matches!(
            error,
            SafetyError::List {
                list: "deny.json",
                ..
            }
        ));
    }

    #[test]
    fn should_block_service_on_deny_list() {
        assert_blocked("RpcSs", Reason::DenyList);
        assert_blocked("WinDefend", Reason::DenyList);
    }

    #[test]
    fn should_block_service_on_never_touch_list() {
        let never_touch = vec!["SomeGameService".to_string()];
        let verdict = lists().assess(&facts("somegameservice"), &never_touch);
        assert_eq!(verdict.level, Level::Blocked);
        assert_eq!(verdict.reason, Reason::NeverTouch);
    }

    #[test]
    fn should_block_service_that_does_not_accept_stop() {
        let mut service = facts("Spooler");
        service.accepts_stop = false;
        let verdict = lists().assess(&service, &[]);
        assert_eq!(verdict.level, Level::Blocked);
        assert_eq!(verdict.reason, Reason::StopNotAccepted);
    }

    #[test]
    fn should_block_driver() {
        let mut service = facts("SomeDriver");
        service.is_driver = true;
        let verdict = lists().assess(&service, &[]);
        assert_eq!(verdict.level, Level::Blocked);
        assert_eq!(verdict.reason, Reason::Driver);
    }

    #[test]
    fn should_block_non_win32_service() {
        let mut service = facts("SomeLegacyService");
        service.is_win32 = false;
        let verdict = lists().assess(&service, &[]);
        assert_eq!(verdict.level, Level::Blocked);
        assert_eq!(verdict.reason, Reason::NotWin32Service);
    }

    #[test]
    fn should_mark_sensitive_service_critical() {
        let verdict = lists().assess(&facts("Audiosrv"), &[]);
        assert_eq!(verdict.level, Level::Critical);
        assert_eq!(verdict.reason, Reason::SensitiveList);
    }

    #[test]
    fn should_mark_service_with_running_dependents_critical() {
        let mut service = facts("Spooler");
        service.running_dependents = 2;
        let verdict = lists().assess(&service, &[]);
        assert_eq!(verdict.level, Level::Critical);
        assert_eq!(verdict.reason, Reason::RunningDependents);
    }

    #[test]
    fn should_allow_normal_service() {
        let verdict = lists().assess(&facts("Spooler"), &[]);
        assert_eq!(verdict.level, Level::Normal);
        assert_eq!(verdict.reason, Reason::None);
        assert!(verdict.stop_allowed());
    }

    #[test]
    fn should_compare_names_without_case() {
        assert_blocked("rpcss", Reason::DenyList);
        assert_blocked("RPCSS", Reason::DenyList);
        assert_blocked("winDEFEND", Reason::DenyList);
    }

    #[test]
    fn should_ignore_the_per_user_suffix() {
        assert_blocked("RpcSs_3f2a1", Reason::DenyList);
        assert!(lists().is_denied("mpssvc_1a"));

        let verdict = lists().assess(&facts("Audiosrv_bdb01"), &[]);
        assert_eq!(verdict.level, Level::Critical);
        assert_eq!(verdict.reason, Reason::SensitiveList);
    }

    #[test]
    fn should_keep_the_name_when_the_suffix_is_not_hex() {
        assert_eq!(normalize("My_Service"), "my_service");
        assert_eq!(normalize("CDPUserSvc_3f2a1"), "cdpusersvc");
        assert_eq!(normalize("Spooler"), "spooler");
        assert_eq!(normalize("SysMain"), "sysmain");
    }

    #[test]
    fn should_block_before_it_warns() {
        let mut service = facts("Audiosrv");
        service.running_dependents = 3;
        let verdict = lists().assess(&service, &[]);
        assert_eq!(verdict.reason, Reason::SensitiveList);

        let never_touch = vec!["Audiosrv".to_string()];
        let verdict = lists().assess(&facts("Audiosrv"), &never_touch);
        assert_eq!(verdict.level, Level::Blocked);
        assert_eq!(verdict.reason, Reason::NeverTouch);
    }

    #[test]
    fn should_not_allow_a_never_touch_entry_to_unblock_a_denied_service() {
        let never_touch = vec!["RpcSs".to_string()];
        let verdict = lists().assess(&facts("RpcSs"), &never_touch);
        assert_eq!(verdict.level, Level::Blocked);
        assert_eq!(verdict.reason, Reason::DenyList);
    }

    #[test]
    fn should_not_put_a_name_in_both_deny_and_suggested_lists() {
        let lists = lists();
        for suggested in lists.suggested() {
            assert!(
                !lists.is_denied(&suggested.name),
                "{} is on the deny list and the suggested list",
                suggested.name
            );
            assert!(
                !lists.is_sensitive(&suggested.name),
                "{} is on the sensitive list and the suggested list",
                suggested.name
            );
        }
    }

    #[test]
    fn should_not_repeat_a_name_inside_one_list() {
        for json in [DENY_JSON, SENSITIVE_JSON] {
            let names: Vec<String> = serde_json::from_str(json).expect("valid JSON");
            let unique: HashSet<String> = names.iter().map(|name| normalize(name)).collect();
            assert_eq!(names.len(), unique.len(), "a name is repeated in {json}");
        }

        let suggested: Vec<Suggested> = serde_json::from_str(SUGGESTED_JSON).expect("valid JSON");
        let unique: HashSet<String> = suggested
            .iter()
            .map(|service| normalize(&service.name))
            .collect();
        assert_eq!(suggested.len(), unique.len());
    }

    #[test]
    fn should_give_every_suggested_service_a_reason_and_an_effect() {
        for service in lists().suggested() {
            assert!(!service.display.trim().is_empty(), "{}", service.name);
            assert!(!service.reason.trim().is_empty(), "{}", service.name);
            assert!(!service.effect.trim().is_empty(), "{}", service.name);
        }
    }
}
