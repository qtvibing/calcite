# Calcite language reference

This document describes the expression language implemented by `calcite-core`. Examples show the default decimal-point locale, USD default currency, automatic output formatting, and no external context unless stated otherwise.

Calcite is case-insensitive for words, names, currency codes, unit aliases, and functions. `KM`, `km`, `Sum`, and `SUM` do not acquire different meanings because of capitalization.

## Contents

- [Documents and lexical rules](#documents-and-lexical-rules)
- [Numbers](#numbers)
- [Operators and precedence](#operators-and-precedence)
- [Percentages](#percentages)
- [Booleans and conditionals](#booleans-and-conditionals)
- [Strings](#strings)
- [Complex numbers](#complex-numbers)
- [Units and rates](#units-and-rates)
- [Money and exchange rates](#money-and-exchange-rates)
- [Dates, times, and calendars](#dates-times-and-calendars)
- [Dice](#dice)
- [Variables and functions](#variables-and-functions)
- [Built-in functions](#built-in-functions)
- [Conversions and output](#conversions-and-output)
- [Context and determinism](#context-and-determinism)
- [Diagnostics and limits](#diagnostics-and-limits)

## Documents and lexical rules

A single expression can be evaluated on its own. A document contains one entry per line and is evaluated from top to bottom.

```text
# Trip estimate
Distance: 680 km
fuel = 52 liters
Economy: distance / fuel
```

Blank lines are ignored. A line may be:

- an expression;
- a variable assignment such as `fuel = 52 liters`;
- a named function definition such as `square(x) = x * x`;
- a heading beginning with `#`;
- a full-line or trailing comment beginning with `//`;
- a labeled expression such as `Economy: distance / fuel`.

Comment markers inside strings are ordinary text. Labels and comments are retained in document results, along with source spans and dependencies.

Identifiers begin with a letter or `_` and may contain letters, digits, or `_`. Identifier comparison is Unicode lowercase-based and therefore case-insensitive. A reassignment affects later lines, not earlier ones.

Whitespace normally separates tokens without changing meaning. One important exception is `%`: attached to a number it forms a percentage (`10%`); between two complete integer expressions it is a remainder operator (`17 % 5`).

Both ASCII and common Unicode operators are accepted:

| Meaning | Forms |
| --- | --- |
| Multiplication | `*`, `×`, `⋅`, full-width `＊`, `multiplied by`, `times` |
| Division | `/`, `÷`, `∕`, full-width `／`, `divided by` |
| Subtraction | `-`, `−`, full-width `－`, `minus` |
| Comparisons | `<=`, `>=`, `!=`, `≤`, `≥`, `≠` |
| Boolean logic | `and`, `or`, `not`, `&&`, `||`, `!`, `∧`, `∨`, `¬` |
| Lambda arrow | `->`, `→` |

## Numbers

### Integers and decimals

Integers have arbitrary precision. Decimal literals are exact base-10 rational values, not binary floating-point approximations.

```text
0.1 + 0.2                    → 0.3
1 / 3 * 3                    → 1
12345678901234567890 + 1     → 12345678901234567891
1_000_000 + 2                → 1000002
```

Scientific notation is accepted: `1.5e3`, `2E-4`. The decimal and grouping separators come from `NumberFormat`; the default is `.` for decimals and `,` for grouping. Under a decimal-comma context, `1,5 + 1` is `2,5`.

### Fractions, mixed numbers, and recurring decimals

A division of exact values remains exact. Literal forms add convenient spellings:

```text
1 1/2                        → 1.5
-2 3/4                       → -2.75
2½ + ¼                       → 2.75
0.(3) as fraction            → 1/3
0.0(15) as fraction          → 1/66
12.34(56) as fraction        → 61111/4950
```

Supported vulgar fractions include `½`, `⅓`, `⅔`, `¼`, `¾`, `⅕`, `⅖`, `⅗`, `⅘`, `⅙`, `⅚`, `⅛`, `⅜`, `⅝`, and `⅞`. A proper fraction may follow an integer without an operator to form a mixed number.

### Number words and suffixes

English number words can form exact integers:

```text
twenty-one + 4                       → 25
one hundred and five * 2             → 210
two million three hundred thousand five
                                      → 2300005
```

Ordinal suffixes do not change a numeric value (`1st`, `2nd`, `3rd`, `4th`), and are also used in dates. Large-number words and supported suffixes are exact. Unit names take priority where an abbreviation would be ambiguous; for example, case does not make `m` alternate between metre and million.

### Constants and implicit multiplication

The constants are `pi`/`π`, `tau`/`τ`, and `e`. Juxtaposition performs multiplication around parentheses, constants, function calls, and imaginary values:

```text
2(3 + 4)                     → 14
(2 + 1)(4 - 1)              → 9
2pi / pi                    → 2
3sqrt(9)                    → 9
3² + 4²                     → 25
2⁻²                         → 0.25
```

Superscript integer powers are supported.

### Bases and Roman numerals

Binary, octal, hexadecimal, and arbitrary bases from 2 through 36 produce typed radix integers:

```text
0b1010                       → 0b1010
0o755                        → 0o755
0xff + 1                     → 256
99 in binary                 → 0b1100011
99 in base 3                 → 3#10200
35 in base 36                → 36#z
36#z + 1                     → 36
hex(99)                      → 0x63
int(0o55)                    → 45
```

Digits above 9 use `a` through `z`. `0b`, `0o`, and `0x` are the canonical prefixes for bases 2, 8, and 16; other bases use `base#digits`.

Roman literals and conversion use the conventional range 1 through 3999:

```text
roman XLIV + 6               → 50
2024 in roman                → MMXXIV
```

### Approximate real results

Square roots, non-integer powers, logarithms, trigonometric operations, hyperbolic operations, and `exp` can produce bounded-precision real values. They are not represented as machine `f64` values.

```text
sqrt(2)                      → 1.4142135624
2 ^ 0.5                     → 1.4142135624
ln(e)                        → 1
```

## Operators and precedence

From lowest to highest precedence:

| Level | Operators | Notes |
| --- | --- | --- |
| Boolean OR | `or`, `||`, `∨` | Short-circuits |
| Boolean AND | `and`, `&&`, `∧` | Short-circuits |
| Bitwise OR | `\|` | Exact integers only |
| Bitwise XOR | `xor` | Exact integers only |
| Bitwise AND | `&` | Exact integers only |
| Comparison | `==`, `=`, `!=`, `<`, `<=`, `>`, `>=` and word forms | Produces a boolean |
| Shifts and rate solving | `<<`, `>>`, `at` | Exact integers for shifts |
| Addition | `+`, `-`, `plus`, `minus` | Typed addition/subtraction |
| Multiplicative | `*`, `/`, `%`, `mod`, `remainder`, `of`, `on`, `off`, combinatoric forms | Left-associative |
| Power | `^`, `**`, `to the power of` | Right-associative |
| Unary | unary `+`, unary `-`, `not` | Power binds inside unary minus |
| Postfix/conversion | `%`, factorial, units, `as`/`in`/`to` | Applied to the completed value |

Exponentiation is right-associative:

```text
2 ^ 3 ^ 2                   → 512
-2 ^ 2                      → -4
(-2) ^ 2                    → 4
```

Division and ordinary multiplicative operations are left-associative:

```text
20 / 5 / 2                  → 2
```

### Integer operations

`mod` and `remainder` have distinct rules for negative dividends:

```text
-17 mod 5                   → 3
-17 remainder 5             → -2
17 % 5                      → 2
remainder of 21 divided by 5
                             → 1
```

Factorials, bitwise operations, shifts, `gcd`, `lcm`, primality checks, and combinatorics require exact integers.

```text
20!                          → 2432902008176640000
0b1100 & 0b1010             → 8
0b1100 xor 0b1010           → 6
1 << 100                    → 1267650600228229401496703205376
25 combination 3            → 2300
3 permutations of 25        → 13800
```

## Percentages

A percentage is a typed ratio. Its default rendering keeps the percent sign.

```text
10% + 20%                   → 30%
50% * 50%                   → 25%
50% / 25%                   → 2
30% + 0.4                   → 70%
```

A percentage can measure or modify a base:

| Form | Meaning | Example |
| --- | --- | --- |
| `p of x` | `x × p` | `10% of 200` → `20` |
| `p on x` | increase `x` by `p` | `10% on 200` → `220` |
| `p off x` | decrease `x` by `p` | `10% off 200` → `180` |
| `x + p` | increase `x` by `p` | `200 + 10%` → `220` |
| `x - p` | decrease `x` by `p` | `200 - 10%` → `180` |

These rules work with typed bases such as money. Explicit representation conversions include `0.35 as %`, `2 / 5 as percent`, `50% as fraction`, and `20% as decimal`.

## Booleans and conditionals

Boolean literals are `true` and `false`. Booleans are not numbers. Equality, ordering, and logical operations preserve boolean type.

```text
24 >= 18.5 and 24 < 25      → true
not 2 == 3                  → true
10% equal to 0.1            → true
5 at least 5                → true
```

`and` and `or` short-circuit. Conditions also evaluate only their selected branch:

```text
true or 1 / 0 == 1
                              → true
if 2 > 1 then 10% of 200 else 1 / 0
                              → 20
unless 2 == 3 then 10 else 20
                              → 10
```

Both branches of a conditional must be semantically usable when selected, but an error in the unselected branch does not occur.

## Strings

Strings can use single or double quotes. Supported escapes are `\\`, `\"`, `\'`, `\n`, `\r`, and `\t`. Invalid escapes and unterminated strings are diagnostics.

```text
"hello" + " world"         → hello world
'line\n' + "two"             → line
                               two
"a" < "b"                  → true
"same" == 'same'            → true
```

Strings concatenate with `+` and compare lexicographically. They do not implicitly convert to numbers or other typed values.

## Complex numbers

`i` is the imaginary unit. Real and imaginary components use Calcite numbers.

```text
3 + 4i                      → 3 + 4i
(1 + 2i) * (3 - 4i)         → 11 + 2i
(3 + 4i) / (1 - 2i)         → -1 + 2i
i ^ 2                       → -1
sqrt(-4)                    → 2i
```

Complex values support arithmetic, integer powers, square roots that cross the negative real axis, and equality. Operations requiring an ordered real value reject complex inputs.

## Units and rates

A number followed by a unit becomes a typed quantity. Unit aliases are case-insensitive and generally accept singular, plural, US, and UK spellings where relevant.

### Built-in unit families

| Family | Built-in units |
| --- | --- |
| Length | metre, kilometre, centimetre, millimetre, inch, foot, yard, mile |
| Mass | gram, kilogram, ounce, pound |
| Fixed duration | nanosecond, microsecond, millisecond, second, minute, hour, day, week |
| Temperature | Celsius, Fahrenheit, kelvin |
| Angle | radian, degree |
| Area | square metric length units, square inch/foot/yard/mile, hectare, acre |
| Volume and cooking | litre, millilitre, cubic metre/centimetre, US gallon/quart/pint/cup/fluid ounce/tablespoon/teaspoon, metric cup/tablespoon/teaspoon |
| Frequency | hertz, kilohertz, megahertz, gigahertz |
| Force | newton, kilonewton |
| Energy | joule, kilojoule, watt-hour, kilowatt-hour |
| Power | watt, kilowatt, megawatt |
| Pressure | pascal, kilopascal, megapascal, bar, psi |
| Electrical | ampere, milliampere, volt, kilovolt, ohm, coulomb |
| Information | bit and byte with decimal kilo/mega/giga/tera forms and binary kibi/mebi/gibi/tebi byte forms |

Common symbol forms include `m`, `km`, `kg`, `ns`, `μs`, `ms`, `s`, `min`, `h`, `°C`, `°F`, `K`, `rad`, `m²`, `m³`, `Hz`, `kWh`, `kPa`, `mA`, `kV`, `Ω`, `Mbyte`, and `MiB`. Write full names when a short spelling would be unclear.

### Conversion and dimensions

Use `in`, `to`, or `as` for conversion:

```text
1 mile in km                 → 1.609344 km
1 acre to square meters      → 4046.8564224 m²
1 m³ in liters               → 1000 L
1 mebibyte in bytes          → 1048576 byte
```

Only dimensionally compatible quantities convert. `1 megabyte in megahertz` is an error.

Temperature scales with non-zero offsets are affine. They convert normally, but cannot be multiplied into arbitrary compounds:

```text
32 F to C                    → 0 °C
2 C * 3 m                    → error
2 K * 3 m                    → 6 m·K
```

### Compound units

Multiplication, division, and integer powers build structural units and cancel matching dimensions:

```text
2 m * 3 m                    → 6 m²
6 m² / 2 m                   → 3 m
90 km / 3 h                  → 30 km/h
30 km/h * 180 s              → 1500 m
10 m/s / 2 s                 → 5 m/s²
1 newton in kg*m/s^2         → 1 m·kg/s²
```

A conversion target can itself be compound: `10 m/s^2 in km/h^2`. Unit exponents must be exact integers from -100 through 100.

### Dimensionless rates

An explicitly requested rate display can be retained even when dimensions would otherwise cancel:

```text
3 hours / 3 days as hour/day → 1 h/day
```

Use `as number`, `as decimal`, or `as dec` to strip the display magnitude from a unit, rate, money, money-rate, or percentage value. This is explicit and discards the type:

```text
90 km/h as number            → 90
20% as number                → 0.2
```

### Custom units

`CalculationContext::custom_units` defines aliases in terms of existing compound units. The CLI exposes the same facility through `custom_units` in its JSON configuration:

```json
{
  "custom_units": {
    "smoot": "1.7 meters",
    "fortnight": "2 weeks"
  }
}
```

Custom units are contextual. They never modify a process-global registry.

## Money and exchange rates

Currency symbols or ISO 4217 codes produce money values:

```text
$100                         → 100 USD
EUR 25.50                    → 25.5 EUR
5000 JPY                     → 5000 JPY
₹2k                          → 2000 INR
```

Calcite includes the 178 currencies and funds from the 2026-01-01 SIX ISO 4217 List One snapshot, including each currency's minor-unit metadata. Codes are case-insensitive.

Common English names resolve to their ISO currency: `dollars` (USD), `euros` (EUR), `sterling` (GBP), `yen` (JPY), `won` (KRW), `rmb`/`renminbi`/`yuan` (CNY), `rupees` (INR), `francs` (CHF), `rubles` (RUB), `rand` (ZAR), `baht` (THB), `ringgit` (MYR), `rupiah` (IDR), `shekels` (ILS), `lira` (TRY), `zloty` (PLN), `forint` (HUF), `koruna` (CZK), and `dong` (VND). Ambiguous names resolve when qualified: `mexican pesos` (MXN), `argentine pesos` (ARS), `philippine pesos` (PHP), `colombian pesos` (COP), `uruguayan pesos` (UYU), `swedish krona` (SEK), `danish krone` (DKK), `norwegian krone` (NOK), `australian dollars` (AUD), `canadian dollars` (CAD), `new zealand dollars` (NZD), `hong kong dollars` (HKD), and `singapore dollars` (SGD). Qualified names work wherever a currency is read: `mexican pesos 100`, `100 mexican pesos`, and `100 USD in philippine pesos`. `pound` on its own remains the mass unit; GBP is `sterling` or `british pounds`.

`$` uses `MonetaryContext::default_currency` (USD by default). `¥` resolves to CNY when that is the configured default and JPY otherwise; `₩` similarly selects KPW when configured and KRW otherwise. An explicit ISO code is always unambiguous.

An ISO code after a complete expression labels the result as money. The attachment binds looser than addition and subtraction, so it applies to the whole result:

```text
(1955.45 - 20%) EUR        → 1564.36 EUR
1955.45 - 20% EUR          → 1564.36 EUR
(5 + 5) USD to CHF         → (requires an exchange-rate snapshot)
```

Attachment requires a plain number result. A percentage or a value that already carries a currency is rejected rather than reinterpreted.

In the default auto output mode, money displays rounded to its currency's minor units (two for EUR, zero for JPY, three for BHD). The typed value keeps full precision, JSON output keeps exact amounts, and any explicitly chosen output mode such as `decimal` with a precision is respected as-is.

Arithmetic preserves currency:

```text
$100 + $25                   → 125 USD
$100 + 10%                   → 110 USD
20% of EUR 50                → 10 EUR
$100 / $25                   → 4
```

Different currencies cannot be added directly. Convert one explicitly:

```text
100 USD in EUR
```

That conversion requires a caller-supplied `ExchangeRateSnapshot`. Snapshots are validated and installed atomically. Lookup supports direct, reciprocal, and coherent shared-base rates. A configured freshness policy can reject an old snapshot. `calcite-core` never fetches a missing rate.

### Money rates

Money divided by a unit produces a distinct money-rate value:

```text
$100 / hour                  → 100 USD/h
$100 / 2 hours               → 50 USD/h
$100/hour * 90 minutes       → 150 USD
$500 / ($100/hour)           → 5 h
$500 at $20/hour             → 25 h
$100/hour + $2/minute        → 220 USD/h
```

Exchange conversion also works on the currency part of a money rate when a snapshot is available.

### Finance functions

Finance rates must be typed percentages. Period counts must satisfy each function's positive-domain requirements.

| Function | Meaning |
| --- | --- |
| `sales_tax(net, rate)`, `tax(net, rate)` | Tax portion added to a net amount |
| `add_tax(net, rate)` | Gross amount after tax |
| `remove_tax(gross, rate)` | Net amount contained in a gross amount |
| `tax_from_total(gross, rate)` | Tax portion contained in a gross amount |
| `future_value(principal, rate, years[, compounds_per_year])` | Compound future value |
| `compound_interest(principal, rate, years[, compounds_per_year])` | Future value minus principal |
| `present_value(future, rate, years[, compounds_per_year])` | Discounted present value |
| `cagr(beginning, ending, years)` | Compound annual growth rate |
| `roi(cost, proceeds)` | Return on investment |
| `mortgage_payment(principal, annual_rate, years)`, `loan_payment(...)` | Monthly amortizing payment |
| `total_repayment(principal, annual_rate, years)` | Total monthly repayments |
| `total_interest(principal, annual_rate, years)` | Total repayment minus principal |
| `purchasing_power(amount, inflation_rate, years)` | Future real purchasing power |
| `break_even(fixed_cost, contribution_per_unit)` | Required unit count |

Natural forms are also accepted:

```text
sales tax on $300 at 15%                     → 45 USD
add tax to $300 at 15%                       → 345 USD
round(future value of $1000 at 7% for 3 years, 2)
                                               → 1225.04 USD
round(mortgage payment on $300000 at 6% for 30 years, 2)
                                               → 1798.65 USD
return on investment from 100 to 125         → 25%
round(cagr from 100 to 121 over 2 years, 2)  → 10%
```

The Rust API additionally provides bounded typed amortization schedules and contextual price-index and crypto quote data. Crypto assets are deliberately separate from ISO currencies, and observations retain source and freshness information.

## Dates, times, and calendars

### Date literals

ISO dates are always unambiguous:

```text
2025-03-08
```

English month forms include:

```text
March 8, 2025
8 March 2025
Aug 1st, 2025
4th Sept 2025
```

A month and day without a year, such as `Aug 5th`, requires a current date in `TemporalContext`; it uses that current year.

Numeric slash and dot dates are recognized only when `TemporalContext::date_order` is set to `DayMonthYear`, `MonthDayYear`, or `YearMonthDay`. Without it, `3/8/2025` remains arithmetic rather than silently assuming a locale.

### Times and date-times

Clock times support hours and minutes, optional seconds, and up to nanosecond precision. ISO date-times can include `Z` or a numeric UTC offset:

```text
10:30
10:30:45.123456789
2025-03-08T10:30:00Z
2025-03-08T10:30:00+02:00
```

Leap seconds are rejected. The explicit core policy is `LEAP_SECOND_POLICY == LeapSecondPolicy::Reject`.

A date and time can be joined with `at`:

```text
2025-03-08 at 10:00 CET      → 2025-03-08T10:00+01:00
```

### Fixed durations and calendar durations

Nanoseconds through weeks are fixed elapsed durations. Months and years are calendar durations. Calendar addition clamps to the last valid day where necessary:

```text
2024-02-29 + 1 day           → 2024-03-01
2025-01-31 + 1 month         → 2025-02-28
2024-02-29 + 1 year          → 2025-02-28
```

Date subtraction returns a fixed day duration. `calendar duration between ... and ...` instead preserves year, month, and day components:

```text
2025-03-08 - 2025-03-01
                              → 7 days
calendar duration between 2024-01-31 and 2025-03-30
                              → 1 year 1 month 30 days
```

### Difference phrases

Supported elapsed-difference forms include:

```text
time between 10:00 and 12:30
                              → 2 h 30 min
time from 10:00 to 12:30     → 2 h 30 min
how long between 10:00 and 12:30
                              → 2 h 30 min
days between 2025-01-01 and 2025-01-10
                              → 9 days
hours between 2025-01-01 and 2025-01-03
                              → 48 h
```

`time_since`, `time_until`, `calendar_since`, `calendar_until`, and their natural phrases compare against the current contextual date-time.

### Relative dates

These require `TemporalContext::current_datetime`:

- `today`, `tomorrow`, `yesterday`, `now`;
- `next month`, `last month`, `previous month`;
- `next year`, `last year`, `previous year`;
- `start of month`, `end of month`, `start of year`, `end of year`;
- `next Monday`, `last Friday`, `previous Tuesday`, `this Sunday`.

`next` and `last` are strict: on a Saturday, `next Saturday` is seven days later and `last Saturday` is seven days earlier. `this weekday` refers to the weekday in the current Monday-through-Sunday week.

### Workdays

The default workday calendar treats Monday through Friday as workdays and has no holidays. An application can inject holidays and an optional validity range.

```text
weekday(2025-03-08)                  → Saturday
is_workday(2025-03-10)              → true
2025-03-07 + 1 workday              → 2025-03-10
add_workdays(2025-03-07, 5)         → 2025-03-14
workdays_between(2025-03-10, 2025-03-17)
                                      → 5 workdays
next workday after 2025-03-07       → 2025-03-10
```

A bounded country holiday calendar rejects dates outside its declared range instead of assuming missing holidays.

### Timezones

Fixed forms include `Z`, numeric offsets, and the built-in abbreviations used by the parser, including UTC/GMT and common CET/CEST, EET/EEST, UK, North American, Australian, New Zealand, and Asian abbreviations.

```text
10:00 CET in UTC                     → 09:00Z
time between 10:00 CEST and 10:00 CET
                                      → 1 h
```

IANA zone names require a caller-provided `TimeZoneSnapshot`. The snapshot has a fixed coverage interval. Unknown zones and times outside that interval are errors. Local times in DST folds or gaps are not guessed: ambiguous and nonexistent local times receive distinct diagnostics.

The CLI creates an explicit snapshot from host zone data. If host-zone discovery fails, it warns before falling back to UTC.

## Dice

`NdS` creates a typed distribution for `N` fair dice with `S` sides. `d20` is shorthand for `1d20`. Integer modifiers are supported.

```text
2d6                          → 2d6
D20                          → 1d20
2d6 + 3                      → 2d6+3
expected_value(2d6 + 1)      → 8
probability(2d6, 7)          → 16.6666666667%
```

Sampling is explicit in the language:

```text
roll 3d12
roll a 3d12
roll the 3d12
roll me 3d12
roll me a 3d12
roll(3d12)

flip coin
flip a coin
flip the coin
coin flip
coinflip
coinflip()
```

The roll forms are equivalent, as are the coin forms. A roll produces an integer; a flip produces the typed value `heads` or `tails`. Modifiers are part of the sampled distribution, so `roll 2d6 + 3` samples `2d6+3`.

The words `roll`, `flip`, and `coin` (when followed by `flip`) are reserved and cannot be used as variable or function names.

Probabilities are exact internally. Entropy is not: sampling requires an explicit source. The CLI uses operating-system entropy. Rust callers use `calculate_with_entropy`, `calculate_with_context_and_entropy`, `CalculationSession::calculate_with_entropy`, or the corresponding document functions. Calling an ordinary deterministic evaluation function with a sampling expression produces `MissingEntropy`.

At the lower-level API, callers can use `DiceDistribution::roll` or `flip_coin` directly with an iterator or another `EntropySource`. Rejection sampling prevents modulo bias, and exhausted entropy is an error.

Dice bounds are listed under [Diagnostics and limits](#diagnostics-and-limits).

## Variables and functions

### Variables and previous answers

Assignment uses `=` at the document or session level:

```text
principal = 1000
growth = 10%
principal + growth           → 1100
```

Names are case-insensitive. Reassignment is allowed and affects subsequent lines. Values retain their type.

A `CalculationSession` also binds the previous successful answer to both `ans` and `_`. `clear` removes session variables, user functions, and previous-answer bindings.

Unknown session names can include a nearby-name suggestion in the diagnostic.

### User functions

Named functions are defined in documents or sessions:

```text
apply_rate(amount, rate) = amount * (1 + rate)
apply_rate(100, 20%)          → 120
```

Anonymous functions use `->` or `→`:

```text
(x -> x * x)(5)              → 25
((x, y) -> x + y)(2, 3)      → 5
```

Arguments are values, parameter names are local and case-insensitive, duplicate parameters are invalid, and arity is checked. Recursive user functions are deliberately unsupported and produce `RecursiveFunction` rather than recursing without a bound.

## Built-in functions

All names and aliases are case-insensitive. The tables show call syntax; many functions also accept a natural `function of value` form.

### Aggregation and general operations

| Function | Arguments and result |
| --- | --- |
| `sum(a, ...)` | Sum of one or more mutually addable values |
| `average(a, ...)`, `avg(...)` | Arithmetic mean |
| `minimum(a, ...)`, `min(...)` | Least comparable value |
| `maximum(a, ...)`, `max(...)` | Greatest comparable value |
| `median(a, ...)` | Median; even counts average their two middle values |
| `count(a, ...)` | Number of arguments |
| `product(a, ...)` | Product of arguments |
| `roll(dice)` | Sample a dice distribution using supplied entropy |
| `coinflip()` | Sample a typed coin side using supplied entropy |
| `abs(x)` | Absolute value |
| `clamp(x, low, high)` | `x` constrained to inclusive bounds |
| `midpoint(a, b)` | `(a + b) / 2` |
| `proportion(a, b, c)` | Solve `a : b = c : ?` |

Natural list syntax is supported for aggregates:

```text
sum of 10, 20, and 30        → 60
average of 10 and 30         → 20
clamp 30 between 5 and 25    → 25
6 is to 60 as 25 is to what  → 250
```

### Rounding

| Function | Behavior |
| --- | --- |
| `round(x)` | Round to an integer |
| `round(x, places)` | Round to decimal places; negative places round left of the decimal |
| `floor(x)` | Greatest integer not above `x` |
| `ceiling(x)`, `ceil(x)` | Least integer not below `x` |

Rounding preserves compatible typed wrappers such as money or units where meaningful.

### Roots, logarithms, trigonometry, and hyperbolic functions

| Group | Functions |
| --- | --- |
| Roots | `sqrt(x)`, `cbrt(x)`, `root(x, n)` |
| Logarithms | `ln(x)`, `log(x)`/`log10(x)`, `log2(x)` |
| Trigonometry | `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2` |
| Exponential | `exp(x)` |
| Hyperbolic | `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh` |

Trigonometric inputs can be angle units. Bare numeric inputs use radians. Inverse trigonometric functions produce angular results according to their implemented typed behavior. Domain violations are explicit diagnostics.

Natural forms include `square root of`, `cube root of`, `natural log of`, `common log of`, `binary log of`, `sine of`, and hyperbolic equivalents.

### Integer and combinatoric functions

| Function | Meaning |
| --- | --- |
| `factorial(n)` | `n!` |
| `permutation(n, r)`, `perm(n, r)` | Ordered selections |
| `combination(n, r)`, `choose(n, r)` | Unordered selections |
| `gcd(a, ...)` | Greatest common divisor |
| `lcm(a, ...)` | Least common multiple |
| `is_prime(n)`, `prime(n)` | Primality boolean |
| `bin(n)`, `binary(n)` | Base-2 typed integer |
| `oct(n)`, `octal(n)` | Base-8 typed integer |
| `hex(n)`, `hexadecimal(n)` | Base-16 typed integer |
| `int(n)`, `integer(n)` | Ordinary decimal integer |

Natural forms include `greatest common divisor of`, `least common multiple of`, and `is prime`.

### Calendar and elapsed-time functions

| Function | Meaning |
| --- | --- |
| `weekday(date)` | Weekday name |
| `is_workday(date)` | Contextual workday boolean |
| `add_workdays(value, count)` | Add signed workdays |
| `workdays_between(start, end)` | Signed workday count |
| `next_workday(value)` | Strictly following workday |
| `previous_workday(value)` | Strictly preceding workday |
| `time_between(a, b)`, `time_from(a, b)` | Absolute elapsed duration |
| `time_since(a)`, `time_until(a)` | Elapsed duration relative to contextual now |
| `calendar_between(a, b)`, `calendar_from(a, b)` | Calendar-component difference |
| `calendar_since(a)`, `calendar_until(a)` | Calendar difference relative to contextual now |

Finance and dice functions are catalogued in their own sections above.

## Conversions and output

### Conversion syntax

`in`, `to`, and `as` introduce explicit conversions. Targets include:

- compatible units and compound units;
- ISO currencies when rates are available;
- `number`, `decimal`, or `dec` to strip a typed magnitude;
- `percent`, `percentage`, or `%`;
- `fraction`;
- `binary`, `octal`, `hex`, `decimal`, or `base N` for exact integers;
- `roman`;
- fixed offsets and named timezones.

Conversions associate with the value to their left. Parenthesize when converting part of a larger expression.

### Number output modes

Formatting never changes the stored numeric value.

| Mode | Behavior | Example for `1 / 3` or `12345.67` |
| --- | --- | --- |
| `Auto` | Readable decimal where practical, otherwise bounded decimal | `0.3333333333` |
| `Plain` | Canonical unlocalized exact representation | `1/3` |
| `Exact` | Exact fraction when the value is exact | `1/3` |
| `Fraction` | Fraction form | `1/3` |
| `Mixed` | Mixed-number form | `2 1/3` for `7 / 3` |
| `Decimal { places }` | Fixed requested decimal places | `0.3333` at 4 places |
| `Significant { digits }` | Requested significant digits | `12300` at 3 digits |
| `Scientific { digits }` | Scientific notation | `1.235e4` at 4 digits |

When `mark_approximate` is enabled, rounded output is prefixed with `≈`:

```text
1 / 3 in decimal mode at 4 places
                              → ≈0.3333
```

Locale-aware formatting is controlled independently through `NumberFormat`, including decimal and grouping separators. `Plain` deliberately bypasses presentation-oriented locale formatting.

Typed formatters choose canonical symbols and readable compound-unit forms. Temporal output omits zero components and unnecessary seconds while preserving nanoseconds when present.

## Context and determinism

`CalculationContext` is the complete boundary for data that can affect evaluation:

| Context area | Supplied state |
| --- | --- |
| Numeric | Locale separators and grouping |
| Output | Number mode, precision, approximation marker |
| Monetary | Default currency, atomic exchange-rate snapshot, freshness policy |
| Temporal | Current date-time, default offset, date order, workday calendar, timezone snapshot |
| Units | Custom unit registry |
| Finance data | Price-index snapshots and crypto quote observations through explicit APIs |

With the same source, context, and supplied entropy sequence, `calcite-core` returns the same result. It performs no I/O and consults no process-global locale, host clock, host timezone, network, filesystem, or random generator.

This means:

- `today` without a current date-time is an error;
- an IANA zone without snapshot data is an error;
- an unavailable currency pair is an error;
- a random roll requires caller-provided entropy (the CLI supplies OS entropy at its boundary);
- stale snapshots are rejected only according to the caller's explicit policy.

`calcite-cli` is the application boundary that supplies host time, loads timezone data, reads configuration, manages history and caches, and optionally calls rate providers.

## Diagnostics and limits

A failed calculation returns one `Diagnostic` with:

- a stable `DiagnosticKind`;
- a byte `Span` into the original UTF-8 source;
- a human-readable message;
- optional detail, such as an expected token, currency pair, or nearby variable suggestion.

Diagnostic kinds cover lexical and parse errors, invalid literals, missing temporal or timezone context, DST ambiguity and gaps, out-of-range snapshots, variable and function errors, finance domains, resource exhaustion, arithmetic domains, integer requirements, dice, shifts, real/complex restrictions, unsupported typed operations, unit mismatches, and currency/rate failures.

Notable messages include:

```text
current date or time was not provided
no exchange rate was provided
expression exceeds Calcite's resource limits
recursive user functions are not supported
random entropy was not provided
invalid string literal
```

Provider transport and decoding failures are not evaluator diagnostics. They are errors from `calcite-rates` or the hosting application.

### Resource boundaries

Limits are deterministic and fail with diagnostics rather than attempting unbounded work:

| Resource | Limit |
| --- | ---: |
| Source text | 1,000,000 bytes |
| Parse nesting | 64 levels |
| Digits in a numeric literal | 10,000 |
| Literal exponent magnitude | 100,000 |
| Exact integer exponent magnitude | 100,000 |
| Decimal rounding places | 10,000 |
| Root degree | 1,000 |
| Exponential argument magnitude | 1,000 |
| Factorial input | 10,000 |
| Combinatoric population | 100,000 |
| Combinatoric selection | 10,000 |
| Shift count | 100,000 |
| User-written unit exponent | -100 through 100 |
| Roman numeral value | 1 through 3,999 |
| Dice count | 100 |
| Sides per die | 1,000,000 |
| Materialized dice distribution states | 100,000 |
| Amortization schedule payments | 1,200 |
| Primality input | unsigned 64-bit integer range |

Some operations can hit a tighter derived bound before the listed individual maximum—for example, a dice count and side count whose combined distribution would exceed the state limit.
