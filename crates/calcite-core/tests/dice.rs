use calcite_core::{CoinSide, DiagnosticKind, DiceDistribution, DiceError, calculate, flip_coin};

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
fn entropy_sampling_rejects_biased_tail_values() {
    let dice = DiceDistribution::new(1, 6).unwrap();
    let mut entropy = [u64::MAX, 0].into_iter();
    assert_eq!(dice.roll(&mut entropy), Ok(1));
}
