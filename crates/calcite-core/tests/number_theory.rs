use calcite_core::{DiagnosticKind, calculate, calculate_document};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn gcd_and_lcm_are_exact_for_arbitrary_precision_integers() {
    assert_eq!(output("gcd(48, 18)"), "6");
    assert_eq!(output("gcd(-48, 18)"), "6");
    assert_eq!(output("gcd(0, 0)"), "0");
    assert_eq!(output("gcd(48, 18, 30)"), "6");
    assert_eq!(output("lcm(21, 6)"), "42");
    assert_eq!(output("lcm(-4, 6, 10)"), "60");
    assert_eq!(output("lcm(0, 999999999999999999999999999999)"), "0");
    assert_eq!(
        output("gcd(1234567891234567891234567890, 9876543219876543210)"),
        "90"
    );
}

#[test]
fn number_theory_functions_have_natural_forms() {
    assert_eq!(output("greatest common divisor of 48, 18"), "6");
    assert_eq!(output("least common multiple of 4, 6, 10"), "60");
    assert_eq!(output("is prime 97"), "true");
}

#[test]
fn primality_is_deterministic_across_the_unsigned_64_bit_range() {
    for value in ["-7", "0", "1", "4", "221", "561", "18446744073709551615"] {
        assert_eq!(output(&format!("is_prime({value})")), "false");
    }
    for value in [
        "2",
        "3",
        "97",
        "2305843009213693951",
        "18446744073709551557",
    ] {
        assert_eq!(output(&format!("is_prime({value})")), "true");
    }
}

#[test]
fn number_theory_rejects_non_integers_and_bounds_primality() {
    assert_eq!(
        calculate("gcd(1.5, 2)").unwrap_err().kind(),
        DiagnosticKind::IntegerRequired
    );
    assert_eq!(
        calculate("is_prime(2.5)").unwrap_err().kind(),
        DiagnosticKind::IntegerRequired
    );
    assert_eq!(
        calculate("is_prime(18446744073709551616)")
            .unwrap_err()
            .kind(),
        DiagnosticKind::PrimalityInputTooLarge
    );
    assert_eq!(
        calculate("gcd(1)").unwrap_err().kind(),
        DiagnosticKind::InvalidFunctionArguments
    );
}

#[test]
fn number_theory_values_flow_through_documents() {
    let document = calculate_document("a = gcd(84, 30)\nb = lcm(a, 14)\nis_prime(b + 1)").unwrap();
    assert_eq!(document.lines[0].output, "6");
    assert_eq!(document.lines[1].output, "42");
    assert_eq!(document.lines[2].output, "true");
}
