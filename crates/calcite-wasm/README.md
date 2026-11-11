# calcite-wasm

WebAssembly bindings for [calcite-core](../calcite-core), for use in browsers
and other JavaScript runtimes.

Two variants exist:

- default: no time-zone database. Named zones and abbreviations report an
  error; explicit offsets, dates, and times work.
- `tzdb` feature: the full IANA time-zone database, embedded. Larger download.

## Usage

The release tarball contains `calcite.js`, `calcite_bg.wasm`, and type
declarations. Serve them together and import the ES module:

```js
import init, { calculate, Session } from './calcite.js';

await init();

const result = calculate('15% of 240 eur in jpy');
// { ok: true, output: "...", value: { type: "money", ... } }

const session = new Session();
session.calculate('hotel = $199 * 3');
session.calculate('hotel + $50');
```

Errors throw a `{ ok: false, error: { kind, message, span } }` object.

## Currency rates

The module never makes network requests. The host fetches rates from a source
of its choice and hands them to the session in calcite's snapshot format:

```js
const session = new Session();

async function refreshRates() {
  const data = await (await fetch('https://api.frankfurter.dev/v1/latest?base=USD')).json();
  session.setRates(JSON.stringify({
    as_of: data.date + 'T00:00:00Z',
    rates: Object.entries(data.rates).map(([target, rate]) =>
      ({ source: data.base, target, rate: String(rate) })),
  }));
}

function calculateWithRates(expression) {
  try {
    return session.calculate(expression);
  } catch (error) {
    if (error?.error?.kind !== 'MissingExchangeRate') throw error;
  }
  return refreshRates().then(() => session.calculate(expression));
}
```

`setRates` throws `{ ok: false, error: { kind: "InvalidRates" } }` when the
JSON does not describe a valid snapshot. Refresh timing is the host's choice:
refetch when a conversion reports `MissingExchangeRate`, or on a schedule
(the calcite CLI refreshes its cache every 12 hours).

The current date, time, and UTC offset come from the browser's clock. Dice and
coin flips use `Math.random`, which is not cryptographically secure.

## Local build

```
cargo build -p calcite-wasm --target wasm32-unknown-unknown --profile release-wasm
wasm-bindgen --target web --out-name calcite \
  --out-dir pkg target/wasm32-unknown-unknown/release-wasm/calcite_wasm.wasm
wasm-opt -Oz -o pkg/calcite_bg.wasm pkg/calcite_bg.wasm
```

Add `--features tzdb` to the build for the time-zone variant. Open
`example.html` from a static file server alongside the `pkg` output to try it.
