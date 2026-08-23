use crate::evaluator::Value;
use crate::{
    CalculationContext, ConversionTarget, Diagnostic, DiagnosticKind, Fraction, Money, Percentage,
    RadixInteger, Span,
};

use super::{classify_compound_value, unit_diagnostic, unsupported};

pub(crate) fn convert(
    value: Value,
    target: ConversionTarget,
    span: Span,
    context: &CalculationContext,
) -> Result<Value, Diagnostic> {
    match target {
        ConversionTarget::Number => match value {
            Value::Number(number) => Ok(Value::Number(number)),
            Value::RadixInteger(value) => Ok(Value::Number(value.into_number())),
            Value::Percentage(percentage) => Ok(Value::Number(percentage.into_ratio())),
            Value::Fraction(fraction) => Ok(Value::Number(fraction.into_number())),
            Value::Unit(value) => Ok(Value::Number(value.magnitude().clone())),
            Value::Money(value) => Ok(Value::Number(value.amount().clone())),
            Value::Rate(value) => Ok(Value::Number(value.magnitude().clone())),
            Value::Date(_)
            | Value::Clock(_)
            | Value::OffsetDateTime(_)
            | Value::CalendarDuration(_)
            | Value::WorkdayDuration(_)
            | Value::Weekday(_)
            | Value::Boolean(_) => Err(unsupported(span)),
        },
        ConversionTarget::Radix(radix) => {
            let number = match value {
                Value::Number(number) => number,
                Value::RadixInteger(value) => value.into_number(),
                _ => return Err(unsupported(span)),
            };
            RadixInteger::new(number, radix)
                .map(Value::RadixInteger)
                .map_err(|_| Diagnostic::new(DiagnosticKind::IntegerRequired, span))
        }
        ConversionTarget::Percentage => match value {
            Value::Number(number) => Ok(Value::Percentage(Percentage::from_ratio(number))),
            Value::RadixInteger(value) => Ok(Value::Percentage(Percentage::from_ratio(
                value.into_number(),
            ))),
            Value::Percentage(percentage) => Ok(Value::Percentage(percentage)),
            Value::Fraction(fraction) => Ok(Value::Percentage(Percentage::from_ratio(
                fraction.into_number(),
            ))),
            Value::Unit(_)
            | Value::Money(_)
            | Value::Rate(_)
            | Value::Date(_)
            | Value::Clock(_)
            | Value::OffsetDateTime(_)
            | Value::CalendarDuration(_)
            | Value::WorkdayDuration(_)
            | Value::Weekday(_)
            | Value::Boolean(_) => Err(unsupported(span)),
        },
        ConversionTarget::Fraction => {
            let number = match value {
                Value::Number(number) => number,
                Value::RadixInteger(value) => value.into_number(),
                Value::Percentage(percentage) => percentage.into_ratio(),
                Value::Fraction(fraction) => return Ok(Value::Fraction(fraction)),
                Value::Unit(_)
                | Value::Money(_)
                | Value::Rate(_)
                | Value::Date(_)
                | Value::Clock(_)
                | Value::OffsetDateTime(_)
                | Value::CalendarDuration(_)
                | Value::WorkdayDuration(_)
                | Value::Weekday(_)
                | Value::Boolean(_) => return Err(unsupported(span)),
            };
            Fraction::from_number(number)
                .map(Value::Fraction)
                .ok_or_else(|| unsupported(span))
        }
        ConversionTarget::CompoundUnit(target) => match value {
            Value::Unit(value) => value
                .converted_to_compound(&target)
                .map(classify_compound_value)
                .map_err(|error| unit_diagnostic(error, span)),
            Value::Rate(rate) => rate
                .converted_to_compound(&target)
                .map(Value::Rate)
                .map_err(|error| unit_diagnostic(error, span)),
            _ => Err(unsupported(span)),
        },
        ConversionTarget::UtcOffset(offset) => match value {
            Value::OffsetDateTime(value) => value
                .converted_to(offset)
                .map(Value::OffsetDateTime)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span)),
            _ => Err(unsupported(span)),
        },
        ConversionTarget::Currency(target) => match value {
            Value::Money(money) => {
                let source = money.currency();
                let rate = context
                    .monetary
                    .exchange_rates
                    .rate(source, target, context.monetary.exchange_rate_freshness)
                    .map_err(|error| {
                        let kind = match error {
                            crate::ExchangeRateLookupError::Missing => {
                                DiagnosticKind::MissingExchangeRate
                            }
                            crate::ExchangeRateLookupError::Stale => {
                                DiagnosticKind::StaleExchangeRate
                            }
                        };
                        Diagnostic::with_detail(kind, span, format!("{source} to {target}"))
                    })?;
                Ok(Value::Money(Money::new(
                    money.amount().clone().multiply(rate),
                    target,
                )))
            }
            _ => Err(unsupported(span)),
        },
    }
}
