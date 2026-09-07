# Oracle Example Scheduler

Background service that keeps TEE worker prices fresh by monitoring external price sources and triggering WASI updates when needed.

## Architecture

The scheduler solves a key problem: how to keep TEE worker prices up-to-date so that any incoming request gets an **instant response with fresh data**, without spending gas on every update.

### Components

```
External APIs                    Scheduler (VPS)                TEE Worker (Phala Cloud)
─────────────                    ───────────────                ────────────────────────
CoinGecko ─┐                     ┌─────────────┐               ┌──────────────────────┐
Binance   ─┤  compare prices     │ Poll loop   │  read stored  │ Public Storage       │
Pyth      ─┼──────────────────>  │ (every 5s)  │ <──────────── │  price:wrap.near     │
Kraken    ─┤  (1 req per venue,  │             │   prices      │  price:eth.bridge... │
Coinbase  ─┤   whole asset set)  │  if delta > │               │  price:nbtc...       │
Chainlink ─┤                     │  threshold: │               │   each with per-source
MEXC ...  ─┘                     │             │  trigger      │   observation times  │
                                 │  call WASI  │ ────────────> │                      │
                                 │  (no data!, │  update       │ WASI Binary          │
                                 │   tier tag) │  (tier)       │  fetches the tier's  │
                                 └─────────────┘               │  venues, MERGES into │
                                                               │  the record, writes  │
                                                               └──────────────────────┘
```

### How it works

1. **TEE worker** (WASI binary inside Intel TDX enclave) holds fresh prices in its public storage. When any user or contract requests a price, the worker returns the cached result immediately — no external API calls at request time.

2. **Scheduler** runs **outside TEE** on a separate VPS. Every 5s (configurable via `POLL_INTERVAL_SECS`) it:
   - Fetches current prices from external sources (CoinGecko, Binance, Pyth, etc.) for comparison — **batched, one request per venue for the whole asset set**, exactly like the worker
   - Reads the TEE worker's stored prices via OutLayer public storage batch API
   - Compares the two and decides whether an update is needed

   Batching this side is not only about cost. The comparison exists to notice when the
   scheduler's view disagrees with the worker's, so any difference in *how* the two fetch
   becomes a permanent disagreement: fetching per token cost 182 requests per poll for 18
   assets, which kept Kraken and Bitstamp rate-limited, dropped them from this side's median
   only, and made the deviation trigger fire on every cycle forever.

   The venues are fetched **concurrently**, and each request carries an 8s deadline. The WASI
   worker has no async runtime and must go one venue at a time; doing the same here makes the
   poll cycle the *sum* of every venue's latency and lets one hanging endpoint stall everything
   behind it — measured in production as a 57-78s cadence where ~15s was expected. Concurrently
   the cycle costs the slowest venue, and the deadline bounds even that.

3. **Triggering an update**: the scheduler sends a `call` request to the OutLayer coordinator with command `update_prices` and the list of tokens to refresh. **The scheduler does NOT send price data** — it only tells the TEE worker *which* tokens need updating. The worker then fetches prices from all configured sources independently inside the enclave, aggregates them, and writes the result to public storage.

   The worker fetches in **batches: one HTTP request per source covering every requested token**, not one per (token, source). A refresh should therefore be a **single call for the whole asset set** — that is one request per source per cycle.

   `GROUP_MAX_TOKENS` (default 64) exists only as an escape hatch and should stay **above the number of tracked assets**. Splitting the set into groups does not parallelise work here, it *multiplies* requests: N groups means N full pulls of every all-tickers endpoint (Coinbase alone is ~107 KB) and pushes rate-limited sources such as CoinGecko over their limit — which surfaces as assets silently missing a source rather than as an error. `FETCH_CONCURRENCY` then bounds how many groups may be in flight if you ever do lower it. When more than one group runs, each writes its own tokens as it finishes, so a failing group never discards the others' work — the failure is alerted and the healthy groups still commit.

4. **Pushing on-chain** (optional) runs as a **separate, much slower phase** — see below.

### Why this design?

- **Trust model preserved** — the scheduler never provides data that needs to be trusted; all price fetching and aggregation happens inside TEE
- **Gas-free** — prices stay fresh in TEE public storage without on-chain transactions
- **Instant responses** — any WASI call requesting prices gets pre-computed results
- **Efficient** — only tokens with significant changes or stale data get updated

## Source tiers

A refresh does not have to cover every venue. The worker **merges** what a refresh fetched into
the stored record, keeping each source's own observation time, so venues can run at different
cadences and still aggregate into one price:

| Tier | Venues | Assets | Cadence |
|------|--------|--------|---------|
| **Fast** | everything except `SLOW_SOURCES` | `PRIORITY_ASSETS` | `UPDATE_INTERVAL_PRIORITY_SECS` (~13-15s at `0`) |
| **Full** | everything except `SLOW_SOURCES` | all | `UPDATE_INTERVAL_SECS` (60s) |
| **Slow** | `SLOW_SOURCES` (`pyth,chainlink`) | all that configure them | `SLOW_SOURCE_INTERVAL_SECS` (90s) |
| **Push** | everything | all | `CONTRACT_PUSH_INTERVAL_SECS` (300s), only when on-chain push is enabled |

Why this split: the cheap venues answer for the entire asset set in one request, while Chainlink
is an EVM `eth_call` through Multicall3 and Pyth a separate API. Paying for those on every cycle
is what made a sub-20s cadence unaffordable. They are not dropped — their last observation stays
in the record and keeps contributing to any consumer whose window is wide enough to include it.

Consumers choose that window per request via `max_age_secs`, which filters **sources**, not
records: `max_age_secs: 40` is answered from the venues seen within 40 seconds, and the returned
`publish_time` is the oldest of them. If too few sources qualify, the worker fetches fresh rather
than serving a thinner set.

## Update triggers

| Trigger | Condition | Default |
|---------|-----------|---------|
| **Price deviation** | `abs(current - stored) / stored > threshold` | 1% |
| **Time interval** | Time since last update exceeds the tier's interval | 60s full / ~13-15s priority |
| **Missing price** | Token has no stored price yet | Always triggers |

Either trigger is sufficient. If a price moves fast, it gets updated before the interval expires.

The loop always waits `POLL_INTERVAL_SECS` between cycles, including after a successful refresh.
It used to skip that wait whenever WASI had been called, which is unbounded by construction: the
deviation trigger does not reset the scheduled timers, so a token whose two medians disagree
persistently would refresh, skip the wait, still disagree, and refresh again — every call costing
money and none of them making prices fresher.

## Optional: on-chain contract push

Set `UPDATE_CONTRACT_ENABLED=true` to have the TEE worker also call `report_prices` on the oracle smart contract after updating public storage. This writes prices on-chain, making them available via contract view methods without an OutLayer call.

Each on-chain update costs gas, so this mode is **disabled by default**. Enable it only when on-chain price availability is required.

Note this scheduler is a convenience, not a dependency: the on-chain push is permissionless, so
anyone can call the worker with `update_prices` + `update_contract: true` and write fresh prices to
the contract even while this scheduler is stopped. They cannot influence the price — the worker
aggregates sources inside the enclave and signs with a TEE-held key that the contract's
`push_signer_accounts` allowlist checks. Repeated triggers are bounded by a 20-second per-asset
skip, and gas always comes from the push signer's funded account.

The push is deliberately **decoupled from the cache refresh**: it runs on its own interval (`CONTRACT_PUSH_INTERVAL_SECS`, default 300s) as a **single call covering all tokens**, never once per refresh group. That means raising the refresh rate — or `FETCH_CONCURRENCY` — makes prices fresher without multiplying transactions or gas. The balance fail-safe still applies: if a signer's balance cannot be confirmed above `ORACLE_MIN_BALANCE_NEAR`, pushes pause and the scheduler keeps running warm-only. A failed push is alerted and retried on the next push cycle; it never fails the refresh cycle.

Setting `UPDATE_CONTRACT_ENABLED=true` also **requires** `ORACLE_CONTRACT_ID` plus `SECRETS_PROFILE` and `SECRETS_ACCOUNT_ID`. Without these, the WASI binary runs without the `PROTECTED_` signing keys and no on-chain push happens (public storage is still updated).

### How the secrets are bound

The scheduler calls `POST /call/{PROJECT_OWNER}/{PROJECT_NAME}`, so OutLayer resolves the code
through the **project**, not through a GitHub URL. Secrets follow the same path: the worker builds a
`Project { project_id }` accessor from the project the call was made against and asks the keystore
for that profile.

The production deployment therefore runs as:

```env
PROJECT_OWNER=price-oracle.near
PROJECT_NAME=price-oracle
SECRETS_PROFILE=oracle
SECRETS_ACCOUNT_ID=price-oracle.near
```

Two consequences worth knowing:

- **Renaming, forking or moving the GitHub repository does not invalidate these secrets.** Nothing in
  the lookup is a repo string. Only `request_execution` calls that declare a `GitHub` code source
  bind secrets to `github.com/owner/repo`, and those break when the repo is renamed.
- The profile name is not `default` here. `SECRETS_PROFILE` selects a named profile under
  `SECRETS_ACCOUNT_ID`, and *all* secrets in that profile are injected into the WASI environment, so
  a wrong profile name fails as "missing `PROTECTED_` key" rather than as an access error.

`SECRETS_PROFILE`/`SECRETS_ACCOUNT_ID` name a profile and an account. Neither is itself a secret —
the secret values live encrypted on-chain and are only ever decrypted inside the TEE.

## Setup

### 1. Copy environment file

```bash
cp .env.example .env
```

### 2. Configure required variables

```env
# OutLayer coordinator
COORDINATOR_URL=https://api.outlayer.ai

# Project identification
PROJECT_OWNER=alice.near
PROJECT_NAME=price-oracle
PROJECT_UUID=p0000000000000001

# Payment key for WASI calls (create via OutLayer dashboard)
PAYMENT_KEY=alice.near:0:your_secret_key_here
```

### 3. Exchange configs (no local file)

The scheduler reads exchange configs from OutLayer public storage under the key `config:assets` (fetched via the public-storage batch API and cached for ~10 minutes). There is no local `tokens.json` to maintain.

This requires `sync_asset_configs` to have been called on the oracle contract first, so that `config:assets` is populated. On startup the scheduler logs:

```
INFO  Exchange configs: loaded from public storage (config:assets)
```

### 4. Run

**Docker (recommended):**

```bash
docker compose up -d
```

The Docker service is configured with `restart: unless-stopped` for automatic recovery.

**Direct:**

```bash
cargo run --release
```

## Configuration reference

| Variable | Default | Description |
|----------|---------|-------------|
| `COORDINATOR_URL` | `https://api.outlayer.ai` | OutLayer API URL |
| `PROJECT_OWNER` | required | NEAR account owning the project |
| `PROJECT_NAME` | required | OutLayer project name |
| `PROJECT_UUID` | required | Project UUID for public storage reads |
| `PAYMENT_KEY` | required | Payment key (format: `owner:nonce:secret`) |
| `UPDATE_INTERVAL_SECS` | `60` | Max staleness before refresh, full asset set (seconds) |
| `UPDATE_INTERVAL_PRIORITY_SECS` | `0` | Extra wait between priority refreshes. `0` = due on every poll, giving a ~13-15s effective cadence (poll interval + own fetch + the call). A positive value adds on top: `10` yields ~25s |
| `SLOW_SOURCES` | `pyth,chainlink` | Venues excluded from the fast and full tiers and refreshed on their own slow cadence |
| `SLOW_SOURCE_INTERVAL_SECS` | `90` | How often `SLOW_SOURCES` are refreshed. Must stay below the worker's 120s canonical window — the real gap is this plus the poll interval plus the call |
| `POLL_INTERVAL_SECS` | `5` | How often the poll loop runs (seconds) |
| `PRIORITY_ASSETS` | `wrap.near,nbtc.bridge.near,eth.bridge.near` | Comma-separated assets refreshed on the priority interval |
| `PRICE_DIFF_THRESHOLD_PERCENT` | `1.0` | Price change % that triggers immediate refresh |
| `GROUP_MAX_TOKENS` | `64` | Max tokens per refresh call — keep above the asset count so one batch request per source is made |
| `FETCH_CONCURRENCY` | `3` | How many refresh groups run concurrently |
| `NEAR_RPC_URL` | `https://rpc.mainnet.fastnear.com` | NEAR RPC endpoint |
| `UPDATE_CONTRACT_ENABLED` | `false` | Also push prices to on-chain contract (costs gas) |
| `CONTRACT_PUSH_INTERVAL_SECS` | `300` | How often the on-chain push runs (seconds) |
| `ORACLE_CONTRACT_ID` | — | Contract to update (required if above is true) |
| `ORACLE_SIGNER_ACCOUNT` | — | Account used to sign on-chain oracle updates |
| `ORACLE_MIN_BALANCE_NEAR` | `0.05` | Minimum signer balance before alerting (NEAR) |
| `SECRETS_PROFILE` | — | Secrets profile injected into WASI (required for on-chain push) |
| `SECRETS_ACCOUNT_ID` | — | Account owning the secrets profile (required for on-chain push) |
| `AGGREGATION_METHOD` | `median` | Aggregation: `median` / `average` / `weighted_average` (an alias of `average` — equal weights, no extra outlier resistance) |
| `MIN_SOURCES_NUM` | `1` | Minimum sources required for valid price |
| `API_KEY` | — | API key for premium price sources (CoinGecko Pro, etc.) |
| `PYTH_API_KEY` | — | Pyth (Hermes) API key, only used if `pyth` is fetched here, i.e. removed from `SLOW_SOURCES`. The worker reads the same-named secret from its secrets profile; without it Pyth is skipped, not errored |
| `TELEGRAM_BOT_TOKEN` | — | Telegram bot token for failure alerts |
| `TELEGRAM_CHAT_ID` | — | Chat ID to send alerts to |
| `RUST_LOG` | `info` | Log level: `trace` / `debug` / `info` / `warn` / `error` |

## Monitoring

### Built-in Telegram alerts

Configure `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` to receive alerts:

- **3+ consecutive poll failures** — error details and project info
- **No prices available** — all external sources failed
- **WASI update failure** — token list and error message

### Logs

The scheduler logs all decisions at `info` level:

```
INFO  Starting oracle scheduler
INFO  Exchange configs: loaded from public storage (config:assets)
INFO  Update intervals: priority=10s, full=60s, poll=5s, threshold=1%
INFO  wrap.near: triggering update (reason: price change, current=5.1234)
INFO  Triggering WASI update for 3 tokens
INFO  Triggering WASI update successful
```

Set `RUST_LOG=debug` for per-token price comparisons:

```
DEBUG wrap.near: current=5.1234, stored=5.0100, diff=2.26%
DEBUG aurora: current=3456.78, stored=3450.12, diff=0.19%
```

## API interactions

### Reading TEE public storage

```
POST {COORDINATOR_URL}/public/storage/batch
{
  "project_uuid": "...",
  "keys": ["price:wrap.near", "price:eth.bridge.near", ...]
}
```

Returns an envelope keyed by storage key; each `value` is base64-encoded JSON:
```json
{
  "results": {
    "price:wrap.near": { "exists": true, "value": "<base64>" }
  }
}
```

The decoded `value` is a `StoredPrice` (timestamps are unix **seconds**):
```json
{
  "price": 5.0123,
  "timestamp": 1706900000,
  "sources": [
    {"name": "mexc",      "price": 5.0140, "timestamp": 1706900000},
    {"name": "chainlink", "price": 5.0090, "timestamp": 1706899890}
  ],
  "aggregation_method": "median"
}
```

`timestamp` is when the record was last **written**, not the age of the price — a partial
refresh touches the record while leaving most sources alone. Each source carries its own
observation time, and the honest staleness bound of any aggregate is the oldest source inside
the window it was built from.

### Triggering WASI update

```
POST {COORDINATOR_URL}/call/{PROJECT_OWNER}/{PROJECT_NAME}
X-Payment-Key: {PAYMENT_KEY}
{
  "input": {
    "command": "update_prices",
    "tokens": ["wrap.near", "eth.bridge.near"],
    "exclude_sources": ["pyth", "chainlink"],
    "update_contract": false,
    "aggregation_method": "median",
    "min_sources_num": 1,
    "contract_id": "oracle.near"
  },
  "resource_limits": {
    "max_execution_seconds": 60
  },
  "async": true
}
```

`exclude_sources` / `only_sources` select the tier. The fast and full tiers send
`exclude_sources: SLOW_SOURCES`; the slow tier sends `only_sources: SLOW_SOURCES`; the on-chain
push sends neither, so it refreshes every venue. Whatever a call fetches is merged into the
stored record — a tier never deletes the venues it skipped. An unknown name is rejected rather
than ignored: a typo would otherwise make the slow tier refresh everything on the fast tier's
cadence.

When on-chain push is enabled, the request also carries `oracle_keys` and a top-level `secrets_ref` so the worker can sign the contract update inside TEE.

Because the call is submitted with `"async": true`, the coordinator returns a `call_id`; the scheduler then polls `GET {COORDINATOR_URL}/calls/{call_id}` until the update completes.

Note: the `input` contains only the token list and configuration — **no price data**. The TEE worker fetches prices independently.
