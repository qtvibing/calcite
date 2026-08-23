use crate::evaluator::Value;
use crate::unit::UnitError;
use crate::{
    BinaryOperator, CompoundUnit, Diagnostic, DiagnosticKind, Number, Percentage, Rate, Span,
    UnitValue,
};

use super::{compare_numbers, compare_ordering, numeric_diagnostic, unsupported};

pub(super) fn unit_pair(
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
        BinaryOperator::Multiply => left
            .multiply_unit(right)
            .map(classify_compound_value)
            .map_err(|error| unit_diagnostic(error, span)),
        BinaryOperator::Divide => left
            .divide_unit(right)
            .map(classify_compound_value)
            .map_err(|error| unit_diagnostic(error, span)),
        BinaryOperator::Power => Err(unsupported(span)),
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

pub(super) fn rate_pair(
    operator: BinaryOperator,
    left: Rate,
    right: Rate,
    span: Span,
) -> Result<Value, Diagnostic> {
    if matches!(operator, BinaryOperator::Multiply | BinaryOperator::Divide) {
        let left = rate_as_unit_value(left);
        let right = rate_as_unit_value(right);
        let result = if operator == BinaryOperator::Multiply {
            left.multiply_unit(right)
        } else {
            left.divide_unit(right)
        };
        return result
            .map(classify_compound_value)
            .map_err(|error| unit_diagnostic(error, span));
    }

    let right = right
        .converted_to_compound(left.compound_unit())
        .map_err(|error| unit_diagnostic(error, span))?;
    match operator {
        BinaryOperator::Add => Ok(Value::Rate(Rate::from_compound(
            left.magnitude().clone().add(right.magnitude().clone()),
            left.compound_unit().clone(),
        ))),
        BinaryOperator::Subtract => Ok(Value::Rate(Rate::from_compound(
            left.magnitude().clone().subtract(right.magnitude().clone()),
            left.compound_unit().clone(),
        ))),
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

pub(super) fn rate_number_pair(
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
        BinaryOperator::Power => {
            let exponent = unit_exponent(&right, span)?;
            rate_as_unit_value(left)
                .pow(exponent)
                .map(classify_compound_value)
                .map_err(|error| unit_diagnostic(error, span))
        }
        _ => Err(unsupported(span)),
    }
}

pub(super) fn number_rate_pair(
    operator: BinaryOperator,
    left: Number,
    right: Rate,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Multiply => Ok(Value::Rate(right.scaled(left))),
        BinaryOperator::Divide => {
            let magnitude = left
                .divide(right.magnitude().clone())
                .map_err(|error| numeric_diagnostic(error, span))?;
            let unit = CompoundUnit::default().divide(right.compound_unit());
            Ok(classify_compound_value(UnitValue::from_compound(
                magnitude, unit,
            )))
        }
        _ => Err(unsupported(span)),
    }
}

pub(super) fn rate_unit_pair(
    operator: BinaryOperator,
    rate: Rate,
    unit: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    let rate = rate_as_unit_value(rate);
    let result = match operator {
        BinaryOperator::Multiply => rate.multiply_unit(unit),
        BinaryOperator::Divide => rate.divide_unit(unit),
        _ => return Err(unsupported(span)),
    };
    result
        .map(classify_compound_value)
        .map_err(|error| unit_diagnostic(error, span))
}

pub(super) fn unit_rate_pair(
    operator: BinaryOperator,
    unit: UnitValue,
    rate: Rate,
    span: Span,
) -> Result<Value, Diagnostic> {
    let rate = rate_as_unit_value(rate);
    let result = match operator {
        BinaryOperator::Multiply => unit.multiply_unit(rate),
        BinaryOperator::Divide => unit.divide_unit(rate),
        _ => return Err(unsupported(span)),
    };
    result
        .map(classify_compound_value)
        .map_err(|error| unit_diagnostic(error, span))
}

pub(super) fn rate_as_unit_value(rate: Rate) -> UnitValue {
    let (magnitude, unit) = rate.into_parts();
    UnitValue::from_compound(magnitude, unit)
}

pub(crate) fn classify_compound_value(value: UnitValue) -> Value {
    let (magnitude, unit) = value.into_parts();
    if unit.is_dimensionless() {
        Value::Number(magnitude)
    } else if unit.has_denominator() {
        Value::Rate(Rate::from_compound(magnitude, unit))
    } else {
        Value::Unit(UnitValue::from_compound(magnitude, unit))
    }
}

pub(super) fn unit_percentage_pair(
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

pub(super) fn percentage_unit_pair(
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

pub(super) fn unit_number_pair(
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
        BinaryOperator::Power => {
            let exponent = unit_exponent(&right, span)?;
            left.pow(exponent)
                .map(classify_compound_value)
                .map_err(|error| unit_diagnostic(error, span))
        }
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            let unit = left.compound_unit().clone();
            unit_pair(operator, left, UnitValue::from_compound(right, unit), span)
        }
        _ => Err(unsupported(span)),
    }
}

pub(super) fn unit_exponent(value: &Number, span: Span) -> Result<i32, Diagnostic> {
    value
        .exact_i64()
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| value.unsigned_abs() <= 100)
        .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidUnitExponent, span))
}

pub(super) fn number_unit_pair(
    operator: BinaryOperator,
    left: Number,
    right: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Multiply => Ok(Value::Unit(right.multiply_number(left))),
        BinaryOperator::Divide => {
            let magnitude = left
                .divide(right.magnitude().clone())
                .map_err(|error| numeric_diagnostic(error, span))?;
            let unit = CompoundUnit::default().divide(right.compound_unit());
            Ok(Value::Rate(Rate::from_compound(magnitude, unit)))
        }
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            let unit = right.compound_unit().clone();
            unit_pair(operator, UnitValue::from_compound(left, unit), right, span)
        }
        _ => Err(unsupported(span)),
    }
}

pub(crate) fn unit_diagnostic(error: UnitError, span: Span) -> Diagnostic {
    match error {
        UnitError::IncompatibleDimensions => Diagnostic::new(DiagnosticKind::UnitMismatch, span),
        UnitError::AffineArithmetic => unsupported(span),
        UnitError::InvalidExponent => Diagnostic::new(DiagnosticKind::InvalidUnitExponent, span),
        UnitError::Numeric(error) => numeric_diagnostic(error, span),
    }
}
