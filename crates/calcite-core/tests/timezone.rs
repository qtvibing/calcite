use calcite_core::{
    CalculationContext, CalendarDuration, ClockTime, Date, DiagnosticKind, LocalTimeResolution,
    OffsetDateTime, TimeZoneCode, TimeZoneDefinition, TimeZoneError, TimeZoneSnapshot,
    TimeZoneTransition, UtcOffset,
};

fn datetime(source: &str) -> OffsetDateTime {
    OffsetDateTime::parse_iso(source).unwrap()
}

fn new_york_2025() -> TimeZoneSnapshot {
    let standard = UtcOffset::from_hours_minutes(-5, 0).unwrap();
    let daylight = UtcOffset::from_hours_minutes(-4, 0).unwrap();
    let zone = TimeZoneDefinition::new(
        "America/New_York",
        datetime("2025-01-01T00:00Z"),
        datetime("2026-01-01T00:00Z"),
        standard,
        [
            TimeZoneTransition::new(datetime("2025-03-09T07:00Z"), daylight),
            TimeZoneTransition::new(datetime("2025-11-02T06:00Z"), standard),
        ],
    )
    .unwrap();
    TimeZoneSnapshot::new("test-2025a", [zone], []).unwrap()
}

#[test]
fn resolves_local_times_against_an_explicit_snapshot() {
    let snapshot = new_york_2025();
    let winter = snapshot
        .resolve_local(
            "america/new_york",
            Date::new(2025, 2, 1).unwrap(),
            ClockTime::new(12, 0, 0).unwrap(),
            LocalTimeResolution::Reject,
        )
        .unwrap();
    assert_eq!(
        winter.to_string(),
        "2025-02-01T12:00-05:00[America/New_York]"
    );

    let summer = snapshot
        .at_instant("AMERICA/NEW_YORK", datetime("2025-07-01T16:00Z"))
        .unwrap();
    assert_eq!(
        summer.to_string(),
        "2025-07-01T12:00-04:00[America/New_York]"
    );
    assert_eq!(snapshot.version(), "test-2025a");
}

#[test]
fn rejects_gaps_and_requires_a_policy_for_folds() {
    let snapshot = new_york_2025();
    let date = Date::new(2025, 3, 9).unwrap();
    let time = ClockTime::new(2, 30, 0).unwrap();
    assert_eq!(
        snapshot.resolve_local("America/New_York", date, time, LocalTimeResolution::Reject),
        Err(TimeZoneError::NonexistentLocalTime)
    );

    let date = Date::new(2025, 11, 2).unwrap();
    let time = ClockTime::new(1, 30, 0).unwrap();
    assert_eq!(
        snapshot.resolve_local("America/New_York", date, time, LocalTimeResolution::Reject),
        Err(TimeZoneError::AmbiguousLocalTime)
    );
    let earlier = snapshot
        .resolve_local("America/New_York", date, time, LocalTimeResolution::Earlier)
        .unwrap();
    let later = snapshot
        .resolve_local("America/New_York", date, time, LocalTimeResolution::Later)
        .unwrap();
    assert_eq!(earlier.datetime().offset().seconds(), -4 * 3600);
    assert_eq!(later.datetime().offset().seconds(), -5 * 3600);
    assert_eq!(later.datetime().seconds_since(earlier.datetime()), 3600);
}

#[test]
fn distinguishes_elapsed_time_from_calendar_arithmetic_across_dst() {
    let snapshot = new_york_2025();
    let start = snapshot
        .resolve_local(
            "America/New_York",
            Date::new(2025, 3, 8).unwrap(),
            ClockTime::new(12, 0, 0).unwrap(),
            LocalTimeResolution::Reject,
        )
        .unwrap();
    let calendar_day = start
        .add_calendar_days(&snapshot, 1, LocalTimeResolution::Reject)
        .unwrap();
    let elapsed_day = start
        .add_nanoseconds(&snapshot, 86_400_000_000_000)
        .unwrap();

    assert_eq!(
        calendar_day.to_string(),
        "2025-03-09T12:00-04:00[America/New_York]"
    );
    assert_eq!(
        elapsed_day.to_string(),
        "2025-03-09T13:00-04:00[America/New_York]"
    );
    assert_eq!(
        calendar_day.datetime().seconds_since(start.datetime()),
        23 * 3600
    );

    let next_month = start
        .add_calendar_duration(
            &snapshot,
            CalendarDuration::months(1),
            LocalTimeResolution::Reject,
        )
        .unwrap();
    assert_eq!(
        next_month.to_string(),
        "2025-04-08T12:00-04:00[America/New_York]"
    );
}

#[test]
fn evaluator_resolves_zoned_wall_times_without_ambient_timezone_data() {
    let mut context = CalculationContext::default();
    assert_eq!(
        calcite_core::calculate_with_context("2025-07-01 at 12:00 America/New_York", &context,)
            .unwrap_err()
            .kind(),
        DiagnosticKind::MissingTimeZoneData
    );

    context.temporal.time_zones = Some(new_york_2025());
    assert_eq!(
        calcite_core::calculate_with_context("2025-07-01 at 12:00 america/new_york", &context,)
            .unwrap()
            .output,
        "2025-07-01T12:00-04:00[America/New_York]"
    );
    assert_eq!(
        calcite_core::calculate_with_context(
            "(2025-03-08 at 12:00 America/New_York) + 24 hours",
            &context,
        )
        .unwrap()
        .output,
        "2025-03-09T13:00-04:00[America/New_York]"
    );
    assert_eq!(
        calcite_core::calculate_with_context("2025-07-01T16:00Z in America/New_York", &context,)
            .unwrap()
            .output,
        "2025-07-01T12:00-04:00[America/New_York]"
    );
    assert_eq!(
        calcite_core::calculate_with_context(
            "(2025-02-08 at 12:00 America/New_York) + 1 month",
            &context,
        )
        .unwrap()
        .output,
        "2025-03-08T12:00-05:00[America/New_York]"
    );
    assert_eq!(
        calcite_core::calculate_with_context("2025-11-02 at 01:30 America/New_York", &context,)
            .unwrap_err()
            .kind(),
        DiagnosticKind::AmbiguousLocalTime
    );
}

#[test]
fn validates_snapshot_boundaries_and_transition_order() {
    let offset = UtcOffset::UTC;
    let invalid = TimeZoneDefinition::new(
        "Etc/Test",
        datetime("2025-01-01T00:00Z"),
        datetime("2026-01-01T00:00Z"),
        offset,
        [
            TimeZoneTransition::new(datetime("2025-06-01T00:00Z"), offset),
            TimeZoneTransition::new(datetime("2025-05-01T00:00Z"), offset),
        ],
    );
    assert_eq!(invalid, Err(TimeZoneError::InvalidDefinition));

    let snapshot = new_york_2025();
    assert_eq!(
        snapshot.at_instant("America/New_York", datetime("2026-01-01T00:00Z")),
        Err(TimeZoneError::OutOfRange)
    );
    assert_eq!(
        snapshot.at_instant("Europe/Paris", datetime("2025-01-01T00:00Z")),
        Err(TimeZoneError::UnknownZone)
    );
}

fn central_europe_snapshot() -> TimeZoneSnapshot {
    let standard = UtcOffset::from_hours_minutes(1, 0).unwrap();
    let daylight = UtcOffset::from_hours_minutes(2, 0).unwrap();
    let berlin = TimeZoneDefinition::new(
        "Europe/Berlin",
        datetime("2025-01-01T00:00Z"),
        datetime("2027-01-01T00:00Z"),
        standard,
        [
            TimeZoneTransition::new(datetime("2025-03-30T01:00Z"), daylight),
            TimeZoneTransition::new(datetime("2025-10-26T01:00Z"), standard),
            TimeZoneTransition::new(datetime("2026-03-29T01:00Z"), daylight),
            TimeZoneTransition::new(datetime("2026-10-25T01:00Z"), standard),
        ],
    )
    .unwrap();
    let tokyo = TimeZoneDefinition::new(
        "Asia/Tokyo",
        datetime("2025-01-01T00:00Z"),
        datetime("2027-01-01T00:00Z"),
        UtcOffset::from_hours_minutes(9, 0).unwrap(),
        [],
    )
    .unwrap();
    let new_york = TimeZoneDefinition::new(
        "America/New_York",
        datetime("2025-01-01T00:00Z"),
        datetime("2027-01-01T00:00Z"),
        UtcOffset::from_hours_minutes(-5, 0).unwrap(),
        [],
    )
    .unwrap();
    TimeZoneSnapshot::new(
        "test-2025a",
        [berlin, tokyo, new_york],
        [
            (
                "CET".to_owned(),
                TimeZoneCode::Resolved {
                    literal_offset: standard,
                    representative_zone: "Europe/Berlin".to_owned(),
                },
            ),
            (
                "CEST".to_owned(),
                TimeZoneCode::Resolved {
                    literal_offset: daylight,
                    representative_zone: "Europe/Berlin".to_owned(),
                },
            ),
            (
                "CST".to_owned(),
                TimeZoneCode::Ambiguous {
                    candidates: vec![
                        "UTC-06:00 (America/Chicago)".to_owned(),
                        "UTC+08:00 (Asia/Shanghai)".to_owned(),
                    ],
                },
            ),
        ],
    )
    .unwrap()
}

fn zoned_context() -> CalculationContext {
    CalculationContext {
        temporal: calcite_core::TemporalContext {
            time_zones: Some(central_europe_snapshot()),
            current_datetime: Some(datetime("2026-10-15T12:00Z")),
            ..calcite_core::TemporalContext::default()
        },
        ..CalculationContext::default()
    }
}

#[test]
fn resolves_zone_abbreviations_with_dst_by_default() {
    let context = zoned_context();
    assert_eq!(
        calcite_core::calculate_with_context("2026-10-15 at 21:00 CET", &context)
            .unwrap()
            .output,
        "2026-10-15T21:00+02:00[Europe/Berlin]"
    );
    assert_eq!(
        calcite_core::calculate_with_context("2026-01-15 at 21:00 CET", &context)
            .unwrap()
            .output,
        "2026-01-15T21:00+01:00[Europe/Berlin]"
    );
}

#[test]
fn literal_zone_codes_resolve_abbreviations_to_fixed_offsets() {
    let mut context = zoned_context();
    context.temporal.literal_zone_codes = true;
    assert_eq!(
        calcite_core::calculate_with_context("2026-10-15 at 21:00 CET", &context)
            .unwrap()
            .output,
        "2026-10-15T21:00+01:00"
    );
    assert_eq!(
        calcite_core::calculate_with_context("2026-10-15 at 21:00 CEST", &context)
            .unwrap()
            .output,
        "2026-10-15T21:00+02:00"
    );
}

#[test]
fn city_words_resolve_zone_name_segments() {
    let context = zoned_context();
    assert_eq!(
        calcite_core::calculate_with_context("2026-10-15 at 21:00 Tokyo time", &context)
            .unwrap()
            .output,
        "2026-10-15T21:00+09:00[Asia/Tokyo]"
    );
    assert_eq!(
        calcite_core::calculate_with_context("2026-10-15 at 21:00 new york time", &context)
            .unwrap()
            .output,
        "2026-10-15T21:00-05:00[America/New_York]"
    );
    assert!(calcite_core::calculate_with_context("berlin", &context).is_err());
}

#[test]
fn ambiguous_codes_report_candidates_instead_of_guessing() {
    let context = zoned_context();
    let error =
        calcite_core::calculate_with_context("2026-10-15 at 21:00 CST", &context).unwrap_err();
    assert_eq!(error.kind(), DiagnosticKind::AmbiguousTimeZone);
    assert!(
        error.to_string().contains("America/Chicago"),
        "the diagnostic lists candidates: {error}"
    );
}

#[test]
fn bare_hours_and_anchored_conversions_use_zone_data() {
    let context = zoned_context();
    assert_eq!(
        calcite_core::calculate_with_context("9pm CET to Tokyo time", &context)
            .unwrap()
            .output,
        "2026-10-16T04:00+09:00[Asia/Tokyo]"
    );
    assert_eq!(
        calcite_core::calculate_with_context("9pm CET to UTC", &context)
            .unwrap()
            .output,
        "2026-10-15T19:00Z"
    );
}
