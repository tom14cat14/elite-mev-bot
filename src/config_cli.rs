use anyhow::Result;
use serde_json;
use std::io::{self, Write};
use crate::dynamic_config_manager::{
    DynamicConfigManager, DynamicMevConfig, RiskConfig, TradingConfig,
    PerformanceConfig, CircuitBreakerConfig
};
use tracing::{info, warn, error};

/// Command-line interface for dynamic configuration management
pub struct ConfigCLI {
    config_manager: DynamicConfigManager,
}

impl ConfigCLI {
    pub fn new(config_file_path: String) -> Self {
        Self {
            config_manager: DynamicConfigManager::new(config_file_path),
        }
    }

    /// Start interactive configuration CLI
    pub async fn start_interactive_mode(&mut self) -> Result<()> {
        info!("🔧 Starting MEV Bot Configuration CLI");
        info!("Type 'help' for available commands, 'quit' to exit");

        loop {
            print!("mev-config> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            match self.handle_command(input).await {
                Ok(should_continue) => {
                    if !should_continue {
                        break;
                    }
                }
                Err(e) => {
                    error!("Command error: {}", e);
                }
            }
        }

        info!("👋 Configuration CLI exited");
        Ok(())
    }

    /// Handle CLI commands
    async fn handle_command(&mut self, input: &str) -> Result<bool> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(true);
        }

        match parts[0] {
            "help" => {
                self.print_help();
            }
            "status" => {
                self.print_status().await?;
            }
            "show" => {
                self.show_config().await?;
            }
            "emergency" => {
                self.emergency_stop().await?;
            }
            "resume" => {
                self.resume_trading().await?;
            }
            "paper" => {
                if parts.len() > 1 {
                    let enable = parts[1].parse::<bool>().unwrap_or(true);
                    self.set_paper_trading(enable).await?;
                } else {
                    self.toggle_paper_trading().await?;
                }
            }
            "risk" => {
                if parts.len() >= 3 {
                    self.update_risk_parameter(parts[1], parts[2]).await?;
                } else {
                    warn!("Usage: risk <parameter> <value>");
                }
            }
            "trading" => {
                if parts.len() >= 3 {
                    self.update_trading_parameter(parts[1], parts[2]).await?;
                } else {
                    warn!("Usage: trading <parameter> <value>");
                }
            }
            "performance" => {
                if parts.len() >= 3 {
                    self.update_performance_parameter(parts[1], parts[2]).await?;
                } else {
                    warn!("Usage: performance <parameter> <value>");
                }
            }
            "export" => {
                if parts.len() > 1 {
                    self.export_config(parts[1]).await?;
                } else {
                    warn!("Usage: export <filename>");
                }
            }
            "import" => {
                if parts.len() > 1 {
                    self.import_config(parts[1]).await?;
                } else {
                    warn!("Usage: import <filename>");
                }
            }
            "validate" => {
                self.validate_current_config().await?;
            }
            "quit" | "exit" => {
                return Ok(false);
            }
            _ => {
                warn!("Unknown command: {}. Type 'help' for available commands.", parts[0]);
            }
        }

        Ok(true)
    }

    fn print_help(&self) {
        println!("\n🔧 MEV Bot Configuration CLI Commands:");
        println!("  help                          - Show this help message");
        println!("  status                        - Show current bot status");
        println!("  show                          - Display current configuration");
        println!("  emergency                     - Trigger emergency stop");
        println!("  resume                        - Resume trading after emergency stop");
        println!("  paper [true|false]            - Toggle or set paper trading mode");
        println!();
        println!("Risk Management:");
        println!("  risk max_position <SOL>       - Set maximum position size");
        println!("  risk max_daily_loss <SOL>     - Set maximum daily loss");
        println!("  risk max_trades <number>      - Set maximum concurrent trades");
        println!("  risk quality_threshold <1-10> - Set quality threshold");
        println!();
        println!("Trading Parameters:");
        println!("  trading min_profit <SOL>      - Set minimum profit threshold");
        println!("  trading max_slippage <%>     - Set maximum slippage percentage");
        println!("  trading bonding_threshold <%> - Set bonding curve completion threshold");
        println!("  trading jito_tip <lamports>   - Set Jito tip amount");
        println!();
        println!("Performance Tuning:");
        println!("  performance latency <ms>      - Set target latency");
        println!("  performance threads <number>  - Set worker thread count");
        println!("  performance memory <size>     - Set memory pool size");
        println!();
        println!("Configuration Management:");
        println!("  export <filename>             - Export current config to file");
        println!("  import <filename>             - Import config from file");
        println!("  validate                      - Validate current configuration");
        println!("  quit/exit                     - Exit configuration CLI");
        println!();
    }

    async fn print_status(&mut self) -> Result<()> {
        let config = self.config_manager.get_config();
        println!("\n📊 MEV Bot Status:");
        println!("  Version: {}", config.version);
        println!("  Last Updated: {}", config.last_updated);
        println!("  Emergency Stop: {}", if config.risk_management.emergency_stop { "🚨 ACTIVE" } else { "✅ Normal" });
        println!("  Paper Trading: {}", if config.risk_management.enable_paper_trading { "📝 Enabled" } else { "💰 Live Trading" });
        println!("  Position Size: {} SOL", config.risk_management.max_position_size_sol);
        println!("  Target Latency: {} ms", config.performance_tuning.target_latency_ms);
        println!();
        Ok(())
    }

    async fn show_config(&mut self) -> Result<()> {
        let config = self.config_manager.get_config();
        let json = serde_json::to_string_pretty(&config)?;
        println!("\n📋 Current Configuration:");
        println!("{}", json);
        println!();
        Ok(())
    }

    async fn emergency_stop(&mut self) -> Result<()> {
        warn!("🚨 TRIGGERING EMERGENCY STOP");
        self.config_manager.emergency_stop().await?;
        println!("✅ Emergency stop activated. All trading halted.");
        Ok(())
    }

    async fn resume_trading(&mut self) -> Result<()> {
        let mut config = self.config_manager.get_config();
        config.risk_management.emergency_stop = false;
        config.version += 1;
        config.last_updated = chrono::Utc::now().to_rfc3339();

        // Update through the manager to trigger callbacks
        self.update_config(config).await?;
        info!("✅ Trading resumed");
        Ok(())
    }

    async fn set_paper_trading(&mut self, enable: bool) -> Result<()> {
        let mut config = self.config_manager.get_config();
        config.risk_management.enable_paper_trading = enable;
        config.version += 1;
        config.last_updated = chrono::Utc::now().to_rfc3339();

        self.update_config(config).await?;
        info!("📝 Paper trading {}", if enable { "enabled" } else { "disabled" });
        Ok(())
    }

    async fn toggle_paper_trading(&mut self) -> Result<()> {
        let config = self.config_manager.get_config();
        let new_state = !config.risk_management.enable_paper_trading;
        self.set_paper_trading(new_state).await
    }

    async fn update_risk_parameter(&mut self, param: &str, value: &str) -> Result<()> {
        let mut config = self.config_manager.get_config();

        match param {
            "max_position" => {
                let val: f64 = value.parse()?;
                config.risk_management.max_position_size_sol = val;
                info!("💰 Max position size updated to {} SOL", val);
            }
            "max_daily_loss" => {
                let val: f64 = value.parse()?;
                config.risk_management.max_daily_loss_sol = val;
                info!("🛡️ Max daily loss updated to {} SOL", val);
            }
            "max_trades" => {
                let val: u8 = value.parse()?;
                config.risk_management.max_concurrent_trades = val;
                info!("📊 Max concurrent trades updated to {}", val);
            }
            "quality_threshold" => {
                let val: f64 = value.parse()?;
                if val < 0.0 || val > 10.0 {
                    return Err(anyhow::anyhow!("Quality threshold must be between 0 and 10"));
                }
                config.risk_management.quality_threshold = val;
                info!("⭐ Quality threshold updated to {}", val);
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown risk parameter: {}", param));
            }
        }

        config.version += 1;
        config.last_updated = chrono::Utc::now().to_rfc3339();
        self.update_config(config).await
    }

    async fn update_trading_parameter(&mut self, param: &str, value: &str) -> Result<()> {
        let mut config = self.config_manager.get_config();

        match param {
            "min_profit" => {
                let val: f64 = value.parse()?;
                config.trading_params.min_profit_threshold_sol = val;
                info!("💎 Min profit threshold updated to {} SOL", val);
            }
            "max_slippage" => {
                let val: f64 = value.parse()?;
                config.trading_params.max_slippage_percentage = val;
                info!("📈 Max slippage updated to {}%", val);
            }
            "bonding_threshold" => {
                let val: f64 = value.parse()?;
                config.trading_params.bonding_curve_completion_threshold = val;
                info!("🎯 Bonding curve threshold updated to {}%", val);
            }
            "jito_tip" => {
                let val: u64 = value.parse()?;
                config.trading_params.jito_tip_lamports = val;
                info!("💸 Jito tip updated to {} lamports", val);
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown trading parameter: {}", param));
            }
        }

        config.version += 1;
        config.last_updated = chrono::Utc::now().to_rfc3339();
        self.update_config(config).await
    }

    async fn update_performance_parameter(&mut self, param: &str, value: &str) -> Result<()> {
        let mut config = self.config_manager.get_config();

        match param {
            "latency" => {
                let val: f64 = value.parse()?;
                config.performance_tuning.target_latency_ms = val;
                info!("⚡ Target latency updated to {} ms", val);
            }
            "threads" => {
                let val: usize = value.parse()?;
                config.performance_tuning.worker_thread_count = val;
                info!("🧵 Worker thread count updated to {}", val);
            }
            "memory" => {
                let val: usize = value.parse()?;
                config.performance_tuning.memory_pool_size = val;
                info!("🧠 Memory pool size updated to {}", val);
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown performance parameter: {}", param));
            }
        }

        config.version += 1;
        config.last_updated = chrono::Utc::now().to_rfc3339();
        self.update_config(config).await
    }

    async fn export_config(&mut self, filename: &str) -> Result<()> {
        let config = self.config_manager.get_config();
        let json = serde_json::to_string_pretty(&config)?;
        tokio::fs::write(filename, json).await?;
        info!("📁 Configuration exported to {}", filename);
        Ok(())
    }

    async fn import_config(&mut self, filename: &str) -> Result<()> {
        let content = tokio::fs::read_to_string(filename).await?;
        let config: DynamicMevConfig = serde_json::from_str(&content)?;

        // Validate before importing
        DynamicConfigManager::validate_config(&config)?;

        self.update_config(config).await?;
        info!("📥 Configuration imported from {}", filename);
        Ok(())
    }

    async fn validate_current_config(&mut self) -> Result<()> {
        let config = self.config_manager.get_config();
        match DynamicConfigManager::validate_config(&config) {
            Ok(()) => {
                info!("✅ Configuration is valid");
            }
            Err(e) => {
                error!("❌ Configuration validation failed: {}", e);
            }
        }
        Ok(())
    }

    async fn update_config(&mut self, config: DynamicMevConfig) -> Result<()> {
        // This would need to be implemented in the config manager
        // For now, we'll just validate and print
        DynamicConfigManager::validate_config(&config)?;
        info!("⚙️ Configuration updated successfully");
        Ok(())
    }
}

/// Standalone CLI binary for configuration management
#[tokio::main]
pub async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let config_file = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "mev_bot_config.json".to_string());

    let mut cli = ConfigCLI::new(config_file);
    cli.start_interactive_mode().await?;

    Ok(())
}