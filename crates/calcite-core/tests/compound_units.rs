use calcite_core::{DiagnosticKind, calculate, calculate_document};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn mixed_unit_forms_sum_same_dimension_parts() {
    assert_eq!(output("6 ft 2 in in cm"), "187.96 cm");
    assert_eq!(output("6 ft 2 in"), "6.1666666667 ft");
    assert_eq!(output("2 lb 3 oz in kg"), "0.9922333094 kg");
    assert_eq!(output("1 hr 30 min"), "1.5 h");
    assert_eq!(output("1 h 45 min 30 s in s"), "6330 s");
    assert_eq!(output("3 kg 500 g in lb"), "7.7161791765 lb");
}

#[test]
fn unit_products_powers_and_cancellation_are_structural() {
    assert_eq!(output("2 m * 3 m"), "6 m²");
    assert_eq!(output("2 m ^ 3"), "2 m³");
    assert_eq!(output("(2 m) ^ 3"), "8 m³");
    assert_eq!(output("2 m ^ 0"), "2");
    assert_eq!(output("(2 m) ^ 0"), "1");
    assert_eq!(output("2 m ^ -1"), "2/m");
    assert_eq!(output("(2 m) ^ -1"), "0.5/m");
    assert_eq!(output("6 m² / 2 m"), "3 m");
    assert_eq!(output("2 m * 3 cm"), "6 m·cm");
    assert_eq!(output("2 m * 3 m + 1 m²"), "7 m²");
    assert_eq!(output("2 m * 3 cm == 0.06 m²"), "true");
}

#[test]
fn compound_conversions_account_for_every_factor_scale() {
    assert_eq!(output("(2 m * 3 cm) in cm^2"), "600 cm²");
    assert_eq!(output("1 m³ in liters"), "1000 L");
    assert_eq!(output("10 m/s^2 in km/h^2"), "129600 km/h²");
    assert_eq!(output("1 kg*m/s^2 in g*cm/s^2"), "100000 cm·g/s²");
    assert_eq!(output("1 / 2 m in m^-1"), "0.5/m");
}

#[test]
fn derived_rates_multiply_divide_and_cancel() {
    assert_eq!(output("90 km / 3 h"), "30 km/h");
    assert_eq!(output("30 km/h * 3 h"), "90 km");
    assert_eq!(output("30 km/h * 180 s"), "1500 m");
    assert_eq!(output("10 m/s / 2 s"), "5 m/s²");
    assert_eq!(output("2 m/s * 3 kg/s"), "6 m·kg/s²");
    assert_eq!(output("1 / (2 m/s)"), "0.5 s/m");
    assert_eq!(output("30 km/h in m/s"), "8.3333333333 m/s");
    assert_eq!(output("3 hours / 3 days as hour/day"), "1 h/day");
}

#[test]
fn affine_temperatures_and_unit_exponents_remain_bounded() {
    assert_eq!(
        calculate("2 C * 3 m").unwrap_err().kind(),
        DiagnosticKind::UnsupportedOperation
    );
    assert_eq!(output("2 K * 3 m"), "6 m·K");
    for source in ["2 m ^ 1.5", "2 m ^ 101"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::InvalidUnitExponent
        );
    }
}

#[test]
fn compound_units_flow_through_documents() {
    let document = calculate_document(
        "width = 2 m\nheight = 30 cm\narea = width * height\narea in cm²\ntime = 4 s\narea / time^2",
    )
    .unwrap();
    assert_eq!(document.lines[2].output, "60 m·cm");
    assert_eq!(document.lines[3].output, "6000 cm²");
    assert_eq!(document.lines[5].output, "3.75 m·cm/s²");
}
