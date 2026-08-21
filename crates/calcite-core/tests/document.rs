use calcite_core::{DiagnosticKind, calculate_document};

fn outputs(source: &str) -> Vec<String> {
    calculate_document(source)
        .unwrap()
        .lines
        .into_iter()
        .map(|line| line.output)
        .collect()
}

#[test]
fn variables_retain_typed_values_in_source_order() {
    let document = calculate_document(
        "principal = 1000\n\
         growth = 10%\n\
         distance = 120 km\n\
         time = 2 h\n\
         principal + growth\n\
         distance / time",
    )
    .unwrap();

    assert_eq!(
        document
            .lines
            .iter()
            .map(|line| line.output.as_str())
            .collect::<Vec<_>>(),
        ["1000", "10%", "120 km", "2 h", "1100", "60 km/h"]
    );
    assert_eq!(document.lines[0].name.as_deref(), Some("principal"));
    assert!(document.lines[4].name.is_none());
    assert!(document.lines[5].value.as_rate().is_some());
}

#[test]
fn reassignment_only_affects_following_lines() {
    assert_eq!(
        outputs("amount = 10\namount + 1\namount = 20\namount + 1"),
        ["10", "11", "20", "21"]
    );
}

#[test]
fn blank_lines_do_not_create_results() {
    assert_eq!(outputs("\n  value = 2\n\n value * 3\n"), ["2", "6"]);
}

#[test]
fn forward_references_report_the_original_document_span() {
    let source = "total = subtotal + 1\nsubtotal = 5";
    let error = calculate_document(source).unwrap_err();
    assert_eq!(error.kind(), DiagnosticKind::UnknownVariable);
    assert_eq!(&source[error.span().start..error.span().end], "subtotal");
}

#[test]
fn reserved_language_names_cannot_create_unreachable_variables() {
    let error = calculate_document("km = 10").unwrap_err();
    assert_eq!(error.kind(), DiagnosticKind::InvalidAssignment);
    assert_eq!(
        error.detail(),
        Some("the name conflicts with calculator syntax")
    );
}
