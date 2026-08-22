use std::cmp::Ordering;

use crate::evaluator::{Value, evaluate_binary};
use crate::{
    BinaryOperator, CalculationContext, Diagnostic, DiagnosticKind, Function, Number, Percentage,
    Span,
};

pub(super) fn evaluate_tax_function(
    function: Function,
    values: &[Value],
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    let [amount, Value::Percentage(rate)] = values else {
        return Err(invalid_arguments(span));
    };
    let one = Number::from_i64(1);
    let factor = match function {
        Function::SalesTax => rate.ratio().clone(),
        Function::AddTax => one.add(rate.ratio().clone()),
        Function::RemoveTax => one
            .add(rate.ratio().clone())
            .pow(Number::from_i64(-1))
            .map_err(|_| invalid_finance_arguments(span))?,
        Function::TaxFromTotal => one.clone().subtract(
            one.add(rate.ratio().clone())
                .pow(Number::from_i64(-1))
                .map_err(|_| invalid_finance_arguments(span))?,
        ),
        _ => unreachable!("tax dispatch only passes tax functions"),
    };
    scale_value(amount.clone(), factor, span, context)
}

pub(super) fn evaluate_growth_function(
    function: Function,
    values: &[Value],
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    let (principal, rate, periods, compounds) = match values {
        [principal, Value::Percentage(rate), Value::Number(periods)] => {
            (principal, rate, periods, Number::from_i64(1))
        }
        [
            principal,
            Value::Percentage(rate),
            Value::Number(periods),
            Value::Number(compounds),
        ] => (principal, rate, periods, compounds.clone()),
        _ => return Err(invalid_arguments(span)),
    };
    ensure_positive(periods, span)?;
    ensure_positive(&compounds, span)?;
    let periodic_rate = rate
        .ratio()
        .clone()
        .divide(compounds.clone())
        .map_err(|_| invalid_finance_arguments(span))?;
    let exponent = periods.clone().multiply(compounds);
    let growth = Number::from_i64(1)
        .add(periodic_rate)
        .pow(exponent)
        .map_err(|_| invalid_finance_arguments(span))?;
    let factor = match function {
        Function::PresentValue | Function::PurchasingPower => Number::from_i64(1)
            .divide(growth)
            .map_err(|_| invalid_finance_arguments(span))?,
        _ => growth,
    };
    let result = scale_value(principal.clone(), factor, span, context)?;
    if function == Function::CompoundInterest {
        evaluate_binary(
            BinaryOperator::Subtract,
            result,
            principal.clone(),
            span,
            context,
        )
    } else {
        Ok(result)
    }
}

pub(super) fn evaluate_return_function(
    function: Function,
    values: &[Value],
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    let (initial, final_value, periods) = match (function, values) {
        (Function::ReturnOnInvestment, [initial, final_value]) => (initial, final_value, None),
        (Function::CompoundAnnualGrowthRate, [initial, final_value, Value::Number(periods)]) => {
            (initial, final_value, Some(periods))
        }
        _ => return Err(invalid_arguments(span)),
    };
    let ratio = evaluate_binary(
        BinaryOperator::Divide,
        final_value.clone(),
        initial.clone(),
        span,
        context,
    )?;
    let Value::Number(ratio) = ratio else {
        return Err(invalid_arguments(span));
    };
    let result = if let Some(periods) = periods {
        ensure_positive(periods, span)?;
        let inverse = Number::from_i64(1)
            .divide(periods.clone())
            .map_err(|_| invalid_finance_arguments(span))?;
        ratio
            .pow(inverse)
            .map_err(|_| invalid_finance_arguments(span))?
            .subtract(Number::from_i64(1))
    } else {
        ratio.subtract(Number::from_i64(1))
    };
    Ok(Value::Percentage(Percentage::from_ratio(result)))
}

pub(super) fn evaluate_loan_function(
    function: Function,
    values: &[Value],
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    let (principal, rate, years, payments_per_year) = match values {
        [principal, Value::Percentage(rate), Value::Number(years)] => {
            (principal, rate, years, Number::from_i64(12))
        }
        [
            principal,
            Value::Percentage(rate),
            Value::Number(years),
            Value::Number(frequency),
        ] => (principal, rate, years, frequency.clone()),
        _ => return Err(invalid_arguments(span)),
    };
    ensure_positive(years, span)?;
    ensure_positive(&payments_per_year, span)?;
    let payments = years.clone().multiply(payments_per_year.clone());
    let periodic_rate = rate
        .ratio()
        .clone()
        .divide(payments_per_year)
        .map_err(|_| invalid_finance_arguments(span))?;
    let payment_factor = if periodic_rate.is_zero() {
        Number::from_i64(1)
            .divide(payments.clone())
            .map_err(|_| invalid_finance_arguments(span))?
    } else {
        let growth = Number::from_i64(1)
            .add(periodic_rate.clone())
            .pow(payments.clone())
            .map_err(|_| invalid_finance_arguments(span))?;
        periodic_rate
            .multiply(growth.clone())
            .divide(growth.subtract(Number::from_i64(1)))
            .map_err(|_| invalid_finance_arguments(span))?
    };
    let payment = scale_value(principal.clone(), payment_factor, span, context)?;
    if function == Function::MortgagePayment {
        return Ok(payment);
    }
    let total = scale_value(payment, payments, span, context)?;
    if function == Function::TotalInterest {
        evaluate_binary(
            BinaryOperator::Subtract,
            total,
            principal.clone(),
            span,
            context,
        )
    } else {
        Ok(total)
    }
}

fn scale_value(
    value: Value,
    factor: Number,
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    evaluate_binary(
        BinaryOperator::Multiply,
        value,
        Value::Number(factor),
        span,
        context,
    )
}

fn ensure_positive(value: &Number, span: Span) -> Result<(), Diagnostic> {
    if value
        .compare(&Number::from_i64(0))
        .is_some_and(Ordering::is_gt)
    {
        Ok(())
    } else {
        Err(invalid_finance_arguments(span))
    }
}

fn invalid_arguments(span: Span) -> Diagnostic {
    Diagnostic::new(DiagnosticKind::InvalidFunctionArguments, span)
}

fn invalid_finance_arguments(span: Span) -> Diagnostic {
    Diagnostic::new(DiagnosticKind::InvalidFinanceArguments, span)
}
