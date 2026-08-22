use std::{collections::HashMap, error::Error, fmt};

use crate::{Currency, Number};

#[derive(Clone, Debug, PartialEq)]
pub struct Money {
    amount: Number,
    currency: Currency,
}

impl Money {
    pub const fn new(amount: Number, currency: Currency) -> Self {
        Self { amount, currency }
    }

    pub const fn amount(&self) -> &Number {
        &self.amount
    }

    pub const fn currency(&self) -> Currency {
        self.currency
    }

    pub(crate) fn negated(self) -> Self {
        Self::new(self.amount.negated(), self.currency)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.amount, self.currency)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExchangeRates {
    rates: HashMap<(Currency, Currency), Number>,
}

impl ExchangeRates {
    pub fn set_rate(
        &mut self,
        source: Currency,
        target: Currency,
        rate: Number,
    ) -> Result<(), InvalidExchangeRate> {
        if source == target
            || rate
                .compare(&Number::from_i64(0))
                .is_none_or(|ordering| !ordering.is_gt())
        {
            return Err(InvalidExchangeRate);
        }
        self.rates.insert((source, target), rate);
        Ok(())
    }

    pub fn rate(&self, source: Currency, target: Currency) -> Option<Number> {
        if source == target {
            return Some(Number::from_i64(1));
        }
        if let Some(rate) = self.rates.get(&(source, target)) {
            return Some(rate.clone());
        }
        Number::from_i64(1)
            .divide(self.rates.get(&(target, source))?.clone())
            .ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidExchangeRate;

impl fmt::Display for InvalidExchangeRate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("exchange rates must be positive and use different currencies")
    }
}

impl Error for InvalidExchangeRate {}

#[derive(Clone, Debug, PartialEq)]
pub struct MonetaryContext {
    pub default_currency: Currency,
    pub exchange_rates: ExchangeRates,
}

impl Default for MonetaryContext {
    fn default() -> Self {
        Self {
            default_currency: Currency::Usd,
            exchange_rates: ExchangeRates::default(),
        }
    }
}
