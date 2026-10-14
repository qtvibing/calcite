use std::{collections::HashMap, fmt};

use crate::functions::evaluate_call;
use crate::operations::{classify_compound_value, convert, evaluate_binary, unsupported};
use crate::syntax::identifier_key;
use crate::{
    BinaryOperator, CalculationContext, CalendarDuration, CalendarSpan, ClockTime, CoinSide,
    ComplexNumber, Constant, CustomUnitValue, Date, Diagnostic, DiagnosticKind, DiceDistribution,
    EntropySource, Expression, Fraction, Money, MoneyRate, Number, OffsetClockTime, OffsetDateTime,
    Percentage, RadixInteger, Rate, RelativeWeekdayDirection, RomanInteger, Span, TemporalKeyword,
    TimeSpan, UnaryOperator, UnitValue, UserFunction, Weekday, WorkdayDuration, ZonedClockTime,
    ZonedDateTime, parse_with_context,
};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    Number(Number),
    String(String),
    RadixInteger(RadixInteger),
    RomanInteger(RomanInteger),
    Complex(ComplexNumber),
    Function(UserFunction),
    Percentage(Percentage),
    Fraction(Fraction),
    Unit(UnitValue),
    CustomUnit(CustomUnitValue),
    Money(Money),
    MoneyRate(MoneyRate),
    Rate(Rate),
    Dice(DiceDistribution),
    Coin(CoinSide),
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
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub const fn as_function(&self) -> Option<&UserFunction> {
        match self {
            Self::Function(value) => Some(value),
            _ => None,
        }
    }

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

    pub const fn as_custom_unit(&self) -> Option<&CustomUnitValue> {
        match self {
            Self::CustomUnit(value) => Some(value),
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

    pub const fn as_coin(&self) -> Option<CoinSide> {
        match self {
            Self::Coin(side) => Some(*side),
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
            Self::String(value) => value.fmt(formatter),
            Self::RadixInteger(value) => value.fmt(formatter),
            Self::RomanInteger(value) => value.fmt(formatter),
            Self::Complex(value) => value.fmt(formatter),
            Self::Function(function) => write!(
                formatter,
                "<function({})>",
                function.parameters().join(", ")
            ),
            Self::Percentage(percentage) => percentage.fmt(formatter),
            Self::Fraction(fraction) => fraction.fmt(formatter),
            Self::Unit(value) => {
                write!(formatter, "{} {}", value.magnitude(), value.display_unit())
            }
            Self::CustomUnit(value) => {
                write!(
                    formatter,
                    "{} {}",
                    value.magnitude(),
                    value.definition().name()
                )
            }
            Self::Money(money) => money.fmt(formatter),
            Self::MoneyRate(rate) => rate.fmt(formatter),
            Self::Rate(rate) => rate.fmt(formatter),
            Self::Dice(dice) => dice.fmt(formatter),
            Self::Coin(side) => side.fmt(formatter),
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
    let mut entropy = None;
    evaluate_with_optional_entropy(source, context, &mut entropy)
}

/// Evaluates one expression with the default context and caller-supplied entropy.
pub fn evaluate_with_entropy(
    source: &str,
    entropy: &mut impl EntropySource,
) -> Result<Value, Diagnostic> {
    evaluate_with_context_and_entropy(source, &CalculationContext::default(), entropy)
}

/// Evaluates one expression with explicit context and caller-supplied entropy.
pub fn evaluate_with_context_and_entropy(
    source: &str,
    context: &CalculationContext,
    entropy: &mut impl EntropySource,
) -> Result<Value, Diagnostic> {
    let mut entropy = Some(entropy as &mut dyn EntropySource);
    evaluate_with_optional_entropy(source, context, &mut entropy)
}

fn evaluate_with_optional_entropy(
    source: &str,
    context: &CalculationContext,
    entropy: &mut Option<&mut dyn EntropySource>,
) -> Result<Value, Diagnostic> {
    let expression = parse_with_context(source, context)?;
    evaluate_expression(&expression, context, &HashMap::new(), entropy)
}

pub(crate) fn evaluate_expression(
    expression: &Expression,
    context: &CalculationContext,
    variables: &HashMap<String, Value>,
    entropy: &mut Option<&mut dyn EntropySource>,
) -> Result<Value, Diagnostic> {
    match expression {
        Expression::Number { value, .. } => Ok(Value::Number(value.clone())),
        Expression::String { value, .. } => Ok(Value::String(value.clone())),
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
        Expression::CustomUnit {
            value, definition, ..
        } => Ok(Value::CustomUnit(CustomUnitValue::new(
            value.clone(),
            definition.clone(),
        ))),
        Expression::Money {
            amount, currency, ..
        } => Ok(Value::Money(Money::new(amount.clone(), *currency))),
        Expression::CurrencyAttach {
            expression,
            currency,
            span,
            ..
        } => {
            let value = evaluate_expression(expression, context, variables, entropy)?;
            match value {
                Value::Number(amount) => Ok(Value::Money(Money::new(amount, *currency))),
                Value::Money(money) => Err(Diagnostic::with_detail(
                    DiagnosticKind::CurrencyMismatch,
                    *span,
                    format!("this value already has {}", money.currency()),
                )),
                _ => Err(Diagnostic::with_detail(
                    DiagnosticKind::UnsupportedOperation,
                    *span,
                    "a currency can only label a plain number",
                )),
            }
        }
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
            TemporalKeyword::Holiday {
                month,
                day,
                year,
                previous,
            } => {
                if let Some(year) = *year {
                    let date = Date::new(year, *month, *day)
                        .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, *span))?;
                    return Ok(Value::Date(date));
                }
                let current = context
                    .temporal
                    .current_datetime
                    .map(OffsetDateTime::date)
                    .ok_or_else(|| {
                        Diagnostic::new(DiagnosticKind::MissingTemporalContext, *span)
                    })?;
                let occurrence = |year: i32| Date::new(year, *month, *day);
                let date = if *previous {
                    occurrence(current.year())
                        .filter(|date| *date < current)
                        .or_else(|| occurrence(current.year() - 1))
                } else {
                    occurrence(current.year())
                        .filter(|date| *date >= current)
                        .or_else(|| occurrence(current.year() + 1))
                }
                .map(Value::Date)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, *span))?;
                Ok(date)
            }
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
            .ok_or_else(|| unknown_variable(name, *span, variables)),
        Expression::Call {
            function,
            function_span,
            arguments,
            ..
        } => evaluate_call(
            *function,
            *function_span,
            arguments,
            context,
            variables,
            entropy,
        ),
        Expression::UserCall {
            name,
            name_span,
            arguments,
            ..
        } => {
            let function = variables
                .get(&identifier_key(name))
                .and_then(Value::as_function)
                .ok_or_else(|| unknown_variable(name, *name_span, variables))?;
            invoke_user_function(
                function,
                Some(identifier_key(name)),
                arguments,
                *name_span,
                context,
                variables,
                entropy,
            )
        }
        Expression::Lambda {
            parameters, body, ..
        } => Ok(Value::Function(UserFunction::new(
            parameters.clone(),
            body.as_ref().clone(),
        ))),
        Expression::Invoke {
            function,
            arguments,
            span,
        } => {
            let function = evaluate_expression(function, context, variables, entropy)?;
            let Value::Function(function) = function else {
                return Err(Diagnostic::new(
                    DiagnosticKind::InvalidFunctionArguments,
                    *span,
                ));
            };
            invoke_user_function(
                &function, None, arguments, *span, context, variables, entropy,
            )
        }
        Expression::Unary {
            operator,
            operator_span,
            operand,
            ..
        } => {
            let operand = evaluate_expression(operand, context, variables, entropy)?;
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
                (UnaryOperator::Positive, Value::CustomUnit(value)) => Ok(Value::CustomUnit(value)),
                (UnaryOperator::Negative, Value::CustomUnit(value)) => {
                    Ok(Value::CustomUnit(value.negated()))
                }
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
                    Value::String(_)
                    | Value::Function(_)
                    | Value::Dice(_)
                    | Value::Coin(_)
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
                    | Value::String(_)
                    | Value::Function(_)
                    | Value::RadixInteger(_)
                    | Value::RomanInteger(_)
                    | Value::Complex(_)
                    | Value::Percentage(_)
                    | Value::Fraction(_)
                    | Value::Unit(_)
                    | Value::CustomUnit(_)
                    | Value::Money(_)
                    | Value::MoneyRate(_)
                    | Value::Rate(_)
                    | Value::Dice(_)
                    | Value::Coin(_)
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
        Expression::Group { expression, .. } => {
            evaluate_expression(expression, context, variables, entropy)
        }
        Expression::Percentage {
            expression,
            operator_span,
            ..
        } => match evaluate_expression(expression, context, variables, entropy)? {
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
            evaluate_expression(expression, context, variables, entropy)?,
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
            let Value::Boolean(mut condition) =
                evaluate_expression(condition, context, variables, entropy)?
            else {
                return Err(unsupported(*keyword_span));
            };
            if *inverted {
                condition = !condition;
            }
            if condition {
                evaluate_expression(then_branch, context, variables, entropy)
            } else {
                evaluate_expression(else_branch, context, variables, entropy)
            }
        }
        Expression::Binary {
            operator,
            operator_span,
            left,
            right,
            ..
        } => {
            let left = evaluate_expression(left, context, variables, entropy)?;
            if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
                let Value::Boolean(left) = left else {
                    return Err(unsupported(*operator_span));
                };
                if (*operator == BinaryOperator::And && !left)
                    || (*operator == BinaryOperator::Or && left)
                {
                    return Ok(Value::Boolean(left));
                }
                return match evaluate_expression(right, context, variables, entropy)? {
                    Value::Boolean(right) => Ok(Value::Boolean(right)),
                    _ => Err(unsupported(*operator_span)),
                };
            }

            let right = evaluate_expression(right, context, variables, entropy)?;
            evaluate_binary(*operator, left, right, *operator_span, context)
        }
    }
}

fn unknown_variable(name: &str, span: Span, variables: &HashMap<String, Value>) -> Diagnostic {
    let normalized = identifier_key(name);
    let suggestion = variables
        .keys()
        .filter_map(|candidate| {
            let distance = edit_distance(&normalized, candidate);
            (distance <= 2).then_some((distance, candidate))
        })
        .min_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)))
        .map(|(_, candidate)| candidate.as_str());
    let detail = suggestion
        .map(|candidate| format!("{name}; did you mean '{candidate}'?"))
        .unwrap_or_else(|| name.to_owned());
    Diagnostic::with_detail(DiagnosticKind::UnknownVariable, span, detail)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let mut previous: Vec<usize> = (0..=right.chars().count()).collect();
    let mut current = vec![0; previous.len()];
    for (left_index, left_character) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_character) in right.chars().enumerate() {
            current[right_index + 1] = if left_character == right_character {
                previous[right_index]
            } else {
                1 + previous[right_index]
                    .min(previous[right_index + 1])
                    .min(current[right_index])
            };
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.chars().count()]
}

fn invoke_user_function(
    function: &UserFunction,
    name: Option<String>,
    arguments: &[Expression],
    span: Span,
    context: &CalculationContext,
    variables: &HashMap<String, Value>,
    entropy: &mut Option<&mut dyn EntropySource>,
) -> Result<Value, Diagnostic> {
    if function.is_active() {
        return Err(Diagnostic::new(DiagnosticKind::RecursiveFunction, span));
    }
    if arguments.len() != function.parameters().len() {
        return Err(Diagnostic::with_detail(
            DiagnosticKind::InvalidFunctionArguments,
            span,
            format!(
                "expected {} argument(s), received {}",
                function.parameters().len(),
                arguments.len()
            ),
        ));
    }
    let mut argument_values = Vec::with_capacity(arguments.len());
    for argument in arguments {
        argument_values.push(evaluate_expression(argument, context, variables, entropy)?);
    }
    let mut local_variables = variables.clone();
    if let Some(name) = name {
        local_variables.insert(name, Value::Function(function.activated()));
    }
    for (parameter, value) in function.parameters().iter().zip(argument_values) {
        local_variables.insert(identifier_key(parameter), value);
    }
    evaluate_expression(function.body(), context, &local_variables, entropy)
}
