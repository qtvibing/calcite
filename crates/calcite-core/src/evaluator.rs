use std::fmt;

use crate::{
    BinaryOperator, Diagnostic, DiagnosticKind, Expression, Number, NumericError, UnaryOperator,
    parse,
};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    Number(Number),
}

impl Value {
    pub const fn as_number(&self) -> Option<&Number> {
        match self {
            Self::Number(number) => Some(number),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => number.fmt(formatter),
        }
    }
}

pub fn evaluate(source: &str) -> Result<Value, Diagnostic> {
    let expression = parse(source)?;
    evaluate_expression(&expression).map(Value::Number)
}

fn evaluate_expression(expression: &Expression) -> Result<Number, Diagnostic> {
    match expression {
        Expression::Number { value, .. } => Ok(value.clone()),
        Expression::Unary {
            operator, operand, ..
        } => {
            let operand = evaluate_expression(operand)?;
            Ok(match operator {
                UnaryOperator::Positive => operand,
                UnaryOperator::Negative => operand.negated(),
            })
        }
        Expression::Group { expression, .. } => evaluate_expression(expression),
        Expression::Binary {
            operator,
            operator_span,
            left,
            right,
            ..
        } => {
            let left = evaluate_expression(left)?;
            let right = evaluate_expression(right)?;
            match operator {
                BinaryOperator::Add => Ok(left.add(right)),
                BinaryOperator::Subtract => Ok(left.subtract(right)),
                BinaryOperator::Multiply => Ok(left.multiply(right)),
                BinaryOperator::Divide => left
                    .divide(right)
                    .map_err(|error| numeric_diagnostic(error, *operator_span)),
                BinaryOperator::Power => left
                    .pow(right)
                    .map_err(|error| numeric_diagnostic(error, *operator_span)),
            }
        }
    }
}

fn numeric_diagnostic(error: NumericError, span: crate::Span) -> Diagnostic {
    let kind = match error {
        NumericError::DivisionByZero => DiagnosticKind::DivisionByZero,
        NumericError::ExponentTooLarge => DiagnosticKind::ExponentTooLarge,
        NumericError::NonRealResult => DiagnosticKind::NonRealResult,
    };
    Diagnostic::new(kind, span)
}
