use std::cmp::Ordering;

use crate::evaluator::Value;
use crate::operations::numeric_diagnostic;
use crate::{Diagnostic, Fraction, Function, Money, Number, Percentage, Rate, Span, UnitValue};

use super::invalid_function_arguments;

pub(super) fn evaluate(
    function: Function,
    values: &[Value],
    span: Span,
) -> Result<Value, Diagnostic> {
    match function {
        Function::AbsoluteValue => {
            let [value] = values else {
                return Err(invalid_function_arguments(span));
            };
            absolute_value(value.clone(), span)
        }
        Function::Round => {
            let (value, decimal_places) = match values {
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
            let [value] = values else {
                return Err(invalid_function_arguments(span));
            };
            let rounding = if function == Function::Floor {
                FunctionRounding::Floor
            } else {
                FunctionRounding::Ceiling
            };
            round_value(value.clone(), 0, rounding, span)
        }
        _ => unreachable!("rounding dispatch only passes rounding functions"),
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
