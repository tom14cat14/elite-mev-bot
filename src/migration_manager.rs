use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn, error};

use crate::pumpfun_executor::PumpFunExecutor;
use crate::sandwich_engine::SandwichEngine;

/// Tracks active positions and monitors for PumpFun token migrations
pub struct MigrationManager {
    active_positions: Arc<Mutex<HashMap<String, ActivePosition>>>,
    pumpfun_executor: Arc<PumpFunExecutor>,
    monitoring_enabled: bool,
    check_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePosition {
    pub token_mint: String,
    pub position_type: PositionType,
    pub entry_time: DateTime<Utc>,
    pub amount: u64,
    pub entry_price_sol: f64,
    pub opportunity_id: String,
    pub last_migration_check: DateTime<Utc>,
    pub migration_progress_last: f64,
    pub exit_triggered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionType {
    Long,  // Bought tokens (need to sell on migration)
    Short, // Sold tokens (need to buy back on migration)
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationAlert {
    pub token_mint: String,
    pub alert_type: AlertType,
    pub migration_progress: f64,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub positions_affected: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum AlertType {
    MigrationDetected,     // Token has migrated - immediate exit required
    MigrationImminent,     // >90% progress - prepare for exit
    MigrationWarning,      // >80% progress - warning
    PositionExited,        // Position successfully exited
    PositionExitFailed,    // Failed to exit position
}

impl MigrationManager {
    pub fn new(pumpfun_executor: Arc<PumpFunExecutor>) -> Self {
        Self {
            active_positions: Arc::new(Mutex::new(HashMap::new())),
            pumpfun_executor,
            monitoring_enabled: true,
            check_interval_seconds: 10, // Check every 10 seconds
        }
    }

    /// Start monitoring for migrations (runs in background)
    pub async fn start_monitoring(&self) -> Result<()> {
        if !self.monitoring_enabled {
            return Ok(());
        }

        info!("🔄 Starting PumpFun migration monitoring (every {}s)", self.check_interval_seconds);

        let positions = self.active_positions.clone();
        let executor = self.pumpfun_executor.clone();
        let check_interval = self.check_interval_seconds;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(check_interval));

            loop {
                interval.tick().await;

                // Get current positions
                let current_positions = {
                    let positions_guard = positions.lock().unwrap();
                    positions_guard.clone()
                };

                if current_positions.is_empty() {
                    debug!("No active PumpFun positions to monitor");
                    continue;
                }

                info!("🔍 Checking {} active PumpFun positions for migration", current_positions.len());

                // Check each position for migration
                for (token_mint, position) in current_positions.iter() {
                    if position.exit_triggered {
                        continue; // Skip positions already marked for exit
                    }

                    // Check migration status
                    match executor.monitor_for_migration(token_mint).await {
                        Ok(migration_detected) => {
                            if migration_detected {
                                warn!("🚨 MIGRATION DETECTED for {}: Marking position for immediate exit!", token_mint);

                                // Mark position for exit
                                {
                                    let mut positions_guard = positions.lock().unwrap();
                                    if let Some(pos) = positions_guard.get_mut(token_mint) {
                                        pos.exit_triggered = true;
                                        pos.last_migration_check = Utc::now();
                                    }
                                }

                                // TODO: Trigger immediate position exit via sandwich engine
                                // This would require integration with the main trading loop
                            }
                        }
                        Err(e) => {
                            debug!("Failed to check migration for {}: {}", token_mint, e);
                        }
                    }

                    // Update migration progress tracking
                    if let Ok(progress) = executor.get_migration_progress(token_mint).await {
                        let mut positions_guard = positions.lock().unwrap();
                        if let Some(pos) = positions_guard.get_mut(token_mint) {
                            pos.migration_progress_last = progress;
                            pos.last_migration_check = Utc::now();

                            // Warn if approaching migration
                            if progress > 0.9 && !pos.exit_triggered {
                                warn!("⚠️ Token {} at {}% migration progress - prepare for exit!",
                                      token_mint, (progress * 100.0) as u32);
                            }
                        }
                    }
                }

                // Clean up old positions (>1 hour old or migrated)
                Self::cleanup_old_positions(&positions).await;
            }
        });

        Ok(())
    }

    /// Add a new position to track
    pub fn track_position(&self, position: ActivePosition) -> Result<()> {
        let mut positions = self.active_positions.lock().unwrap();
        let token_mint = position.token_mint.clone();

        info!("📍 Tracking new PumpFun position: {} {} tokens of {}",
              match position.position_type {
                  PositionType::Long => "LONG",
                  PositionType::Short => "SHORT",
              },
              position.amount,
              token_mint);

        positions.insert(token_mint, position);
        Ok(())
    }

    /// Remove a position from tracking (when manually closed)
    pub fn stop_tracking_position(&self, token_mint: &str) -> Result<()> {
        let mut positions = self.active_positions.lock().unwrap();
        if positions.remove(token_mint).is_some() {
            info!("📍 Stopped tracking position for token: {}", token_mint);
        }
        Ok(())
    }

    /// Get all active positions
    pub fn get_active_positions(&self) -> HashMap<String, ActivePosition> {
        let positions = self.active_positions.lock().unwrap();
        positions.clone()
    }

    /// Get positions that need immediate exit due to migration
    pub fn get_positions_requiring_exit(&self) -> Vec<ActivePosition> {
        let positions = self.active_positions.lock().unwrap();
        positions.values()
            .filter(|pos| pos.exit_triggered)
            .cloned()
            .collect()
    }

    /// Check if a specific token is close to migration
    pub async fn is_token_risky(&self, token_mint: &str) -> Result<bool> {
        self.pumpfun_executor.is_token_close_to_migration(token_mint).await
    }

    /// Get migration alerts for all active positions
    pub async fn get_migration_alerts(&self) -> Vec<MigrationAlert> {
        let mut alerts = Vec::new();
        let positions = self.active_positions.lock().unwrap();

        for (token_mint, position) in positions.iter() {
            // Check current migration status
            if let Ok(progress) = self.pumpfun_executor.get_migration_progress(token_mint).await {
                let alert_type = if position.exit_triggered {
                    AlertType::PositionExited
                } else if progress >= 1.0 {
                    AlertType::MigrationDetected
                } else if progress > 0.9 {
                    AlertType::MigrationImminent
                } else if progress > 0.8 {
                    AlertType::MigrationWarning
                } else {
                    continue; // No alert needed
                };

                let message = match alert_type {
                    AlertType::MigrationDetected => format!("Token {} has migrated! Exit immediately!", token_mint),
                    AlertType::MigrationImminent => format!("Token {} at {}% - migration imminent!", token_mint, (progress * 100.0) as u32),
                    AlertType::MigrationWarning => format!("Token {} at {}% - approaching migration", token_mint, (progress * 100.0) as u32),
                    AlertType::PositionExited => format!("Position exited for token {}", token_mint),
                    AlertType::PositionExitFailed => format!("Failed to exit position for token {}", token_mint),
                };

                alerts.push(MigrationAlert {
                    token_mint: token_mint.clone(),
                    alert_type,
                    migration_progress: progress,
                    message,
                    timestamp: Utc::now(),
                    positions_affected: vec![position.opportunity_id.clone()],
                });
            }
        }

        alerts
    }

    /// Force exit all positions for a specific token (emergency)
    pub async fn emergency_exit_token(&self, token_mint: &str) -> Result<Vec<String>> {
        let mut positions = self.active_positions.lock().unwrap();
        let mut exited_positions = Vec::new();

        if let Some(position) = positions.get_mut(token_mint) {
            if !position.exit_triggered {
                warn!("🚨 EMERGENCY EXIT triggered for token: {}", token_mint);
                position.exit_triggered = true;
                exited_positions.push(position.opportunity_id.clone());
            }
        }

        Ok(exited_positions)
    }

    /// Clean up old or completed positions
    async fn cleanup_old_positions(positions: &Arc<Mutex<HashMap<String, ActivePosition>>>) {
        let now = Utc::now();
        let max_age = chrono::Duration::hours(1); // Remove positions older than 1 hour

        let mut positions_guard = positions.lock().unwrap();
        let mut to_remove = Vec::new();

        for (token_mint, position) in positions_guard.iter() {
            let age = now.signed_duration_since(position.entry_time);
            if age > max_age || position.exit_triggered {
                to_remove.push(token_mint.clone());
            }
        }

        for token_mint in to_remove {
            positions_guard.remove(&token_mint);
            debug!("🧹 Cleaned up old position for token: {}", token_mint);
        }
    }

    /// Get statistics about migration monitoring
    pub fn get_monitoring_stats(&self) -> MigrationStats {
        let positions = self.active_positions.lock().unwrap();
        let total_positions = positions.len();
        let positions_requiring_exit = positions.values().filter(|p| p.exit_triggered).count();
        let oldest_position = positions.values()
            .map(|p| p.entry_time)
            .min()
            .unwrap_or_else(Utc::now);

        MigrationStats {
            total_active_positions: total_positions,
            positions_requiring_exit,
            monitoring_enabled: self.monitoring_enabled,
            check_interval_seconds: self.check_interval_seconds,
            oldest_position_age_minutes: Utc::now().signed_duration_since(oldest_position).num_minutes(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationStats {
    pub total_active_positions: usize,
    pub positions_requiring_exit: usize,
    pub monitoring_enabled: bool,
    pub check_interval_seconds: u64,
    pub oldest_position_age_minutes: i64,
}