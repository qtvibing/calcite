use calcite_core::{
    CalculationContext, CalculationSession, CoinSide, DiagnosticKind, DiceDistribution, DiceError,
    calculate, calculate_document_with_entropy, calculate_with_entropy, flip_coin,
};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn dice_notation_produces_typed_distributions() {
    assert_eq!(output("2d6"), "2d6");
    assert_eq!(output("D20"), "1d20");
    assert_eq!(output("2D6 + 3"), "2d6+3");
    assert_eq!(output("3 + 2d6"), "2d6+3");
    assert_eq!(output("2d6 - 2"), "2d6-2");
    assert_eq!(output("expected_value(2d6 + 1)"), "8");
    assert_eq!(
        calculate("0d6").unwrap_err().kind(),
        DiagnosticKind::InvalidDice
    );
    assert_eq!(
        calculate("2d1").unwrap_err().kind(),
        DiagnosticKind::InvalidDice
    );
}

#[test]
fn probabilities_are_exact() {
    let result = calculate("probability(2d6, 7)").unwrap();
    assert_eq!(
        result
            .value
            .as_percentage()
            .unwrap()
            .ratio()
            .exact_fraction()
            .as_deref(),
        Some("1/6")
    );
    assert_eq!(
        DiceDistribution::new(2, 6)
            .unwrap()
            .probability(2)
            .ratio()
            .exact_fraction()
            .as_deref(),
        Some("1/36")
    );
}

#[test]
fn rolls_and_coin_flips_require_caller_supplied_entropy() {
    let dice = DiceDistribution::new(2, 6).unwrap();
    let mut entropy = [0, 5].into_iter();
    assert_eq!(dice.roll(&mut entropy), Ok(7));

    let mut exhausted = std::iter::empty();
    assert_eq!(dice.roll(&mut exhausted), Err(DiceError::EntropyExhausted));

    let mut heads = [0].into_iter();
    let mut tails = [1].into_iter();
    assert_eq!(flip_coin(&mut heads), Ok(CoinSide::Heads));
    assert_eq!(flip_coin(&mut tails), Ok(CoinSide::Tails));
}

#[test]
fn random_language_forms_use_only_caller_supplied_entropy() {
    for source in [
        "roll 3d12",
        "roll a 3d12",
        "roll me 3d12",
        "roll me a 3d12",
        "roll(3d12)",
    ] {
        let mut entropy = [0, 11, 5].into_iter();
        assert_eq!(
            calculate_with_entropy(source, &mut entropy).unwrap().output,
            "19"
        );
    }

    let mut heads = [0].into_iter();
    let mut tails = [1].into_iter();
    assert_eq!(
        calculate_with_entropy("flip a coin", &mut heads)
            .unwrap()
            .output,
        "heads"
    );
    assert_eq!(
        calculate_with_entropy("coinflip()", &mut tails)
            .unwrap()
            .output,
        "tails"
    );
}

#[test]
fn random_expressions_fail_clearly_without_entropy() {
    for source in ["roll 2d6", "flip coin"] {
        assert_eq!(
            calculate(source).unwrap_err().kind(),
            DiagnosticKind::MissingEntropy
        );
    }
}

#[test]
fn exhausted_entropy_sources_fail_with_a_dedicated_diagnostic() {
    let mut exhausted = std::iter::empty::<u64>();
    assert_eq!(
        calculate_with_entropy("roll 2d6", &mut exhausted)
            .unwrap_err()
            .kind(),
        DiagnosticKind::EntropyExhausted
    );
    let mut exhausted = std::iter::empty::<u64>();
    assert_eq!(
        calculate_with_entropy("flip a coin", &mut exhausted)
            .unwrap_err()
            .kind(),
        DiagnosticKind::EntropyExhausted
    );
}

#[test]
fn random_language_forms_accept_alternative_phrasings() {
    let mut entropy = [0].into_iter();
    assert_eq!(
        calculate_with_entropy("roll the 1d6", &mut entropy)
            .unwrap()
            .output,
        "1"
    );
    for (source, expected) in [
        ("flip the coin", "tails"),
        ("coin flip", "tails"),
        ("coinflip", "tails"),
    ] {
        let mut entropy = [1].into_iter();
        assert_eq!(
            calculate_with_entropy(source, &mut entropy)
                .unwrap()
                .output,
            expected,
            "{source}"
        );
    }
    let mut bare = [0].into_iter();
    assert_eq!(
        calculate_with_entropy("coinflip", &mut bare)
            .unwrap()
            .output,
        "heads"
    );
    let mut parenthesized = [0].into_iter();
    assert_eq!(
        calculate_with_entropy("coinflip()", &mut parenthesized)
            .unwrap()
            .output,
        "heads"
    );
}

#[test]
fn documents_and_sessions_consume_entropy_in_evaluation_order() {
    let mut document_entropy = [0, 1, 2].into_iter();
    let document = calculate_document_with_entropy(
        "roll_one = roll 1d6\nroll_two = roll 1d6\nroll_one + roll_two",
        &mut document_entropy,
    )
    .unwrap();
    assert_eq!(document.lines[0].output, "1");
    assert_eq!(document.lines[1].output, "2");
    assert_eq!(document.lines[2].output, "3");

    let mut session = CalculationSession::new();
    let mut session_entropy = [5].into_iter();
    assert_eq!(
        session
            .calculate_with_entropy(
                "roll_die() = roll 1d6",
                &CalculationContext::default(),
                &mut session_entropy,
            )
            .unwrap()
            .output,
        "<function()>"
    );
    assert_eq!(
        session
            .calculate_with_entropy(
                "roll_die()",
                &CalculationContext::default(),
                &mut session_entropy,
            )
            .unwrap()
            .output,
        "6"
    );
}

#[test]
fn entropy_sampling_rejects_biased_tail_values() {
    let dice = DiceDistribution::new(1, 6).unwrap();
    let mut entropy = [u64::MAX, 0].into_iter();
    assert_eq!(dice.roll(&mut entropy), Ok(1));
}
