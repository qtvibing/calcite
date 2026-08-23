mod calendar;
mod integer;
mod rounding;
mod scientific;
mod temporal;

use std::collections::HashMap;

use crate::evaluator::{Value, evaluate_expression};
use crate::finance::{
    evaluate_growth_function, evaluate_loan_function, evaluate_return_function,
    evaluate_tax_function,
};
use crate::operations::{evaluate_binary, unit_diagnostic};
use crate::{
    BinaryOperator, CalculationContext, Diagnostic, DiagnosticKind, Expression, Function, Number,
    Span,
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
        Function::Probability => match values.as_slice() {
            [Value::Dice(dice), Value::Number(total)] => total
                .exact_i64()
                .map(|total| Value::Percentage(dice.probability(total)))
                .ok_or_else(|| invalid_function_arguments(span)),
            _ => Err(invalid_function_arguments(span)),
        },
        Function::ExpectedValue => match values.as_slice() {
            [Value::Dice(dice)] => Ok(Value::Number(dice.mean())),
            _ => Err(invalid_function_arguments(span)),
        },
        Function::Binary
        | Function::Octal
        | Function::Hexadecimal
        | Function::Integer
        | Function::GreatestCommonDivisor
        | Function::LeastCommonMultiple
        | Function::IsPrime
        | Function::Factorial
        | Function::Permutation
        | Function::Combination => integer::evaluate(function, &values, span),
        Function::SquareRoot
        | Function::CubicRoot
        | Function::NthRoot
        | Function::NaturalLogarithm
        | Function::CommonLogarithm
        | Function::BinaryLogarithm
        | Function::Sine
        | Function::Cosine
        | Function::Tangent
        | Function::ArcSine
        | Function::ArcCosine
        | Function::ArcTangent
        | Function::ArcTangent2
        | Function::Exponential
        | Function::HyperbolicSine
        | Function::HyperbolicCosine
        | Function::HyperbolicTangent
        | Function::InverseHyperbolicSine
        | Function::InverseHyperbolicCosine
        | Function::InverseHyperbolicTangent => scientific::evaluate(function, &values, span),
        Function::Weekday
        | Function::IsWorkday
        | Function::AddWorkdays
        | Function::NextWorkday
        | Function::PreviousWorkday
        | Function::WorkdaysBetween => calendar::evaluate(function, &values, span, context),
        Function::AbsoluteValue | Function::Round | Function::Floor | Function::Ceiling => {
            rounding::evaluate(function, &values, span)
        }
        Function::TimeBetween
        | Function::TimeFrom
        | Function::TimeSince
        | Function::TimeUntil
        | Function::CalendarBetween
        | Function::CalendarFrom
        | Function::CalendarSince
        | Function::CalendarUntil => temporal::evaluate(function, &values, span, context),
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

fn invalid_function_arguments(span: Span) -> Diagnostic {
    Diagnostic::new(DiagnosticKind::InvalidFunctionArguments, span)
}
