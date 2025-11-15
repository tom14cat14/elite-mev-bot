use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::{debug, info};

/// Real-time metrics dashboard for Elite MEV bot monitoring
#[derive(Debug)]
pub struct MetricsDashboard {
    pub latency_metrics: Arc<Mutex<LatencyMetrics>>,
    pub trading_metrics: Arc<Mutex<TradingMetrics>>,
    pub system_metrics: Arc<Mutex<SystemMetrics>>,
    pub performance_metrics: Arc<Mutex<PerformanceMetrics>>,
    pub alert_system: Arc<Mutex<AlertSystem>>,
    pub config: DashboardConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub update_interval_ms: u64,
    pub history_retention_hours: u32,
    pub enable_prometheus_export: bool,
    pub prometheus_port: u16,
    pub enable_grafana_integration: bool,
    pub alert_thresholds: AlertThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub max_latency_ms: f64,
    pub min_success_rate: f64,
    pub max_loss_sol: f64,
    pub max_consecutive_failures: u32,
    pub min_profit_rate_sol_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    pub detection_latency_us: TimeSeries<f64>,
    pub execution_latency_ms: TimeSeries<f64>,
    pub end_to_end_latency_ms: TimeSeries<f64>,
    pub shredstream_latency_ms: TimeSeries<f64>,
    pub jito_submission_latency_ms: TimeSeries<f64>,
    pub current_percentiles: LatencyPercentiles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingMetrics {
    pub trades_executed: TimeSeries<u64>,
    pub success_rate: TimeSeries<f64>,
    pub profit_loss_sol: TimeSeries<f64>,
    pub cumulative_profit_sol: f64,
    pub daily_pnl_sol: f64,
    pub best_trade_sol: f64,
    pub worst_trade_sol: f64,
    pub active_positions: u32,
    pub failed_executions: TimeSeries<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: TimeSeries<f64>,
    pub memory_usage_mb: TimeSeries<f64>,
    pub network_latency_ms: TimeSeries<f64>,
    pub udp_packet_rate: TimeSeries<f64>,
    pub cache_hit_rate: TimeSeries<f64>,
    pub simd_utilization: TimeSeries<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub new_tokens_detected_per_second: TimeSeries<f64>,
    pub opportunities_per_minute: TimeSeries<f64>,
    pub alpha_capture_rate: TimeSeries<f64>,
    pub competition_analysis: CompetitionMetrics,
    pub velocity_scores: TimeSeries<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionMetrics {
    pub estimated_bot_count: u32,
    pub our_speed_rank: u32,
    pub win_rate_vs_competition: f64,
    pub avg_competitor_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyPercentiles {
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub p99_9_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries<T> {
    pub data: VecDeque<TimePoint<T>>,
    pub max_points: usize,
    pub current_value: T,
    pub min_value: T,
    pub max_value: T,
    pub avg_value: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimePoint<T> {
    pub timestamp: u64, // Unix timestamp in milliseconds
    pub value: T,
}

#[derive(Debug)]
pub struct AlertSystem {
    pub active_alerts: HashMap<String, Alert>,
    pub alert_history: VecDeque<Alert>,
    pub notification_channels: Vec<NotificationChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub triggered_at: SystemTime,
    pub resolved_at: Option<SystemTime>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

#[derive(Debug, Clone)]
pub enum NotificationChannel {
    Console,
    Discord(String),  // Webhook URL
    Slack(String),    // Webhook URL
    Email(String),    // Email address
    Telegram(String), // Bot token and chat ID
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: 1000,    // 1 second updates
            history_retention_hours: 24, // 24 hours of data
            enable_prometheus_export: true,
            prometheus_port: 9090,
            enable_grafana_integration: true,
            alert_thresholds: AlertThresholds {
                max_latency_ms: 20.0,
                min_success_rate: 0.75,
                max_loss_sol: 1.0,
                max_consecutive_failures: 5,
                min_profit_rate_sol_per_hour: 0.5,
            },
        }
    }
}

impl MetricsDashboard {
    pub fn new(config: DashboardConfig) -> Self {
        let max_points = (config.history_retention_hours as u64 * 3600 * 1000
            / config.update_interval_ms) as usize;

        Self {
            latency_metrics: Arc::new(Mutex::new(LatencyMetrics::new(max_points))),
            trading_metrics: Arc::new(Mutex::new(TradingMetrics::new(max_points))),
            system_metrics: Arc::new(Mutex::new(SystemMetrics::new(max_points))),
            performance_metrics: Arc::new(Mutex::new(PerformanceMetrics::new(max_points))),
            alert_system: Arc::new(Mutex::new(AlertSystem::new())),
            config,
        }
    }

    /// Start the metrics collection and dashboard
    pub async fn start_dashboard(&self) -> Result<()> {
        info!("🚀 Starting Elite MEV Bot Metrics Dashboard");
        info!("  • Update interval: {}ms", self.config.update_interval_ms);
        info!(
            "  • Data retention: {} hours",
            self.config.history_retention_hours
        );
        info!(
            "  • Prometheus export: {}",
            self.config.enable_prometheus_export
        );

        // Start metrics collection task
        self.start_metrics_collection().await?;

        // Start alert monitoring
        self.start_alert_monitoring().await?;

        // Start Prometheus exporter if enabled
        if self.config.enable_prometheus_export {
            self.start_prometheus_exporter().await?;
        }

        // Start console dashboard
        self.start_console_dashboard().await?;

        Ok(())
    }

    /// Start real-time metrics collection
    async fn start_metrics_collection(&self) -> Result<()> {
        let latency_metrics = Arc::clone(&self.latency_metrics);
        let trading_metrics = Arc::clone(&self.trading_metrics);
        let system_metrics = Arc::clone(&self.system_metrics);
        let performance_metrics = Arc::clone(&self.performance_metrics);
        let update_interval = self.config.update_interval_ms;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(update_interval));

            loop {
                interval.tick().await;

                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                // Collect and update metrics
                Self::collect_system_metrics(&system_metrics, timestamp).await;
                Self::update_performance_metrics(&performance_metrics, timestamp).await;
                Self::calculate_latency_percentiles(&latency_metrics).await;
            }
        });

        Ok(())
    }

    /// Collect system performance metrics
    async fn collect_system_metrics(system_metrics: &Arc<Mutex<SystemMetrics>>, timestamp: u64) {
        // Collect CPU usage
        let cpu_usage = Self::get_cpu_usage().await;

        // Collect memory usage
        let memory_usage = Self::get_memory_usage().await;

        // Collect network latency
        let network_latency = Self::get_network_latency().await;

        // Update metrics
        {
            let mut metrics = system_metrics.lock().unwrap();
            metrics.cpu_usage_percent.add_point(timestamp, cpu_usage);
            metrics.memory_usage_mb.add_point(timestamp, memory_usage);
            metrics
                .network_latency_ms
                .add_point(timestamp, network_latency);
        }
    }

    /// Update performance metrics
    async fn update_performance_metrics(
        performance_metrics: &Arc<Mutex<PerformanceMetrics>>,
        timestamp: u64,
    ) {
        // Calculate derived metrics
        let tokens_per_second = Self::calculate_token_detection_rate().await;
        let opportunities_per_minute = Self::calculate_opportunity_rate().await;
        let alpha_capture_rate = Self::calculate_alpha_capture_rate().await;

        {
            let mut metrics = performance_metrics.lock().unwrap();
            metrics
                .new_tokens_detected_per_second
                .add_point(timestamp, tokens_per_second);
            metrics
                .opportunities_per_minute
                .add_point(timestamp, opportunities_per_minute);
            metrics
                .alpha_capture_rate
                .add_point(timestamp, alpha_capture_rate);
        }
    }

    /// Calculate latency percentiles
    async fn calculate_latency_percentiles(latency_metrics: &Arc<Mutex<LatencyMetrics>>) {
        let mut metrics = latency_metrics.lock().unwrap();

        // Calculate percentiles for end-to-end latency
        let values: Vec<f64> = metrics
            .end_to_end_latency_ms
            .data
            .iter()
            .map(|point| point.value)
            .collect();

        if !values.is_empty() {
            let mut sorted_values = values;
            sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

            metrics.current_percentiles = LatencyPercentiles {
                p50_ms: Self::percentile(&sorted_values, 0.5),
                p95_ms: Self::percentile(&sorted_values, 0.95),
                p99_ms: Self::percentile(&sorted_values, 0.99),
                p99_9_ms: Self::percentile(&sorted_values, 0.999),
            };
        }
    }

    /// Start alert monitoring
    async fn start_alert_monitoring(&self) -> Result<()> {
        let latency_metrics = Arc::clone(&self.latency_metrics);
        let trading_metrics = Arc::clone(&self.trading_metrics);
        let alert_system = Arc::clone(&self.alert_system);
        let thresholds = self.config.alert_thresholds.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5)); // Check every 5 seconds

            loop {
                interval.tick().await;

                // Check latency alerts
                {
                    let latency = latency_metrics.lock().unwrap();
                    if latency.current_percentiles.p99_ms > thresholds.max_latency_ms {
                        let mut alerts = alert_system.lock().unwrap();
                        alerts.trigger_alert(
                            "high_latency".to_string(),
                            AlertSeverity::Warning,
                            format!(
                                "P99 latency {:.1}ms exceeds threshold {:.1}ms",
                                latency.current_percentiles.p99_ms, thresholds.max_latency_ms
                            ),
                        );
                    }
                }

                // Check trading alerts
                {
                    let trading = trading_metrics.lock().unwrap();
                    if trading.success_rate.current_value < thresholds.min_success_rate {
                        let mut alerts = alert_system.lock().unwrap();
                        alerts.trigger_alert(
                            "low_success_rate".to_string(),
                            AlertSeverity::Critical,
                            format!(
                                "Success rate {:.1}% below threshold {:.1}%",
                                trading.success_rate.current_value * 100.0,
                                thresholds.min_success_rate * 100.0
                            ),
                        );
                    }
                }
            }
        });

        Ok(())
    }

    /// Start console dashboard display
    async fn start_console_dashboard(&self) -> Result<()> {
        let latency_metrics = Arc::clone(&self.latency_metrics);
        let trading_metrics = Arc::clone(&self.trading_metrics);
        let system_metrics = Arc::clone(&self.system_metrics);
        let performance_metrics = Arc::clone(&self.performance_metrics);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5)); // Update every 5 seconds

            loop {
                interval.tick().await;

                // Clear screen and print dashboard
                print!("\x1B[2J\x1B[1;1H"); // ANSI clear screen

                println!("🚀 ELITE MEV BOT v2.1 ULTRA SPEED - REAL-TIME DASHBOARD");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

                // Latency section
                {
                    let latency = latency_metrics.lock().unwrap();
                    println!("⚡ LATENCY METRICS:");
                    println!(
                        "  • Detection:      {:.1}μs avg",
                        latency.detection_latency_us.current_value
                    );
                    println!(
                        "  • Execution:      {:.1}ms avg",
                        latency.execution_latency_ms.current_value
                    );
                    println!(
                        "  • End-to-End:     {:.1}ms avg (P99: {:.1}ms)",
                        latency.end_to_end_latency_ms.current_value,
                        latency.current_percentiles.p99_ms
                    );
                    println!(
                        "  • ShredStream:    {:.1}ms avg",
                        latency.shredstream_latency_ms.current_value
                    );

                    let status = if latency.current_percentiles.p99_ms < 15.0 {
                        "🔥 ELITE"
                    } else if latency.current_percentiles.p99_ms < 25.0 {
                        "✅ EXCELLENT"
                    } else if latency.current_percentiles.p99_ms < 50.0 {
                        "📊 GOOD"
                    } else {
                        "⚠️  SLOW"
                    };
                    println!("  • Status:         {}", status);
                }

                println!();

                // Trading section
                {
                    let trading = trading_metrics.lock().unwrap();
                    println!("💰 TRADING METRICS:");
                    println!(
                        "  • Success Rate:   {:.1}%",
                        trading.success_rate.current_value * 100.0
                    );
                    println!(
                        "  • Total P&L:      {:.3} SOL",
                        trading.cumulative_profit_sol
                    );
                    println!("  • Daily P&L:      {:.3} SOL", trading.daily_pnl_sol);
                    println!("  • Best Trade:     {:.3} SOL", trading.best_trade_sol);
                    println!("  • Active Pos:     {}", trading.active_positions);

                    let trading_status = if trading.success_rate.current_value > 0.8 {
                        "🔥 CRUSHING IT"
                    } else if trading.success_rate.current_value > 0.6 {
                        "✅ PROFITABLE"
                    } else {
                        "⚠️  NEEDS ATTENTION"
                    };
                    println!("  • Status:         {}", trading_status);
                }

                println!();

                // Performance section
                {
                    let performance = performance_metrics.lock().unwrap();
                    println!("🎯 PERFORMANCE METRICS:");
                    println!(
                        "  • Tokens/sec:     {:.1}",
                        performance.new_tokens_detected_per_second.current_value
                    );
                    println!(
                        "  • Opportunities:  {:.1}/min",
                        performance.opportunities_per_minute.current_value
                    );
                    println!(
                        "  • Alpha Capture:  {:.1}%",
                        performance.alpha_capture_rate.current_value * 100.0
                    );
                    println!(
                        "  • Competition:    Rank #{} vs {} bots",
                        performance.competition_analysis.our_speed_rank,
                        performance.competition_analysis.estimated_bot_count
                    );
                }

                println!();

                // System section
                {
                    let system = system_metrics.lock().unwrap();
                    println!("🖥️  SYSTEM METRICS:");
                    println!(
                        "  • CPU Usage:      {:.1}%",
                        system.cpu_usage_percent.current_value
                    );
                    println!(
                        "  • Memory Usage:   {:.1} MB",
                        system.memory_usage_mb.current_value
                    );
                    println!(
                        "  • Cache Hit Rate: {:.1}%",
                        system.cache_hit_rate.current_value * 100.0
                    );
                    println!(
                        "  • SIMD Util:      {:.1}%",
                        system.simd_utilization.current_value * 100.0
                    );
                }

                println!();
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!(
                    "Last updated: {}",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
        });

        Ok(())
    }

    /// Prometheus metrics removed as requested
    async fn start_prometheus_exporter(&self) -> Result<()> {
        info!("📊 Prometheus metrics disabled - using WebSocket dashboard only");
        Ok(())
    }

    /// Record latency measurement
    pub fn record_latency(&self, latency_type: LatencyType, value_ms: f64) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut latency_metrics = self.latency_metrics.lock().unwrap();

        match latency_type {
            LatencyType::Detection => {
                latency_metrics
                    .detection_latency_us
                    .add_point(timestamp, value_ms * 1000.0);
            }
            LatencyType::Execution => {
                latency_metrics
                    .execution_latency_ms
                    .add_point(timestamp, value_ms);
            }
            LatencyType::EndToEnd => {
                latency_metrics
                    .end_to_end_latency_ms
                    .add_point(timestamp, value_ms);
            }
            LatencyType::ShredStream => {
                latency_metrics
                    .shredstream_latency_ms
                    .add_point(timestamp, value_ms);
            }
            LatencyType::JitoSubmission => {
                latency_metrics
                    .jito_submission_latency_ms
                    .add_point(timestamp, value_ms);
            }
        }
    }

    /// Record trading outcome
    pub fn record_trade(&self, success: bool, profit_sol: f64) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut trading_metrics = self.trading_metrics.lock().unwrap();

        // Update trade count
        trading_metrics.trades_executed.add_point(timestamp, 1);

        // Update P&L
        trading_metrics
            .profit_loss_sol
            .add_point(timestamp, profit_sol);
        trading_metrics.cumulative_profit_sol += profit_sol;

        // Update daily P&L (simplified)
        trading_metrics.daily_pnl_sol += profit_sol;

        // Update best/worst trade
        if profit_sol > trading_metrics.best_trade_sol {
            trading_metrics.best_trade_sol = profit_sol;
        }
        if profit_sol < trading_metrics.worst_trade_sol {
            trading_metrics.worst_trade_sol = profit_sol;
        }

        // Update success rate (rolling average)
        let current_rate = trading_metrics.success_rate.current_value;
        let new_rate = if success {
            (current_rate * 0.95) + (1.0 * 0.05)
        } else {
            (current_rate * 0.95) + (0.0 * 0.05)
        };
        trading_metrics.success_rate.add_point(timestamp, new_rate);
    }

    // Helper methods
    async fn get_cpu_usage() -> f64 {
        // Would implement actual CPU usage collection
        50.0 // Mock value
    }

    async fn get_memory_usage() -> f64 {
        // Would implement actual memory usage collection
        1024.0 // Mock value in MB
    }

    async fn get_network_latency() -> f64 {
        // Would implement actual network latency measurement
        25.0 // Mock value in ms
    }

    async fn calculate_token_detection_rate() -> f64 {
        // Would calculate actual token detection rate
        15.0 // Mock value
    }

    async fn calculate_opportunity_rate() -> f64 {
        // Would calculate actual opportunity rate
        8.0 // Mock value
    }

    async fn calculate_alpha_capture_rate() -> f64 {
        // Would calculate actual alpha capture rate
        0.75 // Mock value (75%)
    }

    fn percentile(sorted_values: &[f64], percentile: f64) -> f64 {
        if sorted_values.is_empty() {
            return 0.0;
        }

        let index = (sorted_values.len() as f64 * percentile) as usize;
        let clamped_index = index.min(sorted_values.len() - 1);
        sorted_values[clamped_index]
    }
}

#[derive(Debug, Clone)]
pub enum LatencyType {
    Detection,
    Execution,
    EndToEnd,
    ShredStream,
    JitoSubmission,
}

// Implementation for supporting structures
impl<T> TimeSeries<T>
where
    T: Clone + Default + PartialOrd,
{
    pub fn new(max_points: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_points),
            max_points,
            current_value: T::default(),
            min_value: T::default(),
            max_value: T::default(),
            avg_value: T::default(),
        }
    }

    pub fn add_point(&mut self, timestamp: u64, value: T) {
        // Remove old points if at capacity
        if self.data.len() >= self.max_points {
            self.data.pop_front();
        }

        // Add new point
        self.data.push_back(TimePoint {
            timestamp,
            value: value.clone(),
        });

        // Update current value
        self.current_value = value.clone();

        // Update min/max
        if value < self.min_value || self.data.len() == 1 {
            self.min_value = value.clone();
        }
        if value > self.max_value || self.data.len() == 1 {
            self.max_value = value.clone();
        }

        // Update average (simplified - just use the last value for now)
        self.avg_value = value.clone();
    }
}

// Specialized implementations for numeric types
impl TimeSeries<f64> {
    pub fn calculate_average(&mut self) {
        if !self.data.is_empty() {
            let sum: f64 = self.data.iter().map(|p| p.value).sum();
            self.avg_value = sum / self.data.len() as f64;
        }
    }
}

impl TimeSeries<u64> {
    pub fn calculate_average(&mut self) {
        if !self.data.is_empty() {
            let sum: u64 = self.data.iter().map(|p| p.value).sum();
            self.avg_value = sum / self.data.len() as u64;
        }
    }
}

impl LatencyMetrics {
    pub fn new(max_points: usize) -> Self {
        Self {
            detection_latency_us: TimeSeries::new(max_points),
            execution_latency_ms: TimeSeries::new(max_points),
            end_to_end_latency_ms: TimeSeries::new(max_points),
            shredstream_latency_ms: TimeSeries::new(max_points),
            jito_submission_latency_ms: TimeSeries::new(max_points),
            current_percentiles: LatencyPercentiles {
                p50_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
                p99_9_ms: 0.0,
            },
        }
    }
}

impl TradingMetrics {
    pub fn new(max_points: usize) -> Self {
        Self {
            trades_executed: TimeSeries::new(max_points),
            success_rate: TimeSeries::new(max_points),
            profit_loss_sol: TimeSeries::new(max_points),
            cumulative_profit_sol: 0.0,
            daily_pnl_sol: 0.0,
            best_trade_sol: 0.0,
            worst_trade_sol: 0.0,
            active_positions: 0,
            failed_executions: TimeSeries::new(max_points),
        }
    }
}

impl SystemMetrics {
    pub fn new(max_points: usize) -> Self {
        Self {
            cpu_usage_percent: TimeSeries::new(max_points),
            memory_usage_mb: TimeSeries::new(max_points),
            network_latency_ms: TimeSeries::new(max_points),
            udp_packet_rate: TimeSeries::new(max_points),
            cache_hit_rate: TimeSeries::new(max_points),
            simd_utilization: TimeSeries::new(max_points),
        }
    }
}

impl PerformanceMetrics {
    pub fn new(max_points: usize) -> Self {
        Self {
            new_tokens_detected_per_second: TimeSeries::new(max_points),
            opportunities_per_minute: TimeSeries::new(max_points),
            alpha_capture_rate: TimeSeries::new(max_points),
            competition_analysis: CompetitionMetrics {
                estimated_bot_count: 50,
                our_speed_rank: 5,
                win_rate_vs_competition: 0.8,
                avg_competitor_latency_ms: 35.0,
            },
            velocity_scores: TimeSeries::new(max_points),
        }
    }
}

impl AlertSystem {
    pub fn new() -> Self {
        Self {
            active_alerts: HashMap::new(),
            alert_history: VecDeque::with_capacity(1000),
            notification_channels: vec![NotificationChannel::Console],
        }
    }

    pub fn trigger_alert(&mut self, id: String, severity: AlertSeverity, message: String) {
        let alert = Alert {
            id: id.clone(),
            severity: severity.clone(),
            message: message.clone(),
            triggered_at: SystemTime::now(),
            resolved_at: None,
            metadata: HashMap::new(),
        };

        // Store active alert
        self.active_alerts.insert(id, alert.clone());

        // Add to history
        if self.alert_history.len() >= 1000 {
            self.alert_history.pop_front();
        }
        self.alert_history.push_back(alert);

        // Send notifications
        for channel in &self.notification_channels {
            self.send_notification(channel, &severity, &message);
        }
    }

    fn send_notification(
        &self,
        channel: &NotificationChannel,
        severity: &AlertSeverity,
        message: &str,
    ) {
        match channel {
            NotificationChannel::Console => {
                let prefix = match severity {
                    AlertSeverity::Info => "ℹ️",
                    AlertSeverity::Warning => "⚠️",
                    AlertSeverity::Critical => "🚨",
                    AlertSeverity::Emergency => "🆘",
                };
                eprintln!("{} ALERT: {}", prefix, message);
            }
            NotificationChannel::Discord(_) => {
                // Would implement Discord webhook
                debug!("Discord alert: {}", message);
            }
            _ => {
                // Other notification channels
                debug!("Alert notification: {}", message);
            }
        }
    }
}
