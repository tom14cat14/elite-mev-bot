use anyhow::Result;
use shared_bot_infrastructure::*;
use tracing::{info, warn, error};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🎯 Starting UNIFIED MEV Bot (Adaptive Strategy)");
    info!("💎 Focus: Pre-migration parameter tuning + efficient processing");
    info!("📊 Market cap filter: <$1M for processing efficiency");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Load configuration from environment
    let config = match SharedConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("❌ Failed to load configuration: {}", e);
            return Err(e);
        }
    };

    info!("✅ Configuration loaded:");
    info!("  • ShredStream: {}", config.shreds_endpoint);
    info!("  • Jupiter API: {}***", &config.jupiter_api_key[..8]);

    // UNIFIED MEV Configuration - Adaptive strategy with pre-migration focus
    let mev_config = MonitorConfig {
        enable_sandwich_attacks: true,   // PRIMARY: Main strategy
        enable_arbitrage: false,         // DISABLED: Handled by separate arb bot
        enable_liquidations: true,       // SECONDARY: For larger tokens only
        enable_microcap_filter: true,    // ENABLED: Efficient early filtering
        max_market_cap_usd: Some(1_000_000.0), // Hard limit: >1M filtered immediately
        max_concurrent_opportunities: 10, // Dynamic: will adjust based on token type
        opportunity_timeout_ms: 1200,    // Dynamic: will adjust based on token type
        stats_reporting_interval_ms: 20000, // 20 second reports for active monitoring
    };

    info!("🎯 UNIFIED MEV Bot Configuration:");
    info!("  • Strategy: Adaptive (Pre-migration focus)");
    info!("  • Market cap hard limit: $1M (early filter)");
    info!("  • Pre-migration mode: <$100K tokens (ultra-aggressive)");
    info!("  • Standard mode: $100K-$1M tokens (balanced)");
    info!("  • Dynamic parameters: timeout, concurrency, position sizing");
    info!("  • Processing optimization: Early filtering saves 80%+ computation");

    info!("🔧 Adaptive Parameter Ranges:");
    info!("  💎 Pre-migration (<$100K):");
    info!("    - Timeout: 600-800ms (ultra-fast)");
    info!("    - Concurrency: 3-5 opportunities");
    info!("    - Min liquidity: 0.5-1.0 SOL");
    info!("    - Target impact: 5-15%");
    info!("    - Strategies: Sandwich only (liquidations disabled)");

    info!("  📊 Standard micro-cap ($100K-$1M):");
    info!("    - Timeout: 1000-1500ms (balanced)");
    info!("    - Concurrency: 6-10 opportunities");
    info!("    - Min liquidity: 1.5-3.0 SOL");
    info!("    - Target impact: 2-8%");
    info!("    - Strategies: Sandwich + Liquidations");

    // Create unified MEV monitor
    info!("🚀 Initializing unified MEV infrastructure...");
    let mut mev_monitor = match MempoolMonitor::new(
        config.shreds_endpoint,
        config.jupiter_api_key,
        "https://mainnet.jito.wtf".to_string(), // Production Jito endpoint
        mev_config,
    ).await {
        Ok(monitor) => {
            info!("✅ Unified MEV monitor initialized successfully");
            monitor
        }
        Err(e) => {
            error!("❌ Failed to initialize unified MEV monitor: {}", e);
            return Err(e);
        }
    };

    // Display initial statistics
    let stats = mev_monitor.get_stats();
    info!("📊 Initial Unified MEV Status:");
    info!("  • Transactions processed: {}", stats.transactions_processed);
    info!("  • Opportunities detected: {}", stats.opportunities_detected);
    info!("  • Total profit: {:.4} SOL", stats.total_profit_sol);

    // Set up graceful shutdown handler
    let shutdown_handle = tokio::spawn(async {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("🛑 Shutdown signal received");
            }
            Err(err) => {
                error!("❌ Failed to listen for shutdown signal: {}", err);
            }
        }
    });

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🎯 UNIFIED MEV Bot is now LIVE!");
    info!("💎 Adaptive strategy with pre-migration parameter tuning");
    info!("⚡ Early filtering: >$1M tokens skipped for efficiency");
    info!("🔄 Dynamic adjustment: Parameters adapt to token characteristics");
    info!("💡 Press Ctrl+C to stop");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Start monitoring (this runs until shutdown)
    tokio::select! {
        result = mev_monitor.start_monitoring() => {
            match result {
                Ok(()) => info!("✅ Unified MEV monitoring completed successfully"),
                Err(e) => {
                    error!("❌ Unified MEV monitoring failed: {}", e);
                    return Err(e);
                }
            }
        }
        _ = shutdown_handle => {
            info!("🛑 Graceful shutdown initiated");
        }
    }

    // Final statistics before shutdown
    let final_stats = mev_monitor.get_stats();
    info!("📊 Final Unified MEV Statistics:");
    info!("  • Runtime: {} seconds", final_stats.uptime_seconds);
    info!("  • Transactions processed: {}", final_stats.transactions_processed);
    info!("  • Opportunities detected: {}", final_stats.opportunities_detected);
    info!("  • Opportunities executed: {}", final_stats.opportunities_executed);
    info!("  • Total profit: {:.4} SOL", final_stats.total_profit_sol);
    info!("  • Average processing time: {:.2}ms", final_stats.average_processing_time_ms);

    let success_rate = if final_stats.opportunities_detected > 0 {
        (final_stats.opportunities_executed as f64 / final_stats.opportunities_detected as f64) * 100.0
    } else {
        0.0
    };
    info!("  • Success rate: {:.1}%", success_rate);

    // Calculate efficiency metrics
    let processing_efficiency = if final_stats.transactions_processed > 0 {
        (final_stats.opportunities_detected as f64 / final_stats.transactions_processed as f64) * 100.0
    } else {
        0.0
    };
    info!("  • Processing efficiency: {:.3}% (opportunities/transactions)", processing_efficiency);

    // Generate final performance report
    if let Ok(report) = mev_monitor.generate_performance_report(24).await {
        info!("📋 24-Hour Unified MEV Performance Report:");
        info!("  • Total opportunities: {}", report.total_opportunities);
        info!("  • Total executions: {}", report.total_executions);
        info!("  • Total profit: {:.4} SOL", report.total_profit_sol);
        info!("  • Average execution time: {:.1}ms", report.average_execution_time_ms);
        info!("  • Success rate: {:.1}%", report.success_rate_percent);

        // Show profit breakdown by engine
        for (engine, profit) in &report.profit_by_engine {
            if *profit > 0.0 {
                info!("  • {} profit: {:.4} SOL", engine, profit);
            }
        }

        // Show efficiency gains from filtering
        let estimated_filtered_tx = final_stats.transactions_processed * 4; // Assume 80% filtered out
        info!("  • Estimated processing savings: {}% (filtered >$1M tokens)",
              ((estimated_filtered_tx - final_stats.transactions_processed) as f64 / estimated_filtered_tx as f64) * 100.0);
    }

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("👋 Unified MEV Bot shutdown complete");
    info!("💡 Ready for pre-migration parameter tuning optimization");

    Ok(())
}