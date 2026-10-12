//! What day it is, on this machine, in the time the person using it lives in.
//!
//! The engine counts catches against a date and names one log file per day, and
//! Windows keeps that date in local time. A catch at half past midnight belongs to
//! the day that has just started, not the one that ended hours ago.
//!
//! The date is built from a moment in seconds since the epoch and an offset from UTC
//! in minutes. Windows supplies the offset, and every test supplies its own.
//!
//! This file, `scm.rs` and `elevation.rs` are the only places that call the Windows
//! API.

#[cfg(windows)]
use windows::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};

/// What `GetTimeZoneInformation` returns when the zone is in daylight saving.
///
/// The windows crate exports only `TIME_ZONE_ID_INVALID`, so the other two
/// values are written out here. This one means the daylight bias applies.
#[cfg(windows)]
const TIME_ZONE_ID_DAYLIGHT: u32 = 2;

/// Today as `YYYY-MM-DD`, in the local time of this machine.
pub fn today() -> String {
    date_at(now_seconds(), offset_minutes())
}

/// The date at a moment, shifted by an offset from UTC in minutes.
///
/// India is +330, and half past seven in the evening UTC on 11 October is one in
/// the morning on the 12th there, which is the case this exists for.
pub fn date_at(seconds: i64, offset_minutes: i32) -> String {
    let shifted = seconds + i64::from(offset_minutes) * 60;

    format_date(days_of(shifted))
}

/// Now, in seconds since the epoch.
pub fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or(0)
}

/// How far this machine is from UTC, in minutes, right now.
///
/// The daylight bias is added when the zone is in daylight saving, because the
/// two together are what currently applies. Windows reports the bias the other
/// way round, as UTC minus local, so the sign is flipped here.
#[cfg(windows)]
pub fn offset_minutes() -> i32 {
    let mut info = TIME_ZONE_INFORMATION::default();

    // SAFETY: GetTimeZoneInformation fills the struct it is given and reads
    // nothing else. The pointer comes from a local that outlives the call.
    let id = unsafe { GetTimeZoneInformation(&mut info) };

    let bias = if id == TIME_ZONE_ID_DAYLIGHT {
        info.Bias + info.DaylightBias
    } else {
        info.Bias
    };

    -bias
}

/// A machine with no time zone setting is treated as UTC.
#[cfg(not(windows))]
pub fn offset_minutes() -> i32 {
    0
}

/// A date as `YYYY-MM-DD`.
pub fn format_date(days: i64) -> String {
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Days since 1970-01-01 for a moment given in seconds since the epoch.
pub fn days_of(seconds: i64) -> i64 {
    seconds.div_euclid(86_400)
}

/// Days since 1970-01-01 to a calendar date.
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
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
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) as i64 + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * 146_097 + day_of_era - 719_468
}

/// The moment a date and a time of day name, in seconds since the epoch.
#[cfg(test)]
fn seconds_at(year: i64, month: u32, day: u32, hour: i64, minute: i64) -> i64 {
    days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_convert_days_to_a_date() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_000), (2022, 1, 8));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn should_convert_a_date_to_days() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2022, 1, 8), 19_000);
        assert_eq!(days_from_civil(1969, 12, 31), -1);
    }

    #[test]
    fn should_round_trip_the_dates_of_three_years() {
        let start = days_from_civil(2024, 1, 1);
        let end = days_from_civil(2027, 1, 1);

        for days in start..end {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(
                days_from_civil(year, month, day),
                days,
                "{year}-{month}-{day}"
            );
        }
    }

    #[test]
    fn should_place_a_late_evening_catch_on_the_next_day_in_india() {
        // Half past seven in the evening UTC on 11 October is one in the morning
        // on the 12th in India. This is the case that was wrong: the catch
        // landed on the previous day's bar.
        let moment = seconds_at(2026, 10, 11, 19, 30);

        assert_eq!(date_at(moment, 330), "2026-10-12");
    }

    #[test]
    fn should_leave_a_moment_where_it_is_at_no_offset() {
        let moment = seconds_at(2026, 10, 11, 19, 30);

        assert_eq!(date_at(moment, 0), "2026-10-11");
    }

    #[test]
    fn should_shift_a_moment_backwards_behind_utc() {
        // Five hours behind, so the same evening belongs to the same day, and
        // the small hours belong to the one before.
        assert_eq!(
            date_at(seconds_at(2026, 10, 11, 19, 30), -300),
            "2026-10-11"
        );
        assert_eq!(date_at(seconds_at(2026, 10, 12, 2, 0), -300), "2026-10-11");
    }

    #[test]
    fn should_write_a_date_in_the_documented_shape() {
        let today = today();
        let parts: Vec<&str> = today.split('-').collect();
        assert_eq!(parts.len(), 3, "expected YYYY-MM-DD, got {today}");
    }

    #[test]
    fn should_report_a_date_that_the_engine_can_read_back() {
        // A value the parser cannot read would rename every log file and
        // misplace every catch, so the round trip is the property that matters.
        let today = today();
        let parts: Vec<&str> = today.split('-').collect();
        let year: i64 = parts[0].parse().expect("a year");
        let month: u32 = parts[1].parse().expect("a month");
        let day: u32 = parts[2].parse().expect("a day");

        assert_eq!(format_date(days_from_civil(year, month, day)), today);
    }

    #[cfg(windows)]
    #[test]
    fn should_read_an_offset_from_windows_that_matches_the_local_clock() {
        use windows::Win32::System::SystemInformation::GetLocalTime;

        // SAFETY: GetLocalTime reads the system clock and writes nothing this
        // program owns. This is a second, independent reading of the same clock,
        // which is what catches a sign error in the offset.
        let local = unsafe { GetLocalTime() };
        let offset = offset_minutes();

        assert!((-14 * 60..=14 * 60).contains(&offset), "{offset} minutes");

        let utc = now_seconds();
        let shifted_hour = (utc + i64::from(offset) * 60).rem_euclid(86_400) / 3_600;
        assert_eq!(
            shifted_hour,
            i64::from(local.wHour),
            "the offset of {offset} minutes puts UTC on a different hour than the clock reads"
        );
    }
}
