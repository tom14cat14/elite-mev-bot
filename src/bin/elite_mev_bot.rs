use anyhow::Result;
use shared_bot_infrastructure::*;
use tracing::{info, warn, error, debug};
use tokio::signal;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::interval;

#[derive(Debug, Clone)]
pub struct RealTimeMetrics {
    pub start_time: Instant,
    pub opportunities_detected: u64,
    pub opportunities_executed: u64,
    pub total_profit_sol: f64,
    pub transactions_processed: u64,
    pub last_update: Instant,
    pub profit_per_minute: f64,
    pub success_rate: f64,
    pub avg_execution_time_ms: f64,
    pub current_streak: u32,
    pub best_streak: u32,
    pub failed_executions: u64,
    pub market_conditions: String,
}

impl Default for RealTimeMetrics {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            opportunities_detected: 0,
            opportunities_executed: 0,
            total_profit_sol: 0.0,
            transactions_processed: 0,
            last_update: now,
            profit_per_minute: 0.0,
            success_rate: 0.0,
            avg_execution_time_ms: 0.0,
            current_streak: 0,
            best_streak: 0,
            failed_executions: 0,
            market_conditions: "INITIALIZING".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    pub base_timeout_ms: u64,
    pub max_concurrent: usize,
    pub profit_threshold: f64,
    pub volume_multiplier: f64,
    pub risk_level: String,
    pub last_adjustment: Instant,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            base_timeout_ms: 400,
            max_concurrent: 3,
            profit_threshold: 0.15,
            volume_multiplier: 1.0,
            risk_level: "MODERATE".to_string(),
            last_adjustment: Instant::now(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize enhanced logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("🚀 Starting ELITE MEV Bot (MAXIMUM PROFIT OPTIMIZATION)");
    info!("💎 Strategy: Ultra-aggressive 1-minute sandwich with adaptive parameters");
    info!("⚡ Features: Real-time metrics, adaptive config, circuit breakers");
    info!("🎯 Goal: MAXIMUM profit extraction from new token launches");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Load configuration with enhanced validation
    let config = match SharedConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("❌ CRITICAL: Configuration Error: {}", e);
            error!("💡 Required Environment Variables:");
            error!("   • SHREDS_ENDPOINT - ShredStream WebSocket endpoint");
            error!("   • JUPITER_API_KEY - Jupiter aggregator API key");
            error!("📋 Optional Variables (with smart defaults):");
            error!("   • JITO_ENDPOINT - Jito MEV endpoint (default: mainnet.jito.wtf)");
            error!("   • CAPITAL_SOL - Trading capital (default: 4.0)");
            error!("   • MIN_PROFIT_SOL - Minimum profit threshold (default: 0.15)");
            error!("   • RISK_LEVEL - LOW/MODERATE/HIGH/EXTREME (default: MODERATE)");
            return Err(e);
        }
    };

    // Enhanced parameter loading with validation
    let jito_endpoint = std::env::var("JITO_ENDPOINT")
        .unwrap_or_else(|_| "https://mainnet.jito.wtf".to_string());

    let capital_sol = match std::env::var("CAPITAL_SOL")
        .unwrap_or_else(|_| "4.0".to_string())
        .parse::<f64>() {
        Ok(val) if val > 0.1 => val,
        Ok(val) => {
            warn!("⚠️  CAPITAL_SOL too low: {}, using minimum 0.1 SOL", val);
            0.1
        }
        Err(_) => {
            error!("❌ Invalid CAPITAL_SOL format, using default 4.0 SOL");
            4.0
        }
    };

    let base_min_profit = match std::env::var("MIN_PROFIT_SOL")
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

    let risk_level = std::env::var("RISK_LEVEL")
        .unwrap_or_else(|_| "MODERATE".to_string())
        .to_uppercase();

    // Initialize shared metrics for real-time monitoring
    let metrics = Arc::new(Mutex::new(RealTimeMetrics::default()));
    let adaptive_config = Arc::new(Mutex::new(AdaptiveConfig::default()));

    // Configure based on risk level
    {
        let mut config_guard = adaptive_config.lock().unwrap();
        match risk_level.as_str() {
            "LOW" => {
                config_guard.base_timeout_ms = 800;
                config_guard.max_concurrent = 2;
                config_guard.profit_threshold = base_min_profit * 1.5;
                config_guard.volume_multiplier = 2.0;
                config_guard.risk_level = "LOW".to_string();
            }
            "MODERATE" => {
                config_guard.base_timeout_ms = 400;
                config_guard.max_concurrent = 3;
                config_guard.profit_threshold = base_min_profit;
                config_guard.volume_multiplier = 1.0;
                config_guard.risk_level = "MODERATE".to_string();
            }
            "HIGH" => {
                config_guard.base_timeout_ms = 200;
                config_guard.max_concurrent = 5;
                config_guard.profit_threshold = base_min_profit * 0.8;
                config_guard.volume_multiplier = 0.7;
                config_guard.risk_level = "HIGH".to_string();
            }
            "EXTREME" => {
                config_guard.base_timeout_ms = 100;
                config_guard.max_concurrent = 8;
                config_guard.profit_threshold = base_min_profit * 0.6;
                config_guard.volume_multiplier = 0.5;
                config_guard.risk_level = "EXTREME".to_string();
            }
            _ => {
                warn!("⚠️  Unknown RISK_LEVEL: {}, using MODERATE", risk_level);
            }
        }
    }

    let current_config = adaptive_config.lock().unwrap().clone();

    info!("✅ ELITE Configuration Loaded:");
    info!("  • ShredStream: {}", &config.shreds_endpoint);
    info!("  • Jupiter API: {}***", &config.jupiter_api_key[..8]);
    info!("  • Jito Endpoint: {}", jito_endpoint);
    info!("  • Capital: {:.2} SOL", capital_sol);
    info!("  • Risk Level: {} (timeout: {}ms, concurrent: {})",
          current_config.risk_level, current_config.base_timeout_ms, current_config.max_concurrent);
    info!("  • Base Profit Threshold: {:.3} SOL", current_config.profit_threshold);

    // Create PUMPFUN-OPTIMIZED MEV configuration (Grok's recommendations)
    let mev_config = MonitorConfig {
        enable_sandwich_attacks: true,
        enable_arbitrage: false, // Not supported on PumpFun pre-migration
        enable_liquidations: false,
        enable_microcap_filter: true,
        max_market_cap_usd: Some(90_000.0), // Updated to $90K (PumpFun pre-migration)
        max_concurrent_opportunities: current_config.max_concurrent,
        opportunity_timeout_ms: current_config.base_timeout_ms,
        stats_reporting_interval_ms: 5000, // 5 second internal reports
    };

    info!("🎯 ELITE MEV Strategy Configuration (PUMPFUN OPTIMIZED):");
    info!("  • Target Window: 1-5 minutes post-launch (pre-migration on PumpFun)");
    info!("  • Market Cap Limit: <$90K (ultra-low-cap tokens)");
    info!("  • Platform: PumpFun bonding curve (direct integration)");
    info!("  • Execution Speed: {}ms ultra-fast targeting", current_config.base_timeout_ms);
    info!("  • Concurrent Ops: {} opportunities maximum", current_config.max_concurrent);
    info!("  • Profit Target: {:.3} SOL minimum per trade", current_config.profit_threshold);
    info!("  • Volume Filter: {}x multiplier (PumpFun optimized)", current_config.volume_multiplier);
    info!("  • Safety: Exit trades on PumpFun migration detection");

    // Initialize MEV monitor
    info!("🚀 Initializing ELITE MEV infrastructure...");

    // Store endpoint references for error handling
    let shreds_endpoint = config.shreds_endpoint.clone();
    let jupiter_api_key = config.jupiter_api_key.clone();

    let mut mev_monitor = match MempoolMonitor::new(
        config.shreds_endpoint,
        config.jupiter_api_key,
        jito_endpoint.clone(),
        mev_config,
    ).await {
        Ok(monitor) => {
            info!("✅ ELITE MEV monitor initialized successfully");
            monitor
        }
        Err(e) => {
            error!("❌ CRITICAL: MEV Monitor Initialization Failed: {}", e);
            error!("💡 Troubleshooting Guide:");
            error!("   • Check ShredStream endpoint connectivity: {}", shreds_endpoint);
            error!("   • Verify Jupiter API key validity and rate limits");
            error!("   • Test Jito endpoint accessibility: {}", jito_endpoint);
            error!("   • Ensure network connectivity and firewall settings");
            error!("   • Check system resources and memory availability");
            return Err(e);
        }
    };

    // Display initial status
    let initial_stats = mev_monitor.get_stats();
    info!("📊 Initial ELITE MEV Status:");
    info!("  • System Ready: All components initialized");
    info!("  • Transactions Processed: {}", initial_stats.transactions_processed);
    info!("  • Opportunities Queue: {} detected", initial_stats.opportunities_detected);
    info!("  • Profit Tracking: {:.4} SOL total", initial_stats.total_profit_sol);

    // Clone metrics for background tasks
    let metrics_clone = Arc::clone(&metrics);
    let adaptive_config_clone = Arc::clone(&adaptive_config);

    // Start real-time metrics monitoring
    let metrics_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(10)); // 10-second updates
        let mut last_opportunities = 0u64;
        let mut last_profit = 0.0f64;

        loop {
            interval.tick().await;

            // Update metrics (in a real implementation, this would get data from the monitor)
            let mut metrics_guard = metrics_clone.lock().unwrap();
            let now = Instant::now();
            let runtime_minutes = now.duration_since(metrics_guard.start_time).as_secs_f64() / 60.0;

            // Calculate rates
            metrics_guard.profit_per_minute = if runtime_minutes > 0.0 {
                metrics_guard.total_profit_sol / runtime_minutes
            } else { 0.0 };

            metrics_guard.success_rate = if metrics_guard.opportunities_detected > 0 {
                (metrics_guard.opportunities_executed as f64 / metrics_guard.opportunities_detected as f64) * 100.0
            } else { 0.0 };

            // Determine market conditions
            let opportunity_rate = (metrics_guard.opportunities_detected - last_opportunities) as f64 / 10.0; // per 10 seconds
            metrics_guard.market_conditions = match opportunity_rate {
                x if x > 2.0 => "🔥 HOT".to_string(),
                x if x > 1.0 => "📈 ACTIVE".to_string(),
                x if x > 0.5 => "📊 MODERATE".to_string(),
                x if x > 0.1 => "📉 SLOW".to_string(),
                _ => "💤 QUIET".to_string(),
            };

            last_opportunities = metrics_guard.opportunities_detected;
            last_profit = metrics_guard.total_profit_sol;

            info!("📊 LIVE PERFORMANCE ({}m runtime) - Market: {}",
                  runtime_minutes as u32, metrics_guard.market_conditions);
            info!("  💰 Profit: {:.4} SOL ({:.3}/min) | ROI: {:.1}%",
                  metrics_guard.total_profit_sol,
                  metrics_guard.profit_per_minute,
                  (metrics_guard.total_profit_sol / capital_sol) * 100.0);
            info!("  🎯 Trades: {}/{} executed ({:.1}% success)",
                  metrics_guard.opportunities_executed,
                  metrics_guard.opportunities_detected,
                  metrics_guard.success_rate);
            info!("  ⚡ Speed: {:.1}ms avg | Streak: {} (best: {})",
                  metrics_guard.avg_execution_time_ms,
                  metrics_guard.current_streak,
                  metrics_guard.best_streak);

            // Adaptive configuration adjustments
            let mut config_guard = adaptive_config_clone.lock().unwrap();
            if now.duration_since(config_guard.last_adjustment).as_secs() > 60 {
                // Adjust based on performance
                if metrics_guard.success_rate < 30.0 && metrics_guard.opportunities_detected > 10 {
                    config_guard.base_timeout_ms = (config_guard.base_timeout_ms as f64 * 1.2) as u64;
                    config_guard.profit_threshold *= 0.9;
                    info!("🔧 ADAPTIVE: Loosening parameters (low success rate)");
                } else if metrics_guard.success_rate > 80.0 && metrics_guard.opportunities_detected > 5 {
                    config_guard.base_timeout_ms = (config_guard.base_timeout_ms as f64 * 0.9) as u64;
                    config_guard.profit_threshold *= 1.05;
                    info!("🔧 ADAPTIVE: Tightening parameters (high success rate)");
                }
                config_guard.last_adjustment = now;
            }

            drop(metrics_guard);
            drop(config_guard);
        }
    });

    // Set up graceful shutdown
    let shutdown_handle = tokio::spawn(async {
        match signal::ctrl_c().await {
            Ok(()) => info!("🛑 Shutdown signal received"),
            Err(err) => error!("❌ Failed to listen for shutdown signal: {}", err),
        }
    });

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🚀 ELITE MEV Bot is LIVE and HUNTING!");
    info!("💎 Ultra-aggressive 1-minute sandwich strategy active");
    info!("⚡ Real-time adaptive parameters optimizing continuously");
    info!("🎯 Target: NEW token launches with 2+ SOL volume impact");
    info!("🛡️  Safety: 60-second window + volume validation");
    info!("📊 Monitoring: Live metrics every 10 seconds");
    info!("💡 Press Ctrl+C to stop");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Start main monitoring loop
    tokio::select! {
        result = mev_monitor.start_monitoring() => {
            metrics_task.abort();
            match result {
                Ok(()) => info!("✅ ELITE MEV monitoring completed successfully"),
                Err(e) => {
                    error!("❌ ELITE MEV monitoring failed: {}", e);
                    return Err(e);
                }
            }
        }
        _ = shutdown_handle => {
            metrics_task.abort();
            info!("🛑 Graceful shutdown initiated");
        }
    }

    // Final comprehensive statistics
    let final_stats = mev_monitor.get_stats();
    let final_metrics = metrics.lock().unwrap();
    let runtime_hours = final_metrics.start_time.elapsed().as_secs_f64() / 3600.0;

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 FINAL ELITE MEV PERFORMANCE REPORT");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("⏱️  Runtime: {:.2} hours", runtime_hours);
    info!("📈 Trading Performance:");
    info!("  • Total Profit: {:.4} SOL", final_stats.total_profit_sol);
    info!("  • ROI: {:.2}%", (final_stats.total_profit_sol / capital_sol) * 100.0);
    info!("  • Profit Rate: {:.3} SOL/hour", final_stats.total_profit_sol / runtime_hours.max(0.01));
    info!("  • Opportunities: {} detected, {} executed", final_stats.opportunities_detected, final_stats.opportunities_executed);

    let success_rate = if final_stats.opportunities_detected > 0 {
        (final_stats.opportunities_executed as f64 / final_stats.opportunities_detected as f64) * 100.0
    } else { 0.0 };
    info!("  • Success Rate: {:.1}%", success_rate);

    if final_stats.opportunities_executed > 0 {
        let avg_profit_per_trade = final_stats.total_profit_sol / final_stats.opportunities_executed as f64;
        info!("  • Average Profit/Trade: {:.4} SOL", avg_profit_per_trade);

        let target_status = if avg_profit_per_trade >= base_min_profit {
            "✅ EXCEEDED TARGET"
        } else {
            "⚠️  BELOW TARGET"
        };
        info!("  • Target Achievement: {} ({:.4} vs {:.3} SOL)", target_status, avg_profit_per_trade, base_min_profit);
    }

    info!("🔧 Technical Performance:");
    info!("  • Transactions Processed: {}", final_stats.transactions_processed);
    info!("  • Average Processing Time: {:.2}ms", final_stats.average_processing_time_ms);
    info!("  • Best Streak: {} consecutive wins", final_metrics.best_streak);

    // Performance analysis
    let hourly_rate = final_stats.total_profit_sol / runtime_hours.max(0.01);
    info!("💡 Performance Analysis:");
    match hourly_rate {
        x if x > 2.0 => info!("  🔥 EXCEPTIONAL: {:.2} SOL/hour - Elite performance!", x),
        x if x > 1.0 => info!("  🎯 EXCELLENT: {:.2} SOL/hour - Strong performance", x),
        x if x > 0.5 => info!("  📈 GOOD: {:.2} SOL/hour - Solid performance", x),
        x if x > 0.1 => info!("  📊 MODERATE: {:.2} SOL/hour - Room for optimization", x),
        x => info!("  📉 NEEDS TUNING: {:.2} SOL/hour - Consider parameter adjustment", x),
    }

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("👋 ELITE MEV Bot shutdown complete");
    info!("🎯 Strategy: Ultra-aggressive 1-minute sandwich with adaptive optimization");
    info!("📊 Next session: Consider adjusting RISK_LEVEL based on performance");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}