//! Types for storing prices in public storage

use oracle_example_sources::{FailureReason, VenueFailure};
use serde::{Deserialize, Serialize};

/// Price stored in public storage
/// Key format: "price:{token_id}" (e.g., "price:bitcoin", "price:wrap.near")
///
/// The record is a *merged* view: sources are refreshed in tiers at different cadences (cheap
/// all-ticker venues often, Pyth and Chainlink rarely), so `sources` holds the most recent
/// observation of each venue rather than a set captured in one pass. Each entry carries its own
/// observation time, and consumers pick the freshness window they need — see `sources_within`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPrice {
    /// Aggregated price value in USD, over the sources inside `CANONICAL_WINDOW_SECS`
    pub price: f64,

    /// Unix timestamp when this record was last written (seconds).
    ///
    /// This is NOT the age of the price: a partial refresh touches the record while leaving
    /// most sources untouched. The honest staleness bound for a given window is the oldest
    /// source inside it — `oldest_timestamp(&record.sources_within(..))`.
    pub timestamp: u64,

    /// Information about sources that contributed to this price
    pub sources: Vec<SourceInfo>,

    /// Aggregation method used ("average", "median", "weighted_avg")
    pub aggregation_method: String,

    /// Unix timestamp of last successful report_prices to contract (seconds)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_contract_report: Option<u64>,

    /// Venues whose most recent attempt produced no price, with when and why.
    ///
    /// Kept apart from `sources`: a venue can fail before it ever left an observation to
    /// retain, and the source report must still show it. A fresh observation clears the
    /// entry; a venue the refresh did not attempt is never marked, so a slow-tier pass does
    /// not mark the fast venues it skipped. Absent from records written before failures were
    /// tracked, which reads as "nothing known to have failed".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<SourceFailure>,
}

/// One venue's most recent failed attempt, as remembered in the record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceFailure {
    pub name: String,
    /// When the attempt failed (seconds)
    pub at: u64,
    pub reason: FailureReason,
    /// HTTP status for `rate_limited` and `http_error`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

/// Information about a single price source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Source name (e.g., "coingecko", "binance", "pyth")
    pub name: String,

    /// Price from this source
    pub price: f64,

    /// When this price was observed (seconds).
    ///
    /// For venues that publish their own timestamp (Pyth) this is that timestamp; for the rest
    /// — most ticker endpoints report none — it is the moment the enclave fetched it. Optional
    /// only for records written before tiered refresh existed; `effective_timestamp` resolves
    /// those against the record's own write time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

impl SourceInfo {
    /// Observation time, falling back to the record's write time for legacy entries.
    pub fn effective_timestamp(&self, record_timestamp: u64) -> u64 {
        self.timestamp.unwrap_or(record_timestamp)
    }
}

impl StoredPrice {
    /// Create a new StoredPrice
    pub fn new(
        price: f64,
        timestamp: u64,
        sources: Vec<SourceInfo>,
        aggregation_method: &str,
    ) -> Self {
        Self {
            price,
            timestamp,
            sources,
            aggregation_method: aggregation_method.to_string(),
            last_contract_report: None,
            failures: Vec::new(),
        }
    }

    /// The remembered failure of a venue, if its most recent attempt produced no price
    pub fn failure_of(&self, name: &str) -> Option<&SourceFailure> {
        self.failures.iter().find(|f| f.name == name)
    }

    /// The sources observed no longer than `max_age_secs` before `now`.
    ///
    /// This is the primitive the whole freshness model rests on: a caller asking for prices no
    /// older than 40 seconds gets an aggregate over exactly the venues seen within 40 seconds,
    /// and the slow tier (Pyth, Chainlink) simply does not participate. Widening the window
    /// lets them back in. Nothing is recomputed or refetched here — the record already holds
    /// every observation with its own time.
    pub fn sources_within(&self, now: u64, max_age_secs: u64) -> Vec<&SourceInfo> {
        self.sources
            .iter()
            .filter(|s| {
                now.saturating_sub(s.effective_timestamp(self.timestamp)) <= max_age_secs
            })
            .collect()
    }

    /// Oldest observation among the given sources — the honest staleness bound of an aggregate
    /// built from them. `None` for an empty set.
    pub fn oldest_timestamp(&self, sources: &[&SourceInfo]) -> Option<u64> {
        sources
            .iter()
            .map(|s| s.effective_timestamp(self.timestamp))
            .min()
    }

    /// Get storage key for a token
    pub fn storage_key(token: &str) -> String {
        format!("price:{}", token)
    }
}

/// Merge a partial refresh into a previously stored per-source breakdown.
///
/// A tiered refresh only fetches some venues, so the ones it did not touch must survive rather
/// than vanish and reappear — a source that blinks out of the set moves the median without the
/// market moving, which is exactly the artefact a lending protocol must not see.
///
/// Rules: a fresh observation supersedes the previous one for the same venue; an untouched
/// venue is kept until it exceeds `retention_secs`, after which a permanently dead endpoint
/// stops lingering as a phantom source. Results are ordered newest first, which is also the
/// order the dashboard renders.
///
/// Retention is deliberately generous compared to any read window: keeping an entry costs
/// nothing because every read filters by age anyway, while dropping one too early would make a
/// merely-late tier look like a failed one.
pub fn merge_source_entries(
    previous: &[SourceInfo],
    previous_record_timestamp: u64,
    fresh: &[SourceInfo],
    now: u64,
    retention_secs: u64,
) -> Vec<SourceInfo> {
    let mut merged: Vec<SourceInfo> = fresh.to_vec();

    for entry in previous {
        if fresh.iter().any(|f| f.name == entry.name) {
            continue;
        }
        let observed = entry.effective_timestamp(previous_record_timestamp);
        if now.saturating_sub(observed) > retention_secs {
            continue;
        }
        // Legacy entries carry no timestamp of their own; pin the inherited one now, before
        // the record's write time moves and makes them look fresher than they are.
        merged.push(SourceInfo {
            name: entry.name.clone(),
            price: entry.price,
            timestamp: Some(observed),
        });
    }

    merged.sort_by(|a, b| {
        b.timestamp
            .unwrap_or(0)
            .cmp(&a.timestamp.unwrap_or(0))
            .then_with(|| a.name.cmp(&b.name))
    });
    merged
}

/// Carry the failure marks across a refresh.
///
/// A venue observed fresh is cleared — the attempt succeeded. A venue this refresh attempted
/// and got nothing from is marked with `now`, replacing any older mark. A venue the refresh did
/// not attempt keeps whatever mark it had, until the mark is older than `retention_secs`: the
/// same ceiling that drops its retained observation, so a dead venue stops appearing at all.
///
/// Ordered by name so the record is stable to diff.
pub fn merge_failures(
    previous: &[SourceFailure],
    fresh: &[SourceInfo],
    failed: &[VenueFailure],
    now: u64,
    retention_secs: u64,
) -> Vec<SourceFailure> {
    let mut merged: Vec<SourceFailure> = previous
        .iter()
        .filter(|mark| !fresh.iter().any(|f| f.name == mark.name))
        .filter(|mark| !failed.iter().any(|f| f.source == mark.name))
        .filter(|mark| now.saturating_sub(mark.at) <= retention_secs)
        .cloned()
        .collect();
    for failure in failed {
        merged.push(SourceFailure {
            name: failure.source.clone(),
            at: now,
            reason: failure.reason,
            status: failure.status,
        });
    }
    merged.sort_by(|a, b| a.name.cmp(&b.name));
    merged.dedup_by(|a, b| a.name == b.name);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(name: &str, price: f64, timestamp: u64) -> SourceInfo {
        SourceInfo {
            name: name.to_string(),
            price,
            timestamp: Some(timestamp),
        }
    }

    fn mark(name: &str, at: u64, reason: FailureReason) -> SourceFailure {
        SourceFailure {
            name: name.to_string(),
            at,
            reason,
            status: None,
        }
    }

    /// A mark follows attempts, not time: cleared by a fresh observation, re-stamped by a new
    /// failure, kept untouched by a refresh that did not try the venue, and gone with retention.
    #[test]
    fn failure_marks_follow_attempts_not_time() {
        let now = 10_000;
        let previous = vec![
            mark("coingecko", now - 30, FailureReason::RateLimited),
            mark("okx", now - 30, FailureReason::Timeout),
            mark("dead", now - 2_000, FailureReason::Other),
        ];
        let fresh = vec![source("okx", 1.0, now)]; // okx answered this time
        let failed = vec![
            VenueFailure::new("kraken", FailureReason::ConnectFailed),
            VenueFailure {
                source: "coingecko".to_string(),
                reason: FailureReason::HttpError,
                status: Some(503),
            },
        ];

        let merged = merge_failures(&previous, &fresh, &failed, now, 900);
        let names: Vec<&str> = merged.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["coingecko", "kraken"]);
        assert_eq!(merged[0].at, now);
        assert_eq!(merged[0].reason, FailureReason::HttpError);
        assert_eq!(merged[0].status, Some(503));
        assert_eq!(merged[1].at, now);

        // a venue neither observed nor attempted keeps its mark: the slow tier's venues are
        // not judged by a fast refresh that never asked them
        let kept = merge_failures(&[mark("pyth", now - 100, FailureReason::Timeout)], &fresh, &[], now, 900);
        assert_eq!(kept, vec![mark("pyth", now - 100, FailureReason::Timeout)]);
    }

    /// A record written before failures were tracked reads as "nothing known to have failed",
    /// and a record without failures writes no `failures` key — older readers see what they saw.
    #[test]
    fn failures_are_optional_on_the_wire() {
        let legacy = r#"{"price":1.0,"timestamp":10,"sources":[],"aggregation_method":"median"}"#;
        let parsed: StoredPrice = serde_json::from_str(legacy).unwrap();
        assert!(parsed.failures.is_empty());
        assert!(parsed.failure_of("okx").is_none());
        assert!(!serde_json::to_string(&parsed).unwrap().contains("failures"));

        let mut marked = parsed.clone();
        marked.failures = vec![mark("okx", 9, FailureReason::Timeout)];
        let json = serde_json::to_string(&marked).unwrap();
        assert!(json.contains(r#""failures":[{"name":"okx","at":9,"reason":"timeout"}]"#), "{json}");
        assert_eq!(serde_json::from_str::<StoredPrice>(&json).unwrap().failure_of("okx"), marked.failures.first());
    }

    fn record(timestamp: u64, sources: Vec<SourceInfo>) -> StoredPrice {
        StoredPrice::new(1.0, timestamp, sources, "median")
    }

    /// The whole point of tiering: a refresh that skips Pyth and Chainlink must not delete them.
    /// Losing them would shrink the source set every fast cycle and restore it every slow one,
    /// which moves the median without the market moving.
    #[test]
    fn a_partial_refresh_keeps_the_venues_it_did_not_fetch() {
        let previous = vec![
            source("binance", 100.0, 1_000),
            source("pyth", 101.0, 1_000),
            source("chainlink", 99.0, 1_000),
        ];
        let fresh = vec![source("binance", 105.0, 1_060)];

        let merged = merge_source_entries(&previous, 1_000, &fresh, 1_060, 900);
        let names: Vec<&str> = merged.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"pyth") && names.contains(&"chainlink"));

        // the fetched venue is superseded, not duplicated
        let binance: Vec<&SourceInfo> = merged.iter().filter(|s| s.name == "binance").collect();
        assert_eq!(binance.len(), 1);
        assert_eq!(binance[0].price, 105.0);
        assert_eq!(binance[0].timestamp, Some(1_060));
    }

    /// A venue that stops answering has to leave eventually, or its last price keeps voting
    /// forever inside any window wide enough to reach it.
    #[test]
    fn a_venue_that_stops_answering_ages_out() {
        let previous = vec![source("binance", 100.0, 1_000), source("dead", 50.0, 1_000)];
        let fresh = vec![source("binance", 105.0, 3_000)];

        let merged = merge_source_entries(&previous, 1_000, &fresh, 3_000, 900);
        let names: Vec<&str> = merged.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["binance"]);
    }

    /// Records written before tiered refresh have no per-source timestamps. Inheriting the
    /// record's write time is the only honest reading — and it has to be pinned at merge time,
    /// because the new record's write time is `now` and would make those entries look current.
    #[test]
    fn legacy_entries_without_a_timestamp_keep_their_inherited_age() {
        let previous = vec![SourceInfo {
            name: "kraken".to_string(),
            price: 100.0,
            timestamp: None,
        }];
        let fresh = vec![source("binance", 105.0, 1_100)];

        let merged = merge_source_entries(&previous, 1_000, &fresh, 1_100, 900);
        let kraken = merged.iter().find(|s| s.name == "kraken").unwrap();
        assert_eq!(kraken.timestamp, Some(1_000));

        // and that age is what the window sees, not the record's new write time
        let stored = record(1_100, merged);
        assert_eq!(stored.sources_within(1_100, 50).len(), 1);
        assert_eq!(stored.sources_within(1_100, 150).len(), 2);
    }

    /// "Prices no older than 40 seconds" has to mean the sources, not the record: a record
    /// written a second ago can consist entirely of two-minute-old observations.
    #[test]
    fn the_window_filters_sources_not_the_record() {
        let stored = record(
            1_200,
            vec![
                source("mexc", 100.0, 1_195),
                source("okx", 101.0, 1_190),
                source("pyth", 99.0, 1_100),
                source("chainlink", 98.0, 1_090),
            ],
        );

        let tight = stored.sources_within(1_200, 40);
        assert_eq!(tight.len(), 2);
        assert_eq!(stored.oldest_timestamp(&tight), Some(1_190));

        let wide = stored.sources_within(1_200, 120);
        assert_eq!(wide.len(), 4);
        assert_eq!(stored.oldest_timestamp(&wide), Some(1_090));

        assert!(stored.sources_within(1_200, 1).is_empty());
        assert_eq!(stored.oldest_timestamp(&[]), None);
    }
}
