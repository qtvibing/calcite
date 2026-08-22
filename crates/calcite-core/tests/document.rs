use calcite_core::{
    CalculationContext, ClockTime, Date, DiagnosticKind, DocumentDependency, DocumentEntryKind,
    OffsetDateTime, TemporalContext, UtcOffset, WorkdayCalendar, calculate_document,
    calculate_document_with_context,
};

fn outputs(source: &str) -> Vec<String> {
    calculate_document(source)
        .unwrap()
        .lines
        .into_iter()
        .map(|line| line.output)
        .collect()
}

#[test]
fn natural_aggregate_lists_flow_through_variables() {
    assert_eq!(
        outputs("a = 10\nb = 20\nc = 30\nsum of a, b, and c\naverage of a and c"),
        ["10", "20", "30", "60", "20"]
    );
}

#[test]
fn typed_magnitudes_can_be_stripped_through_variables() {
    assert_eq!(
        outputs(
            "distance = 12 km\namount = $25\nspeed = 90 km/h\ndistance as number\namount as decimal\nspeed as dec"
        ),
        ["12 km", "25 USD", "90 km/h", "12", "25", "90"]
    );
}

#[test]
fn range_and_proportion_functions_flow_through_variables() {
    assert_eq!(
        outputs(
            "low = 5\nhigh = 25\nvalue = clamp 30 between low and high\n6 is to 60 as value is to what",
        ),
        ["5", "25", "25", "250"]
    );
}

#[test]
fn combinatorics_flow_through_variables() {
    assert_eq!(
        outputs(
            "population = 25\nselection = 3\npopulation combination selection\nselection permutations of population"
        ),
        ["25", "3", "2300", "13800"]
    );
}

#[test]
fn integer_operators_flow_through_variables() {
    assert_eq!(
        outputs(
            "items = 100\nbox_size = 12\nitems mod box_size\n-items remainder box_size\nitems % box_size\n2 ** 8",
        ),
        ["100", "12", "4", "-4", "4", "256"]
    );
}

#[test]
fn workday_values_flow_through_variables() {
    assert_eq!(
        outputs(
            "start = 2025-03-07\ndeadline = start + 5 workdays\nweekday(deadline)\nworkdays from start to deadline",
        ),
        ["2025-03-07", "2025-03-14", "Friday", "5 workdays"]
    );
}

#[test]
fn document_workdays_use_the_injected_holiday_calendar() {
    let context = CalculationContext {
        temporal: TemporalContext {
            workday_calendar: WorkdayCalendar::new([Date::new(2025, 12, 25).unwrap()]),
            ..TemporalContext::default()
        },
        ..CalculationContext::default()
    };
    let document = calculate_document_with_context(
        "start = 2025-12-24\ndeadline = start + 1 workday\ndeadline",
        &context,
    )
    .unwrap();
    assert_eq!(document.lines[1].output, "2025-12-26");
    assert_eq!(document.lines[2].output, "2025-12-26");
}

#[test]
fn relative_weekdays_flow_through_document_variables() {
    let context = CalculationContext {
        temporal: TemporalContext {
            current_datetime: Some(OffsetDateTime::new(
                Date::new(2025, 3, 8).unwrap(),
                ClockTime::new(12, 0, 0).unwrap(),
                UtcOffset::UTC,
            )),
            ..TemporalContext::default()
        },
        ..CalculationContext::default()
    };
    let document = calculate_document_with_context(
        "meeting = next Monday\nfollowup = meeting + 2 workdays\nweekday(followup)",
        &context,
    )
    .unwrap();
    assert_eq!(document.lines[0].output, "2025-03-10");
    assert_eq!(document.lines[1].output, "2025-03-12");
    assert_eq!(document.lines[2].output, "Wednesday");
}

#[test]
fn area_and_volume_values_flow_through_variables() {
    assert_eq!(
        outputs("plot = 1 square mile\nplot in acres\nbatch = 1 metric cup\nbatch in ml"),
        ["1 mi²", "640 acres", "1 metric cup", "250 mL"]
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
