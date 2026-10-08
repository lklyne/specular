//! The short date on a thread's row: the time for one changed today, else
//! the month and the day, both in local time (`shortDate` in `ChatPane.tsx`).

use std::time::{SystemTime, UNIX_EPOCH};

use objc2::{class, msg_send};

use crate::native::Id;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const DAY: i64 = 86_400;

/// Days from 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let shifted_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The month and the day of the date `days` after 1970-01-01.
const fn month_and_day(days: i64) -> (i64, i64) {
    let z = days + 719_468;
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (month, day)
}

/// Seconds since the epoch of an ISO 8601 UTC timestamp, as `iso8601`
/// writes it: `2026-10-07T09:05:00.000Z`.
fn parse(iso: &str) -> Option<i64> {
    let number = |range: std::ops::Range<usize>| iso.get(range)?.parse::<i64>().ok();
    let punctuated = iso.as_bytes().get(4) == Some(&b'-')
        && iso.as_bytes().get(7) == Some(&b'-')
        && iso.as_bytes().get(10) == Some(&b'T');
    if !punctuated {
        return None;
    }
    let days = days_from_civil(number(0..4)?, number(5..7)?, number(8..10)?);
    Some(days * DAY + number(11..13)? * 3600 + number(14..16)? * 60 + number(17..19)?)
}

/// `iso` as a row shows it, for a clock `offset` seconds ahead of UTC that
/// reads `now` seconds since the epoch. Empty for a timestamp that is not
/// one.
fn short_date_at(iso: &str, now: i64, offset: i64) -> String {
    let Some(then) = parse(iso) else {
        return String::new();
    };
    let (then, now) = (then + offset, now + offset);
    if then.div_euclid(DAY) == now.div_euclid(DAY) {
        let minutes = then.rem_euclid(DAY) / 60;
        let (hour, minute) = (minutes / 60, minutes % 60);
        let half = if hour < 12 { "AM" } else { "PM" };
        let hour = match hour % 12 {
            0 => 12,
            hour => hour,
        };
        return format!("{hour}:{minute:02} {half}");
    }
    let (month, day) = month_and_day(then.div_euclid(DAY));
    let name = usize::try_from(month - 1)
        .ok()
        .and_then(|index| MONTHS.get(index));
    name.map_or_else(String::new, |name| format!("{name} {day}"))
}

/// How far the system's time zone is ahead of UTC now, in seconds.
fn local_offset() -> i64 {
    // SAFETY: `NSTimeZone.localTimeZone` is always there.
    let zone: Id = unsafe { msg_send![class!(NSTimeZone), localTimeZone] };
    // SAFETY: `secondsFromGMT` returns an `NSInteger`.
    let seconds: isize = unsafe { msg_send![zone, secondsFromGMT] };
    seconds as i64
}

/// `iso` as a thread's row shows it now, in local time.
pub(super) fn short_date(iso: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        });
    short_date_at(iso, now, local_offset())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-07T18:30:00Z.
    const NOW: i64 = 1_791_397_800;

    #[test]
    fn a_thread_changed_today_shows_its_local_time_and_an_older_one_its_date() {
        let cases = [
            // Same local day, morning and afternoon, at UTC.
            ("2026-10-07T09:05:00.000Z", 0, "9:05 AM"),
            ("2026-10-07T12:00:00.000Z", 0, "12:00 PM"),
            ("2026-10-07T00:07:59.000Z", 0, "12:07 AM"),
            // Seven hours behind UTC, 01:30Z is still the evening before.
            ("2026-10-08T01:30:00.000Z", -7 * 3600, "6:30 PM"),
            ("2026-10-07T06:59:00.000Z", -7 * 3600, "Oct 6"),
            // Ahead of UTC the local day has already turned.
            ("2026-10-07T13:00:00.000Z", 9 * 3600, "Oct 7"),
            ("2026-10-07T15:10:00.000Z", 9 * 3600, "12:10 AM"),
            ("2024-02-29T23:00:00.000Z", 0, "Feb 29"),
            ("2025-12-31T23:59:59.999Z", 3600, "Jan 1"),
            ("not a date", 0, ""),
            ("", 0, ""),
        ];
        for (iso, offset, expected) in cases {
            assert_eq!(
                short_date_at(iso, NOW, offset),
                expected,
                "{iso} at {offset}"
            );
        }
    }

    #[test]
    fn the_test_clock_is_the_day_it_says() {
        assert_eq!(parse("2026-10-07T18:30:00.000Z"), Some(NOW));
        assert_eq!(parse("1970-01-01T00:00:00.000Z"), Some(0));
    }
}
