use anyhow::Result;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::{Instruction, AccountMeta},
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
    message::Message,
};
use std::str::FromStr;
use std::time::Instant;
use tracing::{debug, info, warn};

use crate::wallet_manager::WalletManager;

/// PumpFun program ID for direct bonding curve interactions
const PUMPFUN_PROGRAM_ID: &str = "PumpFunP4PfMpqd7KsAEL7NKPhpq6M4yDmMRr2tH6gN";

// CONSTANTS: Transaction fee estimates and safety margins
const ESTIMATED_GAS_LAMPORTS: u64 = 50_000;  // ~0.00005 SOL for transaction fees
const SAFETY_BUFFER_LAMPORTS: u64 = 5_000_000;  // 0.005 SOL safety reserve for rent + unexpected fees
const SOL_DECIMALS: u64 = 1_000_000_000;  // 1 SOL = 1 billion lamports

// CONSTANTS: Bonding curve thresholds
const BONDING_CURVE_MIGRATION_SOL: u64 = 85_000_000_000;  // ~85 SOL triggers migration to Raydium
const MINIMUM_REAL_RESERVES: u64 = 1_000_000;  // 0.001 SOL minimum to consider active

/// PumpFun bonding curve executor for pre-migration token trading
pub struct PumpFunExecutor {
    wallet_manager: WalletManager,
    program_id: Pubkey,
    rpc_client: Option<std::sync::Arc<solana_rpc_client::rpc_client::RpcClient>>,  // SECURITY FIX: For balance validation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunSwapParams {
    pub token_mint: String,
    pub amount_in: u64,
    pub minimum_amount_out: u64,
    pub is_buy: bool, // true for SOL->Token, false for Token->SOL
    pub slippage_bps: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct PumpFunSwapResult {
    pub success: bool,
    pub signature: Option<String>,
    pub actual_amount_out: Option<u64>,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BondingCurveState {
    pub token_mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub associated_bonding_curve: Pubkey,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
}

impl PumpFunExecutor {
    pub fn new(wallet_manager: WalletManager) -> Result<Self> {
        let program_id = Pubkey::from_str(PUMPFUN_PROGRAM_ID)
            .map_err(|e| anyhow::anyhow!("Invalid PumpFun program ID: {}", e))?;

        Ok(Self {
            wallet_manager,
            program_id,
            rpc_client: None,  // No RPC client, balance validation disabled
        })
    }

    /// Create executor with RPC client for balance validation (SECURITY FIX)
    pub fn new_with_rpc(
        wallet_manager: WalletManager,
        rpc_client: std::sync::Arc<solana_rpc_client::rpc_client::RpcClient>,
    ) -> Result<Self> {
        let program_id = Pubkey::from_str(PUMPFUN_PROGRAM_ID)
            .map_err(|e| anyhow::anyhow!("Invalid PumpFun program ID: {}", e))?;

        Ok(Self {
            wallet_manager,
            program_id,
            rpc_client: Some(rpc_client),
        })
    }

    /// Execute a buy order on PumpFun bonding curve (SOL -> Token)
    pub async fn execute_buy(
        &self,
        params: PumpFunSwapParams,
    ) -> Result<PumpFunSwapResult> {
        if !params.is_buy {
            return Err(anyhow::anyhow!("Use execute_buy only for SOL->Token swaps"));
        }

        debug!("Executing PumpFun buy: {} SOL for {} tokens",
               params.amount_in as f64 / 1e9, params.token_mint);

        self.execute_swap_internal(params).await
    }

    /// Execute a sell order on PumpFun bonding curve (Token -> SOL)
    pub async fn execute_sell(
        &self,
        params: PumpFunSwapParams,
    ) -> Result<PumpFunSwapResult> {
        if params.is_buy {
            return Err(anyhow::anyhow!("Use execute_sell only for Token->SOL swaps"));
        }

        debug!("Executing PumpFun sell: {} tokens for SOL", params.amount_in);

        self.execute_swap_internal(params).await
    }

    /// Internal swap execution with bonding curve calculation
    async fn execute_swap_internal(
        &self,
        params: PumpFunSwapParams,
    ) -> Result<PumpFunSwapResult> {
        let start_time = Instant::now();

        // SECURITY FIX: Validate wallet balance before attempting trade
        if let Some(rpc_client) = &self.rpc_client {
            let wallet_pubkey = self.wallet_manager.get_main_pubkey();

            match rpc_client.get_balance(&wallet_pubkey) {
                Ok(balance_lamports) => {
                    // Calculate total required: trade amount + gas fees + safety buffer
                    let trade_amount_lamports = if params.is_buy {
                        params.amount_in  // For buy, amount_in is SOL
                    } else {
                        0  // For sell, we're selling tokens not SOL
                    };

                    let total_required = trade_amount_lamports + ESTIMATED_GAS_LAMPORTS + SAFETY_BUFFER_LAMPORTS;

                    if balance_lamports < total_required {
                        let balance_sol = balance_lamports as f64 / SOL_DECIMALS as f64;
                        let required_sol = total_required as f64 / SOL_DECIMALS as f64;

                        warn!("Insufficient balance: have {:.6} SOL, need {:.6} SOL", balance_sol, required_sol);

                        return Ok(PumpFunSwapResult {
                            success: false,
                            signature: None,
                            actual_amount_out: None,
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                            error_message: Some(format!(
                                "Insufficient balance: wallet has {:.6} SOL but trade requires {:.6} SOL (including fees)",
                                balance_sol, required_sol
                            )),
                        });
                    }

                    debug!("Balance validation passed: {:.6} SOL available, {:.6} SOL required",
                           balance_lamports as f64 / SOL_DECIMALS as f64,
                           total_required as f64 / SOL_DECIMALS as f64);
                }
                Err(e) => {
                    warn!("Failed to fetch wallet balance, proceeding without validation: {}", e);
                    // Continue anyway - balance check is a safety measure, not a hard requirement
                }
            }
        } else {
            debug!("RPC client not configured, skipping balance validation");
        }

        // Get bonding curve state
        let bonding_curve_state = match self.get_bonding_curve_state(&params.token_mint).await {
            Ok(state) => state,
            Err(e) => {
                return Ok(PumpFunSwapResult {
                    success: false,
                    signature: None,
                    actual_amount_out: None,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    error_message: Some(format!("Failed to get bonding curve state: {}", e)),
                });
            }
        };

        // Check if token is still on bonding curve (not migrated)
        if bonding_curve_state.complete {
            return Ok(PumpFunSwapResult {
                success: false,
                signature: None,
                actual_amount_out: None,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                error_message: Some("Token has migrated from bonding curve".to_string()),
            });
        }

        // Calculate expected output based on bonding curve
        let expected_output = if params.is_buy {
            self.calculate_buy_output(&bonding_curve_state, params.amount_in)?
        } else {
            self.calculate_sell_output(&bonding_curve_state, params.amount_in)?
        };

        // Validate slippage
        if expected_output < params.minimum_amount_out {
            return Ok(PumpFunSwapResult {
                success: false,
                signature: None,
                actual_amount_out: Some(expected_output),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                error_message: Some(format!("Slippage too high: expected {}, minimum {}",
                                          expected_output, params.minimum_amount_out)),
            });
        }

        // Create swap instruction
        let instruction = self.create_swap_instruction(&params, &bonding_curve_state)?;

        // Execute transaction
        match self.send_transaction(instruction).await {
            Ok(signature) => {
                info!("PumpFun swap successful: {}", signature);
                Ok(PumpFunSwapResult {
                    success: true,
                    signature: Some(signature.to_string()),
                    actual_amount_out: Some(expected_output),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    error_message: None,
                })
            }
            Err(e) => {
                warn!("PumpFun swap failed: {}", e);
                Ok(PumpFunSwapResult {
                    success: false,
                    signature: None,
                    actual_amount_out: None,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    /// Get current bonding curve state for a token
    pub async fn get_bonding_curve_state(&self, token_mint: &str) -> Result<BondingCurveState> {
        let token_mint_pubkey = Pubkey::from_str(token_mint)?;

        // Derive bonding curve PDA
        let (bonding_curve, _) = Pubkey::find_program_address(
            &[b"bonding-curve", token_mint_pubkey.as_ref()],
            &self.program_id
        );

        // Derive associated bonding curve PDA
        let (associated_bonding_curve, _) = Pubkey::find_program_address(
            &[b"bonding-curve", token_mint_pubkey.as_ref()],
            &self.program_id
        );

        // TODO: Fetch actual account data from RPC
        // For now, return mock data - this needs to be implemented with actual RPC calls
        Ok(BondingCurveState {
            token_mint: token_mint_pubkey,
            bonding_curve,
            associated_bonding_curve,
            virtual_token_reserves: 1_000_000_000, // 1B tokens
            virtual_sol_reserves: 30_000_000_000,  // 30 SOL
            real_token_reserves: 800_000_000,      // 800M tokens
            real_sol_reserves: 24_000_000_000,     // 24 SOL
            token_total_supply: 1_000_000_000,     // 1B total supply
            complete: false,
        })
    }

    /// Calculate token output for SOL input (buy)
    fn calculate_buy_output(&self, state: &BondingCurveState, sol_input: u64) -> Result<u64> {
        // PumpFun bonding curve formula: k = virtual_sol_reserves * virtual_token_reserves
        // This is a constant product AMM formula (x * y = k)
        let k = state.virtual_sol_reserves * state.virtual_token_reserves;
        let new_sol_reserves = state.virtual_sol_reserves + sol_input;
        let new_token_reserves = k / new_sol_reserves;
        let token_output = state.virtual_token_reserves - new_token_reserves;

        debug!("Buy calculation: {} SOL -> {} tokens",
               sol_input as f64 / SOL_DECIMALS as f64,
               token_output as f64 / SOL_DECIMALS as f64);

        Ok(token_output)
    }

    /// Calculate SOL output for token input (sell)
    fn calculate_sell_output(&self, state: &BondingCurveState, token_input: u64) -> Result<u64> {
        // PumpFun bonding curve formula: k = virtual_sol_reserves * virtual_token_reserves
        // This is a constant product AMM formula (x * y = k)
        let k = state.virtual_sol_reserves * state.virtual_token_reserves;
        let new_token_reserves = state.virtual_token_reserves + token_input;
        let new_sol_reserves = k / new_token_reserves;
        let sol_output = state.virtual_sol_reserves - new_sol_reserves;

        debug!("Sell calculation: {} tokens -> {} SOL",
               token_input as f64 / SOL_DECIMALS as f64,
               sol_output as f64 / SOL_DECIMALS as f64);

        Ok(sol_output)
    }

    /// Create swap instruction for PumpFun
    pub fn create_swap_instruction(
        &self,
        params: &PumpFunSwapParams,
        state: &BondingCurveState,
    ) -> Result<Instruction> {
        let token_mint = Pubkey::from_str(&params.token_mint)?;
        let user_wallet = self.wallet_manager.get_main_pubkey();

        // PumpFun swap instruction data
        let instruction_data = if params.is_buy {
            // Buy instruction: [instruction_discriminator, amount, max_sol_cost]
            let mut data = vec![0; 17]; // 1 byte discriminator + 8 bytes amount + 8 bytes max_cost
            data[0] = 0x66; // Buy instruction discriminator (example)
            data[1..9].copy_from_slice(&params.amount_in.to_le_bytes());
            data[9..17].copy_from_slice(&(params.amount_in * 110 / 100).to_le_bytes()); // 10% slippage
            data
        } else {
            // Sell instruction: [instruction_discriminator, amount, min_sol_output]
            let mut data = vec![0; 17];
            data[0] = 0x33; // Sell instruction discriminator (example)
            data[1..9].copy_from_slice(&params.amount_in.to_le_bytes());
            data[9..17].copy_from_slice(&params.minimum_amount_out.to_le_bytes());
            data
        };

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(state.bonding_curve, false),
                AccountMeta::new(state.associated_bonding_curve, false),
                AccountMeta::new(token_mint, false),
                AccountMeta::new(user_wallet, true),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
            data: instruction_data,
        })
    }

    /// Send transaction to Solana network
    async fn send_transaction(&self, instruction: Instruction) -> Result<Signature> {
        // Get recent blockhash
        let recent_blockhash = solana_sdk::hash::Hash::default(); // TODO: Get real blockhash from RPC

        // Create transaction
        let message = Message::new(&[instruction], Some(&self.wallet_manager.get_main_pubkey()));
        let mut transaction = Transaction::new_unsigned(message);

        // Sign transaction
        let keypair = &self.wallet_manager.get_main_keypair();
        transaction.sign(&[&keypair], recent_blockhash);

        // TODO: Send to RPC and wait for confirmation
        // For now, return mock signature
        Ok(Signature::from([0u8; 64]))
    }

    /// Check if a token has migrated from bonding curve
    pub async fn is_token_migrated(&self, token_mint: &str) -> Result<bool> {
        match self.get_bonding_curve_state(token_mint).await {
            Ok(state) => {
                // Check multiple migration indicators
                let is_migrated = state.complete ||
                                 state.virtual_sol_reserves >= BONDING_CURVE_MIGRATION_SOL ||
                                 state.real_sol_reserves <= MINIMUM_REAL_RESERVES;

                if is_migrated {
                    info!("🔄 Token {} has migrated: complete={}, virtual_sol={}, real_sol={}",
                          token_mint, state.complete,
                          state.virtual_sol_reserves as f64 / 1e9,
                          state.real_sol_reserves as f64 / 1e9);
                }

                Ok(is_migrated)
            }
            Err(e) => {
                warn!("Failed to check migration status for {}: {}", token_mint, e);
                Ok(true) // If we can't get state, assume migrated for safety
            }
        }
    }

    /// Get migration progress (0.0 = just launched, 1.0 = ready for migration)
    pub async fn get_migration_progress(&self, token_mint: &str) -> Result<f64> {
        let state = self.get_bonding_curve_state(token_mint).await?;

        // Calculate progress based on SOL reserves (target is ~85 SOL for migration)
        let target_sol_for_migration = 85_000_000_000.0; // 85 SOL in lamports
        let progress = (state.virtual_sol_reserves as f64) / target_sol_for_migration;

        Ok(progress.min(1.0)) // Cap at 100%
    }

    /// Check if token is close to migration (>80% progress)
    pub async fn is_token_close_to_migration(&self, token_mint: &str) -> Result<bool> {
        let progress = self.get_migration_progress(token_mint).await?;
        Ok(progress > 0.8) // 80% threshold
    }

    /// Monitor token for migration events - returns true if migration detected
    pub async fn monitor_for_migration(&self, token_mint: &str) -> Result<bool> {
        // Get current state
        let state = self.get_bonding_curve_state(token_mint).await?;

        // Check if migration has occurred
        if state.complete {
            info!("🚨 MIGRATION DETECTED: Token {} bonding curve is now complete!", token_mint);
            return Ok(true);
        }

        // Check if close to migration (warning)
        let progress = (state.virtual_sol_reserves as f64) / 85_000_000_000.0;
        if progress > 0.9 {
            warn!("⚠️ MIGRATION WARNING: Token {} is {}% to migration!",
                  token_mint, (progress * 100.0) as u32);
        }

        Ok(false)
    }

    /// Get current token price in SOL
    pub async fn get_token_price_sol(&self, token_mint: &str) -> Result<f64> {
        let state = self.get_bonding_curve_state(token_mint).await?;
        let price_per_token = state.virtual_sol_reserves as f64 / state.virtual_token_reserves as f64;
        Ok(price_per_token)
    }

}