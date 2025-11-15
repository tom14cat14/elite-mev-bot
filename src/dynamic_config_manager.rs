use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::time::interval;
use tracing::{error, info, warn};

/// Dynamic configuration that can be updated at runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMevConfig {
    pub risk_management: RiskConfig,
    pub trading_params: TradingConfig,
    pub performance_tuning: PerformanceConfig,
    pub circuit_breakers: CircuitBreakerConfig,
    pub last_updated: String,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub max_position_size_sol: f64,
    pub max_daily_loss_sol: f64,
    pub max_concurrent_trades: u8,
    pub quality_threshold: f64,
    pub enable_paper_trading: bool,
    pub emergency_stop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub min_profit_threshold_sol: f64,
    pub max_slippage_percentage: f64,
    pub bonding_curve_completion_threshold: f64,
    pub enable_jito_bundles: bool,
    pub jito_tip_lamports: u64,
    pub gas_price_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub target_latency_ms: f64,
    pub enable_simd_optimizations: bool,
    pub worker_thread_count: usize,
    pub enable_cpu_pinning: bool,
    pub memory_pool_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub max_consecutive_failures: u32,
    pub failure_rate_threshold: f64,
    pub cooldown_duration_seconds: u64,
    pub enable_volatility_protection: bool,
    pub market_impact_threshold: f64,
}

impl Default for DynamicMevConfig {
    fn default() -> Self {
        Self {
            risk_management: RiskConfig {
                max_position_size_sol: 0.5,
                max_daily_loss_sol: 5.0,
                max_concurrent_trades: 3,
                quality_threshold: 6.5,
                enable_paper_trading: true, // Start safe
                emergency_stop: false,
            },
            trading_params: TradingConfig {
                min_profit_threshold_sol: 0.08,
                max_slippage_percentage: 5.0,
                bonding_curve_completion_threshold: 0.75,
                enable_jito_bundles: true,
                jito_tip_lamports: 10_000,
                gas_price_multiplier: 1.2,
            },
            performance_tuning: PerformanceConfig {
                target_latency_ms: 15.0,
                enable_simd_optimizations: true,
                worker_thread_count: 4,
                enable_cpu_pinning: true,
                memory_pool_size: 1000,
            },
            circuit_breakers: CircuitBreakerConfig {
                max_consecutive_failures: 5,
                failure_rate_threshold: 0.3,    // 30% failure rate
                cooldown_duration_seconds: 300, // 5 minutes
                enable_volatility_protection: true,
                market_impact_threshold: 0.1, // 10% market impact
            },
            last_updated: chrono::Utc::now().to_rfc3339(),
            version: 1,
        }
    }
}

/// Dynamic configuration manager with hot-reload capabilities
pub struct DynamicConfigManager {
    config: Arc<RwLock<DynamicMevConfig>>,
    config_file_path: String,
    last_file_modified: Option<Instant>,
    update_callbacks: Vec<Box<dyn Fn(&DynamicMevConfig) + Send + Sync>>,
}

impl DynamicConfigManager {
    pub fn new(config_file_path: String) -> Self {
        Self {
            config: Arc::new(RwLock::new(DynamicMevConfig::default())),
            config_file_path,
            last_file_modified: None,
            update_callbacks: Vec::new(),
        }
    }

    /// Start the configuration watcher
    pub async fn start_config_watcher(&mut self) -> Result<()> {
        info!("🔧 Starting dynamic configuration watcher");

        // Load initial config
        self.load_config_from_file().await?;

        // Spawn config file watcher
        let config_clone = Arc::clone(&self.config);
        let file_path = self.config_file_path.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5)); // Check every 5 seconds

            loop {
                interval.tick().await;

                // Check if config file has been modified
                if let Ok(metadata) = fs::metadata(&file_path).await {
                    if let Ok(modified) = metadata.modified() {
                        // File has been modified, reload config
                        if let Err(e) = Self::reload_config_static(&config_clone, &file_path).await
                        {
                            error!("Failed to reload config: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Load configuration from file
    async fn load_config_from_file(&mut self) -> Result<()> {
        match fs::read_to_string(&self.config_file_path).await {
            Ok(content) => {
                let config: DynamicMevConfig = serde_json::from_str(&content)?;
                self.update_config(config).await?;
                info!("✅ Configuration loaded from: {}", self.config_file_path);
            }
            Err(_) => {
                // File doesn't exist, create default config
                let default_config = DynamicMevConfig::default();
                self.save_config_to_file(&default_config).await?;
                self.update_config(default_config).await?;
                info!(
                    "📁 Created default configuration file: {}",
                    self.config_file_path
                );
            }
        }
        Ok(())
    }

    /// Static reload method for use in async closure
    async fn reload_config_static(
        config: &Arc<RwLock<DynamicMevConfig>>,
        file_path: &str,
    ) -> Result<()> {
        let content = fs::read_to_string(file_path).await?;
        let new_config: DynamicMevConfig = serde_json::from_str(&content)?;

        {
            let mut config_guard = config.write().unwrap();
            if new_config.version > config_guard.version {
                *config_guard = new_config;
                info!(
                    "🔄 Configuration hot-reloaded (version: {})",
                    config_guard.version
                );
            }
        }

        Ok(())
    }

    /// Update configuration and notify callbacks
    async fn update_config(&mut self, new_config: DynamicMevConfig) -> Result<()> {
        {
            let mut config_guard = self.config.write().unwrap();
            *config_guard = new_config.clone();
        }

        // Notify all callbacks of config update
        for callback in &self.update_callbacks {
            callback(&new_config);
        }

        Ok(())
    }

    /// Save configuration to file
    async fn save_config_to_file(&self, config: &DynamicMevConfig) -> Result<()> {
        let content = serde_json::to_string_pretty(config)?;
        fs::write(&self.config_file_path, content).await?;
        Ok(())
    }

    /// Get current configuration (read-only)
    pub fn get_config(&self) -> DynamicMevConfig {
        self.config.read().unwrap().clone()
    }

    /// Update specific risk parameters
    pub async fn update_risk_config(&mut self, risk_config: RiskConfig) -> Result<()> {
        let mut new_config = self.get_config();
        new_config.risk_management = risk_config;
        new_config.version += 1;
        new_config.last_updated = chrono::Utc::now().to_rfc3339();

        self.save_config_to_file(&new_config).await?;
        self.update_config(new_config).await?;

        info!("💼 Risk configuration updated");
        Ok(())
    }

    /// Emergency stop all trading
    pub async fn emergency_stop(&mut self) -> Result<()> {
        let mut new_config = self.get_config();
        new_config.risk_management.emergency_stop = true;
        new_config.risk_management.enable_paper_trading = true;
        new_config.version += 1;
        new_config.last_updated = chrono::Utc::now().to_rfc3339();

        self.save_config_to_file(&new_config).await?;
        self.update_config(new_config).await?;

        warn!("🚨 EMERGENCY STOP ACTIVATED - All trading halted");
        Ok(())
    }

    /// Add configuration update callback
    pub fn add_update_callback<F>(&mut self, callback: F)
    where
        F: Fn(&DynamicMevConfig) + Send + Sync + 'static,
    {
        self.update_callbacks.push(Box::new(callback));
    }

    /// Validate configuration before applying
    pub fn validate_config(config: &DynamicMevConfig) -> Result<()> {
        // Risk validation
        if config.risk_management.max_position_size_sol <= 0.0 {
            return Err(anyhow::anyhow!("Invalid position size"));
        }

        if config.risk_management.quality_threshold < 0.0
            || config.risk_management.quality_threshold > 10.0
        {
            return Err(anyhow::anyhow!("Invalid quality threshold"));
        }

        // Trading validation
        if config.trading_params.max_slippage_percentage < 0.0
            || config.trading_params.max_slippage_percentage > 50.0
        {
            return Err(anyhow::anyhow!("Invalid slippage percentage"));
        }

        // Performance validation
        if config.performance_tuning.target_latency_ms <= 0.0 {
            return Err(anyhow::anyhow!("Invalid target latency"));
        }

        Ok(())
    }
}

/// Enhanced error handling with automatic recovery
#[derive(Debug, Clone)]
pub struct ErrorRecoveryManager {
    consecutive_failures: u32,
    last_failure_time: Option<Instant>,
    recovery_strategies: Vec<RecoveryStrategy>,
}

#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    ReducePositionSize(f64),
    IncreaseLatencyTarget(f64),
    SwitchToBackupEndpoint,
    EnablePaperTrading,
    EmergencyStop,
}

impl ErrorRecoveryManager {
    pub fn new() -> Self {
        Self {
            consecutive_failures: 0,
            last_failure_time: None,
            recovery_strategies: vec![
                RecoveryStrategy::ReducePositionSize(0.5), // Reduce by 50%
                RecoveryStrategy::IncreaseLatencyTarget(1.5), // Increase by 50%
                RecoveryStrategy::SwitchToBackupEndpoint,
                RecoveryStrategy::EnablePaperTrading,
                RecoveryStrategy::EmergencyStop,
            ],
        }
    }

    /// Handle error and apply recovery strategy
    pub async fn handle_error(
        &mut self,
        error: &str,
        config_manager: &mut DynamicConfigManager,
    ) -> Result<()> {
        self.consecutive_failures += 1;
        self.last_failure_time = Some(Instant::now());

        warn!("🔥 Error #{}: {}", self.consecutive_failures, error);

        // Apply recovery strategy based on failure count
        let strategy_index =
            (self.consecutive_failures - 1).min(self.recovery_strategies.len() as u32 - 1) as usize;
        let strategy = self.recovery_strategies[strategy_index].clone();

        match strategy {
            RecoveryStrategy::ReducePositionSize(factor) => {
                let mut config = config_manager.get_config();
                config.risk_management.max_position_size_sol *= factor;
                config.version += 1;
                config_manager.update_config(config).await?;
                info!("💰 Reduced position size by {:.0}%", (1.0 - factor) * 100.0);
            }
            RecoveryStrategy::IncreaseLatencyTarget(factor) => {
                let mut config = config_manager.get_config();
                config.performance_tuning.target_latency_ms *= factor;
                config.version += 1;
                config_manager.update_config(config).await?;
                info!(
                    "⏱️ Increased latency target by {:.0}%",
                    (factor - 1.0) * 100.0
                );
            }
            RecoveryStrategy::SwitchToBackupEndpoint => {
                info!("🔄 Switching to backup endpoint");
                // This would trigger failover
            }
            RecoveryStrategy::EnablePaperTrading => {
                let mut config = config_manager.get_config();
                config.risk_management.enable_paper_trading = true;
                config.version += 1;
                config_manager.update_config(config).await?;
                warn!("📝 Switched to paper trading mode");
            }
            RecoveryStrategy::EmergencyStop => {
                config_manager.emergency_stop().await?;
            }
        }

        Ok(())
    }

    /// Reset failure count on successful operation
    pub fn reset_failures(&mut self) {
        if self.consecutive_failures > 0 {
            info!("✅ Failure count reset after successful operation");
            self.consecutive_failures = 0;
        }
    }
}
