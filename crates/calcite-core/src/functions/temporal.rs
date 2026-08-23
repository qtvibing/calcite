use crate::evaluator::Value;
use crate::operations::{evaluate_binary, unit_diagnostic};
use crate::{
    BinaryOperator, CalculationContext, CalendarDuration, CalendarSpan, Date, Diagnostic,
    DiagnosticKind, Function, Span, TimeSpan,
};

use super::{invalid_function_arguments, rounding};

pub(super) fn evaluate(
    function: Function,
    values: &[Value],
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    match function {
        Function::CalendarBetween | Function::CalendarFrom => {
            let [first, second] = values else {
                return Err(invalid_function_arguments(span));
            };
            let (left, right, absolute) = if function == Function::CalendarBetween {
                (first, second, true)
            } else {
                (second, first, false)
            };
            calendar_span_difference(left, right, absolute, span)
        }
        Function::CalendarSince | Function::CalendarUntil => {
            let [value] = values else {
                return Err(invalid_function_arguments(span));
            };
            let current = current_value_for(value, span, context)?;
            let (left, right) = if function == Function::CalendarSince {
                (&current, value)
            } else {
                (value, &current)
            };
            calendar_span_difference(left, right, false, span)
        }
        Function::TimeBetween | Function::TimeFrom => {
            let (first, second, target) = match values {
                [first, second] => (first, second, None),
                [first, second, target] => (first, second, Some(target)),
                _ => return Err(invalid_function_arguments(span)),
            };
            let (left, right, absolute) = if function == Function::TimeBetween {
                (first, second, true)
            } else {
                (second, first, false)
            };
            if let Some(Value::CalendarDuration(unit)) = target {
                return calendar_difference(left, right, *unit, absolute, span);
            }
            if target.is_none() {
                return fixed_time_span(left, right, absolute, span);
            }
            let difference = subtract(left.clone(), right.clone(), span, context)?;
            let difference = if absolute {
                rounding::absolute_value(difference, span)?
            } else {
                difference
            };
            convert_fixed_difference(difference, target, span)
        }
        Function::TimeSince | Function::TimeUntil => {
            let (value, target) = match values {
                [value] => (value, None),
                [value, target] => (value, Some(target)),
                _ => return Err(invalid_function_arguments(span)),
            };
            let current = current_value_for(value, span, context)?;
            if let Some(Value::CalendarDuration(unit)) = target {
                let (left, right) = if function == Function::TimeSince {
                    (&current, value)
                } else {
                    (value, &current)
                };
                return calendar_difference(left, right, *unit, false, span);
            }
            let (left, right) = if function == Function::TimeSince {
                (&current, value)
            } else {
                (value, &current)
            };
            if target.is_none() {
                return fixed_time_span(left, right, false, span);
            }
            let difference = subtract(left.clone(), right.clone(), span, context)?;
            convert_fixed_difference(difference, target, span)
        }
        _ => unreachable!("temporal dispatch only passes temporal functions"),
    }
}

fn fixed_time_span(
    left: &Value,
    right: &Value,
    absolute: bool,
    span: Span,
) -> Result<Value, Diagnostic> {
    let mut nanoseconds = match (left, right) {
        (Value::Date(left), Value::Date(right)) => {
            i128::from(left.days_since(*right)) * 86_400_000_000_000
        }
        (Value::OffsetDateTime(left), Value::OffsetDateTime(right)) => {
            left.nanoseconds_since(*right)
        }
        (Value::ZonedDateTime(left), Value::ZonedDateTime(right)) => {
            left.datetime().nanoseconds_since(right.datetime())
        }
        (Value::OffsetClock(left), Value::OffsetClock(right)) => left.nanoseconds_since(*right),
        (Value::Clock(left), Value::Clock(right)) => left.nanoseconds_since(*right),
        _ => return Err(invalid_function_arguments(span)),
    };
    if absolute {
        nanoseconds = nanoseconds.abs();
    }
    Ok(Value::TimeSpan(TimeSpan::from_nanoseconds(nanoseconds)))
}

fn subtract(
    left: Value,
    right: Value,
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    evaluate_binary(BinaryOperator::Subtract, left, right, span, context)
        .map_err(|_| invalid_function_arguments(span))
}

fn convert_fixed_difference(
    difference: Value,
    target: Option<&Value>,
    span: Span,
) -> Result<Value, Diagnostic> {
    let Some(target) = target else {
        return Ok(difference);
    };
    let (Value::Unit(difference), Value::Unit(target)) = (difference, target) else {
        return Err(invalid_function_arguments(span));
    };
    difference
        .converted_to_compound(target.compound_unit())
        .map(Value::Unit)
        .map_err(|error| unit_diagnostic(error, span))
}

fn calendar_span_difference(
    left: &Value,
    right: &Value,
    absolute: bool,
    span: Span,
) -> Result<Value, Diagnostic> {
    let mut left = local_date(left).ok_or_else(|| invalid_function_arguments(span))?;
    let mut right = local_date(right).ok_or_else(|| invalid_function_arguments(span))?;
    let negative = if left < right {
        if absolute {
            std::mem::swap(&mut left, &mut right);
            false
        } else {
            std::mem::swap(&mut left, &mut right);
            true
        }
    } else {
        false
    };
    let months = complete_months_since(left, right);
    let anchor = right
        .add_months(months)
        .ok_or_else(|| invalid_function_arguments(span))?;
    let days = left.days_since(anchor);
    let (months, days) = if negative {
        (-months, -days)
    } else {
        (months, days)
    };
    CalendarSpan::new(months, days)
        .map(Value::CalendarSpan)
        .ok_or_else(|| invalid_function_arguments(span))
}

fn calendar_difference(
    left: &Value,
    right: &Value,
    requested_unit: CalendarDuration,
    absolute: bool,
    span: Span,
) -> Result<Value, Diagnostic> {
    let left = local_date(left).ok_or_else(|| invalid_function_arguments(span))?;
    let right = local_date(right).ok_or_else(|| invalid_function_arguments(span))?;
    let mut months = complete_months_since(left, right);
    if absolute {
        months = months.abs();
    }
    if requested_unit.total_months() == 12 {
        let years = months / 12;
        CalendarDuration::years(years)
            .map(Value::CalendarDuration)
            .ok_or_else(|| invalid_function_arguments(span))
    } else {
        Ok(Value::CalendarDuration(CalendarDuration::months(months)))
    }
}

fn local_date(value: &Value) -> Option<Date> {
    match value {
        Value::Date(date) => Some(*date),
        Value::OffsetDateTime(date_time) => Some(date_time.date()),
        Value::ZonedDateTime(date_time) => Some(date_time.datetime().date()),
        _ => None,
    }
}

fn complete_months_since(later: Date, earlier: Date) -> i64 {
    if later < earlier {
        return -complete_months_since(earlier, later);
    }
    let mut months = i64::from(later.year() - earlier.year()) * 12 + i64::from(later.month())
        - i64::from(earlier.month());
    if earlier
        .add_months(months)
        .is_some_and(|anniversary| anniversary > later)
    {
        months -= 1;
    }
    months
}

fn current_value_for(
    value: &Value,
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    let current = context
        .temporal
        .current_datetime
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTemporalContext, span))?;
    match value {
        Value::Date(_) => Ok(Value::Date(current.date())),
        Value::OffsetDateTime(_) => Ok(Value::OffsetDateTime(current)),
        Value::ZonedDateTime(value) => context
            .temporal
            .time_zones
            .as_ref()
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTimeZoneData, span))?
            .at_instant(value.zone(), current)
            .map(Value::ZonedDateTime)
            .map_err(|error| crate::operations::time_zone_diagnostic(error, span)),
        Value::OffsetClock(_) => Ok(Value::OffsetClock(crate::OffsetClockTime::new(
            current.time(),
            current.offset(),
        ))),
        Value::Clock(_) => Ok(Value::Clock(current.time())),
        _ => Err(invalid_function_arguments(span)),
    }
}
