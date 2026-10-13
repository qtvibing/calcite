#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    // The parser is the attack surface: arbitrary input through the lexer,
    // parser, evaluator, and formatter. Any panic, hang, or unbounded
    // allocation here is a bug in a public API.
    let _ = calcite_core::calculate_document(data);
});
