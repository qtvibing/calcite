use calcite_core::{DiagnosticKind, calculate, calculate_document};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn scalar_functions_preserve_exact_arithmetic_where_possible() {
    assert_eq!(output("sqrt(9)"), "3");
    assert_eq!(output("square root of 2"), "1.4142135624");
    assert_eq!(output("abs(-3 / 4)"), "0.75");
    assert_eq!(output("absolute value of -25%"), "25%");
    assert_eq!(output("sum(0.1,0.2,0.3)"), "0.6");
    assert_eq!(output("average of 1,2,6"), "3");
}

#[test]
fn aggregates_reuse_typed_arithmetic() {
    assert_eq!(output("average(1 km, 500 m)"), "0.75 km");
    assert_eq!(output("sum($10, $20, $30)"), "60 USD");
    assert_eq!(output("average(10%, 20%, 30%)"), "20%");
    assert_eq!(output("abs(-5 km)"), "5 km");
}

#[test]
fn minimum_and_maximum_return_the_selected_typed_value() {
    assert_eq!(output("minimum(2 h, 90 min)"), "90 min");
    assert_eq!(output("maximum of 500 m, 1 km"), "1 km");
    assert_eq!(output("max(EUR 10, EUR 25, EUR 20)"), "25 EUR");
}

#[test]
fn function_arguments_can_reference_document_variables() {
    let document =
        calculate_document("first = 10 km\nlater = 500 m\naverage(first, later)").unwrap();
    assert_eq!(document.lines[2].output, "5.25 km");
}

#[test]
fn rounding_is_exact_and_uses_half_away_from_zero() {
    assert_eq!(output("round(2.5)"), "3");
    assert_eq!(output("round(-2.5)"), "-3");
    assert_eq!(output("round(1.2345, 2)"), "1.23");
    assert_eq!(output("round(1234, -2)"), "1200");
    assert_eq!(output("round(sqrt(2), 4)"), "1.4142");
}

#[test]
fn directional_rounding_handles_negative_values_correctly() {
    assert_eq!(output("floor(-1.2)"), "-2");
    assert_eq!(output("ceiling(-1.2)"), "-1");
    assert_eq!(output("floor of 1.9"), "1");
    assert_eq!(output("round 4.6"), "5");
}

#[test]
fn typed_rounding_uses_the_displayed_magnitude() {
    assert_eq!(output("round($12.345, 2)"), "12.35 USD");
    assert_eq!(output("round(12.65%, 1)"), "12.7%");
    assert_eq!(output("round(1234.56 m, -2)"), "1200 m");
    assert_eq!(output("ceiling(1.2 km / h)"), "2 km/h");
}

#[test]
fn invalid_function_domains_and_arities_are_diagnostic() {
    assert_eq!(
        calculate("sqrt(-1)").unwrap_err().kind(),
        DiagnosticKind::NonRealResult
    );
    assert_eq!(
        calculate("sqrt(1, 2)").unwrap_err().kind(),
        DiagnosticKind::InvalidFunctionArguments
    );
    assert_eq!(
        calculate("average()").unwrap_err().kind(),
        DiagnosticKind::InvalidFunctionArguments
    );
    assert_eq!(
        calculate("sum(1 USD, 1 EUR)").unwrap_err().kind(),
        DiagnosticKind::CurrencyMismatch
    );
    assert_eq!(
        calculate("round(1.2, 1.5)").unwrap_err().kind(),
        DiagnosticKind::InvalidFunctionArguments
    );
    assert_eq!(
        calculate("round(1.2, 10001)").unwrap_err().kind(),
        DiagnosticKind::PrecisionTooLarge
    );
}
