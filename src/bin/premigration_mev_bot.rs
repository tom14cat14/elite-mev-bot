use anyhow::Result;
use shared_bot_infrastructure::*;
use tracing::{info, error};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🚀 Starting NEW LAUNCH MEV Bot (1-MINUTE SANDWICH WINDOW)");
    info!("💎 Focus: Sandwich early buyers in FIRST 1 MINUTE after ANY launch");
    info!("⚡ Strategy: Catch 2+ SOL buys that move market % - get in/out fast");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Load configuration from environment with detailed error handling
    let config = match SharedConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("❌ Configuration Error: {}", e);
            error!("💡 Check these environment variables:");
            error!("   • SHREDS_ENDPOINT (ShredStream connection)");
            error!("   • JUPITER_API_KEY (Jupiter API access)");
            error!("   • JITO_ENDPOINT (optional, defaults to mainnet.jito.wtf)");
            error!("   • CAPITAL_SOL (optional, defaults to 4.0)");
            error!("   • MIN_PROFIT_SOL (optional, defaults to 0.15)");
            return Err(e);
        }
    };

    // Load additional configurable parameters with validation
    let jito_endpoint = std::env::var("JITO_ENDPOINT")
        .unwrap_or_else(|_| "https://mainnet.jito.wtf".to_string());

    let capital_sol = match std::env::var("CAPITAL_SOL")
        .unwrap_or_else(|_| "4.0".to_string())
        .parse::<f64>() {
        Ok(val) if val > 0.0 => val,
        Ok(_) => {
            error!("❌ CAPITAL_SOL must be > 0, using default 4.0 SOL");
            4.0
        }
        Err(_) => {
            error!("❌ Invalid CAPITAL_SOL format, using default 4.0 SOL");
            4.0
        }
    };

    let min_profit_threshold = match std::env::var("MIN_PROFIT_SOL")
        .unwrap_or_else(|_| "0.15".to_string())
        .parse::<f64>() {
        Ok(val) if val > 0.0 => val,
        Ok(_) => {
            error!("❌ MIN_PROFIT_SOL must be > 0, using default 0.15 SOL");
            0.15
        }
        Err(_) => {
            error!("❌ Invalid MIN_PROFIT_SOL format, using default 0.15 SOL");
            0.15
        }
    };

    info!("✅ Configuration loaded:");
    info!("  • ShredStream: {}", config.shreds_endpoint);
    info!("  • Jupiter API: {}***", &config.jupiter_api_key[..8]);
    info!("  • Jito Endpoint: {}", jito_endpoint);
    info!("  • Capital: {:.2} SOL", capital_sol);
    info!("  • Min Profit: {:.3} SOL", min_profit_threshold);

    // PRE-MIGRATION FOCUSED Configuration - Speed + Volume + Margins
    let mev_config = MonitorConfig {
        enable_sandwich_attacks: true,   // PRIMARY: Only sandwich attacks
        enable_arbitrage: false,         // DISABLED: Focus on speed
        enable_liquidations: false,      // DISABLED: Focus on sandwich
        enable_microcap_filter: true,    // ENABLED: Pre-migration detection
        max_market_cap_usd: Some(1_000_000.0), // Max 1M (catch any new launch)
        max_concurrent_opportunities: 3, // LOW: Focus on quality over quantity
        opportunity_timeout_ms: 400,     // ULTRA-FAST: Speed advantage
        stats_reporting_interval_ms: 10000, // 10 second reports for rapid feedback
    };

    info!("🚀 NEW LAUNCH SANDWICH Configuration:");
    info!("  • Strategy: Sandwich early buyers in 1st minute");
    info!("  • Market cap limit: ANY size <$1M (catch all new launches)");
    info!("  • Launch window: EXACTLY 1 minute maximum");
    info!("  • Target: 2+ SOL buyers that move market %");
    info!("  • Speed advantage: 400ms timeout (front-run everything)");
    info!("  • Quality focus: 3 concurrent max (precise execution)");
    info!("  • Risk profile: HIGH but TIME-LIMITED");

    info!("🎯 1-Minute Sandwich Window:");
    info!("  • Launch window: 0-60 seconds after token creation");
    info!("  • Target buyers: 2+ SOL transactions");
    info!("  • Market impact: Multi-% price movements");
    info!("  • Profit margin: {:.3} SOL minimum (worth the risk)", min_profit_threshold);
    info!("  • Exit: HARD STOP after 1-minute window");
    info!("  • Safety: Volume validation + precise timing");

    // Create pre-migration MEV monitor
    info!("🚀 Initializing pre-migration MEV infrastructure...");
    let mut mev_monitor = match MempoolMonitor::new(
        config.shreds_endpoint,
        config.jupiter_api_key,
        jito_endpoint.clone(), // Configurable Jito endpoint
        mev_config,
    ).await {
        Ok(monitor) => {
            info!("✅ Pre-migration MEV monitor initialized successfully");
            monitor
        }
        Err(e) => {
            error!("❌ MEV Monitor Initialization Error: {}", e);
            error!("💡 Possible causes:");
            error!("   • Invalid ShredStream endpoint or connection failed");
            error!("   • Invalid Jupiter API key or API unreachable");
            error!("   • Invalid Jito endpoint: {}", jito_endpoint);
            error!("   • Network connectivity issues");
            error!("   • Insufficient permissions or rate limiting");
            return Err(e);
        }
    };

    // Display initial statistics
    let stats = mev_monitor.get_stats();
    info!("📊 Initial Pre-migration MEV Status:");
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
    info!("🚀 PRE-MIGRATION MEV Bot is now LIVE!");
    info!("⚡ Speed: 400ms execution targeting");
    info!("📊 Volume: Mandatory trading activity validation");
    info!("💰 Margins: 3x higher profit requirements");
    info!("🎯 Focus: Brand new coins with volume + sandwich speed");
    info!("💡 Press Ctrl+C to stop");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Start monitoring (this runs until shutdown)
    tokio::select! {
        result = mev_monitor.start_monitoring() => {
            match result {
                Ok(()) => info!("✅ Pre-migration MEV monitoring completed successfully"),
                Err(e) => {
                    error!("❌ Pre-migration MEV monitoring failed: {}", e);
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
    info!("📊 Final Pre-migration MEV Statistics:");
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

    // Calculate risk-adjusted metrics
    let avg_profit_per_execution = if final_stats.opportunities_executed > 0 {
        final_stats.total_profit_sol / final_stats.opportunities_executed as f64
    } else {
        0.0
    };
    info!("  • Average profit per execution: {:.4} SOL", avg_profit_per_execution);

    let risk_adjusted_return = final_stats.total_profit_sol / capital_sol * 100.0; // Using configured capital
    info!("  • Risk-adjusted return: {:.2}%", risk_adjusted_return);

    // Generate final performance report
    if let Ok(report) = mev_monitor.generate_performance_report(24).await {
        info!("📋 24-Hour Pre-migration Performance Report:");
        info!("  • Total opportunities: {}", report.total_opportunities);
        info!("  • Total executions: {}", report.total_executions);
        info!("  • Total profit: {:.4} SOL", report.total_profit_sol);
        info!("  • Average execution time: {:.1}ms", report.average_execution_time_ms);
        info!("  • Success rate: {:.1}%", report.success_rate_percent);

        // Calculate quality metrics for pre-migration strategy
        let execution_rate = if report.total_opportunities > 0 {
            (report.total_executions as f64 / report.total_opportunities as f64) * 100.0
        } else {
            0.0
        };
        info!("  • Quality execution rate: {:.1}%", execution_rate);

        if report.total_executions > 0 {
            let avg_profit_per_trade = report.total_profit_sol / report.total_executions as f64;
            info!("  • Average profit per trade: {:.4} SOL", avg_profit_per_trade);

            if avg_profit_per_trade >= min_profit_threshold {
                info!("  ✅ MEETING RISK TARGET: {:.4} SOL > {:.3} SOL minimum", avg_profit_per_trade, min_profit_threshold);
            } else {
                info!("  ⚠️  BELOW RISK TARGET: {:.4} SOL < {:.3} SOL minimum", avg_profit_per_trade, min_profit_threshold);
            }
        }

        // Show sandwich-specific profits
        for (engine, profit) in &report.profit_by_engine {
            if engine == "Sandwich" && *profit > 0.0 {
                info!("  • Pre-migration sandwich profit: {:.4} SOL", profit);
            }
        }
    }

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("👋 Pre-migration MEV Bot shutdown complete");
    info!("💡 Strategy: Speed + Volume + Higher margins for pre-migration focus");

    Ok(())
}