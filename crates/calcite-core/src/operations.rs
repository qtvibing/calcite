mod conversion;
mod money;
mod numeric;
mod temporal;
mod units;

pub(crate) use conversion::convert;
use money::*;
pub(crate) use numeric::numeric_diagnostic;
use numeric::*;
pub(crate) use temporal::time_zone_diagnostic;
use temporal::*;
use units::*;
pub(crate) use units::{classify_compound_value, unit_diagnostic};

use crate::evaluator::Value;
use crate::{
    BinaryOperator, CalculationContext, ComplexNumber, Diagnostic, DiagnosticKind, OffsetDateTime,
    Span,
};

pub(super) fn evaluate_binary(
    operator: BinaryOperator,
    left: Value,
    right: Value,
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    let left = match left {
        Value::RadixInteger(value) => Value::Number(value.into_number()),
        Value::RomanInteger(value) => Value::Number(value.into_number()),
        value => value,
    };
    let right = match right {
        Value::RadixInteger(value) => Value::Number(value.into_number()),
        Value::RomanInteger(value) => Value::Number(value.into_number()),
        value => value,
    };
    match (left, right) {
        (Value::String(left), Value::String(right)) => string_pair(operator, left, right, span),
        (Value::Complex(left), Value::Complex(right)) => complex_pair(operator, left, right, span),
        (Value::Complex(left), Value::Number(right)) => {
            complex_pair(operator, left, ComplexNumber::from_real(right), span)
        }
        (Value::Number(left), Value::Complex(right)) => {
            complex_pair(operator, ComplexNumber::from_real(left), right, span)
        }
        (Value::Number(left), Value::Number(right)) => number_pair(operator, left, right, span),
        (Value::Percentage(left), Value::Percentage(right)) => {
            percentage_pair(operator, left, right, span)
        }
        (Value::Number(left), Value::Percentage(right)) => {
            number_percentage_pair(operator, left, right, span)
        }
        (Value::Percentage(left), Value::Number(right)) => {
            percentage_number_pair(operator, left, right, span)
        }
        (Value::Fraction(left), Value::Fraction(right)) => {
            fraction_pair(operator, left.into_number(), right.into_number(), span)
        }
        (Value::Fraction(left), Value::Number(right)) => {
            fraction_pair(operator, left.into_number(), right, span)
        }
        (Value::Number(left), Value::Fraction(right)) => {
            fraction_pair(operator, left, right.into_number(), span)
        }
        (Value::Fraction(left), Value::Percentage(right)) => {
            number_percentage_pair(operator, left.into_number(), right, span)
        }
        (Value::Percentage(left), Value::Fraction(right)) => {
            percentage_number_pair(operator, left, right.into_number(), span)
        }
        (Value::TimeSpan(left), Value::TimeSpan(right)) => {
            time_span_pair(operator, left, right, span)
        }
        (Value::TimeSpan(left), Value::Unit(right)) => {
            time_span_unit_pair(operator, left, right, span)
        }
        (Value::Unit(left), Value::TimeSpan(right)) if operator == BinaryOperator::Add => {
            time_span_unit_pair(operator, right, left, span)
        }
        (Value::Unit(left), Value::Unit(right)) => unit_pair(operator, left, right, span),
        (Value::Unit(left), Value::Number(right)) => unit_number_pair(operator, left, right, span),
        (Value::Number(left), Value::Unit(right)) => number_unit_pair(operator, left, right, span),
        (Value::Unit(left), Value::Percentage(right)) => {
            unit_percentage_pair(operator, left, right, span)
        }
        (Value::Percentage(left), Value::Unit(right)) => {
            percentage_unit_pair(operator, left, right, span)
        }
        (Value::Money(left), Value::Money(right)) => money_pair(operator, left, right, span),
        (Value::Money(left), Value::MoneyRate(right)) => {
            money_money_rate_pair(operator, left, right, span)
        }
        (Value::Money(left), Value::Unit(right)) => money_unit_pair(operator, left, right, span),
        (Value::Money(left), Value::Number(right)) => {
            money_number_pair(operator, left, right, span)
        }
        (Value::Number(left), Value::Money(right)) => {
            number_money_pair(operator, left, right, span)
        }
        (Value::MoneyRate(left), Value::MoneyRate(right)) => {
            money_rate_pair(operator, left, right, span)
        }
        (Value::MoneyRate(rate), Value::Unit(quantity)) => {
            money_rate_unit_pair(operator, rate, quantity, span)
        }
        (Value::Unit(quantity), Value::MoneyRate(rate)) if operator == BinaryOperator::Multiply => {
            money_rate_unit_pair(operator, rate, quantity, span)
        }
        (Value::MoneyRate(rate), Value::Number(number)) => {
            money_rate_number_pair(operator, rate, number, span)
        }
        (Value::Number(number), Value::MoneyRate(rate)) if operator == BinaryOperator::Multiply => {
            money_rate_number_pair(operator, rate, number, span)
        }
        (Value::Money(left), Value::Percentage(right)) => {
            money_percentage_pair(operator, left, right, span)
        }
        (Value::Percentage(left), Value::Money(right)) => {
            percentage_money_pair(operator, left, right, span)
        }
        (Value::Dice(dice), Value::Number(modifier)) => {
            let modifier = modifier
                .exact_i64()
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::IntegerRequired, span))?;
            let modifier = if operator == BinaryOperator::Subtract {
                modifier
                    .checked_neg()
                    .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDice, span))?
            } else if operator == BinaryOperator::Add {
                modifier
            } else {
                return Err(unsupported(span));
            };
            dice.with_modifier(modifier)
                .map(Value::Dice)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDice, span))
        }
        (Value::Number(modifier), Value::Dice(dice)) if operator == BinaryOperator::Add => {
            let modifier = modifier
                .exact_i64()
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::IntegerRequired, span))?;
            dice.with_modifier(modifier)
                .map(Value::Dice)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDice, span))
        }
        (Value::Rate(left), Value::Rate(right)) => rate_pair(operator, left, right, span),
        (Value::Rate(left), Value::Number(right)) => rate_number_pair(operator, left, right, span),
        (Value::Number(left), Value::Rate(right)) => number_rate_pair(operator, left, right, span),
        (Value::Rate(left), Value::Unit(right)) => rate_unit_pair(operator, left, right, span),
        (Value::Unit(left), Value::Rate(right)) => unit_rate_pair(operator, left, right, span),
        (Value::Date(date), Value::Clock(time)) if operator == BinaryOperator::At => {
            let offset = context
                .temporal
                .current_datetime
                .map_or(context.temporal.default_offset, |value| value.offset());
            Ok(Value::OffsetDateTime(OffsetDateTime::new(
                date, time, offset,
            )))
        }
        (Value::Date(date), Value::OffsetClock(time)) if operator == BinaryOperator::At => Ok(
            Value::OffsetDateTime(OffsetDateTime::new(date, time.time(), time.offset())),
        ),
        (Value::Date(date), Value::ZonedClock(time)) if operator == BinaryOperator::At => {
            date_zoned_clock_pair(date, time, &context.temporal, span)
        }
        (Value::ZonedDateTime(left), Value::ZonedDateTime(right)) => {
            zoned_datetime_pair(operator, left, right, span)
        }
        (Value::ZonedDateTime(value), Value::Unit(duration)) => {
            zoned_datetime_duration_pair(operator, value, duration, &context.temporal, span)
        }
        (Value::ZonedDateTime(value), Value::CalendarDuration(duration)) => {
            zoned_datetime_calendar_pair(operator, value, duration, &context.temporal, span)
        }
        (Value::CalendarDuration(duration), Value::ZonedDateTime(value))
            if operator == BinaryOperator::Add =>
        {
            zoned_datetime_calendar_pair(operator, value, duration, &context.temporal, span)
        }
        (Value::OffsetClock(left), Value::OffsetClock(right)) => {
            offset_clock_pair(operator, left, right, span)
        }
        (Value::OffsetClock(time), Value::Unit(duration)) => {
            offset_clock_duration_pair(operator, time, duration, span)
        }
        (Value::OffsetDateTime(left), Value::OffsetDateTime(right)) => {
            datetime_pair(operator, left, right, span)
        }
        (Value::OffsetDateTime(value), Value::Unit(duration)) => {
            datetime_duration_pair(operator, value, duration, span)
        }
        (Value::OffsetDateTime(value), Value::CalendarDuration(duration)) => {
            datetime_calendar_pair(operator, value, duration, span)
        }
        (Value::OffsetDateTime(value), Value::WorkdayDuration(duration)) => datetime_workday_pair(
            operator,
            value,
            duration,
            &context.temporal.workday_calendar,
            span,
        ),
        (Value::WorkdayDuration(duration), Value::OffsetDateTime(value))
            if operator == BinaryOperator::Add =>
        {
            datetime_workday_pair(
                operator,
                value,
                duration,
                &context.temporal.workday_calendar,
                span,
            )
        }
        (Value::CalendarDuration(duration), Value::OffsetDateTime(value))
            if operator == BinaryOperator::Add =>
        {
            datetime_calendar_pair(operator, value, duration, span)
        }
        (Value::Date(left), Value::Date(right)) => date_pair(operator, left, right, span),
        (Value::Date(date), Value::CalendarDuration(duration)) => {
            date_calendar_pair(operator, date, duration, span)
        }
        (Value::Date(date), Value::WorkdayDuration(duration)) => date_workday_pair(
            operator,
            date,
            duration,
            &context.temporal.workday_calendar,
            span,
        ),
        (Value::WorkdayDuration(duration), Value::Date(date))
            if operator == BinaryOperator::Add =>
        {
            date_workday_pair(
                operator,
                date,
                duration,
                &context.temporal.workday_calendar,
                span,
            )
        }
        (Value::CalendarDuration(duration), Value::Date(date)) => {
            calendar_date_pair(operator, duration, date, span)
        }
        (Value::Date(date), Value::Unit(duration)) => {
            date_duration_pair(operator, date, duration, span)
        }
        (Value::Clock(time), Value::Clock(right)) => clock_pair(operator, time, right, span),
        (Value::Clock(time), Value::Unit(duration)) => {
            clock_duration_pair(operator, time, duration, span)
        }
        (Value::CalendarDuration(left), Value::CalendarDuration(right)) => {
            calendar_pair(operator, left, right, span)
        }
        (Value::CalendarDuration(duration), Value::Number(number)) => {
            calendar_number_pair(operator, duration, number, span)
        }
        (Value::Number(number), Value::CalendarDuration(duration))
            if operator == BinaryOperator::Multiply =>
        {
            calendar_number_pair(operator, duration, number, span)
        }
        (Value::WorkdayDuration(left), Value::WorkdayDuration(right)) => {
            workday_pair(operator, left, right, span)
        }
        (Value::WorkdayDuration(duration), Value::Number(number)) => {
            workday_number_pair(operator, duration, number, span)
        }
        (Value::Number(number), Value::WorkdayDuration(duration))
            if operator == BinaryOperator::Multiply =>
        {
            workday_number_pair(operator, duration, number, span)
        }
        (Value::Weekday(left), Value::Weekday(right)) => {
            compare_ordering(operator, left.cmp(&right), span)
        }
        (Value::Boolean(left), Value::Boolean(right)) => boolean_pair(operator, left, right, span),
        _ => Err(unsupported(span)),
    }
}

fn string_pair(
    operator: BinaryOperator,
    left: String,
    right: String,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Add => Ok(Value::String(left + &right)),
        BinaryOperator::Equal => Ok(Value::Boolean(left == right)),
        BinaryOperator::NotEqual => Ok(Value::Boolean(left != right)),
        BinaryOperator::Less => Ok(Value::Boolean(left < right)),
        BinaryOperator::LessEqual => Ok(Value::Boolean(left <= right)),
        BinaryOperator::Greater => Ok(Value::Boolean(left > right)),
        BinaryOperator::GreaterEqual => Ok(Value::Boolean(left >= right)),
        _ => Err(unsupported(span)),
    }
}

fn complex_pair(
    operator: BinaryOperator,
    left: ComplexNumber,
    right: ComplexNumber,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Add => Ok(Value::Complex(left.add(right))),
        BinaryOperator::Subtract => Ok(Value::Complex(left.subtract(right))),
        BinaryOperator::Multiply => Ok(Value::Complex(left.multiply(right))),
        BinaryOperator::Divide => left
            .checked_divide(right)
            .map(Value::Complex)
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::DivisionByZero, span)),
        BinaryOperator::Power if right.is_real() => {
            let exponent = right
                .real()
                .exact_i64()
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::IntegerRequired, span))?;
            left.pow(exponent)
                .map(Value::Complex)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::ExponentTooLarge, span))
        }
        BinaryOperator::Equal => Ok(Value::Boolean(left == right)),
        BinaryOperator::NotEqual => Ok(Value::Boolean(left != right)),
        _ => Err(unsupported(span)),
    }
}

pub(super) fn unsupported(span: Span) -> Diagnostic {
    Diagnostic::new(DiagnosticKind::UnsupportedOperation, span)
}
