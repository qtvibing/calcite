use calcite_core::{DiagnosticKind, DocumentDependency, DocumentEntryKind, calculate_document};

fn outputs(source: &str) -> Vec<String> {
    calculate_document(source)
        .unwrap()
        .lines
        .into_iter()
        .map(|line| line.output)
        .collect()
}

#[test]
fn area_and_volume_values_flow_through_variables() {
    assert_eq!(
        outputs("plot = 1 acre\nplot in ha\nbatch = 1 metriccup\nbatch in ml"),
        ["1 acre", "0.4046856422 ha", "1 metric cup", "250 mL"]
    );
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
fn dependencies_resolve_to_source_order_definitions() {
    let document = calculate_document(
        "value = 10\n\
         first = value + 1\n\
         value = value * 2\n\
         if true then value + first else missing\n\
         distance = 1 km\n\
         round((distance + distance) in m)",
    )
    .unwrap();

    assert!(document.lines[0].dependencies.is_empty());
    assert_eq!(
        document.lines[1].dependencies,
        [DocumentDependency {
            name: "value".to_owned(),
            source_result: Some(0),
        }]
    );
    assert_eq!(
        document.lines[2].dependencies,
        [DocumentDependency {
            name: "value".to_owned(),
            source_result: Some(0),
        }]
    );
    assert_eq!(
        document.lines[3].dependencies,
        [
            DocumentDependency {
                name: "value".to_owned(),
                source_result: Some(2),
            },
            DocumentDependency {
                name: "first".to_owned(),
                source_result: Some(1),
            },
            DocumentDependency {
                name: "missing".to_owned(),
                source_result: None,
            },
        ]
    );
    assert_eq!(
        document.lines[5].dependencies,
        [DocumentDependency {
            name: "distance".to_owned(),
            source_result: Some(4),
        }]
    );
}

#[test]
fn blank_lines_do_not_create_results() {
    assert_eq!(outputs("\n  value = 2\n\n value * 3\n"), ["2", "6"]);
}

#[test]
fn headings_comments_and_labels_preserve_document_order() {
    let source =
        "# Trip costs\n\n// prices in USD\nhotel = $200 // two nights\nFood: $50 + $25\nhotel + 75";
    let document = calculate_document(source).unwrap();

    assert_eq!(
        document
            .lines
            .iter()
            .map(|line| line.output.as_str())
            .collect::<Vec<_>>(),
        ["200 USD", "75 USD", "275 USD"]
    );
    assert_eq!(document.entries.len(), 6);
    assert!(matches!(document.entries[1].kind, DocumentEntryKind::Blank));
    assert_eq!(
        document.entries[0].kind,
        DocumentEntryKind::Heading {
            text: "Trip costs".to_owned()
        }
    );
    assert_eq!(
        document.entries[2].kind,
        DocumentEntryKind::Comment {
            text: "prices in USD".to_owned()
        }
    );
    assert_eq!(
        document.entries[3].kind,
        DocumentEntryKind::Calculation {
            result_index: 0,
            label: None,
            comment: Some("two nights".to_owned()),
        }
    );
    assert_eq!(
        document.entries[4].kind,
        DocumentEntryKind::Calculation {
            result_index: 1,
            label: Some("Food".to_owned()),
            comment: None,
        }
    );
    for entry in &document.entries {
        assert!(entry.span.end <= source.len());
        assert!(!source[entry.span.start..entry.span.end].contains('\n'));
    }
}

#[test]
fn comments_do_not_affect_variables_or_diagnostic_spans() {
    assert_eq!(
        outputs("Initial value: amount = 10 // setup\namount + 5 // result"),
        ["10", "15"]
    );

    let source = "# Heading\n// ignored\nmissing + 1 // explanation";
    let error = calculate_document(source).unwrap_err();
    assert_eq!(error.kind(), DiagnosticKind::UnknownVariable);
    assert_eq!(&source[error.span().start..error.span().end], "missing");
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
