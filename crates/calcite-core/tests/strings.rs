use calcite_core::{DiagnosticKind, calculate, calculate_document};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn string_literals_are_typed_escaped_and_comparable() {
    assert_eq!(output(r#""hello" + " world""#), "hello world");
    assert_eq!(output(r#"'line\n' + "two""#), "line\ntwo");
    assert_eq!(output(r#""a" < "b""#), "true");
    assert_eq!(output(r#""same" == 'same'"#), "true");

    let calculation = calculate(r#""typed""#).unwrap();
    assert_eq!(calculation.value.as_string(), Some("typed"));
}

#[test]
fn strings_flow_through_documents_and_report_bad_escapes() {
    let document =
        calculate_document("greeting = \"hello\"\ngreeting + \"!\"\nurl: \"https://example.test\"")
            .unwrap();
    assert_eq!(document.lines[1].output, "hello!");
    assert_eq!(document.lines[2].output, "https://example.test");

    assert_eq!(
        calculate(r#""bad\q""#).unwrap_err().kind(),
        DiagnosticKind::InvalidString
    );
    assert_eq!(
        calculate(r#""unterminated"#).unwrap_err().kind(),
        DiagnosticKind::InvalidString
    );
}
