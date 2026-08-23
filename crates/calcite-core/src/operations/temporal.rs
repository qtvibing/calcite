use crate::evaluator::Value;
use crate::{
    BinaryOperator, CalendarDuration, ClockTime, Date, Diagnostic, DiagnosticKind, Number,
    OffsetDateTime, Span, Unit, UnitValue, WorkdayCalendar, WorkdayDuration,
};

use super::{compare_ordering, unit_diagnostic, unsupported};

pub(super) fn datetime_pair(
    operator: BinaryOperator,
    left: OffsetDateTime,
    right: OffsetDateTime,
    span: Span,
) -> Result<Value, Diagnostic> {
    let seconds = left.seconds_since(right);
    match operator {
        BinaryOperator::Subtract => Ok(Value::Unit(UnitValue::new(
            Number::from_i64(seconds),
            Unit::Second,
        ))),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => compare_ordering(operator, seconds.cmp(&0), span),
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
        .converted_to(Unit::Second)
        .map_err(|error| unit_diagnostic(error, span))?;
    let mut seconds = duration
        .magnitude()
        .exact_i64()
        .ok_or_else(|| unsupported(span))?;
    if operator == BinaryOperator::Subtract {
        seconds = seconds
            .checked_neg()
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidTime, span))?;
    } else if operator != BinaryOperator::Add {
        return Err(unsupported(span));
    }
    value
        .add_seconds(seconds)
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
        .add_workdays(value.date(), days)
        .map(|date| Value::OffsetDateTime(OffsetDateTime::new(date, value.time(), value.offset())))
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))
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
        .add_workdays(date, days)
        .map(Value::Date)
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))
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
            Number::from_i64(left.seconds_since(right)),
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

pub(super) fn clock_duration_pair(
    operator: BinaryOperator,
    time: ClockTime,
    duration: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    let duration = duration
        .converted_to(Unit::Second)
        .map_err(|error| unit_diagnostic(error, span))?;
    let mut seconds = duration
        .magnitude()
        .exact_i64()
        .ok_or_else(|| unsupported(span))?;
    if operator == BinaryOperator::Subtract {
        seconds = seconds
            .checked_neg()
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidTime, span))?;
    } else if operator != BinaryOperator::Add {
        return Err(unsupported(span));
    }
    Ok(Value::Clock(time.add_seconds(seconds)))
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
