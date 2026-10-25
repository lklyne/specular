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
