use calcite_core::{Currency, ExchangeRateSnapshot, Number, OffsetDateTime};
use serde::{Deserialize, Serialize};

use crate::ProviderError;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SnapshotFile {
    #[serde(default)]
    as_of: Option<String>,
    rates: Vec<FileRate>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FileRate {
    source: String,
    target: String,
    rate: String,
}

pub fn load_json_snapshot(source: &str) -> Result<ExchangeRateSnapshot, ProviderError> {
    let file: SnapshotFile = serde_json::from_str(source)
        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
    let as_of = file
        .as_of
        .map(|value| {
            OffsetDateTime::parse_iso(&value).ok_or_else(|| {
                ProviderError::InvalidResponse("as_of must be an ISO-8601 offset date-time".into())
            })
        })
        .transpose()?;
    let rates = file
        .rates
        .into_iter()
        .map(|rate| {
            let source = Currency::from_code(&rate.source).ok_or_else(|| {
                ProviderError::InvalidResponse(format!("unknown currency: {}", rate.source))
            })?;
            let target = Currency::from_code(&rate.target).ok_or_else(|| {
                ProviderError::InvalidResponse(format!("unknown currency: {}", rate.target))
            })?;
            let rate = parse_exact_rate(&rate.rate)?;
            Ok((source, target, rate))
        })
        .collect::<Result<Vec<_>, ProviderError>>()?;
    if rates.is_empty() {
        return Err(ProviderError::NoRates);
    }
    ExchangeRateSnapshot::new(as_of, rates).map_err(|_| ProviderError::InvalidSnapshot)
}

pub fn save_json_snapshot(snapshot: &ExchangeRateSnapshot) -> Result<String, ProviderError> {
    let mut rates = snapshot
        .rates()
        .map(|(source, target, rate)| {
            Ok(FileRate {
                source: source.code().into(),
                target: target.code().into(),
                rate: rate
                    .exact_fraction()
                    .ok_or(ProviderError::InvalidSnapshot)?,
            })
        })
        .collect::<Result<Vec<_>, ProviderError>>()?;
    rates.sort_unstable_by(|left, right| {
        (&left.source, &left.target).cmp(&(&right.source, &right.target))
    });
    serde_json::to_string_pretty(&SnapshotFile {
        as_of: snapshot.as_of().map(|value| value.to_string()),
        rates,
    })
    .map_err(|error| ProviderError::InvalidResponse(error.to_string()))
}

fn parse_exact_rate(source: &str) -> Result<Number, ProviderError> {
    if let Some((numerator, denominator)) = source.split_once('/') {
        let numerator = Number::parse_decimal(numerator).map_err(|_| {
            ProviderError::InvalidResponse("rates must be exact numeric strings".into())
        })?;
        let denominator = Number::parse_decimal(denominator).map_err(|_| {
            ProviderError::InvalidResponse("rates must be exact numeric strings".into())
        })?;
        return numerator
            .checked_divide(denominator)
            .ok_or(ProviderError::InvalidSnapshot);
    }
    Number::parse_decimal(source)
        .map_err(|_| ProviderError::InvalidResponse("rates must be exact numeric strings".into()))
}
