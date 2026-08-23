use crate::evaluator::Value;
use crate::{
    CalculationContext, ConversionTarget, Diagnostic, DiagnosticKind, Fraction, Money, Number,
    Percentage, RadixInteger, RomanInteger, Span, Unit, UnitValue,
};

use super::{classify_compound_value, time_zone_diagnostic, unit_diagnostic, unsupported};

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
            Value::RomanInteger(value) => Ok(Value::Number(value.into_number())),
            Value::Percentage(percentage) => Ok(Value::Number(percentage.into_ratio())),
            Value::Fraction(fraction) => Ok(Value::Number(fraction.into_number())),
            Value::Unit(value) => Ok(Value::Number(value.magnitude().clone())),
            Value::Money(value) => Ok(Value::Number(value.amount().clone())),
            Value::MoneyRate(value) => Ok(Value::Number(value.amount().clone())),
            Value::Rate(value) => Ok(Value::Number(value.magnitude().clone())),
            Value::String(_)
            | Value::Function(_)
            | Value::Complex(_)
            | Value::Dice(_)
            | Value::Date(_)
            | Value::Clock(_)
            | Value::OffsetClock(_)
            | Value::ZonedClock(_)
            | Value::OffsetDateTime(_)
            | Value::ZonedDateTime(_)
            | Value::TimeSpan(_)
            | Value::CalendarDuration(_)
            | Value::CalendarSpan(_)
            | Value::WorkdayDuration(_)
            | Value::Weekday(_)
            | Value::Boolean(_) => Err(unsupported(span)),
        },
        ConversionTarget::Radix(radix) => {
            let number = match value {
                Value::Number(number) => number,
                Value::RadixInteger(value) => value.into_number(),
                Value::RomanInteger(value) => value.into_number(),
                _ => return Err(unsupported(span)),
            };
            RadixInteger::new(number, radix)
                .map(Value::RadixInteger)
                .map_err(|_| Diagnostic::new(DiagnosticKind::IntegerRequired, span))
        }
        ConversionTarget::Roman => {
            let number = match value {
                Value::Number(number) => number,
                Value::RadixInteger(value) => value.into_number(),
                Value::RomanInteger(value) => return Ok(Value::RomanInteger(value)),
                _ => return Err(unsupported(span)),
            };
            RomanInteger::new(number)
                .map(Value::RomanInteger)
                .map_err(|_| Diagnostic::new(DiagnosticKind::IntegerRequired, span))
        }
        ConversionTarget::Percentage => match value {
            Value::Number(number) => Ok(Value::Percentage(Percentage::from_ratio(number))),
            Value::RadixInteger(value) => Ok(Value::Percentage(Percentage::from_ratio(
                value.into_number(),
            ))),
            Value::RomanInteger(value) => Ok(Value::Percentage(Percentage::from_ratio(
                value.into_number(),
            ))),
            Value::Percentage(percentage) => Ok(Value::Percentage(percentage)),
            Value::Fraction(fraction) => Ok(Value::Percentage(Percentage::from_ratio(
                fraction.into_number(),
            ))),
            Value::String(_)
            | Value::Function(_)
            | Value::Complex(_)
            | Value::Unit(_)
            | Value::Money(_)
            | Value::MoneyRate(_)
            | Value::Rate(_)
            | Value::Dice(_)
            | Value::Date(_)
            | Value::Clock(_)
            | Value::OffsetClock(_)
            | Value::ZonedClock(_)
            | Value::OffsetDateTime(_)
            | Value::ZonedDateTime(_)
            | Value::TimeSpan(_)
            | Value::CalendarDuration(_)
            | Value::CalendarSpan(_)
            | Value::WorkdayDuration(_)
            | Value::Weekday(_)
            | Value::Boolean(_) => Err(unsupported(span)),
        },
        ConversionTarget::Fraction => {
            let number = match value {
                Value::Number(number) => number,
                Value::RadixInteger(value) => value.into_number(),
                Value::RomanInteger(value) => value.into_number(),
                Value::Percentage(percentage) => percentage.into_ratio(),
                Value::Fraction(fraction) => return Ok(Value::Fraction(fraction)),
                Value::String(_)
                | Value::Function(_)
                | Value::Complex(_)
                | Value::Unit(_)
                | Value::Money(_)
                | Value::MoneyRate(_)
                | Value::Rate(_)
                | Value::Dice(_)
                | Value::Date(_)
                | Value::Clock(_)
                | Value::OffsetClock(_)
                | Value::ZonedClock(_)
                | Value::OffsetDateTime(_)
                | Value::ZonedDateTime(_)
                | Value::TimeSpan(_)
                | Value::CalendarDuration(_)
                | Value::CalendarSpan(_)
                | Value::WorkdayDuration(_)
                | Value::Weekday(_)
                | Value::Boolean(_) => return Err(unsupported(span)),
            };
            Fraction::from_number(number)
                .map(Value::Fraction)
                .ok_or_else(|| unsupported(span))
        }
        ConversionTarget::CompoundUnit(target) => match value {
            Value::Number(number) => UnitValue::from_compound(number, Default::default())
                .converted_to_compound(&target)
                .map(classify_compound_value)
                .map_err(|error| unit_diagnostic(error, span)),
            Value::Unit(value) => value
                .converted_to_compound(&target)
                .map(classify_compound_value)
                .map_err(|error| unit_diagnostic(error, span)),
            Value::Rate(rate) => rate
                .converted_to_compound(&target)
                .map(Value::Rate)
                .map_err(|error| unit_diagnostic(error, span)),
            Value::MoneyRate(rate) => rate
                .converted_denominator(&target)
                .map(Value::MoneyRate)
                .map_err(|error| unit_diagnostic(error, span)),
            Value::TimeSpan(duration) => {
                let seconds = Number::parse_decimal(&duration.nanoseconds().to_string())
                    .expect("an i128 is a valid bounded decimal")
                    .divide(Number::from_i64(1_000_000_000))
                    .expect("one billion is nonzero");
                UnitValue::new(seconds, Unit::Second)
                    .converted_to_compound(&target)
                    .map(classify_compound_value)
                    .map_err(|error| unit_diagnostic(error, span))
            }
            _ => Err(unsupported(span)),
        },
        ConversionTarget::UtcOffset(offset) => match value {
            Value::OffsetClock(value) => Ok(Value::OffsetClock(value.converted_to(offset))),
            Value::OffsetDateTime(value) => value
                .converted_to(offset)
                .map(Value::OffsetDateTime)
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span)),
            _ => Err(unsupported(span)),
        },
        ConversionTarget::TimeZone(target) => {
            let snapshot = context
                .temporal
                .time_zones
                .as_ref()
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::MissingTimeZoneData, span))?;
            match value {
                Value::OffsetDateTime(value) => snapshot
                    .at_instant(&target, value)
                    .map(Value::ZonedDateTime)
                    .map_err(|error| time_zone_diagnostic(error, span)),
                Value::ZonedDateTime(value) => value
                    .converted_to(snapshot, &target)
                    .map(Value::ZonedDateTime)
                    .map_err(|error| time_zone_diagnostic(error, span)),
                _ => Err(unsupported(span)),
            }
        }
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
            Value::MoneyRate(money_rate) => {
                let source = money_rate.currency();
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
                let amount = money_rate.amount().clone().multiply(rate);
                Ok(Value::MoneyRate(money_rate.with_currency(amount, target)))
            }
            _ => Err(unsupported(span)),
        },
    }
}
