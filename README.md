# Calcite

Calcite is a calculator for expressions written the way people tend to write them. It handles ordinary arithmetic, but it also keeps track of things that should not be flattened into plain numbers: percentages, units, rates, money, dates, times, booleans, dice distributions, and more.

```text
0.1 + 0.2                                  → 0.3
200 + 10%                                  → 220
1 mile in km                               → 1.609344 km
$100/hour * 90 minutes                     → 150 USD
2025-01-31 + 1 month                       → 2025-02-28
time between 10:00 CEST and 10:00 CET      → 1 h
(1 + 2i) * (3 - 4i)                        → 11 + 2i
probability(2d6, 7)                         → 16.6666666667%
```

`calcite-core` is deterministic for the same expression, context, and entropy sequence. The core library does not read the clock, inspect the host timezone, fetch exchange rates, or obtain random data. Applications provide that state explicitly when they need it.

Calcite is currently pre-1.0. The language is usable, but public APIs and some syntax may still change.

## Build it

Calcite is not yet published as an installable release. From a checkout of this repository:

```sh
cargo build --release
./target/release/calcite '2 + 3 * 4'
```

To install the binary into Cargo's bin directory:

```sh
cargo install --path crates/calcite-cli
```

The workspace requires a current stable Rust toolchain.

## Using the CLI

Pass an expression as arguments:

```sh
calcite '90 km / 3 hours'
# 30 km/h
```

Use `--eval` when the expression starts with a dash or when explicit input selection is clearer:

```sh
calcite --eval '-2 ^ 2'
# -4
```

Calcite also reads documents from files or standard input:

```sh
calcite --file trip.calcite

cat <<'EOF' | calcite
fuel = 52 liters
distance = 680 km
distance / fuel
EOF
```

Start the interactive calculator with:

```sh
calcite --repl
```

The REPL preserves variables and the previous result:

```text
> price = $80
80 USD
> price + 25%
100 USD
> ans / 4
25 USD
```

It has editable persistent history, including arrow-key navigation. Tab completion covers variables, function names, currency codes, and `:` commands, and a live hint shows the result of the current line as you type it. `:help` prints a cheat sheet inside the REPL. Colors follow the terminal and respect `NO_COLOR`. The built-in commands are:

| Command | Effect |
| --- | --- |
| `:history` | Print calculation history |
| `:clear-history` | Remove saved history |
| `:clear` | Clear variables, `ans`, and `_` |
| `:help` | Print command and input help |
| `:quit`, `:q`, `:exit` | Leave the REPL |
| `!!` | Repeat the latest history entry |
| `!N` | Repeat history entry `N` |

Use `--plain` to print only the last result of a document, or `--json` for structured output with typed value data and source spans.

```sh
calcite --json '1 km + 500 m'
```

Errors are structured too. Normal terminal output points at the source range that failed:

```text
error: cannot divide by zero
  1 / 0
    ^
```

Run `calcite --help` for the complete option list.

Conversational input is accepted for one-shot expressions. If the raw input is not valid Calcite, the CLI retries with leading question phrases (`what is`, `how much is`, `calculate`, `convert`, ...) stripped and quantity questions (`how many km in 5 miles`) rewritten as conversions. Input the language already accepts is never rewritten.

## A tour of the language

These examples are a quick survey rather than the full specification. See the [language reference](docs/reference.md) for exact syntax and semantics.

### Exact numbers

Decimal and rational arithmetic stays exact internally:

```text
0.1 + 0.2                    → 0.3
1 / 3 * 3                    → 1
0.(3) as fraction            → 1/3
1 1/2 + 2 1/4                → 3.75
2½ + ¼                       → 2.75
twenty-one + 4               → 25
2³ + 3²                      → 17
```

Integers are arbitrary precision. Bases from 2 through 36 are supported:

```text
0xffff + 1                   → 65536
99 in base 3                 → 3#10200
36#z + 1                     → 36
roman XLIV + 6               → 50
choose(52, 5)                → 2598960
12345678901234567890 mod 97  → 3
```

Irrational and transcendental results use bounded-precision real arithmetic:

```text
sqrt(2)                      → 1.4142135624
sin(30 degrees)              → 0.5
```

### Percentages

Percentages remain percentages until an operation calls for their ratio:

```text
10% + 20%                    → 30%
10% of 200                   → 20
10% on 200                   → 220
10% off 200                  → 180
200 + 10%                    → 220
80 as a percentage of 50     → 160%
50% as fraction              → 1/2
```

Whitespace matters around `%`: `10%` is a percentage, while `17 % 5` is integer remainder. Conversational forms work too: `what % of 50 is 10` and `increase 80 by 15%` are accepted alongside the forms above.

### Units and rates

Calcite checks dimensions and converts with exact factors where possible:

```text
1 km + 500 m                 → 1.5 km
6 ft 2 in in cm               → 187.96 cm
5 GB in MB                   → 5000 Mbyte
1 acre in square meters      → 4046.8564224 m²
32 F to C                    → 0 °C
100 F - 20 F                 → 80 °F
1 kilowatt hour in joules    → 3600000 joule
1 mebibyte in bytes          → 1048576 byte
100 megabits / second in megabytes/minute
                              → 750 Mbyte/min
```

Temperature differences are deltas in the first operand's scale; converting the right side keeps its point value (`20C - 68F` is `0 °C`). Conversational date and zone phrasing is accepted too: `2 weeks from today`, `in 2 weeks`, `time in tokyo`, and `percentage change from 50 to 80` → `60%`.

Compound units are structural rather than a list of special cases:

```text
90 km / 3 hours              → 30 km/h
60 km/h in m/s               → 16.6666666667 m/s
1 newton in kg*m/s^2         → 1 m·kg/s²
30 / day * 2 weeks           → 420
```

Custom units can be supplied through the core API or CLI configuration.

### Money and finance

Money keeps its currency, and money-per-unit values remain rates:

```text
$80 + 25%                    → 100 USD
$100/hour * 90 minutes       → 150 USD
$500 at $20/hour             → 25 h
$120/hour in minute          → 2 USD/min
100 dollars + 25%            → 125 USD
(1955.45 - 20%) EUR to CHF
                              → (a conversion using an exchange-rate snapshot)
```

Currency conversion only works when the application has installed an exchange-rate snapshot. The CLI loads a fresh local cache first and contacts a configured provider only when conversion needs rates or `--refresh-rates` is used.

Deterministic finance functions include tax extraction, compound growth, present and future value, CAGR, ROI, mortgage repayment, purchasing power, and break-even calculations:

```text
add_tax($80, 25%)            → 100 USD
future_value($1000, 5%, 10)  → 1628.89 USD   (rounded to the currency's minor units)
break_even($10000, $25)      → 400
```

### Dates, times, and timezones

Dates use Gregorian calendar arithmetic. Months and years are calendar durations; hours and smaller units are fixed durations.

```text
2025-01-31 + 1 month
                              → 2025-02-28
days between 2025-01-01 and 2025-01-10
                              → 9 days
time until christmas          → 72 days
new years eve                 → 2026-12-31
time from 10:00 to 12:30     → 2 h 30 min
2025-03-08 at 10:00 CET      → 2025-03-08T10:00+01:00[CET]
2026-10-14 at 9pm CET to Tokyo time
                              → 2026-10-15T04:00+09:00[Asia/Tokyo]
1750000000 as a date         → 2025-06-15T15:06:40Z
now as unix time             → 1791993488
2025-06-15T15:06:40Z as a date
                              → 2025-06-15
```

Ambiguous slash and dot dates remain arithmetic unless a date order is configured:

```sh
calcite --date-order mdy '3/8/2025'
# 2025-03-08
```

Contextual expressions such as `today`, `now`, `next month`, `time since ...`, and `time until ...` require a current time. The CLI supplies it from the host; library callers must supply it explicitly. Named holidays (`christmas`, `halloween`, `new years day`, `valentines day`, `st patricks day`, `fourth of july`, and their eves) resolve to their next occurrence. `last christmas` selects the previous one, and a year pins a specific date: `christmas 2024`.

### Documents, variables, and functions

A Calcite document is evaluated from top to bottom. Names are case-insensitive and retain typed values.

```text
subtotal = $80
rate = 25%
total = subtotal + rate

square(x) = x * x
square(total as number)
```

Anonymous functions work too:

```text
((x, y) -> x + y)(2, 3)      → 5
```

Strings support escaping, concatenation, and comparison:

```text
"hello" + " world"          → hello world
"a" < "b"                   → true
```

### Dice

Dice expressions produce deterministic probability distributions:

```text
2d6                            → 2d6
expected_value(2d6)            → 7
probability(2d6, 7)            → 16.6666666667%
roll me a 3d12                 → 19        (example roll)
flip a coin                    → heads     (example flip)
```

`roll 3d12`, `roll a 3d12`, `roll me 3d12`, `roll me a 3d12`, and `roll(3d12)` are equivalent (each also accepts `the` in place of `a`). Coin flips accept `flip coin`, `flip a coin`, `flip the coin`, `coin flip`, `coinflip`, and `coinflip()`. The CLI supplies operating-system entropy, so sampled results vary. Library callers use the explicit entropy-aware evaluation APIs; the ordinary deterministic APIs return an error when an expression requests sampling.

## Configuration

Calcite reads JSON configuration from the first available location:

1. `CALCITE_CONFIG`
2. `$XDG_CONFIG_HOME/calcite/config.json`
3. `$HOME/.config/calcite/config.json`

An empty `CALCITE_CONFIG` disables file loading. Command-line options override configured values.

```json
{
  "locale": "en-US",
  "date_order": "mdy",
  "format": "auto",
  "precision": 10,
  "approximate": false,
  "default_currency": "USD",
  "timezone": "America/New_York",
  "literal_zone_codes": false,
  "rate_provider": "fallback",
  "history": true,
  "custom_units": {
    "smoot": "1.7 meters"
  }
}
```

Supported output formats are `auto`, `plain`, `exact`, `fraction`, `mixed`, `decimal`, `significant`, and `scientific`. Supported rate providers are `fallback`, `frankfurter`, and `ecb`.

The exchange-rate cache path can be overridden with `CALCITE_RATE_CACHE`. REPL history can be moved with `CALCITE_HISTORY`.

## Using the Rust crates

The workspace is split by responsibility:

- [`calcite-core`](crates/calcite-core) contains the language, typed values, deterministic evaluation, and formatting.
- [`calcite-rates`](crates/calcite-rates) contains optional file and network exchange-rate providers.
- [`calcite-cli`](crates/calcite-cli) owns host time, networking, cache paths, configuration, and interactive behavior.

See the [`calcite-core` API examples](crates/calcite-core/README.md) for embedding and the [`calcite-rates` README](crates/calcite-rates/README.md) for exchange-rate loading.

## Design boundaries

A few rules shape the implementation:

- Arithmetic is exact until an operation is inherently approximate.
- Typed values are not silently flattened.
- Capitalization does not change language semantics.
- Ambiguous input is rejected or requires explicit context.
- Core evaluation performs no I/O and reads no ambient clock, timezone, or entropy.
- Provider failures remain separate from evaluator diagnostics.
- Expensive operations and parser depth are bounded.

## License

Calcite is available under the [MIT License](LICENSE).
