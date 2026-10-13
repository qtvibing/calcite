use calcite_core::{
    Currency, DiagnosticKind, Money, Number, Percentage, amortization_schedule, calculate,
    calculate_document,
};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn amortization_schedules_are_bounded_typed_and_deterministic() {
    let schedule = amortization_schedule(
        Money::new(Number::parse_decimal("1200").unwrap(), Currency::Usd),
        Percentage::from_ratio(Number::parse_decimal("0").unwrap()),
        1,
        12,
    )
    .unwrap();
    assert_eq!(schedule.payment().to_string(), "100 USD");
    assert_eq!(schedule.payments().len(), 12);
    assert_eq!(schedule.payments()[0].period(), 1);
    assert_eq!(schedule.payments()[0].interest().to_string(), "0 USD");
    assert_eq!(
        schedule
            .payments()
            .last()
            .unwrap()
            .remaining_balance()
            .to_string(),
        "0 USD"
    );

    let interest_schedule = amortization_schedule(
        Money::new(Number::parse_decimal("1000").unwrap(), Currency::Usd),
        Percentage::from_ratio(Number::parse_decimal("0.12").unwrap()),
        1,
        12,
    )
    .unwrap();
    assert_eq!(
        interest_schedule.payments()[0].interest().to_string(),
        "10 USD"
    );
}

#[test]
fn tax_functions_distinguish_net_gross_and_extracted_tax() {
    assert_eq!(output("sales_tax($300, 15%)"), "45 USD");
    assert_eq!(output("add_tax($300, 15%)"), "345 USD");
    assert_eq!(output("round(remove_tax($300, 15%), 2)"), "260.87 USD");
    assert_eq!(output("round(tax_from_total($300, 15%), 2)"), "39.13 USD");
}

#[test]
fn natural_finance_phrases_preserve_typed_results() {
    assert_eq!(output("sales tax on $300 at 15%"), "45 USD");
    assert_eq!(output("add tax to $300 at 15%"), "345 USD");
    assert_eq!(
        output("round(future value of $1000 at 7% for 3 years, 2)"),
        "1225.04 USD"
    );
    assert_eq!(
        output("round(mortgage payment on $300000 at 6% for 30 years, 2)"),
        "1798.65 USD"
    );
    assert_eq!(output("return on investment from 100 to 125"), "25%");
    assert_eq!(output("round(cagr from 100 to 121 over 2 years, 2)"), "10%");
}

#[test]
fn growth_and_return_functions_are_precision_aware() {
    assert_eq!(
        output("round(future_value($1000, 7%, 3), 2)"),
        "1225.04 USD"
    );
    assert_eq!(
        output("round(future_value($1000, 7%, 3, 12), 2)"),
        "1232.93 USD"
    );
    assert_eq!(
        output("round(compound_interest($1000, 7%, 3), 2)"),
        "225.04 USD"
    );
    assert_eq!(
        output("round(present_value($1000, 10%, 20), 2)"),
        "148.64 USD"
    );
    assert_eq!(output("round(cagr($1000, $2500, 7), 2)"), "13.99%");
    assert_eq!(output("roi($500, $1500)"), "200%");
}

#[test]
fn amortizing_loan_functions_share_one_formula() {
    assert_eq!(
        output("round(mortgage_payment($10000, 6%, 6), 2)"),
        "165.73 USD"
    );
    assert_eq!(
        output("round(total_repayment($10000, 6%, 6), 2)"),
        "11932.48 USD"
    );
    assert_eq!(
        output("round(total_interest($10000, 6%, 6), 2)"),
        "1932.48 USD"
    );
    assert_eq!(output("mortgage_payment($1200, 0%, 1)"), "100 USD");
}

#[test]
fn purchasing_power_and_break_even_are_typed() {
    assert_eq!(
        output("round(purchasing_power($500, 5%, 3), 2)"),
        "431.92 USD"
    );
    assert_eq!(output("break_even($10000, $25)"), "400");
}

#[test]
fn finance_domains_are_explicit() {
    for source in [
        "future_value(100, 5%, 0)",
        "future_value(100, 5%, 2, 0)",
        "cagr(100, 200, -1)",
        "mortgage_payment(1000, 5%, 0)",
    ] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::InvalidFinanceArguments
        );
    }
    assert_eq!(
        calculate("future_value(100, 0.05, 2)").unwrap_err().kind(),
        DiagnosticKind::InvalidFunctionArguments
    );
}

#[test]
fn finance_values_flow_through_documents() {
    let document = calculate_document(
        "principal = $10000\nrate = 6%\nyears = 6\npayment = mortgage_payment(principal, rate, years)\nround(payment, 2)",
    )
    .unwrap();
    assert_eq!(document.lines[3].output, "165.73 USD");
    assert_eq!(document.lines[4].output, "165.73 USD");
}
