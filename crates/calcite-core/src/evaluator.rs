use std::{collections::HashMap, fmt};

use crate::functions::evaluate_call;
use crate::operations::{classify_compound_value, convert, evaluate_binary, unsupported};
use crate::{
    BinaryOperator, CalculationContext, CalendarDuration, ClockTime, Constant, Date, Diagnostic,
    DiagnosticKind, Expression, Fraction, Money, Number, OffsetDateTime, Percentage, RadixInteger,
    Rate, RelativeWeekdayDirection, TemporalKeyword, UnaryOperator, UnitValue, Weekday,
    WorkdayDuration, parse_with_context,
};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    Number(Number),
    RadixInteger(RadixInteger),
    Percentage(Percentage),
    Fraction(Fraction),
    Unit(UnitValue),
    Money(Money),
    Rate(Rate),
    Date(Date),
    Clock(ClockTime),
    OffsetDateTime(OffsetDateTime),
    CalendarDuration(CalendarDuration),
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

    pub const fn as_rate(&self) -> Option<&Rate> {
        match self {
            Self::Rate(rate) => Some(rate),
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

    pub const fn as_offset_datetime(&self) -> Option<OffsetDateTime> {
        match self {
            Self::OffsetDateTime(value) => Some(*value),
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
            Self::Percentage(percentage) => percentage.fmt(formatter),
            Self::Fraction(fraction) => fraction.fmt(formatter),
            Self::Unit(value) => {
                write!(formatter, "{} {}", value.magnitude(), value.display_unit())
            }
            Self::Money(money) => money.fmt(formatter),
            Self::Rate(rate) => rate.fmt(formatter),
            Self::Date(date) => date.fmt(formatter),
            Self::Clock(time) => time.fmt(formatter),
            Self::OffsetDateTime(value) => value.fmt(formatter),
            Self::CalendarDuration(duration) => duration.fmt(formatter),
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
        Expression::Date { value, .. } => Ok(Value::Date(*value)),
        Expression::Clock { value, .. } => Ok(Value::Clock(*value)),
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
            .get(name)
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
                (UnaryOperator::Positive, Value::CalendarDuration(duration)) => {
                    Ok(Value::CalendarDuration(duration))
                }
                (UnaryOperator::Negative, Value::CalendarDuration(duration)) => {
                    Ok(Value::CalendarDuration(duration.negated()))
                }
                (UnaryOperator::Positive, Value::WorkdayDuration(duration)) => {
                    Ok(Value::WorkdayDuration(duration))
                }
                (UnaryOperator::Negative, Value::WorkdayDuration(duration)) => {
                    Ok(Value::WorkdayDuration(duration.negated()))
                }
                (UnaryOperator::Not, Value::Boolean(value)) => Ok(Value::Boolean(!value)),
                (
                    UnaryOperator::Positive | UnaryOperator::Negative,
                    Value::Date(_)
                    | Value::Clock(_)
                    | Value::OffsetDateTime(_)
                    | Value::Weekday(_)
                    | Value::Boolean(_),
                )
                | (
                    UnaryOperator::Not,
                    Value::Number(_)
                    | Value::RadixInteger(_)
                    | Value::Percentage(_)
                    | Value::Fraction(_)
                    | Value::Unit(_)
                    | Value::Money(_)
                    | Value::Rate(_)
                    | Value::Date(_)
                    | Value::Clock(_)
                    | Value::OffsetDateTime(_)
                    | Value::CalendarDuration(_)
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
