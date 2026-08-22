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

pub type ProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ExchangeRateSnapshot, ProviderError>> + Send + 'a>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RateRequest {
    base: Currency,
    quotes: Vec<Currency>,
}

impl RateRequest {
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

    pub const fn base(&self) -> Currency {
        self.base
    }

    pub fn quotes(&self) -> &[Currency] {
        &self.quotes
    }
}

pub trait ExchangeRateProvider: Send + Sync {
    fn fetch<'a>(&'a self, request: &'a RateRequest) -> ProviderFuture<'a>;
}

#[derive(Default)]
pub struct FallbackProvider {
    providers: Vec<Box<dyn ExchangeRateProvider>>,
}

impl FallbackProvider {
    pub fn new(providers: Vec<Box<dyn ExchangeRateProvider>>) -> Self {
        Self { providers }
    }

    pub fn push(&mut self, provider: impl ExchangeRateProvider + 'static) {
        self.providers.push(Box::new(provider));
    }

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
