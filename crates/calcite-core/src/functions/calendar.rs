use crate::evaluator::Value;
use crate::{
    CalculationContext, Diagnostic, DiagnosticKind, Function, OffsetDateTime, Span, WorkdayDuration,
};

use super::invalid_function_arguments;

pub(super) fn evaluate(
    function: Function,
    values: &[Value],
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    match function {
        Function::Weekday | Function::IsWorkday => {
            let date = match values {
                [Value::Date(date)] => *date,
                [Value::OffsetDateTime(value)] => value.date(),
                _ => return Err(invalid_function_arguments(span)),
            };
            if function == Function::Weekday {
                Ok(Value::Weekday(date.weekday()))
            } else {
                context
                    .temporal
                    .workday_calendar
                    .checked_is_workday(date)
                    .map(Value::Boolean)
                    .map_err(|_| Diagnostic::new(DiagnosticKind::HolidayCalendarOutOfRange, span))
            }
        }
        Function::AddWorkdays => match values {
            [Value::Date(date), Value::Number(days)] => {
                let days = days
                    .exact_i64()
                    .ok_or_else(|| invalid_function_arguments(span))?;
                context
                    .temporal
                    .workday_calendar
                    .checked_add_workdays(*date, days)
                    .map(Value::Date)
                    .map_err(|_| Diagnostic::new(DiagnosticKind::HolidayCalendarOutOfRange, span))
            }
            [Value::OffsetDateTime(value), Value::Number(days)] => {
                let days = days
                    .exact_i64()
                    .ok_or_else(|| invalid_function_arguments(span))?;
                context
                    .temporal
                    .workday_calendar
                    .checked_add_workdays(value.date(), days)
                    .map(|date| {
                        Value::OffsetDateTime(OffsetDateTime::new(
                            date,
                            value.time(),
                            value.offset(),
                        ))
                    })
                    .map_err(|_| Diagnostic::new(DiagnosticKind::HolidayCalendarOutOfRange, span))
            }
            _ => Err(invalid_function_arguments(span)),
        },
        Function::NextWorkday | Function::PreviousWorkday => {
            let days = if function == Function::NextWorkday {
                1
            } else {
                -1
            };
            match values {
                [Value::Date(date)] => context
                    .temporal
                    .workday_calendar
                    .checked_add_workdays(*date, days)
                    .map(Value::Date)
                    .map_err(|_| Diagnostic::new(DiagnosticKind::HolidayCalendarOutOfRange, span)),
                [Value::OffsetDateTime(value)] => context
                    .temporal
                    .workday_calendar
                    .checked_add_workdays(value.date(), days)
                    .map(|date| {
                        Value::OffsetDateTime(OffsetDateTime::new(
                            date,
                            value.time(),
                            value.offset(),
                        ))
                    })
                    .map_err(|_| Diagnostic::new(DiagnosticKind::HolidayCalendarOutOfRange, span)),
                _ => Err(invalid_function_arguments(span)),
            }
        }
        Function::WorkdaysBetween => {
            let (start, end) = match values {
                [Value::Date(start), Value::Date(end)] => (*start, *end),
                [Value::OffsetDateTime(start), Value::OffsetDateTime(end)] => {
                    (start.date(), end.date())
                }
                _ => return Err(invalid_function_arguments(span)),
            };
            context
                .temporal
                .workday_calendar
                .checked_workdays_between(start, end)
                .map(WorkdayDuration::new)
                .map(Value::WorkdayDuration)
                .map_err(|_| Diagnostic::new(DiagnosticKind::HolidayCalendarOutOfRange, span))
        }
        _ => unreachable!("calendar dispatch only passes calendar functions"),
    }
}
