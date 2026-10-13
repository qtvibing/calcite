# calcite-rates

Optional exchange-rate loading for `calcite-core`. Evaluation never performs network access. Providers include the official ECB daily feed and Frankfurter v2, which aggregates rates from central banks without requiring an API key.

Core only, with caller-injected snapshots:

```toml
[dependencies]
calcite-core = "0.1"
```

Timestamped JSON files:

```toml
[dependencies]
calcite-core = "0.1"
calcite-rates = "0.1"
```

Only the official ECB provider:

```toml
[dependencies]
calcite-core = "0.1"
calcite-rates = { version = "0.1", default-features = false, features = ["ecb"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Multiple providers with explicit fallback order:

```toml
[dependencies]
calcite-core = "0.1"
calcite-rates = { version = "0.1", features = ["ecb", "frankfurter"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Applications choose providers and cache locations, fetch asynchronously, then atomically install the returned `ExchangeRateSnapshot`. `FallbackProvider` selects the first complete successful snapshot; it never merges rates with different timestamps or provenance.

See the [root README](../../README.md) for CLI behavior and the [language reference](../../docs/reference.md#money-and-exchange-rates) for currency semantics.
