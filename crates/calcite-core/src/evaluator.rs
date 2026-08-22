use std::{cmp::Ordering, collections::HashMap, fmt};

use crate::rate::RateApplication;
use crate::unit::UnitError;
use crate::{
    BinaryOperator, CalculationContext, CalendarDuration, ClockTime, Constant, ConversionTarget,
    Date, Diagnostic, DiagnosticKind, Expression, Fraction, Function, Money, Number, NumericError,
    OffsetDateTime, Percentage, Rate, RelativeWeekdayDirection, Span, TemporalKeyword,
    UnaryOperator, Unit, UnitValue, Weekday, WorkdayCalendar, WorkdayDuration, parse_with_context,
};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    Number(Number),
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
        Expression::Boolean { value, .. } => Ok(Value::Boolean(*value)),
        Expression::Constant { constant, .. } => Ok(Value::Number(match constant {
            Constant::Pi => Number::pi(),
            Constant::Tau => Number::pi().multiply(Number::from_i64(2)),
            Constant::E => Number::e(),
        })),
        Expression::Unit { value, unit, .. } => {
            Ok(Value::Unit(UnitValue::new(value.clone(), *unit)))
        }
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
            *target,
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

fn evaluate_call(
    function: Function,
    span: Span,
    arguments: &[Expression],
    context: &CalculationContext,
    variables: &HashMap<String, Value>,
) -> Result<Value, Diagnostic> {
    let values = arguments
        .iter()
        .map(|argument| evaluate_expression(argument, context, variables))
        .collect::<Result<Vec<_>, _>>()?;

    match function {
        Function::SquareRoot | Function::CubicRoot => {
            let [Value::Number(number)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let degree = if function == Function::SquareRoot {
                2
            } else {
                3
            };
            number
                .clone()
                .nth_root(degree)
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::NthRoot => {
            let [Value::Number(number), Value::Number(degree)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let degree = degree
                .exact_i64()
                .and_then(|degree| usize::try_from(degree).ok())
                .filter(|degree| *degree > 0)
                .ok_or_else(|| invalid_function_arguments(span))?;
            number
                .clone()
                .nth_root(degree)
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::NaturalLogarithm | Function::BinaryLogarithm => {
            let [Value::Number(number)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let result = if function == Function::NaturalLogarithm {
                number.clone().natural_log()
            } else {
                number.clone().binary_log()
            };
            result
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::CommonLogarithm => {
            let result = match values.as_slice() {
                [Value::Number(number)] => number.clone().common_log(),
                [Value::Number(number), Value::Number(base)] => {
                    number.clone().logarithm(base.clone())
                }
                _ => return Err(invalid_function_arguments(span)),
            };
            result
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::Sine | Function::Cosine | Function::Tangent => {
            let [value] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            if function == Function::Tangent && tangent_is_undefined(value) {
                return Err(Diagnostic::new(DiagnosticKind::InvalidTrigonometry, span));
            }
            let radians = angle_in_radians(value, span)?;
            Ok(Value::Number(match function {
                Function::Sine => radians.sine(),
                Function::Cosine => radians.cosine(),
                Function::Tangent => radians.tangent(),
                _ => unreachable!(),
            }))
        }
        Function::ArcSine | Function::ArcCosine | Function::ArcTangent => {
            let [Value::Number(number)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let radians = match function {
                Function::ArcSine => number.clone().arc_sine(),
                Function::ArcCosine => number.clone().arc_cosine(),
                Function::ArcTangent => Ok(number.clone().arc_tangent()),
                _ => unreachable!(),
            }
            .map_err(|error| numeric_diagnostic(error, span))?;
            Ok(Value::Unit(UnitValue::new(radians, Unit::Radian)))
        }
        Function::ArcTangent2 => {
            let [Value::Number(y), Value::Number(x)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            y.clone()
                .arc_tangent2(x.clone())
                .map(|radians| Value::Unit(UnitValue::new(radians, Unit::Radian)))
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::Exponential => {
            let [Value::Number(number)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            number
                .clone()
                .exponential()
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::HyperbolicSine | Function::HyperbolicCosine | Function::HyperbolicTangent => {
            let [Value::Number(number)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let result = match function {
                Function::HyperbolicSine => number.clone().hyperbolic_sine(),
                Function::HyperbolicCosine => number.clone().hyperbolic_cosine(),
                Function::HyperbolicTangent => number.clone().hyperbolic_tangent(),
                _ => unreachable!(),
            };
            result
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::InverseHyperbolicSine
        | Function::InverseHyperbolicCosine
        | Function::InverseHyperbolicTangent => {
            let [Value::Number(number)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let result = match function {
                Function::InverseHyperbolicSine => Ok(number.clone().inverse_hyperbolic_sine()),
                Function::InverseHyperbolicCosine => number.clone().inverse_hyperbolic_cosine(),
                Function::InverseHyperbolicTangent => number.clone().inverse_hyperbolic_tangent(),
                _ => unreachable!(),
            };
            result
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::Weekday | Function::IsWorkday => {
            let date = match values.as_slice() {
                [Value::Date(date)] => *date,
                [Value::OffsetDateTime(value)] => value.date(),
                _ => return Err(invalid_function_arguments(span)),
            };
            if function == Function::Weekday {
                Ok(Value::Weekday(date.weekday()))
            } else {
                Ok(Value::Boolean(
                    context.temporal.workday_calendar.is_workday(date),
                ))
            }
        }
        Function::AddWorkdays => match values.as_slice() {
            [Value::Date(date), Value::Number(days)] => {
                let days = days
                    .exact_i64()
                    .ok_or_else(|| invalid_function_arguments(span))?;
                context
                    .temporal
                    .workday_calendar
                    .add_workdays(*date, days)
                    .map(Value::Date)
                    .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))
            }
            [Value::OffsetDateTime(value), Value::Number(days)] => {
                let days = days
                    .exact_i64()
                    .ok_or_else(|| invalid_function_arguments(span))?;
                context
                    .temporal
                    .workday_calendar
                    .add_workdays(value.date(), days)
                    .map(|date| {
                        Value::OffsetDateTime(OffsetDateTime::new(
                            date,
                            value.time(),
                            value.offset(),
                        ))
                    })
                    .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))
            }
            _ => Err(invalid_function_arguments(span)),
        },
        Function::NextWorkday | Function::PreviousWorkday => {
            let days = if function == Function::NextWorkday {
                1
            } else {
                -1
            };
            match values.as_slice() {
                [Value::Date(date)] => context
                    .temporal
                    .workday_calendar
                    .add_workdays(*date, days)
                    .map(Value::Date)
                    .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span)),
                [Value::OffsetDateTime(value)] => context
                    .temporal
                    .workday_calendar
                    .add_workdays(value.date(), days)
                    .map(|date| {
                        Value::OffsetDateTime(OffsetDateTime::new(
                            date,
                            value.time(),
                            value.offset(),
                        ))
                    })
                    .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span)),
                _ => Err(invalid_function_arguments(span)),
            }
        }
        Function::WorkdaysBetween => {
            let (start, end) = match values.as_slice() {
                [Value::Date(start), Value::Date(end)] => (*start, *end),
                [Value::OffsetDateTime(start), Value::OffsetDateTime(end)] => {
                    (start.date(), end.date())
                }
                _ => return Err(invalid_function_arguments(span)),
            };
            Ok(Value::WorkdayDuration(WorkdayDuration::new(
                context
                    .temporal
                    .workday_calendar
                    .workdays_between(start, end),
            )))
        }
        Function::AbsoluteValue => {
            let [value] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            absolute_value(value.clone(), span)
        }
        Function::Round => {
            let (value, decimal_places) = match values.as_slice() {
                [value] => (value.clone(), 0),
                [value, Value::Number(decimal_places)] => (
                    value.clone(),
                    decimal_places
                        .exact_i64()
                        .ok_or_else(|| invalid_function_arguments(span))?,
                ),
                _ => return Err(invalid_function_arguments(span)),
            };
            round_value(value, decimal_places, FunctionRounding::Nearest, span)
        }
        Function::Floor | Function::Ceiling => {
            let [value] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let rounding = if function == Function::Floor {
                FunctionRounding::Floor
            } else {
                FunctionRounding::Ceiling
            };
            round_value(value.clone(), 0, rounding, span)
        }
        Function::Sum | Function::Average => {
            let count = values.len();
            let mut values = values.into_iter();
            let mut result = values
                .next()
                .ok_or_else(|| invalid_function_arguments(span))?;
            for value in values {
                result = evaluate_binary(BinaryOperator::Add, result, value, span, context)?;
            }
            if function == Function::Average {
                result = evaluate_binary(
                    BinaryOperator::Divide,
                    result,
                    Value::Number(Number::from_i64(
                        i64::try_from(count).map_err(|_| invalid_function_arguments(span))?,
                    )),
                    span,
                    context,
                )?;
            }
            Ok(result)
        }
        Function::Minimum | Function::Maximum => {
            let mut values = values.into_iter();
            let mut result = values
                .next()
                .ok_or_else(|| invalid_function_arguments(span))?;
            let comparison = if function == Function::Minimum {
                BinaryOperator::Less
            } else {
                BinaryOperator::Greater
            };
            for value in values {
                let Value::Boolean(replace) =
                    evaluate_binary(comparison, value.clone(), result.clone(), span, context)?
                else {
                    return Err(invalid_function_arguments(span));
                };
                if replace {
                    result = value;
                }
            }
            Ok(result)
        }
        Function::Median => {
            if values.is_empty() {
                return Err(invalid_function_arguments(span));
            }
            let preferred_unit = match &values[0] {
                Value::Unit(value) => Some(value.unit()),
                _ => None,
            };
            let mut sorted: Vec<Value> = Vec::with_capacity(values.len());
            for value in values {
                let mut position = sorted.len();
                for (index, existing) in sorted.iter().enumerate() {
                    let Value::Boolean(precedes) = evaluate_binary(
                        BinaryOperator::Less,
                        value.clone(),
                        existing.clone(),
                        span,
                        context,
                    )?
                    else {
                        return Err(invalid_function_arguments(span));
                    };
                    if precedes {
                        position = index;
                        break;
                    }
                }
                sorted.insert(position, value);
            }
            let middle = sorted.len() / 2;
            if sorted.len() % 2 == 1 {
                Ok(sorted[middle].clone())
            } else {
                let sum = evaluate_binary(
                    BinaryOperator::Add,
                    sorted[middle - 1].clone(),
                    sorted[middle].clone(),
                    span,
                    context,
                )?;
                let result = evaluate_binary(
                    BinaryOperator::Divide,
                    sum,
                    Value::Number(Number::from_i64(2)),
                    span,
                    context,
                )?;
                match (result, preferred_unit) {
                    (Value::Unit(value), Some(unit)) => value
                        .converted_to(unit)
                        .map(Value::Unit)
                        .map_err(|error| unit_diagnostic(error, span)),
                    (result, _) => Ok(result),
                }
            }
        }
        Function::Count => Ok(Value::Number(Number::from_i64(
            i64::try_from(values.len()).map_err(|_| invalid_function_arguments(span))?,
        ))),
        Function::Product => {
            let mut values = values.into_iter();
            let mut result = values
                .next()
                .ok_or_else(|| invalid_function_arguments(span))?;
            for value in values {
                result = evaluate_binary(BinaryOperator::Multiply, result, value, span, context)?;
            }
            Ok(result)
        }
    }
}

fn tangent_is_undefined(value: &Value) -> bool {
    let Value::Unit(value) = value else {
        return false;
    };
    if value.unit() != Unit::Degree {
        return false;
    }
    value
        .magnitude()
        .clone()
        .subtract(Number::from_i64(90))
        .divide(Number::from_i64(180))
        .ok()
        .and_then(|turns| turns.exact_i64())
        .is_some()
}

fn angle_in_radians(value: &Value, span: Span) -> Result<Number, Diagnostic> {
    match value {
        Value::Number(number) => Ok(number.clone()),
        Value::Unit(value) if value.unit().dimension() == crate::Dimension::Angle => value
            .converted_to(Unit::Radian)
            .map(|value| value.magnitude().clone())
            .map_err(|error| unit_diagnostic(error, span)),
        _ => Err(invalid_function_arguments(span)),
    }
}

#[derive(Clone, Copy)]
enum FunctionRounding {
    Nearest,
    Floor,
    Ceiling,
}

fn round_value(
    value: Value,
    decimal_places: i64,
    rounding: FunctionRounding,
    span: Span,
) -> Result<Value, Diagnostic> {
    let round_number = |number: Number| match rounding {
        FunctionRounding::Nearest => number.rounded(decimal_places),
        FunctionRounding::Floor => Ok(number.floor()),
        FunctionRounding::Ceiling => Ok(number.ceiling()),
    };
    match value {
        Value::Number(number) => round_number(number)
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        Value::Fraction(fraction) => round_number(fraction.into_number())
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        Value::Percentage(percentage) => {
            let displayed = percentage.into_ratio().multiply(Number::from_i64(100));
            let rounded = round_number(displayed)
                .map_err(|error| numeric_diagnostic(error, span))?
                .divide(Number::from_i64(100))
                .expect("one hundred is nonzero");
            Ok(Value::Percentage(Percentage::from_ratio(rounded)))
        }
        Value::Unit(value) => {
            let unit = value.unit();
            round_number(value.magnitude().clone())
                .map(|magnitude| Value::Unit(UnitValue::new(magnitude, unit)))
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Value::Money(money) => {
            let currency = money.currency();
            round_number(money.amount().clone())
                .map(|amount| Value::Money(Money::new(amount, currency)))
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Value::Rate(rate) => {
            let numerator = rate.numerator();
            let denominator = rate.denominator();
            round_number(rate.magnitude().clone())
                .map(|magnitude| Value::Rate(Rate::new(magnitude, numerator, denominator)))
                .map_err(|error| numeric_diagnostic(error, span))
        }
        _ => Err(invalid_function_arguments(span)),
    }
}

fn absolute_value(value: Value, span: Span) -> Result<Value, Diagnostic> {
    let negative = |number: &Number| {
        number
            .compare(&Number::from_i64(0))
            .is_some_and(Ordering::is_lt)
    };
    match value {
        Value::Number(number) if negative(&number) => Ok(Value::Number(number.negated())),
        Value::Percentage(percentage) if negative(percentage.ratio()) => {
            Ok(Value::Percentage(percentage.negated()))
        }
        Value::Fraction(fraction) if negative(fraction.number()) => Ok(Value::Fraction(
            Fraction::from_number(fraction.into_number().negated())
                .expect("negating an exact fraction stays exact"),
        )),
        Value::Unit(value) if negative(value.magnitude()) => Ok(Value::Unit(value.negated())),
        Value::Money(money) if negative(money.amount()) => Ok(Value::Money(money.negated())),
        Value::Rate(rate) if negative(rate.magnitude()) => Ok(Value::Rate(rate.negated())),
        Value::CalendarDuration(duration) if duration.total_months() < 0 => {
            Ok(Value::CalendarDuration(duration.negated()))
        }
        Value::WorkdayDuration(duration) if duration.days() < 0 => {
            Ok(Value::WorkdayDuration(duration.negated()))
        }
        Value::Number(_)
        | Value::Percentage(_)
        | Value::Fraction(_)
        | Value::Unit(_)
        | Value::Money(_)
        | Value::Rate(_)
        | Value::CalendarDuration(_)
        | Value::WorkdayDuration(_) => Ok(value),
        _ => Err(invalid_function_arguments(span)),
    }
}

fn invalid_function_arguments(span: Span) -> Diagnostic {
    Diagnostic::new(DiagnosticKind::InvalidFunctionArguments, span)
}

fn evaluate_binary(
    operator: BinaryOperator,
    left: Value,
    right: Value,
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
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
        (Value::Rate(left), Value::Unit(right)) if operator == BinaryOperator::Multiply => {
            apply_rate(left, right, span)
        }
        (Value::Unit(left), Value::Rate(right)) if operator == BinaryOperator::Multiply => {
            apply_rate(right, left, span)
        }
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
        BinaryOperator::Modulo
        | BinaryOperator::Remainder
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
        BinaryOperator::Modulo
        | BinaryOperator::Remainder
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
        BinaryOperator::Modulo
        | BinaryOperator::Remainder
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
        BinaryOperator::Divide if left.unit().dimension() != right.unit().dimension() => {
            let magnitude = left
                .magnitude()
                .clone()
                .divide(right.magnitude().clone())
                .map_err(|error| numeric_diagnostic(error, span))?;
            Ok(Value::Rate(Rate::new(
                magnitude,
                Some(left.unit()),
                right.unit(),
            )))
        }
        BinaryOperator::Divide => left
            .ratio(right)
            .map(Value::Number)
            .map_err(|error| unit_diagnostic(error, span)),
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
    let right = right
        .converted_to(left.numerator(), left.denominator())
        .map_err(|error| unit_diagnostic(error, span))?;
    match operator {
        BinaryOperator::Add => Ok(Value::Rate(Rate::new(
            left.magnitude().clone().add(right.magnitude().clone()),
            left.numerator(),
            left.denominator(),
        ))),
        BinaryOperator::Subtract => Ok(Value::Rate(Rate::new(
            left.magnitude().clone().subtract(right.magnitude().clone()),
            left.numerator(),
            left.denominator(),
        ))),
        BinaryOperator::Divide => left
            .magnitude()
            .clone()
            .divide(right.magnitude().clone())
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
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
        _ => Err(unsupported(span)),
    }
}

fn apply_rate(rate: Rate, value: UnitValue, span: Span) -> Result<Value, Diagnostic> {
    match rate
        .apply(value)
        .map_err(|error| unit_diagnostic(error, span))?
    {
        RateApplication::Number(number) => Ok(Value::Number(number)),
        RateApplication::Unit(value) => Ok(Value::Unit(value)),
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
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            let unit = left.unit();
            unit_pair(operator, left, UnitValue::new(right, unit), span)
        }
        _ => Err(unsupported(span)),
    }
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
            let denominator = right.unit();
            left.divide(right.magnitude().clone())
                .map(|magnitude| Value::Rate(Rate::new(magnitude, None, denominator)))
                .map_err(|error| numeric_diagnostic(error, span))
        }
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            let unit = right.unit();
            unit_pair(operator, UnitValue::new(left, unit), right, span)
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

fn convert(
    value: Value,
    target: ConversionTarget,
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    match target {
        ConversionTarget::Number => match value {
            Value::Number(number) => Ok(Value::Number(number)),
            Value::Percentage(percentage) => Ok(Value::Number(percentage.into_ratio())),
            Value::Fraction(fraction) => Ok(Value::Number(fraction.into_number())),
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
        ConversionTarget::Percentage => match value {
            Value::Number(number) => Ok(Value::Percentage(Percentage::from_ratio(number))),
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
        ConversionTarget::Unit(target) => match value {
            Value::Unit(value) => value
                .converted_to(target)
                .map(Value::Unit)
                .map_err(|error| unit_diagnostic(error, span)),
            _ => Err(unsupported(span)),
        },
        ConversionTarget::Rate {
            numerator,
            denominator,
        } => match value {
            Value::Rate(rate) => rate
                .converted_to(numerator, denominator)
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
                    .rate(source, target)
                    .ok_or_else(|| {
                        Diagnostic::with_detail(
                            DiagnosticKind::MissingExchangeRate,
                            span,
                            format!("{source} to {target}"),
                        )
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

fn unit_diagnostic(error: UnitError, span: Span) -> Diagnostic {
    match error {
        UnitError::IncompatibleDimensions => Diagnostic::new(DiagnosticKind::UnitMismatch, span),
        UnitError::AffineArithmetic => unsupported(span),
        UnitError::Numeric(error) => numeric_diagnostic(error, span),
    }
}

fn numeric_diagnostic(error: NumericError, span: Span) -> Diagnostic {
    let kind = match error {
        NumericError::DivisionByZero => DiagnosticKind::DivisionByZero,
        NumericError::ExponentTooLarge => DiagnosticKind::ExponentTooLarge,
        NumericError::PrecisionTooLarge => DiagnosticKind::PrecisionTooLarge,
        NumericError::RootDegreeTooLarge => DiagnosticKind::RootDegreeTooLarge,
        NumericError::InvalidLogarithm => DiagnosticKind::InvalidLogarithm,
        NumericError::InvalidTrigonometry => DiagnosticKind::InvalidTrigonometry,
        NumericError::InvalidHyperbolic => DiagnosticKind::InvalidHyperbolic,
        NumericError::IntegerRequired => DiagnosticKind::IntegerRequired,
        NumericError::NonRealResult => DiagnosticKind::NonRealResult,
    };
    Diagnostic::new(kind, span)
}

fn unsupported(span: Span) -> Diagnostic {
    Diagnostic::new(DiagnosticKind::UnsupportedOperation, span)
}
