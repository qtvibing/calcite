use calcite_core::{
    CalculationContext, ClockTime, Date, DateOrder, DiagnosticKind, LEAP_SECOND_POLICY,
    LeapSecondPolicy, OffsetDateTime, TemporalContext, UtcOffset, WorkdayCalendar, calculate,
    calculate_with_context,
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
fn numeric_dates_require_an_explicit_date_order() {
    let mut context = CalculationContext::default();
    context.temporal.date_order = Some(DateOrder::MonthDayYear);
    assert_eq!(
        calculate_with_context("3/8/2025", &context).unwrap().output,
        "2025-03-08"
    );
    assert_eq!(
        calculate_with_context("3.8.2025", &context).unwrap().output,
        "2025-03-08"
    );

    context.temporal.date_order = Some(DateOrder::DayMonthYear);
    assert_eq!(
        calculate_with_context("3/8/2025", &context).unwrap().output,
        "2025-08-03"
    );
    assert_eq!(output("3/8/2025"), "0.0001851852");
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
    assert_eq!(output("Aug 1st, 2025 + 1 day"), "2025-08-02");
    assert_eq!(output("2ND jun 2025 - 1 day"), "2025-06-01");
    assert_eq!(output("Jan 3rd 2025"), "2025-01-03");
    assert_eq!(output("4th SEPT 2025"), "2025-09-04");
    assert_eq!(output("11th Feb 2025"), "2025-02-11");

    let context = context_at(2025, 3, 8, 14, 30, 2);
    assert_eq!(
        calculate_with_context("Aug 5th", &context).unwrap().output,
        "2025-08-05"
    );
    assert_eq!(
        calculate("Aug 5th").unwrap_err().kind(),
        DiagnosticKind::MissingTemporalContext
    );
}

#[test]
fn natural_time_differences_work_for_dates_times_and_contextual_now() {
    assert_eq!(output("time between 2025-01-10 and 2025-01-01"), "9 days");
    assert_eq!(
        output("time between 2025-01-10T12:30:01.001Z and 2025-01-01T10:00:00Z"),
        "9 days 2 h 30 min 1 s 1 ms"
    );
    assert_eq!(output("(time between 10:00 and 12:30) + 30 minutes"), "3 h");
    assert_eq!(output("DAYS BETWEEN 2025-01-01 AND 2025-01-10"), "9 days");
    assert_eq!(output("duration between 12:30 and 10:00"), "2 h 30 min");
    assert_eq!(output("time from 10:00 to 12:30"), "2 h 30 min");
    assert_eq!(output("how long between 10:00 and 12:30"), "2 h 30 min");
    assert_eq!(output("hours between 2025-01-01 and 2025-01-03"), "48 h");
    assert_eq!(output("minutes between 12:30 and 10:00"), "150 min");
    assert_eq!(
        output("microseconds between 12:00:01 and 12:00:00"),
        "1000000 μs"
    );
    assert_eq!(
        output("microseconds between 12:00:00.000002 and 12:00:00.000001"),
        "1 μs"
    );
    assert_eq!(
        output("months between Jan 31st 2024 and Mar 30th 2024"),
        "1 month"
    );
    assert_eq!(
        output("years between 29th Feb 2020 and Feb 28th 2025"),
        "5 years"
    );

    let context = context_at(2025, 3, 8, 14, 30, 2);
    assert_eq!(
        calculate_with_context("time since 2025-01-01", &context)
            .unwrap()
            .output,
        "66 days"
    );
    assert_eq!(
        calculate_with_context("days until 2025-03-10", &context)
            .unwrap()
            .output,
        "2 days"
    );
    assert_eq!(
        calculate_with_context("months since 2024-12-08", &context)
            .unwrap()
            .output,
        "3 months"
    );
    assert_eq!(
        calculate_with_context("time since 2025-03-08T12:00:00+02:00", &context)
            .unwrap()
            .output,
        "2 h 30 min"
    );

    assert_eq!(
        calculate("time since 2025-01-01").unwrap_err().kind(),
        DiagnosticKind::MissingTemporalContext
    );
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
    assert_eq!(output("next workday after 2025-03-07"), "2025-03-10");
    assert_eq!(
        output("previous business day before 2025-03-10"),
        "2025-03-07"
    );
    assert_eq!(output("2 workdays * 3"), "6 workdays");
    assert_eq!(
        output("workdays_between(2025-03-10, 2025-03-17)"),
        "5 workdays"
    );
    assert_eq!(
        output("workdays_between(2025-03-17, 2025-03-10)"),
        "-5 workdays"
    );
    assert_eq!(
        output("workdays from March 10, 2025 to March 17, 2025"),
        "5 workdays"
    );
    assert_eq!(
        output("weekdays from 2025-03-17 to 2025-03-10"),
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
        ("next month", "2025-04-08"),
        ("last month", "2025-02-08"),
        ("next year", "2026-03-08"),
        ("previous year", "2024-03-08"),
        ("start of month", "2025-03-01"),
        ("end of month", "2025-03-31"),
        ("start of year", "2025-01-01"),
        ("end of year", "2025-12-31"),
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
    assert_eq!(
        calculate_with_context("next workday after 2025-12-24", &context)
            .unwrap()
            .output,
        "2025-12-26"
    );
    assert_eq!(
        calculate_with_context("previous workday before 2025-12-26", &context)
            .unwrap()
            .output,
        "2025-12-24"
    );
    assert_eq!(
        calculate_with_context("business days from 2025-12-24 to 2025-12-30", &context,)
            .unwrap()
            .output,
        "3 workdays"
    );
}

#[test]
fn country_holiday_calendars_enforce_their_data_boundaries() {
    let calendar = WorkdayCalendar::for_country(
        "us",
        Date::new(2025, 1, 1).unwrap(),
        Date::new(2025, 12, 31).unwrap(),
        [Date::new(2025, 12, 25).unwrap()],
    )
    .unwrap();
    assert_eq!(calendar.country_code(), Some("US"));
    assert_eq!(
        calendar.valid_range(),
        Some((
            Date::new(2025, 1, 1).unwrap(),
            Date::new(2025, 12, 31).unwrap()
        ))
    );
    let context = CalculationContext {
        temporal: TemporalContext {
            workday_calendar: calendar,
            ..TemporalContext::default()
        },
        ..CalculationContext::default()
    };
    assert_eq!(
        calculate_with_context("2025-12-24 + 1 workday", &context)
            .unwrap()
            .output,
        "2025-12-26"
    );
    assert_eq!(
        calculate_with_context("2025-12-31 + 1 workday", &context)
            .unwrap_err()
            .kind(),
        DiagnosticKind::HolidayCalendarOutOfRange
    );
    assert_eq!(
        calculate_with_context("is_workday(2026-01-02)", &context)
            .unwrap_err()
            .kind(),
        DiagnosticKind::HolidayCalendarOutOfRange
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
    assert_eq!(
        output("next workday after 2025-03-07T14:30:00+02:00"),
        "2025-03-10T14:30+02:00"
    );
}

#[test]
fn calendar_differences_retain_year_month_and_day_remainders() {
    assert_eq!(
        output("calendar duration between 2024-01-31 and 2025-03-30"),
        "1 year 1 month 30 days"
    );
    assert_eq!(
        output("calendar time from 2025-03-30 to 2024-01-31"),
        "-1 year 1 month 30 days"
    );
    assert_eq!(
        output("calendar duration between 2020-02-29 and 2025-02-28"),
        "5 years"
    );

    let context = context_at(2025, 3, 30, 12, 0, 0);
    assert_eq!(
        calculate_with_context("calendar time since 2024-01-31", &context)
            .unwrap()
            .output,
        "1 year 1 month 30 days"
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
    assert_eq!(LEAP_SECOND_POLICY, LeapSecondPolicy::Reject);
    assert_eq!(output("09:30 + 2 h"), "11:30");
    assert_eq!(output("9:30 pm + 2 h"), "23:30");
    assert_eq!(output("12:30 am - 45 min"), "23:45");
    assert_eq!(output("18:00 - 09:30"), "30600 s");
    assert_eq!(output("12:00:00.001 - 12:00:00"), "0.001 s");
    assert_eq!(output("12:00:00.001 + 1 us"), "12:00:00.001001");
    assert_eq!(output("10:00 UTC+1 in UTC"), "09:00Z");
    assert_eq!(output("time between 10:00 UTC+2 and 10:00 UTC+1"), "1 h");
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
    assert_eq!(
        output("2025-03-08 at 10:00 UTC+1"),
        "2025-03-08T10:00+01:00"
    );
    assert_eq!(
        output("2025-03-08T14:30:00.123456789+02:00 + 1 ns"),
        "2025-03-08T14:30:00.12345679+02:00"
    );
    assert_eq!(
        output("2025-03-08T14:30:00+02:00 in gMt"),
        "2025-03-08T12:30Z"
    );
    assert_eq!(
        output("2025-03-08T14:30:00+02:00 in UTC+1"),
        "2025-03-08T13:30+01:00"
    );
    assert_eq!(
        output("2025-03-08T14:30:00+02:00 in UTC+2"),
        "2025-03-08T14:30+02:00"
    );
    assert_eq!(
        output("2025-03-08T14:30:00+02:00 in uTc-3"),
        "2025-03-08T09:30-03:00"
    );
    assert_eq!(
        output("2025-03-08T14:30:00+02:00 in GMT+5:30"),
        "2025-03-08T18:00+05:30"
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
        calculate("23:59:60").unwrap_err().kind(),
        DiagnosticKind::InvalidTime
    );
    assert_eq!(
        calculate("2016-12-31T23:59:60Z").unwrap_err().kind(),
        DiagnosticKind::InvalidTime
    );
    assert_eq!(
        calculate("1.5 workdays").unwrap_err().kind(),
        DiagnosticKind::InvalidCalendarDuration
    );
    assert_eq!(
        calculate("workdays from 2025-03-10").unwrap_err().kind(),
        DiagnosticKind::UnexpectedEnd
    );
    assert_eq!(output("2025 - 3 - 8"), "2014");
}

#[test]
fn named_holidays_resolve_to_their_next_occurrence() {
    let june = context_at(2026, 6, 1, 12, 0, 0);
    assert_eq!(
        calculate_with_context("christmas", &june).unwrap().output,
        "2026-12-25"
    );
    assert_eq!(
        calculate_with_context("christmas eve", &june)
            .unwrap()
            .output,
        "2026-12-24"
    );
    assert_eq!(
        calculate_with_context("halloween", &june).unwrap().output,
        "2026-10-31"
    );
    assert_eq!(
        calculate_with_context("time until valentines day", &june)
            .unwrap()
            .output,
        "258 days"
    );

    let new_years_eve = context_at(2026, 12, 31, 23, 0, 0);
    assert_eq!(
        calculate_with_context("new years day", &new_years_eve)
            .unwrap()
            .output,
        "2027-01-01"
    );
    assert_eq!(
        calculate_with_context("time until new years eve", &new_years_eve)
            .unwrap()
            .output,
        "0 s"
    );

    let without_context = CalculationContext::default();
    assert_eq!(
        calculate_with_context("christmas", &without_context)
            .unwrap_err()
            .kind(),
        DiagnosticKind::MissingTemporalContext
    );
    // An explicit year pins the date and needs no current time.
    assert_eq!(
        calculate_with_context("christmas 2024", &without_context)
            .unwrap()
            .output,
        "2024-12-25"
    );
}

#[test]
fn last_holidays_resolve_to_their_previous_occurrence() {
    let june = context_at(2026, 6, 1, 12, 0, 0);
    assert_eq!(
        calculate_with_context("last christmas", &june)
            .unwrap()
            .output,
        "2025-12-25"
    );
    assert_eq!(
        calculate_with_context("last halloween", &june)
            .unwrap()
            .output,
        "2025-10-31"
    );
    assert_eq!(
        calculate_with_context("time since last christmas", &june)
            .unwrap()
            .output,
        "158 days"
    );
    // `since` looks backward on its own.
    assert_eq!(
        calculate_with_context("time since christmas", &june)
            .unwrap()
            .output,
        "158 days"
    );
    // An explicit year is never flipped.
    assert_eq!(
        calculate_with_context("time since christmas 2027", &june)
            .unwrap()
            .output,
        "-572 days"
    );

    let on_christmas = context_at(2026, 12, 25, 9, 0, 0);
    assert_eq!(
        calculate_with_context("christmas", &on_christmas)
            .unwrap()
            .output,
        "2026-12-25"
    );
    assert_eq!(
        calculate_with_context("last christmas", &on_christmas)
            .unwrap()
            .output,
        "2025-12-25"
    );
}
