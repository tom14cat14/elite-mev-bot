//! MEV Bot Database Tracker
//!
//! Logs all detected opportunities, executions, and performance metrics to SQLite database
//! for web dashboard consumption.

use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn, error};
use chrono::Utc;

use crate::mev_sandwich_detector::SandwichOpportunity;

/// MEV tracking database wrapper
pub struct MevDatabaseTracker {
    conn: Arc<Mutex<Connection>>,
}

impl MevDatabaseTracker {
    /// Create new tracker and initialize database
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Initialize schema
        conn.execute_batch(include_str!("../data/mev_tracking_schema.sql"))?;

        info!("✅ MEV database tracker initialized");

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Log a detected opportunity
    pub fn log_detected(
        &self,
        opportunity: &SandwichOpportunity,
        detection_latency_ms: f64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO opportunities (
                signature, dex_name, pool_address, victim_swap_size_sol,
                estimated_profit_sol, status, slot, detection_latency_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(signature) DO NOTHING",
            params![
                &opportunity.signature,
                &opportunity.dex_name,
                opportunity.pool_address.as_ref().unwrap_or(&"unknown".to_string()),
                opportunity.estimated_sol_value,
                0.0,  // Estimated profit calculated later
                "detected",
                opportunity.slot,
                detection_latency_ms,
            ],
        )?;

        // Update daily stats
        self.update_daily_stats_detected(&conn, &opportunity.dex_name)?;

        Ok(())
    }

    /// Log a skipped opportunity
    pub fn log_skipped(
        &self,
        opportunity: &SandwichOpportunity,
        reason: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE opportunities SET status = 'skipped', skip_reason = ?1
             WHERE signature = ?2",
            params![reason, &opportunity.signature],
        )?;

        Ok(())
    }

    /// Log a submitted bundle (sent to JITO, not yet confirmed)
    pub fn log_submitted(
        &self,
        signature: &str,
        estimated_profit_sol: f64,
        fees_paid_sol: f64,
        jito_tip_sol: f64,
        position_size_sol: f64,
        bundle_id: &str,
        execution_latency_ms: f64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let executed_at = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE opportunities SET
                status = 'submitted',
                executed_at = ?1,
                actual_profit_sol = ?2,
                fees_paid_sol = ?3,
                jito_tip_sol = ?4,
                position_size_sol = ?5,
                bundle_id = ?6,
                execution_latency_ms = ?7
             WHERE signature = ?8",
            params![
                executed_at,
                estimated_profit_sol,
                fees_paid_sol,
                jito_tip_sol,
                position_size_sol,
                bundle_id,
                execution_latency_ms,
                signature,
            ],
        )?;

        Ok(())
    }

    /// Log on-chain confirmation of a bundle
    pub fn log_confirmed(
        &self,
        signature: &str,
        actual_profit_sol: f64,
        confirmation_block: u64,
        confirmation_signature: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let confirmed_at = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE opportunities SET
                status = 'confirmed',
                confirmed_on_chain = 1,
                confirmed_at = ?1,
                confirmation_block = ?2,
                confirmation_signature = ?3,
                actual_profit_sol = ?4
             WHERE signature = ?5",
            params![
                confirmed_at,
                confirmation_block as i64,
                confirmation_signature,
                actual_profit_sol,
                signature,
            ],
        )?;

        // Update daily stats for confirmed execution
        self.update_daily_stats_executed(&conn, actual_profit_sol, 0.0)?;

        Ok(())
    }

    /// Legacy method for backward compatibility (calls log_submitted)
    #[deprecated(note = "Use log_submitted instead - this marks as submitted, not confirmed")]
    pub fn log_executed(
        &self,
        signature: &str,
        actual_profit_sol: f64,
        fees_paid_sol: f64,
        jito_tip_sol: f64,
        position_size_sol: f64,
        bundle_id: &str,
        execution_latency_ms: f64,
    ) -> Result<()> {
        self.log_submitted(
            signature,
            actual_profit_sol,
            fees_paid_sol,
            jito_tip_sol,
            position_size_sol,
            bundle_id,
            execution_latency_ms,
        )
    }

    /// Log a failed execution
    pub fn log_failed(
        &self,
        signature: &str,
        reason: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE opportunities SET status = 'failed', skip_reason = ?1
             WHERE signature = ?2",
            params![reason, signature],
        )?;

        Ok(())
    }

    /// Update daily stats for detected opportunity
    fn update_daily_stats_detected(
        &self,
        conn: &Connection,
        dex_name: &str,
    ) -> Result<()> {
        let today = Utc::now().format("%Y-%m-%d").to_string();

        // Insert or increment
        conn.execute(
            "INSERT INTO daily_stats (date, total_detected) VALUES (?1, 1)
             ON CONFLICT(date) DO UPDATE SET
                total_detected = total_detected + 1,
                updated_at = CURRENT_TIMESTAMP",
            params![today],
        )?;

        // Update DEX-specific counts
        let dex_column = match dex_name {
            "Raydium_AMM_V4" => "raydium_v4_detected",
            "Raydium_CLMM" => "raydium_clmm_detected",
            "Raydium_CPMM" => "raydium_cpmm_detected",
            "Orca_Whirlpools" => "orca_detected",
            "Meteora_DLMM" => "meteora_detected",
            "PumpSwap" => "pumpswap_detected",
            _ => return Ok(()),  // Unknown DEX, skip
        };

        let sql = format!(
            "UPDATE daily_stats SET {} = {} + 1 WHERE date = ?1",
            dex_column, dex_column
        );
        conn.execute(&sql, params![today])?;

        Ok(())
    }

    /// Update daily stats for executed opportunity
    fn update_daily_stats_executed(
        &self,
        conn: &Connection,
        profit_sol: f64,
        fees_sol: f64,
    ) -> Result<()> {
        let today = Utc::now().format("%Y-%m-%d").to_string();

        conn.execute(
            "UPDATE daily_stats SET
                total_executed = total_executed + 1,
                total_profit_sol = total_profit_sol + ?1,
                total_fees_sol = total_fees_sol + ?2,
                updated_at = CURRENT_TIMESTAMP
             WHERE date = ?3",
            params![profit_sol, fees_sol, today],
        )?;

        Ok(())
    }

    /// Log current configuration
    pub fn log_config(
        &self,
        mode: &str,
        paper_trading: bool,
        min_swap_size: f64,
        max_swap_size: f64,
        min_profit: f64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO config_snapshots (
                mode, paper_trading, min_swap_size_sol, max_swap_size_sol,
                min_profit_sol, position_size_strategy, jito_tip_strategy
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                mode,
                paper_trading,
                min_swap_size,
                max_swap_size,
                min_profit,
                "Dynamic (40-70%)",
                "Ultra-Aggressive (99th %ile)",
            ],
        )?;

        Ok(())
    }

    /// Log performance metrics
    pub fn log_performance(
        &self,
        avg_detection_latency_ms: f64,
        avg_execution_latency_ms: f64,
        wallet_balance_sol: f64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO performance_metrics (
                avg_detection_latency_ms, avg_execution_latency_ms, wallet_balance_sol
            ) VALUES (?1, ?2, ?3)",
            params![
                avg_detection_latency_ms,
                avg_execution_latency_ms,
                wallet_balance_sol,
            ],
        )?;

        Ok(())
    }

    /// Get overall statistics (for API)
    pub fn get_overall_stats(&self) -> Result<OverallStats> {
        let conn = self.conn.lock().unwrap();

        let total_detected: i64 = conn.query_row(
            "SELECT COUNT(*) FROM opportunities WHERE status != 'failed'",
            [],
            |row| row.get(0),
        )?;

        let total_executed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM opportunities WHERE status = 'executed'",
            [],
            |row| row.get(0),
        )?;

        let total_profit: f64 = conn.query_row(
            "SELECT COALESCE(SUM(actual_profit_sol), 0) FROM opportunities WHERE status = 'executed'",
            [],
            |row| row.get(0),
        )?;

        let total_fees: f64 = conn.query_row(
            "SELECT COALESCE(SUM(fees_paid_sol), 0) FROM opportunities WHERE status = 'executed'",
            [],
            |row| row.get(0),
        )?;

        let avg_detection_latency: f64 = conn.query_row(
            "SELECT COALESCE(AVG(detection_latency_ms), 0) FROM opportunities WHERE detection_latency_ms IS NOT NULL",
            [],
            |row| row.get(0),
        )?;

        let avg_execution_latency: f64 = conn.query_row(
            "SELECT COALESCE(AVG(execution_latency_ms), 0) FROM opportunities WHERE execution_latency_ms IS NOT NULL",
            [],
            |row| row.get(0),
        )?;

        Ok(OverallStats {
            total_detected: total_detected as u64,
            total_executed: total_executed as u64,
            total_profit_sol: total_profit,
            total_fees_sol: total_fees,
            avg_detection_latency_ms: avg_detection_latency,
            avg_execution_latency_ms: avg_execution_latency,
        })
    }

    /// Get today's statistics
    pub fn get_today_stats(&self) -> Result<TodayStats> {
        let conn = self.conn.lock().unwrap();
        let today = Utc::now().format("%Y-%m-%d").to_string();

        let today_detected: i64 = conn.query_row(
            "SELECT COUNT(*) FROM opportunities WHERE DATE(timestamp) = ?1",
            params![today],
            |row| row.get(0),
        ).unwrap_or(0);

        let today_executed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM opportunities WHERE DATE(timestamp) = ?1 AND status = 'executed'",
            params![today],
            |row| row.get(0),
        ).unwrap_or(0);

        let today_profit: f64 = conn.query_row(
            "SELECT COALESCE(SUM(actual_profit_sol), 0) FROM opportunities WHERE DATE(timestamp) = ?1 AND status = 'executed'",
            params![today],
            |row| row.get(0),
        ).unwrap_or(0.0);

        Ok(TodayStats {
            detected: today_detected as u64,
            executed: today_executed as u64,
            profit_sol: today_profit,
        })
    }
}

/// Overall statistics structure
#[derive(Debug, Clone)]
pub struct OverallStats {
    pub total_detected: u64,
    pub total_executed: u64,
    pub total_profit_sol: f64,
    pub total_fees_sol: f64,
    pub avg_detection_latency_ms: f64,
    pub avg_execution_latency_ms: f64,
}

/// Today's statistics structure
#[derive(Debug, Clone)]
pub struct TodayStats {
    pub detected: u64,
    pub executed: u64,
    pub profit_sol: f64,
}
