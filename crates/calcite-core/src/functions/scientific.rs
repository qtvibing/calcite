use std::cmp::Ordering;

use crate::evaluator::Value;
use crate::operations::{numeric_diagnostic, unit_diagnostic};
use crate::{ComplexNumber, Diagnostic, DiagnosticKind, Function, Number, Span, Unit, UnitValue};

use super::invalid_function_arguments;

pub(super) fn evaluate(
    function: Function,
    values: &[Value],
    span: Span,
) -> Result<Value, Diagnostic> {
    match function {
        Function::SquareRoot | Function::CubicRoot => {
            let [Value::Number(number)] = values else {
                return Err(invalid_function_arguments(span));
            };
            let degree = if function == Function::SquareRoot {
                2
            } else {
                3
            };
            if function == Function::SquareRoot
                && number.compare(&Number::from_i64(0)) == Some(Ordering::Less)
            {
                return number
                    .clone()
                    .negated()
                    .nth_root(2)
                    .map(|imaginary| {
                        Value::Complex(ComplexNumber::new(Number::from_i64(0), imaginary))
                    })
                    .map_err(|error| numeric_diagnostic(error, span));
            }
            number
                .clone()
                .nth_root(degree)
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::NthRoot => {
            let [Value::Number(number), Value::Number(degree)] = values else {
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
            let [Value::Number(number)] = values else {
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
            let result = match values {
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
            let [value] = values else {
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
            let [Value::Number(number)] = values else {
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
            let [Value::Number(y), Value::Number(x)] = values else {
                return Err(invalid_function_arguments(span));
            };
            y.clone()
                .arc_tangent2(x.clone())
                .map(|radians| Value::Unit(UnitValue::new(radians, Unit::Radian)))
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::Exponential => {
            let [Value::Number(number)] = values else {
                return Err(invalid_function_arguments(span));
            };
            number
                .clone()
                .exponential()
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::HyperbolicSine | Function::HyperbolicCosine | Function::HyperbolicTangent => {
            let [Value::Number(number)] = values else {
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
            let [Value::Number(number)] = values else {
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
        _ => unreachable!("scientific dispatch only passes scientific functions"),
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
