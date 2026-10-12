//! Shared price sources for oracle-example
//!
//! This crate provides price fetching logic that works with both:
//! - WASI (sync, using wasi-http-client)
//! - Scheduler (async, using reqwest)
//!
//! # Features
//! - `wasi` - Enable sync WASI client support
//! - `async` - Enable async client support (for scheduler)

pub mod parsers;
pub mod security;
pub mod sources;

pub use parsers::*;

use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;

/// Last working Chainlink RPC index (shared across calls within a single WASI run)
pub static LAST_CHAINLINK_RPC: AtomicUsize = AtomicUsize::new(0);

/// If all Chainlink RPCs failed, disable further attempts for this run
pub static CHAINLINK_DISABLED: AtomicBool = AtomicBool::new(false);

/// Price result from a source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcePrice {
    pub source_name: String,
    pub price: f64,
    pub timestamp: u64,
}

/// Why a venue produced no price in one fetch attempt.
///
/// A closed set on purpose. The reason travels to callers and into attested output, and the
/// raw error behind it may quote the request URL — which for CoinGecko carries the API key as
/// a query parameter. The text stays on stderr; only the code leaves the enclave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    /// The connection or the read exceeded its deadline
    Timeout,
    /// No connection could be established (DNS, refused, reset)
    ConnectFailed,
    /// HTTP 429
    RateLimited,
    /// Any other non-success HTTP status; the code is in `VenueFailure::status`
    HttpError,
    /// The venue answered, but not with something the parser accepts
    BadResponse,
    /// The venue answered the batch, but the asset's symbol was not in it
    PairMissing,
    /// The venue answered with data older than it allows itself (Pyth's own `publish_time`)
    StaleUpstream,
    /// The venue needs a credential the worker does not hold, so it was not requested
    NotConfigured,
    /// Nothing above matched
    Other,
}

impl FailureReason {
    /// Classify a fetch error by the text the fetchers produce.
    ///
    /// The fetchers report transport and HTTP failures as `anyhow` strings in a handful of
    /// fixed shapes (`HTTP 429`, `… timed out`, `… parse error: …`), so a lookup over those
    /// shapes is what turns them into a code. Anything unrecognised is `Other`, never the text.
    pub fn classify(error: &anyhow::Error) -> (FailureReason, Option<u16>) {
        let text = format!("{:#}", error).to_ascii_lowercase();
        if let Some(status) = http_status_in(&text) {
            return if status == 429 {
                (FailureReason::RateLimited, Some(status))
            } else {
                (FailureReason::HttpError, Some(status))
            };
        }
        if text.contains("timed out") || text.contains("timeout") {
            return (FailureReason::Timeout, None);
        }
        if text.contains("connect")
            || text.contains("dns")
            || text.contains("resolve")
            || text.contains("connection")
        {
            return (FailureReason::ConnectFailed, None);
        }
        if text.contains("parse")
            || text.contains("returned code")
            || text.contains("returned status")
            || text.contains("api error")
            || text.contains("not found")
            || text.contains("missing")
            || text.contains("unavailable")
        {
            return (FailureReason::BadResponse, None);
        }
        (FailureReason::Other, None)
    }
}

/// The status code in an `HTTP <code>` fragment, if the text has one
fn http_status_in(text: &str) -> Option<u16> {
    let at = text.find("http ")?;
    let digits: String = text[at + 5..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok().filter(|s| (100..1000).contains(s))
}

/// One venue that produced no price for one asset in a fetch attempt
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VenueFailure {
    pub source: String,
    pub reason: FailureReason,
    /// HTTP status for `RateLimited` and `HttpError`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

impl VenueFailure {
    pub fn from_error(source: &str, error: &anyhow::Error) -> Self {
        let (reason, status) = FailureReason::classify(error);
        VenueFailure {
            source: source.to_string(),
            reason,
            status,
        }
    }

    pub fn new(source: &str, reason: FailureReason) -> Self {
        VenueFailure {
            source: source.to_string(),
            reason,
            status: None,
        }
    }
}

/// What one fetch produced: the prices it observed and the venues it tried that gave none.
///
/// Both are keyed by asset. A venue absent from both for an asset was not attempted — that
/// is how a tiered refresh looks, and the caller must not mark such venues as failed.
#[derive(Debug, Clone, Default)]
pub struct FetchOutcome {
    pub prices: std::collections::HashMap<String, Vec<SourcePrice>>,
    pub failures: std::collections::HashMap<String, Vec<VenueFailure>>,
}

impl FetchOutcome {
    pub fn fail(&mut self, token: &str, failure: VenueFailure) {
        self.failures
            .entry(token.to_string())
            .or_default()
            .push(failure);
    }

    pub fn prices_of(&self, token: &str) -> &[SourcePrice] {
        self.prices.get(token).map(Vec::as_slice).unwrap_or_default()
    }

    pub fn failures_of(&self, token: &str) -> &[VenueFailure] {
        self.failures.get(token).map(Vec::as_slice).unwrap_or_default()
    }
}

/// What a single-asset fetch produced
#[derive(Debug, Clone, Default)]
pub struct SingleFetch {
    pub prices: Vec<SourcePrice>,
    pub failures: Vec<VenueFailure>,
}

/// Per-asset exchange configuration.
/// Stored as opaque JSON in the oracle contract, parsed by WASI/scheduler.
/// Unknown fields are silently ignored (allows adding new exchanges without code changes).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExchangeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coingecko: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binance_us: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binance_alpha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pyth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chainlink: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub huobi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kucoin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cryptocom: Option<String>,
    /// Kraken pair in its CANONICAL spelling (`XXBTZUSD`, `XETHZUSD`, `NEARUSD`).
    /// Kraken answers with the canonical name whatever alias you ask for, so storing the
    /// canonical form here is what makes the batch lookup an exact match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kraken: Option<String>,
    /// Coinbase Exchange product id (`NEAR-USD`) — quoted in real fiat USD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coinbase: Option<String>,
    /// Bitstamp pair WITH the slash (`BTC/USD`), as the all-ticker endpoint reports it.
    /// The single-pair URL form (`btcusd`) is derived from this, not stored separately.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitstamp: Option<String>,
    /// OKX instrument id (`NEAR-USDT`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub okx: Option<String>,
    /// Bitget symbol (`NEARUSDT`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitget: Option<String>,
    /// MEXC symbol (`NEARUSDT`) — this endpoint quotes USDT pairs only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mexc: Option<String>,
    #[serde(default)]
    pub stablecoin: bool,
    /// Token decimals (used by UI, not by price fetching)
    #[serde(default)]
    pub decimals: Option<u8>,
}

impl ExchangeConfig {
    /// Get Pyth ID with 0x prefix stripped
    pub fn pyth_id(&self) -> Option<&str> {
        self.pyth
            .as_deref()
            .map(|s| s.strip_prefix("0x").unwrap_or(s))
    }

    /// The venues this asset has configured, by the names used everywhere else.
    ///
    /// Lives next to the struct so there is exactly one mapping from a source name onto a
    /// field. The worker's exclusion filter and the scheduler's tier selection both consume it;
    /// a venue added to the struct but forgotten here would look unconfigured to both.
    pub fn configured_sources(&self) -> Vec<&'static str> {
        let mut sources = Vec::new();
        if self.coingecko.is_some() {
            sources.push("coingecko");
        }
        if self.binance.is_some() {
            sources.push("binance");
        }
        if self.binance_us.is_some() {
            sources.push("binance_us");
        }
        if self.binance_alpha.is_some() {
            sources.push("binance_alpha");
        }
        if self.pyth.is_some() {
            sources.push("pyth");
        }
        if self.chainlink.is_some() {
            sources.push("chainlink");
        }
        if self.huobi.is_some() {
            sources.push("huobi");
        }
        if self.kucoin.is_some() {
            sources.push("kucoin");
        }
        if self.gate.is_some() {
            sources.push("gate");
        }
        if self.cryptocom.is_some() {
            sources.push("cryptocom");
        }
        if self.kraken.is_some() {
            sources.push("kraken");
        }
        if self.coinbase.is_some() {
            sources.push("coinbase");
        }
        if self.bitstamp.is_some() {
            sources.push("bitstamp");
        }
        if self.okx.is_some() {
            sources.push("okx");
        }
        if self.bitget.is_some() {
            sources.push("bitget");
        }
        if self.mexc.is_some() {
            sources.push("mexc");
        }
        sources
    }

    /// Whether this asset configures any of the named venues (case-insensitive).
    /// Used to decide which assets participate in a tier.
    pub fn configures_any(&self, names: &[String]) -> bool {
        self.configured_sources()
            .iter()
            .any(|source| names.iter().any(|n| n.eq_ignore_ascii_case(source)))
    }

    /// A copy with the named venues cleared.
    ///
    /// Fetching is driven purely by which fields are `Some`, so clearing a field removes that
    /// venue from the fetch. A name this does not recognise leaves the config untouched, which
    /// would silently keep a source the caller believes is gone — callers must validate names
    /// first (the worker rejects unknown ones outright).
    pub fn without_sources(&self, names: &[String]) -> Self {
        let mut filtered = self.clone();
        for name in names {
            match name.to_ascii_lowercase().as_str() {
                "coingecko" => filtered.coingecko = None,
                "binance" => filtered.binance = None,
                "binance_us" => filtered.binance_us = None,
                "binance_alpha" => filtered.binance_alpha = None,
                "pyth" => filtered.pyth = None,
                "chainlink" => filtered.chainlink = None,
                "huobi" => filtered.huobi = None,
                "kucoin" => filtered.kucoin = None,
                "gate" => filtered.gate = None,
                "cryptocom" => filtered.cryptocom = None,
                "kraken" => filtered.kraken = None,
                "coinbase" => filtered.coinbase = None,
                "bitstamp" => filtered.bitstamp = None,
                "okx" => filtered.okx = None,
                "bitget" => filtered.bitget = None,
                "mexc" => filtered.mexc = None,
                _ => {}
            }
        }
        filtered
    }
}

/// Custom source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSourceConfig {
    pub url: String,
    pub json_path: String,
    #[serde(default = "default_value_type")]
    pub value_type: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

fn default_value_type() -> String {
    "number".to_string()
}

fn default_method() -> String {
    "GET".to_string()
}

/// HTTP response for parsing
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> anyhow::Result<T> {
        Ok(serde_json::from_slice(&self.body)?)
    }
}

#[cfg(test)]
mod failure_tests {
    use super::*;

    /// A fetch error leaves the enclave as a code. The text is where a URL would be quoted,
    /// and the CoinGecko URL carries the API key, so the text itself must never serialize.
    #[test]
    fn errors_become_codes_and_never_text() {
        let cases = [
            ("HTTP 429", FailureReason::RateLimited, Some(429)),
            ("HTTP 503", FailureReason::HttpError, Some(503)),
            ("https://x/api?x_cg_pro_api_key=CG-secret: HTTP 403", FailureReason::HttpError, Some(403)),
            ("request timed out after 3s", FailureReason::Timeout, None),
            ("connection refused", FailureReason::ConnectFailed, None),
            ("CoinGecko batch response parse error: expected value", FailureReason::BadResponse, None),
            ("KuCoin batch returned code '200001'", FailureReason::BadResponse, None),
            ("Kraken batch API error: EQuery:Unknown asset pair", FailureReason::BadResponse, None),
            ("something nobody foresaw", FailureReason::Other, None),
        ];
        for (text, reason, status) in cases {
            assert_eq!(
                FailureReason::classify(&anyhow::anyhow!(text)),
                (reason, status),
                "{text}"
            );
        }

        let failure = VenueFailure::from_error(
            "coingecko",
            &anyhow::anyhow!(
                "https://pro-api.coingecko.com/api/v3/simple/price?ids=near&x_cg_pro_api_key=CG-secret-123: HTTP 401"
            ),
        );
        let json = serde_json::to_string(&failure).unwrap();
        assert!(!json.contains("secret"), "{json}");
        assert_eq!(json, r#"{"source":"coingecko","reason":"http_error","status":401}"#);

        // a status outside the HTTP range is not a status
        assert_eq!(
            FailureReason::classify(&anyhow::anyhow!("http 99999 nonsense")),
            (FailureReason::Other, None)
        );
    }
}
