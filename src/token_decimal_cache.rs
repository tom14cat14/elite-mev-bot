use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Safe string truncation to prevent panics on short strings
fn truncate_safe(s: &str, max_len: usize) -> String {
    s.chars().take(max_len).collect()
}

/// Cache for token decimal information
/// Critical for accurate price calculations (SOL=9, USDC=6, varies by token)
#[derive(Clone)]
pub struct TokenDecimalCache {
    cache: Arc<RwLock<HashMap<String, u8>>>,
    rpc_client: Arc<RpcClient>,
}

impl TokenDecimalCache {
    pub fn new(rpc_url: String) -> Self {
        let rpc_client = Arc::new(RpcClient::new(rpc_url));
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            rpc_client,
        }
    }

    /// Get token decimals from cache or fetch from chain
    pub async fn get_decimals(&self, token_mint: &str) -> Result<u8, String> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(&decimals) = cache.get(token_mint) {
                return Ok(decimals);
            }
        }

        // Not in cache, fetch from chain
        let decimals = self.fetch_decimals_from_chain(token_mint).await?;

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(token_mint.to_string(), decimals);
        }

        debug!("🔢 Cached decimals for {}: {}", truncate_safe(token_mint, 8), decimals);
        Ok(decimals)
    }

    /// Fetch token decimals from Solana chain by querying the mint account
    async fn fetch_decimals_from_chain(&self, token_mint: &str) -> Result<u8, String> {
        let pubkey = Pubkey::from_str(token_mint)
            .map_err(|e| format!("Invalid pubkey: {}", e))?;

        // Get account info for the mint
        let account = self.rpc_client
            .get_account(&pubkey)
            .await
            .map_err(|e| format!("Failed to fetch mint account: {}", e))?;

        // SPL Token mint account layout:
        // - bytes 0-35: mint_authority (36 bytes)
        // - bytes 36-43: supply (8 bytes)
        // - byte 44: decimals (1 byte)

        if account.data.len() < 45 {
            return Err(format!("Invalid mint account data length: {}", account.data.len()));
        }

        let decimals = account.data[44];

        // Sanity check: decimals should be reasonable (0-18)
        if decimals > 18 {
            warn!("⚠️ Unusual decimals {} for mint {}, using anyway", decimals, truncate_safe(&token_mint, 8));
        }

        Ok(decimals)
    }

    /// Pre-warm cache with known token decimals to avoid RPC calls
    pub async fn add_known_tokens(&self) {
        let mut cache = self.cache.write().await;

        // Common tokens with known decimals
        cache.insert("So11111111111111111111111111111111111111112".to_string(), 9); // SOL
        cache.insert("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), 6); // USDC
        cache.insert("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(), 6); // USDT

        debug!("🔢 Pre-warmed decimal cache with {} known tokens", cache.len());
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, Vec<String>) {
        let cache = self.cache.read().await;
        let size = cache.len();
        let tokens: Vec<String> = cache.keys().take(5).cloned().collect();
        (size, tokens)
    }
}

/// Calculate adjusted price accounting for token decimals
/// This is the CRITICAL fix for accurate price calculations
pub fn calculate_adjusted_price(
    amount_in: u64,
    amount_out: u64,
    decimals_in: u8,
    decimals_out: u8,
) -> f64 {
    if amount_in == 0 {
        return 0.0;
    }

    // Convert raw amounts to decimal-adjusted amounts
    let adjusted_in = amount_in as f64 / 10_f64.powi(decimals_in as i32);
    let adjusted_out = amount_out as f64 / 10_f64.powi(decimals_out as i32);

    // Price = output / input (in real units, not lamports)
    adjusted_out / adjusted_in
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_calculation() {
        // Test: 1 SOL (1e9 lamports) → 100 USDC (100e6 micro-USDC)
        // Expected price: 100 USDC per SOL
        let amount_in = 1_000_000_000; // 1 SOL in lamports
        let amount_out = 100_000_000; // 100 USDC in micro-USDC
        let decimals_in = 9; // SOL decimals
        let decimals_out = 6; // USDC decimals

        let price = calculate_adjusted_price(amount_in, amount_out, decimals_in, decimals_out);

        // Price should be 100.0 (100 USDC per SOL)
        assert!((price - 100.0).abs() < 0.0001, "Expected ~100.0, got {}", price);
    }

    #[test]
    fn test_price_calculation_without_decimals() {
        // Without decimal adjustment, we'd get:
        let amount_in = 1_000_000_000_u64;
        let amount_out = 100_000_000_u64;
        let raw_price = amount_out as f64 / amount_in as f64;

        // Raw price would be 0.1 (wrong by 1000x!)
        assert!((raw_price - 0.1).abs() < 0.0001, "Raw price is {}", raw_price);
    }
}
