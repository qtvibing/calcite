use calcite_core::surface_variants;

#[test]
fn unchanged_input_has_a_single_variant() {
    assert_eq!(surface_variants("2 + 2"), vec!["2 + 2".to_owned()]);
    assert_eq!(surface_variants("  1 mile in km  "), vec!["1 mile in km".to_owned()]);
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
        vec![
            "what is   calculate  2 + 2".to_owned(),
            "2 + 2".to_owned(),
        ]
    );
}

#[test]
fn quantity_questions_are_rewritten_as_conversions() {
    for (source, expected) in [
        ("how many km in 5 miles", "5 miles in km"),
        ("how many kilometers are in 5 miles", "5 miles in kilometers"),
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
