//! Signed price feed (pull model) for external consumers.
//!
//! Consumers fetch the feed themselves and verify the Ed25519 signature over the exact
//! bytes we emit — off-chain today, on-chain later. Nothing here touches the chain, so a
//! pull costs us no gas.
//!
//! Signature domain per format:
//! - `json`  — the UTF-8 bytes of the `payload` string (payload is the signed message)
//! - `borsh` — the raw bytes of `base64_decode(payload)`
//!
//! A verifier MUST NOT re-serialize the payload before checking the signature: key order,
//! float formatting and whitespace would all change the bytes.
//!
//! # Invariant: the key of every payload entry is OUR canonical `asset_id`
//!
//! Nothing a caller sends may influence it. A signature only certifies "this enclave
//! observed this price for this asset", so the asset identity has to be ours end to end —
//! the moment the requester picks the name, the signature certifies the requester's claim
//! instead of our measurement. A rename map used to exist here and made exactly that
//! possible: asking for `bitcoin` under the name `wrap.near` produced a payload our key
//! had signed that said `wrap.near` was worth $118,000. A consumer checking the signature
//! and reading the asset id off the payload — which is the whole point of the feed — would
//! have priced $2.50 of collateral at $118,000. Consumers use our `asset_id` directly;
//! an asset that needs a different name gets published as its own asset in the config.

use crate::types::AggregationMethod;
use base64::Engine;
use borsh::BorshSerialize;
use oracle_example_sources::{parsers, ExchangeConfig};
use serde::Serialize;
use std::collections::BTreeMap;

/// The one in-enclave Ed25519 key that signs this feed.
///
/// # Why this is a constant and not a request parameter
///
/// It used to be `key_name` on the request, validated only by the `PROTECTED_` prefix — so the
/// CALLER chose which enclave key signed. Every `PROTECTED_` secret the worker holds was in
/// reach, including `PROTECTED_KEY_RHEA`, the key that signs `report_prices` transactions.
/// The feed payload is arbitrary caller-influenced bytes; a NEAR transaction is signed over a
/// SHA-256 hash of its borsh encoding. Nothing about the signing primitive distinguishes the
/// two, so "sign these bytes with the transaction key" is a request for a signature that may
/// later be replayed as something else entirely. Who signs is our decision, not the caller's.
///
/// # Why there is no domain-separation tag
///
/// Because this key is fixed here and used ONLY for the feed, it can never sign a NEAR
/// transaction: feed payloads and transaction hashes are separated by the KEY, not by a tag
/// inside the message. That is a stronger separation than a prefix byte — it does not depend
/// on every future code path remembering to add it.
///
/// # The invariant that makes it true
///
/// This key must NEVER be reused for transaction signing, on-chain or otherwise. The moment it
/// signs anything but a feed payload the separation above collapses and a tagged domain would
/// have to be introduced instead. `report_prices` uses its own separate key (see DAO_LOG.md),
/// and `feed_signing_key_is_pinned_and_transaction_free` pins that they stay distinct.
///
/// Operationally this names the secret in the worker's secrets profile (`profile: "oracle"`,
/// `account_id: "price-oracle.near"`). Its public half is what consumers pin, so changing this
/// string changes the feed's identity: it requires creating the new secret, publishing the new
/// public key, and updating every consumer that pinned the old one.
///
/// If you are adapting this worker: pick a name specific to YOUR deployment rather than a
/// generic one. A shared-looking name invites two deployments to reach for the same secret and
/// makes it ambiguous which enclave a given public key belongs to — and the identity of the
/// signer is the entire value of the feed.
pub const FEED_SIGNING_KEY: &str = "PROTECTED_RHEA_FEED_KEY";

/// Default exponent of the signed feed: real price = `price * 10^expo`
pub const DEFAULT_EXPO: i32 = -8;

/// Accepted exponent range (keeps `10^-expo` and the scaled value inside f64/i64 range)
const MIN_EXPO: i32 = -18;
const MAX_EXPO: i32 = 18;

/// 2^63 — the first f64 value that no longer fits in i64
const I64_OVERFLOW: f64 = 9_223_372_036_854_775_808.0;

/// Source names accepted in `exclude_sources` (must match `SourcePrice.source_name`).
///
/// This list, `filter_exchange_config` and `configured_sources` all enumerate the same set
/// and must stay in step — `every_source_is_filterable` fails the build if they drift.
pub const KNOWN_SOURCES: [&str; 16] = [
    "coingecko",
    "binance",
    "binance_us",
    "binance_alpha",
    "pyth",
    "chainlink",
    "huobi",
    "kucoin",
    "gate",
    "cryptocom",
    "kraken",
    "coinbase",
    "bitstamp",
    "okx",
    "bitget",
    "mexc",
];

/// Serialization format of the signed payload
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SigFormat {
    /// Deterministic JSON object, keys sorted (default)
    Json,
    /// Borsh-serialized `BTreeMap<String, PriceEntry>`, base64-encoded in the response
    Borsh,
}

impl SigFormat {
    /// Parse the requested format, defaulting to JSON when omitted
    pub fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("json") {
            "json" => Ok(SigFormat::Json),
            "borsh" => Ok(SigFormat::Borsh),
            other => Err(format!(
                "Unsupported sig_format: '{}' (expected 'json' or 'borsh')",
                other
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SigFormat::Json => "json",
            SigFormat::Borsh => "borsh",
        }
    }
}

/// One asset in the signed payload.
/// `price` is scaled by `10^-expo`; `publish_time` is the unix second at which the TEE
/// observed the oldest venue that contributed to this price — the age of the aggregate, not
/// the time of the request that returned it.
#[derive(Debug, Clone, Copy, PartialEq, BorshSerialize)]
pub struct PriceEntry {
    pub price: i64,
    pub expo: i32,
    pub publish_time: i64,
}

/// JSON projection of `PriceEntry` — `price` is an i64 rendered as a JSON string,
/// `expo` and `publish_time` stay JSON numbers.
#[derive(Debug, Serialize)]
struct JsonPriceEntry {
    price: String,
    expo: i32,
    publish_time: i64,
}

/// Validate the requested exponent
pub fn validate_expo(expo: i32) -> Result<(), String> {
    if !(MIN_EXPO..=MAX_EXPO).contains(&expo) {
        return Err(format!(
            "expo {} out of range (expected {}..={})",
            expo, MIN_EXPO, MAX_EXPO
        ));
    }
    Ok(())
}

/// Scale a float price into the i64 carried by the payload: `round(price * 10^-expo)`.
/// Rejects anything that would silently lose the price (non-finite, non-positive,
/// overflowing i64, or underflowing to zero).
pub fn scale_price(price: f64, expo: i32) -> Result<i64, String> {
    if !price.is_finite() || price <= 0.0 {
        return Err(format!("invalid price {}", price));
    }
    let scaled = (price * 10f64.powi(-expo)).round();
    if !scaled.is_finite() || scaled.abs() >= I64_OVERFLOW {
        return Err(format!("price {} does not fit i64 at expo {}", price, expo));
    }
    if scaled == 0.0 {
        return Err(format!("price {} underflows to 0 at expo {}", price, expo));
    }
    Ok(scaled as i64)
}

/// Build the payload entries from `(asset_id, price, timestamp)` triples.
/// Keys are our canonical asset ids (see the module invariant) and the `BTreeMap` gives the
/// deterministic ordering the signature depends on.
pub fn build_entries(
    priced: &[(String, f64, u64)],
    expo: i32,
) -> Result<BTreeMap<String, PriceEntry>, String> {
    let mut entries: BTreeMap<String, PriceEntry> = BTreeMap::new();

    for (asset_id, price, timestamp) in priced {
        let entry = PriceEntry {
            price: scale_price(*price, expo).map_err(|e| format!("{}: {}", asset_id, e))?,
            expo,
            publish_time: i64::try_from(*timestamp)
                .map_err(|_| format!("{}: timestamp {} does not fit i64", asset_id, timestamp))?,
        };
        entries.insert(asset_id.clone(), entry);
    }

    Ok(entries)
}

/// Encode the payload and return `(payload_string, bytes_to_sign)`.
/// For JSON the two are the same bytes; for borsh the payload is the base64 of the bytes.
pub fn encode_payload(
    entries: &BTreeMap<String, PriceEntry>,
    format: SigFormat,
) -> Result<(String, Vec<u8>), String> {
    match format {
        SigFormat::Json => {
            let json: BTreeMap<&str, JsonPriceEntry> = entries
                .iter()
                .map(|(key, entry)| {
                    (
                        key.as_str(),
                        JsonPriceEntry {
                            price: entry.price.to_string(),
                            expo: entry.expo,
                            publish_time: entry.publish_time,
                        },
                    )
                })
                .collect();
            let payload = serde_json::to_string(&json)
                .map_err(|e| format!("Failed to serialize payload: {}", e))?;
            let bytes = payload.as_bytes().to_vec();
            Ok((payload, bytes))
        }
        SigFormat::Borsh => {
            let bytes = borsh::to_vec(entries)
                .map_err(|e| format!("Failed to borsh-serialize payload: {}", e))?;
            let payload = base64::engine::general_purpose::STANDARD.encode(&bytes);
            Ok((payload, bytes))
        }
    }
}

/// Base64-encode the 64-byte Ed25519 signature for the response
pub fn encode_signature(signature: &[u8; 64]) -> String {
    base64::engine::general_purpose::STANDARD.encode(signature)
}

/// Resolve a tier selection (`only_sources` / `exclude_sources`) into the exclusion list that
/// `filter_exchange_config` consumes.
///
/// "Refresh only Pyth and Chainlink" is expressed as "exclude everything else" on purpose:
/// there is then a single place that maps a source name onto an `ExchangeConfig` field, so a
/// venue added to one list and forgotten in the other cannot silently keep feeding a refresh
/// that was supposed to skip it. Both lists are validated against `KNOWN_SOURCES` first.
///
/// Giving both narrows twice: the effective set is `only_sources` minus `exclude_sources`.
pub fn resolve_source_selection(
    only: Option<&[String]>,
    exclude: Option<&[String]>,
) -> Result<Vec<String>, String> {
    let only = match only {
        Some(list) => Some(validate_selection(list, "only_sources")?),
        None => None,
    };
    let exclude = match exclude {
        Some(list) => validate_exclusions(list)?,
        None => Vec::new(),
    };

    let mut cleared: Vec<String> = Vec::new();
    for source in KNOWN_SOURCES {
        let allowed = only
            .as_ref()
            .map(|list| list.iter().any(|s| s == source))
            .unwrap_or(true)
            && !exclude.iter().any(|s| s == source);
        if !allowed {
            cleared.push(source.to_string());
        }
    }
    Ok(cleared)
}

/// Validate and normalize `exclude_sources`.
/// Unknown names are rejected: silently ignoring a typo would leave a source the caller
/// believes is excluded still contributing to the signed price.
pub fn validate_exclusions(exclude: &[String]) -> Result<Vec<String>, String> {
    validate_selection(exclude, "exclude_sources")
}

/// Validate and normalize a list of source names coming from a request.
///
/// The error names the offending field, because the same rejection reaching a caller as
/// "unknown source" without saying which list it was in is the kind of message that costs an
/// integration an afternoon.
pub fn validate_selection(sources: &[String], field: &str) -> Result<Vec<String>, String> {
    let mut normalized: Vec<String> = Vec::with_capacity(sources.len());
    for source in sources {
        let name = source.trim().to_ascii_lowercase();
        if !KNOWN_SOURCES.contains(&name.as_str()) {
            return Err(format!(
                "Unknown source in {}: '{}' (known: {})",
                field,
                source,
                KNOWN_SOURCES.join(", ")
            ));
        }
        if !normalized.contains(&name) {
            normalized.push(name);
        }
    }
    Ok(normalized)
}

/// Check whether a source name is in the (already normalized) exclusion list
pub fn is_excluded(source_name: &str, exclude: &[String]) -> bool {
    exclude.iter().any(|name| name.eq_ignore_ascii_case(source_name))
}

/// Return a copy of `config` with every excluded source cleared.
/// `fetch_all_sources` picks sources purely by which fields are `Some`, so clearing a
/// field removes that source from the fetch. Names are validated by `validate_exclusions`.
pub fn filter_exchange_config(config: &ExchangeConfig, exclude: &[String]) -> ExchangeConfig {
    config.without_sources(exclude)
}

/// List the sources a config still has configured (used for actionable error messages)
pub fn configured_sources(config: &ExchangeConfig) -> Vec<&'static str> {
    config.configured_sources()
}

/// Set every source field of an `ExchangeConfig` to `Some`, so a test can assert that a
/// source is actually removable rather than only that it is spelled correctly somewhere.
///
/// This is written as an exhaustive struct literal ON PURPOSE: adding a field to
/// `ExchangeConfig` breaks compilation here, which is the earliest possible warning that
/// `KNOWN_SOURCES` / `filter_exchange_config` / `configured_sources` need the new venue too.
#[cfg(test)]
fn all_sources_configured() -> ExchangeConfig {
    ExchangeConfig {
        coingecko: Some("near".to_string()),
        binance: Some("NEARUSDT".to_string()),
        binance_us: Some("NEARUSD".to_string()),
        binance_alpha: Some("0xabc".to_string()),
        pyth: Some("0xdef".to_string()),
        chainlink: Some("0x0123".to_string()),
        huobi: Some("nearusdt".to_string()),
        kucoin: Some("NEAR-USDT".to_string()),
        gate: Some("near_usdt".to_string()),
        cryptocom: Some("NEAR_USDT".to_string()),
        kraken: Some("NEARUSD".to_string()),
        coinbase: Some("NEAR-USD".to_string()),
        bitstamp: Some("NEAR/USD".to_string()),
        okx: Some("NEAR-USDT".to_string()),
        bitget: Some("NEARUSDT".to_string()),
        mexc: Some("NEARUSDT".to_string()),
        stablecoin: false,
        decimals: Some(24),
    }
}

/// Aggregate prices with the requested method (same math as `fetch_and_store_price`).
///
/// `None` means no source produced a usable number, and every caller must surface that as a
/// per-asset failure. It is deliberately not a price: the 0.0 these methods used to return
/// for an empty input would have been signed and handed to a lending market as "this asset is
/// worth nothing" — a statement our key would then be certifying.
pub fn aggregate(prices: &[f64], method: AggregationMethod) -> Option<f64> {
    match method {
        AggregationMethod::Average => parsers::average(prices),
        AggregationMethod::Median => parsers::median(prices),
        AggregationMethod::WeightedAverage => parsers::weighted_average(prices),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::near_tx;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    fn priced(asset_id: &str, price: f64, timestamp: u64) -> (String, f64, u64) {
        (asset_id.to_string(), price, timestamp)
    }

    #[test]
    fn scales_price_to_i64_at_default_expo() {
        assert_eq!(scale_price(0.99920522, DEFAULT_EXPO).unwrap(), 99_920_522);
        assert_eq!(scale_price(1.0, DEFAULT_EXPO).unwrap(), 100_000_000);
        assert_eq!(scale_price(2.5, DEFAULT_EXPO).unwrap(), 250_000_000);
        assert_eq!(scale_price(65_432.1, DEFAULT_EXPO).unwrap(), 6_543_210_000_000);
        // rounds to nearest instead of truncating (truncation would give 199_999_999)
        assert_eq!(scale_price(1.999999999, DEFAULT_EXPO).unwrap(), 200_000_000);
        // other exponents scale accordingly
        assert_eq!(scale_price(2.5, -6).unwrap(), 2_500_000);
        assert_eq!(scale_price(2.5e9, 3).unwrap(), 2_500_000);
        // and anything that would lose the price is an error, never a silent 0
        assert!(scale_price(f64::NAN, DEFAULT_EXPO).is_err());
        assert!(scale_price(0.0, DEFAULT_EXPO).is_err());
        assert!(scale_price(-1.0, DEFAULT_EXPO).is_err());
        assert!(scale_price(1e30, DEFAULT_EXPO).is_err());
        assert!(scale_price(1e-12, DEFAULT_EXPO).is_err());
        assert!(validate_expo(DEFAULT_EXPO).is_ok());
        assert!(validate_expo(-19).is_err());
    }

    #[test]
    fn payload_key_order_is_deterministic() {
        let forward = vec![
            priced("wrap.near", 2.5, 1784720718),
            priced("aurora", 3000.0, 1784720718),
            priced("usdt.tether-token.near", 1.0, 1784720718),
        ];
        let mut reversed = forward.clone();
        reversed.reverse();

        let first = encode_payload(
            &build_entries(&forward, DEFAULT_EXPO).unwrap(),
            SigFormat::Json,
        )
        .unwrap()
        .0;
        let second = encode_payload(
            &build_entries(&reversed, DEFAULT_EXPO).unwrap(),
            SigFormat::Json,
        )
        .unwrap()
        .0;

        assert_eq!(first, second);
        assert_eq!(
            first,
            concat!(
                r#"{"aurora":{"price":"300000000000","expo":-8,"publish_time":1784720718},"#,
                r#""usdt.tether-token.near":{"price":"100000000","expo":-8,"publish_time":1784720718},"#,
                r#""wrap.near":{"price":"250000000","expo":-8,"publish_time":1784720718}}"#
            )
        );
    }

    /// Pins the module invariant: every payload key is our own asset_id.
    ///
    /// The removed alias map broke exactly this — pricing `bitcoin` under the name
    /// `wrap.near` yielded a payload our key had signed claiming `wrap.near` was worth
    /// $118,000, which any consumer reading the asset id off the verified payload would
    /// have booked as collateral. If a change ever makes the key caller-controlled again,
    /// this test is where it must fail.
    #[test]
    fn payload_keys_are_our_asset_ids() {
        let entries = build_entries(
            &[priced("aurora", 3000.0, 7), priced("wrap.near", 2.5, 7)],
            DEFAULT_EXPO,
        )
        .unwrap();

        assert_eq!(
            entries.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["aurora", "wrap.near"]
        );
        assert_eq!(entries["aurora"].price, 300_000_000_000);
    }

    #[test]
    fn signature_verifies_against_derived_public_key() {
        // Deterministic 32-byte seed in NEAR "ed25519:<base58>" form
        let private_key = format!("ed25519:{}", bs58::encode([7u8; 32]).into_string());
        let (_, public_key) = near_tx::derive_implicit_account(&private_key).unwrap();

        let entries =
            build_entries(&[priced("wrap.near", 2.5, 1784720718)], DEFAULT_EXPO).unwrap();
        let (payload, message) = encode_payload(&entries, SigFormat::Json).unwrap();
        assert_eq!(message, payload.as_bytes());

        let signature = near_tx::sign_message(&private_key, &message).unwrap();

        // Exactly what a client does with the response: decode public_key, verify signature
        let key_bytes: [u8; 32] = bs58::decode(public_key.strip_prefix("ed25519:").unwrap())
            .into_vec()
            .unwrap()
            .try_into()
            .unwrap();
        let verifying_key = VerifyingKey::from_bytes(&key_bytes).unwrap();
        verifying_key
            .verify(&message, &Signature::from_bytes(&signature))
            .expect("signature must verify over the exact payload bytes");

        // the base64 we hand out decodes back to the same 64 bytes
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encode_signature(&signature))
            .unwrap();
        assert_eq!(decoded, signature.to_vec());

        // a mutated payload must not verify
        let tampered = payload.replace("250000000", "250000001");
        assert!(verifying_key
            .verify(tampered.as_bytes(), &Signature::from_bytes(&signature))
            .is_err());
    }

    #[test]
    fn borsh_payload_signs_the_decoded_bytes() {
        let entries =
            build_entries(&[priced("wrap.near", 2.5, 1784720718)], DEFAULT_EXPO).unwrap();
        let (payload, message) = encode_payload(&entries, SigFormat::Borsh).unwrap();

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&payload)
            .unwrap();
        assert_eq!(decoded, message);
        assert_eq!(borsh::to_vec(&entries).unwrap(), message);
    }

    #[test]
    fn exclusions_are_validated_and_applied() {
        assert_eq!(
            validate_exclusions(&["Pyth".to_string(), "pyth".to_string()]).unwrap(),
            vec!["pyth".to_string()]
        );
        assert!(validate_exclusions(&["pyht".to_string()]).is_err());

        let config = ExchangeConfig {
            coingecko: Some("near".to_string()),
            binance: Some("NEARUSDT".to_string()),
            pyth: Some("0xabc".to_string()),
            ..Default::default()
        };
        let filtered = filter_exchange_config(&config, &["pyth".to_string()]);
        assert!(filtered.pyth.is_none());
        assert_eq!(filtered.binance.as_deref(), Some("NEARUSDT"));
        assert_eq!(configured_sources(&filtered), vec!["coingecko", "binance"]);

        assert!(is_excluded("pyth", &["pyth".to_string()]));
        assert!(!is_excluded("binance", &["pyth".to_string()]));
    }

    /// Locks the wire format the client sends: command tag, field names and defaults.
    ///
    /// The request deliberately still carries two REMOVED fields, `aliases` and `key_name`. A
    /// client that has not been updated must keep working — with its assets published under
    /// our ids and its feed signed with our key — rather than being rejected, and neither
    /// field may quietly come back to life through a future `Deserialize`.
    ///
    /// `key_name` mattered most: while it existed the caller chose which enclave key signed,
    /// so a request naming the on-chain push key had that key sign caller-supplied bytes. It
    /// is now ignored, not honoured, and there is no field left to name a key.
    #[test]
    fn request_json_parses_into_the_command() {
        use crate::types::OracleCommand;

        let request = r#"{
            "command": "get_signed_prices",
            "tokens": ["wrap.near", "eth.bridge.near"],
            "max_age_secs": 60,
            "key_name": "PROTECTED_KEY_RHEA",
            "aliases": {"eth.bridge.near": "ETH"},
            "exclude_sources": ["pyth"]
        }"#;

        match serde_json::from_str::<OracleCommand>(request).unwrap() {
            OracleCommand::GetSignedPrices {
                tokens,
                max_age_secs,
                sig_format,
                expo,
                exclude_sources,
                aggregation_method,
                min_sources_num,
            } => {
                assert_eq!(tokens, vec!["wrap.near", "eth.bridge.near"]);
                assert_eq!(max_age_secs, 60);
                // omitted fields fall back to the documented defaults
                assert_eq!(sig_format, None);
                assert_eq!(expo, None);
                assert_eq!(exclude_sources, Some(vec!["pyth".to_string()]));
                assert_eq!(aggregation_method, AggregationMethod::Median);
                assert_eq!(min_sources_num, 1);
            }
            other => panic!("wrong command parsed: {:?}", other),
        }

        // The command re-serializes without either field, so a round-trip cannot reintroduce
        // a way to name the signing key
        let reserialized = serde_json::to_value(
            serde_json::from_str::<OracleCommand>(request).unwrap(),
        )
        .unwrap();
        assert!(reserialized.get("key_name").is_none());
        assert!(reserialized.get("aliases").is_none());
    }

    /// The feed key is pinned in code and is only ever used to sign feed payloads.
    /// `main::feed_signing_key_is_pinned_and_transaction_free` covers the handler side.
    #[test]
    fn feed_signing_key_is_a_tee_generated_key() {
        assert!(FEED_SIGNING_KEY.starts_with("PROTECTED_"));
        assert_ne!(FEED_SIGNING_KEY, "PROTECTED_KEY_RHEA");
    }

    /// Aggregating nothing usable must not produce a signable number.
    ///
    /// `build_entries`/`scale_price` reject a 0.0 that reaches them, but only after it has
    /// been recorded as this asset's price; returning `None` here stops it one step earlier,
    /// where the caller can name the asset that failed.
    #[test]
    fn aggregate_reports_nothing_usable_instead_of_zero() {
        for method in [
            AggregationMethod::Median,
            AggregationMethod::Average,
            AggregationMethod::WeightedAverage,
        ] {
            assert_eq!(aggregate(&[], method), None);
            assert_eq!(aggregate(&[f64::NAN, f64::INFINITY], method), None);
            assert_eq!(aggregate(&[2.5], method), Some(2.5));
        }

        // Median is the outlier-resistant one; the other two are the same equal-weight mean
        let prices = [1.0, 1.0, 1.0, 100.0];
        assert_eq!(aggregate(&prices, AggregationMethod::Median), Some(1.0));
        assert_eq!(
            aggregate(&prices, AggregationMethod::WeightedAverage),
            aggregate(&prices, AggregationMethod::Average)
        );

        // and a 0.0 that somehow reached the payload is still refused by the scaler
        assert!(scale_price(0.0, DEFAULT_EXPO).is_err());
    }


    /// `KNOWN_SOURCES`, `filter_exchange_config` and `configured_sources` enumerate the same
    /// set in three places. When they drift the failure is silent and dangerous: a source
    /// absent from `filter_exchange_config` falls through its `_ => {}` arm, so a caller that
    /// asked to exclude it still gets it in the signed price. This pins all three together.
    #[test]
    fn every_source_is_filterable() {
        let all = all_sources_configured();

        // 1. configured_sources() sees exactly the sources KNOWN_SOURCES advertises
        let configured = configured_sources(&all);
        let mut expected = KNOWN_SOURCES.to_vec();
        let mut actual = configured.clone();
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(
            actual, expected,
            "configured_sources() and KNOWN_SOURCES disagree — a venue was added to one but \
             not the other"
        );

        // 2. every known source is accepted by validate_exclusions
        for source in KNOWN_SOURCES {
            assert!(
                validate_exclusions(&[source.to_string()]).is_ok(),
                "'{}' is in KNOWN_SOURCES but rejected by validate_exclusions",
                source
            );
        }

        // 3. and excluding it actually CLEARS its field, rather than falling through the
        //    catch-all arm and leaving the source contributing to the signed price
        for source in KNOWN_SOURCES {
            let filtered = filter_exchange_config(&all, &[source.to_string()]);
            let left = configured_sources(&filtered);
            assert!(
                !left.contains(&source),
                "excluding '{}' did not clear its ExchangeConfig field — filter_exchange_config \
                 is missing a match arm and the source still feeds the signed price",
                source
            );
            assert_eq!(
                left.len(),
                KNOWN_SOURCES.len() - 1,
                "excluding '{}' changed more than one source: {:?}",
                source,
                left
            );
        }

        // 4. excluding everything leaves nothing configured
        let none = filter_exchange_config(
            &all,
            &KNOWN_SOURCES.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        );
        assert!(configured_sources(&none).is_empty());
    }

    #[test]
    fn sig_format_defaults_to_json() {
        assert_eq!(SigFormat::parse(None).unwrap(), SigFormat::Json);
        assert_eq!(SigFormat::parse(Some("json")).unwrap(), SigFormat::Json);
        assert_eq!(SigFormat::parse(Some("borsh")).unwrap(), SigFormat::Borsh);
        assert!(SigFormat::parse(Some("protobuf")).is_err());
    }

    /// A tier selection is expressed as an exclusion list so it runs through the one exhaustive
    /// mapping of names onto config fields. These are the two tiers the scheduler actually
    /// drives, asserted end to end: what survives the filter is what gets fetched.
    #[test]
    fn tier_selection_resolves_to_the_intended_venues() {
        let all = all_sources_configured();

        // slow tier: only Pyth and Chainlink
        let cleared = resolve_source_selection(
            Some(&["pyth".to_string(), "chainlink".to_string()]),
            None,
        )
        .unwrap();
        let mut left = configured_sources(&filter_exchange_config(&all, &cleared));
        left.sort_unstable();
        assert_eq!(left, vec!["chainlink", "pyth"]);

        // fast tier: everything except them
        let cleared = resolve_source_selection(
            None,
            Some(&["pyth".to_string(), "chainlink".to_string()]),
        )
        .unwrap();
        let left = configured_sources(&filter_exchange_config(&all, &cleared));
        assert_eq!(left.len(), KNOWN_SOURCES.len() - 2);
        assert!(!left.contains(&"pyth") && !left.contains(&"chainlink"));

        // no selection at all clears nothing — the full refresh keeps its old behaviour
        assert!(resolve_source_selection(None, None).unwrap().is_empty());

        // both lists narrow twice
        let cleared = resolve_source_selection(
            Some(&["pyth".to_string(), "chainlink".to_string()]),
            Some(&["pyth".to_string()]),
        )
        .unwrap();
        let left = configured_sources(&filter_exchange_config(&all, &cleared));
        assert_eq!(left, vec!["chainlink"]);

        // and a typo is rejected rather than silently widening the tier
        let err = resolve_source_selection(Some(&["pyht".to_string()]), None).unwrap_err();
        assert!(err.contains("only_sources"), "{}", err);
        let err = resolve_source_selection(None, Some(&["pyht".to_string()])).unwrap_err();
        assert!(err.contains("exclude_sources"), "{}", err);
    }
}
