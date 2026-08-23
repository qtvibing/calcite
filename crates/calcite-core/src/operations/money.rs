use crate::evaluator::Value;
use crate::{
    BinaryOperator, Diagnostic, DiagnosticKind, Money, MoneyRate, Number, Percentage, Span,
    UnitValue,
};

use super::{compare_numbers, numeric_diagnostic, unit_diagnostic, unsupported};

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

pub(super) fn money_unit_pair(
    operator: BinaryOperator,
    money: Money,
    unit: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    if operator != BinaryOperator::Divide {
        return Err(unsupported(span));
    }
    let (magnitude, denominator) = unit.into_parts();
    money
        .amount()
        .clone()
        .divide(magnitude)
        .map(|amount| Value::MoneyRate(MoneyRate::new(amount, money.currency(), denominator)))
        .map_err(|error| numeric_diagnostic(error, span))
}

pub(super) fn money_rate_pair(
    operator: BinaryOperator,
    left: MoneyRate,
    right: MoneyRate,
    span: Span,
) -> Result<Value, Diagnostic> {
    if left.currency() != right.currency() {
        return Err(Diagnostic::new(DiagnosticKind::CurrencyMismatch, span));
    }
    let right = right
        .converted_denominator(left.denominator())
        .map_err(|error| unit_diagnostic(error, span))?;
    let amount = match operator {
        BinaryOperator::Add => left.amount().clone().add(right.amount().clone()),
        BinaryOperator::Subtract => left.amount().clone().subtract(right.amount().clone()),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => {
            return compare_numbers(operator, left.amount(), right.amount(), span);
        }
        _ => return Err(unsupported(span)),
    };
    Ok(Value::MoneyRate(MoneyRate::new(
        amount,
        left.currency(),
        left.denominator().clone(),
    )))
}

pub(super) fn money_rate_unit_pair(
    operator: BinaryOperator,
    rate: MoneyRate,
    quantity: UnitValue,
    span: Span,
) -> Result<Value, Diagnostic> {
    if operator != BinaryOperator::Multiply {
        return Err(unsupported(span));
    }
    rate.apply_to(quantity)
        .map(Value::Money)
        .map_err(|error| unit_diagnostic(error, span))
}

pub(super) fn money_money_rate_pair(
    operator: BinaryOperator,
    money: Money,
    rate: MoneyRate,
    span: Span,
) -> Result<Value, Diagnostic> {
    if !matches!(operator, BinaryOperator::Divide | BinaryOperator::At) {
        return Err(unsupported(span));
    }
    if money.currency() != rate.currency() {
        return Err(Diagnostic::new(DiagnosticKind::CurrencyMismatch, span));
    }
    rate.solve_quantity(money)
        .map(Value::Unit)
        .map_err(|error| unit_diagnostic(error, span))
}

pub(super) fn money_rate_number_pair(
    operator: BinaryOperator,
    rate: MoneyRate,
    number: Number,
    span: Span,
) -> Result<Value, Diagnostic> {
    match operator {
        BinaryOperator::Multiply => Ok(Value::MoneyRate(rate.scaled(number))),
        BinaryOperator::Divide => rate
            .amount()
            .clone()
            .divide(number)
            .map(|amount| {
                Value::MoneyRate(MoneyRate::new(
                    amount,
                    rate.currency(),
                    rate.denominator().clone(),
                ))
            })
            .map_err(|error| numeric_diagnostic(error, span)),
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
