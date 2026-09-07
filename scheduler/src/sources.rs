//! Price sources for the scheduler.
//!
//! Everything here delegates to the shared `oracle-example-sources` crate, which is the same code
//! the TEE worker runs. That matters more than it looks: the scheduler's only job is to notice
//! when its own view of a price disagrees with the worker's, so any difference in *how* the two
//! sides fetch shows up as a permanent disagreement and a refresh trigger that never stops
//! firing. This file used to carry a second copy of every fetcher, and it had already drifted —
//! its Pyth staleness bound was a hardcoded 120 rather than `parsers::PYTH_MAX_AGE_SECS`, and
//! it subtracted timestamps without saturating.
//!
//! Exchange configs come from public storage (synced from the contract via DAO).

use anyhow::Result;
use oracle_example_sources::sources::r#async as shared;
use oracle_example_sources::{parsers, ExchangeConfig, SourcePrice};
use std::collections::HashMap;
use tracing::{debug, info};

/// Fetch every configured venue for the whole asset set with ONE request per venue.
pub async fn fetch_all_sources_batch(
    client: &reqwest::Client,
    configs: &HashMap<String, ExchangeConfig>,
    api_key: Option<&str>,
    pyth_api_key: Option<&str>,
) -> HashMap<String, Vec<SourcePrice>> {
    shared::fetch_all_sources_batch(client, configs, api_key, pyth_api_key).await
}

/// Median of one token's batched source prices, for comparison against the worker's stored value.
pub fn aggregate_batched(token: &str, prices: Option<&[SourcePrice]>) -> Result<f64> {
    let prices = prices.unwrap_or_default();

    if prices.is_empty() {
        anyhow::bail!("No sources available for {}", token);
    }

    for p in prices {
        debug!("{} {}: ${:.4}", p.source_name, token, p.price);
    }

    let values: Vec<f64> = prices.iter().map(|p| p.price).collect();
    // `median` returns None when no usable value is left — every source answered with a
    // non-finite number. That is a failed refresh, never a price: the 0.0 this used to return
    // would have been logged and pushed as a real quote of $0.00.
    let median = parsers::median(&values).ok_or_else(|| {
        anyhow::anyhow!(
            "No usable price for {}: all {} source(s) returned non-finite values",
            token,
            values.len()
        )
    })?;

    let source_names: Vec<&str> = prices.iter().map(|p| p.source_name.as_str()).collect();
    info!(
        "{}: ${:.6} ({} sources: {})",
        token,
        median,
        prices.len(),
        source_names.join(", ")
    );
    Ok(median)
}
