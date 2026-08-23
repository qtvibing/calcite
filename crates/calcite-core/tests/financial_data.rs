use calcite_core::{
    CryptoAmount, CryptoAsset, CryptoError, CryptoQuote, CryptoQuoteFreshness, CryptoQuoteSnapshot,
    Currency, Date, Money, Number, OffsetDateTime, PriceIndexError, PriceIndexSeries,
    PriceIndexSnapshot,
};

fn number(value: &str) -> Number {
    Number::parse_decimal(value).unwrap()
}

fn timestamp(value: &str) -> OffsetDateTime {
    OffsetDateTime::parse_iso(value).unwrap()
}

#[test]
fn price_index_adjustment_requires_an_explicit_complete_snapshot() {
    let series = PriceIndexSeries::new(
        "US-CPI-U",
        Currency::Usd,
        [
            (Date::new(2020, 1, 1).unwrap(), number("258.678")),
            (Date::new(2025, 1, 1).unwrap(), number("317.671")),
        ],
    )
    .unwrap();
    let snapshot = PriceIndexSnapshot::new(
        "example-statistics-office",
        timestamp("2025-02-01T00:00Z"),
        [series],
    )
    .unwrap();
    let adjusted = snapshot
        .adjust(
            "us-cpi-u",
            &Money::new(number("100"), Currency::Usd),
            Date::new(2020, 1, 1).unwrap(),
            Date::new(2025, 1, 1).unwrap(),
        )
        .unwrap();
    assert_eq!(adjusted.currency(), Currency::Usd);
    assert_eq!(
        adjusted.amount().exact_fraction().as_deref(),
        Some("15883550/129339")
    );
    assert_eq!(snapshot.source(), "example-statistics-office");
    assert_eq!(
        snapshot
            .adjust(
                "US-CPI-U",
                &Money::new(number("100"), Currency::Eur),
                Date::new(2020, 1, 1).unwrap(),
                Date::new(2025, 1, 1).unwrap(),
            )
            .unwrap_err(),
        PriceIndexError::CurrencyMismatch
    );
}

#[test]
fn crypto_quotes_are_separate_source_attributed_and_freshness_checked() {
    let bitcoin = CryptoAsset::new("btc", "Bitcoin").unwrap();
    let observed_at = timestamp("2025-03-01T12:00Z");
    let quote = CryptoQuote::new(
        bitcoin.clone(),
        Money::new(number("80000"), Currency::Usd),
        observed_at,
        "example-exchange",
    )
    .unwrap();
    assert_eq!(quote.source(), "example-exchange");
    let snapshot = CryptoQuoteSnapshot::new([quote]).unwrap();
    let converted = snapshot
        .convert(
            &CryptoAmount::new(number("0.125"), bitcoin.clone()),
            Currency::Usd,
            CryptoQuoteFreshness::NotBefore(timestamp("2025-03-01T11:55Z")),
        )
        .unwrap();
    assert_eq!(converted.to_string(), "10000 USD");
    assert_eq!(bitcoin.symbol(), "BTC");

    assert_eq!(
        snapshot
            .convert(
                &CryptoAmount::new(number("1"), bitcoin),
                Currency::Usd,
                CryptoQuoteFreshness::NotBefore(timestamp("2025-03-01T12:01Z")),
            )
            .unwrap_err(),
        CryptoError::StaleQuote
    );
}
