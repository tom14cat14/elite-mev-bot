use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::jupiter_rate_limiter::JupiterRateLimiter;
use crate::route_cache::RouteCache;

/// Jupiter executor focused ONLY on execution, not price discovery
/// All price discovery should come from ShredStream data
/// Includes route caching for 5ms cache hits vs 100ms API calls
pub struct JupiterExecutor {
    rate_limiter: JupiterRateLimiter,
    route_cache: RouteCache,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionParams {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u16,
    pub wallet_pubkey: String,
    pub quote_response: Value, // Pre-calculated route from ShredStream analysis
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub signature: Option<String>,
    pub actual_amount_out: Option<u64>,
    pub actual_profit_sol: Option<f64>,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
}

impl JupiterExecutor {
    pub fn new(api_key: String) -> Self {
        Self {
            rate_limiter: JupiterRateLimiter::new(api_key),
            route_cache: RouteCache::default(), // 2s TTL, 10k entries optimized for MEV
        }
    }

    /// Create new executor with custom cache settings
    pub fn new_with_cache(
        api_key: String,
        cache_ttl_seconds: u64,
        max_cache_entries: usize,
    ) -> Self {
        Self {
            rate_limiter: JupiterRateLimiter::new(api_key),
            route_cache: RouteCache::new(cache_ttl_seconds, max_cache_entries),
        }
    }

    /// Execute a swap using pre-calculated route data from ShredStream
    /// This method does NOT do price discovery - it assumes the route is already optimized
    pub async fn execute_swap(&self, params: ExecutionParams) -> Result<ExecutionResult> {
        let start_time = std::time::Instant::now();

        debug!(
            "Executing Jupiter swap: {} -> {} (amount: {})",
            params.input_mint, params.output_mint, params.amount
        );

        // Step 1: Create order using Jupiter Ultra API
        let order_request = self.build_order_request(params.clone())?;

        match self
            .rate_limiter
            .execute_request::<Value>("/ultra/v1/order", Some(order_request))
            .await
        {
            Ok(swap_response) => {
                let execution_time = start_time.elapsed().as_millis() as u64;

                // Extract transaction data
                let signature = swap_response
                    .get("txid")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                // Calculate actual output (simplified - you'd parse the actual transaction)
                let actual_amount_out = self.extract_output_amount(&swap_response);
                let actual_profit_sol = self.calculate_actual_profit(&params, actual_amount_out);

                info!(
                    "Jupiter swap executed successfully in {}ms: {:?}",
                    execution_time, signature
                );

                Ok(ExecutionResult {
                    success: true,
                    signature,
                    actual_amount_out,
                    actual_profit_sol,
                    execution_time_ms: execution_time,
                    error_message: None,
                })
            }
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                warn!("Jupiter swap failed after {}ms: {}", execution_time, e);

                Ok(ExecutionResult {
                    success: false,
                    signature: None,
                    actual_amount_out: None,
                    actual_profit_sol: None,
                    execution_time_ms: execution_time,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    /// Execute multiple swaps in sequence for complex arbitrage
    pub async fn execute_arbitrage_sequence(
        &self,
        swaps: Vec<ExecutionParams>,
    ) -> Result<Vec<ExecutionResult>> {
        let mut results = Vec::new();

        for (i, swap_params) in swaps.into_iter().enumerate() {
            debug!("Executing arbitrage step {}", i + 1);

            match self.execute_swap(swap_params).await {
                Ok(result) => {
                    let success = result.success;
                    results.push(result);

                    if !success {
                        warn!("Arbitrage sequence failed at step {}, aborting", i + 1);
                        break;
                    }
                }
                Err(e) => {
                    warn!("Arbitrage sequence error at step {}: {}", i + 1, e);
                    results.push(ExecutionResult {
                        success: false,
                        signature: None,
                        actual_amount_out: None,
                        actual_profit_sol: None,
                        execution_time_ms: 0,
                        error_message: Some(e.to_string()),
                    });
                    break;
                }
            }
        }

        Ok(results)
    }

    /// Build order request payload for Jupiter Ultra API
    fn build_order_request(&self, params: ExecutionParams) -> Result<Value> {
        let request = serde_json::json!({
            "inputMint": params.input_mint,
            "outputMint": params.output_mint,
            "amount": params.amount,
            "slippageBps": params.slippage_bps,
            "wallet": params.wallet_pubkey,
            "wrapAndUnwrapSol": true,
            "computeUnitPriceMicroLamports": "auto"
        });

        Ok(request)
    }

    /// Extract actual output amount from swap response
    fn extract_output_amount(&self, response: &Value) -> Option<u64> {
        // This is simplified - you'd need to parse the actual transaction logs
        // to get the real output amount
        response
            .get("outAmount")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
    }

    /// Calculate actual profit in SOL
    fn calculate_actual_profit(
        &self,
        params: &ExecutionParams,
        actual_output: Option<u64>,
    ) -> Option<f64> {
        // This is simplified - you'd need proper token price conversion
        // For now, assuming we can convert token amounts to SOL equivalent
        if let Some(output) = actual_output {
            // Placeholder calculation - you'd need real price feeds
            let estimated_profit = (output as f64) - (params.amount as f64);
            let sol_equivalent = estimated_profit / 1_000_000_000.0; // Convert lamports to SOL
            Some(sol_equivalent)
        } else {
            None
        }
    }

    /// Get rate limiter statistics
    pub fn get_rate_limiter_stats(&self) -> crate::jupiter_rate_limiter::RateLimiterStats {
        self.rate_limiter.get_stats()
    }

    /// Get order status from Jupiter Ultra API
    pub async fn get_order_status(&self, order_id: &str) -> Result<Value> {
        let endpoint = format!("/ultra/v1/order?orderId={}", order_id);
        self.rate_limiter
            .execute_request::<Value>(&endpoint, None)
            .await
    }

    /// Get cached route or fetch new one (5ms cache hit vs 100ms API call)
    /// Jupiter Ultra API doesn't have quote endpoints - routes come from ShredStream analysis
    /// This caches the route data that comes from ShredStream transaction analysis
    pub fn get_route_from_cache(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Option<Value> {
        // Check cache first for 5ms response
        if let Some(cached_route) = self.route_cache.get_route(input_mint, output_mint, amount) {
            debug!(
                "💰 Route cache HIT for {}->{} ({})",
                input_mint, output_mint, amount
            );
            return Some(cached_route);
        }

        debug!(
            "❌ Route cache MISS for {}->{} ({})",
            input_mint, output_mint, amount
        );
        None
    }

    /// Store route data in cache (typically from ShredStream analysis)
    pub fn cache_route_data(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        route_data: Value,
        custom_ttl_seconds: Option<u64>,
    ) {
        let custom_ttl = custom_ttl_seconds.map(Duration::from_secs);
        self.route_cache
            .store_route(input_mint, output_mint, amount, route_data, custom_ttl);
        debug!(
            "💾 Stored route in cache: {}->{} ({})",
            input_mint, output_mint, amount
        );
    }

    /// Get route cache statistics for monitoring
    pub fn get_cache_stats(&self) -> crate::route_cache::CacheStats {
        self.route_cache.get_stats()
    }

    /// Get cache efficiency report
    pub fn get_cache_efficiency(&self) -> crate::route_cache::CacheEfficiencyReport {
        self.route_cache.get_efficiency_report()
    }

    /// Clear expired cache entries (call periodically)
    pub fn cleanup_cache(&self) {
        self.route_cache.cleanup_expired()
    }

    /// Invalidate cache for specific pair during high volatility
    pub fn invalidate_pair_cache(&self, input_mint: &str, output_mint: &str) {
        self.route_cache.invalidate_pair(input_mint, output_mint)
    }

    /// Health check for Jupiter Ultra API connectivity
    pub async fn health_check(&self) -> Result<bool> {
        // Use a simple GET request to test Ultra API connectivity
        match self
            .rate_limiter
            .execute_request::<Value>("/ultra/v1/order?orderId=test", None)
            .await
        {
            Ok(_) => {
                debug!("Jupiter Ultra API health check passed");
                Ok(true)
            }
            Err(e) => {
                let error_msg = e.to_string();
                // Ultra API may return 400/404 for invalid order ID, but that means API is responding
                if error_msg.contains("400")
                    || error_msg.contains("404")
                    || error_msg.contains("Bad Request")
                    || error_msg.contains("Not Found")
                {
                    debug!(
                        "Jupiter Ultra API health check passed (400/404 expected for test order)"
                    );
                    Ok(true)
                } else {
                    warn!("Jupiter Ultra API health check failed: {}", e);
                    Ok(false)
                }
            }
        }
    }
}

/// Helper function to create a quote request for Jupiter
/// This is used for initial route discovery but should be used sparingly
pub fn create_quote_request(
    input_mint: &str,
    output_mint: &str,
    amount: u64,
    slippage_bps: u16,
) -> Value {
    serde_json::json!({
        "inputMint": input_mint,
        "outputMint": output_mint,
        "amount": amount,
        "slippageBps": slippage_bps,
        "onlyDirectRoutes": false,
        "asLegacyTransaction": false
    })
}
