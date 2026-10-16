# Oracle Example Deployment Guide

## Architecture

```
┌─────────────────────┐     ┌─────────────────────┐     ┌─────────────────────┐
│     Scheduler       │────▶│   WASI (OutLayer)   │────▶│   Contract (NEAR)   │
│   (Docker/VPS)      │     │      (TEE)          │     │   price-oracle.near   │
└─────────────────────┘     └─────────────────────┘     └─────────────────────┘
        │                           │                           │
   Triggers updates           Fetches prices              Stores prices
   on a schedule or 1%       from APIs in TEE            for DeFi apps
```

All contract state mutations go through **DAO proposals** (council vote, >50% threshold).
The only owner-only operations are `set_council` (bootstrap) and the initial deploy.

## Prerequisites

- NEAR CLI: `npm install -g near-cli`
- Cargo Near: `cargo install cargo-near`
- Rust 1.85+ with `wasm32-wasip2` target
- Docker (for scheduler)
- OutLayer account at https://app.outlayer.ai

## Initialization Order

The steps below must be followed **in this exact order** because of dependencies:

| Step | Action | Requires | Method |
|------|--------|----------|--------|
| 1 | Build contract | - | cargo near build |
| 2 | Deploy & migrate | - | owner (last time) |
| 3 | Set council | deployed contract | owner (bootstrap) |
| 4 | Configure OutLayer | council | DAO proposal |
| 5 | Add assets | council | DAO proposal |
| 6 | Register push signer | OutLayer + assets | DAO (calls OutLayer) |
| 7 | Fund implicit account | push signer registered | NEAR transfer |
| 8 | Enable subsidized calls | council | DAO proposal (optional) |
| 9 | Deploy scheduler | everything above | Docker |

---

## Step 1: Build Contract

```bash
cd oracle-example/contract
cargo near build
```

Output: `target/near/price_oracle.wasm`

## Step 2: Deploy & Migrate

### Fresh deployment

```bash
# Deploy
near contract deploy price-oracle.near \
  use-file target/near/price_oracle.wasm \
  without-init-call network-config mainnet sign-with-keychain send

# Initialize
near call price-oracle.near new '{
  "recency_duration_sec": 300,
  "owner_id": "owner.near",
  "near_claim_amount": "100000000000000000000000"
}' --accountId price-oracle.near
```

### Upgrade existing deployment

This is the **last time** the owner redeploys the contract directly, using the
`price-oracle.near` account's own full-access key. There is no `upgrade` method —
after the council is bootstrapped (Step 3), all future upgrades go through DAO
(`upload_upgrade_code` + `upgrade_contract` proposal, see "Future Upgrades" below).

```bash
near contract deploy price-oracle.near \
  use-file target/near/price_oracle.wasm \
  with-init-call migrate_state2 json-args '{}' \
  prepaid-gas '300.0 Tgas' attached-deposit '0 NEAR' \
  network-config mainnet sign-with-keychain send
```

The `migrate_state2` migration adds:
- `asset_oracle_keys` — per-asset TEE key mapping
- `pending_upgrade_codes` — DAO upgrade code storage

### Verify

```bash
near view price-oracle.near get_version
```

## Step 3: Set Council

Bootstrap the council. This is the only remaining owner-only mutation.
Threshold is automatic: >50% of members.

```bash
near call price-oracle.near set_council '{
  "members": ["member1.near", "member2.near", "member3.near"]
}' --accountId owner.near --deposit 0.000000000000000000000001
```

Verify:
```bash
near view price-oracle.near get_council_members
near view price-oracle.near get_council_threshold
```

> **From this point forward, all changes go through DAO proposals.**
> With 1 council member, proposals auto-execute.
> With 2+ members, use `approve_proposal` to vote.

## Step 4: Configure OutLayer

Required before any OutLayer calls (price fetching, push signer registration).

```bash
near call price-oracle.near create_proposal '{"action": {
  "action": "configure_outlayer",
  "outlayer_contract_id": "outlayer.near",
  "code_source": "{\"Project\":{\"project_id\":\"owner.near/price-oracle\"}}",
  "secrets_profile": "default",
  "secrets_account_id": "owner.near"
}}' --accountId member1.near --deposit 0.1
```

If council has 2+ members:
```bash
near call price-oracle.near approve_proposal '{"id": 0}' \
  --accountId member2.near --deposit 0.000000000000000000000001
```

## Step 5: Add Assets

Assets must exist before registering a push signer.

```bash
near call price-oracle.near create_proposals '{"actions": [
  {"action": "add_asset", "asset_id": "wrap.near", "push_signer_key": null},
  {"action": "add_asset", "asset_id": "usdt.tether-token.near", "push_signer_key": null},
  {"action": "add_asset", "asset_id": "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1", "push_signer_key": null}
]}' --accountId member1.near --deposit 0.1
```

> `push_signer_key: null` — the key will be set by `RegisterPushSigner` in step 6.

Add EMAs if needed:
```bash
near call price-oracle.near create_proposals '{"actions": [
  {"action": "add_asset_ema", "asset_id": "wrap.near", "period_sec": 3600},
  {"action": "add_asset_ema", "asset_id": "wrap.near", "period_sec": 86400}
]}' --accountId member1.near --deposit 0.1
```

## Step 6: Register TEE Push Signer

This calls OutLayer WASI to resolve the `PROTECTED_` key into an implicit account,
then creates a DAO proposal. On approval:
- Registers the implicit account as oracle
- Sets `push_signer_accounts` for each asset (only this account can push)
- Sets `asset_oracle_keys` (which TEE key signs transactions)

### 6.1 Create the PROTECTED key in OutLayer

In OutLayer dashboard → Secrets:
- Add a generated secret: name `PROTECTED_KEY_RHEA`, type `ed25519`

### 6.2 Call propose_register_push_signer

```bash
near call price-oracle.near propose_register_push_signer '{
  "push_signer_key": "PROTECTED_KEY_RHEA",
  "asset_ids": [
    "wrap.near",
    "usdt.tether-token.near",
    "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1"
  ],
  "secrets_profile": "default",
  "secrets_account_id": "owner.near"
}' --accountId member1.near --deposit 0.000000000000000000000001 \
  --gas 300000000000000
```

This triggers an OutLayer call. The callback creates a `RegisterPushSigner` proposal
with the resolved implicit account ID. Check the proposal:

```bash
near view price-oracle.near get_proposals '{"limit": 5}'
```

If council has 2+ members, approve:
```bash
near call price-oracle.near approve_proposal '{"id": <proposal_id>}' \
  --accountId member2.near --deposit 0.000000000000000000000001
```

## Step 7: Fund the Implicit Account

The implicit account (64-char hex) needs NEAR to send transactions.
Get the account ID from the approved proposal or from the contract:

```bash
near view price-oracle.near get_push_signer_accounts '{"asset_id": "wrap.near"}'
# Returns: ["abcdef1234..."]
```

Fund it:
```bash
near send owner.near <implicit_account_id> 0.1
```

## Step 8: Enable Subsidized Calls (Optional)

When enabled and contract balance > 20 NEAR, the contract pays for OutLayer calls
so users can call `oracle_call` without attaching NEAR.

```bash
near call price-oracle.near create_proposal '{"action": {
  "action": "set_subsidize_outlayer_calls",
  "enabled": true
}}' --accountId member1.near --deposit 0.1
```

Check status:
```bash
near view price-oracle.near can_subsidize_outlayer_calls
```

## Step 9: Deploy Scheduler

### 9.1 Build WASI

```bash
cd oracle-example
./build.sh
```

### 9.2 Create OutLayer project

```bash
outlayer upload target/wasm32-wasip2/release/oracle-example.wasm
# prints SHA256 and: FastFS URL: https://main.fastfs.io/<you>/outlayer.near/<sha256>.wasm
outlayer deploy oracle-example <FastFS URL>
```

Or on https://app.outlayer.ai: **Projects** → **Create Project** with the WASM URL and its SHA256.
Note your project UUID.

### 9.3 Configure scheduler

```bash
cd scheduler
cp .env.example .env
```

Key settings in `.env`:
```bash
COORDINATOR_URL=https://api.outlayer.ai
PROJECT_OWNER=owner.near
PROJECT_NAME=price-oracle
PAYMENT_KEY=owner.near:1:your-secret-key

UPDATE_INTERVAL_SECS=60
PRICE_DIFF_THRESHOLD_PERCENT=1.0

UPDATE_CONTRACT_ENABLED=true
ORACLE_CONTRACT_ID=price-oracle.near

SECRETS_PROFILE=default
SECRETS_ACCOUNT_ID=owner.near
```

### 9.4 Run

```bash
docker build -t oracle-scheduler -f scheduler/Dockerfile .
docker run -d --name oracle-scheduler --restart unless-stopped \
  --env-file scheduler/.env oracle-scheduler
```

---

## Future Upgrades (via DAO)

After the initial deploy, all contract upgrades go through DAO:

```bash
# 1. Upload code (attach NEAR for storage, ~1 NEAR per 100KB)
near call price-oracle.near upload_upgrade_code \
  --base64File target/near/price_oracle.wasm \
  --accountId member1.near --deposit 5 --gas 300000000000000
# Returns: code_hash (SHA-256 hex)

# 2. Create upgrade proposal
#    migrate_method: "migrate_state3" if state changed, null otherwise
near call price-oracle.near create_proposal '{"action": {
  "action": "upgrade_contract",
  "code_hash": "<hash from step 1>",
  "migrate_method": null
}}' --accountId member1.near --deposit 0.1

# 3. Approve (deploys code + refunds storage deposit to uploader)
near call price-oracle.near approve_proposal '{"id": <id>}' \
  --accountId member2.near --deposit 0.000000000000000000000001
```

Manage pending uploads:
```bash
# List pending code hashes
near view price-oracle.near get_pending_upgrade_hashes

# Remove uploaded code (refunds deposit to uploader)
near call price-oracle.near remove_pending_upgrade_code '{"code_hash": "<hash>"}' \
  --accountId member1.near --deposit 0.000000000000000000000001
```

---

## DAO Proposal Reference

All available proposal actions:

| Action | Description |
|--------|-------------|
| `add_oracle` | Register an account as oracle |
| `remove_oracle` | Remove oracle |
| `add_asset` | Add price asset (optional `push_signer_key`) |
| `remove_asset` | Remove asset |
| `add_asset_ema` | Add EMA tracking for asset |
| `remove_asset_ema` | Remove EMA |
| `set_push_signer_accounts` | Set allowed push accounts per asset |
| `set_push_signer_keys` | Batch set/remove TEE keys for assets |
| `register_push_signer` | Register TEE signer (created by `propose_register_push_signer`) |
| `set_recency_duration_sec` | Set price staleness threshold |
| `configure_outlayer` | Set OutLayer contract, code source, secrets |
| `set_subsidize_outlayer_calls` | Enable/disable subsidized calls |
| `update_near_claim_amount` | Set oracle NEAR reward amount |
| `add_price_mapping` | Add Pyth price ID → asset mapping |
| `remove_price_mapping` | Remove Pyth mapping |
| `set_pyth_stale_threshold` | Set Pyth staleness threshold |
| `add_council_member` | Add council member |
| `remove_council_member` | Remove council member |
| `update_owner` | Transfer ownership |
| `upgrade_contract` | Deploy uploaded code (with optional migration) |
| `pause` | Pause contract |
| `unpause` | Unpause contract |

---

## View Methods

```bash
near view price-oracle.near get_version
near view price-oracle.near get_owner_id
near view price-oracle.near get_council_members
near view price-oracle.near get_council_threshold
near view price-oracle.near get_proposals '{"limit": 20}'
near view price-oracle.near get_proposal '{"id": 0}'
near view price-oracle.near get_oracles
near view price-oracle.near get_assets
near view price-oracle.near get_price_data '{"asset_ids": ["wrap.near"]}'
near view price-oracle.near get_asset_oracle_keys
near view price-oracle.near get_push_signer_accounts '{"asset_id": "wrap.near"}'
near view price-oracle.near get_pending_upgrade_hashes
near view price-oracle.near can_subsidize_outlayer_calls
near view price-oracle.near is_paused
```

## Costs

| Component | Cost |
|-----------|------|
| Contract deployment | ~2 NEAR |
| Scheduler update (via payment key) | ~0.001 NEAR |
| `oracle_call` (stale, user pays) | ~0.01-0.02 NEAR |
| `oracle_call` (stale, subsidized) | ~0.02 NEAR (from contract) |
| `oracle_call` (cached) | ~0.0001 NEAR (gas only) |
| Upload upgrade code | ~1 NEAR per 100KB (refunded on deploy) |
