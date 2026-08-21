use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

impl Date {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(1..=9999).contains(&year)
            || !(1..=12).contains(&month)
            || !(1..=days_in_month(year, month)).contains(&day)
        {
            return None;
        }
        Some(Self { year, month, day })
    }

    pub const fn year(self) -> i32 {
        self.year
    }

    pub const fn month(self) -> u8 {
        self.month
    }

    pub const fn day(self) -> u8 {
        self.day
    }

    pub fn add_days(self, days: i64) -> Option<Self> {
        civil_from_days(days_from_civil(self) + days)
    }

    pub fn add_months(self, months: i64) -> Option<Self> {
        let total = i64::from(self.year) * 12 + i64::from(self.month) - 1 + months;
        let year = i32::try_from(total.div_euclid(12)).ok()?;
        let month = u8::try_from(total.rem_euclid(12) + 1).ok()?;
        let day = self.day.min(days_in_month(year, month));
        Self::new(year, month, day)
    }

    pub fn days_since(self, earlier: Self) -> i64 {
        days_from_civil(self) - days_from_civil(earlier)
    }

    pub(crate) fn epoch_days(self) -> i64 {
        days_from_civil(self)
    }

    pub(crate) fn from_epoch_days(days: i64) -> Option<Self> {
        civil_from_days(days)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClockTime {
    seconds: u32,
}

impl ClockTime {
    pub fn new(hour: u8, minute: u8, second: u8) -> Option<Self> {
        if hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        Some(Self {
            seconds: u32::from(hour) * 3600 + u32::from(minute) * 60 + u32::from(second),
        })
    }

    pub const fn hour(self) -> u8 {
        (self.seconds / 3600) as u8
    }

    pub const fn minute(self) -> u8 {
        ((self.seconds % 3600) / 60) as u8
    }

    pub const fn second(self) -> u8 {
        (self.seconds % 60) as u8
    }

    pub fn add_seconds(self, seconds: i64) -> Self {
        let wrapped = (i64::from(self.seconds) + seconds).rem_euclid(86_400);
        Self {
            seconds: wrapped as u32,
        }
    }

    pub fn seconds_since(self, earlier: Self) -> i64 {
        i64::from(self.seconds) - i64::from(earlier.seconds)
    }

    pub(crate) const fn seconds_after_midnight(self) -> u32 {
        self.seconds
    }

    pub(crate) fn from_seconds_after_midnight(seconds: u32) -> Option<Self> {
        if seconds >= 86_400 {
            return None;
        }
        Some(Self { seconds })
    }
}

impl fmt::Display for ClockTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.second() == 0 {
            write!(formatter, "{:02}:{:02}", self.hour(), self.minute())
        } else {
            write!(
                formatter,
                "{:02}:{:02}:{:02}",
                self.hour(),
                self.minute(),
                self.second()
            )
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct UtcOffset {
    seconds: i32,
}

impl UtcOffset {
    pub const UTC: Self = Self { seconds: 0 };

    pub const fn from_seconds(seconds: i32) -> Option<Self> {
        if seconds < -86_340 || seconds > 86_340 {
            None
        } else {
            Some(Self { seconds })
        }
    }

    pub const fn from_hours_minutes(hours: i8, minutes: u8) -> Option<Self> {
        if hours < -23 || hours > 23 || minutes > 59 {
            return None;
        }
        let sign = if hours < 0 { -1 } else { 1 };
        Self::from_seconds(hours as i32 * 3600 + sign * minutes as i32 * 60)
    }

    pub const fn seconds(self) -> i32 {
        self.seconds
    }
}

impl fmt::Display for UtcOffset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.seconds == 0 {
            return formatter.write_str("Z");
        }
        let sign = if self.seconds < 0 { '-' } else { '+' };
        let absolute = self.seconds.unsigned_abs();
        write!(
            formatter,
            "{sign}{:02}:{:02}",
            absolute / 3600,
            (absolute % 3600) / 60
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OffsetDateTime {
    date: Date,
    time: ClockTime,
    offset: UtcOffset,
}

impl OffsetDateTime {
    pub const fn new(date: Date, time: ClockTime, offset: UtcOffset) -> Self {
        Self { date, time, offset }
    }

    pub const fn date(self) -> Date {
        self.date
    }

    pub const fn time(self) -> ClockTime {
        self.time
    }

    pub const fn offset(self) -> UtcOffset {
        self.offset
    }

    pub fn add_seconds(self, seconds: i64) -> Option<Self> {
        let local = self.local_seconds().checked_add(seconds)?;
        Self::from_local_seconds(local, self.offset)
    }

    pub fn add_months(self, months: i64) -> Option<Self> {
        self.date
            .add_months(months)
            .map(|date| Self::new(date, self.time, self.offset))
    }

    pub fn converted_to(self, offset: UtcOffset) -> Option<Self> {
        let utc = self.utc_seconds();
        let local = utc.checked_add(i64::from(offset.seconds))?;
        Self::from_local_seconds(local, offset)
    }

    pub fn seconds_since(self, earlier: Self) -> i64 {
        self.utc_seconds() - earlier.utc_seconds()
    }

    pub(crate) fn parse_iso_prefix(source: &str) -> Option<(Self, usize)> {
        let bytes = source.as_bytes();
        if bytes.len() < 17
            || bytes.get(4) != Some(&b'-')
            || bytes.get(7) != Some(&b'-')
            || !matches!(bytes.get(10), Some(b'T' | b't'))
            || bytes.get(13) != Some(&b':')
        {
            return None;
        }

        let date = Date::new(
            parse_ascii(source, 0, 4)?,
            parse_ascii(source, 5, 7)?,
            parse_ascii(source, 8, 10)?,
        )?;
        let hour = parse_ascii(source, 11, 13)?;
        let minute = parse_ascii(source, 14, 16)?;
        let mut end = 16;
        let second = if bytes.get(end) == Some(&b':') {
            let second = parse_ascii(source, end + 1, end + 3)?;
            end += 3;
            second
        } else {
            0
        };
        let time = ClockTime::new(hour, minute, second)?;

        let offset = match bytes.get(end) {
            Some(b'Z' | b'z') => {
                end += 1;
                UtcOffset::UTC
            }
            Some(sign @ (b'+' | b'-'))
                if bytes.get(end + 3) == Some(&b':') && bytes.len() >= end + 6 =>
            {
                let hours = i32::from(parse_ascii::<u8>(source, end + 1, end + 3)?);
                let minutes = i32::from(parse_ascii::<u8>(source, end + 4, end + 6)?);
                if hours > 23 || minutes > 59 {
                    return None;
                }
                let direction = if *sign == b'-' { -1 } else { 1 };
                end += 6;
                UtcOffset::from_seconds(direction * (hours * 3600 + minutes * 60))?
            }
            _ => return None,
        };
        Some((Self::new(date, time, offset), end))
    }

    fn local_seconds(self) -> i64 {
        self.date.epoch_days() * 86_400 + i64::from(self.time.seconds_after_midnight())
    }

    fn utc_seconds(self) -> i64 {
        self.local_seconds() - i64::from(self.offset.seconds)
    }

    fn from_local_seconds(seconds: i64, offset: UtcOffset) -> Option<Self> {
        let days = seconds.div_euclid(86_400);
        let seconds = u32::try_from(seconds.rem_euclid(86_400)).ok()?;
        Some(Self::new(
            Date::from_epoch_days(days)?,
            ClockTime::from_seconds_after_midnight(seconds)?,
            offset,
        ))
    }
}

impl fmt::Display for OffsetDateTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}T{}{}", self.date, self.time, self.offset)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarDuration {
    months: i64,
}

impl CalendarDuration {
    pub const fn months(months: i64) -> Self {
        Self { months }
    }

    pub const fn years(years: i64) -> Option<Self> {
        match years.checked_mul(12) {
            Some(months) => Some(Self { months }),
            None => None,
        }
    }

    pub const fn total_months(self) -> i64 {
        self.months
    }

    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.months.checked_add(other.months) {
            Some(months) => Some(Self { months }),
            None => None,
        }
    }

    pub const fn checked_subtract(self, other: Self) -> Option<Self> {
        match self.months.checked_sub(other.months) {
            Some(months) => Some(Self { months }),
            None => None,
        }
    }

    pub const fn checked_multiply(self, factor: i64) -> Option<Self> {
        match self.months.checked_mul(factor) {
            Some(months) => Some(Self { months }),
            None => None,
        }
    }

    pub const fn negated(self) -> Self {
        Self {
            months: -self.months,
        }
    }
}

impl fmt::Display for CalendarDuration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.months % 12 == 0 {
            let years = self.months / 12;
            write!(
                formatter,
                "{years} {}",
                if years.abs() == 1 { "year" } else { "years" }
            )
        } else {
            write!(
                formatter,
                "{} {}",
                self.months,
                if self.months.abs() == 1 {
                    "month"
                } else {
                    "months"
                }
            )
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporalContext {
    pub current_datetime: Option<OffsetDateTime>,
    pub default_offset: UtcOffset,
}

impl Default for TemporalContext {
    fn default() -> Self {
        Self {
            current_datetime: None,
            default_offset: UtcOffset::UTC,
        }
    }
}

fn parse_ascii<T: std::str::FromStr>(source: &str, start: usize, end: usize) -> Option<T> {
    source.get(start..end)?.parse().ok()
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn days_from_civil(date: Date) -> i64 {
    let mut year = i64::from(date.year);
    let month = i64::from(date.month);
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(date.day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> Option<Date> {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    Date::new(
        i32::try_from(year).ok()?,
        u8::try_from(month).ok()?,
        u8::try_from(day).ok()?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_day_arithmetic_crosses_calendar_boundaries() {
        let leap_day = Date::new(2024, 2, 29).unwrap();
        assert_eq!(leap_day.add_days(1).unwrap().to_string(), "2024-03-01");
        assert_eq!(leap_day.add_days(-60).unwrap().to_string(), "2023-12-31");
    }

    #[test]
    fn month_arithmetic_clamps_to_the_last_valid_day() {
        let january = Date::new(2025, 1, 31).unwrap();
        assert_eq!(january.add_months(1).unwrap().to_string(), "2025-02-28");
    }

    #[test]
    fn civil_day_conversion_round_trips_across_the_supported_range() {
        for date in [
            Date::new(1, 1, 1).unwrap(),
            Date::new(1970, 1, 1).unwrap(),
            Date::new(2000, 2, 29).unwrap(),
            Date::new(9999, 12, 31).unwrap(),
        ] {
            assert_eq!(civil_from_days(days_from_civil(date)), Some(date));
            assert_eq!(date.add_days(0), Some(date));
        }
    }
}
