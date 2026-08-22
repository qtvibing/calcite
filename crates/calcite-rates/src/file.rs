use calcite_core::{Currency, ExchangeRateSnapshot, Number, OffsetDateTime};
use serde::Deserialize;

use crate::ProviderError;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotFile {
    #[serde(default)]
    as_of: Option<String>,
    rates: Vec<FileRate>,
}

#[derive(Deserialize)]
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
            let rate = Number::parse_decimal(&rate.rate).map_err(|_| {
                ProviderError::InvalidResponse("rates must be decimal strings".into())
            })?;
            Ok((source, target, rate))
        })
        .collect::<Result<Vec<_>, ProviderError>>()?;
    if rates.is_empty() {
        return Err(ProviderError::NoRates);
    }
    ExchangeRateSnapshot::new(as_of, rates).map_err(|_| ProviderError::InvalidSnapshot)
}
