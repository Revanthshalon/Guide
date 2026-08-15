//! The date and time FHIR primitives.
//!
//! # Partial precision is the whole problem
//!
//! FHIR dates are deliberately *partial*: `birthDate` may be `1974`,
//! `1974-12`, or `1974-12-25`, because that is genuinely all a registration
//! clerk was told. A type that normalises those into a single timestamp has
//! invented information — an ingestion pipeline that turns `1974` into
//! `1974-01-01` will later report a patient as having been born in January.
//!
//! So these types keep the lexical form *and* the precision, and they refuse to
//! pretend they can always be ordered. Comparison is
//! [`Date::chronological_cmp`], which returns `None` when the two values'
//! possible ranges overlap: `1974` versus `1974-12-25` is genuinely
//! indeterminate, and callers must decide what to do about it rather than get a
//! silently wrong `false`.
//!
//! # Time zones
//!
//! `dateTime` and `instant` require an offset whenever a time is present. That
//! is enforced at construction. A date-only value has no offset at all, which
//! is why its instant range is widened by ±14 hours (the largest offset in use)
//! when comparing against a value that does have one.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{ParseError, ParseErrorKind};
use crate::primitive::{PrimitiveType, StringPrimitive};
use crate::validate::{Validate, Validator};

/// How much of a date or time a value actually specifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Precision {
    /// Year only, e.g. `1974`.
    Year,
    /// Year and month, e.g. `1974-12`.
    Month,
    /// A full calendar date, e.g. `1974-12-25`.
    Day,
    /// Date and time to the second.
    Second,
    /// Date and time with a fractional second.
    Fractional,
}

/// The widest UTC offset in use anywhere (+14:00), in milliseconds.
const MAX_OFFSET_MS: i64 = 14 * 60 * 60 * 1000;
const MS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

/// The half-open interval of instants a value could refer to, in milliseconds
/// relative to 1970-01-01T00:00:00Z.
///
/// `zoned` records whether the interval is anchored to a real UTC offset. A
/// date with no time carries no offset, so its interval is expressed on the
/// local calendar; only when it is compared against a *zoned* value does the
/// ±14 h uncertainty need to be applied. Widening both sides unconditionally
/// would make `1974` versus `1975-01-01` come out indeterminate, which it
/// plainly is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InstantRange {
    start: i64,
    end: i64,
    zoned: bool,
}

impl InstantRange {
    /// Widen by the largest UTC offset in use, for comparison against a value
    /// whose offset is known.
    fn widened(self) -> Self {
        Self {
            start: self.start - MAX_OFFSET_MS,
            end: self.end + MAX_OFFSET_MS,
            zoned: true,
        }
    }

    fn compare(self, other: Self) -> Option<Ordering> {
        if self == other {
            return Some(Ordering::Equal);
        }
        let (left, right) = match (self.zoned, other.zoned) {
            (true, false) => (self, other.widened()),
            (false, true) => (self.widened(), other),
            _ => (self, other),
        };
        if left.end <= right.start {
            return Some(Ordering::Less);
        }
        if right.end <= left.start {
            return Some(Ordering::Greater);
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DateParts {
    year: u16,
    month: Option<u8>,
    day: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TimeParts {
    hour: u8,
    minute: u8,
    second: u8,
    /// Fractional part in milliseconds, and whether one was written at all.
    milliseconds: Option<u16>,
}

/// FHIR `date`: `YYYY`, `YYYY-MM`, or `YYYY-MM-DD`.
///
/// The calendar is checked, so `2023-02-29` is rejected while `2024-02-29` is
/// accepted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Date {
    lexical: String,
    parts: DateParts,
}

impl Date {
    /// Validate a lexical date and wrap it.
    pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
        let lexical = value.into();
        let parts = parse_date_parts(&lexical).map_err(|kind| ParseError::new("date", kind))?;
        Ok(Self { lexical, parts })
    }

    /// The lexical form, exactly as supplied.
    pub fn as_str(&self) -> &str {
        &self.lexical
    }

    /// The year.
    pub const fn year(&self) -> u16 {
        self.parts.year
    }

    /// The month, if the value specifies one.
    pub const fn month(&self) -> Option<u8> {
        self.parts.month
    }

    /// The day of month, if the value specifies one.
    pub const fn day(&self) -> Option<u8> {
        self.parts.day
    }

    /// How much of the date is specified.
    pub const fn precision(&self) -> Precision {
        date_precision(&self.parts)
    }

    /// Compare chronologically, returning `None` when the two values' possible
    /// ranges overlap and no definite answer exists.
    pub fn chronological_cmp(&self, other: &Self) -> Option<Ordering> {
        date_range(&self.parts).compare(date_range(&other.parts))
    }
}

/// FHIR `dateTime`: a [`Date`], optionally with a time and a **mandatory**
/// offset whenever a time is present.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DateTime {
    lexical: String,
    date: DateParts,
    time: Option<TimeParts>,
    offset_minutes: Option<i16>,
}

impl DateTime {
    /// Validate a lexical dateTime and wrap it.
    pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
        let lexical = value.into();
        let parsed =
            parse_date_time(&lexical, false).map_err(|kind| ParseError::new("dateTime", kind))?;
        Ok(Self {
            lexical,
            date: parsed.date,
            time: parsed.time,
            offset_minutes: parsed.offset_minutes,
        })
    }

    /// The lexical form, exactly as supplied.
    pub fn as_str(&self) -> &str {
        &self.lexical
    }

    /// The year.
    pub const fn year(&self) -> u16 {
        self.date.year
    }

    /// How much of the value is specified.
    pub const fn precision(&self) -> Precision {
        match self.time {
            None => date_precision(&self.date),
            Some(time) => match time.milliseconds {
                None => Precision::Second,
                Some(_) => Precision::Fractional,
            },
        }
    }

    /// The UTC offset in minutes, if the value carries a time.
    pub const fn offset_minutes(&self) -> Option<i16> {
        self.offset_minutes
    }

    /// The date part, dropping any time.
    pub fn date(&self) -> Date {
        let lexical = self
            .lexical
            .split('T')
            .next()
            .unwrap_or(&self.lexical)
            .to_owned();
        Date {
            lexical,
            parts: self.date,
        }
    }

    /// Compare chronologically, returning `None` when the comparison is
    /// indeterminate because of differing precision.
    pub fn chronological_cmp(&self, other: &Self) -> Option<Ordering> {
        self.range().compare(other.range())
    }

    fn range(&self) -> InstantRange {
        date_time_range(&self.date, self.time.as_ref(), self.offset_minutes)
    }
}

impl From<Date> for DateTime {
    fn from(date: Date) -> Self {
        Self {
            lexical: date.lexical,
            date: date.parts,
            time: None,
            offset_minutes: None,
        }
    }
}

/// FHIR `instant`: a timestamp known to at least the second, with an offset.
///
/// Unlike `dateTime`, partial precision is not allowed — this is the type used
/// for machine-generated timestamps such as `Meta.lastUpdated`, where a partial
/// value would be meaningless.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Instant {
    lexical: String,
    date: DateParts,
    time: TimeParts,
    offset_minutes: i16,
}

impl Instant {
    /// Validate a lexical instant and wrap it.
    pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
        let lexical = value.into();
        let parsed =
            parse_date_time(&lexical, true).map_err(|kind| ParseError::new("instant", kind))?;
        let (Some(time), Some(offset_minutes)) = (parsed.time, parsed.offset_minutes) else {
            return Err(ParseError::new(
                "instant",
                ParseErrorKind::Malformed {
                    expected: "a full timestamp with a time and a UTC offset",
                },
            ));
        };
        Ok(Self {
            lexical,
            date: parsed.date,
            time,
            offset_minutes,
        })
    }

    /// The lexical form, exactly as supplied.
    pub fn as_str(&self) -> &str {
        &self.lexical
    }

    /// Milliseconds since the Unix epoch.
    pub fn epoch_milliseconds(&self) -> i64 {
        date_time_range(&self.date, Some(&self.time), Some(self.offset_minutes)).start
    }

    /// The UTC offset in minutes.
    pub const fn offset_minutes(&self) -> i16 {
        self.offset_minutes
    }
}

impl PartialOrd for Instant {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// An `instant` is always fully specified, so unlike [`Date`] and [`DateTime`]
/// it is totally ordered.
impl Ord for Instant {
    fn cmp(&self, other: &Self) -> Ordering {
        self.epoch_milliseconds().cmp(&other.epoch_milliseconds())
    }
}

/// FHIR `time`: `hh:mm:ss` with an optional fraction, and never an offset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Time {
    lexical: String,
    parts: TimeParts,
}

impl Time {
    /// Validate a lexical time and wrap it.
    pub fn new(value: impl Into<String>) -> Result<Self, ParseError> {
        let lexical = value.into();
        let parts = parse_time_parts(&lexical).map_err(|kind| ParseError::new("time", kind))?;
        Ok(Self { lexical, parts })
    }

    /// The lexical form, exactly as supplied.
    pub fn as_str(&self) -> &str {
        &self.lexical
    }

    /// Milliseconds since midnight.
    pub fn milliseconds_of_day(&self) -> i64 {
        time_of_day_ms(&self.parts)
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.milliseconds_of_day().cmp(&other.milliseconds_of_day())
    }
}

macro_rules! temporal_boilerplate {
    ($name:ident, $fhir:literal) => {
        impl PrimitiveType for $name {
            const FHIR_TYPE: &'static str = $fhir;
        }

        impl StringPrimitive for $name {
            fn as_str(&self) -> &str {
                &self.lexical
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.lexical)
            }
        }

        impl FromStr for $name {
            type Err = ParseError;

            fn from_str(value: &str) -> Result<Self, ParseError> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ParseError;

            fn try_from(value: &str) -> Result<Self, ParseError> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = ParseError;

            fn try_from(value: String) -> Result<Self, ParseError> {
                Self::new(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.lexical)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                Self::new(raw).map_err(serde::de::Error::custom)
            }
        }

        impl Validate for $name {
            fn validate(&self, _validator: &mut Validator) {}
        }
    };
}

temporal_boilerplate!(Date, "date");
temporal_boilerplate!(DateTime, "dateTime");
temporal_boilerplate!(Instant, "instant");
temporal_boilerplate!(Time, "time");

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

const DATE_SHAPE: &str = "a date shaped YYYY, YYYY-MM, or YYYY-MM-DD";
const DATE_TIME_SHAPE: &str = "a dateTime shaped YYYY[-MM[-DD[Thh:mm:ss[.sss](Z|±hh:mm)]]]";
const TIME_SHAPE: &str = "a time shaped hh:mm:ss[.sss]";

struct ParsedDateTime {
    date: DateParts,
    time: Option<TimeParts>,
    offset_minutes: Option<i16>,
}

fn parse_date_parts(value: &str) -> Result<DateParts, ParseErrorKind> {
    let malformed = || ParseErrorKind::Malformed {
        expected: DATE_SHAPE,
    };
    let mut sections = value.split('-');
    let year_text = sections.next().ok_or_else(malformed)?;
    if year_text.len() != 4 || !year_text.chars().all(|c| c.is_ascii_digit()) {
        return Err(malformed());
    }
    let year: u16 = year_text.parse().map_err(|_| malformed())?;
    if year == 0 {
        return Err(ParseErrorKind::OutOfRange {
            min: 1,
            max: 9999,
            actual: 0,
        });
    }

    let month = match sections.next() {
        None => None,
        Some(text) => {
            if text.len() != 2 || !text.chars().all(|c| c.is_ascii_digit()) {
                return Err(malformed());
            }
            let month: u8 = text.parse().map_err(|_| malformed())?;
            if !(1..=12).contains(&month) {
                return Err(ParseErrorKind::OutOfRange {
                    min: 1,
                    max: 12,
                    actual: month as i64,
                });
            }
            Some(month)
        }
    };

    let day = match sections.next() {
        None => None,
        Some(text) => {
            let Some(month) = month else {
                return Err(malformed());
            };
            if text.len() != 2 || !text.chars().all(|c| c.is_ascii_digit()) {
                return Err(malformed());
            }
            let day: u8 = text.parse().map_err(|_| malformed())?;
            let last = days_in_month(year, month);
            if day < 1 || day > last {
                return Err(ParseErrorKind::OutOfRange {
                    min: 1,
                    max: last as i64,
                    actual: day as i64,
                });
            }
            Some(day)
        }
    };

    if sections.next().is_some() {
        return Err(malformed());
    }

    Ok(DateParts { year, month, day })
}

fn parse_time_parts(value: &str) -> Result<TimeParts, ParseErrorKind> {
    let malformed = || ParseErrorKind::Malformed {
        expected: TIME_SHAPE,
    };
    let (clock, fraction) = match value.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (value, None),
    };

    let sections: Vec<&str> = clock.split(':').collect();
    if sections.len() != 3 || sections.iter().any(|s| s.len() != 2) {
        return Err(malformed());
    }
    if sections
        .iter()
        .any(|s| !s.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(malformed());
    }

    let hour: u8 = sections[0].parse().map_err(|_| malformed())?;
    let minute: u8 = sections[1].parse().map_err(|_| malformed())?;
    let second: u8 = sections[2].parse().map_err(|_| malformed())?;

    if hour > 23 {
        return Err(ParseErrorKind::OutOfRange {
            min: 0,
            max: 23,
            actual: hour as i64,
        });
    }
    if minute > 59 {
        return Err(ParseErrorKind::OutOfRange {
            min: 0,
            max: 59,
            actual: minute as i64,
        });
    }
    // 60 is permitted: FHIR allows a leap second.
    if second > 60 {
        return Err(ParseErrorKind::OutOfRange {
            min: 0,
            max: 60,
            actual: second as i64,
        });
    }

    let milliseconds = match fraction {
        None => None,
        Some(digits) => {
            if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
                return Err(malformed());
            }
            let mut scaled: u32 = 0;
            for (index, digit) in digits.chars().take(3).enumerate() {
                let value = digit.to_digit(10).unwrap_or(0);
                scaled += value * 10u32.pow(2 - index as u32);
            }
            Some(scaled as u16)
        }
    };

    Ok(TimeParts {
        hour,
        minute,
        second,
        milliseconds,
    })
}

fn parse_offset(value: &str) -> Result<i16, ParseErrorKind> {
    let malformed = || ParseErrorKind::Malformed {
        expected: "a UTC offset shaped Z or ±hh:mm",
    };
    if value == "Z" {
        return Ok(0);
    }
    let sign = match value.chars().next() {
        Some('+') => 1,
        Some('-') => -1,
        _ => return Err(malformed()),
    };
    let body = &value[1..];
    let (hours_text, minutes_text) = body.split_once(':').ok_or_else(malformed)?;
    if hours_text.len() != 2 || minutes_text.len() != 2 {
        return Err(malformed());
    }
    let hours: i16 = hours_text.parse().map_err(|_| malformed())?;
    let minutes: i16 = minutes_text.parse().map_err(|_| malformed())?;
    if hours > 14 || minutes > 59 || (hours == 14 && minutes != 0) {
        return Err(ParseErrorKind::OutOfRange {
            min: -14 * 60,
            max: 14 * 60,
            actual: (sign * (hours * 60 + minutes)) as i64,
        });
    }
    Ok(sign * (hours * 60 + minutes))
}

fn parse_date_time(value: &str, require_time: bool) -> Result<ParsedDateTime, ParseErrorKind> {
    let malformed = || ParseErrorKind::Malformed {
        expected: DATE_TIME_SHAPE,
    };

    let Some((date_text, rest)) = value.split_once('T') else {
        if require_time {
            return Err(malformed());
        }
        return Ok(ParsedDateTime {
            date: parse_date_parts(value)?,
            time: None,
            offset_minutes: None,
        });
    };

    let date = parse_date_parts(date_text)?;
    if date.day.is_none() {
        // A time on a partial date is meaningless.
        return Err(malformed());
    }

    // The offset is the trailing `Z`, or the last `+`/`-` in the remainder.
    let (time_text, offset_text) = if let Some(stripped) = rest.strip_suffix('Z') {
        (stripped, "Z")
    } else {
        let split_at = rest.rfind(['+', '-']).ok_or(ParseErrorKind::Malformed {
            expected: "a UTC offset — FHIR requires one whenever a time is present",
        })?;
        rest.split_at(split_at)
    };

    let time = parse_time_parts(time_text)?;
    let offset_minutes = parse_offset(offset_text)?;

    Ok(ParsedDateTime {
        date,
        time: Some(time),
        offset_minutes: Some(offset_minutes),
    })
}

// ---------------------------------------------------------------------------
// Calendar arithmetic
// ---------------------------------------------------------------------------

const fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Days from 1970-01-01 to `year-month-day`, by Howard Hinnant's `days_from_civil`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

const fn date_precision(parts: &DateParts) -> Precision {
    match (parts.month, parts.day) {
        (None, _) => Precision::Year,
        (Some(_), None) => Precision::Month,
        (Some(_), Some(_)) => Precision::Day,
    }
}

fn time_of_day_ms(time: &TimeParts) -> i64 {
    // A leap second lands in the same millisecond window as :59 for ordering.
    let second = time.second.min(59) as i64;
    (time.hour as i64) * 3_600_000
        + (time.minute as i64) * 60_000
        + second * 1000
        + time.milliseconds.unwrap_or(0) as i64
}

/// The window a date denotes, on the local calendar — a date with no time
/// carries no UTC offset, so the interval is not anchored to one.
fn date_range(parts: &DateParts) -> InstantRange {
    let year = parts.year as i64;
    let (start_days, end_days) = match (parts.month, parts.day) {
        (None, _) => (days_from_civil(year, 1, 1), days_from_civil(year + 1, 1, 1)),
        (Some(month), None) => {
            let (next_year, next_month) = if month == 12 {
                (year + 1, 1)
            } else {
                (year, month as i64 + 1)
            };
            (
                days_from_civil(year, month as i64, 1),
                days_from_civil(next_year, next_month, 1),
            )
        }
        (Some(month), Some(day)) => {
            let start = days_from_civil(year, month as i64, day as i64);
            (start, start + 1)
        }
    };
    InstantRange {
        start: start_days * MS_PER_DAY,
        end: end_days * MS_PER_DAY,
        zoned: false,
    }
}

fn date_time_range(
    date: &DateParts,
    time: Option<&TimeParts>,
    offset_minutes: Option<i16>,
) -> InstantRange {
    let Some(time) = time else {
        return date_range(date);
    };
    let day_start = days_from_civil(
        date.year as i64,
        date.month.unwrap_or(1) as i64,
        date.day.unwrap_or(1) as i64,
    ) * MS_PER_DAY;
    let offset_ms = offset_minutes.unwrap_or(0) as i64 * 60_000;
    let start = day_start + time_of_day_ms(time) - offset_ms;
    // Second precision denotes a one-second window; a fraction pins it down.
    let width = if time.milliseconds.is_some() { 1 } else { 1000 };
    InstantRange {
        start,
        end: start + width,
        zoned: offset_minutes.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_checks_the_calendar() {
        assert!(Date::new("2024-02-29").is_ok());
        assert!(Date::new("2023-02-29").is_err());
        assert!(Date::new("1974").is_ok());
        assert!(Date::new("1974-12").is_ok());
        assert!(Date::new("1974-1-1").is_err());
        assert!(Date::new("74-12-25").is_err());
        assert!(Date::new("2020-13-01").is_err());
    }

    #[test]
    fn date_precision_is_preserved() {
        let year_only = Date::new("1974").expect("valid date");
        assert_eq!(year_only.precision(), Precision::Year);
        assert_eq!(year_only.month(), None);
        assert_eq!(year_only.as_str(), "1974");
    }

    #[test]
    fn partial_dates_compare_only_when_unambiguous() {
        let year = Date::new("1974").expect("valid date");
        let day = Date::new("1974-12-25").expect("valid date");
        let later = Date::new("1975-01-01").expect("valid date");

        // 1974-12-25 is inside 1974 — no definite ordering exists.
        assert_eq!(year.chronological_cmp(&day), None);
        assert_eq!(year.chronological_cmp(&later), Some(Ordering::Less));
        assert_eq!(later.chronological_cmp(&day), Some(Ordering::Greater));
        assert_eq!(year.chronological_cmp(&year), Some(Ordering::Equal));
    }

    #[test]
    fn date_time_requires_an_offset_with_a_time() {
        assert!(DateTime::new("2024-05-01T10:00:00Z").is_ok());
        assert!(DateTime::new("2024-05-01T10:00:00+05:30").is_ok());
        assert!(DateTime::new("2024-05-01T10:00:00").is_err());
        assert!(DateTime::new("2024-05-01T10:00Z").is_err());
        assert!(DateTime::new("2024-05").is_ok());
        assert!(DateTime::new("2024-05T10:00:00Z").is_err());
    }

    #[test]
    fn offsets_are_applied_when_comparing() {
        let utc = DateTime::new("2024-05-01T10:00:00Z").expect("valid dateTime");
        let ist = DateTime::new("2024-05-01T15:30:00+05:30").expect("valid dateTime");
        assert_eq!(utc.chronological_cmp(&ist), Some(Ordering::Equal));

        let earlier = DateTime::new("2024-05-01T09:59:59Z").expect("valid dateTime");
        assert_eq!(earlier.chronological_cmp(&utc), Some(Ordering::Less));
    }

    #[test]
    fn instant_requires_full_precision() {
        assert!(Instant::new("2024-05-01T10:00:00.123Z").is_ok());
        assert!(Instant::new("2024-05-01").is_err());
        assert!(Instant::new("2024-05-01T10:00:00").is_err());

        let epoch = Instant::new("1970-01-01T00:00:00Z").expect("valid instant");
        assert_eq!(epoch.epoch_milliseconds(), 0);
    }

    #[test]
    fn time_has_no_offset_and_orders_totally() {
        assert!(Time::new("13:45:00").is_ok());
        assert!(Time::new("13:45:00.250").is_ok());
        assert!(Time::new("13:45").is_err());
        assert!(Time::new("13:45:00Z").is_err());
        assert!(Time::new("24:00:00").is_err());

        let morning = Time::new("08:00:00").expect("valid time");
        let evening = Time::new("20:00:00").expect("valid time");
        assert!(morning < evening);
    }
}
