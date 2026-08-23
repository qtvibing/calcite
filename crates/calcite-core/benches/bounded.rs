use std::{hint::black_box, time::Instant};

use calcite_core::{DiceDistribution, Number, calculate, parse};

fn main() {
    let long_expression = std::iter::repeat_n("1", 2_000)
        .collect::<Vec<_>>()
        .join(" + ");
    benchmark("parse 2,000-term expression", 100, || {
        black_box(parse(black_box(&long_expression))).unwrap();
    });
    benchmark("evaluate exact combinatorics", 100, || {
        black_box(calculate(black_box("choose(10000, 25)"))).unwrap();
    });
    benchmark("evaluate bounded dice probability", 100, || {
        let dice = DiceDistribution::new(20, 20).unwrap();
        black_box(dice.probability(210));
    });
    let maximum_decimal = "9".repeat(10_000);
    benchmark("parse maximum decimal literal", 10, || {
        black_box(Number::parse_decimal(black_box(&maximum_decimal))).unwrap();
    });
}

fn benchmark(name: &str, iterations: u32, mut operation: impl FnMut()) {
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    let elapsed = start.elapsed();
    println!("{name}: {:?}/iteration", elapsed / iterations);
}
