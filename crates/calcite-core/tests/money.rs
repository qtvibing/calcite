use calcite_core::{
    CalculationContext, Currency, DiagnosticKind, Number, calculate, calculate_document,
    calculate_with_context,
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
        ("1.2345 BHD", "1.2345 BHD"),
        ("5000 CLP", "5000 CLP"),
        ("BRL 20 + BRL 5", "25 BRL"),
        ("XAU 2", "2 XAU"),
    ] {
        assert_eq!(output(source), expected);
    }
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
    context
        .monetary
        .exchange_rates
        .set_rate(
            Currency::Usd,
            Currency::Eur,
            Number::parse_decimal("0.8").unwrap(),
        )
        .unwrap();

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
}

#[test]
fn exchange_snapshots_accept_the_expanded_currency_set() {
    let mut context = CalculationContext::default();
    let brl = Currency::from_code("BRL").unwrap();
    let mxn = Currency::from_code("MXN").unwrap();
    context
        .monetary
        .exchange_rates
        .set_rate(brl, mxn, Number::parse_decimal("3.5").unwrap())
        .unwrap();

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
