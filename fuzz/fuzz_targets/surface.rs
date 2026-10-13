#![no_main]
use libfuzzer_sys::fuzz_target;

// The surface layer must accept arbitrary text without a panic.
fuzz_target!(|data: &str| {
    let variants = calcite_core::surface_variants(data);
    for variant in variants {
        let _ = calcite_core::calculate(&variant);
    }
});
