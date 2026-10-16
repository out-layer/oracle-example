# Oracle Example Troubleshooting Guide

Operational runbook for diagnosing and fixing common issues.

## Quick Diagnostics

### Check System Health

```bash
# 1. Is scheduler running?
docker ps | grep oracle-scheduler

# 2. Are stored prices fresh?
curl -s "https://api.outlayer.ai/public/storage/batch" \
  -H "Content-Type: application/json" \
  -d '{"project_uuid": "YOUR_PROJECT_UUID", "keys": ["price:wrap.near"]}' | jq .

# 3. Are contract prices fresh?
near view price-oracle.testnet get_price_data '{"asset_ids": ["wrap.near"]}' --networkId testnet

# 4. Can contract subsidize calls?
near view price-oracle.testnet can_subsidize_outlayer_calls --networkId testnet
```

---

## Scheduler Issues

### Scheduler container not starting

**Symptoms:** `docker ps` shows no running container, or container restarts in a loop.

**Diagnosis:**
```bash
docker logs oracle-scheduler 2>&1 | head -50
```

**Common causes:**

| Error | Cause | Fix |
|-------|-------|-----|
| `PROJECT_OWNER not set` | Missing env variable | Check `.env` file has all required vars |
| `PROJECT_UUID not set` | Missing env variable | Get UUID from OutLayer dashboard |
| `PAYMENT_KEY not set` | Missing env variable | Create payment key in OutLayer dashboard |
| `Failed to load tokens from ...` | Bad tokens.json path | Check `TOKENS_CONFIG` path in `.env` |
| `invalid JSON` in tokens.json | Syntax error | Validate JSON: `python3 -m json.tool tokens.json` |

### Scheduler running but not updating prices

**Symptoms:** Container running, logs show no update triggers.

**Diagnosis:**
```bash
docker logs --tail 200 oracle-scheduler | grep -E "trigger|update|error"
```

**Common causes:**

1. **Prices haven't changed enough** — Default threshold is 1%. Check logs for `diff=X.XX%`.
   - Fix: Lower `PRICE_DIFF_THRESHOLD_PERCENT` (e.g., `0.5`)

2. **Time interval not elapsed** — Default is 60s, scheduler polls every 10s.
   - Fix: Lower `UPDATE_INTERVAL_SECS` for more frequent updates

3. **External APIs unreachable** — Logs show `Failed to fetch X from sources`.
   - Fix: Check network connectivity from container
   - Fix: Some APIs need API keys (CoinGecko Pro, etc.)

### WASI update triggered but fails

**Symptoms:** Logs show `WASI update failed: ...`

**Common causes:**

| Error | Fix |
|-------|-----|
| `HTTP 402` | Payment key has no funds. Refill in OutLayer dashboard |
| `HTTP 401` | Payment key is invalid or expired. Create new key |
| `HTTP 404` | Project not found. Check `PROJECT_OWNER` and `PROJECT_NAME` |
| `HTTP 429` | Rate limited. Increase `UPDATE_INTERVAL_SECS` |
| `WASI execution failed` | Check OutLayer dashboard for execution errors |

### Telegram alerts not working

**Diagnosis:**
```bash
# Test Telegram manually
curl -s "https://api.telegram.org/botYOUR_BOT_TOKEN/sendMessage" \
  -d "chat_id=YOUR_CHAT_ID&text=test"
```

**Common causes:**
- `TELEGRAM_BOT_TOKEN` or `TELEGRAM_CHAT_ID` not set
- Bot not added to the chat/group
- Bot token revoked — create new bot via @BotFather

---

## Contract Issues

### `oracle_call` returns stale prices (price: null)

**Symptoms:** `get_price_data` returns `null` for some assets.

**Diagnosis:**
```bash
# Check which assets have prices
near view price-oracle.testnet get_price_data --networkId testnet

# Check recency duration
near view price-oracle.testnet get_price_data '{"asset_ids": ["wrap.near"]}' --networkId testnet
```

**Common causes:**

1. **No oracle has reported** — No scheduler running, or WASI updates failing.
   - Fix: Check scheduler logs
   - Fix: Manually trigger an update via OutLayer dashboard

2. **Prices expired** — `recency_duration_sec` is too short for update frequency.
   - Fix: Increase `recency_duration_sec` via a council proposal (auto-executes when the approval threshold is met; if threshold == 1 it self-executes):
     ```bash
     near call price-oracle.testnet create_proposal '{
       "action": {"action": "set_recency_duration_sec", "recency_duration_sec": 300}
     }' --accountId COUNCIL_MEMBER --deposit 0.1 --networkId testnet
     ```

3. **Asset not registered** — Asset ID not added to contract.
   - Fix: Add the asset via a council proposal (`push_signer_key` is the name of a TEE `PROTECTED_` secret used to sign price pushes, or `null` for warm-only):
     ```bash
     near call price-oracle.testnet create_proposal '{
       "action": {"action": "add_asset", "asset_id": "wrap.near", "push_signer_key": null}
     }' --accountId COUNCIL_MEMBER --deposit 0.1 --networkId testnet
     ```

4. **Oracle not registered** — Contract's own account not registered as oracle.
   - Fix: Add the oracle via a council proposal:
     ```bash
     near call price-oracle.testnet create_proposal '{
       "action": {"action": "add_oracle", "account_id": "price-oracle.testnet"}
     }' --accountId COUNCIL_MEMBER --deposit 0.1 --networkId testnet
     ```

### `oracle_call` panics with "OutLayer not configured"

**Fix:** Configure OutLayer integration via a council proposal:
```bash
near call price-oracle.testnet create_proposal '{
  "action": {
    "action": "configure_outlayer",
    "outlayer_contract_id": "outlayer.testnet",
    "code_source": "{\"Project\": {\"project_id\": \"OWNER/PROJECT\"}}",
    "secrets_profile": "default",
    "secrets_account_id": "OWNER"
  }
}' --accountId COUNCIL_MEMBER --deposit 0.1 --networkId testnet
```

### `oracle_call` panics with "Requires at least 0.01 NEAR"

**Cause:** Subsidy not enabled or contract balance too low.

**Fix (option A):** Caller attaches deposit:
```bash
near call price-oracle.testnet oracle_call '{...}' --deposit 0.02
```

**Fix (option B):** Enable subsidy and fund the contract:
```bash
# Enable subsidy via a council proposal
near call price-oracle.testnet create_proposal '{
  "action": {"action": "set_subsidize_outlayer_calls", "enabled": true}
}' --accountId COUNCIL_MEMBER --deposit 0.1 --networkId testnet

# Fund contract (needs > 20 NEAR for subsidy to activate)
near send OWNER price-oracle.testnet 25
```

### Contract upgrade fails

**Symptoms:** Upgrade proposal fails, or contract state is broken after upgrade.

There is no direct `upgrade` method. Upgrades run through the council in two steps: first upload the raw WASM via `upload_upgrade_code`, then create an `upgrade_contract` proposal referencing the code hash. On approval the contract deploys the stored code, optionally runs a migration method, and self-checks via `get_version`.

**Upgrade flow:**
```bash
# 1. Upload the raw WASM as function-call input. Attach NEAR for storage (~1 NEAR per 100KB).
#    Prints the SHA-256 code_hash you need for the proposal.
near call price-oracle.testnet upload_upgrade_code \
  --base64 "$(base64 -i contract/res/price_oracle.wasm)" \
  --accountId COUNCIL_MEMBER --deposit 5 --gas 300000000000000 --networkId testnet

# 2. Create the upgrade proposal with the code_hash from step 1.
#    migrate_method is optional: use migrate_state / migrate_state2 / migrate_state3
#    to run a state migration after deploy, or null to skip.
near call price-oracle.testnet create_proposal '{
  "action": {
    "action": "upgrade_contract",
    "code_hash": "CODE_HASH_FROM_STEP_1",
    "migrate_method": "migrate_state3"
  }
}' --accountId COUNCIL_MEMBER --deposit 0.1 --gas 300000000000000 --networkId testnet
```

The proposal auto-executes once the approval threshold is met (self-executes if threshold == 1).

**Prevention:**
- Always keep previous WASM versions in `contract/res/`
- Test upgrades on testnet first
- Use versioned state (VAsset, VOracle enums) for backward compatibility
- Pick the correct `migrate_method` for the state version you are migrating from

**Recovery:**
```bash
# Re-deploy a previous version through the same two-step flow.
near call price-oracle.testnet upload_upgrade_code \
  --base64 "$(base64 -i contract/res/price_oracle_0.5.0.wasm)" \
  --accountId COUNCIL_MEMBER --deposit 5 --gas 300000000000000 --networkId testnet

near call price-oracle.testnet create_proposal '{
  "action": {"action": "upgrade_contract", "code_hash": "CODE_HASH_FROM_UPLOAD", "migrate_method": null}
}' --accountId COUNCIL_MEMBER --deposit 0.1 --gas 300000000000000 --networkId testnet
```

To discard an uploaded blob without deploying, call `remove_pending_upgrade_code '{"code_hash": "..."}'` (refunds the storage deposit to the uploader).

---

## WASI Execution Issues

### WASI binary not found or out of date

**Symptoms:** Execution fails before the WASM runs, or runs an older build.

**Fix:** The oracle runs the active version of the project its `code_source` names. Publish the build
(`outlayer upload target/wasm32-wasip2/release/oracle-example.wasm`, then
`outlayer deploy price-oracle <FastFS URL>`), and if `code_source` points elsewhere, set it via a council proposal:
```bash
near call price-oracle.testnet create_proposal '{
  "action": {
    "action": "configure_outlayer",
    "outlayer_contract_id": "outlayer.testnet",
    "code_source": "{\"Project\": {\"project_id\": \"OWNER/price-oracle\"}}",
    "secrets_profile": null,
    "secrets_account_id": null
  }
}' --accountId COUNCIL_MEMBER --deposit 0.1 --networkId testnet
```

Or pin one published build with
`{"WasmUrl": {"url": "https://main.fastfs.io/<you>/outlayer.near/<sha256>.wasm", "hash": "<sha256>", "build_target": "wasm32-wasip2"}}`.

### WASI returns empty prices

**Symptoms:** WASI executes successfully but returns `prices: []`.

**Common causes:**

1. **Token not in whitelist** — `tokens.json` doesn't include the requested token.
   - Fix: Add token to `tokens.json` and rebuild WASI

2. **All sources failed** — Network issues or API changes.
   - Check WASI execution logs in OutLayer dashboard

3. **`min_sources_num` too high** — Requires more sources than available.
   - Fix: Lower `min_sources_num` in scheduler config

### WASI execution timeout

**Default limits:** 10B instructions, 128MB memory, 60s timeout.

**Fix:** Increase limits in contract call:
```rust
resource_limits: Some(ResourceLimits {
    max_instructions: Some(20_000_000_000),
    max_memory_mb: Some(256),
    max_execution_seconds: Some(120),
})
```

---

## Wrapper Contract Issues

### `get_price` fails with "not enough balance"

**Cause:** Wrapper contract doesn't have enough NEAR to pay oracle (0.02 NEAR per call).

**Fix:** Fund the wrapper contract:
```bash
near send YOUR_ACCOUNT price-oracle-wrapper.testnet 5
```

### `resolve` fails with "No prediction found"

**Cause:** User hasn't called `predict()` first, or prediction was already resolved.

**Fix:** Call `predict()` before `resolve()`:
```bash
near call price-oracle-wrapper.testnet predict \
  '{"token_id": "wrap.near", "predicted_price": 4.50}' \
  --accountId USER --networkId testnet
```

---

## Price Dashboard (oracle-prices-ui) Issues

### Dashboard shows no prices

**Diagnosis:**
1. Check `.env` has correct `API_URL` and `PROJECT_UUID`
2. Check CORS proxy is running: `curl http://localhost:8000/health`
3. Check browser console for errors

**Common causes:**
- Wrong `PROJECT_UUID` — get correct one from OutLayer dashboard
- CORS proxy not running — run `python3 server.py`
- API URL wrong — should be `https://api.outlayer.ai` (not `https://testnet-api...` for mainnet)

### Dashboard shows stale prices

**Cause:** Scheduler not running or WASI updates failing.

**Fix:** Check scheduler first (see Scheduler Issues above).

---

## Network & API Issues

### CoinGecko rate limiting

**Symptoms:** `429 Too Many Requests` in logs.

**Fix:**
- Use CoinGecko Pro API key via secrets:
  ```
  Set API_KEY secret in OutLayer keystore
  ```
- Reduce update frequency

### Binance API blocked in some regions

**Fix:** Use alternative exchanges or configure proxy. The oracle fetches from 10 sources (coingecko, binance, binance_us, binance_alpha, pyth, chainlink, huobi, kucoin, gate, cryptocom), so individual source failures are tolerable as long as `min_sources_num` is met.

### Pyth price feed ID not found

**Symptoms:** Pyth returns empty or error.

**Fix:** Verify the Pyth price feed ID at https://pyth.network/price-feeds. IDs are hex strings starting with `0x`.

**Note:** Pyth is the only source with a source-side staleness check — prices whose `publish_time` is older than 120s are rejected.

---

## Performance Tuning

### Reduce update costs

- Increase `UPDATE_INTERVAL_SECS` (e.g., 120 for low-demand tokens)
- Increase `PRICE_DIFF_THRESHOLD_PERCENT` (e.g., 2.0)

### Reduce latency for DeFi

- Enable scheduler with low `UPDATE_INTERVAL_SECS` (30-60s)
- Enable subsidized mode so users don't need to attach deposit
- Set `recency_duration_sec` to 30 for Ref Finance compatibility

### Reduce gas costs

- Use `get_price_data` (view, free) when possible instead of `oracle_call`
- Use `request_price_data` for direct returns without callback overhead
- Batch multiple assets in a single call: `asset_ids: ["wrap.near", "aurora", "usdt.tether-token.near"]`
