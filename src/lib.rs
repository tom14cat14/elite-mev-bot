//! Shared infrastructure for MEV and Arbitrage bots
//!
//! This library provides:
//! - ShredStream integration for real-time Solana data feeds
//! - Jupiter API rate limiting with exponential backoff
//! - Bot coordination for shared resource management
//! - Burst protection and prioritized execution

use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::env;

/// Account update structure for ShredStream data
#[derive(Debug, Clone)]
pub struct AccountUpdate {
    pub pubkey: Pubkey,
    pub account: solana_sdk::account::Account,
    pub slot: u64,
}

/// Configuration for the shared infrastructure
#[derive(Debug, Clone)]
pub struct SharedConfig {
    pub shreds_endpoint: String,
    pub solana_rpc_endpoint: String,
    pub jupiter_api_key: String,
}

pub mod arbitrage_engine;
pub mod blocktime;
pub mod bot_coordinator;
pub mod constants; // NEW: Centralized constants module
pub mod database_tracker;
pub mod dex_parser;
pub mod dex_registry;
pub mod dynamic_fee_model;
pub mod intelligent_failover;
pub mod jito_bundle_manager;
pub mod jupiter_executor;
pub mod jupiter_rate_limiter;
pub mod liquidation_engine;
pub mod mempool_monitor;
pub mod microcap_filter;
pub mod migration_manager;
pub mod pumpfun_executor;
pub mod raydium_pool_state;
pub mod raydium_swap_builder;
pub mod token_decimal_cache;
// Multi-DEX support modules (Raydium CLMM, CPMM, Orca, Meteora, PumpSwap)
pub mod meteora_dlmm_state;
pub mod meteora_dlmm_swap;
pub mod orca_whirlpool_state;
pub mod orca_whirlpool_swap;
pub mod pumpswap_state;
pub mod pumpswap_swap;
pub mod raydium_clmm_state;
pub mod raydium_clmm_swap;
pub mod raydium_cpmm_state;
pub mod raydium_cpmm_swap;
pub mod token_account_manager;
// Unified DEX pool state router
pub mod dex_pool_state;
pub mod dynamic_config_manager;
pub mod market_cap_filter;
pub mod optimized_shred_processor;
pub mod pumpfun_integration;
pub mod pumpfun_new_coin_detector;
pub mod pumpfun_simd_optimizations;
pub mod route_cache;
pub mod sandwich_engine;
pub mod simd_bincode;
pub mod transaction_processor;
pub mod wallet_manager;
pub mod websocket_dashboard;
// pub mod optimized_udp_manager; // Removed old UDP manager
pub mod config_cli;
pub mod error_recovery_manager;
pub mod jito_bundle_client;
pub mod jito_submitter;
pub mod metrics_dashboard;
pub mod monitoring_alerting;
pub mod production_testing_framework;
pub mod realtime_price_monitor;
pub mod secure_wallet_manager;
pub mod shredstream_processor;
pub mod ultra_fast_detector;
pub mod volume_tracker;
// GROK CYCLE 2: Dynamic slippage disabled for MEV speed (user choice)
// pub mod dynamic_slippage;

// Missing module stubs for compilation
pub mod missing_modules;

// MEV-specific sandwich detection
pub mod mev_database_tracker;
pub mod mev_sandwich_detector;
pub mod pool_validator;

// Re-export main types for convenience
pub use arbitrage_engine::{
    ArbitrageEngine, ArbitrageExecution, ArbitrageOpportunity, ArbitrageStats, TokenPair,
};
pub use bot_coordinator::{
    BotCoordinator, BotRegistration, BotType, CoordinatorHandle, CoordinatorStats,
    ExecutionRequest, OpportunityData,
};
pub use database_tracker::{
    DatabaseConfig, DatabaseStats, DatabaseTracker, ExecutionRecord, OpportunityRecord,
    PerformanceReport,
};
pub use dex_parser::{DexSwapParser, SwapInfo};
pub use dex_pool_state::{fetch_pool_state, fetch_pool_state_by_dex, DexPoolState, DexType};
pub use dex_registry::{DexInfo, DexRegistry};
pub use dynamic_fee_model::{DynamicFeeModel, FeeCalculation, ProfitTier};
pub use error_recovery_manager::{
    classify_error, CircuitBreakerState, CircuitState, ErrorRecoveryManager, ErrorType,
    FailureStatistics, RetryPolicy,
};
pub use intelligent_failover::{DataSourceConfig, FailoverStatus, IntelligentFailover};
pub use jito_bundle_manager::{
    AtomicBundle, BundlePerformanceReport, BundleType, JitoBundleManager,
};
pub use jupiter_executor::{
    create_quote_request, ExecutionParams, ExecutionResult, JupiterExecutor,
};
pub use jupiter_rate_limiter::{JupiterRateLimiter, RateLimiterStats};
pub use liquidation_engine::{
    LiquidatablePosition, LiquidationEngine, LiquidationExecution, LiquidationOpportunity,
    LiquidationStats, ProtocolRegistry,
};
pub use market_cap_filter::{
    FilterStats, MarketCapFilter, MarketCapThresholds, ShredStreamTokenFilter, TokenMetrics,
};
pub use mempool_monitor::{
    ExecutionResult as MevExecutionResult, MempoolMonitor, MonitorConfig, MonitorStats,
    OpportunityEvent, StreamData,
};
pub use mev_database_tracker::{MevDatabaseTracker, OverallStats, TodayStats};
pub use microcap_filter::{
    AdaptiveParams, MicroCapFilter, MicroCapOpportunity, PreMigrationParams, TokenInfo,
};
pub use migration_manager::{
    ActivePosition, AlertType, MigrationAlert, MigrationManager, MigrationStats, PositionType,
};
pub use monitoring_alerting::{
    Alert, AlertCondition, AlertRule, AlertSeverity, MonitoringSystem, SystemMetrics,
};
pub use optimized_shred_processor::{
    MEVOpportunity, OpportunityType as ShredOpportunityType, OptimizedShredProcessor,
    PerformanceStats, ProcessingResult,
};
pub use pumpfun_executor::{
    BondingCurveState, PumpFunExecutor, PumpFunSwapParams, PumpFunSwapResult,
};
pub use pumpfun_new_coin_detector::{
    BondingCurveState as DetectorBondingCurveState, DetectionMetrics, DetectorConfig,
    NewTokenEvent, PumpFunNewCoinDetector,
};
pub use pumpfun_simd_optimizations::{
    ParsedBondingCurveState, PumpFunBuyData, PumpFunCreateData, PumpFunInstructionType,
    PumpFunSellData, PumpFunSimdOptimizations,
};
pub use realtime_price_monitor::{run_price_monitoring, RealtimePriceMonitor, TokenPrice};
pub use route_cache::{CacheEfficiencyReport, CacheStats, RouteCache};
pub use sandwich_engine::{
    SandwichEngine, SandwichExecution, SandwichOpportunity, SandwichStats, TradeParams,
    VictimTransaction,
};
pub use shredstream_processor::{ShredStreamEvent, ShredStreamProcessor};
pub use simd_bincode::{SafeSimdBincode, SimdBincode};
pub use token_decimal_cache::{calculate_adjusted_price, TokenDecimalCache};
pub use transaction_processor::{
    ActionType, MevOpportunity, OpportunityType as TxOpportunityType, RequiredAction,
    TransactionInfo, TransactionProcessor,
};
pub use volume_tracker::{SwapRecord, VolumeTracker};
pub use wallet_manager::{TokenBalance, WalletBalanceInfo, WalletManager};

// Re-export ShredStream types
pub use chrono::{DateTime, Utc};
pub use solana_stream_sdk::{CommitmentLevel, ShredstreamClient};

// Core data structures are already defined above

impl SharedConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let shreds_endpoint = env::var("SHREDS_ENDPOINT")
            .unwrap_or_else(|_| "https://shreds-ny6-1.erpc.global".to_string());

        let solana_rpc_endpoint = env::var("SOLANA_RPC_ENDPOINT")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

        let jupiter_api_key = env::var("JUPITER_API_KEY")
            .map_err(|_| anyhow::anyhow!("JUPITER_API_KEY environment variable required"))?;

        Ok(Self {
            shreds_endpoint,
            solana_rpc_endpoint,
            jupiter_api_key,
        })
    }
}

/// Initialize the shared infrastructure
pub async fn initialize_shared_infrastructure() -> Result<(ShredstreamClient, BotCoordinator)> {
    let config = SharedConfig::from_env()?;

    // Initialize ShredStream client
    let shredstream_client = ShredstreamClient::connect(&config.shreds_endpoint).await?;

    // Initialize bot coordinator
    let coordinator = BotCoordinator::new(config.jupiter_api_key);
    coordinator.start().await?;

    Ok((shredstream_client, coordinator))
}
