use anyhow::Result;
use shared_bot_infrastructure::*;
use tracing::{info, warn, error};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🥪 Starting MEV Bot (Sandwich + Liquidation)");
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

    // MEV Bot Configuration - Optimized for speed and sandwich attacks with 1M market cap limit
    let mev_config = MonitorConfig {
        enable_sandwich_attacks: true,   // PRIMARY: Sandwich attacks
        enable_arbitrage: false,         // DISABLED: Handled by separate arb bot
        enable_liquidations: true,       // SECONDARY: Liquidation opportunities
        enable_microcap_filter: true,    // ENABLED: Filter for sub-1M tokens
        max_market_cap_usd: Some(1_000_000.0), // Max 1M market cap limit
        max_concurrent_opportunities: 8, // Moderate concurrency for MEV
        opportunity_timeout_ms: 1500,    // Fast timeout for MEV speed
        stats_reporting_interval_ms: 30000, // 30 second reports
    };

    info!("🔧 MEV Bot Configuration:");
    info!("  • Sandwich attacks: ✅ ENABLED");
    info!("  • Arbitrage: ❌ DISABLED (separate arb bot)");
    info!("  • Liquidations: ✅ ENABLED");
    info!("  • Micro-cap filter: ✅ ENABLED (MAX $1M market cap)");
    info!("  • Max concurrent: {}", mev_config.max_concurrent_opportunities);
    info!("  • Opportunity timeout: {}ms", mev_config.opportunity_timeout_ms);
    info!("  • Target: <400ms Solana block time");

    // Create MEV monitor with optimized settings
    info!("🚀 Initializing MEV monitoring infrastructure...");
    let mut mev_monitor = match MempoolMonitor::new(
        config.shreds_endpoint,
        config.jupiter_api_key,
        "https://mainnet.jito.wtf".to_string(), // Production Jito endpoint
        mev_config,
    ).await {
        Ok(monitor) => {
            info!("✅ MEV monitor initialized successfully");
            monitor
        }
        Err(e) => {
            error!("❌ Failed to initialize MEV monitor: {}", e);
            return Err(e);
        }
    };

    // Display initial statistics
    let stats = mev_monitor.get_stats();
    info!("📊 Initial MEV Bot Status:");
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
    info!("🚀 MEV Bot is now LIVE - Monitoring for opportunities...");
    info!("💡 Press Ctrl+C to stop");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Start monitoring (this runs until shutdown)
    tokio::select! {
        result = mev_monitor.start_monitoring() => {
            match result {
                Ok(()) => info!("✅ MEV monitoring completed successfully"),
                Err(e) => {
                    error!("❌ MEV monitoring failed: {}", e);
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
    info!("📊 Final MEV Bot Statistics:");
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

    // Generate final performance report
    if let Ok(report) = mev_monitor.generate_performance_report(24).await {
        info!("📋 24-Hour Performance Report:");
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
    }

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("👋 MEV Bot shutdown complete");

    Ok(())
}