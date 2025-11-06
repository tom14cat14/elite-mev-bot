/// Global constants for Elite MEV Bot v2.1
///
/// This module centralizes all magic numbers and configuration constants
/// to improve code maintainability and reduce errors from inconsistent values.

use std::time::Duration;

// ============================================================================
// SOLANA BLOCKCHAIN CONSTANTS
// ============================================================================

/// 1 SOL = 1 billion lamports
pub const SOL_DECIMALS: u64 = 1_000_000_000;

/// Convert lamports to SOL (floating point)
pub const fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / SOL_DECIMALS as f64
}

/// Convert SOL to lamports
pub const fn sol_to_lamports(sol: f64) -> u64 {
    (sol * SOL_DECIMALS as f64) as u64
}

/// Solana block time (approximately 400ms per slot)
pub const SOLANA_BLOCK_TIME_MS: u64 = 400;

/// Solana slot time (same as block time)
pub const SOLANA_SLOT_TIME_MS: u64 = 400;

/// Default transaction timeout (30 seconds)
pub const DEFAULT_TX_TIMEOUT_SECONDS: u64 = 30;

// ============================================================================
// TRANSACTION FEE CONSTANTS
// ============================================================================

/// Estimated gas cost for a standard transaction (~0.00005 SOL)
pub const ESTIMATED_GAS_LAMPORTS: u64 = 50_000;

/// Safety buffer for unexpected fees and rent (~0.005 SOL)
pub const SAFETY_BUFFER_LAMPORTS: u64 = 5_000_000;

/// Maximum transaction compute units
pub const MAX_COMPUTE_UNITS: u32 = 1_400_000;

/// Default compute units for simple transactions
pub const DEFAULT_COMPUTE_UNITS: u32 = 200_000;

/// Minimum rent-exempt balance for accounts (approximately)
pub const MIN_RENT_EXEMPT_BALANCE: u64 = 2_039_280; // ~0.002 SOL

// ============================================================================
// PUMPFUN BONDING CURVE CONSTANTS
// ============================================================================

/// PumpFun program ID
pub const PUMPFUN_PROGRAM_ID: &str = "PumpFunP4PfMpqd7KsAEL7NKPhpq6M4yDmMRr2tH6gN";

/// Bonding curve completion threshold (~85 SOL triggers migration to Raydium)
pub const BONDING_CURVE_COMPLETION_SOL: f64 = 85.0;
pub const BONDING_CURVE_MIGRATION_SOL: u64 = 85_000_000_000; // In lamports

/// Minimum real reserves to consider bonding curve active (~0.001 SOL)
pub const MINIMUM_REAL_RESERVES: u64 = 1_000_000;

/// PumpFun instruction discriminators (TODO: Verify these are correct!)
/// WARNING: These are placeholder values and must be validated against actual program
pub const PUMPFUN_BUY_DISCRIMINATOR: u8 = 0x66;  // NEEDS VERIFICATION
pub const PUMPFUN_SELL_DISCRIMINATOR: u8 = 0x33; // NEEDS VERIFICATION
pub const PUMPFUN_CREATE_DISCRIMINATOR: u8 = 0x01; // NEEDS VERIFICATION

// ============================================================================
// DEX CONSTANTS
// ============================================================================

/// Raydium AMM V4 program ID
pub const RAYDIUM_AMM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Orca Whirlpools program ID
pub const ORCA_WHIRLPOOLS_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

/// Raydium swap instruction discriminator
pub const RAYDIUM_SWAP_DISCRIMINATOR: u8 = 9;

/// Orca swap instruction discriminator
pub const ORCA_SWAP_DISCRIMINATOR: u8 = 6;

/// Typical DEX fee in basis points (0.25% = 25 bps)
pub const TYPICAL_DEX_FEE_BPS: u16 = 25;

// ============================================================================
// TRADING STRATEGY CONSTANTS
// ============================================================================

/// Minimum token quality score to consider trading
pub const MIN_TOKEN_QUALITY_SCORE: f64 = 8.5;

/// Maximum market cap for pre-migration tokens (USD)
pub const MAX_MARKET_CAP_USD: f64 = 90_000.0;

/// Minimum volume per minute to consider token active (USD)
pub const MIN_VOLUME_USD_PER_MINUTE: f64 = 5_000.0;

/// Coin launch delay before trading (anti-rug protection)
pub const COIN_LAUNCH_DELAY_SECONDS: u64 = 60; // 1 minute

/// Minimum net profit threshold (SOL)
pub const MIN_NET_PROFIT_SOL: f64 = 0.015; // 0.015 SOL minimum

/// Maximum position size (SOL)
pub const MAX_POSITION_SIZE_SOL: f64 = 0.5;

/// Maximum slippage tolerance (percentage)
pub const MAX_SLIPPAGE_PERCENTAGE: f64 = 5.0;

// ============================================================================
// JITO BUNDLE CONSTANTS
// ============================================================================

/// JITO block engine endpoints
pub const JITO_MAINNET_BLOCK_ENGINE: &str = "https://mainnet.block-engine.jito.wtf";
pub const JITO_NY_BLOCK_ENGINE: &str = "https://ny.mainnet.block-engine.jito.wtf";

/// JITO bundle rate limit (1 bundle per 1.1 seconds)
pub const JITO_RATE_LIMIT_MS: u64 = 1100;

/// Maximum bundle size (transactions)
pub const MAX_BUNDLE_SIZE: usize = 5;

/// Target bundle creation time (milliseconds)
pub const TARGET_BUNDLE_CREATION_MS: u64 = 58;

/// Maximum JITO tip (lamports)
pub const MAX_JITO_TIP_LAMPORTS: u64 = 5_000_000; // 0.005 SOL

/// Baseline JITO tip percentile (99th percentile)
pub const JITO_TIP_BASELINE_PERCENTILE: u8 = 99;

/// JITO tip scaling factors
pub const JITO_TIP_SCALE_HIGH_MARGIN: f64 = 3.0;  // <5% fee ratio
pub const JITO_TIP_SCALE_MED_MARGIN: f64 = 2.0;   // 5-10% fee ratio
pub const JITO_TIP_SCALE_LOW_MARGIN: f64 = 1.0;   // >10% fee ratio

/// JITO tip floor refresh interval (minutes)
pub const JITO_TIP_REFRESH_MINUTES: u64 = 10;

// ============================================================================
// SAFETY & RISK MANAGEMENT CONSTANTS
// ============================================================================

/// Maximum daily loss limit (SOL)
pub const MAX_DAILY_LOSS_SOL: f64 = 5.0;

/// Minimum wallet reserve for fees (SOL)
pub const MIN_WALLET_RESERVE_SOL: f64 = 0.1;

/// Volatility buffer multiplier
pub const VOLATILITY_BUFFER: f64 = 0.8; // 20% buffer

/// Maximum concurrent trades
pub const MAX_CONCURRENT_TRADES: u8 = 3;

/// Circuit breaker failure threshold
pub const CIRCUIT_BREAKER_THRESHOLD: u64 = 5;

/// Circuit breaker reset timeout (seconds)
pub const CIRCUIT_BREAKER_RESET_SECONDS: u64 = 120;

// ============================================================================
// PERFORMANCE & OPTIMIZATION CONSTANTS
// ============================================================================

/// Target end-to-end latency (milliseconds)
pub const TARGET_LATENCY_MS: f64 = 15.0;

/// ShredStream connection timeout (milliseconds)
pub const SHREDSTREAM_TIMEOUT_MS: u64 = 5000;

/// RPC request timeout (milliseconds)
pub const RPC_TIMEOUT_MS: u64 = 5000;

/// Jupiter API request timeout (milliseconds)
pub const JUPITER_TIMEOUT_MS: u64 = 3000;

/// Route cache TTL (seconds)
pub const ROUTE_CACHE_TTL_SECONDS: u64 = 2;

/// Route cache max entries
pub const ROUTE_CACHE_MAX_ENTRIES: usize = 10_000;

/// Token detection cache size
pub const TOKEN_DETECTION_CACHE_SIZE: usize = 10_000;

/// LRU cache eviction size
pub const LRU_CACHE_EVICTION_SIZE: usize = 1000;

// ============================================================================
// RETRY & BACKOFF CONSTANTS
// ============================================================================

/// Maximum retry attempts for failed operations
pub const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Base delay for exponential backoff (milliseconds)
pub const BASE_RETRY_DELAY_MS: u64 = 100;

/// Maximum retry delay (milliseconds)
pub const MAX_RETRY_DELAY_MS: u64 = 30_000;

/// Backoff multiplier
pub const BACKOFF_MULTIPLIER: f64 = 2.0;

/// Jitter factor for retry delays
pub const RETRY_JITTER_FACTOR: f64 = 0.1;

/// RPC-specific retry attempts
pub const RPC_MAX_RETRIES: u32 = 4;

/// Bundle submission retry attempts
pub const BUNDLE_MAX_RETRIES: u32 = 3;

// ============================================================================
// SECURITY CONSTANTS
// ============================================================================

/// PBKDF2 iteration count for key derivation (OWASP recommended)
pub const PBKDF2_ITERATIONS: u32 = 100_000;

/// AES key size (bytes)
pub const AES_KEY_SIZE: usize = 32; // 256 bits

/// AES nonce size (bytes)
pub const AES_NONCE_SIZE: usize = 12; // 96 bits

/// Salt size for key derivation (bytes)
pub const SALT_SIZE: usize = 32; // 256 bits

/// Maximum wallet inactivity days before warning
pub const MAX_WALLET_INACTIVITY_DAYS: i64 = 90;

// ============================================================================
// MONITORING & ALERTING CONSTANTS
// ============================================================================

/// Prometheus metrics port
pub const PROMETHEUS_PORT: u16 = 9090;

/// WebSocket dashboard port
pub const WEBSOCKET_DASHBOARD_PORT: u16 = 8080;

/// Metrics collection interval (seconds)
pub const METRICS_COLLECTION_INTERVAL_SECONDS: u64 = 10;

/// Alert cooldown period (seconds) - prevent alert spam
pub const ALERT_COOLDOWN_SECONDS: u64 = 300; // 5 minutes

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Calculate percentage from basis points
pub const fn bps_to_percentage(bps: u16) -> f64 {
    bps as f64 / 100.0
}

/// Calculate basis points from percentage
pub const fn percentage_to_bps(percentage: f64) -> u16 {
    (percentage * 100.0) as u16
}

/// Create a Duration from milliseconds
pub const fn duration_from_ms(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

// ============================================================================
// VALIDATION FUNCTIONS
// ============================================================================

/// Validate if an amount meets minimum profit threshold
pub fn meets_min_profit(profit_sol: f64) -> bool {
    profit_sol >= MIN_NET_PROFIT_SOL
}

/// Validate if position size is within limits
pub fn is_valid_position_size(size_sol: f64) -> bool {
    size_sol > 0.0 && size_sol <= MAX_POSITION_SIZE_SOL
}

/// Validate if quality score is acceptable
pub fn is_quality_acceptable(score: f64) -> bool {
    score >= MIN_TOKEN_QUALITY_SCORE
}

/// Validate if market cap is within acceptable range
pub fn is_market_cap_acceptable(market_cap_usd: f64) -> bool {
    market_cap_usd > 0.0 && market_cap_usd <= MAX_MARKET_CAP_USD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sol_lamports_conversion() {
        assert_eq!(sol_to_lamports(1.0), SOL_DECIMALS);
        assert_eq!(lamports_to_sol(SOL_DECIMALS), 1.0);
        assert_eq!(sol_to_lamports(0.5), 500_000_000);
    }

    #[test]
    fn test_bps_percentage_conversion() {
        assert_eq!(bps_to_percentage(25), 0.25);
        assert_eq!(percentage_to_bps(0.25), 25);
    }

    #[test]
    fn test_validation_functions() {
        assert!(meets_min_profit(0.02));
        assert!(!meets_min_profit(0.01));

        assert!(is_valid_position_size(0.3));
        assert!(!is_valid_position_size(1.0));

        assert!(is_quality_acceptable(9.0));
        assert!(!is_quality_acceptable(8.0));

        assert!(is_market_cap_acceptable(50000.0));
        assert!(!is_market_cap_acceptable(100000.0));
    }
}
