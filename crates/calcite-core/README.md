# calcite-core

`calcite-core` is Calcite's deterministic parser and evaluator.

## Evaluate an expression

```rust
use calcite_core::{Value, calculate};

let result = calculate("1 km + 500 m")?;
assert_eq!(result.output, "1.5 km");

let Value::Unit(distance) = result.value else {
    unreachable!("the expression evaluates to a unit value");
};
assert_eq!(distance.magnitude().exact_fraction().as_deref(), Some("3/2"));

# Ok::<(), calcite_core::Diagnostic>(())
```

`Calculation` contains both the typed `Value` and its formatted output. Diagnostics include a `DiagnosticKind` and a byte `Span` into the original source.

```rust
use calcite_core::{DiagnosticKind, calculate};

let error = calculate("1 / 0").unwrap_err();
assert_eq!(error.kind(), DiagnosticKind::DivisionByZero);
assert_eq!(error.span().start, 2);
assert_eq!(error.span().end, 3);
```

## Supply context explicitly

Use `CalculationContext` for locale, formatting, current time, timezone snapshots, holidays, currencies, exchange rates, and custom units.

```rust
use calcite_core::{
    CalculationContext, CompoundUnit, Number, Unit, calculate_with_context,
};

let mut context = CalculationContext::default();
context.custom_units.define(
    "smoot",
    Number::parse_decimal("1.7")?,
    CompoundUnit::from_unit(Unit::Meter),
)?;

let result = calculate_with_context("2 smoot in meters", &context)?;
assert_eq!(result.output, "3.4 m");

# Ok::<(), Box<dyn std::error::Error>>(())
```

Contextual expressions fail rather than guessing when their required state is absent. For example, `today` needs `TemporalContext::current_datetime`, named IANA zones need a `TimeZoneSnapshot`, and currency conversion needs an `ExchangeRateSnapshot`.

## Keep variables between calculations

`CalculationSession` provides explicit, reusable state for variables, functions, `ans`, and `_`.

```rust
use calcite_core::{CalculationContext, CalculationSession};

let context = CalculationContext::default();
let mut session = CalculationSession::new();

session.calculate("price = $80", &context)?;
assert_eq!(session.calculate("price + 25%", &context)?.output, "100 USD");
assert_eq!(session.calculate("ans / 4", &context)?.output, "25 USD");

# Ok::<(), calcite_core::Diagnostic>(())
```

## Supply entropy for rolls and coin flips

The ordinary evaluation functions stay deterministic and reject sampling expressions. Use an entropy-aware variant when the caller intends to sample:

```rust
use calcite_core::calculate_with_entropy;

let mut entropy = [0, 11, 5].into_iter();
let result = calculate_with_entropy("roll me a 3d12", &mut entropy)?;
assert_eq!(result.output, "19");

# Ok::<(), calcite_core::Diagnostic>(())
```

`CalculationSession::calculate_with_entropy` and the `calculate_document_*_with_entropy` functions provide the same explicit boundary for stateful and multiline evaluation. Production applications can adapt an operating-system random source; tests can supply fixed values as above.

## Documents

Use `calculate_document_with_context` for source-order, multiline calculations. The result preserves values, rendered output, labels, comments, dependencies, and source spans for each calculation line.

```rust
use calcite_core::{CalculationContext, calculate_document_with_context};

let source = "subtotal = $80\nrate = 25%\nsubtotal + rate";
let document = calculate_document_with_context(source, &CalculationContext::default())?;

assert_eq!(document.lines[2].output, "100 USD");
assert_eq!(document.lines[2].dependencies.len(), 2);

# Ok::<(), calcite_core::Diagnostic>(())
```

For exchange-rate providers and snapshot persistence, see [`calcite-rates`](../calcite-rates/README.md). The full expression language is documented in the [language reference](../../docs/reference.md).
