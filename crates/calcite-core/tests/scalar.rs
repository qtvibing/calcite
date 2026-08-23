use calcite_core::{
    CalculationContext, DiagnosticKind, NumberFormat, NumberOutputMode, calculate,
    calculate_with_context,
};

fn output(expression: &str) -> String {
    calculate(expression).unwrap().output
}

#[test]
fn symbolic_arithmetic_obeys_precedence() {
    assert_eq!(output("2 + 3 * 4"), "14");
    assert_eq!(output("(2 + 3) * 4"), "20");
    assert_eq!(output("20 / 5 / 2"), "2");
}

#[test]
fn double_asterisk_is_right_associative_power() {
    assert_eq!(output("2 ** 3"), "8");
    assert_eq!(output("2 ** 3 ** 2"), "512");
    assert_eq!(output("-2 ** 2"), "-4");
    assert_eq!(output("2 * 3 ** 2"), "18");
}

#[test]
fn spaced_percent_is_remainder_while_attached_percent_is_percentage() {
    assert_eq!(output("17 % 5"), "2");
    assert_eq!(output("-17 % 5"), "-2");
    assert_eq!(output("17 % -5"), "2");
    assert_eq!(output("10%"), "10%");
    assert_eq!(output("100 + 10%"), "110");
    assert_eq!(output("100 + 10 %"), "110");

    assert_eq!(
        calculate("5.5 % 2").unwrap_err().kind(),
        DiagnosticKind::IntegerRequired
    );
    assert_eq!(
        calculate("5 % 0").unwrap_err().kind(),
        DiagnosticKind::DivisionByZero
    );
}

#[test]
fn exact_integer_modulo_and_remainder_have_distinct_sign_rules() {
    assert_eq!(output("17 mod 5"), "2");
    assert_eq!(output("17 modulo 5"), "2");
    assert_eq!(output("-17 mod 5"), "3");
    assert_eq!(output("17 mod -5"), "2");
    assert_eq!(output("-17 remainder 5"), "-2");
    assert_eq!(output("17 remainder -5"), "2");
    assert_eq!(output("remainder of 21 divided by 5"), "1");
    assert_eq!(output("2 + 10 mod 4 * 3"), "8");
    assert_eq!(output("123456789012345678901234567890 mod 97"), "52");
}

#[test]
fn integer_operators_reject_zero_and_non_integer_operands() {
    assert_eq!(
        calculate("5 mod 0").unwrap_err().kind(),
        DiagnosticKind::DivisionByZero
    );
    for source in ["5.5 mod 2", "5 remainder 2.5", "sqrt(4) mod 2"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::IntegerRequired
        );
    }
}

#[test]
fn common_operator_phrases_are_supported() {
    assert_eq!(output("30 plus 20"), "50");
    assert_eq!(output("1000 divided by 200"), "5");
    assert_eq!(output("3 multiplied by 4"), "12");
    assert_eq!(output("3 to the power of 2"), "9");
}

#[test]
fn power_has_conventional_binding_and_associativity() {
    assert_eq!(output("-2 ^ 2"), "-4");
    assert_eq!(output("2 ^ -3"), "0.125");
    assert_eq!(output("2 ^ 3 ^ 2"), "512");
    assert_eq!(output("(-2) ^ 2"), "4");
}

#[test]
fn number_words_are_case_insensitive_and_bounded() {
    assert_eq!(output("twenty-one + 4"), "25");
    assert_eq!(output("One Hundred and Five * 2"), "210");
    assert_eq!(output("two million three hundred thousand five"), "2300005");
    assert_eq!(output("one hundred kilometers in meters"), "100000 m");
    assert_eq!(
        calculate("twenty thirteen").unwrap_err().kind(),
        DiagnosticKind::InvalidNumber
    );
}

#[test]
fn implicit_multiplication_supports_constants_functions_and_groups() {
    assert_eq!(output("2(3 + 4)"), "14");
    assert_eq!(output("(2 + 1)(4 - 1)"), "9");
    assert_eq!(output("2pi / pi"), "2");
    assert_eq!(output("2π / π"), "2");
    assert_eq!(output("3sqrt(9)"), "9");
    assert_eq!(output("2³"), "8");
    assert_eq!(output("2⁻²"), "0.25");
    assert_eq!(output("3² + 4²"), "25");
}

#[test]
fn number_output_modes_preserve_exactness_and_mark_approximations() {
    let mut context = CalculationContext::default();
    context.output_format.number_mode = NumberOutputMode::Exact;
    assert_eq!(
        calculate_with_context("1 / 3", &context).unwrap().output,
        "1/3"
    );

    context.output_format.number_mode = NumberOutputMode::Mixed;
    assert_eq!(
        calculate_with_context("7 / 3", &context).unwrap().output,
        "2 1/3"
    );

    context.output_format.number_mode = NumberOutputMode::Decimal { places: 4 };
    context.output_format.mark_approximate = true;
    assert_eq!(
        calculate_with_context("1 / 3", &context).unwrap().output,
        "≈0.3333"
    );

    context.output_format.number_mode = NumberOutputMode::Significant { digits: 3 };
    assert_eq!(
        calculate_with_context("12345.67", &context).unwrap().output,
        "≈12300"
    );

    context.output_format.number_mode = NumberOutputMode::Scientific { digits: 4 };
    assert_eq!(
        calculate_with_context("12345.67", &context).unwrap().output,
        "≈1.235e4"
    );
}

#[test]
fn mixed_number_literals_are_exact_fractions() {
    assert_eq!(output("1 1/2"), "1.5");
    assert_eq!(output("-2 3/4"), "-2.75");
    assert_eq!(output("1 1/2 + 2 1/4"), "3.75");
    assert_eq!(output("1 1/2 km in m"), "1500 m");
    assert_eq!(
        calculate("1 3/2").unwrap_err().kind(),
        DiagnosticKind::InvalidNumber
    );
}

#[test]
fn ordinary_decimal_and_fraction_arithmetic_remains_exact() {
    let decimal = calculate("0.1 + 0.2").unwrap();
    assert_eq!(decimal.output, "0.3");
    assert_eq!(
        decimal
            .value
            .as_number()
            .unwrap()
            .exact_fraction()
            .as_deref(),
        Some("3/10")
    );

    let fraction = calculate("1 / 3 * 3").unwrap();
    assert_eq!(fraction.output, "1");
    assert_eq!(
        fraction
            .value
            .as_number()
            .unwrap()
            .exact_fraction()
            .as_deref(),
        Some("1")
    );
}

#[test]
fn recurring_decimal_literals_are_exact() {
    assert_eq!(output("0.(3) as fraction"), "1/3");
    assert_eq!(output("0.0(15) as fraction"), "1/66");
    assert_eq!(output("12.34(56) as fraction"), "61111/4950");
    assert_eq!(output("0.(9)"), "1");

    let comma_context = CalculationContext {
        number_format: NumberFormat::decimal_comma(),
        ..CalculationContext::default()
    };
    assert_eq!(
        calculate_with_context("0,(3) as fraction", &comma_context)
            .unwrap()
            .output,
        "1/3"
    );
}

#[test]
fn fractional_powers_are_precision_aware_reals() {
    let calculation = calculate("2 ^ 0.5").unwrap();
    assert_eq!(calculation.output, "1.4142135624");
    assert!(!calculation.value.as_number().unwrap().is_exact());
}

#[test]
fn unicode_operator_symbols_are_accepted() {
    assert_eq!(output("6 × 7"), "42");
    assert_eq!(output("84 ÷ 2"), "42");
    assert_eq!(output("50 − 8"), "42");
}

#[test]
fn percentages_retain_their_meaning() {
    let sum = calculate("10% + 20%").unwrap();
    assert_eq!(sum.output, "30%");
    assert_eq!(
        sum.value
            .as_percentage()
            .unwrap()
            .ratio()
            .exact_fraction()
            .as_deref(),
        Some("3/10")
    );

    assert_eq!(output("50% * 50%"), "25%");
    assert_eq!(output("50% / 25%"), "2");
    assert_eq!(output("30% + 0.4"), "70%");
}

#[test]
fn percentages_can_modify_or_measure_a_base_value() {
    assert_eq!(output("10% of 200"), "20");
    assert_eq!(output("10% on 200"), "220");
    assert_eq!(output("10% off 200"), "180");
    assert_eq!(output("200 + 10%"), "220");
    assert_eq!(output("200 - 10%"), "180");
    assert_eq!(output("30 * 50%"), "15");
}

#[test]
fn comparisons_and_boolean_logic_are_typed() {
    let comparison = calculate("20 > 10").unwrap();
    assert_eq!(comparison.output, "true");
    assert_eq!(comparison.value.as_boolean(), Some(true));
}

#[test]
fn logical_operators_have_conventional_precedence_and_short_circuit() {
    assert_eq!(output("24 >= 18.5 and 24 < 25"), "true");
    assert_eq!(output("not 2 == 3"), "true");
    assert_eq!(output("true || 1 / 0 == 1"), "true");
    assert_eq!(output("false && 1 / 0 == 1"), "false");
    assert_eq!(output("10% equal to 0.1"), "true");
    assert_eq!(output("5 at least 5"), "true");
}

#[test]
fn conditionals_only_evaluate_the_selected_branch() {
    assert_eq!(output("if 2 > 1 then 10% of 200 else 1 / 0"), "20");
    assert_eq!(output("if false then 1 / 0 else 42"), "42");
    assert_eq!(output("unless 2 == 3 then 10 else 20"), "10");
}

#[test]
fn explicit_representation_conversions_preserve_semantics() {
    let percentage = calculate("0.35 as %").unwrap();
    assert_eq!(percentage.output, "35%");
    assert!(percentage.value.as_percentage().is_some());

    assert_eq!(output("2 / 5 as percent"), "40%");
    assert_eq!(output("2 / 10 as fraction"), "1/5");
    assert_eq!(output("50% as fraction"), "1/2");
    assert_eq!(output("20% as decimal"), "0.2");
    assert_eq!(output("(0.5 as fraction) + 0.25"), "3/4");
}

#[test]
fn explicit_number_conversion_strips_typed_display_magnitudes() {
    assert_eq!(output("$100 as number"), "100");
    assert_eq!(output("40/day to decimal"), "40");
    assert_eq!(output("90 km/h as dec"), "90");
    assert_eq!(output("1 km as number"), "1");
    assert_eq!(output("32 F as number"), "32");
    assert_eq!(output("20% as number"), "0.2");

    for source in ["true as number", "2025-01-01 as number"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::UnsupportedOperation
        );
    }
}

#[test]
fn ordinal_suffixes_do_not_change_numeric_values() {
    assert_eq!(output("1st + 2ND + 3rd + 4TH"), "10");
    assert_eq!(output("21st day in hours"), "504 h");
}

#[test]
fn large_number_suffixes_and_words_are_exact() {
    assert_eq!(output("2.5k"), "2500");
    assert_eq!(output("5 million"), "5000000");
    assert_eq!(output("2B"), "2000000000");
    assert_eq!(output("2b"), "2000000000");
    assert_eq!(output("5M"), output("5m"));
    assert_eq!(output("3 million + 10%"), "3300000");
}

#[test]
fn subsecond_duration_units_convert_exactly() {
    assert_eq!(output("1 second in milliseconds"), "1000 ms");
    assert_eq!(output("1 ms in microseconds"), "1000 μs");
    assert_eq!(output("1 μs in nanoseconds"), "1000 ns");
    assert_eq!(output("500 us + 500 US"), "1000 μs");
}

#[test]
fn compatible_units_convert_exactly_during_arithmetic() {
    assert_eq!(output("1 km + 500 m"), "1.5 km");
    assert_eq!(output("500 m + 1 km"), "1500 m");
    assert_eq!(output("300 + 20 km"), "320 km");
    assert_eq!(output("1 mile in km"), "1.609344 km");
    assert_eq!(output("1 km / 250 m"), "4");
    assert_eq!(output("1 km > 999 m"), "true");
    assert_eq!(output("10% of 50 km"), "5 km");
    assert_eq!(output("50 km + 10%"), "55 km");
}

#[test]
fn rates_form_convert_and_apply_as_typed_values() {
    let rate = calculate("90 km / 3 h").unwrap();
    assert_eq!(rate.output, "30 km/h");
    assert!(rate.value.as_rate().is_some());

    assert_eq!(output("30 km per h"), "30 km/h");
    assert_eq!(output("30 / day * 2 weeks"), "420");
    assert_eq!(output("90 km / 3 h * 2 h"), "60 km");
    assert_eq!(output("60 km / h in m / s"), "16.6666666667 m/s");
    assert_eq!(output("60 km / h + 10 m / s"), "96 km/h");
    assert_eq!(output("14 / day as / week"), "98/week");
}

#[test]
fn area_and_volume_units_convert_exactly() {
    assert_eq!(output("1 acre in m2"), "4046.8564224 m²");
    assert_eq!(output("1 km² in hectares"), "100 ha");
    assert_eq!(output("1 sqft in cm2"), "929.0304 cm²");
    assert_eq!(output("1 m³ in L"), "1000 L");
    assert_eq!(output("2 L + 500 ml"), "2.5 L");
    assert_eq!(output("1 usgal in L"), "3.785411784 L");
    assert_eq!(output("1 uscup in usfloz"), "8 US fl oz");
    assert_eq!(output("1 metriccup in ml"), "250 mL");
    assert_eq!(output("10 L / 2 h"), "5 L/h");
}

#[test]
fn multiword_units_work_in_values_conversions_and_rates() {
    assert_eq!(
        output("1 square foot in square centimeters"),
        "929.0304 cm²"
    );
    assert_eq!(output("1 cubic meter in liters"), "1000 L");
    assert_eq!(output("1 US gallon in liters"), "3.785411784 L");
    assert_eq!(output("1 metric cup in milliliters"), "250 mL");
    assert_eq!(output("10 US gallons / 2 hours"), "5 US gal/h");
    assert_eq!(
        output("60 L / h in US gal / min"),
        "0.2641720524 US gal/min"
    );
    assert_eq!(output("1 sq mile in acres"), "640 acres");
    assert_eq!(output("2 US cups"), "2 US cups");
    assert_eq!(output("2 metric cups"), "2 metric cups");

    let formatted = output("1 uscup in usfloz");
    assert_eq!(output(&formatted), formatted);
}

#[test]
fn affine_temperature_units_convert_without_losing_precision() {
    assert_eq!(output("32 F to C"), "0 °C");
    assert_eq!(output("32 f to c"), "0 °C");
    assert_eq!(output("0 C in F"), "32 °F");
    assert_eq!(output("273.15 K in C"), "0 °C");
}

#[test]
fn number_punctuation_is_explicitly_contextual() {
    assert_eq!(output("1,234.5 + 0.5"), "1235");

    let context = CalculationContext {
        number_format: NumberFormat::decimal_comma(),
        ..CalculationContext::default()
    };
    assert_eq!(
        calculate_with_context("1.234,5 + 0,75", &context)
            .unwrap()
            .output,
        "1235,25"
    );

    let malformed = calculate("12,34 + 1").unwrap_err();
    assert_eq!(malformed.kind(), DiagnosticKind::InvalidNumber);
}

#[test]
fn incompatible_units_report_a_dimension_error() {
    let error = calculate("1 km + 1 kg").unwrap_err();
    assert_eq!(error.kind(), DiagnosticKind::UnitMismatch);
    assert_eq!(&"1 km + 1 kg"[error.span().start..error.span().end], "+");

    let error = calculate("1 m² + 1 m").unwrap_err();
    assert_eq!(error.kind(), DiagnosticKind::UnitMismatch);
}

#[test]
fn calculation_errors_are_structured_and_spanned() {
    let division = calculate("1 / 0").unwrap_err();
    assert_eq!(division.kind(), DiagnosticKind::DivisionByZero);
    assert_eq!(&"1 / 0"[division.span().start..division.span().end], "/");

    let unknown = calculate("2 bananas 3").unwrap_err();
    assert_eq!(unknown.kind(), DiagnosticKind::UnexpectedToken);
    assert_eq!(
        &"2 bananas 3"[unknown.span().start..unknown.span().end],
        "bananas"
    );
}
