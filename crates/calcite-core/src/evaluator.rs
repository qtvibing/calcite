use std::{cmp::Ordering, fmt};

use crate::{
    BinaryOperator, ConversionTarget, Diagnostic, DiagnosticKind, Expression, Fraction, Number,
    NumericError, Percentage, Span, UnaryOperator, parse,
};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    Number(Number),
    Percentage(Percentage),
    Fraction(Fraction),
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
            Self::Boolean(value) => value.fmt(formatter),
        }
    }
}

pub fn evaluate(source: &str) -> Result<Value, Diagnostic> {
    let expression = parse(source)?;
    evaluate_expression(&expression)
}

fn evaluate_expression(expression: &Expression) -> Result<Value, Diagnostic> {
    match expression {
        Expression::Number { value, .. } => Ok(Value::Number(value.clone())),
        Expression::Boolean { value, .. } => Ok(Value::Boolean(*value)),
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
                (UnaryOperator::Not, Value::Boolean(value)) => Ok(Value::Boolean(!value)),
                (UnaryOperator::Positive | UnaryOperator::Negative, Value::Boolean(_))
                | (
                    UnaryOperator::Not,
                    Value::Number(_) | Value::Percentage(_) | Value::Fraction(_),
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
            Value::Boolean(_) => Err(unsupported(span)),
        },
        ConversionTarget::Percentage => match value {
            Value::Number(number) => Ok(Value::Percentage(Percentage::from_ratio(number))),
            Value::Percentage(percentage) => Ok(Value::Percentage(percentage)),
            Value::Fraction(fraction) => Ok(Value::Percentage(Percentage::from_ratio(
                fraction.into_number(),
            ))),
            Value::Boolean(_) => Err(unsupported(span)),
        },
        ConversionTarget::Fraction => {
            let number = match value {
                Value::Number(number) => number,
                Value::Percentage(percentage) => percentage.into_ratio(),
                Value::Fraction(fraction) => return Ok(Value::Fraction(fraction)),
                Value::Boolean(_) => return Err(unsupported(span)),
            };
            Fraction::from_number(number)
                .map(Value::Fraction)
                .ok_or_else(|| unsupported(span))
        }
    }
}

fn compare_numbers(
    operator: BinaryOperator,
    left: &Number,
    right: &Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    let ordering = left.compare(right).ok_or_else(|| unsupported(span))?;
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
