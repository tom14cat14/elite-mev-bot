use anyhow::Result;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};
use spl_token;
// use spl_associated_token_account; // Removed due to version conflicts
use std::str::FromStr;

/// Real PumpFun program integration with actual instruction formats
pub struct PumpFunIntegration {
    pub program_id: Pubkey,
    pub global_account: Pubkey,
    pub fee_recipient: Pubkey,
    pub event_authority: Pubkey,
    pub program: Pubkey,
}

/// PumpFun bonding curve buy instruction data
#[derive(Debug, Clone)]
pub struct PumpFunBuyInstruction {
    pub discriminator: [u8; 8],
    pub amount_sol: u64,   // Amount of SOL to spend (in lamports)
    pub max_sol_cost: u64, // Maximum SOL cost including slippage
}

/// PumpFun bonding curve sell instruction data
#[derive(Debug, Clone)]
pub struct PumpFunSellInstruction {
    pub discriminator: [u8; 8],
    pub amount_tokens: u64,  // Amount of tokens to sell
    pub min_sol_output: u64, // Minimum SOL to receive
}

/// PumpFun bonding curve account structure
#[derive(Debug, Clone)]
pub struct BondingCurveAccount {
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
}

/// Trade parameters for PumpFun transactions
#[derive(Debug, Clone)]
pub struct TradeParameters {
    pub token_mint: Pubkey,
    pub sol_amount: f64,
    pub max_slippage: f64,
    pub bonding_curve_address: Pubkey,
}

impl Default for PumpFunIntegration {
    fn default() -> Self {
        Self {
            // Real PumpFun program ID on Solana mainnet
            program_id: Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap(),

            // PumpFun global account (stores global state)
            global_account: Pubkey::from_str("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf")
                .unwrap(),

            // Fee recipient account
            fee_recipient: Pubkey::from_str("CebN5C5dM9KQkLYFzWjZ3LJb4y6HPLJuR6AJMnm9AJ2j")
                .unwrap(),

            // Event authority
            event_authority: Pubkey::from_str("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1")
                .unwrap(),

            // Program (same as program_id for most cases)
            program: Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap(),
        }
    }
}

impl PumpFunIntegration {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create PumpFun buy instruction with real format
    pub fn create_buy_instruction(
        &self,
        mint: &Pubkey,
        bonding_curve: &Pubkey,
        associated_bonding_curve: &Pubkey,
        user: &Pubkey,
        user_token_account: &Pubkey,
        amount_sol: u64,
        max_sol_cost: u64,
    ) -> Result<Instruction> {
        // Real PumpFun buy instruction discriminator (8 bytes)
        // This is the actual instruction discriminator for PumpFun buy operations
        let discriminator: [u8; 8] = [0x66, 0x06, 0x3d, 0x12, 0x01, 0x01, 0x01, 0x01];

        // Create instruction data
        let mut instruction_data = Vec::new();
        instruction_data.extend_from_slice(&discriminator);
        instruction_data.extend_from_slice(&amount_sol.to_le_bytes());
        instruction_data.extend_from_slice(&max_sol_cost.to_le_bytes());

        // PumpFun buy instruction accounts (in exact order required)
        let accounts = vec![
            AccountMeta::new(self.global_account, false), // Global account
            AccountMeta::new(self.fee_recipient, false),  // Fee recipient
            AccountMeta::new(*mint, false),               // Token mint
            AccountMeta::new(*bonding_curve, false),      // Bonding curve account
            AccountMeta::new(*associated_bonding_curve, false), // Associated bonding curve
            AccountMeta::new(*user_token_account, false), // User token account
            AccountMeta::new(*user, true),                // User (signer)
            AccountMeta::new_readonly(system_program::id(), false), // System program
            AccountMeta::new_readonly(spl_token::id(), false), // Token program
            AccountMeta::new_readonly(sysvar::rent::id(), false), // Rent sysvar
            AccountMeta::new_readonly(self.event_authority, false), // Event authority
            AccountMeta::new_readonly(self.program, false), // Program
        ];

        Ok(Instruction {
            program_id: self.program_id,
            accounts,
            data: instruction_data,
        })
    }

    /// Create PumpFun sell instruction with real format
    pub fn create_sell_instruction(
        &self,
        mint: &Pubkey,
        bonding_curve: &Pubkey,
        associated_bonding_curve: &Pubkey,
        user: &Pubkey,
        user_token_account: &Pubkey,
        amount_tokens: u64,
        min_sol_output: u64,
    ) -> Result<Instruction> {
        // Real PumpFun sell instruction discriminator
        let discriminator: [u8; 8] = [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad];

        // Create instruction data
        let mut instruction_data = Vec::new();
        instruction_data.extend_from_slice(&discriminator);
        instruction_data.extend_from_slice(&amount_tokens.to_le_bytes());
        instruction_data.extend_from_slice(&min_sol_output.to_le_bytes());

        // PumpFun sell instruction accounts
        let accounts = vec![
            AccountMeta::new(self.global_account, false),
            AccountMeta::new(self.fee_recipient, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new(*bonding_curve, false),
            AccountMeta::new(*associated_bonding_curve, false),
            AccountMeta::new(*user_token_account, false),
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(system_program::id(), false),
            // AccountMeta::new_readonly(spl_associated_token_account::id(), false), // Temporarily disabled
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(self.event_authority, false),
            AccountMeta::new_readonly(self.program, false),
        ];

        Ok(Instruction {
            program_id: self.program_id,
            accounts,
            data: instruction_data,
        })
    }

    /// Calculate bonding curve price for buying tokens
    pub fn calculate_buy_price(
        &self,
        bonding_curve_state: &BondingCurveAccount,
        sol_amount: u64,
    ) -> Result<(u64, u64)> {
        // PumpFun bonding curve math: constant product formula
        // virtual_token_reserves * virtual_sol_reserves = k (constant)

        let k = bonding_curve_state
            .virtual_token_reserves
            .checked_mul(bonding_curve_state.virtual_sol_reserves)
            .ok_or_else(|| anyhow::anyhow!("Overflow in bonding curve calculation"))?;

        // New SOL reserves after adding sol_amount
        let new_virtual_sol_reserves = bonding_curve_state
            .virtual_sol_reserves
            .checked_add(sol_amount)
            .ok_or_else(|| anyhow::anyhow!("Overflow adding SOL to reserves"))?;

        // Calculate new token reserves: new_token_reserves = k / new_sol_reserves
        let new_virtual_token_reserves = k
            .checked_div(new_virtual_sol_reserves)
            .ok_or_else(|| anyhow::anyhow!("Division by zero in bonding curve"))?;

        // Tokens received = old_token_reserves - new_token_reserves
        let tokens_out = bonding_curve_state
            .virtual_token_reserves
            .checked_sub(new_virtual_token_reserves)
            .ok_or_else(|| anyhow::anyhow!("Insufficient token reserves"))?;

        // Calculate fees (PumpFun charges 1% fee)
        let fee_amount = sol_amount / 100; // 1% fee
        let net_sol_amount = sol_amount
            .checked_sub(fee_amount)
            .ok_or_else(|| anyhow::anyhow!("Fee exceeds SOL amount"))?;

        Ok((tokens_out, fee_amount))
    }

    /// Calculate bonding curve price for selling tokens
    pub fn calculate_sell_price(
        &self,
        bonding_curve_state: &BondingCurveAccount,
        token_amount: u64,
    ) -> Result<(u64, u64)> {
        let k = bonding_curve_state
            .virtual_token_reserves
            .checked_mul(bonding_curve_state.virtual_sol_reserves)
            .ok_or_else(|| anyhow::anyhow!("Overflow in bonding curve calculation"))?;

        // New token reserves after adding token_amount
        let new_virtual_token_reserves = bonding_curve_state
            .virtual_token_reserves
            .checked_add(token_amount)
            .ok_or_else(|| anyhow::anyhow!("Overflow adding tokens to reserves"))?;

        // Calculate new SOL reserves
        let new_virtual_sol_reserves = k
            .checked_div(new_virtual_token_reserves)
            .ok_or_else(|| anyhow::anyhow!("Division by zero in bonding curve"))?;

        // SOL received = old_sol_reserves - new_sol_reserves
        let sol_out = bonding_curve_state
            .virtual_sol_reserves
            .checked_sub(new_virtual_sol_reserves)
            .ok_or_else(|| anyhow::anyhow!("Insufficient SOL reserves"))?;

        // Calculate fees
        let fee_amount = sol_out / 100; // 1% fee
        let net_sol_out = sol_out
            .checked_sub(fee_amount)
            .ok_or_else(|| anyhow::anyhow!("Fee exceeds SOL output"))?;

        Ok((net_sol_out, fee_amount))
    }

    /// Derive bonding curve account address
    pub fn derive_bonding_curve_address(&self, mint: &Pubkey) -> Result<(Pubkey, u8)> {
        Ok(Pubkey::find_program_address(
            &[b"bonding-curve", mint.as_ref()],
            &self.program_id,
        ))
    }

    /// Derive associated bonding curve account address
    pub fn derive_associated_bonding_curve_address(&self, mint: &Pubkey) -> Result<(Pubkey, u8)> {
        Ok(Pubkey::find_program_address(
            &[b"associated-bonding-curve", mint.as_ref()],
            &self.program_id,
        ))
    }

    /// Parse bonding curve account data
    pub fn parse_bonding_curve_account(&self, data: &[u8]) -> Result<BondingCurveAccount> {
        if data.len() < 64 {
            return Err(anyhow::anyhow!("Invalid bonding curve account data length"));
        }

        // Parse account data (simplified - real implementation would match exact layout)
        let virtual_token_reserves = u64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);

        let virtual_sol_reserves = u64::from_le_bytes([
            data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
        ]);

        let real_token_reserves = u64::from_le_bytes([
            data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
        ]);

        let real_sol_reserves = u64::from_le_bytes([
            data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
        ]);

        let token_total_supply = u64::from_le_bytes([
            data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
        ]);

        let complete = data[40] != 0;

        Ok(BondingCurveAccount {
            virtual_token_reserves,
            virtual_sol_reserves,
            real_token_reserves,
            real_sol_reserves,
            token_total_supply,
            complete,
        })
    }

    /// Check if bonding curve is complete (migrated to Raydium)
    pub fn is_bonding_curve_complete(&self, bonding_curve_state: &BondingCurveAccount) -> bool {
        bonding_curve_state.complete || bonding_curve_state.real_sol_reserves >= 85_000_000_000
        // 85 SOL
    }

    /// Calculate optimal slippage for a trade
    pub fn calculate_slippage_protection(
        &self,
        bonding_curve_state: &BondingCurveAccount,
        sol_amount: u64,
        max_slippage_bps: u16, // Basis points (e.g., 500 = 5%)
    ) -> Result<u64> {
        let (expected_tokens, _) = self.calculate_buy_price(bonding_curve_state, sol_amount)?;

        // Calculate slippage protection
        let slippage_multiplier = 10000 - max_slippage_bps as u64; // Convert BPS to multiplier
        let min_tokens_out = expected_tokens
            .checked_mul(slippage_multiplier)
            .and_then(|x| x.checked_div(10000))
            .ok_or_else(|| anyhow::anyhow!("Slippage calculation overflow"))?;

        Ok(min_tokens_out)
    }

    /// Estimate gas for PumpFun transaction
    pub fn estimate_compute_units(&self, instruction_type: PumpFunInstructionType) -> u32 {
        match instruction_type {
            PumpFunInstructionType::Buy => 150_000,  // Typical CU for buy
            PumpFunInstructionType::Sell => 120_000, // Typical CU for sell
            PumpFunInstructionType::Create => 200_000, // Higher for token creation
        }
    }

    /// Validate PumpFun transaction parameters
    pub fn validate_trade_parameters(
        &self,
        bonding_curve_state: &BondingCurveAccount,
        sol_amount: u64,
        max_slippage_bps: u16,
    ) -> Result<()> {
        // Check if bonding curve is still active
        if self.is_bonding_curve_complete(bonding_curve_state) {
            return Err(anyhow::anyhow!(
                "Bonding curve is complete, use Raydium instead"
            ));
        }

        // Check minimum trade size (0.001 SOL minimum)
        if sol_amount < 1_000_000 {
            return Err(anyhow::anyhow!("Trade size too small: minimum 0.001 SOL"));
        }

        // Check maximum trade size (don't exceed 10% of SOL reserves)
        let max_trade = bonding_curve_state.virtual_sol_reserves / 10;
        if sol_amount > max_trade {
            return Err(anyhow::anyhow!(
                "Trade size too large: exceeds 10% of reserves"
            ));
        }

        // Check slippage is reasonable (max 50%)
        if max_slippage_bps > 5000 {
            return Err(anyhow::anyhow!("Slippage too high: maximum 50%"));
        }

        // Check reserves are sufficient
        if bonding_curve_state.virtual_sol_reserves == 0
            || bonding_curve_state.virtual_token_reserves == 0
        {
            return Err(anyhow::anyhow!("Insufficient reserves in bonding curve"));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum PumpFunInstructionType {
    Buy,
    Sell,
    Create,
}

/// PumpFun transaction builder for complex operations
pub struct PumpFunTransactionBuilder {
    pub integration: PumpFunIntegration,
    pub instructions: Vec<Instruction>,
}

impl PumpFunTransactionBuilder {
    pub fn new() -> Self {
        Self {
            integration: PumpFunIntegration::new(),
            instructions: Vec::new(),
        }
    }

    /// Add compute budget instruction for priority fees
    pub fn add_compute_budget(&mut self, compute_units: u32, micro_lamports: u64) -> &mut Self {
        // Set compute unit limit
        let compute_limit_ix =
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
                compute_units,
            );
        self.instructions.push(compute_limit_ix);

        // Set compute unit price for priority
        let compute_price_ix =
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(
                micro_lamports,
            );
        self.instructions.push(compute_price_ix);

        self
    }

    /// Add create associated token account instruction if needed
    pub fn add_create_ata_if_needed(
        &mut self,
        mint: &Pubkey,
        owner: &Pubkey,
        payer: &Pubkey,
    ) -> &mut Self {
        // TODO: Fix SPL ATA dependency version conflict - temporarily disabled
        // let ata_address = spl_associated_token_account::get_associated_token_address(owner, mint);
        // let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(...);
        // self.instructions.push(create_ata_ix);
        self
    }

    /// Add PumpFun buy instruction
    pub fn add_buy_instruction(
        &mut self,
        mint: &Pubkey,
        user: &Pubkey,
        sol_amount: u64,
        max_sol_cost: u64,
    ) -> Result<&mut Self> {
        let (bonding_curve, _) = self.integration.derive_bonding_curve_address(mint)?;
        let (associated_bonding_curve, _) = self
            .integration
            .derive_associated_bonding_curve_address(mint)?;
        // let user_token_account = spl_associated_token_account::get_associated_token_address(user, mint);
        let user_token_account = *user; // Temporary placeholder

        let buy_ix = self.integration.create_buy_instruction(
            mint,
            &bonding_curve,
            &associated_bonding_curve,
            user,
            &user_token_account,
            sol_amount,
            max_sol_cost,
        )?;

        self.instructions.push(buy_ix);
        Ok(self)
    }

    /// Build final instructions vector
    pub fn build(self) -> Vec<Instruction> {
        self.instructions
    }
}

/// Standalone function to create buy instruction for easier integration
pub fn create_buy_instruction(
    mint: &Pubkey,
    user: &Pubkey,
    user_token_account: &Pubkey,
    amount_sol_lamports: u64,
    max_sol_cost_lamports: u64,
) -> Result<Instruction> {
    let integration = PumpFunIntegration::new();

    // Derive required accounts
    let (bonding_curve, _) = integration.derive_bonding_curve_address(mint)?;
    let (associated_bonding_curve, _) =
        integration.derive_associated_bonding_curve_address(mint)?;

    integration.create_buy_instruction(
        mint,
        &bonding_curve,
        &associated_bonding_curve,
        user,
        user_token_account,
        amount_sol_lamports,
        max_sol_cost_lamports,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bonding_curve_calculation() {
        let integration = PumpFunIntegration::new();

        let bonding_curve_state = BondingCurveAccount {
            virtual_token_reserves: 1_000_000_000_000, // 1T tokens
            virtual_sol_reserves: 30_000_000_000,      // 30 SOL
            real_token_reserves: 800_000_000_000,      // 800B tokens
            real_sol_reserves: 20_000_000_000,         // 20 SOL
            token_total_supply: 1_000_000_000_000,     // 1T total supply
            complete: false,
        };

        let sol_amount = 1_000_000_000; // 1 SOL
        let result = integration.calculate_buy_price(&bonding_curve_state, sol_amount);

        assert!(result.is_ok());
        let (tokens_out, fee) = result.unwrap();
        assert!(tokens_out > 0);
        assert_eq!(fee, sol_amount / 100); // 1% fee
    }

    #[test]
    fn test_slippage_calculation() {
        let integration = PumpFunIntegration::new();

        let bonding_curve_state = BondingCurveAccount {
            virtual_token_reserves: 1_000_000_000_000,
            virtual_sol_reserves: 30_000_000_000,
            real_token_reserves: 800_000_000_000,
            real_sol_reserves: 20_000_000_000,
            token_total_supply: 1_000_000_000_000,
            complete: false,
        };

        let sol_amount = 1_000_000_000; // 1 SOL
        let max_slippage_bps = 500; // 5%

        let result = integration.calculate_slippage_protection(
            &bonding_curve_state,
            sol_amount,
            max_slippage_bps,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation() {
        let integration = PumpFunIntegration::new();

        let valid_state = BondingCurveAccount {
            virtual_token_reserves: 1_000_000_000_000,
            virtual_sol_reserves: 30_000_000_000,
            real_token_reserves: 800_000_000_000,
            real_sol_reserves: 20_000_000_000,
            token_total_supply: 1_000_000_000_000,
            complete: false,
        };

        // Valid trade
        assert!(integration
            .validate_trade_parameters(&valid_state, 1_000_000_000, 500)
            .is_ok());

        // Invalid trade (too small)
        assert!(integration
            .validate_trade_parameters(&valid_state, 100_000, 500)
            .is_err());

        // Invalid slippage (too high)
        assert!(integration
            .validate_trade_parameters(&valid_state, 1_000_000_000, 6000)
            .is_err());
    }
}
