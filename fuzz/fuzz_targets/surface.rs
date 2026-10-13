#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    // The conversational surface layer must terminate and never panic on
    // arbitrary text, even though valid input only ever reaches the parser.
    let variants = calcite_core::surface_variants(data);
    for variant in variants {
        let _ = calcite_core::calculate(&variant);
    }
});
