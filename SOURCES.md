# Available Price Sources

This document lists the price sources the oracle actually supports, and the exact
configuration format used to attach them to an asset.

> Source dispatch lives in [`sources/src/sources.rs`](sources/src/sources.rs)
> (`fetch_all_sources` for a single asset, `fetch_all_sources_batch` for a whole set —
> one request per source instead of one per asset); the config type is `ExchangeConfig` in
> [`sources/src/lib.rs`](sources/src/lib.rs). Adding a key that no source reads is
> silently ignored (unknown fields are dropped on parse), so keep this list in sync
> with the code.

## Per-asset configuration format

An asset's sources are configured as a single JSON object (an `ExchangeConfig`),
**not** a list of `{name, id}` entries. Each supported exchange is an optional field;
a source is queried only if its field is present. This is exactly the shape stored in
[`tokens.json`](tokens.json) and pushed on-chain via the `SetAssetExchangeConfig`
governance action.

```json
{
  "decimals": 24,
  "coingecko": "near",
  "binance": "NEARUSDT",
  "binance_us": "NEARUSD",
  "huobi": "nearusdt",
  "cryptocom": "NEAR_USDT",
  "kucoin": "NEAR-USDT",
  "gate": "near_usdt",
  "pyth": "0xc415de8d2eba7db216527dff4b60e8f3a5311c740dadb233e13e12547e226750",
  "chainlink": "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419",
  "kraken": "NEARUSD",
  "coinbase": "NEAR-USD",
  "bitstamp": "NEAR/USD",
  "okx": "NEAR-USDT",
  "bitget": "NEARUSDT",
  "mexc": "NEARUSDT",
  "stablecoin": false
}
```

`decimals` and `stablecoin` are metadata (used by the UI / aggregation), not sources.

## Supported sources

| Field | Source | ID format | Example |
|-------|--------|-----------|---------|
| `coingecko` | CoinGecko API | CoinGecko coin id | `"near"`, `"bitcoin"`, `"tether"` |
| `binance` | Binance global | trading symbol | `"NEARUSDT"` |
| `binance_us` | Binance US | trading symbol | `"NEARUSD"` |
| `binance_alpha` | Binance Alpha (wallet token list) | BSC contract address | `"0x…"` |
| `pyth` | Pyth Network (Hermes) | price feed id (`0x` optional) | `"0xc415…6750"` |
| `chainlink` | Chainlink (Ethereum aggregator) | aggregator contract address | `"0x5f4eC3…8419"` |
| `huobi` | Huobi / HTX | lowercase symbol | `"nearusdt"` |
| `kucoin` | KuCoin | dash symbol | `"NEAR-USDT"` |
| `gate` | Gate.io | lowercase underscore pair | `"near_usdt"` |
| `cryptocom` | Crypto.com | underscore instrument | `"NEAR_USDT"` |
| `kraken` | Kraken | **canonical** pair name | `"NEARUSD"`, `"XXBTZUSD"` |
| `coinbase` | Coinbase Exchange | product id (fiat USD) | `"NEAR-USD"` |
| `bitstamp` | Bitstamp | slashed pair (fiat USD) | `"NEAR/USD"` |
| `okx` | OKX | dash instrument id | `"NEAR-USDT"` |
| `bitget` | Bitget | plain symbol | `"NEARUSDT"` |
| `mexc` | MEXC | plain symbol (USDT only) | `"NEARUSDT"` |

Quote currency matters: `kraken`, `coinbase` and `bitstamp` quote **real fiat USD**, so they
price a stablecoin independently of USDT. `okx`, `bitget` and `mexc` are USDT-quoted (except
`okx`'s `USDT-USD`), so their reading carries whatever USDT is worth at the time.

Notes:
- **CoinGecko** accepts an optional API key via the `API_KEY` secret (sent as
  `x_cg_pro_api_key`).
- **Pyth** requires one: Hermes serves prices only with `Authorization: Bearer <key>`, and
  keys are issued by [Pyth Terminal](https://pythdata.app/signup) with per-feed grants (a key
  without a grant for a feed gets `403 Not entitled`). The key lives in the `PYTH_API_KEY`
  secret — its own name, never the CoinGecko `API_KEY`, since the two go to different
  providers. Without the secret Pyth is not requested at all: no call, no per-token error,
  the venue is simply absent from every record, exactly as if no asset configured it. The
  `pyth` entries in the asset configs can therefore stay in place while the key is missing.
  A request that names Pyth explicitly (`fetch_external` / `fetch_custom_data` with source
  `Pyth`) is refused with a message naming the secret instead.
- All other exchanges are keyless.
- **Chainlink** reads Ethereum RPC with automatic multi-RPC failover; if every RPC fails,
  Chainlink is skipped for that run and an alert is sent. A single asset is read with
  `latestAnswer()`; a set of assets is read in one `eth_call` through Multicall3
  (`aggregate3` with `allowFailure = true`, `latestRoundData()` per feed), so a delisted
  feed is reported on its own instead of taking the other feeds down with it.
- **Binance** requests go to `data-api.binance.vision`, which serves the same public
  market-data API as `api.binance.com` — the latter answers HTTP 451 from geo-blocked
  regions, including our worker egress.
- **Kraken** uses legacy asset codes, so the pair is often not what you would guess:
  `XXBTZUSD`, `XETHZUSD`, `XXRPZUSD`, `XDGUSD` (Dogecoin), `XXLMZUSD`, `XLTCZUSD`,
  `XZECZUSD`, while newer listings are plain (`NEARUSD`, `SOLUSD`, `USDCUSD`, `WBTCUSD`).
  Store the **canonical** name: Kraken answers under it whatever alias you request, and the
  batch parser only falls back to alias matching when the canonical name does not match.
  Kraken also drops the *entire* batch (HTTP 200, `error: ["EQuery:Unknown asset pair"]`)
  when one pair is unknown, so a bad entry triggers a per-pair retry rather than losing
  Kraken for every asset.
- **Coinbase** has no batch form of `/ticker`, so a set of assets is read from
  `/products/stats`. That endpoint is cached ~5s against the ticker's ~1s and therefore
  trails it slightly (measured: 3.68 bps median divergence, 8.60 bps max) — well inside the
  freshness SLA. `/products` also lists delisted markets whose `/ticker` answers HTTP 400;
  those are absent from `/products/stats`, and a missing or zero stat is treated as a
  missing source, never as a price.
- **Bitget** carries its price in `lastPr`, not `last`.
- **MEXC**'s `ticker/price` endpoint quotes USDT pairs only and reports no timestamp.
- **FRAX is deliberately configured without any exchange source.** After Frax's April-2025
  rebrand the ticker `FRAX` on exchanges is the *governance* token (~$0.27, verified live on
  MEXC), while the asset the oracle prices (`0x853d955a…`) is the *stablecoin* (~$0.99).
  Attaching a CEX `FRAX` symbol would feed a ~73%-wrong price into a lending oracle, so FRAX
  stays address-based (`chainlink` / `coingecko`) only.

## Source-side freshness

Only **Pyth** exposes an upstream data timestamp that the oracle checks: prices whose
`publish_time` is older than 120 seconds are rejected
([`sources/src/sources.rs`](sources/src/sources.rs), `fetch_pyth`). Every other
endpoint above is stamped with the fetch time and is only as fresh as the moment it was
read — including `bitstamp`, `okx` and `bitget`, whose batch endpoints *do* report an
upstream time that the parsers carry but the aggregator does not act on. See
[the platform freshness docs](https://app.outlayer.ai/docs/tee-attestation#data-freshness)
for how consumers should reason about age.

## Refresh tiers

Sources are not all refreshed together. Each stored price keeps the **per-source observation
time**, and a refresh merges into that record rather than replacing it, so venues can run at
different cadences:

| Tier | Venues | Assets | Interval |
|------|--------|--------|----------|
| fast | everything except the slow tier | `PRIORITY_ASSETS` | `UPDATE_INTERVAL_PRIORITY_SECS` |
| full | everything except the slow tier | all | `UPDATE_INTERVAL_SECS` |
| slow | `SLOW_SOURCES` (`pyth`, `chainlink`) | all that configure them | `SLOW_SOURCE_INTERVAL_SECS` |

The intervals are scheduler configuration and are deliberately not quoted here as numbers — they
live in one `.env` on one host, and a figure copied into documentation outlives the setting it
described. Read the effective values off the scheduler's startup log, or off the per-source
timestamps in any stored record.

The split exists because cost per refresh is wildly uneven: the exchange endpoints answer for
the whole asset set in one request, while Chainlink is an EVM `eth_call` through Multicall3 and
Pyth a separate API. Paying for those on every cycle is what made a fast cadence unaffordable.

Consumers pick their own window with `max_age_secs`, which filters **sources**: a request for 40
seconds is answered from the venues seen within 40 seconds — the slow tier does not participate —
and the returned `publish_time` is the oldest source that did. Widening the window lets the slow
tier back in and moves `publish_time` accordingly. A source that stops answering keeps its last
observation for 15 minutes and then drops out of the record entirely.

The window is anchored to the moment the request is evaluated. Which sources vote — on a cache
hit and on the rebuild after a refetch alike — and whether `get_signed_prices` agrees to sign
the result are one predicate taken against that one instant. The seconds the call itself takes
to run are not counted against the caller's window, so a price that was inside `max_age_secs`
when selected is never refused for having aged during the call; a consumer that checks
`publish_time` on its own side should allow for its request latency on top of the window it
asked for.

A fetch does not change this. A plain refetch writes a merged record — what it observed plus the
venues it did not reach, retained from earlier refreshes — and a request that excludes sources
fetches without writing at all; either way the answer is the caller's window applied to what
the fetch produced, never that set stamped with the fetch time. A venue outside the window
neither votes nor ages `publish_time`, whether it is a retained entry from an earlier refresh or
a Pyth reading fetched this instant whose own `publish_time` is older than the window. Fewer
venues inside the window than `min_sources_num` is an error for that asset; the venues outside
it do not make up the numbers.

## Custom data sources

Arbitrary HTTP endpoints are **not** part of `ExchangeConfig`. They are fetched through
the `request_custom_data` / `custom_call` path using a `CustomSourceConfig`, which
supports GET/POST, request bodies, custom headers, and dot/array JSON-path extraction:

```json
{
  "custom": {
    "url": "https://api.example.com/price",
    "json_path": "data.price",
    "value_type": "number",
    "method": "GET",
    "headers": []
  }
}
```

- `value_type`: `"number"` (default), `"string"`, or `"boolean"`.
- `method`: `"GET"` (default) or `"POST"` (add a `body` object for POST).
- `json_path`: dot notation with array indexes, e.g. `"1245620.data.price_overview.final"`.
- Custom-source URLs are validated against local/private network access (SSRF guard).

See [CUSTOM_POST_BODY.md](CUSTOM_POST_BODY.md) for POST-body examples.
