use std::{collections::HashMap, fmt};

use crate::functions::evaluate_call;
use crate::operations::{classify_compound_value, convert, evaluate_binary, unsupported};
use crate::syntax::identifier_key;
use crate::{
    BinaryOperator, CalculationContext, CalendarDuration, CalendarSpan, ClockTime, ComplexNumber,
    Constant, Date, Diagnostic, DiagnosticKind, DiceDistribution, Expression, Fraction, Money,
    MoneyRate, Number, OffsetClockTime, OffsetDateTime, Percentage, RadixInteger, Rate,
    RelativeWeekdayDirection, RomanInteger, TemporalKeyword, TimeSpan, UnaryOperator, UnitValue,
    Weekday, WorkdayDuration, ZonedClockTime, ZonedDateTime, parse_with_context,
};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    Number(Number),
    RadixInteger(RadixInteger),
    RomanInteger(RomanInteger),
    Complex(ComplexNumber),
    Percentage(Percentage),
    Fraction(Fraction),
    Unit(UnitValue),
    Money(Money),
    MoneyRate(MoneyRate),
    Rate(Rate),
    Dice(DiceDistribution),
    Date(Date),
    Clock(ClockTime),
    OffsetClock(OffsetClockTime),
    ZonedClock(ZonedClockTime),
    OffsetDateTime(OffsetDateTime),
    ZonedDateTime(ZonedDateTime),
    TimeSpan(TimeSpan),
    CalendarDuration(CalendarDuration),
    CalendarSpan(CalendarSpan),
    WorkdayDuration(WorkdayDuration),
    Weekday(Weekday),
    Boolean(bool),
}

impl Value {
    pub const fn as_number(&self) -> Option<&Number> {
        match self {
            Self::Number(number) => Some(number),
            _ => None,
        }
    }

    pub const fn as_radix_integer(&self) -> Option<&RadixInteger> {
        match self {
            Self::RadixInteger(value) => Some(value),
            _ => None,
        }
    }

    pub const fn as_roman_integer(&self) -> Option<&RomanInteger> {
        match self {
            Self::RomanInteger(value) => Some(value),
            _ => None,
        }
    }

    pub const fn as_complex(&self) -> Option<&ComplexNumber> {
        match self {
            Self::Complex(value) => Some(value),
            _ => None,
        }
    }

    pub const fn as_percentage(&self) -> Option<&Percentage> {
        match self {
            Self::Percentage(percentage) => Some(percentage),
            _ => None,
        }
    }

    pub const fn as_fraction(&self) -> Option<&Fraction> {
        match self {
            Self::Fraction(fraction) => Some(fraction),
            _ => None,
        }
    }

    pub const fn as_unit(&self) -> Option<&UnitValue> {
        match self {
            Self::Unit(value) => Some(value),
            _ => None,
        }
    }

    pub const fn as_money(&self) -> Option<&Money> {
        match self {
            Self::Money(money) => Some(money),
            _ => None,
        }
    }

    pub const fn as_money_rate(&self) -> Option<&MoneyRate> {
        match self {
            Self::MoneyRate(rate) => Some(rate),
            _ => None,
        }
    }

    pub const fn as_rate(&self) -> Option<&Rate> {
        match self {
            Self::Rate(rate) => Some(rate),
            _ => None,
        }
    }

    pub const fn as_dice(&self) -> Option<DiceDistribution> {
        match self {
            Self::Dice(dice) => Some(*dice),
            _ => None,
        }
    }

    pub const fn as_date(&self) -> Option<Date> {
        match self {
            Self::Date(date) => Some(*date),
            _ => None,
        }
    }

    pub const fn as_clock(&self) -> Option<ClockTime> {
        match self {
            Self::Clock(time) => Some(*time),
            _ => None,
        }
    }

    pub const fn as_offset_clock(&self) -> Option<OffsetClockTime> {
        match self {
            Self::OffsetClock(time) => Some(*time),
            _ => None,
        }
    }

    pub const fn as_zoned_clock(&self) -> Option<&ZonedClockTime> {
        match self {
            Self::ZonedClock(time) => Some(time),
            _ => None,
        }
    }

    pub const fn as_offset_datetime(&self) -> Option<OffsetDateTime> {
        match self {
            Self::OffsetDateTime(value) => Some(*value),
            _ => None,
        }
    }

    pub const fn as_zoned_datetime(&self) -> Option<&ZonedDateTime> {
        match self {
            Self::ZonedDateTime(value) => Some(value),
            _ => None,
        }
    }

    pub const fn as_time_span(&self) -> Option<TimeSpan> {
        match self {
            Self::TimeSpan(duration) => Some(*duration),
            _ => None,
        }
    }

    pub const fn as_calendar_span(&self) -> Option<CalendarSpan> {
        match self {
            Self::CalendarSpan(duration) => Some(*duration),
            _ => None,
        }
    }

    pub const fn as_calendar_duration(&self) -> Option<CalendarDuration> {
        match self {
            Self::CalendarDuration(duration) => Some(*duration),
            _ => None,
        }
    }

    pub const fn as_workday_duration(&self) -> Option<WorkdayDuration> {
        match self {
            Self::WorkdayDuration(duration) => Some(*duration),
            _ => None,
        }
    }

    pub const fn as_weekday(&self) -> Option<Weekday> {
        match self {
            Self::Weekday(weekday) => Some(*weekday),
            _ => None,
        }
    }

    pub const fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => number.fmt(formatter),
            Self::RadixInteger(value) => value.fmt(formatter),
            Self::RomanInteger(value) => value.fmt(formatter),
            Self::Complex(value) => value.fmt(formatter),
            Self::Percentage(percentage) => percentage.fmt(formatter),
            Self::Fraction(fraction) => fraction.fmt(formatter),
            Self::Unit(value) => {
                write!(formatter, "{} {}", value.magnitude(), value.display_unit())
            }
            Self::Money(money) => money.fmt(formatter),
            Self::MoneyRate(rate) => rate.fmt(formatter),
            Self::Rate(rate) => rate.fmt(formatter),
            Self::Dice(dice) => dice.fmt(formatter),
            Self::Date(date) => date.fmt(formatter),
            Self::Clock(time) => time.fmt(formatter),
            Self::OffsetClock(time) => time.fmt(formatter),
            Self::ZonedClock(time) => time.fmt(formatter),
            Self::OffsetDateTime(value) => value.fmt(formatter),
            Self::ZonedDateTime(value) => value.fmt(formatter),
            Self::TimeSpan(duration) => duration.fmt(formatter),
            Self::CalendarDuration(duration) => duration.fmt(formatter),
            Self::CalendarSpan(duration) => duration.fmt(formatter),
            Self::WorkdayDuration(duration) => duration.fmt(formatter),
            Self::Weekday(weekday) => weekday.fmt(formatter),
            Self::Boolean(value) => value.fmt(formatter),
        }
    }
}

pub fn evaluate(source: &str) -> Result<Value, Diagnostic> {
    evaluate_with_context(source, &CalculationContext::default())
}

pub fn evaluate_with_context(
    source: &str,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    let expression = parse_with_context(source, context)?;
    evaluate_expression(&expression, context, &HashMap::new())
}

pub(crate) fn evaluate_expression(
    expression: &Expression,
    context: &CalculationContext,
    variables: &HashMap<String, Value>,
) -> Result<Value, Diagnostic> {
    match expression {
        Expression::Number { value, .. } => Ok(Value::Number(value.clone())),
        Expression::RadixInteger { value, .. } => Ok(Value::RadixInteger(value.clone())),
        Expression::RomanInteger { value, .. } => Ok(Value::RomanInteger(value.clone())),
        Expression::Complex { value, .. } => Ok(Value::Complex(value.clone())),
        Expression::Boolean { value, .. } => Ok(Value::Boolean(*value)),
        Expression::Constant { constant, .. } => Ok(Value::Number(match constant {
            Constant::Pi => Number::pi(),
            Constant::Tau => Number::pi().multiply(Number::from_i64(2)),
            Constant::E => Number::e(),
        })),
        Expression::Unit { value, unit, .. } => Ok(classify_compound_value(
            UnitValue::from_compound(value.clone(), unit.clone()),
        )),
        Expression::Money {
            amount, currency, ..
        } => Ok(Value::Money(Money::new(amount.clone(), *currency))),
        Expression::Dice { value, .. } => Ok(Value::Dice(*value)),
        Expression::Date { value, .. } => Ok(Value::Date(*value)),
        Expression::PartialDate { month, day, span } => {
            let year = context
                .temporal
                .current_datetime
                .map(|current| current.date().year())
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTemporalContext, *span))?;
            Date::new(year, *month, *day)
                .map(Value::Date)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, *span))
        }
        Expression::Clock { value, .. } => Ok(Value::Clock(*value)),
        Expression::OffsetClock { value, .. } => Ok(Value::OffsetClock(*value)),
        Expression::ZonedClock { value, .. } => Ok(Value::ZonedClock(value.clone())),
        Expression::OffsetDateTime { value, .. } => Ok(Value::OffsetDateTime(*value)),
        Expression::CalendarDuration { value, .. } => Ok(Value::CalendarDuration(*value)),
        Expression::WorkdayDuration { value, .. } => Ok(Value::WorkdayDuration(*value)),
        Expression::TemporalKeyword { keyword, span } => match keyword {
            TemporalKeyword::Today => context
                .temporal
                .current_datetime
                .map(|value| Value::Date(value.date()))
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTemporalContext, *span)),
            TemporalKeyword::Tomorrow => context
                .temporal
                .current_datetime
                .and_then(|value| value.date().add_days(1))
                .map(Value::Date)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTemporalContext, *span)),
            TemporalKeyword::Yesterday => context
                .temporal
                .current_datetime
                .and_then(|value| value.date().add_days(-1))
                .map(Value::Date)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTemporalContext, *span)),
            TemporalKeyword::Now => context
                .temporal
                .current_datetime
                .map(Value::OffsetDateTime)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTemporalContext, *span)),
            TemporalKeyword::NextMonth
            | TemporalKeyword::PreviousMonth
            | TemporalKeyword::NextYear
            | TemporalKeyword::PreviousYear
            | TemporalKeyword::StartOfMonth
            | TemporalKeyword::EndOfMonth
            | TemporalKeyword::StartOfYear
            | TemporalKeyword::EndOfYear => {
                let current = context
                    .temporal
                    .current_datetime
                    .map(OffsetDateTime::date)
                    .ok_or_else(|| {
                        Diagnostic::new(DiagnosticKind::MissingTemporalContext, *span)
                    })?;
                let date = match keyword {
                    TemporalKeyword::NextMonth => current.add_months(1),
                    TemporalKeyword::PreviousMonth => current.add_months(-1),
                    TemporalKeyword::NextYear => current.add_months(12),
                    TemporalKeyword::PreviousYear => current.add_months(-12),
                    TemporalKeyword::StartOfMonth => Date::new(current.year(), current.month(), 1),
                    TemporalKeyword::EndOfMonth => current
                        .add_months(1)
                        .and_then(|next| Date::new(next.year(), next.month(), 1))
                        .and_then(|next| next.add_days(-1)),
                    TemporalKeyword::StartOfYear => Date::new(current.year(), 1, 1),
                    TemporalKeyword::EndOfYear => Date::new(current.year(), 12, 31),
                    _ => unreachable!("current-date keyword was matched above"),
                };
                date.map(Value::Date)
                    .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, *span))
            }
        },
        Expression::RelativeWeekday {
            direction,
            weekday,
            span,
        } => {
            let current = context
                .temporal
                .current_datetime
                .map(OffsetDateTime::date)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTemporalContext, *span))?;
            let current_number = i64::from(current.weekday().number_from_monday());
            let target_number = i64::from(weekday.number_from_monday());
            let days = match direction {
                RelativeWeekdayDirection::Next => {
                    let distance = (target_number - current_number).rem_euclid(7);
                    if distance == 0 { 7 } else { distance }
                }
                RelativeWeekdayDirection::Previous => {
                    let distance = (current_number - target_number).rem_euclid(7);
                    -(if distance == 0 { 7 } else { distance })
                }
                RelativeWeekdayDirection::This => target_number - current_number,
            };
            current
                .add_days(days)
                .map(Value::Date)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, *span))
        }
        Expression::Variable { name, span } => variables
            .get(&identifier_key(name))
            .cloned()
            .ok_or_else(|| Diagnostic::with_detail(DiagnosticKind::UnknownVariable, *span, name)),
        Expression::Call {
            function,
            function_span,
            arguments,
            ..
        } => evaluate_call(*function, *function_span, arguments, context, variables),
        Expression::Unary {
            operator,
            operator_span,
            operand,
            ..
        } => {
            let operand = evaluate_expression(operand, context, variables)?;
            match (operator, operand) {
                (UnaryOperator::Positive, Value::Number(number)) => Ok(Value::Number(number)),
                (UnaryOperator::Positive, Value::RadixInteger(value)) => {
                    Ok(Value::RadixInteger(value))
                }
                (UnaryOperator::Negative, Value::RadixInteger(value)) => {
                    Ok(Value::RadixInteger(value.negated()))
                }
                (UnaryOperator::Positive, Value::RomanInteger(value)) => {
                    Ok(Value::RomanInteger(value))
                }
                (UnaryOperator::Negative, Value::RomanInteger(value)) => {
                    Ok(Value::Number(value.into_number().negated()))
                }
                (UnaryOperator::Positive, Value::Complex(value)) => Ok(Value::Complex(value)),
                (UnaryOperator::Negative, Value::Complex(value)) => {
                    Ok(Value::Complex(value.negated()))
                }
                (UnaryOperator::Positive, Value::Percentage(percentage)) => {
                    Ok(Value::Percentage(percentage))
                }
                (UnaryOperator::Negative, Value::Number(number)) => {
                    Ok(Value::Number(number.negated()))
                }
                (UnaryOperator::Negative, Value::Percentage(percentage)) => {
                    Ok(Value::Percentage(percentage.negated()))
                }
                (UnaryOperator::Positive, Value::Fraction(fraction)) => {
                    Ok(Value::Fraction(fraction))
                }
                (UnaryOperator::Negative, Value::Fraction(fraction)) => {
                    Ok(Value::Fraction(fraction.negated()))
                }
                (UnaryOperator::Positive, Value::Unit(value)) => Ok(Value::Unit(value)),
                (UnaryOperator::Negative, Value::Unit(value)) => Ok(Value::Unit(value.negated())),
                (UnaryOperator::Positive, Value::Money(money)) => Ok(Value::Money(money)),
                (UnaryOperator::Negative, Value::Money(money)) => Ok(Value::Money(money.negated())),
                (UnaryOperator::Positive, Value::Rate(rate)) => Ok(Value::Rate(rate)),
                (UnaryOperator::Negative, Value::Rate(rate)) => Ok(Value::Rate(rate.negated())),
                (UnaryOperator::Positive, Value::MoneyRate(rate)) => Ok(Value::MoneyRate(rate)),
                (UnaryOperator::Negative, Value::MoneyRate(rate)) => {
                    Ok(Value::MoneyRate(rate.negated()))
                }
                (UnaryOperator::Positive, Value::TimeSpan(duration)) => {
                    Ok(Value::TimeSpan(duration))
                }
                (UnaryOperator::Negative, Value::TimeSpan(duration)) => duration
                    .checked_negated()
                    .map(Value::TimeSpan)
                    .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidTime, *operator_span)),
                (UnaryOperator::Positive, Value::CalendarDuration(duration)) => {
                    Ok(Value::CalendarDuration(duration))
                }
                (UnaryOperator::Negative, Value::CalendarDuration(duration)) => {
                    Ok(Value::CalendarDuration(duration.negated()))
                }
                (UnaryOperator::Positive, Value::CalendarSpan(duration)) => {
                    Ok(Value::CalendarSpan(duration))
                }
                (UnaryOperator::Negative, Value::CalendarSpan(duration)) => duration
                    .checked_negated()
                    .map(Value::CalendarSpan)
                    .ok_or_else(|| {
                        Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, *operator_span)
                    }),
                (UnaryOperator::Positive, Value::WorkdayDuration(duration)) => {
                    Ok(Value::WorkdayDuration(duration))
                }
                (UnaryOperator::Negative, Value::WorkdayDuration(duration)) => {
                    Ok(Value::WorkdayDuration(duration.negated()))
                }
                (UnaryOperator::Not, Value::Boolean(value)) => Ok(Value::Boolean(!value)),
                (
                    UnaryOperator::Positive | UnaryOperator::Negative,
                    Value::Dice(_)
                    | Value::Date(_)
                    | Value::Clock(_)
                    | Value::OffsetClock(_)
                    | Value::ZonedClock(_)
                    | Value::OffsetDateTime(_)
                    | Value::ZonedDateTime(_)
                    | Value::Weekday(_)
                    | Value::Boolean(_),
                )
                | (
                    UnaryOperator::Not,
                    Value::Number(_)
                    | Value::RadixInteger(_)
                    | Value::RomanInteger(_)
                    | Value::Complex(_)
                    | Value::Percentage(_)
                    | Value::Fraction(_)
                    | Value::Unit(_)
                    | Value::Money(_)
                    | Value::MoneyRate(_)
                    | Value::Rate(_)
                    | Value::Dice(_)
                    | Value::Date(_)
                    | Value::Clock(_)
                    | Value::OffsetClock(_)
                    | Value::ZonedClock(_)
                    | Value::OffsetDateTime(_)
                    | Value::ZonedDateTime(_)
                    | Value::TimeSpan(_)
                    | Value::CalendarDuration(_)
                    | Value::CalendarSpan(_)
                    | Value::WorkdayDuration(_)
                    | Value::Weekday(_),
                ) => Err(unsupported(*operator_span)),
            }
        }
        Expression::Group { expression, .. } => evaluate_expression(expression, context, variables),
        Expression::Percentage {
            expression,
            operator_span,
            ..
        } => match evaluate_expression(expression, context, variables)? {
            Value::Number(number) => Ok(Value::Percentage(Percentage::from_ratio(
                number
                    .divide(Number::from_i64(100))
                    .expect("one hundred is nonzero"),
            ))),
            _ => Err(unsupported(*operator_span)),
        },
        Expression::Conversion {
            expression,
            target,
            target_span,
            ..
        } => convert(
            evaluate_expression(expression, context, variables)?,
            target.clone(),
            *target_span,
            context,
        ),
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            inverted,
            keyword_span,
            ..
        } => {
            let Value::Boolean(mut condition) = evaluate_expression(condition, context, variables)?
            else {
                return Err(unsupported(*keyword_span));
            };
            if *inverted {
                condition = !condition;
            }
            if condition {
                evaluate_expression(then_branch, context, variables)
            } else {
                evaluate_expression(else_branch, context, variables)
            }
        }
        Expression::Binary {
            operator,
            operator_span,
            left,
            right,
            ..
        } => {
            let left = evaluate_expression(left, context, variables)?;
            if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
                let Value::Boolean(left) = left else {
                    return Err(unsupported(*operator_span));
                };
                if (*operator == BinaryOperator::And && !left)
                    || (*operator == BinaryOperator::Or && left)
                {
                    return Ok(Value::Boolean(left));
                }
                return match evaluate_expression(right, context, variables)? {
                    Value::Boolean(right) => Ok(Value::Boolean(right)),
                    _ => Err(unsupported(*operator_span)),
                };
            }

            let right = evaluate_expression(right, context, variables)?;
            evaluate_binary(*operator, left, right, *operator_span, context)
        }
    }
}
