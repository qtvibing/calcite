use calcite_core::{
    CalculationContext, ClockTime, Date, DiagnosticKind, OffsetDateTime, TemporalContext,
    UtcOffset, WorkdayCalendar, calculate, calculate_with_context,
};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

fn context_at(
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    offset_hours: i32,
) -> CalculationContext {
    CalculationContext {
        temporal: TemporalContext {
            current_datetime: Some(OffsetDateTime::new(
                Date::new(year, month, day).unwrap(),
                ClockTime::new(hour, minute, 0).unwrap(),
                UtcOffset::from_seconds(offset_hours * 3600).unwrap(),
            )),
            default_offset: UtcOffset::UTC,
            ..TemporalContext::default()
        },
        ..CalculationContext::default()
    }
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
fn weekdays_and_workdays_use_a_fixed_monday_to_friday_calendar() {
    assert_eq!(output("weekday(2025-03-08)"), "Saturday");
    assert_eq!(output("weekday on March 10, 2025"), "Monday");
    assert_eq!(output("day of the week on January 24, 1984"), "Tuesday");
    assert_eq!(output("is_workday(2025-03-08)"), "false");
    assert_eq!(output("is_workday(2025-03-10)"), "true");
}

#[test]
fn workday_arithmetic_skips_weekends_without_assuming_holidays() {
    assert_eq!(output("2025-03-07 + 1 workday"), "2025-03-10");
    assert_eq!(output("2025-03-08 + 1 business day"), "2025-03-10");
    assert_eq!(output("2025-03-10 - 1 weekday"), "2025-03-07");
    assert_eq!(output("add_workdays(2025-03-07, 5)"), "2025-03-14");
    assert_eq!(output("2 workdays * 3"), "6 workdays");
    assert_eq!(
        output("workdays_between(2025-03-10, 2025-03-17)"),
        "5 workdays"
    );
    assert_eq!(
        output("workdays_between(2025-03-17, 2025-03-10)"),
        "-5 workdays"
    );
}

#[test]
fn relative_weekdays_have_explicit_strict_semantics() {
    let context = context_at(2025, 3, 8, 14, 30, 2);
    for (source, expected) in [
        ("next Monday", "2025-03-10"),
        ("last Monday", "2025-03-03"),
        ("previous Fri", "2025-03-07"),
        ("this Monday", "2025-03-03"),
        ("this Saturday", "2025-03-08"),
        ("next Saturday", "2025-03-15"),
        ("last Saturday", "2025-03-01"),
        ("next Monday at 9:00", "2025-03-10T09:00+02:00"),
    ] {
        assert_eq!(
            calculate_with_context(source, &context).unwrap().output,
            expected
        );
    }

    let error = calculate("next Monday").unwrap_err();
    assert_eq!(error.kind(), DiagnosticKind::MissingTemporalContext);
    assert_eq!(error.span().start, 0);
    assert_eq!(error.span().end, "next Monday".len());
}

#[test]
fn injected_holidays_change_workday_rules_deterministically() {
    let christmas = Date::new(2025, 12, 25).unwrap();
    let context = CalculationContext {
        temporal: TemporalContext {
            workday_calendar: WorkdayCalendar::new([christmas]),
            ..TemporalContext::default()
        },
        ..CalculationContext::default()
    };

    assert_eq!(output("2025-12-24 + 1 workday"), "2025-12-25");
    assert_eq!(
        calculate_with_context("2025-12-24 + 1 workday", &context)
            .unwrap()
            .output,
        "2025-12-26"
    );
    assert_eq!(
        calculate_with_context("2025-12-24 + 2 workdays", &context)
            .unwrap()
            .output,
        "2025-12-29"
    );
    assert_eq!(
        calculate_with_context("workdays_between(2025-12-24, 2025-12-30)", &context)
            .unwrap()
            .output,
        "3 workdays"
    );
    assert_eq!(
        calculate_with_context("is_workday(2025-12-25)", &context)
            .unwrap()
            .output,
        "false"
    );
}

#[test]
fn workday_functions_preserve_context_and_date_time_fields() {
    let context = context_at(2025, 3, 8, 14, 30, 2);
    assert_eq!(
        calculate_with_context("weekday(today)", &context)
            .unwrap()
            .output,
        "Saturday"
    );
    assert_eq!(
        calculate_with_context("today + 1 workday", &context)
            .unwrap()
            .output,
        "2025-03-10"
    );
    assert_eq!(
        output("2025-03-07T14:30:00+02:00 + 1 workday"),
        "2025-03-10T14:30+02:00"
    );
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
fn offset_date_times_cross_days_and_convert_the_same_instant() {
    let value = calculate("2025-03-08T23:30:00+02:00 + 2 h").unwrap();
    assert_eq!(value.output, "2025-03-09T01:30+02:00");
    assert!(value.value.as_offset_datetime().is_some());

    assert_eq!(
        output("2025-03-08T14:30:00+02:00 in UTC"),
        "2025-03-08T12:30Z"
    );
    assert_eq!(
        output("2025-03-08T14:30:00+02:00 in -05:00"),
        "2025-03-08T07:30-05:00"
    );
    assert_eq!(output("2025-03-08T14:30+02:00 - 2025-03-08T12:30Z"), "0 s");
    assert_eq!(
        output("2025-03-08T14:30+02:00 == 2025-03-08T12:30Z"),
        "true"
    );
}

#[test]
fn dates_and_times_compose_using_the_context_offset() {
    let context = CalculationContext {
        temporal: TemporalContext {
            current_datetime: None,
            default_offset: UtcOffset::from_seconds(5 * 3600 + 30 * 60).unwrap(),
            ..TemporalContext::default()
        },
        ..CalculationContext::default()
    };
    assert_eq!(
        calculate_with_context("March 8, 2025 at 9:30 pm", &context)
            .unwrap()
            .output,
        "2025-03-08T21:30+05:30"
    );
}

#[test]
fn relative_temporal_words_require_injected_context() {
    let context = CalculationContext {
        temporal: TemporalContext {
            current_datetime: Some(OffsetDateTime::new(
                Date::new(2025, 3, 8).unwrap(),
                ClockTime::new(14, 30, 0).unwrap(),
                UtcOffset::from_seconds(2 * 3600).unwrap(),
            )),
            default_offset: UtcOffset::UTC,
            ..TemporalContext::default()
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
        "2025-03-08T16:00+02:00"
    );
    assert_eq!(
        calculate_with_context("tomorrow at 9:00", &context)
            .unwrap()
            .output,
        "2025-03-09T09:00+02:00"
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
    assert_eq!(
        calculate("2025-03-08T24:00Z").unwrap_err().kind(),
        DiagnosticKind::InvalidTime
    );
    assert_eq!(
        calculate("1.5 workdays").unwrap_err().kind(),
        DiagnosticKind::InvalidCalendarDuration
    );
    assert_eq!(output("2025 - 3 - 8"), "2014");
}
