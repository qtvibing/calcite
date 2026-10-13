#![doc = include_str!("../README.md")]

use std::{error::Error, fmt, future::Future, pin::Pin};

#[cfg(any(feature = "ecb", feature = "frankfurter"))]
use calcite_core::{ClockTime, Date, OffsetDateTime, UtcOffset};
use calcite_core::{Currency, ExchangeRateSnapshot};

#[cfg(feature = "ecb")]
mod ecb;
#[cfg(feature = "file")]
mod file;
#[cfg(feature = "frankfurter")]
mod frankfurter;

#[cfg(feature = "ecb")]
pub use ecb::EcbProvider;
#[cfg(feature = "file")]
pub use file::{load_json_snapshot, save_json_snapshot};
#[cfg(feature = "frankfurter")]
pub use frankfurter::FrankfurterProvider;

/// The boxed asynchronous result returned by an [`ExchangeRateProvider`].
pub type ProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ExchangeRateSnapshot, ProviderError>> + Send + 'a>>;

/// A normalized request for quote currencies relative to one base currency.
///
/// Construction removes the base currency from `quotes`, sorts the remaining
/// currencies by ISO code, and removes duplicates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RateRequest {
    base: Currency,
    quotes: Vec<Currency>,
}

impl RateRequest {
    /// Creates a normalized request for the supplied base and quote currencies.
    pub fn new<I>(base: Currency, quotes: I) -> Self
    where
        I: IntoIterator<Item = Currency>,
    {
        let mut quotes = quotes
            .into_iter()
            .filter(|quote| *quote != base)
            .collect::<Vec<_>>();
        quotes.sort_unstable_by_key(|currency| currency.code());
        quotes.dedup();
        Self { base, quotes }
    }

    /// Returns the currency against which all quotes are requested.
    pub const fn base(&self) -> Currency {
        self.base
    }

    /// Returns the sorted, deduplicated quote currencies.
    pub fn quotes(&self) -> &[Currency] {
        &self.quotes
    }
}

/// An asynchronous source of complete, validated exchange-rate snapshots.
///
/// Providers return data only; callers decide when to fetch, cache, and install
/// a snapshot into `calcite-core`.
pub trait ExchangeRateProvider: Send + Sync {
    /// Fetches one coherent snapshot for `request`.
    fn fetch<'a>(&'a self, request: &'a RateRequest) -> ProviderFuture<'a>;
}

/// Tries providers in order and returns the first successful complete snapshot.
///
/// Snapshots from multiple providers are never merged.
#[derive(Default)]
pub struct FallbackProvider {
    providers: Vec<Box<dyn ExchangeRateProvider>>,
}

impl FallbackProvider {
    /// Creates a fallback chain in attempt order.
    pub fn new(providers: Vec<Box<dyn ExchangeRateProvider>>) -> Self {
        Self { providers }
    }

    /// Appends a provider to the end of the fallback chain.
    pub fn push(&mut self, provider: impl ExchangeRateProvider + 'static) {
        self.providers.push(Box::new(provider));
    }

    /// Returns whether the fallback chain has no providers.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

impl ExchangeRateProvider for FallbackProvider {
    fn fetch<'a>(&'a self, request: &'a RateRequest) -> ProviderFuture<'a> {
        Box::pin(async move {
            let mut failures = Vec::new();
            for provider in &self.providers {
                match provider.fetch(request).await {
                    Ok(snapshot) => return Ok(snapshot),
                    Err(error) => failures.push(error.to_string()),
                }
            }
            Err(ProviderError::AllProvidersFailed(failures))
        })
    }
}

/// A transport, decoding, validation, or fallback failure from a rate provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderError {
    Transport(String),
    InvalidResponse(String),
    InvalidSnapshot,
    NoRates,
    AllProvidersFailed(Vec<String>),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(message) => {
                write!(formatter, "exchange-rate transport failed: {message}")
            }
            Self::InvalidResponse(message) => {
                write!(formatter, "exchange-rate response was invalid: {message}")
            }
            Self::InvalidSnapshot => formatter.write_str("exchange-rate snapshot was invalid"),
            Self::NoRates => formatter.write_str("the provider returned no requested rates"),
            Self::AllProvidersFailed(failures) => {
                write!(
                    formatter,
                    "all exchange-rate providers failed: {}",
                    failures.join("; ")
                )
            }
        }
    }
}

impl Error for ProviderError {}

#[cfg(any(feature = "ecb", feature = "frankfurter"))]
fn date_at_utc(source: &str) -> Result<OffsetDateTime, ProviderError> {
    let mut parts = source.split('-');
    let year = parts.next().and_then(|value| value.parse().ok());
    let month = parts.next().and_then(|value| value.parse().ok());
    let day = parts.next().and_then(|value| value.parse().ok());
    if parts.next().is_some() {
        return Err(ProviderError::InvalidResponse(
            "invalid provider date".into(),
        ));
    }
    let date = year
        .zip(month)
        .zip(day)
        .and_then(|((year, month), day)| Date::new(year, month, day))
        .ok_or_else(|| ProviderError::InvalidResponse("invalid provider date".into()))?;
    Ok(OffsetDateTime::new(
        date,
        ClockTime::new(0, 0, 0).expect("midnight is valid"),
        UtcOffset::UTC,
    ))
}
