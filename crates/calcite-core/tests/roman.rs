use calcite_core::{DiagnosticKind, RomanInteger, calculate};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn roman_numerals_are_explicit_and_canonical() {
    assert_eq!(output("2024 in roman"), "MMXXIV");
    assert_eq!(output("roman MMXXIV"), "MMXXIV");
    assert_eq!(output("roman numeral mmxxiv as decimal"), "2024");
    assert_eq!(output("roman XLIV + 6"), "50");
    assert_eq!(
        RomanInteger::parse("MCMXCIX").unwrap().to_string(),
        "MCMXCIX"
    );
}

#[test]
fn rejects_noncanonical_and_out_of_range_roman_numerals() {
    assert_eq!(
        calculate("roman IIII").unwrap_err().kind(),
        DiagnosticKind::InvalidNumber
    );
    assert_eq!(
        calculate("4000 in roman").unwrap_err().kind(),
        DiagnosticKind::IntegerRequired
    );
}
