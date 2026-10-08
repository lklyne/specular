//! Formatting the wall clock the way `.canvas` files store it.

/// `unix_ms` as an ISO 8601 UTC timestamp with milliseconds, the form
/// JavaScript's `Date.toISOString` writes.
pub fn iso8601(unix_ms: u64) -> String {
    let millis = unix_ms % 1000;
    let seconds = unix_ms / 1000;
    let (hour, minute, second) = (seconds / 3600 % 24, seconds / 60 % 60, seconds % 60);
    let (year, month, day) = civil_from_days(seconds / 86_400);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// The proleptic Gregorian date `days` after 1970-01-01 (Howard Hinnant's
/// `civil_from_days`, for non-negative day counts).
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z % 146_097;
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
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_milliseconds_format_as_iso_8601_utc() {
        for (ms, want) in [
            (0, "1970-01-01T00:00:00.000Z"),
            (1_709_210_096_789, "2024-02-29T12:34:56.789Z"),
            (1_767_225_599_999, "2025-12-31T23:59:59.999Z"),
        ] {
            assert_eq!(iso8601(ms), want, "{ms}");
        }
    }
}
