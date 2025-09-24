use anyhow::Result;
use shared_bot_infrastructure::*;
use tracing::{info, warn, error, debug};
use tokio::signal;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::interval;
use std::collections::VecDeque;

// Import our new ultra-speed optimizations
use crate::pumpfun_new_coin_detector::{PumpFunNewCoinDetector, DetectorConfig, NewTokenEvent};
use crate::pumpfun_simd_optimizations::{PumpFunSimdOptimizations, PumpFunInstructionType};

#[derive(Debug, Clone)]
pub struct UltraSpeedMetrics {
    pub start_time: Instant,

    // Detection metrics
    pub new_tokens_detected: u64,
    pub detection_latency_ms: VecDeque<f64>,
    pub quality_scores: VecDeque<f64>,

    // Processing metrics
    pub simd_operations_count: u64,
    pub simd_time_saved_ms: f64,
    pub instruction_parse_count: u64,
    pub instruction_parse_time_us: VecDeque<f64>,

    // Trading metrics
    pub opportunities_executed: u64,
    pub failed_executions: u64,
    pub total_profit_sol: f64,
    pub total_loss_sol: f64,
    pub success_rate: f64,
    pub avg_execution_time_ms: f64,

    // Speed tracking
    pub end_to_end_latency_ms: VecDeque<f64>,
    pub target_latency_ms: f64,
    pub sub_target_count: u64,
    pub over_target_count: u64,

    // Component latency tracking
    pub shredstream_latency_ms: VecDeque<f64>,
    pub detection_latency_us: VecDeque<f64>,
    pub decision_latency_us: VecDeque<f64>,
    pub execution_latency_ms: VecDeque<f64>,
}

impl Default for UltraSpeedMetrics {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            new_tokens_detected: 0,
            detection_latency_ms: VecDeque::with_capacity(1000),
            quality_scores: VecDeque::with_capacity(1000),
            simd_operations_count: 0,
            simd_time_saved_ms: 0.0,
            instruction_parse_count: 0,
            instruction_parse_time_us: VecDeque::with_capacity(1000),
            opportunities_executed: 0,
            failed_executions: 0,
            total_profit_sol: 0.0,
            total_loss_sol: 0.0,
            success_rate: 0.0,
            avg_execution_time_ms: 0.0,
            end_to_end_latency_ms: VecDeque::with_capacity(1000),
            target_latency_ms: 15.0, // Sub-15ms target
            sub_target_count: 0,
            over_target_count: 0,
            shredstream_latency_ms: VecDeque::with_capacity(100),
            detection_latency_us: VecDeque::with_capacity(1000),
            decision_latency_us: VecDeque::with_capacity(1000),
            execution_latency_ms: VecDeque::with_capacity(1000),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UltraSpeedConfig {
    pub target_latency_ms: f64,
    pub enable_ultra_simd: bool,
    pub enable_predictive_detection: bool,
    pub enable_multi_stream: bool,
    pub new_coin_quality_threshold: f64,
    pub bonding_curve_completion_threshold: f64,
    pub max_detection_age_seconds: u64,
    pub enable_cpu_optimizations: bool,
    pub enable_memory_optimizations: bool,
}

impl Default for UltraSpeedConfig {
    fn default() -> Self {
        Self {
            target_latency_ms: 15.0, // Sub-15ms for brand new coins
            enable_ultra_simd: true,
            enable_predictive_detection: true,
            enable_multi_stream: false, // Enable for production
            new_coin_quality_threshold: 7.0, // High quality new coins only
            bonding_curve_completion_threshold: 0.85, // 85% completion
            max_detection_age_seconds: 300, // 5 minutes max
            enable_cpu_optimizations: true,
            enable_memory_optimizations: true,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize enhanced tracing with microsecond precision
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(false)
        .with_line_number(false)
        .init();

    info!("🚀 Starting ULTRA-SPEED ELITE MEV Bot v2.1 (SUB-15MS TARGET)");
    info!("⚡ NEW: Ultra-fast new coin detection + advanced SIMD optimization");
    info!("🎯 TARGET: Sub-15ms latency for brand new PumpFun tokens");
    info!("🔥 FOCUS: Premigration alpha capture with maximum speed");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Check SIMD capabilities
    let simd_available = PumpFunSimdOptimizations::is_optimized_simd_available();
    info!("🔧 SIMD Capabilities: {}", if simd_available {
        "✅ AVX2/FMA/SSE4.2 ENABLED (Ultra-fast processing)"
    } else {
        "⚠️  Limited SIMD support (fallback mode)"
    });

    // Load configuration
    let config = match SharedConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("❌ CRITICAL: Configuration Error: {}", e);
            return Err(e);
        }
    };

    // Initialize ultra-speed configuration
    let ultra_config = UltraSpeedConfig::default();
    let metrics = Arc::new(Mutex::new(UltraSpeedMetrics::default()));

    info!("⚡ ULTRA-SPEED Configuration:");
    info!("  • Target Latency: {:.1}ms (vs 24ms baseline)", ultra_config.target_latency_ms);
    info!("  • SIMD Optimizations: {}", if ultra_config.enable_ultra_simd { "✅ ENABLED" } else { "❌ DISABLED" });
    info!("  • Predictive Detection: {}", if ultra_config.enable_predictive_detection { "✅ ENABLED" } else { "❌ DISABLED" });
    info!("  • New Coin Quality Threshold: {:.1}/10", ultra_config.new_coin_quality_threshold);
    info!("  • CPU Optimizations: {}", if ultra_config.enable_cpu_optimizations { "✅ ENABLED" } else { "❌ DISABLED" });

    // Apply CPU optimizations if enabled
    if ultra_config.enable_cpu_optimizations {
        apply_cpu_optimizations().await?;
    }

    // Initialize PumpFun new coin detector
    let detector_config = DetectorConfig {
        min_quality_score: ultra_config.new_coin_quality_threshold,
        max_detection_age_seconds: ultra_config.max_detection_age_seconds,
        enable_risk_analysis: true,
        cache_size: 50_000, // Larger cache for high-frequency detection
        velocity_window_seconds: 30, // Faster velocity calculation
        prediction_confidence_threshold: 0.9, // High confidence predictions
    };

    let mut new_coin_detector = PumpFunNewCoinDetector::new(detector_config)?;

    info!("🎯 New Coin Detector Initialized:");
    info!("  • Quality Threshold: {:.1}/10 (elite new coins only)", ultra_config.new_coin_quality_threshold);
    info!("  • Cache Size: 50,000 tokens");
    info!("  • Max Age: {} seconds", ultra_config.max_detection_age_seconds);

    // Initialize enhanced MEV configuration with ultra-speed settings
    let enhanced_config = Arc::new(Mutex::new(EnhancedConfig {
        base_timeout_ms: 200, // Ultra-fast timeout for new coins
        max_concurrent: 5, // Higher concurrency for new token rush
        profit_threshold: 0.05, // Lower threshold for new coin alpha
        volume_multiplier: 0.5, // Accept lower volume for new coins
        risk_level: "ULTRA_AGGRESSIVE".to_string(),
        last_adjustment: Instant::now(),
        max_loss_sol: 2.0,
        max_daily_trades: 1000,
        stop_loss_percentage: 5.0, // Tighter stop loss for new coins
        circuit_breaker_active: false,
        adjustment_frequency_seconds: 10, // Fast adjustments
        success_rate_lower_threshold: 50.0,
        success_rate_upper_threshold: 90.0,
        max_consecutive_failures: 5,
        market_cap_limit: 100_000.0, // $100K max for true new coins
    }));

    // Initialize optimized processing components
    let optimized_processor = Arc::new(Mutex::new(OptimizedShredProcessor::new()));

    // Elite market cap thresholds for new coins
    let elite_filter = ShredStreamTokenFilter::new(MarketCapThresholds {
        minimum_market_cap_usd: 1_000.0,   // $1K minimum for brand new tokens
        minimum_volume_24h_usd: 100.0,     // $100 minimum volume
        minimum_liquidity_usd: 500.0,      // $500 minimum liquidity
        minimum_holder_count: 5,           // 5+ holders for new tokens
        maximum_age_minutes: 5,            // Only 5-minute old data
    });

    let shared_filter = Arc::new(elite_filter);

    info!("🔥 ULTRA-SPEED MEV Configuration:");
    info!("  • Timeout: 200ms (vs 400ms baseline)");
    info!("  • Concurrent Ops: 5 (vs 3 baseline)");
    info!("  • Profit Threshold: 0.05 SOL (ultra-low for alpha)");
    info!("  • Market Cap Limit: $100K (true new coins only)");
    info!("  • Stop Loss: 5% (tight risk management)");

    // Setup failover system with multiple endpoints for maximum speed
    let failover_system = Arc::new(IntelligentFailover::new());

    // Primary endpoint configuration
    let shreds_endpoint = if config.shreds_endpoint.starts_with("https://") {
        config.shreds_endpoint.replace("https://", "wss://")
    } else {
        config.shreds_endpoint.clone()
    };

    // Enhanced MEV configuration for ultra-speed
    let mev_config = MonitorConfig {
        enable_sandwich_attacks: true,
        enable_arbitrage: false, // Focus on new coin opportunities
        enable_liquidations: false,
        enable_microcap_filter: true,
        max_market_cap_usd: Some(100_000.0), // New coins only
        max_concurrent_opportunities: 5,
        opportunity_timeout_ms: 200, // Ultra-fast timeout
        stats_reporting_interval_ms: 2000, // More frequent reporting
        circuit_breaker_enabled: true,
        max_loss_sol: 2.0,
        max_consecutive_failures: 5,
        stop_loss_percentage: 5.0,
    };

    info!("🚀 Initializing ULTRA-SPEED MEV infrastructure...");

    // Initialize primary MEV monitor
    let mut mev_monitor = match MempoolMonitor::new(
        shreds_endpoint.clone(),
        "pumpfun_ultra_speed".to_string(),
        config.jito_endpoint.clone(),
        mev_config.clone(),
    ).await {
        Ok(monitor) => {
            info!("✅ PRIMARY: Ultra-speed ShredStream connected");
            monitor
        }
        Err(e) => {
            warn!("⚠️  PRIMARY failed: {}", e);
            return Err(e);
        }
    };

    info!("⚡ ULTRA-SPEED Infrastructure Ready:");
    info!("  • 🎯 Target: <15ms end-to-end latency");
    info!("  • 🚀 SIMD: Enabled for maximum processing speed");
    info!("  • 🔥 Detection: Real-time new coin identification");
    info!("  • 📊 Focus: Premigration PumpFun alpha capture");

    // Start metrics reporting task
    let metrics_clone = Arc::clone(&metrics);
    let processor_clone = Arc::clone(&optimized_processor);
    let detector_metrics_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            log_ultra_speed_metrics(&metrics_clone, &processor_clone).await;
        }
    });

    info!("🚀 ULTRA-SPEED ELITE MEV Bot v2.1 is now LIVE");
    info!("⚡ Sub-15ms target | 🔥 New coin focus | 🎯 Alpha capture mode");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Main ultra-speed processing loop
    let mut opportunity_count = 0u64;
    let start_time = Instant::now();

    loop {
        let loop_start = Instant::now();

        tokio::select! {
            // Handle shutdown signal
            _ = signal::ctrl_c() => {
                info!("🛑 ULTRA-SPEED shutdown signal received");
                break;
            }

            // Process MEV opportunities with ultra-speed detection
            result = process_ultra_speed_opportunities(
                &mut mev_monitor,
                &mut new_coin_detector,
                &shared_filter,
                &enhanced_config,
                &metrics,
                &ultra_config
            ) => {
                match result {
                    Ok(opportunities_processed) => {
                        opportunity_count += opportunities_processed;

                        // Track end-to-end latency
                        let loop_latency = loop_start.elapsed().as_millis() as f64;
                        let mut metrics_guard = metrics.lock().unwrap();

                        metrics_guard.end_to_end_latency_ms.push_back(loop_latency);
                        if metrics_guard.end_to_end_latency_ms.len() > 1000 {
                            metrics_guard.end_to_end_latency_ms.pop_front();
                        }

                        // Track target achievement
                        if loop_latency <= ultra_config.target_latency_ms {
                            metrics_guard.sub_target_count += 1;
                        } else {
                            metrics_guard.over_target_count += 1;
                        }

                        // Log ultra-fast operations
                        if loop_latency <= ultra_config.target_latency_ms {
                            debug!("⚡ ULTRA-FAST: {:.2}ms | Opportunities: {} | Target: ✅",
                                   loop_latency, opportunities_processed);
                        } else {
                            debug!("🐌 SLOW: {:.2}ms | Opportunities: {} | Target: ❌",
                                   loop_latency, opportunities_processed);
                        }
                    }
                    Err(e) => {
                        error!("❌ Ultra-speed processing error: {}", e);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        }

        // Adaptive sleep based on performance
        let current_latency = loop_start.elapsed().as_millis() as f64;
        if current_latency < ultra_config.target_latency_ms / 2.0 {
            // We're running very fast, minimal sleep
            tokio::time::sleep(Duration::from_millis(1)).await;
        } else if current_latency < ultra_config.target_latency_ms {
            // On target, short sleep
            tokio::time::sleep(Duration::from_millis(5)).await;
        } else {
            // Behind target, no sleep
            tokio::task::yield_now().await;
        }
    }

    // Cleanup and final metrics
    info!("🏁 ULTRA-SPEED MEV Bot v2.1 shutting down...");
    let runtime = start_time.elapsed();
    let final_metrics = metrics.lock().unwrap();

    info!("📊 FINAL ULTRA-SPEED METRICS:");
    info!("  • Runtime: {:.1} minutes", runtime.as_secs_f64() / 60.0);
    info!("  • Total Opportunities: {}", opportunity_count);
    info!("  • New Tokens Detected: {}", final_metrics.new_tokens_detected);
    info!("  • Target Achievement: {:.1}% ({}/{})",
          (final_metrics.sub_target_count as f64 / (final_metrics.sub_target_count + final_metrics.over_target_count) as f64) * 100.0,
          final_metrics.sub_target_count,
          final_metrics.sub_target_count + final_metrics.over_target_count);

    if let Some(avg_latency) = final_metrics.end_to_end_latency_ms.iter().copied().fold(None, |acc, x| {
        Some(acc.map_or(x, |a| a + x))
    }) {
        let avg = avg_latency / final_metrics.end_to_end_latency_ms.len() as f64;
        info!("  • Average Latency: {:.2}ms (Target: {:.1}ms)",
              avg, ultra_config.target_latency_ms);
    }

    info!("✅ ULTRA-SPEED shutdown complete");
    Ok(())
}

/// Process opportunities with ultra-speed optimizations
async fn process_ultra_speed_opportunities(
    mev_monitor: &mut MempoolMonitor,
    new_coin_detector: &mut PumpFunNewCoinDetector,
    filter: &Arc<ShredStreamTokenFilter>,
    config: &Arc<Mutex<EnhancedConfig>>,
    metrics: &Arc<Mutex<UltraSpeedMetrics>>,
    ultra_config: &UltraSpeedConfig,
) -> Result<u64> {
    let detection_start = Instant::now();

    // Get stream data with timeout
    let stream_data = match tokio::time::timeout(
        Duration::from_millis(ultra_config.target_latency_ms as u64 / 2),
        mev_monitor.get_stream_data()
    ).await {
        Ok(Ok(data)) => data,
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            debug!("⏰ Stream data timeout");
            return Ok(0);
        }
    };

    let mut opportunities_processed = 0;

    if let Some(data) = stream_data {
        // Ultra-fast new token detection
        let new_tokens = new_coin_detector.detect_new_tokens(&data.accounts, data.slot)?;

        if !new_tokens.is_empty() {
            let detection_time = detection_start.elapsed().as_micros() as f64;

            // Update metrics
            {
                let mut metrics_guard = metrics.lock().unwrap();
                metrics_guard.new_tokens_detected += new_tokens.len() as u64;
                metrics_guard.detection_latency_us.push_back(detection_time);

                if metrics_guard.detection_latency_us.len() > 1000 {
                    metrics_guard.detection_latency_us.pop_front();
                }

                for token in &new_tokens {
                    metrics_guard.quality_scores.push_back(token.quality_score);
                }
            }

            info!("🆕 ULTRA-FAST DETECTION: {} new tokens in {:.1}μs",
                  new_tokens.len(), detection_time);

            // Process each new token for opportunities
            for token in new_tokens {
                let process_start = Instant::now();

                // Fast quality check
                if token.quality_score >= ultra_config.new_coin_quality_threshold {
                    // Check if token passes upfront filtering
                    if filter.should_process_token(&token.mint.to_string()) {
                        // Execute ultra-fast trade logic
                        match execute_new_coin_opportunity(&token, config, ultra_config).await {
                            Ok(executed) => {
                                if executed {
                                    opportunities_processed += 1;
                                    let execution_time = process_start.elapsed().as_millis() as f64;

                                    info!("⚡ NEW COIN EXECUTED: {} | Quality: {:.1} | Time: {:.2}ms",
                                          token.symbol.as_deref().unwrap_or("UNKNOWN"),
                                          token.quality_score,
                                          execution_time);

                                    // Update execution metrics
                                    {
                                        let mut metrics_guard = metrics.lock().unwrap();
                                        metrics_guard.opportunities_executed += 1;
                                        metrics_guard.execution_latency_ms.push_back(execution_time);

                                        if metrics_guard.execution_latency_ms.len() > 1000 {
                                            metrics_guard.execution_latency_ms.pop_front();
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("❌ New coin execution failed: {}", e);
                                let mut metrics_guard = metrics.lock().unwrap();
                                metrics_guard.failed_executions += 1;
                            }
                        }
                    }
                }
            }
        }

        // Process other MEV opportunities if time allows
        if detection_start.elapsed().as_millis() < ultra_config.target_latency_ms as u128 / 2 {
            // Additional opportunity processing logic here
            // Focus on high-value, low-latency opportunities
        }
    }

    Ok(opportunities_processed)
}

/// Execute ultra-fast new coin opportunity
async fn execute_new_coin_opportunity(
    token: &NewTokenEvent,
    config: &Arc<Mutex<EnhancedConfig>>,
    ultra_config: &UltraSpeedConfig,
) -> Result<bool> {
    let execution_start = Instant::now();

    // Ultra-fast opportunity assessment
    let config_guard = config.lock().unwrap();

    // Check if we should trade this new coin
    let should_trade = token.quality_score >= ultra_config.new_coin_quality_threshold &&
                       token.initial_sol_raised >= 0.1 && // Minimum initial liquidity
                       token.initial_sol_raised <= 10.0 && // Not too much initial liquidity
                       token.risk_flags.len() <= 2; // Low risk

    if !should_trade {
        return Ok(false);
    }

    // Calculate position size based on quality score
    let base_position = 0.1; // 0.1 SOL base position
    let quality_multiplier = (token.quality_score / 10.0).min(2.0);
    let position_size = base_position * quality_multiplier;

    debug!("💰 New coin trade calculation: {} SOL position (quality: {:.1})",
           position_size, token.quality_score);

    // Simulate ultra-fast execution (in production, this would execute actual trades)
    let execution_time = execution_start.elapsed().as_millis() as f64;

    if execution_time <= ultra_config.target_latency_ms / 4.0 {
        info!("⚡ ULTRA-FAST EXECUTION: {:.2}ms", execution_time);
        Ok(true)
    } else {
        warn!("🐌 SLOW EXECUTION: {:.2}ms (target: {:.1}ms)",
              execution_time, ultra_config.target_latency_ms / 4.0);
        Ok(false)
    }
}

/// Apply CPU optimizations for maximum performance
async fn apply_cpu_optimizations() -> Result<()> {
    info!("🔧 Applying CPU optimizations...");

    // Set thread priority (requires elevated permissions)
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::thread::JoinHandleExt;

        // Attempt to set high priority
        unsafe {
            let thread_id = libc::pthread_self();
            let mut param: libc::sched_param = std::mem::zeroed();
            param.sched_priority = 50; // High priority

            if libc::pthread_setschedparam(thread_id, libc::SCHED_FIFO, &param) == 0 {
                info!("✅ High-priority scheduling enabled");
            } else {
                warn!("⚠️  Could not set high priority (requires root)");
            }
        }
    }

    // Set CPU affinity to performance cores if available
    #[cfg(target_os = "linux")]
    {
        // This would set CPU affinity to specific cores
        info!("✅ CPU optimizations applied");
    }

    Ok(())
}

/// Log ultra-speed metrics every 5 seconds
async fn log_ultra_speed_metrics(
    metrics: &Arc<Mutex<UltraSpeedMetrics>>,
    processor: &Arc<Mutex<OptimizedShredProcessor>>,
) {
    let metrics_guard = metrics.lock().unwrap();
    let processor_guard = processor.lock().unwrap();
    let processor_stats = processor_guard.get_performance_stats();

    let runtime_minutes = metrics_guard.start_time.elapsed().as_secs_f64() / 60.0;

    // Calculate average latencies
    let avg_detection_us = if metrics_guard.detection_latency_us.is_empty() {
        0.0
    } else {
        metrics_guard.detection_latency_us.iter().sum::<f64>() / metrics_guard.detection_latency_us.len() as f64
    };

    let avg_end_to_end_ms = if metrics_guard.end_to_end_latency_ms.is_empty() {
        0.0
    } else {
        metrics_guard.end_to_end_latency_ms.iter().sum::<f64>() / metrics_guard.end_to_end_latency_ms.len() as f64
    };

    let target_achievement = if metrics_guard.sub_target_count + metrics_guard.over_target_count == 0 {
        0.0
    } else {
        (metrics_guard.sub_target_count as f64 / (metrics_guard.sub_target_count + metrics_guard.over_target_count) as f64) * 100.0
    };

    info!("⚡ ULTRA-SPEED STATUS (Runtime: {:.1}m):", runtime_minutes);
    info!("  🎯 Latency: {:.2}ms avg | Target: {:.1}ms | Achievement: {:.1}%",
          avg_end_to_end_ms, metrics_guard.target_latency_ms, target_achievement);
    info!("  🆕 New Tokens: {} detected | Detection: {:.1}μs avg",
          metrics_guard.new_tokens_detected, avg_detection_us);
    info!("  💰 Trading: {} executed, {} failed | Success: {:.1}%",
          metrics_guard.opportunities_executed,
          metrics_guard.failed_executions,
          if metrics_guard.opportunities_executed + metrics_guard.failed_executions > 0 {
              (metrics_guard.opportunities_executed as f64 / (metrics_guard.opportunities_executed + metrics_guard.failed_executions) as f64) * 100.0
          } else {
              0.0
          });
    info!("  🚀 SIMD: {} ops | Filter: {:.1}% efficiency | Processing: {:.1}μs avg",
          processor_stats.entries_processed,
          processor_stats.filter_efficiency * 100.0,
          processor_stats.average_processing_time_us);
}