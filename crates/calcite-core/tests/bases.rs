use calcite_core::{DiagnosticKind, Radix, calculate, calculate_document};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn prefixed_integer_literals_are_arbitrary_precision_and_canonical() {
    assert_eq!(output("0xff"), "0xff");
    assert_eq!(output("0B1010"), "0b1010");
    assert_eq!(output("0o755"), "0o755");
    assert_eq!(output("-0x2A"), "-0x2a");
    assert_eq!(
        output("0xffffffffffffffffffffffffffffffff in decimal"),
        "340282366920938463463374607431768211455"
    );

    let calculation = calculate("0x2a").unwrap();
    assert_eq!(
        calculation.value.as_radix_integer().unwrap().radix(),
        Radix::Hexadecimal
    );
}

#[test]
fn explicit_base_conversions_support_words_numbers_and_functions() {
    assert_eq!(output("256 as hex"), "0x100");
    assert_eq!(output("99 in binary"), "0b1100011");
    assert_eq!(output("0x9F31 to decimal"), "40753");
    assert_eq!(output("0b1000101 to octal"), "0o105");
    assert_eq!(output("0b101101 as base 8"), "0o55");
    assert_eq!(output("0x2D as base 2"), "0b101101");
    assert_eq!(output("99 in base 3"), "3#10200");
    assert_eq!(output("35 in base 36"), "36#z");
    assert_eq!(output("36#z + 1"), "36");
    assert_eq!(output("2#101010 in base 36"), "36#16");
    assert_eq!(output("hex(99)"), "0x63");
    assert_eq!(output("bin(0x73)"), "0b1110011");
    assert_eq!(output("int(0o55)"), "45");
}

#[test]
fn based_integers_coerce_to_exact_numbers_for_calculation() {
    assert_eq!(output("0xff + 1"), "256");
    assert_eq!(output("0xff == 255"), "true");
    assert_eq!(output("0b1010 mod 3"), "1");
    assert_eq!(output("factorial(0x5)"), "120");
}

#[test]
fn bitwise_operators_use_arbitrary_precision_signed_integers() {
    assert_eq!(output("0b1100 & 0b1010"), "8");
    assert_eq!(output("0b1100 | 0b0011"), "15");
    assert_eq!(output("0b1100 xor 0b1010"), "6");
    assert_eq!(output("0xff & 0x0f in hex"), "0xf");
    assert_eq!(output("1 << 100"), "1267650600228229401496703205376");
    assert_eq!(output("0x100 >> 4"), "16");
    assert_eq!(output("-8 >> 2"), "-2");
    assert_eq!(output("1 << 2 + 1"), "8");
    assert_eq!(output("1 + 2 << 1"), "6");
}

#[test]
fn bitwise_operators_validate_operands_and_shift_bounds() {
    assert_eq!(
        calculate("1.5 & 1").unwrap_err().kind(),
        DiagnosticKind::IntegerRequired
    );
    assert_eq!(
        calculate("1 << -1").unwrap_err().kind(),
        DiagnosticKind::InvalidShift
    );
    assert_eq!(
        calculate("1 << 100001").unwrap_err().kind(),
        DiagnosticKind::ShiftTooLarge
    );
}

#[test]
fn invalid_literals_and_non_integer_conversions_are_diagnostic() {
    for source in ["0x", "0b102", "0o9", "0xnope"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::InvalidNumber
        );
    }
    assert_eq!(
        calculate("1.5 as hex").unwrap_err().kind(),
        DiagnosticKind::IntegerRequired
    );

    let oversized = format!("0x{}", "f".repeat(10_001));
    assert_eq!(
        calculate(&oversized).unwrap_err().kind(),
        DiagnosticKind::NumberTooLarge
    );
}

#[test]
fn based_values_flow_through_document_variables() {
    let document = calculate_document(
        "mask = 0xff\nvalue = mask + 1\nvalue in binary\nflag = 1 << 8\nmask | flag",
    )
    .unwrap();
    assert_eq!(document.lines[0].output, "0xff");
    assert_eq!(document.lines[1].output, "256");
    assert_eq!(document.lines[2].output, "0b100000000");
    assert_eq!(document.lines[3].output, "256");
    assert_eq!(document.lines[4].output, "511");
}
