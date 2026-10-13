//! The DOS date and time of a ZIP entry as seconds since the Unix epoch.
//!
//! A DOS date packs `year - 1980` in bits 9-15, the month (1-12) in bits 5-8
//! and the day in bits 0-4; a DOS time packs the hour in bits 11-15, the
//! minute in bits 5-10 and half the seconds (0-29) in bits 0-4. There is no
//! zone: the fields are read as UTC, proleptic Gregorian.

use std::fmt;

const FIRST_YEAR: i64 = 1980;

/// Which field of a DOS date or time is not a calendar value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DosTimeError {
    Month(u16),
    Day {
        month: u16,
        day: u16,
    },
    Hour(u16),
    Minute(u16),
    /// Half the seconds, 30 or 31.
    Second(u16),
}

impl fmt::Display for DosTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DosTimeError::Month(m) => write!(f, "DOS month {m} is not 1-12"),
            DosTimeError::Day { month, day } => {
                write!(f, "DOS day {day} does not exist in month {month}")
            }
            DosTimeError::Hour(h) => write!(f, "DOS hour {h} is not 0-23"),
            DosTimeError::Minute(m) => write!(f, "DOS minute {m} is not 0-59"),
            DosTimeError::Second(s) => write!(f, "DOS seconds field {s} is not 0-29"),
        }
    }
}

impl std::error::Error for DosTimeError {}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i64, month: u16) -> u16 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days from 1970-01-01 to the civil date (proleptic Gregorian).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// `date` and `time` as seconds since 1970-01-01T00:00:00Z.
///
/// Every value of the 16-bit year field (1980 to 2107) is a year; the month,
/// day, hour, minute and seconds fields must be calendar values.
pub fn dos_to_unix_seconds(date: u16, time: u16) -> Result<i64, DosTimeError> {
    let year = FIRST_YEAR + i64::from(date >> 9);
    let month = (date >> 5) & 0x0f;
    let day = date & 0x1f;
    let (hour, minute, half_seconds) = (time >> 11, (time >> 5) & 0x3f, time & 0x1f);

    if !(1..=12).contains(&month) {
        return Err(DosTimeError::Month(month));
    }
    if day == 0 || day > days_in_month(year, month) {
        return Err(DosTimeError::Day { month, day });
    }
    if hour > 23 {
        return Err(DosTimeError::Hour(hour));
    }
    if minute > 59 {
        return Err(DosTimeError::Minute(minute));
    }
    if half_seconds > 29 {
        return Err(DosTimeError::Second(half_seconds));
    }
    let days = days_from_civil(year, i64::from(month), i64::from(day));
    Ok(days * 86_400
        + i64::from(hour) * 3_600
        + i64::from(minute) * 60
        + i64::from(half_seconds) * 2)
}
