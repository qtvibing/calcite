use crate::evaluator::Value;
use crate::{BinaryOperator, Diagnostic, DiagnosticKind, Money, Number, Percentage, Span};

use super::{compare_numbers, numeric_diagnostic, unsupported};

pub(super) fn money_pair(
    operator: BinaryOperator,
    left: Money,
    right: Money,
    span: Span,
) -> Result<Value, Diagnostic> {
    if left.currency() != right.currency() {
        return Err(Diagnostic::new(DiagnosticKind::CurrencyMismatch, span));
    }
    let currency = left.currency();
    match operator {
        BinaryOperator::Add => Ok(Value::Money(Money::new(
            left.amount().clone().add(right.amount().clone()),
            currency,
        ))),
        BinaryOperator::Subtract => Ok(Value::Money(Money::new(
            left.amount().clone().subtract(right.amount().clone()),
            currency,
        ))),
        BinaryOperator::Divide => left
            .amount()
            .clone()
            .divide(right.amount().clone())
            .map(Value::Number)
            .map_err(|error| numeric_diagnostic(error, span)),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            compare_numbers(operator, left.amount(), right.amount(), span)
        }
        _ => Err(unsupported(span)),
    }
}

pub(super) fn money_number_pair(
    operator: BinaryOperator,
    money: Money,
    number: Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    let currency = money.currency();
    match operator {
        BinaryOperator::Add => Ok(Value::Money(Money::new(
            money.amount().clone().add(number),
            currency,
        ))),
        BinaryOperator::Subtract => Ok(Value::Money(Money::new(
            money.amount().clone().subtract(number),
            currency,
        ))),
        BinaryOperator::Multiply => Ok(Value::Money(Money::new(
            money.amount().clone().multiply(number),
            currency,
        ))),
        BinaryOperator::Divide => money
            .amount()
            .clone()
            .divide(number)
            .map(|amount| Value::Money(Money::new(amount, currency)))
            .map_err(|error| numeric_diagnostic(error, span)),
        _ => Err(unsupported(span)),
    }
}

pub(super) fn number_money_pair(
    operator: BinaryOperator,
    number: Number,
    money: Money,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Add => money_number_pair(operator, money, number, span),
        BinaryOperator::Subtract => {
            let currency = money.currency();
            Ok(Value::Money(Money::new(
                number.subtract(money.amount().clone()),
                currency,
            )))
        }
        BinaryOperator::Multiply => money_number_pair(operator, money, number, span),
        _ => Err(unsupported(span)),
    }
}

pub(super) fn money_percentage_pair(
    operator: BinaryOperator,
    money: Money,
    percentage: Percentage,
    span: Span,
) -> Result<Value, Diagnostic> {
    let ratio = percentage.into_ratio();
    let currency = money.currency();
    let amount = money.amount().clone();
    match operator {
        BinaryOperator::Add | BinaryOperator::On => Ok(Value::Money(Money::new(
            amount.clone().add(amount.multiply(ratio)),
            currency,
        ))),
        BinaryOperator::Subtract | BinaryOperator::Off => Ok(Value::Money(Money::new(
            amount.clone().subtract(amount.multiply(ratio)),
            currency,
        ))),
        BinaryOperator::Multiply => Ok(Value::Money(Money::new(amount.multiply(ratio), currency))),
        BinaryOperator::Divide => amount
            .divide(ratio)
            .map(|amount| Value::Money(Money::new(amount, currency)))
            .map_err(|error| numeric_diagnostic(error, span)),
        _ => Err(unsupported(span)),
    }
}

pub(super) fn percentage_money_pair(
    operator: BinaryOperator,
    percentage: Percentage,
    money: Money,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Multiply | BinaryOperator::Of => {
            money_percentage_pair(BinaryOperator::Multiply, money, percentage, span)
        }
        _ => Err(unsupported(span)),
    }
}
