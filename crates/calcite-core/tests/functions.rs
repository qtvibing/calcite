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
}
