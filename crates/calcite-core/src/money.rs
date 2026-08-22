use std::{collections::HashMap, error::Error, fmt};

use crate::{Currency, Number, OffsetDateTime};

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
pub struct ExchangeRateSnapshot {
    as_of: Option<OffsetDateTime>,
    rates: HashMap<(Currency, Currency), Number>,
}

impl ExchangeRateSnapshot {
    pub fn new<I>(as_of: Option<OffsetDateTime>, rates: I) -> Result<Self, InvalidExchangeRate>
    where
        I: IntoIterator<Item = (Currency, Currency, Number)>,
    {
        let mut validated = HashMap::new();
        for (source, target, rate) in rates {
            if source == target
                || rate
                    .compare(&Number::from_i64(0))
                    .is_none_or(|ordering| !ordering.is_gt())
            {
                return Err(InvalidExchangeRate);
            }
            validated.insert((source, target), rate);
        }
        Ok(Self {
            as_of,
            rates: validated,
        })
    }

    pub const fn as_of(&self) -> Option<OffsetDateTime> {
        self.as_of
    }

    pub fn len(&self) -> usize {
        self.rates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rates.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExchangeRates {
    snapshot: ExchangeRateSnapshot,
}

impl ExchangeRates {
    pub fn reload(&mut self, snapshot: ExchangeRateSnapshot) -> ExchangeRateSnapshot {
        std::mem::replace(&mut self.snapshot, snapshot)
    }

    pub const fn snapshot(&self) -> &ExchangeRateSnapshot {
        &self.snapshot
    }

    pub fn rate(
        &self,
        source: Currency,
        target: Currency,
        freshness: ExchangeRateFreshness,
    ) -> Result<Number, ExchangeRateLookupError> {
        if source == target {
            return Ok(Number::from_i64(1));
        }
        if let ExchangeRateFreshness::NotBefore(minimum) = freshness
            && self
                .snapshot
                .as_of
                .is_none_or(|as_of| as_of.seconds_since(minimum) < 0)
        {
            return Err(ExchangeRateLookupError::Stale);
        }
        if let Some(rate) = self.snapshot.rates.get(&(source, target)) {
            return Ok(rate.clone());
        }
        Number::from_i64(1)
            .divide(
                self.snapshot
                    .rates
                    .get(&(target, source))
                    .ok_or(ExchangeRateLookupError::Missing)?
                    .clone(),
            )
            .map_err(|_| ExchangeRateLookupError::Missing)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExchangeRateFreshness {
    #[default]
    Any,
    NotBefore(OffsetDateTime),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExchangeRateLookupError {
    Missing,
    Stale,
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
    pub exchange_rate_freshness: ExchangeRateFreshness,
}

impl Default for MonetaryContext {
    fn default() -> Self {
        Self {
            default_currency: Currency::Usd,
            exchange_rates: ExchangeRates::default(),
            exchange_rate_freshness: ExchangeRateFreshness::Any,
        }
    }
}
