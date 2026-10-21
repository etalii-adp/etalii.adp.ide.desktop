//! Reading a time the way the whole timeline reads times: at face value, offset zero (the standalone host's
//! `TimelineInstants.cs` and the `TimelineInstant` and `TimelinePrecision` of its `_Model` at `8b6ccfc`).
//!
//! The standalone module read a time with .NET's `DateTimeOffset.TryParse` under the invariant culture, a value
//! without an offset taken as UTC (`AssumeUniversal | AdjustToUniversal`). That parse accepts more than ISO 8601, and
//! the timeline's specification says so (`readableTime` in `timeline.dis`), so this reads the forms of it a timeline
//! is written with: ISO 8601 dates and date-times, with `T` or a space, fractional seconds and an offset or `Z`; the
//! same with `/` or `.` between the parts (`2026/07/01`); the invariant culture's `MM/dd/yyyy`; a year and a month
//! (`2026-07`, the first of the month); and dates written out in English (`July 5, 2026`, `5 July 2026`, `Jul 5
//! 2026`), with a day of the week before them that must be that date's (`Tuesday 7 July 2026`). A date that is not
//! in the calendar (`2026-02-30`) and the empty text do not read. A time of day without a date, which .NET reads as
//! today's, does not read here: a timeline's times do not move with the clock.
//!
//! An instant is held as .NET held it, in ticks of 100 nanoseconds, here counted from the Unix epoch.

/// Ticks of 100 nanoseconds in a second.
pub const TICKS_PER_SECOND: i64 = 10_000_000;

/// Ticks in a day.
pub const TICKS_PER_DAY: i64 = 86_400 * TICKS_PER_SECOND;

/// How finely a time was written: a date and a midnight date-time are the same instant and a different statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    /// A whole day: `2026-01-05`.
    Date,
    /// A time of day as well: `2026-01-05T14:30:00`.
    DateTime,
}

impl Precision {
    /// The precision of `text` as written: a date-time when it contains a `T`, `Tuesday 7 July 2026` included, else
    /// a date, as the standalone module's `TimelineReading.Instant` decided it.
    pub fn of(text: &str) -> Precision {
        if text.trim().contains('T') {
            Precision::DateTime
        } else {
            Precision::Date
        }
    }
}

/// The instant `text` names, in ticks since the Unix epoch, UTC; none when it does not read.
pub fn parse(text: &str) -> Option<i64> {
    Parser::new(text.trim()).instant()
}

/// The day number since the Unix epoch of a proleptic Gregorian date (Howard Hinnant's `days_from_civil`).
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The proleptic Gregorian date of a day number since the Unix epoch, as year, month and day.
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

const WEEKDAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

/// A month or weekday name, in full or by its first three letters.
fn named(word: &str, names: &[&str]) -> Option<usize> {
    let word = word.to_ascii_lowercase();
    names
        .iter()
        .position(|name| *name == word || (word.len() == 3 && name.starts_with(&word)))
}

struct Parser<'a> {
    text: &'a [u8],
    at: usize,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Self {
        Parser {
            text: text.as_bytes(),
            at: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.text.get(self.at).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn spaces(&mut self) -> bool {
        let start = self.at;
        while self.peek().is_some_and(|b| b == b' ' || b == b'\t') {
            self.at += 1;
        }
        self.at > start
    }

    /// Between `min` and `max` digits, as a number.
    fn number(&mut self, min: usize, max: usize) -> Option<u32> {
        let start = self.at;
        while self.at - start < max && self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.at += 1;
        }
        if self.at - start < min {
            self.at = start;
            return None;
        }
        std::str::from_utf8(&self.text[start..self.at]).ok()?.parse().ok()
    }

    fn word(&mut self) -> Option<&'a str> {
        let start = self.at;
        while self.peek().is_some_and(|b| b.is_ascii_alphabetic()) {
            self.at += 1;
        }
        (self.at > start)
            .then(|| std::str::from_utf8(&self.text[start..self.at]).ok())
            .flatten()
    }

    fn instant(mut self) -> Option<i64> {
        // A day of the week, which must be the date's.
        let start = self.at;
        let weekday = match self.word().and_then(|w| named(w, &WEEKDAYS)) {
            Some(weekday) => {
                self.eat(b',');
                self.spaces();
                Some(weekday)
            }
            None => {
                self.at = start;
                None
            }
        };
        let (year, month, day) = self.date()?;
        if month == 0 || month > 12 || day == 0 || day > days_in_month(i64::from(year), month) {
            return None;
        }
        let days = days_from_civil(i64::from(year), month, day);
        if let Some(weekday) = weekday
            && (days + 3).rem_euclid(7) != weekday as i64
        {
            return None;
        }
        let mut ticks = days * TICKS_PER_DAY;
        let before_time = self.at;
        let separated = self.eat(b'T') || self.spaces();
        if separated && let Some(time) = self.time() {
            ticks += time;
        } else {
            self.at = before_time;
        }
        let before_zone = self.at;
        self.spaces();
        match self.zone() {
            Some(offset) => ticks -= offset,
            None => self.at = before_zone,
        }
        self.spaces();
        (self.at == self.text.len()).then_some(ticks)
    }

    /// The date: numeric (`yyyy-MM-dd`, `yyyy/MM/dd`, `yyyy.MM.dd`, `yyyy-MM`, `MM/dd/yyyy`) or written out.
    fn date(&mut self) -> Option<(u32, u32, u32)> {
        let start = self.at;
        if let Some(date) = self.year_first() {
            return Some(date);
        }
        self.at = start;
        if let Some(date) = self.month_first_numeric() {
            return Some(date);
        }
        self.at = start;
        if let Some(date) = self.month_name_first() {
            return Some(date);
        }
        self.at = start;
        self.day_first()
    }

    fn year_first(&mut self) -> Option<(u32, u32, u32)> {
        let year = self.number(4, 4)?;
        let separator = self.peek().filter(|b| matches!(b, b'-' | b'/' | b'.'))?;
        self.at += 1;
        let month = self.number(1, 2)?;
        if !self.eat(separator) {
            // A year and a month: the first of the month.
            return (separator == b'-').then_some((year, month, 1));
        }
        let day = self.number(1, 2)?;
        Some((year, month, day))
    }

    fn month_first_numeric(&mut self) -> Option<(u32, u32, u32)> {
        let month = self.number(1, 2)?;
        if !self.eat(b'/') {
            return None;
        }
        let day = self.number(1, 2)?;
        if !self.eat(b'/') {
            return None;
        }
        let year = self.number(4, 4)?;
        Some((year, month, day))
    }

    fn month_name_first(&mut self) -> Option<(u32, u32, u32)> {
        let month = named(self.word()?, &MONTHS)? as u32 + 1;
        self.eat(b'.');
        if !self.spaces() {
            return None;
        }
        let day = self.number(1, 2)?;
        self.eat(b',');
        if !self.spaces() {
            return None;
        }
        let year = self.number(4, 4)?;
        Some((year, month, day))
    }

    fn day_first(&mut self) -> Option<(u32, u32, u32)> {
        let day = self.number(1, 2)?;
        if !self.spaces() {
            return None;
        }
        let month = named(self.word()?, &MONTHS)? as u32 + 1;
        self.eat(b'.');
        self.eat(b',');
        if !self.spaces() {
            return None;
        }
        let year = self.number(4, 4)?;
        Some((year, month, day))
    }

    /// `H:mm`, `H:mm:ss` or `H:mm:ss.fffffff`, in ticks.
    fn time(&mut self) -> Option<i64> {
        let hour = self.number(1, 2)?;
        if !self.eat(b':') {
            return None;
        }
        let minute = self.number(2, 2)?;
        let mut second = 0;
        let mut fraction = 0i64;
        if self.eat(b':') {
            second = self.number(2, 2)?;
            if self.eat(b'.') {
                let start = self.at;
                while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                    self.at += 1;
                }
                let digits = &self.text[start..self.at];
                if digits.is_empty() || digits.len() > 7 {
                    return None;
                }
                let mut padded = String::from_utf8_lossy(digits).into_owned();
                while padded.len() < 7 {
                    padded.push('0');
                }
                fraction = padded.parse().ok()?;
            }
        }
        if hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        Some((i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second)) * TICKS_PER_SECOND + fraction)
    }

    /// `Z`, or an offset `±HH:mm`, `±HHmm` or `±HH`, in ticks east of UTC.
    fn zone(&mut self) -> Option<i64> {
        if self.eat(b'Z') {
            return Some(0);
        }
        let sign = match self.peek()? {
            b'+' => 1,
            b'-' => -1,
            _ => return None,
        };
        self.at += 1;
        let hours = self.number(1, 2)?;
        self.eat(b':');
        let minutes = self.number(2, 2).unwrap_or(0);
        if hours > 14 || minutes > 59 || (hours == 14 && minutes > 0) {
            return None;
        }
        Some(sign * (i64::from(hours) * 3600 + i64::from(minutes) * 60) * TICKS_PER_SECOND)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i64, month: u32, day: u32) -> i64 {
        days_from_civil(year, month, day) * TICKS_PER_DAY
    }

    #[test]
    fn an_iso_date_and_date_time_read_at_face_value_in_utc() {
        assert_eq!(parse("2026-01-05"), Some(date(2026, 1, 5)));
        assert_eq!(
            parse("2026-01-05T14:30:00"),
            Some(date(2026, 1, 5) + (14 * 3600 + 30 * 60) * TICKS_PER_SECOND)
        );
        assert_eq!(parse("  2026-01-05  "), Some(date(2026, 1, 5)));
    }

    #[test]
    fn an_offset_is_applied() {
        assert_eq!(parse("2026-08-01T10:00:00+02:00"), parse("2026-08-01T08:00:00Z"));
    }

    #[test]
    fn the_forms_dotnet_reads_beyond_iso_read() {
        assert_eq!(parse("2026/07/01"), Some(date(2026, 7, 1)));
        assert_eq!(parse("July 5, 2026"), Some(date(2026, 7, 5)));
        assert_eq!(parse("Tuesday 7 July 2026"), Some(date(2026, 7, 7)));
        assert_eq!(parse("07/04/2026"), Some(date(2026, 7, 4)));
    }

    #[test]
    fn a_date_outside_the_calendar_or_a_wrong_weekday_does_not_read() {
        assert_eq!(parse("2026-02-30"), None);
        assert_eq!(parse("2026-11-31"), None);
        assert_eq!(parse("Monday 7 July 2026"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("someday"), None);
        assert_eq!(parse("later"), None);
    }

    #[test]
    fn days_and_dates_round_trip() {
        for days in [-800_000, -1, 0, 1, 20_454, 2_932_896] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn precision_is_decided_by_a_t() {
        assert_eq!(Precision::of("2026-01-05"), Precision::Date);
        assert_eq!(Precision::of("2026-01-05T00:00:00"), Precision::DateTime);
        assert_eq!(Precision::of("Tuesday 7 July 2026"), Precision::DateTime);
    }
}
