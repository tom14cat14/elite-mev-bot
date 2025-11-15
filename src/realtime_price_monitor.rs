use anyhow::Result;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use solana_entry::entry::Entry;
use solana_stream_sdk::{CommitmentLevel, ShredstreamClient};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::dex_parser::DexSwapParser;
use crate::token_decimal_cache::TokenDecimalCache;
use crate::volume_tracker::VolumeTracker;

/// Price information for a token on a specific pool
#[derive(Debug, Clone)]
pub struct TokenPrice {
    pub token_mint: String,
    pub dex: String,
    pub pool_address: String,
    pub price_sol: f64,
    pub last_update: DateTime<Utc>,
    pub volume_24h: f64,
    pub swap_count_24h: usize,
}

/// Internal price data with volume tracking
#[derive(Debug, Clone)]
struct PriceData {
    price: TokenPrice,
    volume_tracker: VolumeTracker,
}

/// Real-time price monitor using ShredStream
pub struct RealtimePriceMonitor {
    endpoint: String,
    rpc_url: String,
    price_cache: Arc<RwLock<HashMap<String, PriceData>>>,
}

impl RealtimePriceMonitor {
    pub fn new(endpoint: String, rpc_url: String) -> Self {
        Self {
            endpoint,
            rpc_url,
            price_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get filtered prices with 3-layer quality control
    pub async fn get_filtered_prices(&self) -> Vec<TokenPrice> {
        let cache = self.price_cache.read().await;
        let now = Utc::now();

        // Filter constants (same as Arb_Bot)
        const MAX_AGE_MINUTES: i64 = 30;
        const MIN_VOLUME_24H_SOL: f64 = 0.01; // Layer 1: Minimum volume
        const MIN_SWAP_COUNT_24H: usize = 5; // Layer 2a: Minimum swaps
        const MAX_PRICE_DEVIATION: f64 = 0.50; // Layer 3: Max deviation from median

        // LAYER 3: Calculate median prices per token
        let mut token_prices: HashMap<String, Vec<f64>> = HashMap::new();
        for price_data in cache.values() {
            // Apply Layers 1 & 2 filters first
            let age = now.signed_duration_since(price_data.price.last_update);
            if age.num_minutes() >= MAX_AGE_MINUTES {
                continue;
            }

            let volume_24h = price_data.volume_tracker.get_24h_volume();
            if volume_24h < MIN_VOLUME_24H_SOL {
                continue;
            }

            let swap_count = price_data.volume_tracker.get_swap_count();
            if swap_count < MIN_SWAP_COUNT_24H {
                continue;
            }

            // Collect prices for median calculation
            token_prices
                .entry(price_data.price.token_mint.clone())
                .or_insert_with(Vec::new)
                .push(price_data.price.price_sol);
        }

        // Calculate median price for each token
        let mut token_medians: HashMap<String, f64> = HashMap::new();
        for (token, mut prices) in token_prices {
            if prices.is_empty() {
                continue;
            }

            prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = if prices.len() % 2 == 0 {
                (prices[prices.len() / 2 - 1] + prices[prices.len() / 2]) / 2.0
            } else {
                prices[prices.len() / 2]
            };
            token_medians.insert(token, median);
        }

        // Apply all 3 layers of filtering
        cache
            .values()
            .filter_map(|price_data| {
                // Layer 1: Freshness check
                let age = now.signed_duration_since(price_data.price.last_update);
                if age.num_minutes() >= MAX_AGE_MINUTES {
                    return None;
                }

                // Layer 2a: Volume check
                let volume_24h = price_data.volume_tracker.get_24h_volume();
                if volume_24h < MIN_VOLUME_24H_SOL {
                    return None;
                }

                // Layer 2b: Swap count check (filters illiquid "one-trade wonders")
                let swap_count = price_data.volume_tracker.get_swap_count();
                if swap_count < MIN_SWAP_COUNT_24H {
                    return None;
                }

                // Layer 3: Price deviation check
                // Reject price == 0.0 (dead pools)
                if price_data.price.price_sol == 0.0 {
                    return None;
                }

                // Reject prices >50% deviation from median (outliers/manipulated pools)
                if let Some(&median) = token_medians.get(&price_data.price.token_mint) {
                    if median > 0.0 {
                        let deviation = (price_data.price.price_sol - median).abs() / median;
                        if deviation > MAX_PRICE_DEVIATION {
                            return None;
                        }
                    }
                }

                // Return clean price
                let mut price = price_data.price.clone();
                price.volume_24h = volume_24h;
                price.swap_count_24h = swap_count;
                Some(price)
            })
            .collect()
    }

    /// Get all cached prices (unfiltered)
    pub async fn get_all_prices(&self) -> Vec<TokenPrice> {
        let cache = self.price_cache.read().await;
        cache
            .values()
            .map(|price_data| {
                let mut price = price_data.price.clone();
                price.volume_24h = price_data.volume_tracker.get_24h_volume();
                price.swap_count_24h = price_data.volume_tracker.get_swap_count();
                price
            })
            .collect()
    }

    /// Update price cache with new swap data
    pub async fn update_price(
        &self,
        token_mint: String,
        dex_name: String,
        pool_address: String,
        price_sol: f64,
        volume_sol: f64,
    ) {
        let pool_id = if pool_address.len() >= 8 {
            &pool_address[..8]
        } else {
            &pool_address
        };

        let cache_key = format!("{}_{}_{}", token_mint, dex_name, pool_id);

        let mut cache = self.price_cache.write().await;

        // Get or create price data
        let price_data = cache.entry(cache_key.clone()).or_insert_with(|| PriceData {
            price: TokenPrice {
                token_mint: token_mint.clone(),
                dex: format!("{}_{}", dex_name, pool_id),
                pool_address: pool_address.clone(),
                price_sol: 0.0,
                last_update: Utc::now(),
                volume_24h: 0.0,
                swap_count_24h: 0,
            },
            volume_tracker: VolumeTracker::new(),
        });

        // Add swap to volume tracker (automatically expires old swaps)
        price_data.volume_tracker.add_swap(volume_sol);

        // Update price and timestamp
        price_data.price.price_sol = price_sol;
        price_data.price.last_update = Utc::now();
    }

    /// Get the endpoint URL (for standalone monitoring function)
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Get the RPC URL (for standalone monitoring function)
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }
}

/// Standalone monitoring function (matches Arb_Bot working pattern)
/// This avoids the Arc<Self> method call issue inside tokio::spawn
pub async fn run_price_monitoring(
    endpoint: String,
    rpc_url: String,
    monitor: Arc<RealtimePriceMonitor>,
) -> Result<()> {
    // CRITICAL DEBUG: First line of function execution
    eprintln!("🔴 DEBUG: run_price_monitoring FUNCTION ENTERED");
    eprintln!("🔴 DEBUG: endpoint = {}", endpoint);
    eprintln!("🔴 DEBUG: rpc_url = {}", rpc_url);

    use std::time::Instant;

    eprintln!("🔴 DEBUG: About to log connection message");
    info!("🌊 Connecting to ShredStream: {}", endpoint);
    debug!("⏱️  Connection timeout: 30 seconds");
    eprintln!("🔴 DEBUG: Logged connection message");

    let start = Instant::now();
    eprintln!("🔴 DEBUG: Starting connection attempt");

    // LOOP 3 FIX: Add timeout and explicit error logging for connection debugging
    eprintln!("🔴 DEBUG: About to call ShredstreamClient::connect");
    let connect_result = tokio::time::timeout(
        std::time::Duration::from_secs(30), // Increased from 10s
        ShredstreamClient::connect(&endpoint),
    )
    .await;
    eprintln!("🔴 DEBUG: Connection attempt completed");

    eprintln!("🔴 DEBUG: Matching connection result");
    let mut client = match connect_result {
        Ok(Ok(client)) => {
            eprintln!("🔴 DEBUG: Connection SUCCESS branch");
            info!(
                "✅ Connected to ShredStream successfully in {:?}",
                start.elapsed()
            );
            debug!("🔌 Connection established, proceeding to subscription...");
            eprintln!("🔴 DEBUG: Returning connected client");
            client
        }
        Ok(Err(e)) => {
            eprintln!("🔴 DEBUG: Connection ERROR branch: {}", e);
            error!(
                "❌ ShredStream connection failed after {:?}: {}",
                start.elapsed(),
                e
            );
            error!("   Endpoint: {}", endpoint);
            error!("   Error type: {:?}", e);
            error!("   Check: IP whitelist (151.243.244.130), endpoint URL");
            return Err(anyhow::anyhow!("ShredStream connection failed: {}", e));
        }
        Err(_) => {
            eprintln!("🔴 DEBUG: Connection TIMEOUT branch");
            error!("❌ ShredStream connection TIMEOUT after 30 seconds");
            error!("   Endpoint: {}", endpoint);
            error!("   Possible causes:");
            error!("     1. Firewall blocking connection");
            error!("     2. Wrong endpoint URL");
            error!("     3. Network connectivity issue");
            error!("     4. ERPC service down");
            return Err(anyhow::anyhow!("ShredStream connection timeout after 30s"));
        }
    };
    eprintln!("🔴 DEBUG: Client obtained from match statement");

    // Define DEX programs to monitor (for arbitrage/MEV)
    let dex_programs = vec![
        // Raydium - Multiple Pool Types (4 variants)
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(), // Raydium AMM V4
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK".to_string(), // Raydium CLMM
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C".to_string(), // Raydium CPMM
        "5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h".to_string(), // Raydium Stable
        // Orca - Multiple Pool Types (2 variants)
        "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP".to_string(), // Orca Legacy
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(),  // Orca Whirlpools
        // Jupiter
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(), // Jupiter Aggregator
        // Serum
        "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin".to_string(), // Serum DEX
        // Meteora - Multiple Pool Types (3 variants)
        "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB".to_string(), // Meteora DAMM V1
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo".to_string(),  // Meteora DLMM
        "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG".to_string(),  // Meteora DAMM V2
        // PumpFun/PumpSwap
        "GMk6j2defJhS7F194toqmJNFNhAkbDXhYJo5oR3Rpump".to_string(), // PumpSwap
        // Additional DEXs
        "AMM55ShdkoGRB5jVYPjWziwk8m5MpwyDgsMWHaMSQWH6".to_string(), // Aldrin
        "SSwpkEEWHvCXCNWnMYXVW7gCYDXkF4aQMxKdpEqrZks".to_string(),  // Saros
        "6MLxLqiXaaSUpkgMnWDTuejNZEz3kE7k2woyHGVFw319".to_string(), // Crema
        "CTMAxxk34HjKWxQ3QLZQA1EQdxtjbYGP4Qjrw7nTn8bM".to_string(), // Cropper
        "EewxydAPCCVuNEyrVN68PuSYdQ7wKn27V9Gjeoi8dy3S".to_string(), // Lifinity
        "FLUXBmPhT3Fd1EDVFdg46YREqHBeNypn1h4EbnTzWERX".to_string(), // Fluxbeam
    ];

    info!(
        "🎯 Filtering ShredStream for {} DEX programs",
        dex_programs.len()
    );
    debug!("📋 DEX programs to monitor: {}", dex_programs.len());

    // Create subscription request with DEX program filters
    debug!("📝 Creating subscription request...");
    let request = ShredstreamClient::create_entries_request_for_accounts(
        vec![],                           // accounts (empty = all)
        dex_programs,                     // owner addresses = DEX program IDs
        vec![],                           // transaction account addresses (empty = all)
        Some(CommitmentLevel::Processed), // commitment level
    );

    // Subscribe to entries stream
    debug!("📡 Subscribing to ShredStream entries...");
    let mut stream = client.subscribe_entries(request).await?;
    info!("✅ Subscribed to ShredStream entries successfully");
    debug!("⏳ Waiting for first entry from stream...");

    // Create token decimal cache (CRITICAL for accurate prices!)
    let decimal_cache = TokenDecimalCache::new(rpc_url);
    decimal_cache.add_known_tokens().await;
    info!("🔢 Token decimal cache initialized");

    // Create DEX parser with decimal cache (with proper error handling)
    let dex_parser = DexSwapParser::new(decimal_cache)
        .map_err(|e| anyhow::anyhow!("Failed to initialize DEX parser: {}", e))?;

    let mut entries_processed = 0u64;
    let mut swaps_detected = 0u64;

    // Process incoming shred entries (REAL blockchain data)
    while let Some(slot_entry_result) = stream.next().await {
        match slot_entry_result {
            Ok(slot_entry) => {
                // Deserialize REAL entries from binary data
                let entries: Vec<Entry> = match bincode::deserialize(&slot_entry.entries) {
                    Ok(entries) => entries,
                    Err(e) => {
                        warn!("⚠️ Failed to deserialize entries: {}", e);
                        continue;
                    }
                };

                entries_processed += entries.len() as u64;

                // Extract REAL transactions from entries and parse DEX swaps
                for entry in entries {
                    for tx in entry.transactions {
                        // Parse DEX swap transaction (now async for decimal fetching)
                        if let Some(swap_info) = dex_parser
                            .parse_transaction(
                                &tx,
                                format!("{:?}", tx.signatures[0]),
                                slot_entry.slot,
                            )
                            .await
                        {
                            swaps_detected += 1;

                            // Update price cache with real swap data
                            monitor
                                .update_price(
                                    swap_info.token_mint,
                                    swap_info.dex_name,
                                    swap_info.pool_address,
                                    swap_info.price_sol,
                                    swap_info.amount_in as f64 / 1e9, // Convert lamports to SOL
                                )
                                .await;

                            if swaps_detected % 100 == 0 {
                                let cache_size = monitor.price_cache.read().await.len();
                                debug!(
                                    "📊 Stats: {} entries, {} swaps parsed, {} cached prices",
                                    entries_processed, swaps_detected, cache_size
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("❌ ShredStream error: {}", e);
                // Continue processing despite errors (no fake data fallback!)
            }
        }
    }

    warn!("📡 ShredStream stream ended");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_price_update() {
        let monitor = RealtimePriceMonitor::new("test_endpoint".to_string());

        monitor
            .update_price(
                "TokenA".to_string(),
                "Raydium".to_string(),
                "PoolABC123".to_string(),
                0.5,
                1.0,
            )
            .await;

        let prices = monitor.get_all_prices().await;
        assert_eq!(prices.len(), 1);
        assert_eq!(prices[0].price_sol, 0.5);
    }

    #[tokio::test]
    async fn test_filtering() {
        let monitor = RealtimePriceMonitor::new("test_endpoint".to_string());

        // Add some prices
        for i in 0..10 {
            monitor
                .update_price(
                    format!("Token{}", i),
                    "Raydium".to_string(),
                    format!("Pool{}", i),
                    0.5 + (i as f64 * 0.01),
                    0.02, // Above minimum volume
                )
                .await;
        }

        // Should filter out prices with < 5 swaps
        let filtered = monitor.get_filtered_prices().await;
        assert!(filtered.len() < 10); // Some should be filtered
    }
}
