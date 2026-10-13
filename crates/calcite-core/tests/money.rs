use calcite_core::{
    CalculationContext, ClockTime, Currency, Date, DiagnosticKind, ExchangeRateFreshness,
    ExchangeRateSnapshot, Number, NumberOutputMode, OffsetDateTime, UtcOffset, calculate,
    calculate_document, calculate_with_context,
};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn iso_currency_snapshot_exposes_codes_and_minor_units() {
    assert_eq!(Currency::all().len(), 178);
    for currency in Currency::all() {
        assert_eq!(Currency::from_code(currency.code()), Some(*currency));
    }
    assert_eq!(Currency::from_code("BHD").unwrap().minor_units(), Some(3));
    assert_eq!(Currency::from_code("JPY").unwrap().minor_units(), Some(0));
    assert_eq!(Currency::from_code("CLF").unwrap().minor_units(), Some(4));
    assert_eq!(Currency::from_code("XAU").unwrap().minor_units(), None);
    assert!(Currency::from_code("NOT").is_none());
}

#[test]
fn expanded_currency_codes_create_typed_money() {
    for (source, expected) in [
        ("AED 100", "100 AED"),
        ("1.2345 BHD", "1.235 BHD"),
        ("5000 CLP", "5000 CLP"),
        ("BRL 20 + BRL 5", "25 BRL"),
        ("XAU 2", "2 XAU"),
    ] {
        assert_eq!(output(source), expected);
    }
}

#[test]
fn currency_postfix_labels_any_plain_number_result() {
    for (source, expected) in [
        ("(100 - 20%) EUR", "80 EUR"),
        ("100 - 20% EUR", "80 EUR"),
        ("(5 + 5) USD", "10 USD"),
        ("(2 * 3) EUR", "6 EUR"),
    ] {
        assert_eq!(output(source), expected);
    }
    let diagnostic = calculate("(100 - 20%) EUR EUR").unwrap_err();
    assert_eq!(diagnostic.kind(), DiagnosticKind::CurrencyMismatch);
    let diagnostic = calculate("20% EUR").unwrap_err();
    assert_eq!(diagnostic.kind(), DiagnosticKind::UnsupportedOperation);
}

#[test]
fn english_currency_aliases_resolve_to_iso_currencies() {
    for (source, expected) in [
        ("100 dollars", "100 USD"),
        ("dollar 100", "100 USD"),
        ("25 euros", "25 EUR"),
        ("sterling 40", "40 GBP"),
        ("50 rmb", "50 CNY"),
        ("3000 yen + 200 yen", "3200 JPY"),
        ("(30 - 10%) rupees", "27 INR"),
    ] {
        assert_eq!(output(source), expected);
    }
    // `pound` remains the mass unit; GBP is spelled `sterling`.
    assert_eq!(calculate("5 pounds").unwrap().output, "5 lb");
}

#[test]
fn money_displays_rounded_to_currency_minor_units() {
    assert_eq!(output("1564.3636363636 EUR"), "1564.36 EUR");
    assert_eq!(output("3200.5 JPY"), "3201 JPY");
    assert_eq!(output("1.234567 BHD"), "1.235 BHD");
    assert_eq!(output("80 USD"), "80 USD");
    // Explicitly chosen precision is respected; auto mode is the only mode
    // that rounds to minor units.
    let mut context = CalculationContext::default();
    context.output_format.number_mode = NumberOutputMode::Decimal { places: 10 };
    assert_eq!(
        calculate_with_context("1564.3636363636 EUR", &context)
            .unwrap()
            .output,
        "1564.3636363636 EUR"
    );
    // The typed value keeps full precision regardless of display rounding.
    let calculation = calculate("1564.3636363636 EUR").unwrap();
    assert!(calculation.value.as_money().is_some());
}

#[test]
fn qualified_currency_names_disambiguate_pesos_and_krona() {
    for (source, expected) in [
        ("100 mexican pesos", "100 MXN"),
        ("swedish krona 250", "250 SEK"),
        ("30 philippine peso", "30 PHP"),
        ("(30 - 10%) danish kroner", "27 DKK"),
        ("british pounds 12", "12 GBP"),
        ("new zealand dollars 99", "99 NZD"),
        ("hong kong dollar 75", "75 HKD"),
    ] {
        assert_eq!(output(source), expected);
    }
}

#[test]
fn multi_word_currency_aliases_work_as_conversion_targets() {
    let mut context = CalculationContext::default();
    context.monetary.exchange_rates.reload(
        ExchangeRateSnapshot::new(
            None,
            [
                (
                    Currency::Usd,
                    Currency::from_code("MXN").unwrap(),
                    Number::parse_decimal("17.0").unwrap(),
                ),
                (
                    Currency::Usd,
                    Currency::from_code("HKD").unwrap(),
                    Number::parse_decimal("7.8").unwrap(),
                ),
            ],
        )
        .unwrap(),
    );
    for (source, expected) in [
        ("100 USD in mexican pesos", "1700 MXN"),
        ("100 USD to hong kong dollars", "780 HKD"),
        ("(100 - 50%) us dollars in mexican pesos", "850 MXN"),
    ] {
        assert_eq!(
            calculate_with_context(source, &context).unwrap().output,
            expected
        );
    }
}

#[test]
fn currency_codes_are_case_insensitive() {
    assert_eq!(output("usd 10 + UsD 5"), "15 USD");
    assert_eq!(output("cup 10 + 5 CuP"), "15 CUP");
}

#[test]
fn symbols_and_codes_create_unambiguous_typed_money() {
    for (source, expected) in [
        ("$100", "100 USD"),
        ("EUR 100", "100 EUR"),
        ("100 GBP", "100 GBP"),
        ("€25.50", "25.5 EUR"),
        ("5000 ¥", "5000 JPY"),
        ("₹2k", "2000 INR"),
    ] {
        let calculation = calculate(source).unwrap();
        assert_eq!(calculation.output, expected);
        assert!(calculation.value.as_money().is_some());
    }
}

#[test]
fn ambiguous_yen_and_won_symbols_follow_compatible_context_defaults() {
    let mut context = CalculationContext::default();
    context.monetary.default_currency = Currency::Cny;
    assert_eq!(
        calculate_with_context("¥100", &context).unwrap().output,
        "100 CNY"
    );

    context.monetary.default_currency = Currency::from_code("KPW").unwrap();
    assert_eq!(
        calculate_with_context("₩100", &context).unwrap().output,
        "100 KPW"
    );

    context.monetary.default_currency = Currency::Eur;
    assert_eq!(
        calculate_with_context("¥100", &context).unwrap().output,
        "100 JPY"
    );
    assert_eq!(
        calculate_with_context("₩100", &context).unwrap().output,
        "100 KRW"
    );
}

#[test]
fn dollar_symbols_use_the_explicit_context_default() {
    let mut context = CalculationContext::default();
    context.monetary.default_currency = Currency::Cad;
    assert_eq!(
        calculate_with_context("$20", &context).unwrap().output,
        "20 CAD"
    );
}

#[test]
fn money_per_unit_rates_preserve_currency_and_solve_quantities() {
    assert_eq!(output("$100 / hour"), "100 USD/h");
    assert_eq!(output("$100 / 2 hours"), "50 USD/h");
    assert_eq!(output("$100/hour * 90 minutes"), "150 USD");
    assert_eq!(output("90 minutes * ($100/hour)"), "150 USD");
    assert_eq!(output("$500 / ($100/hour)"), "5 h");
    assert_eq!(output("$500 at $20/hour"), "25 h");
    assert_eq!(output("$120/hour in minute"), "2 USD/min");
    assert_eq!(output("$100/hour + $2/minute"), "220 USD/h");
}

#[test]
fn money_arithmetic_and_percentages_preserve_currency() {
    assert_eq!(output("$100 + $25"), "125 USD");
    assert_eq!(output("$100 + 10%"), "110 USD");
    assert_eq!(output("20% of EUR 50"), "10 EUR");
    assert_eq!(output("1.5k GBP / 3"), "500 GBP");
    assert_eq!(output("$100 / $25"), "4");
}

#[test]
fn exchange_rates_are_injected_snapshots_and_work_in_both_directions() {
    let mut context = CalculationContext::default();
    context.monetary.exchange_rates.reload(
        ExchangeRateSnapshot::new(
            None,
            [(
                Currency::Usd,
                Currency::Eur,
                Number::parse_decimal("0.8").unwrap(),
            )],
        )
        .unwrap(),
    );

    assert_eq!(
        calculate_with_context("100 USD in EUR", &context)
            .unwrap()
            .output,
        "80 EUR"
    );
    assert_eq!(
        calculate_with_context("80 EUR in USD", &context)
            .unwrap()
            .output,
        "100 USD"
    );
    assert_eq!(
        calculate_with_context("100 USD in EUR as number", &context)
            .unwrap()
            .output,
        "80"
    );
    assert_eq!(
        calculate_with_context("100 USD/hour in EUR", &context)
            .unwrap()
            .output,
        "80 EUR/h"
    );
}

#[test]
fn coherent_snapshots_support_cross_rates_through_a_shared_base() {
    let mut context = CalculationContext::default();
    context.monetary.exchange_rates.reload(
        ExchangeRateSnapshot::new(
            None,
            [
                (
                    Currency::Usd,
                    Currency::Eur,
                    Number::parse_decimal("0.8").unwrap(),
                ),
                (
                    Currency::Usd,
                    Currency::Jpy,
                    Number::parse_decimal("150").unwrap(),
                ),
            ],
        )
        .unwrap(),
    );

    assert_eq!(
        calculate_with_context("80 EUR in JPY", &context)
            .unwrap()
            .output,
        "15000 JPY"
    );
}

#[test]
fn exchange_snapshots_reload_as_whole_validated_values() {
    let first = ExchangeRateSnapshot::new(
        None,
        [(
            Currency::Usd,
            Currency::Eur,
            Number::parse_decimal("0.8").unwrap(),
        )],
    )
    .unwrap();
    let second = ExchangeRateSnapshot::new(
        None,
        [(
            Currency::Usd,
            Currency::Eur,
            Number::parse_decimal("0.9").unwrap(),
        )],
    )
    .unwrap();
    let mut rates = calcite_core::ExchangeRates::default();
    assert!(rates.reload(first).is_empty());
    let replaced = rates.reload(second);
    assert_eq!(replaced.len(), 1);
    assert_eq!(rates.snapshot().len(), 1);

    assert!(
        ExchangeRateSnapshot::new(
            None,
            [(
                Currency::Usd,
                Currency::Usd,
                Number::parse_decimal("1").unwrap(),
            )],
        )
        .is_err()
    );
    assert_eq!(rates.snapshot().len(), 1);
}

#[test]
fn exchange_snapshot_freshness_is_an_explicit_policy() {
    let as_of = OffsetDateTime::new(
        Date::new(2025, 1, 10).unwrap(),
        ClockTime::new(0, 0, 0).unwrap(),
        UtcOffset::UTC,
    );
    let mut context = CalculationContext::default();
    context.monetary.exchange_rates.reload(
        ExchangeRateSnapshot::new(
            Some(as_of),
            [(
                Currency::Usd,
                Currency::Eur,
                Number::parse_decimal("0.8").unwrap(),
            )],
        )
        .unwrap(),
    );
    context.monetary.exchange_rate_freshness =
        ExchangeRateFreshness::NotBefore(OffsetDateTime::new(
            Date::new(2025, 1, 11).unwrap(),
            ClockTime::new(0, 0, 0).unwrap(),
            UtcOffset::UTC,
        ));
    assert_eq!(
        calculate_with_context("100 USD in EUR", &context)
            .unwrap_err()
            .kind(),
        DiagnosticKind::StaleExchangeRate
    );

    context.monetary.exchange_rate_freshness = ExchangeRateFreshness::NotBefore(as_of);
    assert_eq!(
        calculate_with_context("100 USD in EUR", &context)
            .unwrap()
            .output,
        "80 EUR"
    );
}

#[test]
fn exchange_snapshots_accept_the_expanded_currency_set() {
    let mut context = CalculationContext::default();
    let brl = Currency::from_code("BRL").unwrap();
    let mxn = Currency::from_code("MXN").unwrap();
    context.monetary.exchange_rates.reload(
        ExchangeRateSnapshot::new(None, [(brl, mxn, Number::parse_decimal("3.5").unwrap())])
            .unwrap(),
    );

    assert_eq!(
        calculate_with_context("20 BRL in MXN", &context)
            .unwrap()
            .output,
        "70 MXN"
    );
}

#[test]
fn cross_currency_arithmetic_requires_an_explicit_conversion() {
    let mismatch = calculate("10 USD + 10 EUR").unwrap_err();
    assert_eq!(mismatch.kind(), DiagnosticKind::CurrencyMismatch);

    let missing = calculate("100 USD in EUR").unwrap_err();
    assert_eq!(missing.kind(), DiagnosticKind::MissingExchangeRate);
    assert_eq!(missing.detail(), Some("USD to EUR"));
}

#[test]
fn money_values_flow_through_document_variables() {
    let document = calculate_document("subtotal = $80\ntax = 25%\nsubtotal + tax").unwrap();
    assert_eq!(document.lines[2].output, "100 USD");
    assert!(document.lines[2].value.as_money().is_some());
}
