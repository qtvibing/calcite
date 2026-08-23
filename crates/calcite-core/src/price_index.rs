use std::{cmp::Ordering, collections::BTreeMap, fmt};

use crate::{Currency, Date, Money, Number, OffsetDateTime};

#[derive(Clone, Debug, PartialEq)]
pub struct PriceIndexSeries {
    id: String,
    currency: Currency,
    observations: BTreeMap<Date, Number>,
}

impl PriceIndexSeries {
    pub fn new(
        id: impl Into<String>,
        currency: Currency,
        observations: impl IntoIterator<Item = (Date, Number)>,
    ) -> Result<Self, PriceIndexError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(PriceIndexError::InvalidSeries);
        }
        let mut indexed = BTreeMap::new();
        for (date, value) in observations {
            if !value
                .compare(&Number::from_i64(0))
                .is_some_and(Ordering::is_gt)
                || indexed.insert(date, value).is_some()
            {
                return Err(PriceIndexError::InvalidSeries);
            }
        }
        if indexed.is_empty() {
            return Err(PriceIndexError::InvalidSeries);
        }
        Ok(Self {
            id,
            currency,
            observations: indexed,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn currency(&self) -> Currency {
        self.currency
    }

    pub fn observations(&self) -> impl Iterator<Item = (Date, &Number)> {
        self.observations.iter().map(|(date, value)| (*date, value))
    }

    pub fn value_on(&self, date: Date) -> Option<&Number> {
        self.observations.get(&date)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PriceIndexSnapshot {
    source: String,
    published_at: OffsetDateTime,
    series: BTreeMap<String, PriceIndexSeries>,
}

impl PriceIndexSnapshot {
    pub fn new(
        source: impl Into<String>,
        published_at: OffsetDateTime,
        series: impl IntoIterator<Item = PriceIndexSeries>,
    ) -> Result<Self, PriceIndexError> {
        let source = source.into();
        if source.trim().is_empty() {
            return Err(PriceIndexError::InvalidSnapshot);
        }
        let mut indexed = BTreeMap::new();
        for series in series {
            let key = series.id.to_ascii_lowercase();
            if indexed.insert(key, series).is_some() {
                return Err(PriceIndexError::DuplicateSeries);
            }
        }
        if indexed.is_empty() {
            return Err(PriceIndexError::InvalidSnapshot);
        }
        Ok(Self {
            source,
            published_at,
            series: indexed,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub const fn published_at(&self) -> OffsetDateTime {
        self.published_at
    }

    pub fn series(&self, id: &str) -> Option<&PriceIndexSeries> {
        self.series.get(&id.to_ascii_lowercase())
    }

    pub fn adjust(
        &self,
        series: &str,
        amount: &Money,
        from: Date,
        to: Date,
    ) -> Result<Money, PriceIndexError> {
        let series = self.series(series).ok_or(PriceIndexError::UnknownSeries)?;
        if amount.currency() != series.currency {
            return Err(PriceIndexError::CurrencyMismatch);
        }
        let from_index = series
            .value_on(from)
            .ok_or(PriceIndexError::MissingObservation)?;
        let to_index = series
            .value_on(to)
            .ok_or(PriceIndexError::MissingObservation)?;
        let adjusted = amount
            .amount()
            .clone()
            .multiply(to_index.clone())
            .divide(from_index.clone())
            .map_err(|_| PriceIndexError::InvalidSeries)?;
        Ok(Money::new(adjusted, amount.currency()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriceIndexError {
    InvalidSeries,
    InvalidSnapshot,
    DuplicateSeries,
    UnknownSeries,
    MissingObservation,
    CurrencyMismatch,
}

impl fmt::Display for PriceIndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSeries => "the price-index series is invalid",
            Self::InvalidSnapshot => "the price-index snapshot is invalid",
            Self::DuplicateSeries => "the price-index snapshot contains duplicate series",
            Self::UnknownSeries => "the requested price-index series is unavailable",
            Self::MissingObservation => "the requested price-index observation is unavailable",
            Self::CurrencyMismatch => "the amount uses a different currency from the index",
        })
    }
}

impl std::error::Error for PriceIndexError {}
