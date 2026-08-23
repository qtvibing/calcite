use std::cmp::Ordering;

use crate::evaluator::Value;
use crate::{
    BinaryOperator, Diagnostic, DiagnosticKind, Fraction, Number, NumericError, Percentage, Span,
};

use super::unsupported;

pub(crate) fn number_pair(
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

pub(crate) fn percentage_pair(
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

pub(crate) fn number_percentage_pair(
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

pub(crate) fn percentage_number_pair(
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

pub(crate) fn fraction_pair(
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

pub(crate) fn compare_numbers(
    operator: BinaryOperator,
    left: &Number,
    right: &Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    let ordering = left.compare(right).ok_or_else(|| unsupported(span))?;
    compare_ordering(operator, ordering, span)
}

pub(crate) fn compare_ordering(
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

pub(crate) fn boolean_pair(
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

pub(crate) fn numeric_diagnostic(error: NumericError, span: Span) -> Diagnostic {
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
