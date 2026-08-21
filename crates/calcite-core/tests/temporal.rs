use calcite_core::{
    CalculationContext, ClockTime, Date, DiagnosticKind, TemporalContext, calculate,
    calculate_with_context,
};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn iso_dates_use_exact_gregorian_arithmetic() {
    assert_eq!(output("2024-02-29 + 1 day"), "2024-03-01");
    assert_eq!(output("2025-03-01 - 1 day"), "2025-02-28");
    assert_eq!(output("2025-03-08 - 2025-03-01"), "7 days");
    assert_eq!(output("2025-01-31 + 1 month"), "2025-02-28");
    assert_eq!(output("2024-02-29 + 1 year"), "2025-02-28");
    assert_eq!(output("March 8, 2025 + 1 day"), "2025-03-09");
    assert_eq!(output("8 March 2025 - 1 week"), "2025-03-01");
}

#[test]
fn calendar_durations_remain_distinct_from_fixed_durations() {
    let result = calculate("2 years + 6 months").unwrap();
    assert_eq!(result.output, "30 months");
    assert_eq!(
        result.value.as_calendar_duration().unwrap().total_months(),
        30
    );
    assert_eq!(output("2 * 3 months"), "6 months");
}

#[test]
fn clock_times_accept_24_hour_and_meridiem_forms() {
    assert_eq!(output("09:30 + 2 h"), "11:30");
    assert_eq!(output("9:30 pm + 2 h"), "23:30");
    assert_eq!(output("12:30 am - 45 min"), "23:45");
    assert_eq!(output("18:00 - 09:30"), "30600 s");
}

#[test]
fn relative_temporal_words_require_injected_context() {
    let context = CalculationContext {
        temporal: TemporalContext {
            today: Date::new(2025, 3, 8),
            now: ClockTime::new(14, 30, 0),
            utc_offset_seconds: 0,
        },
        ..CalculationContext::default()
    };

    assert_eq!(
        calculate_with_context("tomorrow + 1 month", &context)
            .unwrap()
            .output,
        "2025-04-09"
    );
    assert_eq!(
        calculate_with_context("now + 90 min", &context)
            .unwrap()
            .output,
        "16:00"
    );

    let missing = calculate("today").unwrap_err();
    assert_eq!(missing.kind(), DiagnosticKind::MissingTemporalContext);
}

#[test]
fn invalid_and_ambiguous_temporal_input_is_not_guessed() {
    assert_eq!(
        calculate("2025-02-29").unwrap_err().kind(),
        DiagnosticKind::InvalidDate
    );
    assert_eq!(
        calculate("24:00").unwrap_err().kind(),
        DiagnosticKind::InvalidTime
    );
    assert_eq!(output("2025 - 3 - 8"), "2014");
}
