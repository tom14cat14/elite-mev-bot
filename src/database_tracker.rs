// Database Tracker Module
// Tracks MEV opportunities, execution attempts, and results

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

/// Database tracker for MEV opportunities
#[derive(Clone)]
pub struct DatabaseTracker {
    conn: Arc<Mutex<Connection>>,
}

impl DatabaseTracker {
    /// Initialize database connection and create schema
    pub fn new(db_path: &str) -> Result<Self> {
        // Create data directory if it doesn't exist
        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent).context("Failed to create data directory")?;
        }

        let conn = Connection::open(db_path).context("Failed to open database")?;

        // Load schema
        let schema = include_str!("../schema.sql");
        conn.execute_batch(schema)
            .context("Failed to initialize database schema")?;

        info!("✅ Database initialized: {}", db_path);

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Log a detected opportunity
    pub fn log_detected(
        &self,
        signature: &str,
        dex_name: &str,
        pool_address: &str,
        estimated_sol_value: f64,
        estimated_price_impact: Option<f64>,
        quality_score: Option<f64>,
        token_mint: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR IGNORE INTO opportunities (
                signature, dex_name, pool_address, estimated_sol_value,
                estimated_price_impact, quality_score, status, token_mint
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'detected', ?7)",
            params![
                signature,
                dex_name,
                pool_address,
                estimated_sol_value,
                estimated_price_impact,
                quality_score,
                token_mint,
            ],
        )
        .context("Failed to insert detected opportunity")?;

        Ok(conn.last_insert_rowid())
    }

    /// Log a skipped opportunity
    pub fn log_skipped(&self, signature: &str, dex_name: &str, skip_reason: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE opportunities SET status = 'skipped', skip_reason = ?3
             WHERE signature = ?1 AND dex_name = ?2",
            params![signature, dex_name, skip_reason],
        )
        .context("Failed to update skipped opportunity")?;

        Ok(())
    }

    /// Log an execution attempt
    pub fn log_attempted(
        &self,
        signature: &str,
        dex_name: &str,
        position_size_sol: f64,
        estimated_gross_profit: f64,
        estimated_net_profit: f64,
        total_fees: f64,
        jito_tip_lamports: u64,
        token_amount: Option<f64>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE opportunities SET
                status = 'attempted',
                position_size_sol = ?3,
                estimated_gross_profit = ?4,
                estimated_net_profit = ?5,
                total_fees = ?6,
                jito_tip_lamports = ?7,
                token_amount = ?8,
                bundle_submitted_at = CURRENT_TIMESTAMP
             WHERE signature = ?1 AND dex_name = ?2",
            params![
                signature,
                dex_name,
                position_size_sol,
                estimated_gross_profit,
                estimated_net_profit,
                total_fees,
                jito_tip_lamports as i64,
                token_amount,
            ],
        )
        .context("Failed to update attempted execution")?;

        Ok(())
    }

    /// Log a successful execution
    pub fn log_executed(
        &self,
        signature: &str,
        dex_name: &str,
        our_signature: &str,
        actual_profit_sol: f64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE opportunities SET
                status = 'executed',
                our_signature = ?3,
                actual_profit_sol = ?4,
                executed_at = CURRENT_TIMESTAMP
             WHERE signature = ?1 AND dex_name = ?2",
            params![signature, dex_name, our_signature, actual_profit_sol],
        )
        .context("Failed to update executed opportunity")?;

        info!(
            "✅ EXECUTION TRACKED | Victim: {} | Our Tx: {} | Profit: {:.6} SOL",
            &signature[..20],
            &our_signature[..20],
            actual_profit_sol
        );

        Ok(())
    }

    /// Log a failed execution
    pub fn log_failed(&self, signature: &str, dex_name: &str, error_msg: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE opportunities SET
                status = 'failed',
                execution_error = ?3
             WHERE signature = ?1 AND dex_name = ?2",
            params![signature, dex_name, error_msg],
        )
        .context("Failed to update failed execution")?;

        warn!(
            "❌ EXECUTION FAILED | Victim: {} | DEX: {} | Error: {}",
            &signature[..20],
            dex_name,
            error_msg
        );

        Ok(())
    }

    /// Get total opportunities detected today
    pub fn get_today_stats(&self) -> Result<OpportunityStats> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'detected' THEN 1 ELSE 0 END) as detected,
                SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END) as skipped,
                SUM(CASE WHEN status = 'attempted' THEN 1 ELSE 0 END) as attempted,
                SUM(CASE WHEN status = 'executed' THEN 1 ELSE 0 END) as executed,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
                COALESCE(SUM(actual_profit_sol), 0.0) as total_profit
             FROM opportunities
             WHERE DATE(detected_at) = DATE('now')",
        )?;

        let stats = stmt.query_row([], |row| {
            Ok(OpportunityStats {
                total: row.get(0)?,
                detected: row.get(1)?,
                skipped: row.get(2)?,
                attempted: row.get(3)?,
                executed: row.get(4)?,
                failed: row.get(5)?,
                total_profit: row.get(6)?,
            })
        })?;

        Ok(stats)
    }
}

#[derive(Debug, Clone)]
pub struct OpportunityStats {
    pub total: i64,
    pub detected: i64,
    pub skipped: i64,
    pub attempted: i64,
    pub executed: i64,
    pub failed: i64,
    pub total_profit: f64,
}

impl OpportunityStats {
    pub fn execution_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.executed as f64 / self.total as f64) * 100.0
        }
    }

    pub fn success_rate(&self) -> f64 {
        let attempts = self.attempted + self.executed + self.failed;
        if attempts == 0 {
            0.0
        } else {
            (self.executed as f64 / attempts as f64) * 100.0
        }
    }
}
