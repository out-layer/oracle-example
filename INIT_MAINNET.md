# Mainnet Initialization Commands

Contract: `price-oracle.near`
Owner: `owner.price-oracle.near`

---

## 1. Build

```bash
cd oracle-example/contract
cargo near build
```

## 2. Upgrade + migrate (last owner redeploy)

There is no `upgrade` method. Redeploy directly with the `price-oracle.near`
account's own full-access key, running the migration that matches the currently
deployed state (`migrate_state` → pre-Pyth/Council, `migrate_state2` →
pre-oracle-keys, `migrate_state3` → pre-exchange-configs). After this, the
council owns all future upgrades.

```bash
near contract deploy price-oracle.near \
  use-file target/near/price_oracle.wasm \
  with-init-call migrate_state2 json-args '{}' \
  prepaid-gas '300.0 Tgas' attached-deposit '0 NEAR' \
  network-config mainnet sign-with-keychain send
```

## 3. Verify

```bash
near view price-oracle.near get_version --networkId mainnet
```

## 4. Set council

```bash
near call price-oracle.near set_council '{
  "members": ["owner.price-oracle.near"]
}' --accountId owner.price-oracle.near --depositYocto 1 --networkId mainnet
```

## 5. Configure OutLayer

```bash
near call price-oracle.near create_proposal '{"action": {
  "action": "configure_outlayer",
  "outlayer_contract_id": "outlayer.near",
  "code_source": "{\"Project\":{\"project_id\":\"price-oracle.near/price-oracle\"}}",
  "secrets_profile": "oracle",
  "secrets_account_id": "price-oracle.near"
}}' --accountId owner.price-oracle.near --deposit 0.1 --networkId mainnet
```

Both secrets fields are optional on the contract. The live `price-oracle.near` currently has them
unset, which means contract-initiated OutLayer calls carry no `secrets_ref`; the scheduler supplies
its own via `SECRETS_PROFILE`/`SECRETS_ACCOUNT_ID`. Set them here only if you want the contract
itself to trigger executions that need secrets.

## 6. Add all assets

```bash
near call price-oracle.near create_proposals '{"actions": [
  {"action": "add_asset", "asset_id": "xrp.omft.near", "push_signer_key": null},
  {"action": "add_asset", "asset_id": "doge.omft.near", "push_signer_key": null},
  {"action": "add_asset", "asset_id": "cardano.omft.near", "push_signer_key": null},
  {"action": "add_asset", "asset_id": "xlm", "push_signer_key": null},
  {"action": "add_asset", "asset_id": "ltc.omft.near", "push_signer_key": null}
]}' --accountId owner.price-oracle.near --deposit 0.1 --networkId mainnet
```

## 7. Add EMAs (wrap.near)

```bash
near call price-oracle.near create_proposals '{"actions": [
  {"action": "add_asset_ema", "asset_id": "wrap.near", "period_sec": 3600},
  {"action": "add_asset_ema", "asset_id": "wrap.near", "period_sec": 86400}
]}' --accountId owner.price-oracle.near --deposit 0.1 --networkId mainnet
```

# UPLOAD WASM TO FASTFS

python3 upload_wasm_fastfs.py ../oracle-example/oracle-example.wasm ../worker/.env.mainnet.worker1

UPLOAD FILE TO OUTLAYER PROJECT

ADD A SECRET

## 8. Register TEE push signer (PROTECTED_KEY_RHEA)

```bash
near call price-oracle.near propose_register_push_signer '{
  "push_signer_key": "PROTECTED_KEY_RHEA",
  "asset_ids": [
    "wrap.near",
    "aurora",
    "usdt.tether-token.near",
    "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1",
    "nbtc.bridge.near",
    "2260fac5e5542a773aa44fbcfedf7c193bc2c599.factory.bridge.near",
    "6b175474e89094c44da98b954eedeac495271d0f.factory.bridge.near",
    "aaaaaa20d9e0e2461697782ef11675f668207961.factory.bridge.near",
    "4691937a7508860f876c9c0a2a617e7d9e945d4b.factory.bridge.near",
    "853d955acef822db058eb8505911ed77f175b99e.factory.bridge.near",
    "22.contract.portalbridge.near",
    "zec.omft.near",
    "token.rhealab.near"
  ],
  "secrets_profile": "oracle",
  "secrets_account_id": "price-oracle.near"
}' --accountId owner.price-oracle.near --depositYocto 1 --gas 300000000000000 --networkId mainnet
```

## 9. Check the proposal created by callback

```bash
near view price-oracle.near get_proposals '{"limit": 5}' --networkId mainnet
```

## 10. Fund the implicit account

```bash
# Get the implicit account ID from the proposal
near view price-oracle.near get_push_signer_accounts '{"asset_id": "wrap.near"}' --networkId mainnet

# Fund it (needs NEAR for gas to send report_prices transactions)
near send owner.price-oracle.near <implicit_account_id> 0.1 --networkId mainnet
```

## 11. Enable subsidized OutLayer calls

```bash
near call price-oracle.near create_proposal '{"action": {
  "action": "set_subsidize_outlayer_calls",
  "enabled": true
}}' --accountId owner.price-oracle.near --deposit 0.1 --networkId mainnet
```

## 12. Verify everything

```bash
near view price-oracle.near get_version --networkId mainnet
near view price-oracle.near get_council_members --networkId mainnet
near view price-oracle.near get_assets --networkId mainnet
near view price-oracle.near get_asset_oracle_keys --networkId mainnet
near view price-oracle.near get_oracles --networkId mainnet
near view price-oracle.near can_subsidize_outlayer_calls --networkId mainnet
```

---

# Exchange Config Migration (DAO-managed)

After upgrading the contract with `migrate_state3`, exchange configs are stored
on-chain as opaque JSON and synced to WASI public storage. This replaces the
compiled `tokens.json` — no WASM rebuild needed when adding tokens/exchanges.

## 1. Upgrade contract + migrate

Build and upload the new WASM, then create an upgrade proposal with migration:

```bash
cd oracle-example/contract
cargo near build

# Upload code
WASM_BASE64=$(base64 -i target/near/price_oracle.wasm)
near call price-oracle.near upload_upgrade_code \
  --base64 "$WASM_BASE64" \
  --accountId owner.price-oracle.near \
  --deposit 5 \
  --gas 300000000000000 \
  --networkId mainnet

# Get code hash from logs, then create upgrade proposal
near call price-oracle.near create_proposal '{"action": {
  "action": "upgrade_contract",
  "code_hash": "<CODE_HASH_FROM_UPLOAD>",
  "migrate_method": "migrate_state3"
}}' --accountId owner.price-oracle.near --deposit 0.1 --networkId mainnet
```

## 2. Set exchange configs (batch proposal)

One proposal sets all 18 asset configs at once. Attach deposit for storage (~0.5 NEAR).

```bash
near call price-oracle.near create_proposal '{"action": {
  "action": "set_asset_exchange_configs",
  "configs": [
    ["wrap.near", "{\"decimals\":24,\"coingecko\":\"near\",\"binance\":\"NEARUSDT\",\"binance_us\":\"NEARUSD\",\"huobi\":\"nearusdt\",\"cryptocom\":\"NEAR_USDT\",\"kucoin\":\"NEAR-USDT\",\"gate\":\"near_usdt\",\"pyth\":\"0xc415de8d2eba7db216527dff4b60e8f3a5311c740dadb233e13e12547e226750\"}"],
    ["aurora", "{\"decimals\":18,\"coingecko\":\"ethereum\",\"binance\":\"ETHUSDT\",\"binance_us\":\"ETHUSD\",\"huobi\":\"ethusdt\",\"cryptocom\":\"ETH_USDT\",\"kucoin\":\"ETH-USDT\",\"gate\":\"eth_usdt\",\"pyth\":\"0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace\",\"chainlink\":\"0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419\"}"],
    ["usdt.tether-token.near", "{\"decimals\":6,\"stablecoin\":true,\"coingecko\":\"tether\",\"pyth\":\"0x2b89b9dc8fdf9f34709a5b106b472f0f39bb6ca9ce04b0fd7f2e971688e2e53b\",\"chainlink\":\"0x3E7d1eAB13ad0104d2750B8863b489D65364e32D\"}"],
    ["17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1", "{\"decimals\":6,\"stablecoin\":true,\"coingecko\":\"usd-coin\",\"cryptocom\":\"USDC_USDT\",\"kucoin\":\"USDC-USDT\",\"binance\":\"USDCUSDT\",\"pyth\":\"0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a\",\"chainlink\":\"0x8fFfFfd4AfB6115b954Bd326cbe7B4BA576818f6\"}"],
    ["nbtc.bridge.near", "{\"decimals\":8,\"coingecko\":\"bitcoin\",\"binance\":\"BTCUSDT\",\"binance_us\":\"BTCUSD\",\"huobi\":\"btcusdt\",\"cryptocom\":\"BTC_USDT\",\"kucoin\":\"BTC-USDT\",\"gate\":\"btc_usdt\",\"pyth\":\"0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43\",\"chainlink\":\"0xF4030086522a5bEEa4988F8cA5B36dbC97BeE88c\"}"],
    ["2260fac5e5542a773aa44fbcfedf7c193bc2c599.factory.bridge.near", "{\"decimals\":8,\"coingecko\":\"wrapped-bitcoin\",\"binance\":\"WBTCBTC\",\"huobi\":\"wbtcusdt\",\"cryptocom\":\"WBTC_USDT\",\"kucoin\":\"WBTC-USDT\",\"gate\":\"wbtc_usdt\",\"pyth\":\"0xc9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33\"}"],
    ["6b175474e89094c44da98b954eedeac495271d0f.factory.bridge.near", "{\"decimals\":18,\"stablecoin\":true,\"coingecko\":\"dai\",\"binance\":\"DAIUSDT\",\"binance_us\":\"DAIUSD\",\"huobi\":\"daiusdt\",\"cryptocom\":\"DAI_USDT\",\"gate\":\"dai_usdt\",\"pyth\":\"0xb0948a5e5313200c632b51bb5ca32f6de0d36e9950a942d19751e833f70dabfd\",\"chainlink\":\"0xAed0c38402a5d19df6E4c03F4E2DceD6e29c1ee9\"}"],
    ["aaaaaa20d9e0e2461697782ef11675f668207961.factory.bridge.near", "{\"decimals\":18,\"coingecko\":\"aurora-near\",\"cryptocom\":\"AURORA_USDT\",\"huobi\":\"aurorausdt\",\"kucoin\":\"AURORA-USDT\",\"gate\":\"aurora_usdt\",\"pyth\":\"0x2f7c4f738d498585065a4b87b637069ec99474597da7f0ca349ba8ac3ba9cac5\"}"],
    ["4691937a7508860f876c9c0a2a617e7d9e945d4b.factory.bridge.near", "{\"decimals\":18,\"coingecko\":\"woo-network\",\"binance\":\"WOOUSDT\",\"huobi\":\"woousdt\",\"cryptocom\":\"WOO_USDT\",\"kucoin\":\"WOO-USDT\",\"gate\":\"woo_usdt\",\"pyth\":\"0xb82449fd728133488d2d41131cffe763f9c1693b73c544d9ef6aaa371060dd25\"}"],
    ["853d955acef822db058eb8505911ed77f175b99e.factory.bridge.near", "{\"decimals\":18,\"stablecoin\":true,\"coingecko\":\"frax\",\"pyth\":\"0x7c53208632935ba5122c3cf65a0f4b3e72ba4955b49ad6ba0acf3d9ba405aef3\",\"chainlink\":\"0xB9E1E3A9feFf48998E45Fa90847ed4D467E8BcfD\"}"],
    ["22.contract.portalbridge.near", "{\"decimals\":8,\"coingecko\":\"solana\",\"binance\":\"SOLUSDT\",\"binance_us\":\"SOLUSD\",\"huobi\":\"solusdt\",\"cryptocom\":\"SOL_USDT\",\"kucoin\":\"SOL-USDT\",\"gate\":\"sol_usdt\",\"pyth\":\"0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d\"}"],
    ["zec.omft.near", "{\"decimals\":8,\"coingecko\":\"zcash\",\"binance\":\"ZECUSDT\",\"binance_us\":\"ZECUSD\",\"huobi\":\"zecusdt\",\"kucoin\":\"ZEC-USDT\",\"gate\":\"zec_usdt\",\"pyth\":\"0xbe9b59d178f0d6a97ab4c343bff2aa69caa1eaae3e9048a65788c529b125bb24\"}"],
    ["token.rhealab.near", "{\"decimals\":18,\"binance_alpha\":\"0x4c067de26475e1cefee8b8d1f6e2266b33a2372e\",\"pyth\":\"0xded2a0d2624278a32c56725397cc98b24ddb83d8c4d2ce108b1fc44b1d8de22b\"}"],
    ["xrp.omft.near", "{\"decimals\":6,\"coingecko\":\"ripple\",\"binance\":\"XRPUSDT\",\"binance_us\":\"XRPUSD\",\"huobi\":\"xrpusdt\",\"cryptocom\":\"XRP_USDT\",\"kucoin\":\"XRP-USDT\",\"gate\":\"xrp_usdt\",\"pyth\":\"0xec5d399846a9209f3fe5881d70aae9268c94339ff9817e8d18ff19fa05eea1c8\",\"chainlink\":\"0xCed2660c6Dd1Ffd856A5A82C67f3482d88C50b12\"}"],
    ["doge.omft.near", "{\"decimals\":8,\"coingecko\":\"dogecoin\",\"binance\":\"DOGEUSDT\",\"binance_us\":\"DOGEUSD\",\"huobi\":\"dogeusdt\",\"cryptocom\":\"DOGE_USDT\",\"kucoin\":\"DOGE-USDT\",\"gate\":\"doge_usdt\",\"pyth\":\"0xdcef50dd0a4cd2dcc17e45df1676dcb336a11a61c69df7a0299b0150c672d25c\"}"],
    ["cardano.omft.near", "{\"decimals\":6,\"coingecko\":\"cardano\",\"binance\":\"ADAUSDT\",\"binance_us\":\"ADAUSD\",\"huobi\":\"adausdt\",\"cryptocom\":\"ADA_USDT\",\"kucoin\":\"ADA-USDT\",\"gate\":\"ada_usdt\",\"pyth\":\"0x2a01deaec9e51a579277b34b122399984d0bbf57e2458a7e42fecd2829867a0d\",\"chainlink\":\"0xAE48c91dF1fE419994FFDa27da09D5aC69c30f55\"}"],
    ["xlm", "{\"decimals\":7,\"coingecko\":\"stellar\",\"binance\":\"XLMUSDT\",\"binance_us\":\"XLMUSD\",\"huobi\":\"xlmusdt\",\"cryptocom\":\"XLM_USDT\",\"kucoin\":\"XLM-USDT\",\"gate\":\"xlm_usdt\",\"pyth\":\"0xb7a8eba68a997cd0210c2e1e4ee811ad2d174b3611c22d9ebf16f4cb7e9ba850\"}"],
    ["ltc.omft.near", "{\"decimals\":8,\"coingecko\":\"litecoin\",\"binance\":\"LTCUSDT\",\"binance_us\":\"LTCUSD\",\"huobi\":\"ltcusdt\",\"cryptocom\":\"LTC_USDT\",\"kucoin\":\"LTC-USDT\",\"gate\":\"ltc_usdt\",\"pyth\":\"0x6e3f3fa8253588df9326580180233eb791e03b443a3ba7a1d892e73874e19a54\",\"chainlink\":\"0x6AF09DF7563C363B5763b9102712EbeD3b9e859B\"}"]
  ]
}}' --accountId owner.price-oracle.near --deposit 0.5 --gas 300000000000000 --networkId mainnet
```

## 3. Deploy updated WASI

Upload the new WASM that reads config from storage instead of compiled `tokens.json`.

```bash
cd oracle-example
cargo build --target wasm32-wasip2 --release
outlayer upload target/wasm32-wasip2/release/oracle-example.wasm
outlayer deploy price-oracle <FastFS URL printed by upload>   # as price-oracle.near
```

## 4. Sync configs to WASI public storage

After the proposal is approved, sync writes `config:assets` to public storage.
Anyone can call this (idempotent). Deposit covers OutLayer execution.

```bash
near call price-oracle.near sync_asset_configs '{}' \
  --accountId owner.price-oracle.near \
  --deposit 0.05 \
  --gas 300000000000000 \
  --networkId mainnet
```



## 5. Verify configs

```bash
# Check configs stored on contract
near view price-oracle.near get_asset_exchange_configs --networkId mainnet

# Check single asset config
near view price-oracle.near get_asset_exchange_config '{"asset_id": "wrap.near"}' --networkId mainnet
```

### Adding a new token later

```bash
# 1. Add asset
near call price-oracle.near create_proposal '{"action": {
  "action": "add_asset",
  "asset_id": "new-token.near",
  "push_signer_key": "PROTECTED_KEY_RHEA"
}}' --accountId owner.price-oracle.near --deposit 0.1 --networkId mainnet

# 2. Set exchange config
near call price-oracle.near create_proposal '{"action": {
  "action": "set_asset_exchange_config",
  "asset_id": "new-token.near",
  "config": "{\"decimals\":18,\"coingecko\":\"new-token\",\"binance\":\"NEWTOKENUSDT\"}"
}}' --accountId owner.price-oracle.near --deposit 0.1 --networkId mainnet

# 3. Sync to WASI — no WASM rebuild needed
near call price-oracle.near sync_asset_configs '{}' \
  --accountId owner.price-oracle.near \
  --deposit 0.05 \
  --gas 300000000000000 \
  --networkId mainnet
```
