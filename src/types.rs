use oracle_example_sources::FailureReason;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Default max age for cached prices (2 minutes)
pub const DEFAULT_MAX_AGE_SECS: u64 = 120;

/// Window the stored `price` field is aggregated over.
///
/// The record keeps every observation, but its headline price has to commit to one window, and
/// this is it — the same 2 minutes a caller gets by default, and the window the slow tier
/// (Pyth, Chainlink) is sized to stay inside. Consumers who need tighter freshness do not
/// depend on this: they pass their own `max_age_secs` and the aggregate is rebuilt for them.
pub const CANONICAL_WINDOW_SECS: u64 = DEFAULT_MAX_AGE_SECS;

/// How long an untouched source survives in the stored record.
///
/// Generous on purpose: every read filters by age, so keeping an entry costs nothing, while
/// dropping one too early would make a merely-late tier look like a failed one. The ceiling
/// exists so a permanently dead venue eventually stops appearing as a source at all.
pub const SOURCE_RETENTION_SECS: u64 = 900;

// Price deviation threshold for alerts (5%)
pub const PRICE_DEVIATION_ALERT_THRESHOLD: f64 = 5.0;

// Default minimum number of sources required
pub const DEFAULT_MIN_SOURCES: u8 = 1;

/// Aggregation method for combining prices from multiple sources
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AggregationMethod {
    /// Arithmetic mean of all prices
    Average,
    /// Median value (default, more robust against outliers)
    #[default]
    Median,
    /// Equal-weight mean — an exact ALIAS for `Average`, kept because the value
    /// `"weighted_average"` is already accepted by live callers.
    ///
    /// It gives NO extra outlier resistance despite the name: one bad venue moves it by 1/n
    /// of its error, exactly as it moves `Average`. Choose `Median` for outlier resistance.
    /// See `parsers::weighted_average` for why no real weighting is invented here.
    WeightedAverage,
}

impl AggregationMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            AggregationMethod::Average => "average",
            AggregationMethod::Median => "median",
            AggregationMethod::WeightedAverage => "weighted_average",
        }
    }
}

fn default_aggregation() -> AggregationMethod {
    AggregationMethod::default()
}

fn default_min_sources() -> u8 {
    DEFAULT_MIN_SOURCES
}

/// Command enum for routing different request types
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum OracleCommand {
    /// Update prices in public storage (triggered by scheduler)
    /// WASI fetches prices from sources and stores them
    ///
    /// A refresh may cover only part of the configured venues — see `only_sources` /
    /// `exclude_sources`. What it fetches is merged into the stored record rather than
    /// replacing it, so tiers running at different cadences accumulate into one breakdown.
    UpdatePrices {
        /// Tokens to update (e.g., ["bitcoin", "ethereum"])
        tokens: Vec<String>,
        /// Refresh only these sources, leaving every other venue in the record untouched.
        /// This is how the slow tier is driven: `["pyth", "chainlink"]` on a 2-minute cadence,
        /// while the cheap all-ticker venues refresh every few seconds.
        #[serde(default)]
        only_sources: Option<Vec<String>>,
        /// Refresh every source except these — the fast tier's half of the same split.
        /// Combined with `only_sources` it narrows twice.
        #[serde(default)]
        exclude_sources: Option<Vec<String>>,
        /// Whether to also call report_prices on the contract
        #[serde(default)]
        update_contract: bool,
        /// Contract ID to update (if update_contract is true)
        #[serde(skip_serializing_if = "Option::is_none")]
        contract_id: Option<String>,
        /// How to aggregate prices from multiple sources (default: median)
        #[serde(default = "default_aggregation")]
        aggregation_method: AggregationMethod,
        /// Minimum number of sources required (default: 1)
        #[serde(default = "default_min_sources")]
        min_sources_num: u8,
        /// Per-asset oracle key mapping: asset_id -> PROTECTED_ env var name
        /// E.g., {"wrap.near": "PROTECTED_ORACLE_KEY_A", "usdt.tether-token.near": "PROTECTED_ORACLE_KEY_B"}
        /// Assets not in this map use the default PROTECTED_ORACLE_KEY
        /// If omitted, all assets use PROTECTED_ORACLE_KEY
        #[serde(default)]
        oracle_keys: Option<std::collections::HashMap<String, String>>,
    },

    /// Get prices (for blockchain requests via yield/resume)
    /// Returns cached prices if fresh, otherwise fetches new ones
    GetPrices {
        /// Whitelisted tokens to get prices for
        tokens: Vec<String>,
        /// Maximum age of cached price in seconds (default: 120)
        #[serde(default = "default_max_age")]
        max_age_secs: u64,
        /// How to aggregate prices from multiple sources (default: median)
        #[serde(default = "default_aggregation")]
        aggregation_method: AggregationMethod,
        /// Minimum number of sources required (default: 1)
        #[serde(default = "default_min_sources")]
        min_sources_num: u8,
    },

    /// Get prices signed in-enclave with an Ed25519 key (pull model, no gas on our side)
    /// Consumers verify the signature over the exact bytes of the returned payload,
    /// off-chain today and on-chain later. Freshness follows get_prices semantics.
    ///
    /// There is deliberately NO field naming the signing key: it is pinned in code as
    /// `signed_prices::FEED_SIGNING_KEY`. A caller that still sends the old `key_name` is
    /// parsed normally and the field is ignored (serde skips unknown fields), so an
    /// un-updated client keeps working — with our key, not one of its choosing.
    GetSignedPrices {
        /// Asset ids to price — every one must resolve, otherwise the request fails
        tokens: Vec<String>,
        /// Maximum age of cached price in seconds (default: 120)
        #[serde(default = "default_max_age")]
        max_age_secs: u64,
        /// Payload format: "json" (default) or "borsh"
        #[serde(default)]
        sig_format: Option<String>,
        /// Price exponent: real price = price * 10^expo (default: -8)
        #[serde(default)]
        expo: Option<i32>,
        /// Sources to exclude from this request (e.g. ["pyth"] for a Pyth-independent leg)
        /// Names must match SourcePrice.source_name; unknown names are rejected
        #[serde(default)]
        exclude_sources: Option<Vec<String>>,
        /// How to aggregate prices from multiple sources (default: median)
        #[serde(default = "default_aggregation")]
        aggregation_method: AggregationMethod,
        /// Minimum number of sources required (default: 1)
        #[serde(default = "default_min_sources")]
        min_sources_num: u8,
        /// A venue whose most recent refresh attempt failed does not vote, as long as the
        /// venues left inside the window still satisfy `min_sources_num`. Otherwise it votes
        /// as usual, so an asset with a single venue stays priced while that venue is down.
        /// Off by default: the stamp then stays the oldest venue inside the window, failed
        /// or not.
        #[serde(default)]
        skip_failed_sources: bool,
        /// An asset that cannot be priced is left out of the signed payload and listed under
        /// `skipped` with a reason, instead of failing the whole request. No asset priced at
        /// all is still an error. Off by default: the feed is then all-or-nothing.
        #[serde(default)]
        allow_partial: bool,
        /// Report every configured venue per asset — used, skipped, failed, outside the
        /// window, excluded or missing — under `sources`, outside the signed payload.
        #[serde(default)]
        report_sources: bool,
    },

    /// Force update prices - anyone can call if they pay for execution
    /// Always fetches fresh prices from sources, ignoring cache
    ForceUpdate {
        /// Tokens to force update
        tokens: Vec<String>,
        /// How to aggregate prices from multiple sources (default: median)
        #[serde(default = "default_aggregation")]
        aggregation_method: AggregationMethod,
        /// Minimum number of sources required (default: 1)
        #[serde(default = "default_min_sources")]
        min_sources_num: u8,
    },

    /// Fetch price for any token from external API (not whitelisted)
    /// Returns price directly without storing in public storage
    FetchExternal {
        /// Token identifier (depends on source):
        /// - CoinGecko: "bitcoin", "ethereum", "near"
        /// - Binance: "BTCUSDT", "ETHUSDT", "NEARUSDT"
        /// - Pyth: price feed ID
        token_id: String,
        /// Which API source to use
        source: ExternalPriceSource,
    },

    /// Fetch custom data from external sources (for custom_call)
    /// Used for any data: prices, weather, game data, API responses, etc.
    FetchCustomData {
        /// List of data requests
        requests: Vec<CustomDataRequest>,
    },

    /// Test Telegram alert delivery
    TestTelegram {
        /// Optional custom message to send
        #[serde(default)]
        message: Option<String>,
    },

    /// Get public key and implicit account ID for a PROTECTED_ key
    /// Used by DAO to verify TEE worker identity before registering as oracle
    GetPublicKey {
        /// Environment variable name (default: "PROTECTED_ORACLE_KEY")
        #[serde(default = "default_oracle_key_name")]
        key_name: String,
    },

    /// Sync asset exchange configs to public storage.
    /// Called by the oracle contract after DAO config updates.
    /// WASI stores the full config map in public storage key "config:assets".
    SyncAssetConfigs {
        /// Full config map: asset_id -> exchange config JSON string
        configs: std::collections::HashMap<String, String>,
    },
}

/// External price source for non-whitelisted tokens
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExternalPriceSource {
    CoinGecko,
    Binance,
    Pyth,
    /// Custom source - fetch from any URL with JSON path extraction
    /// The API_KEY secret is attached only for allowlisted providers (see
    /// `oracle_example_sources::security::may_receive_api_key`); any other source must carry its
    /// own credential in `headers`
    Custom(CustomSourceConfig),
}

impl std::fmt::Display for ExternalPriceSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExternalPriceSource::CoinGecko => write!(f, "coingecko"),
            ExternalPriceSource::Binance => write!(f, "binance"),
            ExternalPriceSource::Pyth => write!(f, "pyth"),
            ExternalPriceSource::Custom(_) => write!(f, "custom"),
        }
    }
}

/// Response for external price fetch (single token from single source)
#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalPriceResponse {
    pub success: bool,
    pub token_id: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Warning about single-source price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

fn default_max_age() -> u64 {
    DEFAULT_MAX_AGE_SECS
}

/// Response for new command format
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResponse {
    pub success: bool,
    pub prices: Vec<PriceResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Single price result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceResult {
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_cache: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Every configured venue and what it did for this answer; only built when a caller
    /// asked for it (`report_sources`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_report: Option<Vec<SourceReport>>,
}

/// What one configured venue did for one asset in one answer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceReport {
    pub name: String,
    pub status: SourceStatus,
    /// When the venue was last observed (seconds); absent when it never was
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<u64>,
    /// Age of that observation at the moment the answer was built
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_secs: Option<u64>,
    /// When the venue's most recent attempt failed, if it did
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<FailureReason>,
    /// HTTP status behind `rate_limited` / `http_error`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
}

/// Whether a venue took part in an answer, and if not, why
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    /// Its observation is inside the window and voted
    Used,
    /// Inside the window, but its last attempt failed and the caller asked to skip such venues
    SkippedFailed,
    /// Inside the window, yet no answer was built from this record: the asset failed
    Unused,
    /// Attempted in this very call and produced no price
    Failed,
    /// Has an observation, older than the caller's window
    OutsideWindow,
    /// Left out by the caller's `exclude_sources`
    Excluded,
    /// Configured, but no observation is held for it
    Missing,
}

/// An asset left out of a partial signed answer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkippedAsset {
    pub asset_id: String,
    pub reason: SkipReason,
    /// For `price_too_old`: how old the best available price was
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_secs: Option<u64>,
}

/// Why an asset was left out of a partial signed answer
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// Not an asset this oracle prices
    UnknownAsset,
    /// Fewer venues inside the window than `min_sources_num`, and no live fetch could help
    NotEnoughSources,
    /// The only price available is older than the caller's window
    PriceTooOld,
    /// The cache could not serve the window, and the refetch it triggered failed; whatever
    /// fallback existed is one the caller's own rules refuse
    FetchFailed,
    /// No price at all, for any other reason
    NoPrice,
}

/// Custom source configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomSourceConfig {
    /// HTTP URL to fetch data from
    pub url: String,

    /// JSON path to extract value (dot notation, e.g. "data.price" or "rates.USD")
    pub json_path: String,

    /// Type of value to extract: "number", "string", "boolean" (default: "number")
    #[serde(default = "default_value_type")]
    pub value_type: String,

    /// Optional HTTP method (default: GET)
    #[serde(default = "default_http_method")]
    pub method: String,

    /// Optional HTTP headers as key-value pairs
    #[serde(default)]
    pub headers: Vec<(String, String)>,

    /// Optional JSON body for POST/PUT requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

fn default_oracle_key_name() -> String {
    "PROTECTED_ORACLE_KEY".to_string()
}

fn default_value_type() -> String {
    "number".to_string()
}

fn default_http_method() -> String {
    "GET".to_string()
}

/// Request for custom data (matches contract's CustomDataRequest)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomDataRequest {
    /// Identifier for the data in callback
    pub id: String,
    /// Token/query identifier for the source (required for coingecko/binance/pyth, optional for custom)
    #[serde(default)]
    pub token_id: String,
    /// Data source to query
    pub source: ExternalPriceSource,
}

/// Response for fetch_custom_data command
#[derive(Debug, Serialize, Deserialize)]
pub struct CustomDataResponse {
    pub success: bool,
    pub results: Vec<CustomDataResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Single custom data result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomDataResult {
    pub id: String,
    /// Value as JSON (number, string, etc. based on value_type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for sync_asset_configs command
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResponse {
    pub success: bool,
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for get_signed_prices command
///
/// The signature covers the exact bytes of `payload`:
/// - sig_format "json": the UTF-8 bytes of the `payload` string
/// - sig_format "borsh": the raw bytes of `base64_decode(payload)`
///
/// Verifiers MUST NOT re-serialize the payload before checking the signature.
/// All fields are always present (null when unset) so the envelope is stable for clients.
#[derive(Debug, Serialize, Deserialize)]
pub struct SignedPricesResponse {
    pub success: bool,
    /// Signed payload: JSON object string, or base64 borsh blob when sig_format="borsh"
    pub payload: Option<String>,
    /// Base64 of the 64-byte Ed25519 signature
    pub signature: Option<String>,
    /// Signer public key in NEAR format (ed25519:base58...)
    pub public_key: Option<String>,
    /// Payload format actually used ("json" or "borsh")
    pub sig_format: String,
    pub error: Option<String>,
    /// Assets left out of the payload; a list only when the caller passed `allow_partial`
    pub skipped: Option<Vec<SkippedAsset>>,
    /// Per-asset venue report; a map only when the caller passed `report_sources`.
    /// Outside the signature, inside the attested output.
    pub sources: Option<BTreeMap<String, Vec<SourceReport>>>,
}

/// Response for get_public_key command
#[derive(Debug, Serialize, Deserialize)]
pub struct PublicKeyResponse {
    pub success: bool,
    /// NEAR implicit account ID (hex-encoded ed25519 public key, 64 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit_account_id: Option<String>,
    /// Public key in NEAR format (ed25519:base58...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
