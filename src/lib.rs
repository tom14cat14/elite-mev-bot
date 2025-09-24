//! Shared infrastructure for MEV and Arbitrage bots
//!
//! This library provides:
//! - ShredStream integration for real-time Solana data feeds
//! - Jupiter API rate limiting with exponential backoff
//! - Bot coordination for shared resource management
//! - Burst protection and prioritized execution

use anyhow::Result;
use std::env;
use solana_sdk::pubkey::Pubkey;

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
pub mod database_tracker;
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
pub mod route_cache;
pub mod sandwich_engine;
pub mod transaction_processor;
pub mod wallet_manager;
pub mod simd_bincode;
pub mod market_cap_filter;
pub mod optimized_shred_processor;
pub mod pumpfun_new_coin_detector;
pub mod pumpfun_simd_optimizations;
pub mod websocket_dashboard;
pub mod pumpfun_integration;
pub mod dynamic_config_manager;
// pub mod optimized_udp_manager; // Removed old UDP manager
pub mod ultra_fast_detector;
pub mod metrics_dashboard;
pub mod shredstream_processor;
pub mod config_cli;
pub mod jito_bundle_client;
pub mod secure_wallet_manager;
pub mod production_testing_framework;
pub mod error_recovery_manager;
pub mod monitoring_alerting;

// Missing module stubs for compilation
pub mod missing_modules;
pub use missing_modules::*;

// Re-export main types for convenience
pub use jupiter_rate_limiter::{JupiterRateLimiter, RateLimiterStats};
pub use jupiter_executor::{JupiterExecutor, ExecutionParams, ExecutionResult, create_quote_request};
pub use pumpfun_executor::{PumpFunExecutor, PumpFunSwapParams, PumpFunSwapResult, BondingCurveState};
pub use migration_manager::{MigrationManager, ActivePosition, PositionType, MigrationAlert, AlertType, MigrationStats};
pub use dynamic_fee_model::{DynamicFeeModel, FeeCalculation, ProfitTier};
pub use dex_registry::{DexRegistry, DexInfo};
pub use wallet_manager::{WalletManager, WalletBalanceInfo, TokenBalance};
pub use transaction_processor::{TransactionProcessor, MevOpportunity, OpportunityType as TxOpportunityType, TransactionInfo, RequiredAction, ActionType};
pub use route_cache::{RouteCache, CacheStats, CacheEfficiencyReport};
pub use jito_bundle_manager::{JitoBundleManager, AtomicBundle, BundleType, BundlePerformanceReport};
pub use sandwich_engine::{SandwichEngine, SandwichOpportunity, SandwichExecution, SandwichStats, VictimTransaction, TradeParams};
pub use arbitrage_engine::{ArbitrageEngine, ArbitrageOpportunity, ArbitrageExecution, ArbitrageStats, TokenPair};
pub use liquidation_engine::{LiquidationEngine, LiquidationOpportunity, LiquidationExecution, LiquidationStats, LiquidatablePosition, ProtocolRegistry};
pub use mempool_monitor::{MempoolMonitor, MonitorConfig, MonitorStats, OpportunityEvent, ExecutionResult as MevExecutionResult, StreamData};
pub use microcap_filter::{MicroCapFilter, MicroCapOpportunity, TokenInfo, AdaptiveParams, PreMigrationParams};
pub use database_tracker::{DatabaseTracker, DatabaseConfig, DatabaseStats, PerformanceReport, OpportunityRecord, ExecutionRecord};
pub use bot_coordinator::{
    BotCoordinator,
    CoordinatorHandle,
    BotRegistration,
    BotType,
    ExecutionRequest,
    OpportunityData,
    CoordinatorStats,
};
pub use intelligent_failover::{IntelligentFailover, FailoverStatus, DataSourceConfig};
pub use simd_bincode::{SimdBincode, SafeSimdBincode};
pub use market_cap_filter::{MarketCapFilter, MarketCapThresholds, TokenMetrics, FilterStats, ShredStreamTokenFilter};
pub use optimized_shred_processor::{OptimizedShredProcessor, ProcessingResult, MEVOpportunity, OpportunityType as ShredOpportunityType, PerformanceStats};
pub use pumpfun_new_coin_detector::{PumpFunNewCoinDetector, DetectorConfig, NewTokenEvent, BondingCurveState as DetectorBondingCurveState, DetectionMetrics};
pub use shredstream_processor::{ShredStreamProcessor, ShredStreamEvent};
pub use pumpfun_simd_optimizations::{PumpFunSimdOptimizations, PumpFunInstructionType, PumpFunCreateData, PumpFunBuyData, PumpFunSellData, ParsedBondingCurveState};
pub use error_recovery_manager::{ErrorRecoveryManager, ErrorType, RetryPolicy, FailureStatistics, CircuitBreakerState, CircuitState, classify_error};
pub use monitoring_alerting::{MonitoringSystem, SystemMetrics, AlertRule, AlertCondition, AlertSeverity, Alert};

// Re-export ShredStream types
pub use solana_stream_sdk::{CommitmentLevel, ShredstreamClient};
pub use chrono::{DateTime, Utc};

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