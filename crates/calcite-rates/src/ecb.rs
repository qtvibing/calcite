use std::collections::HashMap;

use calcite_core::{Currency, ExchangeRateSnapshot, Number};
use quick_xml::{Reader, events::Event};

use crate::{ExchangeRateProvider, ProviderError, ProviderFuture, RateRequest, date_at_utc};

/// Loads the European Central Bank's daily euro reference-rate snapshot.
#[derive(Clone, Debug)]
pub struct EcbProvider {
    client: reqwest::Client,
    url: String,
}

impl EcbProvider {
    /// Creates a provider using the official ECB daily-feed URL.
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            url: "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml".into(),
        }
    }

    /// Creates a provider with an alternate feed URL, primarily for controlled deployments and tests.
    pub fn with_url(client: reqwest::Client, url: impl Into<String>) -> Self {
        Self {
            client,
            url: url.into(),
        }
    }
}

impl Default for EcbProvider {
    fn default() -> Self {
        Self::new(reqwest::Client::new())
    }
}

impl ExchangeRateProvider for EcbProvider {
    fn fetch<'a>(&'a self, request: &'a RateRequest) -> ProviderFuture<'a> {
        Box::pin(async move {
            let xml = self
                .client
                .get(&self.url)
                .send()
                .await
                .map_err(|error| ProviderError::Transport(error.to_string()))?
                .error_for_status()
                .map_err(|error| ProviderError::Transport(error.to_string()))?
                .text()
                .await
                .map_err(|error| ProviderError::Transport(error.to_string()))?;
            let (date, euro_rates) = parse_ecb(&xml)?;
            let euro = Currency::Eur;
            let one = Number::parse_decimal("1").expect("one is valid");
            let base_rate = if request.base() == euro {
                one.clone()
            } else {
                euro_rates
                    .get(&request.base())
                    .cloned()
                    .ok_or(ProviderError::NoRates)?
            };
            let quotes = if request.quotes().is_empty() {
                euro_rates.keys().copied().chain([euro]).collect::<Vec<_>>()
            } else {
                request.quotes().to_vec()
            };
            let mut rates = Vec::new();
            for quote in quotes {
                if quote == request.base() {
                    continue;
                }
                let quote_rate = if quote == euro {
                    one.clone()
                } else if let Some(rate) = euro_rates.get(&quote) {
                    rate.clone()
                } else {
                    continue;
                };
                let rate = quote_rate
                    .checked_divide(base_rate.clone())
                    .ok_or(ProviderError::InvalidSnapshot)?;
                rates.push((request.base(), quote, rate));
            }
            if rates.is_empty()
                || (!request.quotes().is_empty() && rates.len() != request.quotes().len())
            {
                return Err(ProviderError::NoRates);
            }
            ExchangeRateSnapshot::new(Some(date_at_utc(&date)?), rates)
                .map_err(|_| ProviderError::InvalidSnapshot)
        })
    }
}

fn parse_ecb(source: &str) -> Result<(String, HashMap<Currency, Number>), ProviderError> {
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);
    let mut date = None;
    let mut rates = HashMap::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) | Ok(Event::Empty(element))
                if element.local_name().as_ref() == b"Cube" =>
            {
                let mut currency = None;
                let mut rate = None;
                for attribute in element.attributes() {
                    let attribute = attribute
                        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
                    match attribute.key.local_name().as_ref() {
                        b"time" => {
                            date = Some(
                                attribute
                                    .unescape_value()
                                    .map_err(|error| {
                                        ProviderError::InvalidResponse(error.to_string())
                                    })?
                                    .into_owned(),
                            );
                        }
                        b"currency" => {
                            currency = Currency::from_code(&attribute.unescape_value().map_err(
                                |error| ProviderError::InvalidResponse(error.to_string()),
                            )?);
                        }
                        b"rate" => {
                            rate = Number::parse_decimal(&attribute.unescape_value().map_err(
                                |error| ProviderError::InvalidResponse(error.to_string()),
                            )?)
                            .ok();
                        }
                        _ => {}
                    }
                }
                if let (Some(currency), Some(rate)) = (currency, rate) {
                    rates.insert(currency, rate);
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(ProviderError::InvalidResponse(error.to_string())),
        }
    }
    let date = date.ok_or_else(|| ProviderError::InvalidResponse("missing ECB date".into()))?;
    if rates.is_empty() {
        return Err(ProviderError::NoRates);
    }
    Ok((date, rates))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_ecb_cube_format() {
        let (date, rates) = parse_ecb(
            r#"<Envelope><Cube><Cube time="2025-03-07"><Cube currency="USD" rate="1.0837"/><Cube currency="JPY" rate="160.12"/></Cube></Cube></Envelope>"#,
        )
        .unwrap();
        assert_eq!(date, "2025-03-07");
        assert_eq!(rates.get(&Currency::Usd).unwrap().to_string(), "1.0837");
        assert_eq!(rates.get(&Currency::Jpy).unwrap().to_string(), "160.12");
    }
}
