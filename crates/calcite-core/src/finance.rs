use std::{cmp::Ordering, fmt};

use crate::evaluator::Value;
use crate::operations::evaluate_binary;
use crate::{
    BinaryOperator, CalculationContext, Diagnostic, DiagnosticKind, Function, Money, Number,
    Percentage, Span,
};

const MAX_AMORTIZATION_PAYMENTS: u32 = 1_200;

#[derive(Clone, Debug, PartialEq)]
pub struct AmortizationPayment {
    period: u32,
    payment: Money,
    principal: Money,
    interest: Money,
    remaining_balance: Money,
}

impl AmortizationPayment {
    pub const fn period(&self) -> u32 {
        self.period
    }

    pub const fn payment(&self) -> &Money {
        &self.payment
    }

    pub const fn principal(&self) -> &Money {
        &self.principal
    }

    pub const fn interest(&self) -> &Money {
        &self.interest
    }

    pub const fn remaining_balance(&self) -> &Money {
        &self.remaining_balance
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AmortizationSchedule {
    payment: Money,
    payments: Vec<AmortizationPayment>,
}

impl AmortizationSchedule {
    pub const fn payment(&self) -> &Money {
        &self.payment
    }

    pub fn payments(&self) -> &[AmortizationPayment] {
        &self.payments
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinanceError {
    InvalidPrincipal,
    InvalidRate,
    InvalidTerm,
    TooManyPayments,
    Numeric,
}

impl fmt::Display for FinanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPrincipal => "principal must be positive",
            Self::InvalidRate => "interest rate must not be less than zero",
            Self::InvalidTerm => "loan term and payment frequency must be positive",
            Self::TooManyPayments => "amortization schedule exceeds the payment limit",
            Self::Numeric => "amortization calculation failed",
        })
    }
}

impl std::error::Error for FinanceError {}

pub fn amortization_schedule(
    principal: Money,
    annual_rate: Percentage,
    years: u32,
    payments_per_year: u32,
) -> Result<AmortizationSchedule, FinanceError> {
    if !principal
        .amount()
        .compare(&Number::from_i64(0))
        .is_some_and(Ordering::is_gt)
    {
        return Err(FinanceError::InvalidPrincipal);
    }
    if annual_rate
        .ratio()
        .compare(&Number::from_i64(0))
        .is_some_and(Ordering::is_lt)
    {
        return Err(FinanceError::InvalidRate);
    }
    let count = years
        .checked_mul(payments_per_year)
        .filter(|count| *count > 0)
        .ok_or(FinanceError::InvalidTerm)?;
    if count > MAX_AMORTIZATION_PAYMENTS {
        return Err(FinanceError::TooManyPayments);
    }

    let count_number = Number::from_i64(i64::from(count));
    let periodic_rate = annual_rate
        .ratio()
        .clone()
        .divide(Number::from_i64(i64::from(payments_per_year)))
        .map_err(|_| FinanceError::Numeric)?;
    let payment_amount = if periodic_rate.is_zero() {
        principal
            .amount()
            .clone()
            .divide(count_number)
            .map_err(|_| FinanceError::Numeric)?
    } else {
        let growth = Number::from_i64(1)
            .add(periodic_rate.clone())
            .pow(Number::from_i64(i64::from(count)))
            .map_err(|_| FinanceError::Numeric)?;
        principal
            .amount()
            .clone()
            .multiply(periodic_rate.clone())
            .multiply(growth.clone())
            .divide(growth.subtract(Number::from_i64(1)))
            .map_err(|_| FinanceError::Numeric)?
    };
    let currency = principal.currency();
    let payment = Money::new(payment_amount.clone(), currency);
    let mut balance = principal.amount().clone();
    let mut payments = Vec::with_capacity(count as usize);
    for period in 1..=count {
        let interest = balance.clone().multiply(periodic_rate.clone());
        let principal_paid = if period == count {
            balance.clone()
        } else {
            payment_amount.clone().subtract(interest.clone())
        };
        let actual_payment = principal_paid.clone().add(interest.clone());
        balance = balance.subtract(principal_paid.clone());
        payments.push(AmortizationPayment {
            period,
            payment: Money::new(actual_payment, currency),
            principal: Money::new(principal_paid, currency),
            interest: Money::new(interest, currency),
            remaining_balance: Money::new(balance.clone(), currency),
        });
    }
    Ok(AmortizationSchedule { payment, payments })
}

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
        [principal, Value::Percentage(rate), periods] => (
            principal,
            rate,
            finance_years(periods).ok_or_else(|| invalid_arguments(span))?,
            Number::from_i64(1),
        ),
        [
            principal,
            Value::Percentage(rate),
            periods,
            Value::Number(compounds),
        ] => (
            principal,
            rate,
            finance_years(periods).ok_or_else(|| invalid_arguments(span))?,
            compounds.clone(),
        ),
        _ => return Err(invalid_arguments(span)),
    };
    ensure_positive(&periods, span)?;
    ensure_positive(&compounds, span)?;
    let periodic_rate = rate
        .ratio()
        .clone()
        .divide(compounds.clone())
        .map_err(|_| invalid_finance_arguments(span))?;
    let exponent = periods.multiply(compounds);
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
        (Function::CompoundAnnualGrowthRate, [initial, final_value, periods]) => (
            initial,
            final_value,
            Some(finance_years(periods).ok_or_else(|| invalid_arguments(span))?),
        ),
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
        ensure_positive(&periods, span)?;
        let inverse = Number::from_i64(1)
            .divide(periods)
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
        [principal, Value::Percentage(rate), years] => (
            principal,
            rate,
            finance_years(years).ok_or_else(|| invalid_arguments(span))?,
            Number::from_i64(12),
        ),
        [
            principal,
            Value::Percentage(rate),
            years,
            Value::Number(frequency),
        ] => (
            principal,
            rate,
            finance_years(years).ok_or_else(|| invalid_arguments(span))?,
            frequency.clone(),
        ),
        _ => return Err(invalid_arguments(span)),
    };
    ensure_positive(&years, span)?;
    ensure_positive(&payments_per_year, span)?;
    let payments = years.multiply(payments_per_year.clone());
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

fn finance_years(value: &Value) -> Option<Number> {
    match value {
        Value::Number(value) => Some(value.clone()),
        Value::CalendarDuration(value) if value.total_months() % 12 == 0 => {
            Some(Number::from_i64(value.total_months() / 12))
        }
        _ => None,
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
