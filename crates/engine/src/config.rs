//! The config file: load, validate, and save.
//!
//! The file lives in `%APPDATA%\Leash\config.toml`. It is untrusted input. A
//! normal process can edit it, so every entry is checked on load.

use crate::safety::normalize;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Longest accepted service name.
const MAX_NAME_LEN: usize = 256;

/// What the watchdog does when a monitored service starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    /// Log the event and tell the user. Do not stop.
    #[default]
    Notify,
    /// Log what a stop would do. Do not stop.
    DryRun,
    /// Stop the service.
    Stop,
}

/// App-wide switches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct General {
    /// The watchdog takes no action while this is true.
    pub paused: bool,
    pub notifications: bool,
    pub start_with_windows: bool,
    pub start_minimized: bool,
    pub start_watchdog: bool,
}

impl Default for General {
    fn default() -> Self {
        Self {
            paused: false,
            notifications: true,
            start_with_windows: true,
            start_minimized: true,
            start_watchdog: true,
        }
    }
}

/// The `[never_touch]` table. The user can add names here. Leash never touches
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct NeverTouch {
    pub names: Vec<String>,
}

/// One `[[services]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Monitored {
    pub name: String,
    /// A new monitored service always starts at `notify`.
    pub policy: Policy,
}

/// The whole config file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub general: General,
    pub never_touch: NeverTouch,
    pub services: Vec<Monitored>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("cannot build the config text: {source}")]
    Build {
        #[source]
        source: toml::ser::Error,
    },
    #[error("the config has a bad entry: {reason}")]
    Invalid { reason: String },
}

impl Config {
    /// Read and check the file. A missing file gives the defaults.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };

        let config: Self = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;

        validate(config)
    }

    /// Write the file. The write is atomic: a temp file goes next to the
    /// target and a rename replaces it.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let text = toml::to_string_pretty(self).map_err(|source| ConfigError::Build { source })?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let mut temp = path.as_os_str().to_owned();
        temp.push(".tmp");
        let temp = PathBuf::from(temp);

        if let Err(source) = fs::write(&temp, text) {
            return Err(ConfigError::Write { path: temp, source });
        }
        if let Err(source) = fs::rename(&temp, path) {
            let _ = fs::remove_file(&temp);
            return Err(ConfigError::Write {
                path: path.to_path_buf(),
                source,
            });
        }

        Ok(())
    }

    /// True when the name is protected by the user.
    pub fn is_never_touch(&self, name: &str) -> bool {
        let key = normalize(name);
        self.never_touch
            .names
            .iter()
            .any(|entry| normalize(entry) == key)
    }

    /// The entry for a service, if it is monitored.
    pub fn monitored(&self, name: &str) -> Option<&Monitored> {
        let key = normalize(name);
        self.services.iter().find(|s| normalize(&s.name) == key)
    }
}

/// `%APPDATA%\Leash\config.toml`. None when `APPDATA` is not set.
pub fn config_path() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(appdata).join("Leash").join("config.toml"))
}

/// Check every entry. Names are trimmed in place.
fn validate(mut config: Config) -> Result<Config, ConfigError> {
    let mut seen = HashSet::new();
    let mut services = Vec::with_capacity(config.services.len());
    for mut service in config.services.drain(..) {
        service.name = clean_name(&service.name)?;
        if !seen.insert(normalize(&service.name)) {
            return Err(ConfigError::Invalid {
                reason: format!("{} is listed more than once", service.name),
            });
        }
        services.push(service);
    }
    config.services = services;

    let mut seen = HashSet::new();
    let mut names = Vec::with_capacity(config.never_touch.names.len());
    for name in config.never_touch.names.drain(..) {
        let name = clean_name(&name)?;
        if !seen.insert(normalize(&name)) {
            return Err(ConfigError::Invalid {
                reason: format!("{name} is listed more than once in never_touch"),
            });
        }
        names.push(name);
    }
    config.never_touch.names = names;

    Ok(config)
}

/// Trim a name and reject anything that cannot be a service name.
fn clean_name(name: &str) -> Result<String, ConfigError> {
    let name = name.trim();

    if name.is_empty() {
        return Err(ConfigError::Invalid {
            reason: "a name is empty".to_string(),
        });
    }
    if name.len() > MAX_NAME_LEN {
        return Err(ConfigError::Invalid {
            reason: format!("a name is longer than {MAX_NAME_LEN} bytes"),
        });
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err(ConfigError::Invalid {
            reason: format!("{name} has a character that a service name cannot have"),
        });
    }

    Ok(name.to_string())
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        // Tests run in parallel, so each call gets its own directory.
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "leash-config-test-{}-{tag}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir.join("config.toml")
    }

    fn load_text(text: &str) -> Result<Config, ConfigError> {
        let path = temp_path("load");
        fs::write(&path, text).expect("write");
        Config::load(&path)
    }

    #[test]
    fn should_return_defaults_when_the_file_is_missing() {
        let path = temp_path("missing");
        let config = Config::load(&path).expect("load");
        assert_eq!(config, Config::default());
        assert!(config.general.notifications);
        assert!(config.general.start_with_windows);
        assert!(!config.general.paused);
        assert!(config.services.is_empty());
    }

    #[test]
    fn should_default_a_new_monitored_service_to_notify() {
        assert_eq!(Policy::default(), Policy::Notify);
    }

    #[test]
    fn should_round_trip_a_config_through_a_file() {
        let path = temp_path("round-trip");
        let mut config = Config::default();
        config.general.paused = true;
        config.never_touch.names = vec!["SomeGameService".to_string()];
        config.services = vec![
            Monitored {
                name: "Spooler".to_string(),
                policy: Policy::DryRun,
            },
            Monitored {
                name: "DiagTrack".to_string(),
                policy: Policy::Stop,
            },
        ];

        config.save(&path).expect("save");
        assert_eq!(Config::load(&path).expect("load"), config);
    }

    #[test]
    fn should_write_the_documented_shape() {
        let path = temp_path("shape");
        let mut config = Config::default();
        config.never_touch.names = vec!["SomeGameService".to_string()];
        config.services = vec![Monitored {
            name: "Spooler".to_string(),
            policy: Policy::Notify,
        }];
        config.save(&path).expect("save");

        let text = fs::read_to_string(&path).expect("read");
        assert!(text.contains("[general]"), "{text}");
        assert!(text.contains("[never_touch]"), "{text}");
        assert!(text.contains("[[services]]"), "{text}");
        assert!(text.contains("policy = \"notify\""), "{text}");
        assert!(text.contains("paused = false"), "{text}");
    }

    #[test]
    fn should_leave_no_temp_file_behind() {
        let path = temp_path("temp-file");
        Config::default().save(&path).expect("save");

        let temp = PathBuf::from(format!("{}.tmp", path.display()));
        assert!(!temp.exists());
        assert!(path.exists());
    }

    #[test]
    fn should_overwrite_an_existing_file() {
        let path = temp_path("overwrite");
        Config::default().save(&path).expect("first save");

        let mut config = Config::default();
        config.general.paused = true;
        config.save(&path).expect("second save");

        assert!(Config::load(&path).expect("load").general.paused);
    }

    #[test]
    fn should_keep_the_defaults_for_a_partial_file() {
        let config = load_text("[general]\npaused = true\n").expect("load");
        assert!(config.general.paused);
        assert!(config.general.notifications);
    }

    #[test]
    fn should_reject_an_unknown_policy() {
        let text = "[[services]]\nname = \"Spooler\"\npolicy = \"kill\"\n";
        assert!(matches!(load_text(text), Err(ConfigError::Parse { .. })));
    }

    #[test]
    fn should_reject_an_unknown_key() {
        let text = "[general]\npaused = true\nfast = true\n";
        assert!(matches!(load_text(text), Err(ConfigError::Parse { .. })));
    }

    #[test]
    fn should_reject_a_deny_list_in_the_config() {
        let text = "[deny]\nnames = [\"RpcSs\"]\n";
        assert!(matches!(load_text(text), Err(ConfigError::Parse { .. })));
    }

    #[test]
    fn should_reject_a_broken_toml_file() {
        assert!(matches!(
            load_text("this is not toml"),
            Err(ConfigError::Parse { .. })
        ));
    }

    #[test]
    fn should_reject_a_duplicate_service_name() {
        let text = "[[services]]\nname = \"Spooler\"\npolicy = \"notify\"\n\n\
                    [[services]]\nname = \"spooler\"\npolicy = \"stop\"\n";
        match load_text(text) {
            Err(ConfigError::Invalid { reason }) => assert!(reason.contains("more than once")),
            other => panic!("expected an invalid entry, got {other:?}"),
        }
    }

    #[test]
    fn should_reject_a_duplicate_never_touch_name() {
        let text = "[never_touch]\nnames = [\"A\", \"a\"]\n";
        assert!(matches!(load_text(text), Err(ConfigError::Invalid { .. })));
    }

    #[test]
    fn should_reject_an_empty_service_name() {
        let text = "[[services]]\nname = \"   \"\npolicy = \"notify\"\n";
        assert!(matches!(load_text(text), Err(ConfigError::Invalid { .. })));
    }

    #[test]
    fn should_reject_a_name_with_a_bad_character() {
        let text = "[[services]]\nname = \"Spooler;rm\"\npolicy = \"notify\"\n";
        assert!(matches!(load_text(text), Err(ConfigError::Invalid { .. })));
    }

    #[test]
    fn should_reject_a_name_that_is_too_long() {
        let long = "A".repeat(MAX_NAME_LEN + 1);
        let text = format!("[[services]]\nname = \"{long}\"\npolicy = \"notify\"\n");
        assert!(matches!(load_text(&text), Err(ConfigError::Invalid { .. })));
    }

    #[test]
    fn should_trim_whitespace_from_names() {
        let text = "[[services]]\nname = \"  Spooler  \"\npolicy = \"notify\"\n";
        let config = load_text(text).expect("load");
        assert_eq!(config.services[0].name, "Spooler");
    }

    #[test]
    fn should_find_a_monitored_service_without_case_or_suffix() {
        let mut config = Config::default();
        config.services = vec![Monitored {
            name: "Spooler".to_string(),
            policy: Policy::Notify,
        }];
        config.never_touch.names = vec!["SomeGameService".to_string()];

        assert!(config.monitored("spooler").is_some());
        assert!(config.monitored("SPOOLER").is_some());
        assert!(config.monitored("Spooler_3f2a1").is_some());
        assert!(config.monitored("DiagTrack").is_none());
        assert!(config.is_never_touch("somegameservice"));
        assert!(!config.is_never_touch("Spooler"));
    }

    #[test]
    fn should_build_the_config_path_from_appdata() {
        // APPDATA exists on Windows, the only supported platform. On another
        // OS the function returns None and there is nothing to check.
        if let Some(path) = config_path() {
            assert!(path.ends_with(Path::new("Leash").join("config.toml")));
        }
    }
}
