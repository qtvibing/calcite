#![no_main]
use libfuzzer_sys::fuzz_target;

// Arbitrary input goes through the lexer, parser, evaluator, and formatter.
// A panic, hang, or unbounded allocation here is a bug in a public API.
fuzz_target!(|data: &str| {
    let _ = calcite_core::calculate_document(data);
});
