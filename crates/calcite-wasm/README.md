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
