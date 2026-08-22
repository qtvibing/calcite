use std::sync::Arc;

#[cfg(feature = "file")]
use calcite_core::ExchangeRateFreshness;
use calcite_core::{Currency, ExchangeRateSnapshot, Number};
use calcite_rates::{
    ExchangeRateProvider, FallbackProvider, ProviderError, ProviderFuture, RateRequest,
};

struct FailingProvider;

impl ExchangeRateProvider for FailingProvider {
    fn fetch<'a>(&'a self, _request: &'a RateRequest) -> ProviderFuture<'a> {
        Box::pin(async { Err(ProviderError::Transport("offline".into())) })
    }
}

struct FixedProvider(Arc<ExchangeRateSnapshot>);

impl ExchangeRateProvider for FixedProvider {
    fn fetch<'a>(&'a self, _request: &'a RateRequest) -> ProviderFuture<'a> {
        Box::pin(async { Ok((*self.0).clone()) })
    }
}

#[tokio::test]
async fn fallback_uses_the_first_successful_complete_snapshot() {
    let snapshot = ExchangeRateSnapshot::new(
        None,
        [(
            Currency::Usd,
            Currency::Eur,
            Number::parse_decimal("0.8").unwrap(),
        )],
    )
    .unwrap();
    let provider = FallbackProvider::new(vec![
        Box::new(FailingProvider),
        Box::new(FixedProvider(Arc::new(snapshot))),
    ]);
    let loaded = provider
        .fetch(&RateRequest::new(Currency::Usd, [Currency::Eur]))
        .await
        .unwrap();
    assert_eq!(loaded.len(), 1);
}

#[test]
fn requests_deduplicate_quotes_and_remove_the_base() {
    let request = RateRequest::new(
        Currency::Usd,
        [Currency::Eur, Currency::Usd, Currency::Eur, Currency::Gbp],
    );
    assert_eq!(request.quotes(), &[Currency::Eur, Currency::Gbp]);
}

#[cfg(feature = "file")]
#[test]
fn timestamped_json_files_load_exact_rates() {
    let snapshot = calcite_rates::load_json_snapshot(
        r#"{
            "as_of": "2025-03-08T12:30:00Z",
            "rates": [
                {"source": "USD", "target": "EUR", "rate": "0.81234567890123456789"}
            ]
        }"#,
    )
    .unwrap();
    assert!(snapshot.as_of().is_some());
    let rate = calcite_core::ExchangeRates::default();
    let mut rates = rate;
    rates.reload(snapshot);
    assert_eq!(
        rates
            .rate(Currency::Usd, Currency::Eur, ExchangeRateFreshness::Any)
            .unwrap()
            .to_string(),
        "0.8123456789"
    );
    let serialized = calcite_rates::save_json_snapshot(rates.snapshot()).unwrap();
    let reloaded = calcite_rates::load_json_snapshot(&serialized).unwrap();
    assert_eq!(
        reloaded
            .rates()
            .next()
            .unwrap()
            .2
            .exact_fraction()
            .as_deref(),
        Some("81234567890123456789/100000000000000000000")
    );
}

#[cfg(feature = "file")]
#[test]
fn malformed_or_empty_files_are_rejected() {
    assert!(calcite_rates::load_json_snapshot("{}").is_err());
    assert!(calcite_rates::load_json_snapshot(r#"{"rates": []}"#).is_err());
}
