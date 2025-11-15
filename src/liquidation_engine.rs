use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use solana_sdk::{instruction::Instruction, signature::Signer};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, error, info};

use crate::dex_registry::DexRegistry;
use crate::dynamic_fee_model::DynamicFeeModel;
use crate::jito_bundle_manager::JitoBundleManager;
use crate::jupiter_executor::JupiterExecutor;
use crate::wallet_manager::WalletManager;

/// High-performance liquidation engine for DeFi protocols
/// Monitors lending protocols for underwater positions and executes liquidations
pub struct LiquidationEngine {
    dex_registry: DexRegistry,
    fee_model: DynamicFeeModel,
    jupiter_executor: JupiterExecutor,
    bundle_manager: JitoBundleManager,
    wallet_manager: WalletManager,
    protocol_registry: ProtocolRegistry,
    min_profit_sol: f64,
    max_position_size_sol: f64,
    position_cache: HashMap<String, LiquidatablePosition>,
    stats: LiquidationStats,
}

#[derive(Debug, Clone)]
pub struct ProtocolRegistry {
    pub protocols: HashMap<String, ProtocolInfo>,
}

#[derive(Debug, Clone)]
pub struct ProtocolInfo {
    pub name: String,
    pub program_id: String,
    pub liquidation_threshold: f64,
    pub liquidation_bonus: f64,
    pub min_collateral_value: f64,
    pub supported_assets: Vec<String>,
    pub oracle_program_id: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct LiquidationStats {
    pub opportunities_detected: u64,
    pub opportunities_executed: u64,
    pub total_profit_sol: f64,
    pub average_execution_time_ms: f64,
    pub failed_executions: u64,
    pub positions_monitored: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiquidationOpportunity {
    pub opportunity_id: String,
    pub position: LiquidatablePosition,
    pub liquidation_params: LiquidationParams,
    pub estimated_profit_sol: f64,
    pub confidence_score: f64,
    pub execution_priority: u8,
    pub detected_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiquidatablePosition {
    pub position_id: String,
    pub user_wallet: String,
    pub protocol_name: String,
    pub collateral_mint: String,
    pub debt_mint: String,
    pub collateral_amount: u64,
    pub debt_amount: u64,
    pub collateral_value_usd: f64,
    pub debt_value_usd: f64,
    pub health_factor: f64,
    pub liquidation_threshold: f64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiquidationParams {
    pub liquidator_wallet: String,
    pub repay_amount: u64,
    pub collateral_to_receive: u64,
    pub liquidation_bonus: f64,
    pub gas_estimate: u64,
    pub protocol_program_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiquidationExecution {
    pub opportunity_id: String,
    pub bundle_id: String,
    pub execution_time_ms: u64,
    pub actual_profit_sol: f64,
    pub success: bool,
    pub error_message: Option<String>,
    pub liquidation_signature: Option<String>,
    pub repay_amount: u64,
    pub collateral_received: u64,
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        let mut protocols = HashMap::new();

        // Solend (most popular Solana lending protocol)
        protocols.insert(
            "solend".to_string(),
            ProtocolInfo {
                name: "Solend".to_string(),
                program_id: "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo".to_string(),
                liquidation_threshold: 0.85, // 85% LTV
                liquidation_bonus: 0.05,     // 5% bonus
                min_collateral_value: 100.0, // $100 minimum
                supported_assets: vec![
                    "So11111111111111111111111111111111111111112".to_string(), // SOL
                    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
                    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(), // USDT
                ],
                oracle_program_id: "FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH".to_string(), // Pyth
            },
        );

        // Marginfi
        protocols.insert(
            "marginfi".to_string(),
            ProtocolInfo {
                name: "MarginFi".to_string(),
                program_id: "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA".to_string(),
                liquidation_threshold: 0.80, // 80% LTV
                liquidation_bonus: 0.06,     // 6% bonus
                min_collateral_value: 50.0,  // $50 minimum
                supported_assets: vec![
                    "So11111111111111111111111111111111111111112".to_string(), // SOL
                    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
                    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So".to_string(), // mSOL
                ],
                oracle_program_id: "FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH".to_string(), // Pyth
            },
        );

        // Kamino Finance
        protocols.insert(
            "kamino".to_string(),
            ProtocolInfo {
                name: "Kamino".to_string(),
                program_id: "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD".to_string(),
                liquidation_threshold: 0.82, // 82% LTV
                liquidation_bonus: 0.04,     // 4% bonus
                min_collateral_value: 75.0,  // $75 minimum
                supported_assets: vec![
                    "So11111111111111111111111111111111111111112".to_string(), // SOL
                    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
                    "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj".to_string(), // stSOL
                ],
                oracle_program_id: "FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH".to_string(), // Pyth
            },
        );

        Self { protocols }
    }

    pub fn get_protocol(&self, name: &str) -> Option<&ProtocolInfo> {
        self.protocols.get(name)
    }

    pub fn get_protocol_by_program_id(&self, program_id: &str) -> Option<&ProtocolInfo> {
        self.protocols.values().find(|p| p.program_id == program_id)
    }

    pub fn all_protocols(&self) -> impl Iterator<Item = &ProtocolInfo> {
        self.protocols.values()
    }
}

impl LiquidationEngine {
    /// Create new liquidation engine targeting underwater positions
    pub fn new(
        jupiter_api_key: String,
        jito_endpoint: String,
        rpc_url: String,
        min_profit_sol: f64,
        max_position_size_sol: f64,
    ) -> Result<Self> {
        let wallet_manager = WalletManager::from_env()?;

        Ok(Self {
            dex_registry: DexRegistry::new(),
            fee_model: DynamicFeeModel::new(),
            jupiter_executor: JupiterExecutor::new(jupiter_api_key),
            bundle_manager: JitoBundleManager::new(jito_endpoint, rpc_url),
            wallet_manager,
            protocol_registry: ProtocolRegistry::new(),
            min_profit_sol,
            max_position_size_sol,
            position_cache: HashMap::new(),
            stats: LiquidationStats::default(),
        })
    }

    /// Monitor lending protocols for liquidatable positions
    pub async fn scan_for_liquidations(&mut self) -> Result<Vec<LiquidationOpportunity>> {
        let mut opportunities = Vec::new();
        self.stats.positions_monitored = 0;

        let protocols: Vec<_> = self.protocol_registry.all_protocols().cloned().collect();
        for protocol in protocols {
            let protocol_opportunities = self.scan_protocol_positions(&protocol).await?;
            opportunities.extend(protocol_opportunities);
        }

        info!(
            "🔍 Scanned {} positions across {} protocols, found {} liquidation opportunities",
            self.stats.positions_monitored,
            self.protocol_registry.protocols.len(),
            opportunities.len()
        );

        self.stats.opportunities_detected += opportunities.len() as u64;
        Ok(opportunities)
    }

    /// Scan specific protocol for liquidatable positions
    async fn scan_protocol_positions(
        &mut self,
        protocol: &ProtocolInfo,
    ) -> Result<Vec<LiquidationOpportunity>> {
        let mut opportunities = Vec::new();

        // In production, this would query the protocol's on-chain state
        // For testing, we'll simulate some positions
        let simulated_positions = self.simulate_protocol_positions(protocol);

        for position in simulated_positions {
            self.stats.positions_monitored += 1;

            // Check if position is liquidatable
            if position.health_factor < 1.0 {
                if let Some(opportunity) = self
                    .analyze_liquidation_opportunity(position.clone(), protocol)
                    .await?
                {
                    opportunities.push(opportunity);
                }
            }

            // Cache position for future monitoring
            self.position_cache
                .insert(position.position_id.clone(), position);
        }

        Ok(opportunities)
    }

    /// Simulate positions for testing (in production, would query on-chain data)
    fn simulate_protocol_positions(&self, protocol: &ProtocolInfo) -> Vec<LiquidatablePosition> {
        let mut positions = Vec::new();
        let now = Utc::now();

        // Simulate some healthy and underwater positions
        for i in 0..5 {
            let is_underwater = i == 0 || i == 3; // Make positions 0 and 3 liquidatable

            let health_factor = if is_underwater {
                0.85 + (fastrand::f64() * 0.10) // 0.85-0.95 (underwater)
            } else {
                1.10 + (fastrand::f64() * 0.50) // 1.10-1.60 (healthy)
            };

            let collateral_value = 1000.0 + (fastrand::f64() * 5000.0); // $1K-6K
            let debt_value = collateral_value * health_factor * protocol.liquidation_threshold;

            positions.push(LiquidatablePosition {
                position_id: format!("{}_{}", protocol.name.to_lowercase(), i),
                user_wallet: format!("User{}Wallet{}", protocol.name, i),
                protocol_name: protocol.name.clone(),
                collateral_mint: protocol.supported_assets[0].clone(), // SOL
                debt_mint: protocol.supported_assets[1].clone(),       // USDC
                collateral_amount: (collateral_value * 1_000_000_000.0 / 150.0) as u64, // Assume $150/SOL
                debt_amount: (debt_value * 1_000_000.0) as u64, // USDC has 6 decimals
                collateral_value_usd: collateral_value,
                debt_value_usd: debt_value,
                health_factor,
                liquidation_threshold: protocol.liquidation_threshold,
                last_updated: now,
            });
        }

        positions
    }

    /// Analyze liquidation opportunity for profitability
    async fn analyze_liquidation_opportunity(
        &mut self,
        position: LiquidatablePosition,
        protocol: &ProtocolInfo,
    ) -> Result<Option<LiquidationOpportunity>> {
        // Calculate maximum liquidatable amount (typically 50% of debt)
        let max_liquidatable_debt = position.debt_amount / 2;

        // Calculate collateral to receive (debt + bonus)
        let collateral_to_receive =
            (max_liquidatable_debt as f64 * (1.0 + protocol.liquidation_bonus)) as u64;

        // Check if we have enough balance to repay debt
        let balance_info = self.wallet_manager.get_balance_info().await?;
        let required_usdc = max_liquidatable_debt as f64 / 1_000_000.0; // Convert to USDC

        if required_usdc > balance_info.sol_balance * 0.8 {
            // Use SOL balance as proxy
            debug!(
                "Insufficient balance for liquidation: need ${:.2} USDC, have {:.2} SOL",
                required_usdc, balance_info.sol_balance
            );
            return Ok(None);
        }

        // Calculate expected profit
        let collateral_value_received = (collateral_to_receive as f64 / 1_000_000_000.0) * 150.0; // $150/SOL
        let debt_value_repaid = max_liquidatable_debt as f64 / 1_000_000.0;
        let gross_profit = collateral_value_received - debt_value_repaid;

        // Account for gas fees and slippage
        let estimated_fees = 0.01; // ~$0.01 in SOL for transaction fees
        let slippage_cost = gross_profit * 0.005; // 0.5% slippage
        let net_profit_usd = gross_profit - estimated_fees - slippage_cost;
        let net_profit_sol = net_profit_usd / 150.0; // Convert to SOL

        // Check profitability threshold
        if net_profit_sol < self.min_profit_sol {
            debug!(
                "Liquidation below profit threshold: {:.4} SOL < {:.4} SOL minimum",
                net_profit_sol, self.min_profit_sol
            );
            return Ok(None);
        }

        // Validate with fee model
        let fee_calculation = self.fee_model.calculate_fees(net_profit_sol, 0.003)?; // Assume 0.3% protocol fee
        if !fee_calculation.should_execute {
            return Ok(None);
        }

        let opportunity_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let health_factor = position.health_factor;
        let confidence_score = self.calculate_liquidation_confidence(&position, protocol);

        Ok(Some(LiquidationOpportunity {
            opportunity_id,
            position,
            liquidation_params: LiquidationParams {
                liquidator_wallet: self.wallet_manager.get_main_keypair().pubkey().to_string(),
                repay_amount: max_liquidatable_debt,
                collateral_to_receive,
                liquidation_bonus: protocol.liquidation_bonus,
                gas_estimate: 50000, // Estimated compute units
                protocol_program_id: protocol.program_id.clone(),
            },
            estimated_profit_sol: fee_calculation.net_profit_sol,
            confidence_score,
            execution_priority: self
                .calculate_liquidation_priority(fee_calculation.net_profit_sol, health_factor),
            detected_at: now,
            expires_at: now + chrono::Duration::minutes(2), // Liquidations are time-sensitive
        }))
    }

    /// Execute liquidation opportunity
    pub async fn execute_liquidation(
        &mut self,
        opportunity: LiquidationOpportunity,
        recent_blockhash: solana_sdk::hash::Hash,
    ) -> Result<LiquidationExecution> {
        let start_time = Instant::now();
        let opportunity_id = opportunity.opportunity_id.clone();

        info!(
            "💧 Executing liquidation: {} (health factor: {:.3})",
            opportunity_id, opportunity.position.health_factor
        );

        // Build liquidation instructions
        let liquidation_instructions = self.build_liquidation_instructions(&opportunity).await?;

        // Create atomic bundle for liquidation
        let bundle = self
            .bundle_manager
            .create_liquidation_bundle(
                liquidation_instructions,
                self.wallet_manager.get_main_keypair(),
                recent_blockhash,
            )
            .await?;

        info!(
            "📦 Liquidation bundle created in {}ms: {}",
            start_time.elapsed().as_millis(),
            bundle.bundle_id
        );

        // Submit bundle to Jito
        match self.bundle_manager.submit_bundle(&bundle).await {
            Ok(jito_bundle_id) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                self.stats.opportunities_executed += 1;
                self.stats.total_profit_sol += opportunity.estimated_profit_sol;

                // Update average execution time
                let total_executions = self.stats.opportunities_executed as f64;
                self.stats.average_execution_time_ms = (self.stats.average_execution_time_ms
                    * (total_executions - 1.0)
                    + execution_time as f64)
                    / total_executions;

                info!(
                    "✅ Liquidation executed successfully in {}ms: {} -> Jito: {}",
                    execution_time, opportunity_id, jito_bundle_id
                );

                Ok(LiquidationExecution {
                    opportunity_id,
                    bundle_id: jito_bundle_id,
                    execution_time_ms: execution_time,
                    actual_profit_sol: opportunity.estimated_profit_sol,
                    success: true,
                    error_message: None,
                    liquidation_signature: None, // Will be populated when bundle executes
                    repay_amount: opportunity.liquidation_params.repay_amount,
                    collateral_received: opportunity.liquidation_params.collateral_to_receive,
                })
            }
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                self.stats.failed_executions += 1;

                error!(
                    "❌ Liquidation execution failed in {}ms: {} - {}",
                    execution_time, opportunity_id, e
                );

                Ok(LiquidationExecution {
                    opportunity_id,
                    bundle_id: bundle.bundle_id,
                    execution_time_ms: execution_time,
                    actual_profit_sol: 0.0,
                    success: false,
                    error_message: Some(e.to_string()),
                    liquidation_signature: None,
                    repay_amount: opportunity.liquidation_params.repay_amount,
                    collateral_received: 0,
                })
            }
        }
    }

    /// Build liquidation instructions for the specific protocol
    async fn build_liquidation_instructions(
        &self,
        opportunity: &LiquidationOpportunity,
    ) -> Result<Vec<Instruction>> {
        // In production, this would build actual protocol-specific liquidation instructions
        // For testing, return simplified instructions
        Ok(vec![solana_sdk::system_instruction::transfer(
            &self.wallet_manager.get_main_keypair().pubkey(),
            &self.wallet_manager.get_main_keypair().pubkey(),
            opportunity.liquidation_params.repay_amount,
        )])
    }

    /// Calculate confidence score for liquidation opportunity
    fn calculate_liquidation_confidence(
        &self,
        position: &LiquidatablePosition,
        protocol: &ProtocolInfo,
    ) -> f64 {
        let mut score: f64 = 0.8; // Base confidence

        // Higher confidence for more underwater positions
        if position.health_factor < 0.90 {
            score += 0.1;
        }
        if position.health_factor < 0.85 {
            score += 0.1;
        }

        // Higher confidence for larger positions
        if position.collateral_value_usd > 5000.0 {
            score += 0.05;
        }

        // Higher confidence for established protocols
        if protocol.name == "Solend" || protocol.name == "MarginFi" {
            score += 0.05;
        }

        score.min(1.0).max(0.1)
    }

    /// Calculate execution priority based on profit and urgency
    fn calculate_liquidation_priority(&self, profit_sol: f64, health_factor: f64) -> u8 {
        let base_priority = match profit_sol {
            p if p >= 2.0 => 9,
            p if p >= 1.0 => 7,
            p if p >= 0.5 => 5,
            p if p >= 0.1 => 3,
            _ => 1,
        };

        // Increase priority for more underwater positions (closer to bad debt)
        let urgency_bonus = if health_factor < 0.85 { 1 } else { 0 };

        (base_priority + urgency_bonus).min(10)
    }

    /// Clean up old cached positions
    pub fn cleanup_old_positions(&mut self) {
        let cutoff = Utc::now() - chrono::Duration::minutes(10);
        self.position_cache
            .retain(|_, position| position.last_updated > cutoff);
    }

    /// Get liquidation engine statistics
    pub fn get_stats(&self) -> LiquidationStats {
        self.stats.clone()
    }

    /// Get success rate percentage
    pub fn get_success_rate(&self) -> f64 {
        if self.stats.opportunities_detected == 0 {
            0.0
        } else {
            (self.stats.opportunities_executed as f64 / self.stats.opportunities_detected as f64)
                * 100.0
        }
    }

    /// Get number of cached positions
    pub fn get_cached_positions_count(&self) -> usize {
        self.position_cache.len()
    }
}
