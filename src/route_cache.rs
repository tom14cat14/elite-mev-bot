use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// High-performance route cache for Jupiter API optimization
/// Reduces API calls from ~100ms to ~5ms for cached routes
#[derive(Clone)]
pub struct RouteCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    default_ttl: Duration,
    max_entries: usize,
    stats: Arc<RwLock<CacheStats>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    route_data: Value,
    cached_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    hit_count: u64,
    last_accessed: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
    pub total_time_saved_ms: u64,
}

impl RouteCache {
    /// Create new route cache optimized for MEV performance
    pub fn new(default_ttl_seconds: u64, max_entries: usize) -> Self {
        info!("🚀 Initializing route cache with {}s TTL, {} max entries",
              default_ttl_seconds, max_entries);

        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(default_ttl_seconds),
            max_entries,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Get cached route if available and valid
    /// Returns None if cache miss or expired
    pub fn get_route(&self, input_mint: &str, output_mint: &str, amount: u64) -> Option<Value> {
        let key = self.generate_cache_key(input_mint, output_mint, amount);
        let now = Utc::now();

        {
            let mut stats = self.stats.write();
            stats.total_requests += 1;
        }

        let cache_result = {
            let cache = self.cache.read();
            if let Some(entry) = cache.get(&key) {
                if now <= entry.expires_at {
                    Some(entry.route_data.clone())
                } else {
                    debug!("⏰ Route cache EXPIRED for {}->{} ({})",
                           input_mint, output_mint, amount);
                    None
                }
            } else {
                None
            }
        };

        if let Some(route_data) = cache_result {
            // Cache hit - update stats and access time
            {
                let mut stats = self.stats.write();
                stats.cache_hits += 1;
                stats.total_time_saved_ms += 95; // ~95ms saved vs API call
            }

            // Update last accessed
            {
                let mut cache_write = self.cache.write();
                if let Some(entry) = cache_write.get_mut(&key) {
                    entry.hit_count += 1;
                    entry.last_accessed = now;
                }
            }

            debug!("💰 Route cache HIT for {}->{} ({}), saved ~95ms",
                   input_mint, output_mint, amount);

            return Some(route_data);
        }

        // Cache miss
        {
            let mut stats = self.stats.write();
            stats.cache_misses += 1;
        }

        debug!("❌ Route cache MISS for {}->{} ({})",
               input_mint, output_mint, amount);
        None
    }

    /// Store route in cache with TTL
    pub fn store_route(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        route_data: Value,
        custom_ttl: Option<Duration>,
    ) {
        let key = self.generate_cache_key(input_mint, output_mint, amount);
        let now = Utc::now();
        let ttl = custom_ttl.unwrap_or(self.default_ttl);

        let entry = CacheEntry {
            route_data,
            cached_at: now,
            expires_at: now + chrono::Duration::from_std(ttl).unwrap(),
            hit_count: 0,
            last_accessed: now,
        };

        {
            let mut cache = self.cache.write();

            // Evict if at capacity
            if cache.len() >= self.max_entries {
                self.evict_lru(&mut cache);
            }

            cache.insert(key.clone(), entry);
        }

        debug!("💾 Stored route {}->{} ({}) in cache with TTL {}s",
               input_mint, output_mint, amount, ttl.as_secs());
    }

    /// Get cache performance statistics
    pub fn get_stats(&self) -> CacheStats {
        self.stats.read().clone()
    }

    /// Get cache hit rate as percentage
    pub fn get_hit_rate(&self) -> f64 {
        let stats = self.stats.read();
        if stats.total_requests == 0 {
            0.0
        } else {
            (stats.cache_hits as f64 / stats.total_requests as f64) * 100.0
        }
    }

    /// Clear expired entries from cache
    pub fn cleanup_expired(&self) {
        let now = Utc::now();
        let mut cache = self.cache.write();
        let initial_size = cache.len();

        cache.retain(|_key, entry| now <= entry.expires_at);

        let removed = initial_size - cache.len();
        if removed > 0 {
            info!("🧹 Cleaned up {} expired cache entries", removed);
        }
    }

    /// Generate cache key for route
    fn generate_cache_key(&self, input_mint: &str, output_mint: &str, amount: u64) -> String {
        // For MEV, we cache based on token pair and amount tiers
        let amount_tier = self.get_amount_tier(amount);
        format!("{}:{}:{}", input_mint, output_mint, amount_tier)
    }

    /// Group amounts into tiers for better cache hit rates
    /// Small differences in amounts can use same route
    fn get_amount_tier(&self, amount: u64) -> String {
        match amount {
            0..=1_000_000 => "micro".to_string(),           // <1M
            1_000_001..=10_000_000 => "small".to_string(),  // 1M-10M
            10_000_001..=100_000_000 => "medium".to_string(), // 10M-100M
            100_000_001..=1_000_000_000 => "large".to_string(), // 100M-1B
            _ => "huge".to_string(),                         // >1B
        }
    }

    /// Evict least recently used entry
    fn evict_lru(&self, cache: &mut HashMap<String, CacheEntry>) {
        if let Some(lru_key) = cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone())
        {
            cache.remove(&lru_key);
            {
                let mut stats = self.stats.write();
                stats.evictions += 1;
            }
            debug!("🗑️ Evicted LRU cache entry: {}", lru_key);
        }
    }

    /// Get cache efficiency metrics for monitoring
    pub fn get_efficiency_report(&self) -> CacheEfficiencyReport {
        let stats = self.stats.read();
        let cache_size = self.cache.read().len();

        CacheEfficiencyReport {
            hit_rate_percent: self.get_hit_rate(),
            total_requests: stats.total_requests,
            cache_hits: stats.cache_hits,
            cache_misses: stats.cache_misses,
            evictions: stats.evictions,
            current_entries: cache_size,
            max_entries: self.max_entries,
            estimated_time_saved_ms: stats.total_time_saved_ms,
            estimated_api_calls_avoided: stats.cache_hits,
        }
    }

    /// Invalidate cache for specific token pair (useful during high volatility)
    pub fn invalidate_pair(&self, input_mint: &str, output_mint: &str) {
        let mut cache = self.cache.write();
        let mut removed = 0;

        cache.retain(|key, _| {
            if key.starts_with(&format!("{}:{}", input_mint, output_mint)) {
                removed += 1;
                false
            } else {
                true
            }
        });

        if removed > 0 {
            info!("🔄 Invalidated {} cache entries for pair {}->{}",
                  removed, input_mint, output_mint);
        }
    }

    /// Pre-warm cache with common trading pairs
    pub async fn prewarm_cache(&self, common_pairs: Vec<(String, String, u64)>) {
        info!("🔥 Pre-warming cache with {} common pairs", common_pairs.len());

        // This would integrate with Jupiter API to pre-fetch routes
        // For now, we'll just log the pairs that should be pre-warmed
        for (input_mint, output_mint, amount) in common_pairs {
            debug!("🎯 Should pre-warm: {} -> {} ({})", input_mint, output_mint, amount);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheEfficiencyReport {
    pub hit_rate_percent: f64,
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
    pub current_entries: usize,
    pub max_entries: usize,
    pub estimated_time_saved_ms: u64,
    pub estimated_api_calls_avoided: u64,
}

impl Default for RouteCache {
    fn default() -> Self {
        // Optimized defaults for MEV trading
        Self::new(
            2,      // 2 second TTL for high volatility
            10_000  // 10k max entries for memory efficiency
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cache_basic_operations() {
        let cache = RouteCache::new(2, 100);
        let route_data = json!({"route": "test_data"});

        // Test miss
        assert!(cache.get_route("SOL", "USDC", 1000000).is_none());

        // Test store and hit
        cache.store_route("SOL", "USDC", 1000000, route_data.clone(), None);
        assert!(cache.get_route("SOL", "USDC", 1000000).is_some());

        // Test stats
        let stats = cache.get_stats();
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
    }

    #[test]
    fn test_amount_tiers() {
        let cache = RouteCache::default();

        assert_eq!(cache.get_amount_tier(500_000), "micro");
        assert_eq!(cache.get_amount_tier(5_000_000), "small");
        assert_eq!(cache.get_amount_tier(50_000_000), "medium");
        assert_eq!(cache.get_amount_tier(500_000_000), "large");
        assert_eq!(cache.get_amount_tier(5_000_000_000), "huge");
    }

    #[test]
    fn test_cache_key_generation() {
        let cache = RouteCache::default();

        let key1 = cache.generate_cache_key("SOL", "USDC", 1_000_000);
        let key2 = cache.generate_cache_key("SOL", "USDC", 999_999);

        // Both should map to same tier for cache efficiency
        assert_eq!(key1, key2);
    }
}