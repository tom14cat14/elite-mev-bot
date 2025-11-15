use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signer,
};
use std::str::FromStr;
use std::time::Instant;
use tracing::{debug, error, info, warn};

use crate::dex_registry::{DexInfo, DexRegistry};
use crate::dynamic_fee_model::DynamicFeeModel;
use crate::jito_bundle_manager::JitoBundleManager;
use crate::jupiter_executor::JupiterExecutor;
use crate::migration_manager::MigrationManager;
use crate::pumpfun_executor::{PumpFunExecutor, PumpFunSwapParams};
use crate::wallet_manager::WalletManager;

// Raydium AMM V4 program ID
const RAYDIUM_AMM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

// Orca Whirlpools program ID
const ORCA_WHIRLPOOLS_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

// Borsh-serializable struct for Raydium swap instruction data
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct RaydiumSwapInstructionData {
    pub discriminant: u8, // Always 9 for swap
    pub amount_in: u64,
    pub minimum_amount_out: u64,
}

// Borsh-serializable struct for Orca Whirlpools swap instruction data
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct OrcaWhirlpoolsSwapInstructionData {
    pub discriminant: u8, // Always 6 for swap
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub sqrt_price_limit: u128,
    pub amount_specified_is_input: bool,
    pub a_to_b: bool,
}

/// High-performance sandwich attack engine
/// Detects profitable sandwich opportunities and executes them via Jito bundles
/// Now supports PumpFun pre-migration tokens via direct bonding curve interaction
pub struct SandwichEngine {
    dex_registry: DexRegistry,
    fee_model: DynamicFeeModel,
    jupiter_executor: JupiterExecutor,
    pumpfun_executor: std::sync::Arc<PumpFunExecutor>,
    migration_manager: Option<MigrationManager>,
    bundle_manager: JitoBundleManager,
    wallet_manager: WalletManager,
    min_profit_sol: f64,
    max_position_size_sol: f64,
    stats: SandwichStats,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SandwichStats {
    pub opportunities_detected: u64,
    pub opportunities_executed: u64,
    pub total_profit_sol: f64,
    pub average_execution_time_ms: f64,
    pub failed_executions: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SandwichOpportunity {
    pub opportunity_id: String,
    pub victim_transaction: VictimTransaction,
    pub front_run_params: TradeParams,
    pub back_run_params: TradeParams,
    pub estimated_profit_sol: f64,
    pub confidence_score: f64,
    pub execution_priority: u8,
    pub detected_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VictimTransaction {
    pub signature: String,
    pub user_wallet: String,
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_tolerance: f64,
    pub dex_program_id: String,
    pub estimated_price_impact: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TradeParams {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u16,
    pub dex_name: String,
    pub estimated_gas: u64,
    pub pool_address: String, // Added: Raydium/Orca pool address for DEX-specific routing
}

#[derive(Debug, Clone, Serialize)]
pub struct SandwichExecution {
    pub opportunity_id: String,
    pub bundle_id: String,
    pub execution_time_ms: u64,
    pub actual_profit_sol: f64,
    pub success: bool,
    pub error_message: Option<String>,
    pub front_run_signature: Option<String>,
    pub back_run_signature: Option<String>,
}

impl SandwichEngine {
    /// Create new sandwich engine targeting profitable MEV opportunities
    pub fn new(
        jupiter_api_key: String,
        jito_endpoint: String,
        rpc_url: String,
        min_profit_sol: f64,
        max_position_size_sol: f64,
    ) -> Result<Self> {
        let wallet_manager = WalletManager::from_env()?;

        // Create a second wallet manager for PumpFun executor
        let pumpfun_wallet_manager = WalletManager::from_env()?;
        let pumpfun_executor = PumpFunExecutor::new(pumpfun_wallet_manager)?;

        // Create migration manager for PumpFun position tracking (shared Arc)
        let pumpfun_executor_arc = std::sync::Arc::new(pumpfun_executor);
        let migration_manager = MigrationManager::new(pumpfun_executor_arc.clone());

        Ok(Self {
            dex_registry: DexRegistry::new(),
            fee_model: DynamicFeeModel::new(),
            jupiter_executor: JupiterExecutor::new(jupiter_api_key),
            pumpfun_executor: pumpfun_executor_arc,
            migration_manager: Some(migration_manager),
            bundle_manager: JitoBundleManager::new(jito_endpoint, rpc_url),
            wallet_manager,
            min_profit_sol,
            max_position_size_sol,
            stats: SandwichStats::default(),
        })
    }

    /// Analyze a pending transaction for sandwich opportunities
    pub async fn analyze_transaction(
        &mut self,
        tx_data: &Value,
        recent_blockhash: solana_sdk::hash::Hash,
    ) -> Result<Option<SandwichOpportunity>> {
        let start_time = Instant::now();

        // Parse transaction data from ShredStream
        let victim_tx = match self.parse_victim_transaction(tx_data) {
            Ok(tx) => tx,
            Err(e) => {
                debug!("Failed to parse transaction: {}", e);
                return Ok(None);
            }
        };

        // Check if this DEX supports sandwich attacks
        let dex_info = match self.dex_registry.get_dex_by_name(&victim_tx.dex_program_id) {
            Some(dex) if dex.supports_sandwich => dex,
            _ => {
                debug!(
                    "DEX {} doesn't support sandwich attacks",
                    victim_tx.dex_program_id
                );
                return Ok(None);
            }
        };

        // Calculate sandwich opportunity
        let opportunity = self
            .calculate_sandwich_opportunity(victim_tx, dex_info)
            .await?;

        if let Some(opp) = &opportunity {
            // Validate profitability
            if opp.estimated_profit_sol < self.min_profit_sol {
                debug!(
                    "Opportunity {} below minimum profit threshold: {:.4} SOL",
                    opp.opportunity_id, opp.estimated_profit_sol
                );
                return Ok(None);
            }

            // Validate position size
            if opp.front_run_params.amount > (self.max_position_size_sol * 1_000_000_000.0) as u64 {
                debug!(
                    "Opportunity {} exceeds maximum position size",
                    opp.opportunity_id
                );
                return Ok(None);
            }

            self.stats.opportunities_detected += 1;
            let analysis_time = start_time.elapsed().as_millis();

            info!(
                "🎯 Sandwich opportunity detected in {}ms: {:.4} SOL profit ({})",
                analysis_time, opp.estimated_profit_sol, opp.opportunity_id
            );
        }

        Ok(opportunity)
    }

    /// Execute a sandwich attack using Jito bundles
    pub async fn execute_sandwich(
        &mut self,
        opportunity: SandwichOpportunity,
        recent_blockhash: solana_sdk::hash::Hash,
    ) -> Result<SandwichExecution> {
        let start_time = Instant::now();
        let opportunity_id = opportunity.opportunity_id.clone();

        info!("🥪 Executing sandwich attack: {}", opportunity_id);

        // Build front-run instructions
        let front_run_instructions = self
            .build_trade_instructions(
                &opportunity.front_run_params,
                &self.wallet_manager.get_main_keypair().pubkey(),
            )
            .await?;

        // Build back-run instructions
        let back_run_instructions = self
            .build_trade_instructions(
                &opportunity.back_run_params,
                &self.wallet_manager.get_main_keypair().pubkey(),
            )
            .await?;

        // Create atomic bundle
        let bundle = self
            .bundle_manager
            .create_sandwich_bundle(
                front_run_instructions,
                opportunity.victim_transaction.signature.clone(),
                back_run_instructions,
                self.wallet_manager.get_main_keypair(),
                recent_blockhash,
            )
            .await?;

        info!(
            "📦 Bundle created in {}ms: {}",
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
                    "✅ Sandwich executed successfully in {}ms: {} -> Jito: {}",
                    execution_time, opportunity_id, jito_bundle_id
                );

                Ok(SandwichExecution {
                    opportunity_id,
                    bundle_id: jito_bundle_id,
                    execution_time_ms: execution_time,
                    actual_profit_sol: opportunity.estimated_profit_sol, // Will be updated with real data
                    success: true,
                    error_message: None,
                    front_run_signature: None, // Will be populated when bundle executes
                    back_run_signature: None,
                })
            }
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                self.stats.failed_executions += 1;

                error!(
                    "❌ Sandwich execution failed in {}ms: {} - {}",
                    execution_time, opportunity_id, e
                );

                Ok(SandwichExecution {
                    opportunity_id,
                    bundle_id: bundle.bundle_id,
                    execution_time_ms: execution_time,
                    actual_profit_sol: 0.0,
                    success: false,
                    error_message: Some(e.to_string()),
                    front_run_signature: None,
                    back_run_signature: None,
                })
            }
        }
    }

    /// Parse victim transaction from ShredStream data
    fn parse_victim_transaction(&self, tx_data: &Value) -> Result<VictimTransaction> {
        // This is a simplified parser - in production you'd parse actual Solana transaction data
        let signature = tx_data
            .get("signature")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing signature"))?;

        let user_wallet = tx_data
            .get("accounts")
            .and_then(|v| v.get(0))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing user wallet"))?;

        let instructions = tx_data
            .get("instructions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing instructions"))?;

        // Find swap instruction
        for instruction in instructions {
            let program_id = instruction
                .get("programId")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if self.dex_registry.is_dex_program_str(program_id) {
                // Parse swap data (simplified)
                return Ok(VictimTransaction {
                    signature: signature.to_string(),
                    user_wallet: user_wallet.to_string(),
                    input_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL (example)
                    output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC (example)
                    amount: 1_000_000_000,    // 1 SOL (example)
                    slippage_tolerance: 0.01, // 1% (example)
                    dex_program_id: program_id.to_string(),
                    estimated_price_impact: 0.02, // 2% (example)
                });
            }
        }

        Err(anyhow::anyhow!("No supported DEX instruction found"))
    }

    /// Calculate sandwich opportunity parameters
    async fn calculate_sandwich_opportunity(
        &self,
        victim_tx: VictimTransaction,
        dex_info: &DexInfo,
    ) -> Result<Option<SandwichOpportunity>> {
        // Calculate optimal front-run size (typically 1-5x victim size)
        let optimal_front_run_size = self
            .calculate_optimal_front_run_size(victim_tx.amount, victim_tx.estimated_price_impact);

        // Check if we have enough balance
        let balance_info = self.wallet_manager.get_balance_info().await?;
        if optimal_front_run_size > (balance_info.sol_balance * 0.8 * 1_000_000_000.0) as u64 {
            debug!(
                "Insufficient balance for front-run: need {} SOL, have {} SOL",
                optimal_front_run_size as f64 / 1_000_000_000.0,
                balance_info.sol_balance
            );
            return Ok(None);
        }

        // Calculate expected profit
        let estimated_profit =
            self.calculate_expected_profit(&victim_tx, optimal_front_run_size, dex_info)?;

        // Validate with fee model
        let fee_calculation = self
            .fee_model
            .calculate_fees(estimated_profit, dex_info.fee_rate)?;
        if !fee_calculation.should_execute {
            return Ok(None);
        }

        let opportunity_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        Ok(Some(SandwichOpportunity {
            opportunity_id,
            victim_transaction: victim_tx.clone(),
            front_run_params: TradeParams {
                input_mint: victim_tx.input_mint.clone(),
                output_mint: victim_tx.output_mint.clone(),
                amount: optimal_front_run_size,
                slippage_bps: 50, // 0.5% slippage for MEV
                dex_name: dex_info.name.clone(),
                estimated_gas: 5000, // Estimated compute units
                pool_address: victim_tx.dex_program_id.clone(), // LOOP 2: Simplified (full pool resolution in LOOP 3)
            },
            back_run_params: TradeParams {
                input_mint: victim_tx.output_mint.clone(),
                output_mint: victim_tx.input_mint.clone(),
                amount: 0, // Will be calculated based on front-run output
                slippage_bps: 50,
                dex_name: dex_info.name.clone(),
                estimated_gas: 5000,
                pool_address: victim_tx.dex_program_id.clone(), // LOOP 2: Simplified (full pool resolution in LOOP 3)
            },
            estimated_profit_sol: fee_calculation.net_profit_sol,
            confidence_score: self.calculate_confidence_score(&victim_tx, dex_info),
            execution_priority: self.calculate_execution_priority(fee_calculation.net_profit_sol),
            detected_at: now,
            expires_at: now + chrono::Duration::milliseconds(400), // Expire before next block
        }))
    }

    /// Calculate optimal front-run size based on victim transaction
    fn calculate_optimal_front_run_size(
        &self,
        victim_amount: u64,
        victim_price_impact: f64,
    ) -> u64 {
        // Use 2-3x victim size for optimal profit extraction
        let multiplier = if victim_price_impact > 0.05 { 2.0 } else { 3.0 };
        (victim_amount as f64 * multiplier) as u64
    }

    /// Calculate expected profit from sandwich attack
    fn calculate_expected_profit(
        &self,
        victim_tx: &VictimTransaction,
        front_run_size: u64,
        dex_info: &DexInfo,
    ) -> Result<f64> {
        // Simplified profit calculation
        // In production, this would use real price data and slippage models
        let price_impact_profit =
            victim_tx.estimated_price_impact * front_run_size as f64 / 1_000_000_000.0;
        let fees = (front_run_size as f64 / 1_000_000_000.0) * dex_info.fee_rate * 2.0; // Two trades
        Ok(price_impact_profit - fees)
    }

    /// Calculate confidence score for opportunity
    fn calculate_confidence_score(&self, victim_tx: &VictimTransaction, dex_info: &DexInfo) -> f64 {
        let mut score: f64 = 0.7; // Base confidence

        // Higher confidence for larger transactions
        if victim_tx.amount > 10_000_000_000 {
            score += 0.1;
        } // >10 SOL

        // Higher confidence for low-slippage DEXs
        if dex_info.typical_slippage < 0.002 {
            score += 0.1;
        }

        // Lower confidence for high price impact
        if victim_tx.estimated_price_impact > 0.05 {
            score -= 0.2;
        }

        score.min(1.0).max(0.1)
    }

    /// Calculate execution priority (1-10, 10 = highest)
    fn calculate_execution_priority(&self, profit_sol: f64) -> u8 {
        match profit_sol {
            p if p >= 5.0 => 10,
            p if p >= 2.0 => 8,
            p if p >= 1.0 => 6,
            p if p >= 0.5 => 4,
            _ => 2,
        }
    }

    /// Build trade instructions - routes to Raydium, Orca, PumpFun, or Jupiter based on DEX
    async fn build_trade_instructions(
        &self,
        trade_params: &TradeParams,
        wallet_pubkey: &Pubkey,
    ) -> Result<Vec<Instruction>> {
        // Check if this is a Raydium trade
        if trade_params.dex_name.starts_with("Raydium") {
            info!(
                "🚀 Routing to Raydium AMM V4 for pool: {}",
                trade_params.pool_address
            );
            return self
                .build_raydium_trade_instructions(trade_params, wallet_pubkey)
                .await;
        }

        // Check if this is an Orca Whirlpools trade
        if trade_params.dex_name.starts_with("Orca") {
            info!(
                "🌊 Routing to Orca Whirlpools for pool: {}",
                trade_params.pool_address
            );
            return self
                .build_orca_trade_instructions(trade_params, wallet_pubkey)
                .await;
        }

        // CRITICAL: Check if this is a PumpFun token (pre-migration)
        let is_pumpfun_token = self.is_pumpfun_trade(trade_params).await?;

        if is_pumpfun_token {
            info!(
                "🎯 Routing to PumpFun executor for pre-migration token: {}",
                if trade_params.input_mint == "So11111111111111111111111111111111111111112" {
                    &trade_params.output_mint // SOL->Token (buy)
                } else {
                    &trade_params.input_mint // Token->SOL (sell)
                }
            );

            return self.build_pumpfun_trade_instructions(trade_params).await;
        }

        // Use Jupiter for migrated tokens and other DEXs
        debug!("🚀 Routing to Jupiter executor for migrated token");

        // Check cache first for route
        if let Some(cached_route) = self.jupiter_executor.get_route_from_cache(
            &trade_params.input_mint,
            &trade_params.output_mint,
            trade_params.amount,
        ) {
            debug!("Using cached route for trade instructions");
            return self.route_to_instructions(&cached_route, wallet_pubkey);
        }

        // For now, return simplified instruction
        // In production, this would build real Jupiter swap instructions
        Ok(vec![solana_sdk::system_instruction::transfer(
            wallet_pubkey,
            wallet_pubkey,
            trade_params.amount,
        )])
    }

    /// Build PumpFun-specific trade instructions using bonding curve
    async fn build_pumpfun_trade_instructions(
        &self,
        trade_params: &TradeParams,
    ) -> Result<Vec<Instruction>> {
        // Determine if this is a buy (SOL->Token) or sell (Token->SOL)
        let sol_mint = "So11111111111111111111111111111111111111112";
        let (token_mint, is_buy) = if trade_params.input_mint == sol_mint {
            (&trade_params.output_mint, true) // SOL->Token = buy
        } else {
            (&trade_params.input_mint, false) // Token->SOL = sell
        };

        // Calculate minimum amount out based on slippage
        let minimum_amount_out = if is_buy {
            // For SOL->Token, we expect tokens out, apply slippage protection
            (trade_params.amount as f64 * (10000 - trade_params.slippage_bps as u32) as f64
                / 10000.0) as u64
        } else {
            // For Token->SOL, we expect SOL out, apply slippage protection
            (trade_params.amount as f64 * (10000 - trade_params.slippage_bps as u32) as f64
                / 10000.0) as u64
        };

        // Create PumpFun swap parameters
        let pumpfun_params = PumpFunSwapParams {
            token_mint: token_mint.to_string(),
            amount_in: trade_params.amount,
            minimum_amount_out,
            is_buy,
            slippage_bps: trade_params.slippage_bps,
        };

        // Get bonding curve state to build instruction
        let bonding_curve_state = self
            .pumpfun_executor
            .get_bonding_curve_state(token_mint)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get bonding curve state: {}", e))?;

        // Build the actual PumpFun swap instruction
        let instruction = self
            .pumpfun_executor
            .create_swap_instruction(&pumpfun_params, &bonding_curve_state)?;

        debug!(
            "Built PumpFun {} instruction for {} tokens",
            if is_buy { "buy" } else { "sell" },
            token_mint
        );

        Ok(vec![instruction])
    }

    /// Build Raydium AMM V4 swap instructions
    /// CRITICAL FIX for Issue #1: DEX-specific instruction building
    async fn build_raydium_trade_instructions(
        &self,
        trade_params: &TradeParams,
        wallet_pubkey: &Pubkey,
    ) -> Result<Vec<Instruction>> {
        // Parse pool address
        let _pool_address = Pubkey::from_str(&trade_params.pool_address)
            .map_err(|_| anyhow::anyhow!("Invalid Raydium pool address"))?;

        // Calculate minimum amount out based on slippage
        let minimum_amount_out = ((trade_params.amount as f64)
            * (10000 - trade_params.slippage_bps as u32) as f64
            / 10000.0) as u64;

        // Build instruction data
        let instruction_data = RaydiumSwapInstructionData {
            discriminant: 9, // Swap instruction
            amount_in: trade_params.amount,
            minimum_amount_out,
        };

        let data = borsh::to_vec(&instruction_data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize Raydium instruction: {}", e))?;

        let program_id = Pubkey::from_str(RAYDIUM_AMM_V4_PROGRAM_ID)
            .map_err(|e| anyhow::anyhow!("Invalid Raydium program ID: {}", e))?;

        // SIMPLIFIED IMPLEMENTATION for LOOP 2
        // TODO (LOOP 3): Add full account metas fetching from pool state
        // For now, return a valid instruction structure that will be enhanced with:
        // - 17 account metas (AMM accounts, Serum accounts, user ATAs)
        // - Pool state fetching via RPC
        // - Proper ATA derivation for user token accounts

        warn!("⚠️  Raydium instruction builder: Using simplified implementation");
        warn!("   Full 17-account implementation pending pool state fetching infrastructure");

        let instruction = Instruction {
            program_id,
            accounts: vec![
                // Placeholder - will be populated with actual accounts in LOOP 3
                AccountMeta::new(*wallet_pubkey, true),
            ],
            data,
        };

        info!(
            "Built Raydium swap instruction: pool={}, amount_in={}, min_out={}",
            trade_params.pool_address, trade_params.amount, minimum_amount_out
        );

        Ok(vec![instruction])
    }

    /// Build Orca Whirlpools swap instructions
    /// CRITICAL FIX for Issue #1: DEX-specific instruction building
    async fn build_orca_trade_instructions(
        &self,
        trade_params: &TradeParams,
        wallet_pubkey: &Pubkey,
    ) -> Result<Vec<Instruction>> {
        // Parse pool address
        let _pool_address = Pubkey::from_str(&trade_params.pool_address)
            .map_err(|_| anyhow::anyhow!("Invalid Orca pool address"))?;

        // Calculate minimum amount out based on slippage
        let minimum_amount_out = ((trade_params.amount as f64)
            * (10000 - trade_params.slippage_bps as u32) as f64
            / 10000.0) as u64;

        // Determine swap direction (a_to_b) based on mint ordering
        // LOOP 2 simplification: Use lexicographic ordering
        // TODO (LOOP 3): Derive from pool state and token mints
        let a_to_b = trade_params.input_mint < trade_params.output_mint;

        // Build instruction data
        let instruction_data = OrcaWhirlpoolsSwapInstructionData {
            discriminant: 6, // Swap instruction
            amount: trade_params.amount,
            other_amount_threshold: minimum_amount_out,
            sqrt_price_limit: 0, // No price limit for LOOP 2
            amount_specified_is_input: true,
            a_to_b,
        };

        let data = borsh::to_vec(&instruction_data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize Orca instruction: {}", e))?;

        let program_id = Pubkey::from_str(ORCA_WHIRLPOOLS_PROGRAM_ID)
            .map_err(|e| anyhow::anyhow!("Invalid Orca program ID: {}", e))?;

        // SIMPLIFIED IMPLEMENTATION for LOOP 2
        // TODO (LOOP 3): Add full account metas fetching from whirlpool state
        // For now, return a valid instruction structure that will be enhanced with:
        // - Whirlpool account metas (whirlpool, token vaults, oracle)
        // - Tick array accounts for price range
        // - User ATA derivation

        warn!("⚠️  Orca instruction builder: Using simplified implementation");
        warn!("   Full account implementation pending whirlpool state fetching infrastructure");

        let instruction = Instruction {
            program_id,
            accounts: vec![
                // Placeholder - will be populated with actual accounts in LOOP 3
                AccountMeta::new(*wallet_pubkey, true),
            ],
            data,
        };

        info!(
            "Built Orca swap instruction: pool={}, amount={}, min_out={}, a_to_b={}",
            trade_params.pool_address, trade_params.amount, minimum_amount_out, a_to_b
        );

        Ok(vec![instruction])
    }

    /// Check if this trade involves a PumpFun token (pre-migration)
    async fn is_pumpfun_trade(&self, trade_params: &TradeParams) -> Result<bool> {
        let sol_mint = "So11111111111111111111111111111111111111112";

        // Get the token mint (non-SOL side of the trade)
        let token_mint = if trade_params.input_mint == sol_mint {
            &trade_params.output_mint // SOL->Token trade
        } else if trade_params.output_mint == sol_mint {
            &trade_params.input_mint // Token->SOL trade
        } else {
            // Token->Token trade, check both sides (unlikely for PumpFun)
            return Ok(false);
        };

        // Check if token is still on PumpFun bonding curve (not migrated)
        match self.pumpfun_executor.is_token_migrated(token_mint).await {
            Ok(is_migrated) => Ok(!is_migrated), // PumpFun if not migrated
            Err(_) => {
                // If we can't check migration status, assume migrated (use Jupiter)
                debug!(
                    "Could not check migration status for {}, assuming migrated",
                    token_mint
                );
                Ok(false)
            }
        }
    }

    /// Convert route data to Solana instructions
    fn route_to_instructions(
        &self,
        route_data: &Value,
        wallet_pubkey: &Pubkey,
    ) -> Result<Vec<Instruction>> {
        // Simplified - in production this would parse route data and build real swap instructions
        Ok(vec![solana_sdk::system_instruction::transfer(
            wallet_pubkey,
            wallet_pubkey,
            1000,
        )])
    }

    /// Get sandwich engine statistics
    pub fn get_stats(&self) -> SandwichStats {
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
}
