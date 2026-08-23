use std::{collections::BTreeMap, fmt};

use crate::{Currency, Money, Number, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CryptoAsset {
    symbol: String,
    name: String,
}

impl CryptoAsset {
    pub fn new(symbol: impl Into<String>, name: impl Into<String>) -> Result<Self, CryptoError> {
        let symbol = symbol.into().to_ascii_uppercase();
        let name = name.into();
        if !(2..=12).contains(&symbol.len())
            || !symbol.bytes().all(|byte| byte.is_ascii_alphanumeric())
            || name.trim().is_empty()
        {
            return Err(CryptoError::InvalidAsset);
        }
        Ok(Self { symbol, name })
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for CryptoAsset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.symbol)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CryptoAmount {
    amount: Number,
    asset: CryptoAsset,
}

impl CryptoAmount {
    pub const fn new(amount: Number, asset: CryptoAsset) -> Self {
        Self { amount, asset }
    }

    pub const fn amount(&self) -> &Number {
        &self.amount
    }

    pub const fn asset(&self) -> &CryptoAsset {
        &self.asset
    }
}

impl fmt::Display for CryptoAmount {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.amount, self.asset)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CryptoQuote {
    asset: CryptoAsset,
    price: Money,
    observed_at: OffsetDateTime,
    source: String,
}

impl CryptoQuote {
    pub fn new(
        asset: CryptoAsset,
        price: Money,
        observed_at: OffsetDateTime,
        source: impl Into<String>,
    ) -> Result<Self, CryptoError> {
        let source = source.into();
        if source.trim().is_empty()
            || !price
                .amount()
                .compare(&Number::from_i64(0))
                .is_some_and(std::cmp::Ordering::is_gt)
        {
            return Err(CryptoError::InvalidQuote);
        }
        Ok(Self {
            asset,
            price,
            observed_at,
            source,
        })
    }

    pub const fn asset(&self) -> &CryptoAsset {
        &self.asset
    }

    pub const fn price(&self) -> &Money {
        &self.price
    }

    pub const fn observed_at(&self) -> OffsetDateTime {
        self.observed_at
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CryptoQuoteFreshness {
    Any,
    NotBefore(OffsetDateTime),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CryptoQuoteSnapshot {
    quotes: BTreeMap<(String, Currency), CryptoQuote>,
}

impl CryptoQuoteSnapshot {
    pub fn new(quotes: impl IntoIterator<Item = CryptoQuote>) -> Result<Self, CryptoError> {
        let mut indexed = BTreeMap::new();
        for quote in quotes {
            let key = (
                quote.asset.symbol.to_ascii_lowercase(),
                quote.price.currency(),
            );
            if indexed.insert(key, quote).is_some() {
                return Err(CryptoError::DuplicateQuote);
            }
        }
        if indexed.is_empty() {
            return Err(CryptoError::InvalidSnapshot);
        }
        Ok(Self { quotes: indexed })
    }

    pub fn quotes(&self) -> impl Iterator<Item = &CryptoQuote> {
        self.quotes.values()
    }

    pub fn quote(
        &self,
        asset: &CryptoAsset,
        currency: Currency,
        freshness: CryptoQuoteFreshness,
    ) -> Result<&CryptoQuote, CryptoError> {
        let quote = self
            .quotes
            .get(&(asset.symbol.to_ascii_lowercase(), currency))
            .ok_or(CryptoError::MissingQuote)?;
        if let CryptoQuoteFreshness::NotBefore(not_before) = freshness
            && quote.observed_at.nanoseconds_since(not_before) < 0
        {
            return Err(CryptoError::StaleQuote);
        }
        Ok(quote)
    }

    pub fn convert(
        &self,
        amount: &CryptoAmount,
        currency: Currency,
        freshness: CryptoQuoteFreshness,
    ) -> Result<Money, CryptoError> {
        let quote = self.quote(amount.asset(), currency, freshness)?;
        Ok(Money::new(
            amount
                .amount()
                .clone()
                .multiply(quote.price.amount().clone()),
            currency,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CryptoError {
    InvalidAsset,
    InvalidQuote,
    InvalidSnapshot,
    DuplicateQuote,
    MissingQuote,
    StaleQuote,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidAsset => "the crypto asset is invalid",
            Self::InvalidQuote => "the crypto quote is invalid",
            Self::InvalidSnapshot => "the crypto quote snapshot is invalid",
            Self::DuplicateQuote => "the crypto quote snapshot contains duplicate quotes",
            Self::MissingQuote => "no crypto quote was provided",
            Self::StaleQuote => "the crypto quote is stale",
        })
    }
}

impl std::error::Error for CryptoError {}
