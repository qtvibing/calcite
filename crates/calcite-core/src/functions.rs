use std::{cmp::Ordering, collections::HashMap};

use crate::evaluator::{Value, evaluate_expression};
use crate::finance::{
    evaluate_growth_function, evaluate_loan_function, evaluate_return_function,
    evaluate_tax_function,
};
use crate::operations::{evaluate_binary, numeric_diagnostic, unit_diagnostic};
use crate::{
    BinaryOperator, CalculationContext, Diagnostic, DiagnosticKind, Expression, Fraction, Function,
    Money, Number, OffsetDateTime, Percentage, RadixInteger, Rate, Span, Unit, UnitValue,
    WorkdayDuration,
};

pub(super) fn evaluate_call(
    function: Function,
    span: Span,
    arguments: &[Expression],
    context: &CalculationContext,
    variables: &HashMap<String, Value>,
) -> Result<Value, Diagnostic> {
    let values = arguments
        .iter()
        .map(|argument| {
            evaluate_expression(argument, context, variables).map(|value| match value {
                Value::RadixInteger(value) => Value::Number(value.into_number()),
                value => value,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    match function {
        Function::Binary | Function::Octal | Function::Hexadecimal => {
            let [Value::Number(number)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let radix = match function {
                Function::Binary => crate::Radix::Binary,
                Function::Octal => crate::Radix::Octal,
                Function::Hexadecimal => crate::Radix::Hexadecimal,
                _ => unreachable!(),
            };
            RadixInteger::new(number.clone(), radix)
                .map(Value::RadixInteger)
                .map_err(|_| Diagnostic::new(DiagnosticKind::IntegerRequired, span))
        }
        Function::Integer => {
            let [Value::Number(number)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            if number.is_integer() {
                Ok(Value::Number(number.clone()))
            } else {
                Err(Diagnostic::new(DiagnosticKind::IntegerRequired, span))
            }
        }
        Function::GreatestCommonDivisor | Function::LeastCommonMultiple => {
            let [Value::Number(first), Value::Number(second), rest @ ..] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let mut result = if function == Function::GreatestCommonDivisor {
                first.clone().greatest_common_divisor(second.clone())
            } else {
                first.clone().least_common_multiple(second.clone())
            };
            for value in rest {
                let Value::Number(value) = value else {
                    return Err(invalid_function_arguments(span));
                };
                result = result.and_then(|result| {
                    if function == Function::GreatestCommonDivisor {
                        result.greatest_common_divisor(value.clone())
                    } else {
                        result.least_common_multiple(value.clone())
                    }
                });
            }
            result
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::IsPrime => {
            let [Value::Number(number)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            number
                .is_prime()
                .map(Value::Boolean)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::Factorial => {
            let [Value::Number(number)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            number
                .clone()
                .factorial()
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::Permutation | Function::Combination => {
            let [Value::Number(population), Value::Number(selection)] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let result = if function == Function::Permutation {
                population.clone().permutation(selection.clone())
            } else {
                population.clone().combination(selection.clone())
            };
            result
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
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
        Function::SalesTax | Function::AddTax | Function::RemoveTax | Function::TaxFromTotal => {
            evaluate_tax_function(function, &values, span, context)
        }
        Function::FutureValue
        | Function::CompoundInterest
        | Function::PresentValue
        | Function::PurchasingPower => evaluate_growth_function(function, &values, span, context),
        Function::CompoundAnnualGrowthRate | Function::ReturnOnInvestment => {
            evaluate_return_function(function, &values, span, context)
        }
        Function::MortgagePayment | Function::TotalRepayment | Function::TotalInterest => {
            evaluate_loan_function(function, &values, span, context)
        }
        Function::BreakEven => {
            let [fixed_cost, contribution] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            evaluate_binary(
                BinaryOperator::Divide,
                fixed_cost.clone(),
                contribution.clone(),
                span,
                context,
            )
        }
        Function::Clamp => {
            let [value, lower, upper] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let Value::Boolean(invalid_bounds) = evaluate_binary(
                BinaryOperator::Greater,
                lower.clone(),
                upper.clone(),
                span,
                context,
            )?
            else {
                return Err(invalid_function_arguments(span));
            };
            if invalid_bounds {
                return Err(Diagnostic::new(DiagnosticKind::InvalidRange, span));
            }
            let Value::Boolean(below) = evaluate_binary(
                BinaryOperator::Less,
                value.clone(),
                lower.clone(),
                span,
                context,
            )?
            else {
                return Err(invalid_function_arguments(span));
            };
            if below {
                return Ok(lower.clone());
            }
            let Value::Boolean(above) = evaluate_binary(
                BinaryOperator::Greater,
                value.clone(),
                upper.clone(),
                span,
                context,
            )?
            else {
                return Err(invalid_function_arguments(span));
            };
            Ok(if above { upper.clone() } else { value.clone() })
        }
        Function::Midpoint => {
            let [first, second] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let sum = evaluate_binary(
                BinaryOperator::Add,
                first.clone(),
                second.clone(),
                span,
                context,
            )?;
            evaluate_binary(
                BinaryOperator::Divide,
                sum,
                Value::Number(Number::from_i64(2)),
                span,
                context,
            )
        }
        Function::ProportionDenominator | Function::ProportionNumerator => {
            let [first, second, third] = values.as_slice() else {
                return Err(invalid_function_arguments(span));
            };
            let (numerator, denominator) = if function == Function::ProportionDenominator {
                (second, first)
            } else {
                (first, second)
            };
            let ratio = evaluate_binary(
                BinaryOperator::Divide,
                numerator.clone(),
                denominator.clone(),
                span,
                context,
            )?;
            evaluate_binary(
                BinaryOperator::Multiply,
                ratio,
                third.clone(),
                span,
                context,
            )
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
                Value::Unit(value) => Some(value.compound_unit().clone()),
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
                        .converted_to_compound(&unit)
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
    if value.atomic_unit() != Some(Unit::Degree) {
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
        Value::Unit(value) if matches!(value.atomic_unit(), Some(Unit::Degree | Unit::Radian)) => {
            value
                .converted_to(Unit::Radian)
                .map(|value| value.magnitude().clone())
                .map_err(|error| unit_diagnostic(error, span))
        }
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
            let unit = value.compound_unit().clone();
            round_number(value.magnitude().clone())
                .map(|magnitude| Value::Unit(UnitValue::from_compound(magnitude, unit)))
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Value::Money(money) => {
            let currency = money.currency();
            round_number(money.amount().clone())
                .map(|amount| Value::Money(Money::new(amount, currency)))
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Value::Rate(rate) => {
            let unit = rate.compound_unit().clone();
            round_number(rate.magnitude().clone())
                .map(|magnitude| Value::Rate(Rate::from_compound(magnitude, unit)))
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
