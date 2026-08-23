use crate::evaluator::Value;
use crate::operations::numeric_diagnostic;
use crate::{Diagnostic, DiagnosticKind, Function, RadixInteger, Span};

use super::invalid_function_arguments;

pub(super) fn evaluate(
    function: Function,
    values: &[Value],
    span: Span,
) -> Result<Value, Diagnostic> {
    match function {
        Function::Binary | Function::Octal | Function::Hexadecimal => {
            let [Value::Number(number)] = values else {
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
            let [Value::Number(number)] = values else {
                return Err(invalid_function_arguments(span));
            };
            if number.is_integer() {
                Ok(Value::Number(number.clone()))
            } else {
                Err(Diagnostic::new(DiagnosticKind::IntegerRequired, span))
            }
        }
        Function::GreatestCommonDivisor | Function::LeastCommonMultiple => {
            let [Value::Number(first), Value::Number(second), rest @ ..] = values else {
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
            let [Value::Number(number)] = values else {
                return Err(invalid_function_arguments(span));
            };
            number
                .is_prime()
                .map(Value::Boolean)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::Factorial => {
            let [Value::Number(number)] = values else {
                return Err(invalid_function_arguments(span));
            };
            number
                .clone()
                .factorial()
                .map(Value::Number)
                .map_err(|error| numeric_diagnostic(error, span))
        }
        Function::Permutation | Function::Combination => {
            let [Value::Number(population), Value::Number(selection)] = values else {
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
        _ => unreachable!("integer dispatch only passes integer functions"),
    }
}
