//! The activity log: one JSON object per line, with rotation and daily counts.
//!
//! The log lives in `%LOCALAPPDATA%\Leash\logs\`. One file holds one day.
//! When a file passes 1 MB, the day gets another part.

use crate::config::write_atomic;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Days of history to keep.
pub const KEEP_DAYS: i64 = 30;
/// Largest file before the day rolls to another part.
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;

/// One line of the activity log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// RFC 3339 UTC, for example `2026-10-11T18:58:02Z`.
    pub ts: String,
    /// Internal service name. Empty for an engine error.
    pub service: String,
    /// What happened, for example `started`.
    pub event: String,
    /// `stop`, `dry_run`, or `notify`. None when the engine took no action.
    pub action: Option<String>,
    /// `success`, `failure`, or None.
    pub result: Option<String>,
    /// Error text, or None.
    pub error: Option<String>,
}

impl Entry {
    /// A new entry with the current time.
    pub fn new(service: &str, event: &str) -> Self {
        Self {
            ts: now_timestamp(),
            service: service.to_string(),
            event: event.to_string(),
            action: None,
            result: None,
            error: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("cannot use the log folder {dir}: {source}")]
    Dir {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot encode a log line: {source}")]
    Encode {
        #[source]
        source: serde_json::Error,
    },
    #[error("cannot parse {path}: {source}")]
    StatsRead {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

/// The activity log writer. One instance per app.
#[derive(Debug)]
pub struct ActivityLog {
    dir: PathBuf,
    /// The day that was last pruned, so the folder is scanned once per day.
    pruned_for: Option<String>,
}

impl ActivityLog {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            pruned_for: None,
        }
    }

    /// The folder this log writes to.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Add one line. The file is created when it is missing.
    pub fn append(&mut self, entry: &Entry) -> Result<(), LogError> {
        let date = today();
        let path = self.file_for(&date)?;

        if self.pruned_for.as_deref() != Some(date.as_str()) {
            self.pruned_for = Some(date);
            self.prune();
        }

        let mut line =
            serde_json::to_string(entry).map_err(|source| LogError::Encode { source })?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| LogError::Write {
                path: path.clone(),
                source,
            })?;
        file.write_all(line.as_bytes())
            .map_err(|source| LogError::Write { path, source })?;

        Ok(())
    }

    /// The file for today, or the next part of it when it is full.
    fn file_for(&self, date: &str) -> Result<PathBuf, LogError> {
        fs::create_dir_all(&self.dir).map_err(|source| LogError::Dir {
            dir: self.dir.clone(),
            source,
        })?;

        let first = self.dir.join(format!("activity-{date}.jsonl"));
        if !is_full(&first) {
            return Ok(first);
        }

        let mut part = 1;
        loop {
            let candidate = self.dir.join(format!("activity-{date}.{part}.jsonl"));
            if !is_full(&candidate) {
                return Ok(candidate);
            }
            part += 1;
        }
    }

    /// Remove files that are outside the keep window. Best effort: a file
    /// that cannot be removed is reported and skipped.
    fn prune(&self) {
        let Some(cutoff) = parse_date(&today()).map(|days| days - KEEP_DAYS) else {
            return;
        };
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Some(days) = file_date(name) else {
                continue;
            };
            if days < cutoff
                && let Err(error) = fs::remove_file(entry.path())
            {
                tracing::warn!(file = %entry.path().display(), %error, "cannot remove an old log file");
            }
        }
    }
}

/// `%LOCALAPPDATA%\Leash\logs`. None when `LOCALAPPDATA` is not set.
pub fn log_dir() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(local).join("Leash").join("logs"))
}

/// `%LOCALAPPDATA%\Leash\stats.json`. None when `LOCALAPPDATA` is not set.
pub fn stats_path() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(local).join("Leash").join("stats.json"))
}

/// Count one catch for a date. Returns the new count for that date.
pub fn bump_catch(path: &Path, date: &str) -> Result<u32, LogError> {
    let mut stats = read_stats(path)?;
    let count = {
        let counted = stats.entry(date.to_string()).or_insert(0);
        *counted += 1;
        *counted
    };

    let text =
        serde_json::to_string_pretty(&stats).map_err(|source| LogError::Encode { source })?;
    write_atomic(path, &text).map_err(|source| LogError::Write {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(count)
}

/// Read the catch counts. A missing file gives an empty map.
pub fn read_stats(path: &Path) -> Result<BTreeMap<String, u32>, LogError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(source) => {
            return Err(LogError::Write {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    serde_json::from_str(&text).map_err(|source| LogError::StatsRead {
        path: path.to_path_buf(),
        source,
    })
}

/// The current time as RFC 3339 UTC.
pub fn now_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or(0);
    format_timestamp(secs.div_euclid(86_400), secs.rem_euclid(86_400))
}

/// Today as `YYYY-MM-DD`.
pub fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or(0);
    format_date_of(secs.div_euclid(86_400))
}

fn format_timestamp(days: i64, secs_of_day: i64) -> String {
    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn format_date_of(days: i64) -> String {
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Days since 1970-01-01 to a calendar date.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_part + 2) / 5 + 1) as u32;
    let month = if month_part < 10 {
        month_part + 3
    } else {
        month_part - 9
    } as u32;
    let year = if month <= 2 { year + 1 } else { year };

    (year, month, day)
}

/// A calendar date to days since 1970-01-01.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_part = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_part + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * 146_097 + day_of_era - 719_468
}

/// `2026-10-11` to days since 1970-01-01.
fn parse_date(text: &str) -> Option<i64> {
    let mut parts = text.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    Some(days_from_civil(year, month, day))
}

/// The time of the newest catch, which is the newest entry whose action was a
/// stop and whose result was a success. Only the newest [`CATCH_SCAN_LIMIT`]
/// entries are looked at, so the call stays cheap on a long log.
pub fn read_latest_catch(dir: &Path) -> Result<Option<String>, LogError> {
    for entry in read_entries(dir, CATCH_SCAN_LIMIT)? {
        if entry.action.as_deref() == Some("stop") && entry.result.as_deref() == Some("success") {
            return Ok(Some(entry.ts));
        }
    }

    Ok(None)
}

/// How far back the newest catch is looked for.
const CATCH_SCAN_LIMIT: usize = 200;

/// The newest entries, newest first, up to `limit`.
pub fn read_entries(dir: &Path, limit: usize) -> Result<Vec<Entry>, LogError> {
    let listing = match fs::read_dir(dir) {
        Ok(listing) => listing,
        // No log folder yet simply means nothing has happened.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(LogError::Dir {
                dir: dir.to_path_buf(),
                source,
            });
        }
    };

    let mut files = Vec::new();
    for item in listing.flatten() {
        let name = item.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if let Some((date, part)) = log_file_order(name) {
            files.push((date, part, item.path()));
        }
    }
    // Newest day first, and inside a day the highest part first.
    files.sort_by(|left, right| (right.0, right.1).cmp(&(left.0, left.1)));

    let mut entries = Vec::new();
    for (_, _, path) in files {
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                tracing::warn!(file = %path.display(), %error, "cannot read a log file");
                continue;
            }
        };

        for line in text.lines().rev() {
            if entries.len() >= limit {
                return Ok(entries);
            }
            match serde_json::from_str::<Entry>(line) {
                Ok(entry) => entries.push(entry),
                // A line cut short by a power loss is skipped, not fatal.
                Err(error) => tracing::warn!(%error, "cannot parse a log line"),
            }
        }
    }

    Ok(entries)
}

/// The day and the part number in `activity-2026-10-11.jsonl` and
/// `activity-2026-10-11.2.jsonl`.
fn log_file_order(name: &str) -> Option<(i64, u32)> {
    let rest = name.strip_prefix("activity-")?;
    let date = parse_date(rest.get(..10)?)?;
    let tail = rest.get(10..)?;

    if tail == ".jsonl" {
        return Some((date, 0));
    }

    let part = tail
        .strip_prefix('.')?
        .strip_suffix(".jsonl")?
        .parse()
        .ok()?;

    Some((date, part))
}

/// The date in `activity-2026-10-11.jsonl` or `activity-2026-10-11.2.jsonl`.
fn file_date(name: &str) -> Option<i64> {
    let rest = name.strip_prefix("activity-")?;
    parse_date(rest.get(..10)?)
}

fn is_full(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.len() >= MAX_FILE_BYTES)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        // Tests run in parallel, so each call gets its own directory.
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("leash-log-test-{}-{tag}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn lines(path: &Path) -> Vec<String> {
        fs::read_to_string(path)
            .expect("read")
            .lines()
            .map(|line| line.to_string())
            .collect()
    }

    #[test]
    fn should_convert_days_to_a_date() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(civil_from_days(20_737), (2026, 10, 11));
    }

    #[test]
    fn should_convert_a_date_to_days() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(days_from_civil(2024, 2, 29), 19_782);
        assert_eq!(days_from_civil(2026, 10, 11), 20_737);
    }

    #[test]
    fn should_round_trip_the_dates_of_three_years() {
        for days in 20_000..21_100 {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(
                days_from_civil(year, month, day),
                days,
                "{year}-{month}-{day}"
            );
        }
    }

    #[test]
    fn should_format_the_documented_timestamp() {
        assert_eq!(format_timestamp(20_737, 68_282), "2026-10-11T18:58:02Z");
        assert_eq!(format_timestamp(0, 0), "1970-01-01T00:00:00Z");
        assert_eq!(format_timestamp(19_782, 86_399), "2024-02-29T23:59:59Z");
    }

    #[test]
    fn should_write_a_timestamp_that_looks_like_the_documented_one() {
        let stamp = now_timestamp();
        assert_eq!(stamp.len(), 20, "{stamp}");
        assert!(stamp.ends_with('Z'), "{stamp}");
        assert_eq!(stamp.get(10..11), Some("T"), "{stamp}");
        assert_eq!(stamp.get(4..5), Some("-"), "{stamp}");
    }

    #[test]
    fn should_report_today_as_a_date() {
        let date = today();
        assert_eq!(date.len(), 10, "{date}");
        assert!(parse_date(&date).is_some(), "{date}");
    }

    #[test]
    fn should_parse_a_date_from_a_file_name() {
        assert_eq!(file_date("activity-2026-10-11.jsonl"), Some(20_737));
        assert_eq!(file_date("activity-2026-10-11.2.jsonl"), Some(20_737));
        assert_eq!(file_date("activity-not-a-date.jsonl"), None);
        assert_eq!(file_date("other-2026-10-11.jsonl"), None);
        assert_eq!(file_date("activity-2026-13-11.jsonl"), None);
    }

    #[test]
    fn should_append_one_json_line_per_entry() {
        let dir = temp_dir("append");
        let mut log = ActivityLog::new(dir.clone());

        let mut first = Entry::new("Spooler", "started");
        first.action = Some("stop".to_string());
        first.result = Some("success".to_string());
        log.append(&first).expect("append");

        let mut second = Entry::new("DiagTrack", "started");
        second.action = Some("dry_run".to_string());
        second.result = Some("would_stop".to_string());
        log.append(&second).expect("append");

        let written = lines(&dir.join(format!("activity-{}.jsonl", today())));
        assert_eq!(written.len(), 2);
        assert!(written[0].contains("\"Spooler\""), "{}", written[0]);
        assert!(written[1].contains("\"DiagTrack\""), "{}", written[1]);
    }

    #[test]
    fn should_write_the_documented_log_shape() {
        let dir = temp_dir("shape");
        let mut log = ActivityLog::new(dir.clone());

        let mut entry = Entry::new("Spooler", "started");
        entry.action = Some("stop".to_string());
        entry.result = Some("success".to_string());
        log.append(&entry).expect("append");

        let line = lines(&dir.join(format!("activity-{}.jsonl", today()))).remove(0);
        assert!(line.starts_with("{\"ts\":\""), "{line}");

        let value: serde_json::Value = serde_json::from_str(&line).expect("json");

        assert_eq!(value["service"], "Spooler");
        assert_eq!(value["event"], "started");
        assert_eq!(value["action"], "stop");
        assert_eq!(value["result"], "success");
        assert_eq!(value["error"], serde_json::Value::Null);
        assert!(value["ts"].as_str().expect("ts").ends_with('Z'));

        // A JSON object is unordered, so check the names and the count.
        let object = value.as_object().expect("object");
        assert_eq!(object.len(), 6);
        for key in ["ts", "service", "event", "action", "result", "error"] {
            assert!(object.contains_key(key), "{key} is missing");
        }
    }

    #[test]
    fn should_roll_to_a_new_part_when_the_file_is_full() {
        let dir = temp_dir("roll");
        let mut log = ActivityLog::new(dir.clone());
        let date = today();

        let mut entry = Entry::new("Spooler", "started");
        entry.error = Some("x".repeat(100));
        let per_line = serde_json::to_string(&entry).expect("json").len() + 1;
        let wanted = MAX_FILE_BYTES as usize / per_line + 10;

        for _ in 0..wanted {
            log.append(&entry).expect("append");
        }

        assert!(dir.join(format!("activity-{date}.jsonl")).exists());
        assert!(dir.join(format!("activity-{date}.1.jsonl")).exists());
    }

    #[test]
    fn should_prune_files_outside_the_keep_window() {
        let dir = temp_dir("prune");
        let old = format_date_of(parse_date(&today()).expect("today") - KEEP_DAYS - 1);
        let old_file = dir.join(format!("activity-{old}.jsonl"));
        fs::write(&old_file, "{}\n").expect("write");

        let mut log = ActivityLog::new(dir.clone());
        log.append(&Entry::new("Spooler", "started"))
            .expect("append");

        assert!(!old_file.exists(), "the old file must be removed");
        assert!(dir.join(format!("activity-{}.jsonl", today())).exists());
    }

    #[test]
    fn should_keep_a_file_inside_the_keep_window() {
        let dir = temp_dir("keep");
        let recent = format_date_of(parse_date(&today()).expect("today") - KEEP_DAYS + 1);
        let recent_file = dir.join(format!("activity-{recent}.jsonl"));
        fs::write(&recent_file, "{}\n").expect("write");

        let mut log = ActivityLog::new(dir.clone());
        log.append(&Entry::new("Spooler", "started"))
            .expect("append");

        assert!(recent_file.exists(), "a recent file must stay");
    }

    #[test]
    fn should_not_prune_a_file_whose_name_it_cannot_read() {
        let dir = temp_dir("odd-name");
        let odd = dir.join("activity-not-a-date.jsonl");
        fs::write(&odd, "{}\n").expect("write");

        let mut log = ActivityLog::new(dir.clone());
        log.append(&Entry::new("Spooler", "started"))
            .expect("append");

        assert!(odd.exists());
    }

    #[test]
    fn should_create_the_log_folder() {
        let dir = temp_dir("create").join("logs");
        let mut log = ActivityLog::new(dir.clone());
        log.append(&Entry::new("Spooler", "started"))
            .expect("append");

        assert!(dir.is_dir());
        assert_eq!(log.dir(), dir.as_path());
    }

    #[test]
    fn should_read_the_newest_entries_first() {
        let dir = temp_dir("read");
        let mut log = ActivityLog::new(dir.clone());

        for service in ["One", "Two", "Three"] {
            log.append(&Entry::new(service, "started")).expect("append");
        }

        let entries = read_entries(&dir, 2).expect("read");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].service, "Three");
        assert_eq!(entries[1].service, "Two");
    }

    #[test]
    fn should_read_every_entry_when_the_limit_is_large() {
        let dir = temp_dir("read-all");
        let mut log = ActivityLog::new(dir.clone());
        for service in ["One", "Two"] {
            log.append(&Entry::new(service, "started")).expect("append");
        }

        assert_eq!(read_entries(&dir, 100).expect("read").len(), 2);
        assert!(read_entries(&dir, 0).expect("read").is_empty());
    }

    #[test]
    fn should_order_the_files_by_day_and_part() {
        assert_eq!(
            log_file_order("activity-2026-10-11.jsonl"),
            Some((20_737, 0))
        );
        assert_eq!(
            log_file_order("activity-2026-10-11.2.jsonl"),
            Some((20_737, 2))
        );
        assert_eq!(
            log_file_order("activity-2026-10-12.jsonl"),
            Some((20_738, 0))
        );
        assert_eq!(log_file_order("activity-nope.jsonl"), None);
        assert_eq!(log_file_order("other-2026-10-11.jsonl"), None);
        assert_eq!(log_file_order("activity-2026-10-11.2.txt"), None);
    }

    #[test]
    fn should_read_the_newest_day_before_an_older_one() {
        let dir = temp_dir("read-days");
        let line = |service: &str| {
            format!(
                "{{\"ts\":\"2026-01-01T00:00:00Z\",\"service\":\"{service}\",\"event\":\"started\",\"action\":null,\"result\":null,\"error\":null}}\n"
            )
        };
        fs::write(dir.join("activity-2026-10-10.jsonl"), line("Older")).expect("write");
        fs::write(
            dir.join("activity-2026-10-11.2.jsonl"),
            line("NewerPartTwo"),
        )
        .expect("write");
        fs::write(dir.join("activity-2026-10-11.jsonl"), line("NewerPartOne")).expect("write");

        let entries = read_entries(&dir, 10).expect("read");
        let names: Vec<&str> = entries.iter().map(|e| e.service.as_str()).collect();
        assert_eq!(names, ["NewerPartTwo", "NewerPartOne", "Older"]);
    }

    #[test]
    fn should_give_no_entries_when_the_folder_is_missing() {
        let dir = temp_dir("read-missing").join("gone");
        assert!(read_entries(&dir, 10).expect("read").is_empty());
    }

    #[test]
    fn should_skip_a_broken_line() {
        let dir = temp_dir("read-broken");
        let name = format!("activity-{}.jsonl", today());
        fs::write(
            dir.join(name),
            "not json\n{\"ts\":\"t\",\"service\":\"Good\",\"event\":\"started\",\"action\":null,\"result\":null,\"error\":null}\n",
        )
        .expect("write");

        let entries = read_entries(&dir, 10).expect("read");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].service, "Good");
    }

    #[test]
    fn should_find_the_newest_catch() {
        let dir = temp_dir("latest-catch");
        let mut log = ActivityLog::new(dir.clone());

        // Nothing happened yet.
        assert_eq!(read_latest_catch(&dir).expect("read"), None);

        // A blocked service is not a catch.
        let mut blocked = Entry::new("One", "started");
        blocked.result = Some("blocked".to_string());
        log.append(&blocked).expect("append");
        assert_eq!(read_latest_catch(&dir).expect("read"), None);

        let mut caught = Entry::new("Two", "started");
        caught.action = Some("stop".to_string());
        caught.result = Some("success".to_string());
        log.append(&caught).expect("append");

        let stamp = caught.ts.clone();
        assert_eq!(read_latest_catch(&dir).expect("read"), Some(stamp.clone()));

        // A later notice does not move the newest catch.
        log.append(&Entry::new("Three", "started")).expect("append");
        assert_eq!(read_latest_catch(&dir).expect("read"), Some(stamp));
    }

    #[test]
    fn should_count_catches_per_date() {
        let dir = temp_dir("stats");
        let path = dir.join("stats.json");

        assert_eq!(bump_catch(&path, "2026-10-11").expect("bump"), 1);
        assert_eq!(bump_catch(&path, "2026-10-11").expect("bump"), 2);
        assert_eq!(bump_catch(&path, "2026-10-12").expect("bump"), 1);

        let stats = read_stats(&path).expect("read");
        assert_eq!(stats.get("2026-10-11"), Some(&2));
        assert_eq!(stats.get("2026-10-12"), Some(&1));
        assert_eq!(stats.len(), 2);
    }

    #[test]
    fn should_give_empty_stats_when_the_file_is_missing() {
        let path = temp_dir("no-stats").join("stats.json");
        assert!(read_stats(&path).expect("read").is_empty());
    }

    #[test]
    fn should_report_broken_stats_json() {
        let path = temp_dir("broken-stats").join("stats.json");
        fs::write(&path, "not json").expect("write");
        assert!(matches!(read_stats(&path), Err(LogError::StatsRead { .. })));
    }

    #[test]
    fn should_leave_no_temp_file_behind_after_a_stats_write() {
        let path = temp_dir("stats-temp").join("stats.json");
        bump_catch(&path, "2026-10-11").expect("bump");

        assert!(!PathBuf::from(format!("{}.tmp", path.display())).exists());
    }
}
