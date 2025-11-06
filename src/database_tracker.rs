use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{debug, info};

use crate::arbitrage_engine::{ArbitrageOpportunity, ArbitrageExecution, ArbitrageStats};
use crate::sandwich_engine::{SandwichOpportunity, SandwichExecution, SandwichStats};
use crate::liquidation_engine::{LiquidationOpportunity, LiquidationExecution, LiquidationStats};
use crate::mempool_monitor::{MonitorStats, OpportunityEvent};

/// High-performance in-memory database with optional persistence
/// Tracks all MEV opportunities, executions, and performance metrics
pub struct DatabaseTracker {
    // In-memory storage
    opportunities: Arc<RwLock<HashMap<String, OpportunityRecord>>>,
    executions: Arc<RwLock<HashMap<String, ExecutionRecord>>>,
    performance_metrics: Arc<RwLock<Vec<PerformanceSnapshot>>>,
    daily_stats: Arc<RwLock<HashMap<String, DailyStats>>>, // Date -> Stats

    // Configuration
    config: DatabaseConfig,

    // Statistics
    stats: DatabaseStats,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub enable_persistence: bool,
    pub persistence_interval_minutes: u64,
    pub max_records_in_memory: usize,
    pub performance_snapshot_interval_seconds: u64,
    pub data_retention_days: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DatabaseStats {
    pub total_opportunities_tracked: u64,
    pub total_executions_tracked: u64,
    pub total_profit_tracked_sol: f64,
    pub database_size_mb: f64,
    pub last_persistence_time: Option<DateTime<Utc>>,
    pub persistence_errors: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpportunityRecord {
    pub opportunity_id: String,
    pub opportunity_type: OpportunityType,
    pub detected_at: DateTime<Utc>,
    pub estimated_profit_sol: f64,
    pub confidence_score: f64,
    pub execution_priority: u8,
    pub metadata: Value,
    pub executed: bool,
    pub execution_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionRecord {
    pub execution_id: String,
    pub opportunity_id: String,
    pub execution_type: ExecutionType,
    pub executed_at: DateTime<Utc>,
    pub execution_time_ms: u64,
    pub actual_profit_sol: f64,
    pub success: bool,
    pub error_message: Option<String>,
    pub bundle_id: Option<String>,
    pub transaction_signatures: Vec<String>,
    pub gas_used: u64,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
pub enum OpportunityType {
    Sandwich,
    Arbitrage,
    Liquidation,
}

#[derive(Debug, Clone, Serialize)]
pub enum ExecutionType {
    SandwichAttack,
    ArbitrageExecution,
    LiquidationExecution,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerformanceSnapshot {
    pub timestamp: DateTime<Utc>,
    pub mempool_stats: MonitorStats,
    pub sandwich_stats: SandwichStats,
    pub arbitrage_stats: ArbitrageStats,
    pub liquidation_stats: LiquidationStats,
    pub database_stats: DatabaseStats,
    pub system_metrics: SystemMetrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub network_latency_ms: f64,
    pub jupiter_api_rate_limit_remaining: u32,
    pub jito_bundle_queue_size: usize,
    pub active_opportunities: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyStats {
    pub date: String,
    pub opportunities_detected: u64,
    pub opportunities_executed: u64,
    pub total_profit_sol: f64,
    pub average_execution_time_ms: f64,
    pub success_rate_percent: f64,
    pub top_profitable_opportunity: Option<String>,
    pub engine_breakdown: EngineBreakdown,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineBreakdown {
    pub sandwich_profit_sol: f64,
    pub arbitrage_profit_sol: f64,
    pub liquidation_profit_sol: f64,
    pub sandwich_executions: u64,
    pub arbitrage_executions: u64,
    pub liquidation_executions: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerformanceReport {
    pub report_period: String,
    pub total_opportunities: u64,
    pub total_executions: u64,
    pub total_profit_sol: f64,
    pub average_execution_time_ms: f64,
    pub success_rate_percent: f64,
    pub profit_by_engine: HashMap<String, f64>,
    pub hourly_performance: Vec<HourlyPerformance>,
    pub top_opportunities: Vec<OpportunityRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HourlyPerformance {
    pub hour: String,
    pub opportunities_detected: u64,
    pub profit_sol: f64,
    pub average_execution_time_ms: f64,
    pub success_rate_percent: f64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            enable_persistence: false, // Disabled by default for testing
            persistence_interval_minutes: 5,
            max_records_in_memory: 100_000,
            performance_snapshot_interval_seconds: 30,
            data_retention_days: 30,
        }
    }
}

impl DatabaseTracker {
    /// Create new database tracker with in-memory storage
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            opportunities: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            performance_metrics: Arc::new(RwLock::new(Vec::new())),
            daily_stats: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: DatabaseStats::default(),
        }
    }

    /// Track a new opportunity detection
    pub async fn track_opportunity(&mut self, opportunity: &OpportunityEvent) -> Result<()> {
        let record = OpportunityRecord {
            opportunity_id: opportunity.opportunity_id.clone(),
            opportunity_type: match opportunity.engine_type {
                crate::mempool_monitor::EngineType::Sandwich => OpportunityType::Sandwich,
                crate::mempool_monitor::EngineType::Arbitrage => OpportunityType::Arbitrage,
                crate::mempool_monitor::EngineType::Liquidation => OpportunityType::Liquidation,
            },
            detected_at: opportunity.detected_at,
            estimated_profit_sol: opportunity.estimated_profit_sol,
            confidence_score: 0.8, // Default confidence
            execution_priority: 5, // Default priority
            metadata: serde_json::json!({
                "event_type": opportunity.event_type,
                "engine_type": opportunity.engine_type
            }),
            executed: false,
            execution_id: None,
        };

        let mut opportunities = self.opportunities.write().await;
        opportunities.insert(opportunity.opportunity_id.clone(), record);

        self.stats.total_opportunities_tracked += 1;

        debug!("📊 Tracked opportunity: {} ({:.4} SOL profit)",
               opportunity.opportunity_id, opportunity.estimated_profit_sol);

        Ok(())
    }

    /// Track a sandwich opportunity
    pub async fn track_sandwich_opportunity(&mut self, opportunity: &SandwichOpportunity) -> Result<()> {
        let record = OpportunityRecord {
            opportunity_id: opportunity.opportunity_id.clone(),
            opportunity_type: OpportunityType::Sandwich,
            detected_at: opportunity.detected_at,
            estimated_profit_sol: opportunity.estimated_profit_sol,
            confidence_score: opportunity.confidence_score,
            execution_priority: opportunity.execution_priority,
            metadata: serde_json::to_value(opportunity)?,
            executed: false,
            execution_id: None,
        };

        let mut opportunities = self.opportunities.write().await;
        opportunities.insert(opportunity.opportunity_id.clone(), record);

        self.stats.total_opportunities_tracked += 1;
        Ok(())
    }

    /// Track an arbitrage opportunity
    pub async fn track_arbitrage_opportunity(&mut self, opportunity: &ArbitrageOpportunity) -> Result<()> {
        let record = OpportunityRecord {
            opportunity_id: opportunity.opportunity_id.clone(),
            opportunity_type: OpportunityType::Arbitrage,
            detected_at: opportunity.detected_at,
            estimated_profit_sol: opportunity.estimated_profit_sol,
            confidence_score: opportunity.confidence_score,
            execution_priority: opportunity.execution_priority,
            metadata: serde_json::to_value(opportunity)?,
            executed: false,
            execution_id: None,
        };

        let mut opportunities = self.opportunities.write().await;
        opportunities.insert(opportunity.opportunity_id.clone(), record);

        self.stats.total_opportunities_tracked += 1;
        Ok(())
    }

    /// Track a liquidation opportunity
    pub async fn track_liquidation_opportunity(&mut self, opportunity: &LiquidationOpportunity) -> Result<()> {
        let record = OpportunityRecord {
            opportunity_id: opportunity.opportunity_id.clone(),
            opportunity_type: OpportunityType::Liquidation,
            detected_at: opportunity.detected_at,
            estimated_profit_sol: opportunity.estimated_profit_sol,
            confidence_score: opportunity.confidence_score,
            execution_priority: opportunity.execution_priority,
            metadata: serde_json::to_value(opportunity)?,
            executed: false,
            execution_id: None,
        };

        let mut opportunities = self.opportunities.write().await;
        opportunities.insert(opportunity.opportunity_id.clone(), record);

        self.stats.total_opportunities_tracked += 1;
        Ok(())
    }

    /// Track a sandwich execution
    pub async fn track_sandwich_execution(&mut self, execution: &SandwichExecution) -> Result<()> {
        let execution_id = uuid::Uuid::new_v4().to_string();

        let record = ExecutionRecord {
            execution_id: execution_id.clone(),
            opportunity_id: execution.opportunity_id.clone(),
            execution_type: ExecutionType::SandwichAttack,
            executed_at: Utc::now(),
            execution_time_ms: execution.execution_time_ms,
            actual_profit_sol: execution.actual_profit_sol,
            success: execution.success,
            error_message: execution.error_message.clone(),
            bundle_id: Some(execution.bundle_id.clone()),
            transaction_signatures: vec![
                execution.front_run_signature.clone().unwrap_or_default(),
                execution.back_run_signature.clone().unwrap_or_default(),
            ].into_iter().filter(|s| !s.is_empty()).collect(),
            gas_used: 100000, // Estimated
            metadata: serde_json::to_value(execution)?,
        };

        let mut executions = self.executions.write().await;
        executions.insert(execution_id.clone(), record);

        // Update opportunity record
        let mut opportunities = self.opportunities.write().await;
        if let Some(opp) = opportunities.get_mut(&execution.opportunity_id) {
            opp.executed = true;
            opp.execution_id = Some(execution_id);
        }

        self.stats.total_executions_tracked += 1;
        if execution.success {
            self.stats.total_profit_tracked_sol += execution.actual_profit_sol;
        }

        self.update_daily_stats(&execution.opportunity_id, execution.actual_profit_sol, execution.success).await;

        info!("📊 Tracked sandwich execution: {} ({:.4} SOL profit, {}ms)",
              execution.opportunity_id, execution.actual_profit_sol, execution.execution_time_ms);

        Ok(())
    }

    /// Track an arbitrage execution
    pub async fn track_arbitrage_execution(&mut self, execution: &ArbitrageExecution) -> Result<()> {
        let execution_id = uuid::Uuid::new_v4().to_string();

        let record = ExecutionRecord {
            execution_id: execution_id.clone(),
            opportunity_id: execution.opportunity_id.clone(),
            execution_type: ExecutionType::ArbitrageExecution,
            executed_at: Utc::now(),
            execution_time_ms: execution.execution_time_ms,
            actual_profit_sol: execution.actual_profit_sol,
            success: execution.success,
            error_message: execution.error_message.clone(),
            bundle_id: Some(execution.bundle_id.clone()),
            transaction_signatures: vec![], // ArbitrageExecution doesn't have transaction_signatures yet
            gas_used: 150000, // Estimated (two trades)
            metadata: serde_json::to_value(execution)?,
        };

        let mut executions = self.executions.write().await;
        executions.insert(execution_id.clone(), record);

        // Update opportunity record
        let mut opportunities = self.opportunities.write().await;
        if let Some(opp) = opportunities.get_mut(&execution.opportunity_id) {
            opp.executed = true;
            opp.execution_id = Some(execution_id);
        }

        self.stats.total_executions_tracked += 1;
        if execution.success {
            self.stats.total_profit_tracked_sol += execution.actual_profit_sol;
        }

        self.update_daily_stats(&execution.opportunity_id, execution.actual_profit_sol, execution.success).await;

        info!("📊 Tracked arbitrage execution: {} ({:.4} SOL profit, {}ms)",
              execution.opportunity_id, execution.actual_profit_sol, execution.execution_time_ms);

        Ok(())
    }

    /// Track a liquidation execution
    pub async fn track_liquidation_execution(&mut self, execution: &LiquidationExecution) -> Result<()> {
        let execution_id = uuid::Uuid::new_v4().to_string();

        let record = ExecutionRecord {
            execution_id: execution_id.clone(),
            opportunity_id: execution.opportunity_id.clone(),
            execution_type: ExecutionType::LiquidationExecution,
            executed_at: Utc::now(),
            execution_time_ms: execution.execution_time_ms,
            actual_profit_sol: execution.actual_profit_sol,
            success: execution.success,
            error_message: execution.error_message.clone(),
            bundle_id: Some(execution.bundle_id.clone()),
            transaction_signatures: execution.liquidation_signature.clone().into_iter().collect(),
            gas_used: 80000, // Estimated
            metadata: serde_json::to_value(execution)?,
        };

        let mut executions = self.executions.write().await;
        executions.insert(execution_id.clone(), record);

        // Update opportunity record
        let mut opportunities = self.opportunities.write().await;
        if let Some(opp) = opportunities.get_mut(&execution.opportunity_id) {
            opp.executed = true;
            opp.execution_id = Some(execution_id);
        }

        self.stats.total_executions_tracked += 1;
        if execution.success {
            self.stats.total_profit_tracked_sol += execution.actual_profit_sol;
        }

        self.update_daily_stats(&execution.opportunity_id, execution.actual_profit_sol, execution.success).await;

        info!("📊 Tracked liquidation execution: {} ({:.4} SOL profit, {}ms)",
              execution.opportunity_id, execution.actual_profit_sol, execution.execution_time_ms);

        Ok(())
    }

    /// Take a performance snapshot
    pub async fn take_performance_snapshot(
        &mut self,
        mempool_stats: MonitorStats,
        sandwich_stats: SandwichStats,
        arbitrage_stats: ArbitrageStats,
        liquidation_stats: LiquidationStats,
    ) -> Result<()> {
        let snapshot = PerformanceSnapshot {
            timestamp: Utc::now(),
            mempool_stats,
            sandwich_stats,
            arbitrage_stats,
            liquidation_stats,
            database_stats: self.stats.clone(),
            system_metrics: self.collect_system_metrics().await,
        };

        {
            let mut metrics = self.performance_metrics.write().await;
            metrics.push(snapshot);

            // Keep only recent snapshots to prevent memory growth
            let max_snapshots = 1000; // ~8 hours at 30-second intervals
            let current_len = metrics.len();
            if current_len > max_snapshots {
                metrics.drain(0..current_len - max_snapshots);
            }
        }

        debug!("📊 Performance snapshot taken");
        Ok(())
    }

    /// Update daily statistics
    async fn update_daily_stats(&self, opportunity_id: &str, profit_sol: f64, success: bool) {
        let today = Utc::now().format("%Y-%m-%d").to_string();

        let mut daily_stats = self.daily_stats.write().await;
        let stats = daily_stats.entry(today.clone()).or_insert_with(|| DailyStats {
            date: today,
            opportunities_detected: 0,
            opportunities_executed: 0,
            total_profit_sol: 0.0,
            average_execution_time_ms: 0.0,
            success_rate_percent: 0.0,
            top_profitable_opportunity: None,
            engine_breakdown: EngineBreakdown {
                sandwich_profit_sol: 0.0,
                arbitrage_profit_sol: 0.0,
                liquidation_profit_sol: 0.0,
                sandwich_executions: 0,
                arbitrage_executions: 0,
                liquidation_executions: 0,
            },
        });

        if success {
            stats.opportunities_executed += 1;
            stats.total_profit_sol += profit_sol;

            // Update top profitable opportunity
            if stats.top_profitable_opportunity.is_none() || profit_sol > 0.0 {
                stats.top_profitable_opportunity = Some(opportunity_id.to_string());
            }
        }

        // Recalculate success rate
        if stats.opportunities_detected > 0 {
            stats.success_rate_percent = (stats.opportunities_executed as f64 / stats.opportunities_detected as f64) * 100.0;
        }
    }

    /// Collect system metrics (simulated for testing)
    async fn collect_system_metrics(&self) -> SystemMetrics {
        SystemMetrics {
            cpu_usage_percent: 45.0 + (fastrand::f64() * 20.0), // 45-65%
            memory_usage_mb: 2048.0 + (fastrand::f64() * 512.0), // 2-2.5GB
            network_latency_ms: 10.0 + (fastrand::f64() * 20.0), // 10-30ms
            jupiter_api_rate_limit_remaining: 45 + fastrand::u32(..10), // 45-54 remaining
            jito_bundle_queue_size: fastrand::usize(..5), // 0-4 bundles
            active_opportunities: fastrand::usize(..20), // 0-19 opportunities
        }
    }

    /// Generate performance report for specified period
    pub async fn generate_performance_report(&self, hours: u64) -> Result<PerformanceReport> {
        let cutoff = Utc::now() - chrono::Duration::hours(hours as i64);

        let opportunities = self.opportunities.read().await;
        let executions = self.executions.read().await;

        let recent_opportunities: Vec<_> = opportunities.values()
            .filter(|opp| opp.detected_at > cutoff)
            .cloned()
            .collect();

        let recent_executions: Vec<_> = executions.values()
            .filter(|exec| exec.executed_at > cutoff)
            .cloned()
            .collect();

        let total_opportunities = recent_opportunities.len() as u64;
        let total_executions = recent_executions.len() as u64;
        let total_profit_sol: f64 = recent_executions.iter()
            .filter(|e| e.success)
            .map(|e| e.actual_profit_sol)
            .sum();

        let average_execution_time_ms = if total_executions > 0 {
            recent_executions.iter().map(|e| e.execution_time_ms as f64).sum::<f64>() / total_executions as f64
        } else {
            0.0
        };

        let success_rate_percent = if total_opportunities > 0 {
            (total_executions as f64 / total_opportunities as f64) * 100.0
        } else {
            0.0
        };

        // Profit breakdown by engine
        let mut profit_by_engine = HashMap::new();
        profit_by_engine.insert("sandwich".to_string(),
            recent_executions.iter()
                .filter(|e| matches!(e.execution_type, ExecutionType::SandwichAttack) && e.success)
                .map(|e| e.actual_profit_sol)
                .sum::<f64>()
        );
        profit_by_engine.insert("arbitrage".to_string(),
            recent_executions.iter()
                .filter(|e| matches!(e.execution_type, ExecutionType::ArbitrageExecution) && e.success)
                .map(|e| e.actual_profit_sol)
                .sum::<f64>()
        );
        profit_by_engine.insert("liquidation".to_string(),
            recent_executions.iter()
                .filter(|e| matches!(e.execution_type, ExecutionType::LiquidationExecution) && e.success)
                .map(|e| e.actual_profit_sol)
                .sum::<f64>()
        );

        // Top opportunities by profit
        let mut top_opportunities = recent_opportunities;
        top_opportunities.sort_by(|a, b| b.estimated_profit_sol.partial_cmp(&a.estimated_profit_sol).unwrap());
        top_opportunities.truncate(10);

        Ok(PerformanceReport {
            report_period: format!("Last {} hours", hours),
            total_opportunities,
            total_executions,
            total_profit_sol,
            average_execution_time_ms,
            success_rate_percent,
            profit_by_engine,
            hourly_performance: vec![], // Could implement detailed hourly breakdown
            top_opportunities,
        })
    }

    /// Clean up old records to prevent memory growth
    pub async fn cleanup_old_records(&mut self) -> Result<()> {
        let cutoff = Utc::now() - chrono::Duration::days(self.config.data_retention_days as i64);

        let mut opportunities = self.opportunities.write().await;
        let mut executions = self.executions.write().await;
        let mut daily_stats = self.daily_stats.write().await;

        let initial_opp_count = opportunities.len();
        let initial_exec_count = executions.len();

        opportunities.retain(|_, opp| opp.detected_at > cutoff);
        executions.retain(|_, exec| exec.executed_at > cutoff);
        daily_stats.retain(|date, _| {
            if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
                let date_time = parsed_date.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Utc).unwrap();
                date_time > cutoff
            } else {
                false
            }
        });

        let cleaned_opp = initial_opp_count - opportunities.len();
        let cleaned_exec = initial_exec_count - executions.len();

        if cleaned_opp > 0 || cleaned_exec > 0 {
            info!("🧹 Cleaned up {} old opportunities and {} old executions", cleaned_opp, cleaned_exec);
        }

        Ok(())
    }

    /// Get database statistics
    pub fn get_stats(&self) -> DatabaseStats {
        self.stats.clone()
    }

    /// Get current database size estimate
    pub async fn get_database_size_mb(&self) -> f64 {
        let opportunities = self.opportunities.read().await;
        let executions = self.executions.read().await;
        let metrics = self.performance_metrics.read().await;

        // Rough estimate of memory usage
        let opp_size = opportunities.len() * 512; // ~512 bytes per opportunity
        let exec_size = executions.len() * 1024; // ~1KB per execution
        let metrics_size = metrics.len() * 2048; // ~2KB per snapshot

        (opp_size + exec_size + metrics_size) as f64 / (1024.0 * 1024.0)
    }

    /// Get summary statistics
    pub async fn get_summary_stats(&self) -> Result<Value> {
        let opportunities = self.opportunities.read().await;
        let executions = self.executions.read().await;

        Ok(serde_json::json!({
            "total_opportunities": opportunities.len(),
            "total_executions": executions.len(),
            "total_profit_sol": self.stats.total_profit_tracked_sol,
            "database_size_mb": self.get_database_size_mb().await,
            "success_rate_percent": if opportunities.len() > 0 {
                (executions.len() as f64 / opportunities.len() as f64) * 100.0
            } else { 0.0 }
        }))
    }
}