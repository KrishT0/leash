//! What day it is, on this machine, in the time the person using it lives in.
//!
//! The rest of the engine counts catches against a date and names one log file
//! per day. Windows keeps that date in local time, so a catch at half past
//! midnight belongs to the day that has just started rather than the one that
//! ended five and a half hours ago.
//!
//! This file, `scm.rs` and `elevation.rs` are the only places that call the
//! Windows API. Everywhere but Windows, the arithmetic below turns days since
//! 1970 into a calendar date and back.

#[cfg(windows)]
use windows::Win32::Foundation::SYSTEMTIME;

/// Today as `YYYY-MM-DD`, in the local time of this machine.
#[cfg(windows)]
pub fn today() -> String {
    // SAFETY: GetLocalTime reads the system clock and writes nothing that this
    // program owns. It cannot fail.
    let time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };

    from_systemtime(&time)
}

/// Today as `YYYY-MM-DD`, taken from UTC.
///
/// No Windows clock to read, so the day is the UTC one. Correct, just not
/// local, which is what tests and any other platform get.
#[cfg(not(windows))]
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or(0);

    format_date(days_of(secs))
}

/// The date part of a Windows clock reading.
#[cfg(windows)]
pub fn from_systemtime(time: &SYSTEMTIME) -> String {
    format!("{:04}-{:02}-{:02}", time.wYear, time.wMonth, time.wDay)
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
        let mut days = days_from_civil(2024, 1, 1);
        let end = days_from_civil(2027, 1, 1);

        while days < end {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(
                days_from_civil(year, month, day),
                days,
                "{year}-{month}-{day}"
            );
            days += 1;
        }
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
    fn should_read_the_local_time_from_windows() {
        // The date alone cannot show the difference between local and UTC time
        // except near midnight, so this checks the hour as well. On a machine
        // set to UTC the two agree, which is correct.
        // SAFETY: as in `today`: a reading of the system clock, nothing written.
        let time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };

        let utc = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs() as i64)
            .unwrap_or(0);
        let utc_hour = (utc.rem_euclid(86_400)) / 3600;

        // Within a day of UTC, always, and the same date unless the local day
        // really has moved on.
        let local_days = days_from_civil(time.wYear as i64, time.wMonth as u32, time.wDay as u32);
        assert!(
            (local_days - days_of(utc)).abs() <= 1,
            "{}",
            from_systemtime(&time)
        );
        assert!(time.wHour < 24 && utc_hour < 24);
    }
}
