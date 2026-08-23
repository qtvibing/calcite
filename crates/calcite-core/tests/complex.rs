use calcite_core::{ComplexNumber, Number, calculate};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn parses_and_calculates_complex_numbers() {
    assert_eq!(output("i"), "i");
    assert_eq!(output("3 + 4i"), "3 + 4i");
    assert_eq!(output("(1 + 2i) * (3 - 4i)"), "11 + 2i");
    assert_eq!(output("(3 + 4i) / (1 - 2i)"), "-1 + 2i");
    assert_eq!(output("i ^ 2"), "-1");
    assert_eq!(output("i ^ -1"), "-i");
    assert_eq!(output("sqrt(-1)"), "i");
    assert_eq!(output("sqrt(-4)"), "2i");
    assert_eq!(output("(3 + 4i) == (3 + 4i)"), "true");
}

#[test]
fn exposes_typed_complex_components() {
    let calculation = calculate("2 - 3i").unwrap();
    let value = calculation.value.as_complex().unwrap();
    assert_eq!(value.real(), &Number::parse_decimal("2").unwrap());
    assert_eq!(value.imaginary(), &Number::parse_decimal("-3").unwrap());
    assert_eq!(
        ComplexNumber::imaginary_unit(),
        ComplexNumber::new(
            Number::parse_decimal("0").unwrap(),
            Number::parse_decimal("1").unwrap()
        )
    );
}
