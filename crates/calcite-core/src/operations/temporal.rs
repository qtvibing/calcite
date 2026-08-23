use crate::evaluator::Value;
use crate::{
    BinaryOperator, CalendarDuration, ClockTime, Date, Diagnostic, DiagnosticKind,
    LocalTimeResolution, Number, OffsetClockTime, OffsetDateTime, Span, TemporalContext, TimeSpan,
    TimeZoneError, Unit, UnitValue, WorkdayCalendar, WorkdayDuration, ZonedClockTime,
    ZonedDateTime,
};

use super::{compare_ordering, unit_diagnostic, unsupported};

pub(super) fn date_zoned_clock_pair(
    date: Date,
    time: ZonedClockTime,
    context: &TemporalContext,
    span: Span,
) -> Result<Value, Diagnostic> {
    let snapshot = context
        .time_zones
        .as_ref()
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTimeZoneData, span))?;
    snapshot
        .resolve_local(time.zone(), date, time.time(), LocalTimeResolution::Reject)
        .map(Value::ZonedDateTime)
        .map_err(|error| time_zone_diagnostic(error, span))
}

pub(super) fn zoned_datetime_pair(
    operator: BinaryOperator,
    left: ZonedDateTime,
    right: ZonedDateTime,
    span: Span,
) -> Result<Value, Diagnostic> {
    datetime_pair(operator, left.datetime(), right.datetime(), span)
}

pub(super) fn zoned_datetime_duration_pair(
    operator: BinaryOperator,
    value: ZonedDateTime,
    duration: UnitValue,
    context: &TemporalContext,
    span: Span,
) -> Result<Value, Diagnostic> {
    let snapshot = context
        .time_zones
        .as_ref()
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTimeZoneData, span))?;
    let duration = duration
        .converted_to(Unit::Nanosecond)
        .map_err(|error| unit_diagnostic(error, span))?;
    let mut nanoseconds = duration
        .magnitude()
        .exact_i64()
        .map(i128::from)
        .ok_or_else(|| unsupported(span))?;
    if operator == BinaryOperator::Subtract {
        nanoseconds = nanoseconds
            .checked_neg()
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidTime, span))?;
    } else if operator != BinaryOperator::Add {
        return Err(unsupported(span));
    }
    value
        .add_nanoseconds(snapshot, nanoseconds)
        .map(Value::ZonedDateTime)
        .map_err(|error| time_zone_diagnostic(error, span))
}

pub(super) fn zoned_datetime_calendar_pair(
    operator: BinaryOperator,
    value: ZonedDateTime,
    duration: CalendarDuration,
    context: &TemporalContext,
    span: Span,
) -> Result<Value, Diagnostic> {
    let snapshot = context
        .time_zones
        .as_ref()
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTimeZoneData, span))?;
    let duration = match operator {
        BinaryOperator::Add => duration,
        BinaryOperator::Subtract => duration.negated(),
        _ => return Err(unsupported(span)),
    };
    value
        .add_calendar_duration(snapshot, duration, LocalTimeResolution::Reject)
        .map(Value::ZonedDateTime)
        .map_err(|error| time_zone_diagnostic(error, span))
}

pub(super) fn datetime_pair(
    operator: BinaryOperator,
    left: OffsetDateTime,
    right: OffsetDateTime,
    span: Span,
) -> Result<Value, Diagnostic> {
    let nanoseconds = left.nanoseconds_since(right);
    match operator {
        BinaryOperator::Subtract => Ok(Value::Unit(UnitValue::new(
            seconds_from_nanoseconds(nanoseconds),
            Unit::Second,
        ))),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => compare_ordering(operator, nanoseconds.cmp(&0), span),
        _ => Err(unsupported(span)),
    }
}

pub(super) fn datetime_duration_pair(
    operator: BinaryOperator,
    value: OffsetDateTime,
    duration: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    let duration = duration
        .converted_to(Unit::Nanosecond)
        .map_err(|error| unit_diagnostic(error, span))?;
    let mut nanoseconds = duration
        .magnitude()
        .exact_i64()
        .map(i128::from)
        .ok_or_else(|| unsupported(span))?;
    if operator == BinaryOperator::Subtract {
        nanoseconds = nanoseconds
            .checked_neg()
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidTime, span))?;
    } else if operator != BinaryOperator::Add {
        return Err(unsupported(span));
    }
    value
        .add_nanoseconds(nanoseconds)
        .map(Value::OffsetDateTime)
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))
}

pub(super) fn datetime_calendar_pair(
    operator: BinaryOperator,
    value: OffsetDateTime,
    duration: CalendarDuration,
    span: Span,
) -> Result<Value, Diagnostic> {
    let months = match operator {
        BinaryOperator::Add => duration.total_months(),
        BinaryOperator::Subtract => duration
            .total_months()
            .checked_neg()
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span))?,
        _ => return Err(unsupported(span)),
    };
    value
        .add_months(months)
        .map(Value::OffsetDateTime)
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))
}

pub(super) fn datetime_workday_pair(
    operator: BinaryOperator,
    value: OffsetDateTime,
    duration: WorkdayDuration,
    calendar: &WorkdayCalendar,
    span: Span,
) -> Result<Value, Diagnostic> {
    let days = match operator {
        BinaryOperator::Add => duration.days(),
        BinaryOperator::Subtract => duration
            .days()
            .checked_neg()
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span))?,
        _ => return Err(unsupported(span)),
    };
    calendar
        .checked_add_workdays(value.date(), days)
        .map(|date| Value::OffsetDateTime(OffsetDateTime::new(date, value.time(), value.offset())))
        .map_err(|_| Diagnostic::new(DiagnosticKind::HolidayCalendarOutOfRange, span))
}

pub(super) fn date_pair(
    operator: BinaryOperator,
    left: Date,
    right: Date,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Subtract => Ok(Value::Unit(UnitValue::new(
            Number::from_i64(left.days_since(right)),
            Unit::Day,
        ))),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => compare_ordering(operator, left.cmp(&right), span),
        _ => Err(unsupported(span)),
    }
}

pub(super) fn date_calendar_pair(
    operator: BinaryOperator,
    date: Date,
    duration: CalendarDuration,
    span: Span,
) -> Result<Value, Diagnostic> {
    let months = match operator {
        BinaryOperator::Add => duration.total_months(),
        BinaryOperator::Subtract => duration
            .total_months()
            .checked_neg()
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span))?,
        _ => return Err(unsupported(span)),
    };
    date.add_months(months)
        .map(Value::Date)
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))
}

pub(super) fn date_workday_pair(
    operator: BinaryOperator,
    date: Date,
    duration: WorkdayDuration,
    calendar: &WorkdayCalendar,
    span: Span,
) -> Result<Value, Diagnostic> {
    let days = match operator {
        BinaryOperator::Add => duration.days(),
        BinaryOperator::Subtract => duration
            .days()
            .checked_neg()
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span))?,
        _ => return Err(unsupported(span)),
    };
    calendar
        .checked_add_workdays(date, days)
        .map(Value::Date)
        .map_err(|_| Diagnostic::new(DiagnosticKind::HolidayCalendarOutOfRange, span))
}

pub(super) fn calendar_date_pair(
    operator: BinaryOperator,
    duration: CalendarDuration,
    date: Date,
    span: Span,
) -> Result<Value, Diagnostic> {
    if operator == BinaryOperator::Add {
        date_calendar_pair(operator, date, duration, span)
    } else {
        Err(unsupported(span))
    }
}

pub(super) fn date_duration_pair(
    operator: BinaryOperator,
    date: Date,
    duration: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    let duration = duration
        .converted_to(Unit::Day)
        .map_err(|error| unit_diagnostic(error, span))?;
    let mut days = duration
        .magnitude()
        .exact_i64()
        .ok_or_else(|| unsupported(span))?;
    if operator == BinaryOperator::Subtract {
        days = days
            .checked_neg()
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))?;
    } else if operator != BinaryOperator::Add {
        return Err(unsupported(span));
    }
    date.add_days(days)
        .map(Value::Date)
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))
}

pub(super) fn clock_pair(
    operator: BinaryOperator,
    left: ClockTime,
    right: ClockTime,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Subtract => Ok(Value::Unit(UnitValue::new(
            seconds_from_nanoseconds(left.nanoseconds_since(right)),
            Unit::Second,
        ))),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => compare_ordering(operator, left.cmp(&right), span),
        _ => Err(unsupported(span)),
    }
}

pub(super) fn offset_clock_pair(
    operator: BinaryOperator,
    left: OffsetClockTime,
    right: OffsetClockTime,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Subtract => Ok(Value::Unit(UnitValue::new(
            seconds_from_nanoseconds(left.nanoseconds_since(right)),
            Unit::Second,
        ))),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            compare_ordering(operator, left.nanoseconds_since(right).cmp(&0), span)
        }
        _ => Err(unsupported(span)),
    }
}

pub(super) fn offset_clock_duration_pair(
    operator: BinaryOperator,
    value: OffsetClockTime,
    duration: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    clock_duration_pair(operator, value.time(), duration, span).map(|result| match result {
        Value::Clock(time) => Value::OffsetClock(OffsetClockTime::new(time, value.offset())),
        _ => unreachable!("clock duration arithmetic returns a clock"),
    })
}

pub(super) fn clock_duration_pair(
    operator: BinaryOperator,
    time: ClockTime,
    duration: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    let duration = duration
        .converted_to(Unit::Nanosecond)
        .map_err(|error| unit_diagnostic(error, span))?;
    let mut nanoseconds = duration
        .magnitude()
        .exact_i64()
        .map(i128::from)
        .ok_or_else(|| unsupported(span))?;
    if operator == BinaryOperator::Subtract {
        nanoseconds = nanoseconds
            .checked_neg()
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidTime, span))?;
    } else if operator != BinaryOperator::Add {
        return Err(unsupported(span));
    }
    Ok(Value::Clock(time.add_nanoseconds(nanoseconds)))
}

pub(crate) fn time_zone_diagnostic(error: TimeZoneError, span: Span) -> Diagnostic {
    let kind = match error {
        TimeZoneError::UnknownZone => DiagnosticKind::UnknownTimeZone,
        TimeZoneError::OutOfRange => DiagnosticKind::TimeZoneDataOutOfRange,
        TimeZoneError::AmbiguousLocalTime => DiagnosticKind::AmbiguousLocalTime,
        TimeZoneError::NonexistentLocalTime => DiagnosticKind::NonexistentLocalTime,
        TimeZoneError::InvalidDefinition | TimeZoneError::DuplicateZone => {
            DiagnosticKind::MissingTimeZoneData
        }
    };
    Diagnostic::new(kind, span)
}

fn seconds_from_nanoseconds(nanoseconds: i128) -> Number {
    Number::parse_decimal(&nanoseconds.to_string())
        .expect("an i128 is a valid bounded decimal")
        .divide(Number::from_i64(1_000_000_000))
        .expect("one billion is nonzero")
}

pub(super) fn time_span_pair(
    operator: BinaryOperator,
    left: TimeSpan,
    right: TimeSpan,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Add => left
            .checked_add(right)
            .map(Value::TimeSpan)
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidTime, span)),
        BinaryOperator::Subtract => left
            .checked_subtract(right)
            .map(Value::TimeSpan)
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidTime, span)),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            compare_ordering(operator, left.nanoseconds().cmp(&right.nanoseconds()), span)
        }
        _ => Err(unsupported(span)),
    }
}

pub(super) fn time_span_unit_pair(
    operator: BinaryOperator,
    left: TimeSpan,
    right: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    let right = right
        .converted_to(Unit::Nanosecond)
        .map_err(|error| unit_diagnostic(error, span))?
        .magnitude()
        .exact_i64()
        .map(i128::from)
        .ok_or_else(|| unsupported(span))?;
    let right = TimeSpan::from_nanoseconds(right);
    time_span_pair(operator, left, right, span)
}

pub(super) fn calendar_pair(
    operator: BinaryOperator,
    left: CalendarDuration,
    right: CalendarDuration,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Add => left
            .checked_add(right)
            .map(Value::CalendarDuration)
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span)),
        BinaryOperator::Subtract => left
            .checked_subtract(right)
            .map(Value::CalendarDuration)
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span)),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => compare_ordering(
            operator,
            left.total_months().cmp(&right.total_months()),
            span,
        ),
        _ => Err(unsupported(span)),
    }
}

pub(super) fn calendar_number_pair(
    operator: BinaryOperator,
    duration: CalendarDuration,
    number: Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    if operator != BinaryOperator::Multiply {
        return Err(unsupported(span));
    }
    let factor = number.exact_i64().ok_or_else(|| unsupported(span))?;
    duration
        .checked_multiply(factor)
        .map(Value::CalendarDuration)
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span))
}

pub(super) fn workday_pair(
    operator: BinaryOperator,
    left: WorkdayDuration,
    right: WorkdayDuration,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Add => left
            .checked_add(right)
            .map(Value::WorkdayDuration)
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span)),
        BinaryOperator::Subtract => left
            .checked_subtract(right)
            .map(Value::WorkdayDuration)
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span)),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            compare_ordering(operator, left.days().cmp(&right.days()), span)
        }
        _ => Err(unsupported(span)),
    }
}

pub(super) fn workday_number_pair(
    operator: BinaryOperator,
    duration: WorkdayDuration,
    number: Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    if operator != BinaryOperator::Multiply {
        return Err(unsupported(span));
    }
    let factor = number.exact_i64().ok_or_else(|| unsupported(span))?;
    duration
        .checked_multiply(factor)
        .map(Value::WorkdayDuration)
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span))
}
