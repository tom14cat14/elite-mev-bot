use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct DataSourceConfig {
    pub name: String,
    pub endpoint: String,
    pub max_latency_ms: f64,
    pub timeout_duration_ms: u64,
    pub is_primary: bool,
}

#[derive(Debug, Clone)]
pub struct FailoverMetrics {
    pub current_source: String,
    pub switch_count: u32,
    pub last_switch_time: Option<Instant>,
    pub shredstream_latency_ms: Vec<f64>,
    pub grpc_latency_ms: Vec<f64>,
    pub shredstream_failures: u32,
    pub grpc_failures: u32,
}

impl Default for FailoverMetrics {
    fn default() -> Self {
        Self {
            current_source: "ShredStream".to_string(),
            switch_count: 0,
            last_switch_time: None,
            shredstream_latency_ms: Vec::with_capacity(100),
            grpc_latency_ms: Vec::with_capacity(100),
            shredstream_failures: 0,
            grpc_failures: 0,
        }
    }
}

pub struct IntelligentFailover {
    primary: DataSourceConfig,
    backup: DataSourceConfig,
    metrics: Arc<Mutex<FailoverMetrics>>,
    stability_window_secs: u64,
}

impl Default for IntelligentFailover {
    fn default() -> Self {
        Self::new()
    }
}

impl IntelligentFailover {
    pub fn new() -> Self {
        let primary = DataSourceConfig {
            name: "ShredStream".to_string(),
            endpoint: "https://shreds-ny6-1.erpc.global".to_string(),
            max_latency_ms: 40.0,      // Grok's threshold
            timeout_duration_ms: 5000, // 5 second timeout
            is_primary: true,
        };

        let backup = DataSourceConfig {
            name: "gRPC".to_string(),
            endpoint: "https://grpc-ny6-1.erpc.global".to_string(),
            max_latency_ms: 50.0, // Slightly higher threshold for backup
            timeout_duration_ms: 5000,
            is_primary: false,
        };

        Self {
            primary,
            backup,
            metrics: Arc::new(Mutex::new(FailoverMetrics::default())),
            stability_window_secs: 10, // Grok's recommendation: 10s stable before switching back
        }
    }

    /// Check if we should switch data sources based on Grok's criteria
    pub fn should_switch_source(&self, current_latency_ms: f64, current_source: &str) -> bool {
        let metrics = self.metrics.lock().unwrap();

        match current_source {
            "ShredStream" => {
                // Switch to gRPC if ShredStream latency >40ms (Grok's threshold)
                if current_latency_ms > self.primary.max_latency_ms {
                    warn!(
                        "🔄 FAILOVER TRIGGER: ShredStream {:.1}ms > {:.1}ms threshold",
                        current_latency_ms, self.primary.max_latency_ms
                    );
                    return true;
                }
            }
            "gRPC" => {
                // Switch back to ShredStream if it's been stable <30ms for 10s
                if let Some(last_switch) = metrics.last_switch_time {
                    if last_switch.elapsed().as_secs() >= self.stability_window_secs {
                        let recent_shred_latency = self.get_recent_avg_latency("ShredStream");
                        if recent_shred_latency < 30.0 && recent_shred_latency > 0.0 {
                            info!(
                                "🔄 REVERT TRIGGER: ShredStream stable at {:.1}ms for {}s",
                                recent_shred_latency, self.stability_window_secs
                            );
                            return true;
                        }
                    }
                }
            }
            _ => {}
        }

        false
    }

    /// Record latency for monitoring and decision making
    pub fn record_latency(&self, source: &str, latency_ms: f64) {
        let mut metrics = self.metrics.lock().unwrap();

        match source {
            "ShredStream" => {
                metrics.shredstream_latency_ms.push(latency_ms);
                if metrics.shredstream_latency_ms.len() > 100 {
                    metrics.shredstream_latency_ms.remove(0);
                }
            }
            "gRPC" => {
                metrics.grpc_latency_ms.push(latency_ms);
                if metrics.grpc_latency_ms.len() > 100 {
                    metrics.grpc_latency_ms.remove(0);
                }
            }
            _ => {}
        }
    }

    /// Execute failover switch with logging
    pub fn execute_failover(&self, from: &str, to: &str, reason: &str) -> String {
        let mut metrics = self.metrics.lock().unwrap();

        metrics.current_source = to.to_string();
        metrics.switch_count += 1;
        metrics.last_switch_time = Some(Instant::now());

        match from {
            "ShredStream" => metrics.shredstream_failures += 1,
            "gRPC" => metrics.grpc_failures += 1,
            _ => {}
        }

        let switch_msg = format!(
            "🔄 FAILOVER #{}: {} → {} | Reason: {}",
            metrics.switch_count, from, to, reason
        );

        if to == "gRPC" {
            warn!("{}", switch_msg);
            warn!("  • Backup active: gRPC (26.77ms avg) - Still ELITE tier performance");
            warn!("  • Monitoring: Will revert when ShredStream <30ms for 10s");
        } else {
            info!("{}", switch_msg);
            info!("  • Primary restored: ShredStream (26.47ms avg) - Optimal performance");
        }

        to.to_string()
    }

    /// Get recent average latency for a source
    pub fn get_recent_avg_latency(&self, source: &str) -> f64 {
        let metrics = self.metrics.lock().unwrap();

        let data = match source {
            "ShredStream" => &metrics.shredstream_latency_ms,
            "gRPC" => &metrics.grpc_latency_ms,
            _ => return 0.0,
        };

        if data.is_empty() {
            0.0
        } else {
            // Get last 10 readings for recent average
            let recent_count = data.len().min(10);
            let recent_data: Vec<f64> = data.iter().rev().take(recent_count).cloned().collect();
            recent_data.iter().sum::<f64>() / recent_data.len() as f64
        }
    }

    /// Check for timeout conditions (no data for 5s)
    pub fn check_timeout(&self, last_data_time: Instant) -> bool {
        last_data_time.elapsed().as_millis() > self.primary.timeout_duration_ms as u128
    }

    /// Get current failover status for monitoring
    pub fn get_status(&self) -> FailoverStatus {
        let metrics = self.metrics.lock().unwrap();

        FailoverStatus {
            current_source: metrics.current_source.clone(),
            switch_count: metrics.switch_count,
            shredstream_avg_latency: self.calculate_avg(&metrics.shredstream_latency_ms),
            grpc_avg_latency: self.calculate_avg(&metrics.grpc_latency_ms),
            shredstream_failures: metrics.shredstream_failures,
            grpc_failures: metrics.grpc_failures,
            is_primary_active: metrics.current_source == "ShredStream",
        }
    }

    fn calculate_avg(&self, data: &[f64]) -> f64 {
        if data.is_empty() {
            0.0
        } else {
            data.iter().sum::<f64>() / data.len() as f64
        }
    }

    /// Log performance analysis as recommended by Grok
    pub fn log_performance_analysis(&self) {
        let status = self.get_status();

        info!("📊 INTELLIGENT FAILOVER STATUS:");
        info!(
            "  • Current Source: {} (switches: {})",
            status.current_source, status.switch_count
        );
        info!(
            "  • ShredStream: {:.1}ms avg (failures: {})",
            status.shredstream_avg_latency, status.shredstream_failures
        );
        info!(
            "  • gRPC Backup: {:.1}ms avg (failures: {})",
            status.grpc_avg_latency, status.grpc_failures
        );

        // Grok's pipeline analysis
        let pipeline_latency = if status.is_primary_active {
            status.shredstream_avg_latency + 75.0 + 45.0 // ShredStream + PumpFun RPC + Jito
        } else {
            status.grpc_avg_latency + 75.0 + 45.0 // gRPC + PumpFun RPC + Jito
        };

        info!(
            "  • Pipeline Est: {:.0}ms (Target: <150ms for EXTREME mode)",
            pipeline_latency
        );

        if pipeline_latency < 150.0 {
            info!("  • Status: 🔥 ELITE TIER - Optimal for MEV front-running");
        } else if pipeline_latency < 200.0 {
            info!("  • Status: 🎯 COMPETITIVE - Good for MEV execution");
        } else {
            warn!("  • Status: ⚠️  NEEDS OPTIMIZATION - Review RPC performance");
        }
    }
}

#[derive(Debug, Clone)]
pub struct FailoverStatus {
    pub current_source: String,
    pub switch_count: u32,
    pub shredstream_avg_latency: f64,
    pub grpc_avg_latency: f64,
    pub shredstream_failures: u32,
    pub grpc_failures: u32,
    pub is_primary_active: bool,
}
