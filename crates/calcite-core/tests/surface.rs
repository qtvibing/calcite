use calcite_core::surface_variants;

#[test]
fn unchanged_input_has_a_single_variant() {
    assert_eq!(surface_variants("2 + 2"), vec!["2 + 2".to_owned()]);
    assert_eq!(
        surface_variants("  1 mile in km  "),
        vec!["1 mile in km".to_owned()]
    );
}

#[test]
fn conversational_prefixes_are_stripped() {
    for (source, expected) in [
        ("what is 5% of 200", "5% of 200"),
        ("What's 2+2?", "2+2"),
        ("whats 2 + 2", "2 + 2"),
        ("what are 2 + 2 and 3", "2 + 2 and 3"),
        ("how much is 1 mile in km", "1 mile in km"),
        ("tell me the time", "the time"),
        ("calculate 90 km / 3 hours", "90 km / 3 hours"),
        ("compute 2 + 2", "2 + 2"),
        ("evaluate 2 + 2", "2 + 2"),
        ("convert 5 miles to km", "5 miles to km"),
        ("calc 2 + 2", "2 + 2"),
        ("WHAT IS 2 + 2?", "2 + 2"),
    ] {
        let variants = surface_variants(source);
        assert!(
            variants.contains(&expected.to_owned()),
            "{source}: variants {variants:?} should contain {expected:?}"
        );
    }
}

#[test]
fn prefixes_strip_repeatedly_and_collapse_whitespace() {
    assert_eq!(
        surface_variants("what is   calculate  2 + 2"),
        vec!["what is   calculate  2 + 2".to_owned(), "2 + 2".to_owned(),]
    );
}

#[test]
fn quantity_questions_are_rewritten_as_conversions() {
    for (source, expected) in [
        ("how many km in 5 miles", "5 miles in km"),
        (
            "how many kilometers are in 5 miles",
            "5 miles in kilometers",
        ),
        ("how much gas in 20 gallons", "20 gallons in gas"),
    ] {
        let variants = surface_variants(source);
        assert!(
            variants.contains(&expected.to_owned()),
            "{source}: variants {variants:?} should contain {expected:?}"
        );
    }
}

#[test]
fn similar_words_are_not_stripped_or_rewritten() {
    let variants = surface_variants("converter 2 + 2");
    assert!(!variants.contains(&"2 + 2".to_owned()));
    let variants = surface_variants("however many 2 + 2");
    assert!(!variants.contains(&"many 2 + 2".to_owned()));
}

#[test]
fn statements_with_is_are_not_rewritten_as_conversions() {
    let variants = surface_variants("how much is 1 mile in km");
    assert!(!variants.contains(&"km in 1 mile".to_owned()));
}

#[test]
fn percent_questions_are_rewritten_as_percent_of() {
    for (source, expected) in [
        ("what % of 50 is 10", "10 as a percentage of 50"),
        ("what percentage of 50 is 10", "10 as a percentage of 50"),
        ("what percent of 80 is 20", "20 as a percentage of 80"),
    ] {
        let variants = surface_variants(source);
        assert!(
            variants.contains(&expected.to_owned()),
            "{source}: variants {variants:?} should contain {expected:?}"
        );
    }
}

#[test]
fn directional_change_phrasing_is_rewritten_as_arithmetic() {
    for (source, expected) in [
        ("increase 80 by 15%", "80 + 15%"),
        ("decrease 80 by 15%", "80 - 15%"),
        ("reduce 80 by 25%", "80 - 25%"),
        ("80 increased by 15%", "80 + 15%"),
        ("80 reduced by 25%", "80 - 25%"),
        ("raise 100 by 20", "100 + 20"),
    ] {
        let variants = surface_variants(source);
        assert!(
            variants.contains(&expected.to_owned()),
            "{source}: variants {variants:?} should contain {expected:?}"
        );
    }
}

#[test]
fn zone_time_queries_are_rewritten_as_now_conversions() {
    for (source, expected) in [
        ("time in tokyo", "now in tokyo"),
        ("time in new york", "now in new york"),
    ] {
        let variants = surface_variants(source);
        assert!(
            variants.contains(&expected.to_owned()),
            "{source}: variants {variants:?} should contain {expected:?}"
        );
    }
    // Time differences are statements, not zone queries.
    assert!(
        surface_variants("time from 10:00 to 12:30")
            .iter()
            .all(|variant| variant != "now from 10:00 to 12:30")
    );
}

#[test]
fn offsets_from_contextual_dates_are_rewritten_as_addition() {
    for (source, expected) in [
        ("2 weeks from today", "today + 2 weeks"),
        ("in 2 weeks", "today + 2 weeks"),
        ("in 3 days", "today + 3 days"),
        ("3 days from tomorrow", "tomorrow + 3 days"),
    ] {
        let variants = surface_variants(source);
        assert!(
            variants.contains(&expected.to_owned()),
            "{source}: variants {variants:?} should contain {expected:?}"
        );
    }
    // `from` between explicit dates is a difference, not an offset.
    assert!(
        !surface_variants("days from 2025-01-01 to 2025-01-10")
            .contains(&"2025-01-01 + days".to_owned())
    );
}

#[test]
fn percentage_change_questions_are_rewritten_as_percent_of() {
    let variants = surface_variants("percentage change from 50 to 80");
    assert!(
        variants.contains(&"(80 - 50) as a percentage of 50".to_owned()),
        "variants {variants:?}"
    );
    let variants = surface_variants("percent change from 80 to 50");
    assert!(
        variants.contains(&"(50 - 80) as a percentage of 80".to_owned()),
        "variants {variants:?}"
    );
}
