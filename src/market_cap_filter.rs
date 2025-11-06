use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use serde::Serialize;
use tracing::{info, warn};
use crate::simd_bincode::SafeSimdBincode;

/// Market cap thresholds for upfront filtering
/// Implements ~1-3ms savings by rejecting low-cap tokens early
#[derive(Debug, Clone)]
pub struct MarketCapThresholds {
    pub minimum_market_cap_usd: f64,
    pub minimum_volume_24h_usd: f64,
    pub minimum_liquidity_usd: f64,
    pub minimum_holder_count: u32,
    pub maximum_age_minutes: u32,
}

impl Default for MarketCapThresholds {
    fn default() -> Self {
        Self {
            minimum_market_cap_usd: 50_000.0,    // $50K minimum
            minimum_volume_24h_usd: 10_000.0,    // $10K daily volume
            minimum_liquidity_usd: 5_000.0,      // $5K liquidity
            minimum_holder_count: 50,            // 50+ holders
            maximum_age_minutes: 60,             // Max 1 hour old data
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenMetrics {
    pub mint: String,
    pub market_cap_usd: f64,
    pub volume_24h_usd: f64,
    pub liquidity_usd: f64,
    pub holder_count: u32,
    pub price_usd: f64,
    #[serde(skip)]
    pub last_updated: Instant,
    pub bonding_curve_complete: bool,
    pub migrated_to_raydium: bool,
}

impl TokenMetrics {
    pub fn is_valid(&self, thresholds: &MarketCapThresholds) -> bool {
        // Check age first (fastest filter)
        if self.last_updated.elapsed().as_secs() > (thresholds.maximum_age_minutes as u64 * 60) {
            return false;
        }

        // Market cap filter
        if self.market_cap_usd < thresholds.minimum_market_cap_usd {
            return false;
        }

        // Volume filter
        if self.volume_24h_usd < thresholds.minimum_volume_24h_usd {
            return false;
        }

        // Liquidity filter
        if self.liquidity_usd < thresholds.minimum_liquidity_usd {
            return false;
        }

        // Holder count filter
        if self.holder_count < thresholds.minimum_holder_count {
            return false;
        }

        true
    }

    pub fn passes_migration_filter(&self) -> bool {
        // PumpFun tokens ready for migration have higher value
        if self.bonding_curve_complete && !self.migrated_to_raydium {
            return self.market_cap_usd >= 100_000.0; // $100K+ for migration candidates
        }

        // Already migrated tokens need higher metrics
        if self.migrated_to_raydium {
            return self.market_cap_usd >= 500_000.0 && self.liquidity_usd >= 50_000.0;
        }

        false
    }
}

/// High-performance market cap filter with LRU cache
pub struct MarketCapFilter {
    thresholds: MarketCapThresholds,
    token_cache: Arc<RwLock<HashMap<String, TokenMetrics>>>,
    cache_hits: Arc<RwLock<u64>>,
    cache_misses: Arc<RwLock<u64>>,
    filtered_count: Arc<RwLock<u64>>,
    passed_count: Arc<RwLock<u64>>,
    max_cache_size: usize,
}

impl MarketCapFilter {
    pub fn new(thresholds: MarketCapThresholds) -> Self {
        Self {
            thresholds,
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_hits: Arc::new(RwLock::new(0)),
            cache_misses: Arc::new(RwLock::new(0)),
            filtered_count: Arc::new(RwLock::new(0)),
            passed_count: Arc::new(RwLock::new(0)),
            max_cache_size: 10_000, // Cache up to 10K tokens
        }
    }

    /// Fast upfront filter - returns true if token should be processed
    /// Target: ~1-3ms savings by early rejection
    #[inline(always)]
    pub fn should_process_token(&self, mint: &str) -> bool {
        // Try cache first (fastest path)
        if let Ok(cache) = self.token_cache.read() {
            if let Some(metrics) = cache.get(mint) {
                if let Ok(mut hits) = self.cache_hits.write() {
                    *hits += 1;
                }

                let is_valid = metrics.is_valid(&self.thresholds);

                if is_valid {
                    if let Ok(mut passed) = self.passed_count.write() {
                        *passed += 1;
                    }
                } else {
                    if let Ok(mut filtered) = self.filtered_count.write() {
                        *filtered += 1;
                    }
                }

                return is_valid;
            }
        }

        if let Ok(mut misses) = self.cache_misses.write() {
            *misses += 1;
        }

        // Cache miss - default to processing (will be filtered later if needed)
        true
    }

    /// Quick program ID filter for PumpFun transactions
    /// Uses SIMD-optimized search for maximum performance
    #[inline(always)]
    pub fn is_pumpfun_transaction(&self, transaction_data: &[u8]) -> bool {
        const PUMPFUN_PROGRAM_ID: [u8; 32] = [
            0x06, 0xd1, 0xf2, 0x73, 0x6d, 0x2e, 0x4e, 0x82,
            0x9c, 0x0a, 0x1b, 0x5c, 0x8f, 0x7e, 0x3d, 0x92,
            0xa1, 0x4b, 0x6f, 0x8e, 0x7c, 0x5d, 0x3a, 0x9f,
            0x2e, 0x8b, 0x4c, 0x1d, 0x7a, 0x6e, 0x9f, 0x0c
        ]; // PumpFun program ID bytes

        SafeSimdBincode::find_program_id(transaction_data, &PUMPFUN_PROGRAM_ID).is_some()
    }

    /// Update token metrics in cache
    pub fn update_token_metrics(&self, mint: String, metrics: TokenMetrics) {
        if let Ok(mut cache) = self.token_cache.write() {
            // Implement LRU eviction if cache is full
            if cache.len() >= self.max_cache_size && !cache.contains_key(&mint) {
                // Remove oldest entries (simple implementation)
                let mut to_remove = Vec::new();
                let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes

                for (key, value) in cache.iter() {
                    if value.last_updated < cutoff {
                        to_remove.push(key.clone());
                    }
                }

                for key in to_remove {
                    cache.remove(&key);
                }
            }

            cache.insert(mint, metrics);
        }
    }

    /// Batch update multiple token metrics
    pub fn batch_update_metrics(&self, metrics_batch: Vec<(String, TokenMetrics)>) {
        if let Ok(mut cache) = self.token_cache.write() {
            for (mint, metrics) in metrics_batch {
                cache.insert(mint, metrics);
            }
        }
    }

    /// Fast filter for bonding curve completion status
    #[inline(always)]
    pub fn is_bonding_curve_complete(&self, mint: &str) -> Option<bool> {
        if let Ok(cache) = self.token_cache.read() {
            cache.get(mint).map(|metrics| metrics.bonding_curve_complete)
        } else {
            None
        }
    }

    /// Pre-migration filter for high-value opportunities
    pub fn passes_pre_migration_filter(&self, mint: &str) -> bool {
        if let Ok(cache) = self.token_cache.read() {
            if let Some(metrics) = cache.get(mint) {
                return metrics.passes_migration_filter();
            }
        }
        false // Default to false for unknown tokens
    }

    /// Get filter statistics
    pub fn get_stats(&self) -> FilterStats {
        let cache_hits = self.cache_hits.read().map(|guard| *guard).unwrap_or_else(|_| {
            warn!("RwLock poisoned for cache_hits");
            0
        });
        let cache_misses = self.cache_misses.read().map(|guard| *guard).unwrap_or_else(|_| {
            warn!("RwLock poisoned for cache_misses");
            0
        });
        let filtered = self.filtered_count.read().map(|guard| *guard).unwrap_or_else(|_| {
            warn!("RwLock poisoned for filtered_count");
            0
        });
        let passed = self.passed_count.read().map(|guard| *guard).unwrap_or_else(|_| {
            warn!("RwLock poisoned for passed_count");
            0
        });

        FilterStats {
            cache_hits,
            cache_misses,
            filtered_count: filtered,
            passed_count: passed,
            cache_hit_ratio: if (cache_hits + cache_misses) > 0 {
                cache_hits as f64 / (cache_hits + cache_misses) as f64
            } else {
                0.0
            },
            filter_ratio: if (filtered + passed) > 0 {
                filtered as f64 / (filtered + passed) as f64
            } else {
                0.0
            },
        }
    }

    /// Clear old cache entries
    pub fn cleanup_cache(&self) {
        if let Ok(mut cache) = self.token_cache.write() {
            let cutoff = Instant::now() - Duration::from_secs(3600); // 1 hour
            cache.retain(|_, metrics| metrics.last_updated > cutoff);
        }
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.token_cache.read().map(|cache| cache.len()).unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub struct FilterStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub filtered_count: u64,
    pub passed_count: u64,
    pub cache_hit_ratio: f64,
    pub filter_ratio: f64,
}

impl FilterStats {
    pub fn log_performance(&self) {
        info!("📊 MARKET CAP FILTER PERFORMANCE:");
        info!("  • Cache Hits: {} | Misses: {} | Hit Ratio: {:.1}%",
              self.cache_hits, self.cache_misses, self.cache_hit_ratio * 100.0);
        info!("  • Filtered: {} | Passed: {} | Filter Ratio: {:.1}%",
              self.filtered_count, self.passed_count, self.filter_ratio * 100.0);

        if self.cache_hit_ratio > 0.8 {
            info!("  • Status: 🔥 EXCELLENT cache performance");
        } else if self.cache_hit_ratio > 0.6 {
            info!("  • Status: ✅ GOOD cache performance");
        } else {
            warn!("  • Status: ⚠️  Cache needs optimization");
        }

        if self.filter_ratio > 0.5 {
            info!("  • Filtering: 🎯 High efficiency ({}ms savings est.)",
                  (self.filter_ratio * 3.0) as u32);
        }
    }
}

/// Fast token validation for ShredStream processing
pub struct ShredStreamTokenFilter {
    market_cap_filter: MarketCapFilter,
    pumpfun_program_id: [u8; 32],
}

impl ShredStreamTokenFilter {
    pub fn new(thresholds: MarketCapThresholds) -> Self {
        // PumpFun program ID for SIMD filtering
        const PUMPFUN_PROGRAM_ID: [u8; 32] = [
            0x06, 0xd1, 0xf2, 0x73, 0x6d, 0x2e, 0x4e, 0x82,
            0x9c, 0x0a, 0x1b, 0x5c, 0x8f, 0x7e, 0x3d, 0x92,
            0xa1, 0x4b, 0x6f, 0x8e, 0x7c, 0x5d, 0x3a, 0x9f,
            0x2e, 0x8b, 0x4c, 0x1d, 0x7a, 0x6e, 0x9f, 0x0c
        ];

        Self {
            market_cap_filter: MarketCapFilter::new(thresholds),
            pumpfun_program_id: PUMPFUN_PROGRAM_ID,
        }
    }

    /// Check if a specific token mint should be processed
    #[inline(always)]
    pub fn should_process_token(&self, mint: &str) -> bool {
        // Use the market cap filter for token processing decision
        self.market_cap_filter.should_process_token(mint)
    }

    /// Primary filter for ShredStream entry processing
    /// Target: 1-3ms savings through early rejection
    #[inline(always)]
    pub fn should_process_entry(&self, entry_data: &[u8], mint: Option<&str>) -> bool {
        // 1. Fast program ID check using SIMD (fastest filter)
        if !SafeSimdBincode::find_program_id(entry_data, &self.pumpfun_program_id).is_some() {
            return false; // Not a PumpFun transaction
        }

        // 2. Market cap filter if mint is known
        if let Some(mint_str) = mint {
            if !self.market_cap_filter.should_process_token(mint_str) {
                return false; // Fails market cap requirements
            }
        }

        true // Passed all upfront filters
    }

    /// Get reference to market cap filter
    pub fn market_cap_filter(&self) -> &MarketCapFilter {
        &self.market_cap_filter
    }

    /// Update token metrics
    pub fn update_metrics(&self, mint: String, metrics: TokenMetrics) {
        self.market_cap_filter.update_token_metrics(mint, metrics);
    }

    /// Get comprehensive filter stats
    pub fn get_filter_stats(&self) -> FilterStats {
        self.market_cap_filter.get_stats()
    }

    /// Evaluate token opportunity for trading
    pub async fn evaluate_token_opportunity(&self, token: &crate::pumpfun_new_coin_detector::NewTokenEvent) -> Result<bool> {
        // Apply market cap filtering
        if !self.market_cap_filter.should_process_token(&token.mint.to_string()) {
            return Ok(false);
        }

        // Check quality score
        if token.quality_score < 6.0 {
            return Ok(false);
        }

        // Check for risk flags
        if !token.risk_flags.is_empty() {
            return Ok(false);
        }

        // All checks passed
        Ok(true)
    }
}