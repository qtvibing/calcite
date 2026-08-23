use std::panic::{AssertUnwindSafe, catch_unwind};

use calcite_core::{DiagnosticKind, calculate, parse};

#[test]
fn parser_rejects_excessive_nesting_without_overflowing() {
    let source = format!("{}1{}", "(".repeat(300), ")".repeat(300));
    assert_eq!(
        parse(&source).unwrap_err().kind(),
        DiagnosticKind::ExpressionTooComplex
    );
}

#[test]
fn parser_rejects_oversized_sources_before_tokenization() {
    let source = "1".repeat(1_000_001);
    assert_eq!(
        parse(&source).unwrap_err().kind(),
        DiagnosticKind::ExpressionTooComplex
    );
}

#[test]
fn varied_bounded_input_never_panics() {
    const ALPHABET: &[char] = &[
        '0', '1', '9', '+', '-', '*', '/', '^', '(', ')', ',', '.', '%', ' ', 'a', 'i', 'π', '½',
        '≤', '"', '\\', '→', '$', ':', '!', '&', '|', '\n',
    ];
    let mut state = 0x9e37_79b9_u32;
    for _ in 0..2_000 {
        let length = (next(&mut state) % 96) as usize;
        let mut source = String::new();
        for _ in 0..length {
            source.push(ALPHABET[(next(&mut state) as usize) % ALPHABET.len()]);
        }
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = calculate(&source);
            }))
            .is_ok(),
            "parser panicked for {source:?}"
        );
    }
}

fn next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}
