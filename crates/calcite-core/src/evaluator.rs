use std::{cmp::Ordering, fmt};

use crate::unit::UnitError;
use crate::{
    BinaryOperator, CalculationContext, ConversionTarget, Diagnostic, DiagnosticKind, Expression,
    Fraction, Number, NumericError, Percentage, Span, UnaryOperator, UnitValue, parse_with_context,
};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    Number(Number),
    Percentage(Percentage),
    Fraction(Fraction),
    Unit(UnitValue),
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
            Self::Unit(value) => write!(formatter, "{} {}", value.magnitude(), value.unit()),
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
    evaluate_expression(&expression)
}

fn evaluate_expression(expression: &Expression) -> Result<Value, Diagnostic> {
    match expression {
        Expression::Number { value, .. } => Ok(Value::Number(value.clone())),
        Expression::Boolean { value, .. } => Ok(Value::Boolean(*value)),
        Expression::Unit { value, unit, .. } => {
            Ok(Value::Unit(UnitValue::new(value.clone(), *unit)))
        }
        Expression::Unary {
            operator,
            operator_span,
            operand,
            ..
        } => {
            let operand = evaluate_expression(operand)?;
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
                (UnaryOperator::Not, Value::Boolean(value)) => Ok(Value::Boolean(!value)),
                (UnaryOperator::Positive | UnaryOperator::Negative, Value::Boolean(_))
                | (
                    UnaryOperator::Not,
                    Value::Number(_) | Value::Percentage(_) | Value::Fraction(_) | Value::Unit(_),
                ) => Err(unsupported(*operator_span)),
            }
        }
        Expression::Group { expression, .. } => evaluate_expression(expression),
        Expression::Percentage {
            expression,
            operator_span,
            ..
        } => match evaluate_expression(expression)? {
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
        } => convert(evaluate_expression(expression)?, *target, *target_span),
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            inverted,
            keyword_span,
            ..
        } => {
            let Value::Boolean(mut condition) = evaluate_expression(condition)? else {
                return Err(unsupported(*keyword_span));
            };
            if *inverted {
                condition = !condition;
            }
            if condition {
                evaluate_expression(then_branch)
            } else {
                evaluate_expression(else_branch)
            }
        }
        Expression::Binary {
            operator,
            operator_span,
            left,
            right,
            ..
        } => {
            let left = evaluate_expression(left)?;
            if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
                let Value::Boolean(left) = left else {
                    return Err(unsupported(*operator_span));
                };
                if (*operator == BinaryOperator::And && !left)
                    || (*operator == BinaryOperator::Or && left)
                {
                    return Ok(Value::Boolean(left));
                }
                return match evaluate_expression(right)? {
                    Value::Boolean(right) => Ok(Value::Boolean(right)),
                    _ => Err(unsupported(*operator_span)),
                };
            }

            let right = evaluate_expression(right)?;
            evaluate_binary(*operator, left, right, *operator_span)
        }
    }
}

fn evaluate_binary(
    operator: BinaryOperator,
    left: Value,
    right: Value,
    span: Span,
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
        BinaryOperator::Of
        | BinaryOperator::On
        | BinaryOperator::Off
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
        BinaryOperator::Of
        | BinaryOperator::On
        | BinaryOperator::Off
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
        BinaryOperator::And | BinaryOperator::Or => Err(unsupported(span)),
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

fn convert(value: Value, target: ConversionTarget, span: Span) -> Result<Value, Diagnostic> {
    match target {
        ConversionTarget::Number => match value {
            Value::Number(number) => Ok(Value::Number(number)),
            Value::Percentage(percentage) => Ok(Value::Number(percentage.into_ratio())),
            Value::Fraction(fraction) => Ok(Value::Number(fraction.into_number())),
            Value::Unit(_) | Value::Boolean(_) => Err(unsupported(span)),
        },
        ConversionTarget::Percentage => match value {
            Value::Number(number) => Ok(Value::Percentage(Percentage::from_ratio(number))),
            Value::Percentage(percentage) => Ok(Value::Percentage(percentage)),
            Value::Fraction(fraction) => Ok(Value::Percentage(Percentage::from_ratio(
                fraction.into_number(),
            ))),
            Value::Unit(_) | Value::Boolean(_) => Err(unsupported(span)),
        },
        ConversionTarget::Fraction => {
            let number = match value {
                Value::Number(number) => number,
                Value::Percentage(percentage) => percentage.into_ratio(),
                Value::Fraction(fraction) => return Ok(Value::Fraction(fraction)),
                Value::Unit(_) | Value::Boolean(_) => return Err(unsupported(span)),
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
        NumericError::NonRealResult => DiagnosticKind::NonRealResult,
    };
    Diagnostic::new(kind, span)
}

fn unsupported(span: Span) -> Diagnostic {
    Diagnostic::new(DiagnosticKind::UnsupportedOperation, span)
}
