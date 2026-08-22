use calcite_core::{DiagnosticKind, calculate, calculate_document};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn factorial_and_combinatorics_are_exact_and_bounded() {
    assert_eq!(output("0!"), "1");
    assert_eq!(output("20!"), "2432902008176640000");
    assert_eq!(output("factorial of 6"), "720");
    assert_eq!(output("2 ^ 3!"), "64");
    assert_eq!(output("-3!"), "-6");
    assert_eq!(output("10 permutation 3"), "720");
    assert_eq!(output("3 permutations of 10"), "720");
    assert_eq!(output("permutation(10, 3)"), "720");
    assert_eq!(output("25 combination 3"), "2300");
    assert_eq!(output("3 combinations of 25"), "2300");
    assert_eq!(output("choose(100, 97)"), "161700");
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
fn median_sorts_compatible_typed_values() {
    assert_eq!(output("median(9, 1, 5)"), "5");
    assert_eq!(output("median of 1, 2, 10, 20"), "6");
    assert_eq!(output("median(1 km, 500 m, 2 km)"), "1 km");
    assert_eq!(output("median(1 km, 500 m)"), "0.75 km");
    assert_eq!(output("median(10%, 30%, 20%, 40%)"), "25%");
}

#[test]
fn count_accepts_mixed_values_and_product_reuses_typed_multiplication() {
    assert_eq!(output("count()"), "0");
    assert_eq!(output("count(1, true, 2 km, EUR 3)"), "4");
    assert_eq!(output("product(2, 3, 4)"), "24");
    assert_eq!(output("product of 3, 2 km"), "6 km");
    assert_eq!(output("product(EUR 5, 4)"), "20 EUR");
}

#[test]
fn function_arguments_can_reference_document_variables() {
    let document = calculate_document(
        "first = 10 km\nlater = 500 m\naverage(first, later)\nmedian(first, later)",
    )
    .unwrap();
    assert_eq!(document.lines[2].output, "5.25 km");
    assert_eq!(document.lines[3].output, "5.25 km");
}

#[test]
fn mathematical_constants_share_the_configured_precision() {
    assert_eq!(output("round(pi, 10)"), "3.1415926536");
    assert_eq!(output("round(tau / 2, 10)"), "3.1415926536");
    assert_eq!(output("round(e, 10)"), "2.7182818285");
    assert_eq!(output("round(sin(pi / 2), 10)"), "1");
    assert_eq!(output("round(ln(e), 10)"), "1");
}

#[test]
fn exponential_and_hyperbolic_functions_are_precision_aware() {
    assert_eq!(output("round(exp(1), 10)"), "2.7182818285");
    assert_eq!(output("hyperbolic sine of 0"), "0");
    assert_eq!(output("cosh(0)"), "1");
    assert_eq!(output("tanh(0)"), "0");
    assert_eq!(output("round(asinh(1), 6)"), "0.881374");
    assert_eq!(output("round(acosh(2), 6)"), "1.316958");
    assert_eq!(output("round(atanh(0.5), 6)"), "0.549306");
}

#[test]
fn constants_and_exponentials_work_in_documents() {
    let document =
        calculate_document("radius = 2\narea = pi * radius ^ 2\nround(area, 6)").unwrap();
    assert_eq!(document.lines[2].output, "12.566371");
}

#[test]
fn trigonometry_requires_explicit_angle_semantics() {
    assert_eq!(output("sin(90 deg)"), "1");
    assert_eq!(output("cosine of 180 degrees"), "-1");
    assert_eq!(output("tan(45 deg)"), "1");
    assert_eq!(output("round(sin(1.5707963267948966), 8)"), "1");
}

#[test]
fn inverse_trigonometry_returns_convertible_angles() {
    assert_eq!(output("asin(1) in deg"), "90 deg");
    assert_eq!(output("acos(0) in deg"), "90 deg");
    assert_eq!(output("atan2(1, -1) in deg"), "135 deg");
    assert_eq!(output("180 deg in rad"), "3.1415926536 rad");
    assert_eq!(output("round(1 rad in deg, 6)"), "57.29578 deg");
}

#[test]
fn trigonometry_uses_document_angle_values() {
    let document = calculate_document("heading = 30 deg\nsine of heading").unwrap();
    assert_eq!(document.lines[1].output, "0.5");
}

#[test]
fn roots_support_explicit_degrees_and_natural_phrases() {
    assert_eq!(output("cube root of -8"), "-2");
    assert_eq!(output("root(81, 4)"), "3");
    assert_eq!(output("cbrt(125)"), "5");
    assert_eq!(output("round(root(2, 3), 6)"), "1.259921");
}

#[test]
fn logarithms_have_explicit_and_predictable_bases() {
    assert_eq!(output("ln(1)"), "0");
    assert_eq!(output("log(1000)"), "3");
    assert_eq!(output("log2(1024)"), "10");
    assert_eq!(output("log(81, 3)"), "4");
    assert_eq!(output("round(ln(2), 6)"), "0.693147");
    assert_eq!(output("natural logarithm of 1"), "0");
    assert_eq!(output("binary log of 32"), "5");
}

#[test]
fn roots_and_logs_use_document_variables() {
    let document = calculate_document("value = 625\nindex = 4\nroot(value, index)").unwrap();
    assert_eq!(document.lines[2].output, "5");
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
    for source in ["average()", "median()", "product()"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::InvalidFunctionArguments
        );
    }
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
    assert_eq!(
        calculate("root(-16, 2)").unwrap_err().kind(),
        DiagnosticKind::NonRealResult
    );
    assert_eq!(
        calculate("root(2, 0)").unwrap_err().kind(),
        DiagnosticKind::InvalidFunctionArguments
    );
    assert_eq!(
        calculate("root(2, 1001)").unwrap_err().kind(),
        DiagnosticKind::RootDegreeTooLarge
    );
    for source in ["log(0)", "log(-1)", "log(10, 1)", "ln(-2)"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::InvalidLogarithm
        );
    }
    for source in ["asin(2)", "acos(-2)", "atan2(0, 0)", "tan(90 deg)"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::InvalidTrigonometry
        );
    }
    for source in ["acosh(0)", "atanh(1)", "atanh(-1)"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::InvalidHyperbolic
        );
    }
    assert_eq!(
        calculate("exp(1001)").unwrap_err().kind(),
        DiagnosticKind::ExponentTooLarge
    );
    for source in ["(-1)!", "5 combination 6", "permutation(5, -1)"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::InvalidCombinatorics
        );
    }
    for source in ["5.5!", "combination(5, 2.5)"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::IntegerRequired
        );
    }
    for source in ["10001!", "100000 permutation 10001"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::CombinatoricsTooLarge
        );
    }
}
