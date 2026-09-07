# TEE-Secured Price Oracle

> **[Full documentation](https://app.outlayer.ai/docs/examples#oracle-example)** on the OutLayer dashboard.

**On-Demand Oracle with Sustainable Economics**

Based on [OutLayer](https://app.outlayer.ai) — verifiable compute and custody for NEAR Protocol.

[Dashboard & Playground](https://price-oracle.outlayer.ai/) | [GitHub](https://github.com/out-layer/oracle-example)

---

## How It Works

TEE-Secured Price Oracle delivers cryptocurrency prices with **instant response times** and **zero trust** in external operators.

### Key Principles

1. **Warm Prices in TEE** — Prices are pre-fetched and cached inside TEE (Trusted Execution Environment). When your contract requests a price, it's delivered instantly without waiting for external API calls.

2. **Zero Trust** — All price fetching and aggregation happens exclusively inside Intel TDX enclave. No external operator ever sees or touches the raw price data.

3. **Permissionless Updates** — An external scheduler monitors price freshness and triggers TEE updates when:
   - Prices become stale (time-based, default: 60 seconds)
   - High volatility detected (price deviation > 1% from stored value)

   Anyone can run their own scheduler or trigger updates manually.

   **This extends to the on-chain push.** Anyone can call the WASI worker with
   `update_prices` + `update_contract: true` and have fresh prices written to the oracle
   contract — no permission needed, and it works even if our scheduler is down. The caller
   pays only for the WASI execution; they cannot influence the price, because the worker
   fetches and aggregates the sources itself inside the enclave, and the resulting
   `report_prices` transaction is signed by a TEE-generated key whose private half never
   leaves the enclave. The contract accepts a report only from the `push_signer_accounts`
   registered for that asset, so a caller-supplied price is impossible by construction.

   Two things bound the cost of that openness: the worker skips any asset reported to the
   contract less than **20 seconds** ago (so repeated triggers cannot spam transactions),
   and gas comes from the push signer's implicit account, which is funded deliberately —
   an empty balance simply means no on-chain push, while public-storage prices keep
   updating. The result is liveness without a trusted operator: the feed does not stop
   because one scheduler stopped.

4. **On-Demand Fallback** — If cached prices are stale, the WASI worker fetches fresh data first, then returns it. You always get a price.

> **Note:** The scheduler runs on **mainnet** only. On testnet, prices are fetched on-demand to save TEE resources. Anyone can run their own scheduler for testnet using the `scheduler/` directory.

### Contract Integration Flow

How your contract gets prices via `oracle_call`:

```
┌──────────────────┐                          ┌─────────────────┐
│   Your Contract  │                          │  Price Oracle   │
│                  │                          │  price-oracle.  │
└────────┬─────────┘                          └────────┬────────┘
         │                                             │
         │  1. oracle_call(receiver_id, asset_ids)     │
         │ ─────────────────────────────────────────>  │
         │         (attached: 0.02 NEAR)               │
         │                                             │
         │                             ┌───────────────┴───────────────┐
         │                             │ 2. yield: request to OutLayer │
         │                             │    (if cache stale)           │
         │                             └───────────────┬───────────────┘
         │                                             │
         │                                             ▼
         │                             ┌───────────────────────────────┐
         │                             │      TEE Worker (Intel TDX)   │
         │                             │   - Fetch from 9+ sources     │
         │                             │   - Aggregate (median)        │
         │                             │   - Return to OutLayer        │
         │                             └───────────────┬───────────────┘
         │                                             │
         │                             ┌───────────────┴───────────────┐
         │                             │ 3. resume: continue execution │
         │                             └───────────────┬───────────────┘
         │                                             │
         │  4. oracle_on_call(sender_id, data, msg)    │
         │ <─────────────────────────────────────────  │
         │         (callback with PriceData)           │
         │                                             │
         ▼                                             ▼
```

**If cache is fresh:** Steps 2-3 are skipped, callback happens immediately.

### Internal Architecture

How the scheduler keeps prices warm in TEE:

```
┌─────────────────┐     monitors      ┌──────────────────────┐
│    Scheduler    │ ───────────────>  │   TEE Public Storage │
│    (external)   │                   │   - price:wrap.near  │
└────────┬────────┘                   │   - price:eth.bridge │
         │                            │   - price:nbtc...    │
         │ if stale or deviation      │  each with per-source│
         ↓                            │  observation times   │
┌─────────────────┐                   └──────────────────────┘
│    OutLayer     │   execute WASI    ┌──────────────────────┐
│   Coordinator   │ ───────────────>  │     TEE Worker       │
└─────────────────┘                   │     (Intel TDX)      │
                                      │                      │
                                      │  Fetches the tier's  │
                                      │  venues, 1 req each  │
                                      │  ↓                   │
                                      │  MERGES into the     │
                                      │  stored record       │
                                      │  ↓                   │
                                      │  Aggregates (median) │
                                      │  ↓                   │
                                      │  Stores in TEE       │
                                      │  ↓                   │
                                      │  Can update contract │
                                      └──────────────────────┘
```

Sources run in tiers: the cheap all-ticker venues refresh fastest for the priority assets and on
a wider interval for the rest, while Pyth and Chainlink — an EVM `eth_call` and a separate API —
run on their own slower cycle. Because a refresh merges rather than replaces, all of them
accumulate into one record and every consumer picks how far back it is willing to look. The
intervals are scheduler configuration, not a property of the feed; the per-source timestamps in
each record are what a consumer should actually read. See [SOURCES.md](SOURCES.md#refresh-tiers).

---

## Deployments

| Contract | Testnet | Mainnet |
|----------|---------|---------|
| **Price Oracle** (main) | `price-oracle.testnet` | `price-oracle.near` |
| **Simple Wrapper** | `price-oracle-wrapper.testnet` | `price-oracle-wrapper.near` |
| **Pyth-Compatible Wrapper** | `price-oracle-pyth.testnet` | `price-oracle-pyth.near` |

**OutLayer Project ID:**
- Mainnet: `price-oracle.near/price-oracle`
- Testnet: `zavodil2.testnet/price-oracle`

The production oracle is invoked **through the project**, not through this GitHub repository: calls
go to `POST /call/{owner}/{project}` (or on-chain with a
`{"Project": {"project_id": "price-oracle.near/price-oracle"}}` code source), and OutLayer resolves
both the WASM and the secrets from the project. The mainnet secrets reference is
`{"profile": "oracle", "account_id": "price-oracle.near"}` — a profile name and an account, not a
secret; the values themselves stay encrypted on-chain and are decrypted only inside the TEE.

This matters when reading the examples below: anything that declares a `GitHub` code source binds
its secrets to `github.com/owner/repo`, so a fork or a rename needs its secrets re-registered. The
project path has no such coupling.

**Dashboard:** https://price-oracle.outlayer.ai/
- Live prices display
- Playground — run transactions and see how the oracle works
- Documentation portal — integration guides and API reference

---

## Quick Start

### Request Prices (recommended)

This is an **on-demand oracle** — prices are fetched when you need them. Use `oracle_call` to get prices with a callback:

```bash
near call price-oracle.near oracle_call '{
  "receiver_id": "your-contract.near",
  "asset_ids": ["wrap.near", "aurora"],
  "msg": ""
}' --accountId your.near --deposit 0.02 --gas 200000000000000
```

Your contract receives prices via `oracle_on_call` callback.

### View Cached Prices (unreliable)

> **Warning:** Cached prices are only available if someone recently paid for an update. Due to the **on-demand nature** of this oracle, the cache is usually empty or stale — prices are fetched when needed for specific operations (liquidations, borrowing, swaps), not stored permanently.
>
> **This is by design:** Unlike traditional oracles with a central price feed contract, this oracle delivers prices directly to your contract via callback. Any contract can integrate without intermediaries. **Don't rely on view methods for production** — use `oracle_call` or `request_price_data` instead.

```bash
# May return null prices if cache is stale!
near view price-oracle.near get_price_data '{"asset_ids": ["wrap.near"]}' --networkId mainnet
```

### Price Format

**Conversion:** `multiplier / 10^decimals = USD`
- `500000000 / 10^8 = $5.00` (NEAR)
- `320000000000 / 10^8 = $3200.00` (ETH)

---

## Integration Guide #1: Price Oracle Contract

For DeFi protocols integrating price feeds.

### View Methods (free)

| Method | Arguments | Description |
|--------|-----------|-------------|
| `get_price_data` | `asset_ids?: string[]` | Get cached prices. Returns `null` for stale assets. |
| `can_subsidize_outlayer_calls` | — | Returns `true` if contract pays for OutLayer calls |
| `get_oracle_price_data` | `account_id, asset_ids?, recency_duration_sec?` | Get prices from specific oracle |
| `get_asset` | `asset_id` | Get asset configuration |
| `get_assets` | `from_index?, limit?` | List all registered assets |

### Call Methods

| Method | Deposit | Description |
|--------|---------|-------------|
| `request_price_data` | 0.01+ NEAR | Get prices directly (returns `PriceData`) |
| `oracle_call` | 0.01+ NEAR | Get prices with callback to your contract |
| `request_custom_data` | 0.01+ NEAR | Fetch arbitrary external data |
| `custom_call` | 0.01+ NEAR | Custom data with callback |

> **Subsidized Mode:** If contract has >20 NEAR and subsidy is enabled, calls are free.

### Data Types

```rust
/// Price format: multiplier / 10^decimals = USD
struct Price {
    multiplier: u128,  // e.g., 500000000 for $5.00
    decimals: u8,      // usually 8
}

/// Response from get_price_data / oracle_call
struct PriceData {
    timestamp: u64,              // nanoseconds
    recency_duration_sec: u32,   // max age for "fresh" prices
    prices: Vec<AssetOptionalPrice>,
}

struct AssetOptionalPrice {
    asset_id: String,
    price: Option<Price>,  // None if stale/unavailable
}
```

### Callback Interface

Your contract must implement `oracle_on_call` to receive prices from `oracle_call`:

```rust
use near_sdk::{near_bindgen, AccountId};

#[near_bindgen]
impl Contract {
    pub fn oracle_on_call(
        &mut self,
        sender_id: AccountId,
        data: PriceData,
        msg: String,
    ) {
        // Verify caller is the oracle
        assert_eq!(
            env::predecessor_account_id(),
            "price-oracle.near".parse::<AccountId>().unwrap(),
            "Only oracle can call"
        );

        // Process prices
        for asset_price in data.prices {
            if let Some(price) = asset_price.price {
                let price_usd = price.multiplier as f64
                    / 10f64.powi(price.decimals as i32);
                // Use price_usd...
            }
        }
    }
}
```

For `custom_call`, implement `on_custom_data`:

```rust
pub fn on_custom_data(
    &mut self,
    sender_id: AccountId,
    data: Vec<CustomDataResult>,
    msg: String,
)
```

### Rust Integration Example

```rust
use near_sdk::{ext_contract, AccountId, Gas, NearToken, Promise};

#[ext_contract(ext_oracle)]
pub trait Oracle {
    fn oracle_call(
        &mut self,
        receiver_id: AccountId,
        asset_ids: Option<Vec<String>>,
        msg: String,
        resource_limits: Option<serde_json::Value>,
    ) -> Promise;
}

impl Contract {
    pub fn get_prices_with_callback(&self) -> Promise {
        ext_oracle::ext("price-oracle.near".parse().unwrap())
            .with_attached_deposit(NearToken::from_millinear(20)) // 0.02 NEAR
            .with_static_gas(Gas::from_tgas(150))
            .oracle_call(
                env::current_account_id(),
                Some(vec!["wrap.near".to_string(), "aurora".to_string()]),
                "swap".to_string(),
                None,
            )
    }
}
```

### JavaScript Integration Example

```typescript
import { connect, Contract, keyStores } from 'near-api-js';

const oracle = new Contract(account, 'price-oracle.near', {
  viewMethods: ['get_price_data', 'can_subsidize_outlayer_calls'],
  changeMethods: ['request_price_data', 'oracle_call'],
});

// View cached prices (free)
const cached = await oracle.get_price_data({
  asset_ids: ['wrap.near', 'aurora'],
});

// Request fresh prices
const fresh = await oracle.request_price_data(
  { asset_ids: ['wrap.near'] },
  '200000000000000', // 200 TGas
  '20000000000000000000000', // 0.02 NEAR
);

// Convert price
const nearPrice = cached.prices[0].price;
const priceUsd = Number(nearPrice.multiplier) / Math.pow(10, nearPrice.decimals);
console.log(`NEAR = $${priceUsd}`);
```

### EMA Prices

Request Exponential Moving Average by appending `#<period_sec>` to asset ID:

```bash
near view price-oracle.near get_price_data '{"asset_ids": ["wrap.near#3600"]}'
```

This returns 1-hour EMA for NEAR.

---

## Integration Guide #2: Pyth-Compatible Wrapper

**Drop-in replacement** for `pyth-oracle.near`. Switch oracles with zero code changes.

### Contract

| Network | Address |
|---------|---------|
| Testnet | `price-oracle-pyth.testnet` |
| Mainnet | `price-oracle-pyth.near` |

### View Methods

| Method | Description |
|--------|-------------|
| `get_price(price_identifier)` | Get price with staleness check |
| `get_price_unsafe(price_identifier)` | Get price without staleness check |
| `get_price_no_older_than(price_id, age)` | Get price with custom max age |
| `get_ema_price(price_id)` | Get EMA price |
| `list_prices(price_ids)` | Batch get multiple prices |
| `price_feed_exists(price_identifier)` | Check if feed is configured |
| `get_stale_threshold()` | Get staleness threshold in seconds |

### Pyth Price Format

```rust
struct Price {
    price: i64,        // Price value
    conf: u64,         // Confidence interval (always 0 for Oracle Example)
    expo: i32,         // Exponent (usually -8)
    publish_time: i64, // Unix timestamp (seconds)
}
// actual_price = price * 10^expo
// Example: { price: 500000000, expo: -8 } = $5.00
```

### Pre-configured Price Feed Mappings

| Asset | Contract ID | Pyth Price ID |
|-------|-------------|---------------|
| NEAR | `wrap.near` | `c415de8d2efa7db216527dff4b60e8f3a5311c740dadb233e13e12547e226750` |
| ETH | `aurora` | `ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace` |
| BTC | `nbtc.bridge.near` | `e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43` |
| USDT | `usdt.tether-token.near` | `2b89b9dc8fdf9f34709a5b106b472f0f39bb6ca9ce04b0fd7f2e971688e2e53b` |
| USDC | `17208628f84f5d6ad...` | `eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a` |

### Migration from Pyth

```rust
// Before (Pyth)
const ORACLE: &str = "pyth-oracle.near";

// After (Oracle Example) — no other changes needed!
const ORACLE: &str = "price-oracle-pyth.near";
```

### Example

```bash
# Get NEAR price using Pyth API
near view price-oracle-pyth.near get_price '{
  "price_identifier": "c415de8d2efa7db216527dff4b60e8f3a5311c740dadb233e13e12547e226750"
}'
```

---

## Integration Guide #3: Building Your Own WASI Worker

Create custom data feeds using OutLayer's WASI infrastructure.

### Prerequisites

1. Read [WASI_TUTORIAL.md](https://github.com/out-layer/outlayer/blob/main/wasi-examples/WASI_TUTORIAL.md)
2. Familiarity with Rust and WASI

### Project Structure

```
your-oracle/
├── src/
│   └── main.rs          # WASI entry point
├── Cargo.toml
├── build.sh
└── tokens.json          # Token configuration
```

### Custom Data Sources

Fetch from any HTTP API using `CustomSourceConfig`:

```rust
pub struct CustomSourceConfig {
    pub url: String,           // HTTP endpoint
    pub json_path: String,     // Dot notation: "data.price"
    pub value_type: String,    // "number", "string", "boolean"
    pub method: String,        // "GET" or "POST"
    pub headers: Vec<(String, String)>,
}
```

Example — fetch game price from Steam:

```json
{
  "custom_data_request": [{
    "id": "elden_ring_price",
    "token_id": "",
    "source": {
      "custom": {
        "url": "https://store.steampowered.com/api/appdetails?appids=1245620",
        "json_path": "1245620.data.price_overview.final",
        "value_type": "number"
      }
    }
  }]
}
```

### Deployment

```bash
# Build WASI binary
./build.sh

# Deploy via OutLayer dashboard or CLI
# See: https://app.outlayer.ai
```

---

## Supported Tokens

13 tokens pre-configured with multiple sources:

| Token | Contract ID | Sources |
|-------|-------------|---------|
| **NEAR** | `wrap.near` | CoinGecko, Binance, Binance US, Pyth, Huobi, KuCoin, Gate.io, Crypto.com |
| **ETH** | `aurora` | CoinGecko, Binance, Binance US, Pyth, Huobi, KuCoin, Gate.io, Crypto.com |
| **BTC** | `nbtc.bridge.near` | CoinGecko, Binance, Binance US, Pyth, Huobi, KuCoin, Gate.io, Crypto.com |
| **USDT** | `usdt.tether-token.near` | CoinGecko, Pyth |
| **USDC** | `17208628f84f5d6ad...` | CoinGecko, Binance, Pyth, Crypto.com, KuCoin |
| **WBTC** | `2260fac5e5...factory.bridge.near` | CoinGecko, Binance, Pyth, Huobi, Crypto.com, KuCoin, Gate.io |
| **DAI** | `6b175474e8...factory.bridge.near` | CoinGecko, Binance, Binance US, Pyth, Huobi, Gate.io |
| **AURORA** | `aaaaaa20d9...factory.bridge.near` | CoinGecko, Pyth, Crypto.com, Huobi, KuCoin, Gate.io |
| **WOO** | `4691937a75...factory.bridge.near` | CoinGecko, Binance, Pyth, Huobi, Crypto.com, KuCoin, Gate.io |
| **FRAX** | `853d955ace...factory.bridge.near` | CoinGecko, Pyth |
| **SOL** | `22.contract.portalbridge.near` | CoinGecko, Binance, Binance US, Pyth, Huobi, KuCoin, Gate.io, Crypto.com |
| **ZEC** | `zec.omft.near` | CoinGecko, Binance, Binance US, Pyth, Huobi, KuCoin, Gate.io |
| **RHEA** | `token.rhealab.near` | Binance Alpha, Pyth |

---

## Price Sources

10 exchanges + custom sources:

| Source | Type | API Key | Example Tokens |
|--------|------|---------|----------------|
| **CoinGecko** | REST | Optional | `bitcoin`, `ethereum`, `near` |
| **Binance** | REST | No | `BTCUSDT`, `ETHUSDT` |
| **Binance US** | REST | No | `BTCUSD`, `ETHUSD` |
| **Pyth Network** | REST | No | Price feed IDs |
| **Chainlink** | Ethereum RPC | No | Aggregator contract address |
| **Huobi** | REST | No | `btcusdt`, `ethusdt` |
| **KuCoin** | REST | No | `BTC-USDT`, `ETH-USDT` |
| **Gate.io** | REST | No | `btc_usdt`, `eth_usdt` |
| **Crypto.com** | REST | No | `BTC_USDT`, `ETH_USDT` |
| **Binance Alpha** | REST | No | BSC contract addresses |
| **Custom** | Any HTTP | Configurable | Any URL + JSON path |

**Aggregation:** Median (default) — resistant to outliers and manipulation.

---

## Deposit Summary

| Method | Fresh Cache | Stale (OutLayer) | Subsidized |
|--------|-------------|------------------|------------|
| `get_price_data` | Free | N/A | N/A |
| `get_oracle_price_data` | Free | N/A | N/A |
| `request_price_data` | Free | 0.01+ NEAR | Free |
| `oracle_call` | None (refunded) | 0.01+ NEAR | Free |
| `request_custom_data` | N/A | 0.01+ NEAR | Free |
| `custom_call` | N/A | 0.01+ NEAR | Free |

**Subsidized mode:** When enabled and contract balance > 20 NEAR, users don't pay.

Check status:
```bash
near view price-oracle.near can_subsidize_outlayer_calls
```

---

## Project Structure

```
oracle-example/
├── src/                        # WASI worker source
├── contract/                   # Main price oracle contract
├── wrapper-contract/           # Simple wrapper example
├── pyth-compatible-wrapper/    # Pyth API drop-in replacement
├── scheduler/                  # Off-chain price update automation
├── sources/                    # Shared price sources library
├── oracle-prices-ui/           # Dashboard, playground, docs
├── tokens.json                 # Token configuration
└── README.md                   # This file
```

---

## Links

- **Dashboard & Playground & Docs:** https://price-oracle.outlayer.ai/
- **GitHub:** https://github.com/out-layer/oracle-example
- **OutLayer Platform:** https://app.outlayer.ai
- **Integration Guide:** [integration.md](integration.md)
- **SDK Reference:** [sdk.md](sdk.md)
- **WASI Tutorial:** [WASI_TUTORIAL.md](https://github.com/out-layer/outlayer/blob/main/wasi-examples/WASI_TUTORIAL.md)

---

## License

MIT OR Apache-2.0, at your option — see `LICENSE-MIT` and `LICENSE-APACHE`.
