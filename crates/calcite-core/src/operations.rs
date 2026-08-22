use std::cmp::Ordering;

use crate::evaluator::Value;
use crate::unit::UnitError;
use crate::{
    BinaryOperator, CalculationContext, CalendarDuration, ClockTime, CompoundUnit,
    ConversionTarget, Date, Diagnostic, DiagnosticKind, Fraction, Money, Number, NumericError,
    OffsetDateTime, Percentage, RadixInteger, Rate, Span, Unit, UnitValue, WorkdayCalendar,
    WorkdayDuration,
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
        value => value,
    };
    let right = match right {
        Value::RadixInteger(value) => Value::Number(value.into_number()),
        value => value,
    };
    match (left, right) {
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
        (Value::Money(left), Value::Number(right)) => {
            money_number_pair(operator, left, right, span)
        }
        (Value::Number(left), Value::Money(right)) => {
            number_money_pair(operator, left, right, span)
        }
        (Value::Money(left), Value::Percentage(right)) => {
            money_percentage_pair(operator, left, right, span)
        }
        (Value::Percentage(left), Value::Money(right)) => {
            percentage_money_pair(operator, left, right, span)
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

fn number_pair(
    operator: BinaryOperator,
    left: Number,
    right: Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Add => Ok(Value::Number(left.add(right))),
        BinaryOperator::Subtract => Ok(Value::Number(left.subtract(right))),
        BinaryOperator::Multiply => Ok(Value::Number(left.multiply(right))),
        BinaryOperator::Divide => left
            .divide(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Modulo => left
            .modulo(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Remainder => left
            .remainder(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Permutation => left
            .permutation(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Combination => left
            .combination(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::PermutationsOf => right
            .permutation(left)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::CombinationsOf => right
            .combination(left)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::BitwiseAnd => left
            .bitwise_and(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::BitwiseOr => left
            .bitwise_or(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::BitwiseXor => left
            .bitwise_xor(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::ShiftLeft => left
            .shift_left(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::ShiftRight => left
            .shift_right(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Power => left
            .pow(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => compare_numbers(operator, &left, &right, span),
        BinaryOperator::Of
        | BinaryOperator::On
        | BinaryOperator::Off
        | BinaryOperator::At
        | BinaryOperator::And
        | BinaryOperator::Or => Err(unsupported(span)),
    }
}

fn percentage_pair(
    operator: BinaryOperator,
    left: Percentage,
    right: Percentage,
    span: Span,
) -> Result<Value, Diagnostic> {
    let left = left.into_ratio();
    let right = right.into_ratio();
    match operator {
        BinaryOperator::Add => Ok(Value::Percentage(Percentage::from_ratio(left.add(right)))),
        BinaryOperator::Subtract => Ok(Value::Percentage(Percentage::from_ratio(
            left.subtract(right),
        ))),
        BinaryOperator::Multiply => Ok(Value::Percentage(Percentage::from_ratio(
            left.multiply(right),
        ))),
        BinaryOperator::Divide => left
            .divide(right)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Power => left
            .pow(right)
            .map(|number| Value::Percentage(Percentage::from_ratio(number)))
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => compare_numbers(operator, &left, &right, span),
        BinaryOperator::BitwiseAnd
        | BinaryOperator::BitwiseXor
        | BinaryOperator::BitwiseOr
        | BinaryOperator::ShiftLeft
        | BinaryOperator::ShiftRight
        | BinaryOperator::Modulo
        | BinaryOperator::Remainder
        | BinaryOperator::Permutation
        | BinaryOperator::Combination
        | BinaryOperator::PermutationsOf
        | BinaryOperator::CombinationsOf
        | BinaryOperator::Of
        | BinaryOperator::On
        | BinaryOperator::Off
        | BinaryOperator::At
        | BinaryOperator::And
        | BinaryOperator::Or => Err(unsupported(span)),
    }
}

fn number_percentage_pair(
    operator: BinaryOperator,
    number: Number,
    percentage: Percentage,
    span: Span,
) -> Result<Value, Diagnostic> {
    let ratio = percentage.into_ratio();
    match operator {
        BinaryOperator::Add => Ok(Value::Number(number.clone().add(number.multiply(ratio)))),
        BinaryOperator::Subtract => Ok(Value::Number(
            number.clone().subtract(number.multiply(ratio)),
        )),
        BinaryOperator::Multiply => Ok(Value::Number(number.multiply(ratio))),
        BinaryOperator::Divide => number
            .divide(ratio)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Power => number
            .pow(ratio.clone())
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => compare_numbers(operator, &number, &ratio, span),
        BinaryOperator::BitwiseAnd
        | BinaryOperator::BitwiseXor
        | BinaryOperator::BitwiseOr
        | BinaryOperator::ShiftLeft
        | BinaryOperator::ShiftRight
        | BinaryOperator::Modulo
        | BinaryOperator::Remainder
        | BinaryOperator::Permutation
        | BinaryOperator::Combination
        | BinaryOperator::PermutationsOf
        | BinaryOperator::CombinationsOf
        | BinaryOperator::Of
        | BinaryOperator::On
        | BinaryOperator::Off
        | BinaryOperator::At
        | BinaryOperator::And
        | BinaryOperator::Or => Err(unsupported(span)),
    }
}

fn percentage_number_pair(
    operator: BinaryOperator,
    percentage: Percentage,
    number: Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    let ratio = percentage.into_ratio();
    match operator {
        BinaryOperator::Add => Ok(Value::Percentage(Percentage::from_ratio(ratio.add(number)))),
        BinaryOperator::Subtract => Ok(Value::Percentage(Percentage::from_ratio(
            ratio.subtract(number),
        ))),
        BinaryOperator::Multiply | BinaryOperator::Of => Ok(Value::Number(number.multiply(ratio))),
        BinaryOperator::Divide => ratio
            .divide(number)
            .map(|ratio| Value::Percentage(Percentage::from_ratio(ratio)))
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Power => ratio
            .pow(number)
            .map(|ratio| Value::Percentage(Percentage::from_ratio(ratio)))
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::On => Ok(Value::Number(number.clone().add(number.multiply(ratio)))),
        BinaryOperator::Off => Ok(Value::Number(
            number.clone().subtract(number.multiply(ratio)),
        )),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => compare_numbers(operator, &ratio, &number, span),
        BinaryOperator::BitwiseAnd
        | BinaryOperator::BitwiseXor
        | BinaryOperator::BitwiseOr
        | BinaryOperator::ShiftLeft
        | BinaryOperator::ShiftRight
        | BinaryOperator::Modulo
        | BinaryOperator::Remainder
        | BinaryOperator::Permutation
        | BinaryOperator::Combination
        | BinaryOperator::PermutationsOf
        | BinaryOperator::CombinationsOf
        | BinaryOperator::At
        | BinaryOperator::And
        | BinaryOperator::Or => Err(unsupported(span)),
    }
}

fn money_pair(
    operator: BinaryOperator,
    left: Money,
    right: Money,
    span: Span,
) -> Result<Value, Diagnostic> {
    if left.currency() != right.currency() {
        return Err(Diagnostic::new(DiagnosticKind::CurrencyMismatch, span));
    }
    let currency = left.currency();
    match operator {
        BinaryOperator::Add => Ok(Value::Money(Money::new(
            left.amount().clone().add(right.amount().clone()),
            currency,
        ))),
        BinaryOperator::Subtract => Ok(Value::Money(Money::new(
            left.amount().clone().subtract(right.amount().clone()),
            currency,
        ))),
        BinaryOperator::Divide => left
            .amount()
            .clone()
            .divide(right.amount().clone())
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            compare_numbers(operator, left.amount(), right.amount(), span)
        }
        _ => Err(unsupported(span)),
    }
}

fn money_number_pair(
    operator: BinaryOperator,
    money: Money,
    number: Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    let currency = money.currency();
    match operator {
        BinaryOperator::Add => Ok(Value::Money(Money::new(
            money.amount().clone().add(number),
            currency,
        ))),
        BinaryOperator::Subtract => Ok(Value::Money(Money::new(
            money.amount().clone().subtract(number),
            currency,
        ))),
        BinaryOperator::Multiply => Ok(Value::Money(Money::new(
            money.amount().clone().multiply(number),
            currency,
        ))),
        BinaryOperator::Divide => money
            .amount()
            .clone()
            .divide(number)
            .map(|amount| Value::Money(Money::new(amount, currency)))
            .map_err(|error| numeric_diagnostic(error, span)),
        _ => Err(unsupported(span)),
    }
}

fn number_money_pair(
    operator: BinaryOperator,
    number: Number,
    money: Money,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Add => money_number_pair(operator, money, number, span),
        BinaryOperator::Subtract => {
            let currency = money.currency();
            Ok(Value::Money(Money::new(
                number.subtract(money.amount().clone()),
                currency,
            )))
        }
        BinaryOperator::Multiply => money_number_pair(operator, money, number, span),
        _ => Err(unsupported(span)),
    }
}

fn money_percentage_pair(
    operator: BinaryOperator,
    money: Money,
    percentage: Percentage,
    span: Span,
) -> Result<Value, Diagnostic> {
    let ratio = percentage.into_ratio();
    let currency = money.currency();
    let amount = money.amount().clone();
    match operator {
        BinaryOperator::Add | BinaryOperator::On => Ok(Value::Money(Money::new(
            amount.clone().add(amount.multiply(ratio)),
            currency,
        ))),
        BinaryOperator::Subtract | BinaryOperator::Off => Ok(Value::Money(Money::new(
            amount.clone().subtract(amount.multiply(ratio)),
            currency,
        ))),
        BinaryOperator::Multiply => Ok(Value::Money(Money::new(amount.multiply(ratio), currency))),
        BinaryOperator::Divide => amount
            .divide(ratio)
            .map(|amount| Value::Money(Money::new(amount, currency)))
            .map_err(|error| numeric_diagnostic(error, span)),
        _ => Err(unsupported(span)),
    }
}

fn percentage_money_pair(
    operator: BinaryOperator,
    percentage: Percentage,
    money: Money,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Multiply | BinaryOperator::Of => {
            money_percentage_pair(BinaryOperator::Multiply, money, percentage, span)
        }
        _ => Err(unsupported(span)),
    }
}

fn unit_pair(
    operator: BinaryOperator,
    left: UnitValue,
    right: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Add => left
            .add(right)
            .map(Value::Unit)
            .map_err(|error| unit_diagnostic(error, span)),
        BinaryOperator::Subtract => left
            .subtract(right)
            .map(Value::Unit)
            .map_err(|error| unit_diagnostic(error, span)),
        BinaryOperator::Multiply => left
            .multiply_unit(right)
            .map(classify_compound_value)
            .map_err(|error| unit_diagnostic(error, span)),
        BinaryOperator::Divide => left
            .divide_unit(right)
            .map(classify_compound_value)
            .map_err(|error| unit_diagnostic(error, span)),
        BinaryOperator::Power => Err(unsupported(span)),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            let ordering = left
                .compare(&right)
                .map_err(|error| unit_diagnostic(error, span))?
                .ok_or_else(|| unsupported(span))?;
            compare_ordering(operator, ordering, span)
        }
        _ => Err(unsupported(span)),
    }
}

fn datetime_pair(
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

fn datetime_duration_pair(
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

fn datetime_calendar_pair(
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

fn datetime_workday_pair(
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

fn date_pair(
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

fn date_calendar_pair(
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

fn date_workday_pair(
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

fn calendar_date_pair(
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

fn date_duration_pair(
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

fn clock_pair(
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

fn clock_duration_pair(
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

fn calendar_pair(
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

fn calendar_number_pair(
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

fn workday_pair(
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

fn workday_number_pair(
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

fn rate_pair(
    operator: BinaryOperator,
    left: Rate,
    right: Rate,
    span: Span,
) -> Result<Value, Diagnostic> {
    if matches!(operator, BinaryOperator::Multiply | BinaryOperator::Divide) {
        let left = rate_as_unit_value(left);
        let right = rate_as_unit_value(right);
        let result = if operator == BinaryOperator::Multiply {
            left.multiply_unit(right)
        } else {
            left.divide_unit(right)
        };
        return result
            .map(classify_compound_value)
            .map_err(|error| unit_diagnostic(error, span));
    }

    let right = right
        .converted_to_compound(left.compound_unit())
        .map_err(|error| unit_diagnostic(error, span))?;
    match operator {
        BinaryOperator::Add => Ok(Value::Rate(Rate::from_compound(
            left.magnitude().clone().add(right.magnitude().clone()),
            left.compound_unit().clone(),
        ))),
        BinaryOperator::Subtract => Ok(Value::Rate(Rate::from_compound(
            left.magnitude().clone().subtract(right.magnitude().clone()),
            left.compound_unit().clone(),
        ))),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            compare_numbers(operator, left.magnitude(), right.magnitude(), span)
        }
        _ => Err(unsupported(span)),
    }
}

fn rate_number_pair(
    operator: BinaryOperator,
    left: Rate,
    right: Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Multiply => Ok(Value::Rate(left.scaled(right))),
        BinaryOperator::Divide => left
            .divided_by(right)
            .map(Value::Rate)
            .map_err(|error| unit_diagnostic(error, span)),
        BinaryOperator::Power => {
            let exponent = unit_exponent(&right, span)?;
            rate_as_unit_value(left)
                .pow(exponent)
                .map(classify_compound_value)
                .map_err(|error| unit_diagnostic(error, span))
        }
        _ => Err(unsupported(span)),
    }
}

fn number_rate_pair(
    operator: BinaryOperator,
    left: Number,
    right: Rate,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Multiply => Ok(Value::Rate(right.scaled(left))),
        BinaryOperator::Divide => {
            let magnitude = left
                .divide(right.magnitude().clone())
                .map_err(|error| numeric_diagnostic(error, span))?;
            let unit = CompoundUnit::default().divide(right.compound_unit());
            Ok(classify_compound_value(UnitValue::from_compound(
                magnitude, unit,
            )))
        }
        _ => Err(unsupported(span)),
    }
}

fn rate_unit_pair(
    operator: BinaryOperator,
    rate: Rate,
    unit: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    let rate = rate_as_unit_value(rate);
    let result = match operator {
        BinaryOperator::Multiply => rate.multiply_unit(unit),
        BinaryOperator::Divide => rate.divide_unit(unit),
        _ => return Err(unsupported(span)),
    };
    result
        .map(classify_compound_value)
        .map_err(|error| unit_diagnostic(error, span))
}

fn unit_rate_pair(
    operator: BinaryOperator,
    unit: UnitValue,
    rate: Rate,
    span: Span,
) -> Result<Value, Diagnostic> {
    let rate = rate_as_unit_value(rate);
    let result = match operator {
        BinaryOperator::Multiply => unit.multiply_unit(rate),
        BinaryOperator::Divide => unit.divide_unit(rate),
        _ => return Err(unsupported(span)),
    };
    result
        .map(classify_compound_value)
        .map_err(|error| unit_diagnostic(error, span))
}

fn rate_as_unit_value(rate: Rate) -> UnitValue {
    let (magnitude, unit) = rate.into_parts();
    UnitValue::from_compound(magnitude, unit)
}

pub(super) fn classify_compound_value(value: UnitValue) -> Value {
    let (magnitude, unit) = value.into_parts();
    if unit.is_dimensionless() {
        Value::Number(magnitude)
    } else if unit.has_denominator() {
        Value::Rate(Rate::from_compound(magnitude, unit))
    } else {
        Value::Unit(UnitValue::from_compound(magnitude, unit))
    }
}

fn unit_percentage_pair(
    operator: BinaryOperator,
    left: UnitValue,
    right: Percentage,
    span: Span,
) -> Result<Value, Diagnostic> {
    let ratio = right.into_ratio();
    match operator {
        BinaryOperator::Add | BinaryOperator::On => {
            let change = left.clone().multiply_number(ratio);
            left.add(change)
                .map(Value::Unit)
                .map_err(|error| unit_diagnostic(error, span))
        }
        BinaryOperator::Subtract | BinaryOperator::Off => {
            let change = left.clone().multiply_number(ratio);
            left.subtract(change)
                .map(Value::Unit)
                .map_err(|error| unit_diagnostic(error, span))
        }
        BinaryOperator::Multiply => Ok(Value::Unit(left.multiply_number(ratio))),
        BinaryOperator::Divide => left
            .divide_number(ratio)
            .map(Value::Unit)
            .map_err(|error| unit_diagnostic(error, span)),
        _ => Err(unsupported(span)),
    }
}

fn percentage_unit_pair(
    operator: BinaryOperator,
    left: Percentage,
    right: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Multiply | BinaryOperator::Of => {
            Ok(Value::Unit(right.multiply_number(left.into_ratio())))
        }
        _ => Err(unsupported(span)),
    }
}

fn unit_number_pair(
    operator: BinaryOperator,
    left: UnitValue,
    right: Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Multiply => Ok(Value::Unit(left.multiply_number(right))),
        BinaryOperator::Divide => left
            .divide_number(right)
            .map(Value::Unit)
            .map_err(|error| unit_diagnostic(error, span)),
        BinaryOperator::Power => {
            let exponent = unit_exponent(&right, span)?;
            left.pow(exponent)
                .map(classify_compound_value)
                .map_err(|error| unit_diagnostic(error, span))
        }
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            let unit = left.compound_unit().clone();
            unit_pair(operator, left, UnitValue::from_compound(right, unit), span)
        }
        _ => Err(unsupported(span)),
    }
}

fn unit_exponent(value: &Number, span: Span) -> Result<i32, Diagnostic> {
    value
        .exact_i64()
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| value.unsigned_abs() <= 100)
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidUnitExponent, span))
}

fn number_unit_pair(
    operator: BinaryOperator,
    left: Number,
    right: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Multiply => Ok(Value::Unit(right.multiply_number(left))),
        BinaryOperator::Divide => {
            let magnitude = left
                .divide(right.magnitude().clone())
                .map_err(|error| numeric_diagnostic(error, span))?;
            let unit = CompoundUnit::default().divide(right.compound_unit());
            Ok(Value::Rate(Rate::from_compound(magnitude, unit)))
        }
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            let unit = right.compound_unit().clone();
            unit_pair(operator, UnitValue::from_compound(left, unit), right, span)
        }
        _ => Err(unsupported(span)),
    }
}

fn fraction_pair(
    operator: BinaryOperator,
    left: Number,
    right: Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    match number_pair(operator, left, right, span)? {
        Value::Number(number) => Ok(Fraction::from_number(number.clone())
            .map(Value::Fraction)
            .unwrap_or(Value::Number(number))),
        value => Ok(value),
    }
}

pub(super) fn convert(
    value: Value,
    target: ConversionTarget,
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    match target {
        ConversionTarget::Number => match value {
            Value::Number(number) => Ok(Value::Number(number)),
            Value::RadixInteger(value) => Ok(Value::Number(value.into_number())),
            Value::Percentage(percentage) => Ok(Value::Number(percentage.into_ratio())),
            Value::Fraction(fraction) => Ok(Value::Number(fraction.into_number())),
            Value::Unit(value) => Ok(Value::Number(value.magnitude().clone())),
            Value::Money(value) => Ok(Value::Number(value.amount().clone())),
            Value::Rate(value) => Ok(Value::Number(value.magnitude().clone())),
            Value::Date(_)
            | Value::Clock(_)
            | Value::OffsetDateTime(_)
            | Value::CalendarDuration(_)
            | Value::WorkdayDuration(_)
            | Value::Weekday(_)
            | Value::Boolean(_) => Err(unsupported(span)),
        },
        ConversionTarget::Radix(radix) => {
            let number = match value {
                Value::Number(number) => number,
                Value::RadixInteger(value) => value.into_number(),
                _ => return Err(unsupported(span)),
            };
            RadixInteger::new(number, radix)
                .map(Value::RadixInteger)
                .map_err(|_| Diagnostic::new(DiagnosticKind::IntegerRequired, span))
        }
        ConversionTarget::Percentage => match value {
            Value::Number(number) => Ok(Value::Percentage(Percentage::from_ratio(number))),
            Value::RadixInteger(value) => Ok(Value::Percentage(Percentage::from_ratio(
                value.into_number(),
            ))),
            Value::Percentage(percentage) => Ok(Value::Percentage(percentage)),
            Value::Fraction(fraction) => Ok(Value::Percentage(Percentage::from_ratio(
                fraction.into_number(),
            ))),
            Value::Unit(_)
            | Value::Money(_)
            | Value::Rate(_)
            | Value::Date(_)
            | Value::Clock(_)
            | Value::OffsetDateTime(_)
            | Value::CalendarDuration(_)
            | Value::WorkdayDuration(_)
            | Value::Weekday(_)
            | Value::Boolean(_) => Err(unsupported(span)),
        },
        ConversionTarget::Fraction => {
            let number = match value {
                Value::Number(number) => number,
                Value::RadixInteger(value) => value.into_number(),
                Value::Percentage(percentage) => percentage.into_ratio(),
                Value::Fraction(fraction) => return Ok(Value::Fraction(fraction)),
                Value::Unit(_)
                | Value::Money(_)
                | Value::Rate(_)
                | Value::Date(_)
                | Value::Clock(_)
                | Value::OffsetDateTime(_)
                | Value::CalendarDuration(_)
                | Value::WorkdayDuration(_)
                | Value::Weekday(_)
                | Value::Boolean(_) => return Err(unsupported(span)),
            };
            Fraction::from_number(number)
                .map(Value::Fraction)
                .ok_or_else(|| unsupported(span))
        }
        ConversionTarget::CompoundUnit(target) => match value {
            Value::Unit(value) => value
                .converted_to_compound(&target)
                .map(classify_compound_value)
                .map_err(|error| unit_diagnostic(error, span)),
            Value::Rate(rate) => rate
                .converted_to_compound(&target)
                .map(Value::Rate)
                .map_err(|error| unit_diagnostic(error, span)),
            _ => Err(unsupported(span)),
        },
        ConversionTarget::UtcOffset(offset) => match value {
            Value::OffsetDateTime(value) => value
                .converted_to(offset)
                .map(Value::OffsetDateTime)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span)),
            _ => Err(unsupported(span)),
        },
        ConversionTarget::Currency(target) => match value {
            Value::Money(money) => {
                let source = money.currency();
                let rate = context
                    .monetary
                    .exchange_rates
                    .rate(source, target, context.monetary.exchange_rate_freshness)
                    .map_err(|error| {
                        let kind = match error {
                            crate::ExchangeRateLookupError::Missing => {
                                DiagnosticKind::MissingExchangeRate
                            }
                            crate::ExchangeRateLookupError::Stale => {
                                DiagnosticKind::StaleExchangeRate
                            }
                        };
                        Diagnostic::with_detail(kind, span, format!("{source} to {target}"))
                    })?;
                Ok(Value::Money(Money::new(
                    money.amount().clone().multiply(rate),
                    target,
                )))
            }
            _ => Err(unsupported(span)),
        },
    }
}

fn compare_numbers(
    operator: BinaryOperator,
    left: &Number,
    right: &Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    let ordering = left.compare(right).ok_or_else(|| unsupported(span))?;
    compare_ordering(operator, ordering, span)
}

fn compare_ordering(
    operator: BinaryOperator,
    ordering: Ordering,
    span: Span,
) -> Result<Value, Diagnostic> {
    let result = match operator {
        BinaryOperator::Equal => ordering == Ordering::Equal,
        BinaryOperator::NotEqual => ordering != Ordering::Equal,
        BinaryOperator::Greater => ordering == Ordering::Greater,
        BinaryOperator::GreaterEqual => ordering != Ordering::Less,
        BinaryOperator::Less => ordering == Ordering::Less,
        BinaryOperator::LessEqual => ordering != Ordering::Greater,
        _ => return Err(unsupported(span)),
    };
    Ok(Value::Boolean(result))
}

fn boolean_pair(
    operator: BinaryOperator,
    left: bool,
    right: bool,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Equal => Ok(Value::Boolean(left == right)),
        BinaryOperator::NotEqual => Ok(Value::Boolean(left != right)),
        BinaryOperator::And => Ok(Value::Boolean(left && right)),
        BinaryOperator::Or => Ok(Value::Boolean(left || right)),
        _ => Err(unsupported(span)),
    }
}

pub(super) fn unit_diagnostic(error: UnitError, span: Span) -> Diagnostic {
    match error {
        UnitError::IncompatibleDimensions => Diagnostic::new(DiagnosticKind::UnitMismatch, span),
        UnitError::AffineArithmetic => unsupported(span),
        UnitError::InvalidExponent => Diagnostic::new(DiagnosticKind::InvalidUnitExponent, span),
        UnitError::Numeric(error) => numeric_diagnostic(error, span),
    }
}

pub(super) fn numeric_diagnostic(error: NumericError, span: Span) -> Diagnostic {
    let kind = match error {
        NumericError::DivisionByZero => DiagnosticKind::DivisionByZero,
        NumericError::ExponentTooLarge => DiagnosticKind::ExponentTooLarge,
        NumericError::PrecisionTooLarge => DiagnosticKind::PrecisionTooLarge,
        NumericError::RootDegreeTooLarge => DiagnosticKind::RootDegreeTooLarge,
        NumericError::InvalidLogarithm => DiagnosticKind::InvalidLogarithm,
        NumericError::InvalidTrigonometry => DiagnosticKind::InvalidTrigonometry,
        NumericError::InvalidHyperbolic => DiagnosticKind::InvalidHyperbolic,
        NumericError::IntegerRequired => DiagnosticKind::IntegerRequired,
        NumericError::InvalidCombinatorics => DiagnosticKind::InvalidCombinatorics,
        NumericError::CombinatoricsTooLarge => DiagnosticKind::CombinatoricsTooLarge,
        NumericError::InvalidShift => DiagnosticKind::InvalidShift,
        NumericError::ShiftTooLarge => DiagnosticKind::ShiftTooLarge,
        NumericError::PrimalityInputTooLarge => DiagnosticKind::PrimalityInputTooLarge,
        NumericError::NonRealResult => DiagnosticKind::NonRealResult,
    };
    Diagnostic::new(kind, span)
}

pub(super) fn unsupported(span: Span) -> Diagnostic {
    Diagnostic::new(DiagnosticKind::UnsupportedOperation, span)
}
