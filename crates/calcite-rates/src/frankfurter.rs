use calcite_core::{Currency, ExchangeRateSnapshot, Number};

use crate::{ExchangeRateProvider, ProviderError, ProviderFuture, RateRequest, date_at_utc};

#[derive(Clone, Debug)]
pub struct FrankfurterProvider {
    client: reqwest::Client,
    base_url: String,
}

impl FrankfurterProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            base_url: "https://api.frankfurter.app".into(),
        }
    }

    pub fn with_base_url(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into(),
        }
    }
}

impl Default for FrankfurterProvider {
    fn default() -> Self {
        Self::new(reqwest::Client::new())
    }
}

impl ExchangeRateProvider for FrankfurterProvider {
    fn fetch<'a>(&'a self, request: &'a RateRequest) -> ProviderFuture<'a> {
        Box::pin(async move {
            let mut url = format!("{}/latest?from={}", self.base_url, request.base());
            if !request.quotes().is_empty() {
                url.push_str("&to=");
                url.push_str(
                    &request
                        .quotes()
                        .iter()
                        .map(|currency| currency.code())
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
            let response = self
                .client
                .get(url)
                .send()
                .await
                .map_err(|error| ProviderError::Transport(error.to_string()))?
                .error_for_status()
                .map_err(|error| ProviderError::Transport(error.to_string()))?;
            let text = response
                .text()
                .await
                .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
            parse_response(&text, request)
        })
    }
}

fn parse_response(
    source: &str,
    request: &RateRequest,
) -> Result<ExchangeRateSnapshot, ProviderError> {
    let body: serde_json::Value = serde_json::from_str(source)
        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
    let date = body
        .get("date")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ProviderError::InvalidResponse("missing date".into()))?;
    let rates = body
        .get("rates")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| ProviderError::InvalidResponse("missing rates".into()))?;
    let mut parsed = Vec::new();
    for (code, value) in rates {
        let Some(currency) = Currency::from_code(code) else {
            continue;
        };
        if !request.quotes().is_empty() && !request.quotes().contains(&currency) {
            continue;
        }
        let number = Number::parse_decimal(&value.to_string())
            .map_err(|_| ProviderError::InvalidResponse(format!("invalid rate for {code}")))?;
        parsed.push((request.base(), currency, number));
    }
    if parsed.is_empty() {
        return Err(ProviderError::NoRates);
    }
    ExchangeRateSnapshot::new(Some(date_at_utc(date)?), parsed)
        .map_err(|_| ProviderError::InvalidSnapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_filters_frankfurter_rates() {
        let request = RateRequest::new(Currency::Usd, [Currency::Eur]);
        let snapshot = parse_response(
            r#"{"date":"2025-03-07","base":"USD","rates":{"EUR":0.9234,"GBP":0.77}}"#,
            &request,
        )
        .unwrap();
        assert_eq!(snapshot.len(), 1);
        assert!(snapshot.as_of().is_some());
    }
}
