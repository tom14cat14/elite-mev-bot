//! PumpSwap Bonding Curve Pool State
//!
//! Fetches PumpSwap bonding curve state for sandwich execution.
//! PumpSwap uses a simple bonding curve model (pump.fun / PumpSwap).

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// PumpSwap program ID
pub const PUMPSWAP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// PumpSwap bonding curve state - essential fields for swap execution
#[derive(Debug, Clone)]
pub struct PumpSwapBondingCurveState {
    pub bonding_curve: Pubkey,            // Bonding curve address
    pub token_mint: Pubkey,               // Token mint address
    pub associated_bonding_curve: Pubkey, // Associated bonding curve token account
    pub global: Pubkey,                   // Global state account
    pub fee_recipient: Pubkey,            // Fee recipient account
}

impl PumpSwapBondingCurveState {
    /// Parse PumpSwap bonding curve state from account data
    ///
    /// PumpSwap bonding curve uses a simple structure.
    /// Essential fields layout (estimated offsets):
    /// - virtual_token_reserves: u64 at offset 8
    /// - virtual_sol_reserves: u64 at offset 16
    /// - real_token_reserves: u64 at offset 24
    /// - real_sol_reserves: u64 at offset 32
    /// - token_total_supply: u64 at offset 40
    /// - complete: bool at offset 48
    pub fn parse(bonding_curve_pubkey: &Pubkey, data: &[u8]) -> Result<Self> {
        // Minimum size check (at least 49 bytes for basic fields)
        if data.len() < 49 {
            return Err(anyhow!(
                "PumpSwap bonding curve account data too small: {} bytes (expected at least 49)",
                data.len()
            ));
        }

        // For bonding curve state, we primarily need the bonding curve address
        // The token mint and other accounts will be derived or passed in during swap construction
        // This is a simplified parser - full implementation would parse all fields

        // We'll need to derive the associated accounts when building swaps
        Ok(PumpSwapBondingCurveState {
            bonding_curve: *bonding_curve_pubkey,
            token_mint: Pubkey::default(), // Will be set when fetching
            associated_bonding_curve: Pubkey::default(), // Will be derived
            global: Pubkey::default(),     // Will be set from constants
            fee_recipient: Pubkey::default(), // Will be set from constants
        })
    }

    /// Fetch PumpSwap bonding curve state from RPC
    ///
    /// # Arguments
    /// * `rpc_client` - RPC client for blockchain queries
    /// * `bonding_curve_address` - The bonding curve account address
    /// * `token_mint` - The token mint address (needed for PDA derivations)
    pub fn fetch(
        rpc_client: &RpcClient,
        bonding_curve_address: &Pubkey,
        token_mint: &Pubkey,
    ) -> Result<Self> {
        // Get bonding curve account data
        let account = rpc_client
            .get_account(bonding_curve_address)
            .map_err(|e| anyhow!("Failed to fetch PumpSwap bonding curve account: {}", e))?;

        // Verify owner is PumpSwap program
        let pumpswap_program = Pubkey::from_str(PUMPSWAP_PROGRAM_ID)?;
        if account.owner != pumpswap_program {
            return Err(anyhow!(
                "Account is not owned by PumpSwap program. Owner: {}, Expected: {}",
                account.owner,
                pumpswap_program
            ));
        }

        // Parse bonding curve state
        let mut state = Self::parse(bonding_curve_address, &account.data)?;
        state.token_mint = *token_mint;

        // Derive associated bonding curve (PDA for bonding curve's token account)
        let (associated_bonding_curve, _bump) = Pubkey::find_program_address(
            &[b"bonding-curve", token_mint.as_ref()],
            &pumpswap_program,
        );
        state.associated_bonding_curve = associated_bonding_curve;

        // Set global and fee_recipient from known constants
        // These are standard PumpSwap addresses
        state.global = Pubkey::from_str("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf")
            .unwrap_or(Pubkey::default());
        state.fee_recipient = Pubkey::from_str("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM")
            .unwrap_or(Pubkey::default());

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bonding_curve_state_too_small() {
        let bonding_curve = Pubkey::new_unique();
        let data = vec![0u8; 30]; // Too small

        let result = PumpSwapBondingCurveState::parse(&bonding_curve, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too small"));
    }

    #[test]
    fn test_parse_bonding_curve_state() {
        let bonding_curve = Pubkey::new_unique();
        let mut data = vec![0u8; 100]; // Sufficient size

        // Set some dummy data
        data[8..16].copy_from_slice(&1_000_000_000u64.to_le_bytes()); // virtual_token_reserves
        data[16..24].copy_from_slice(&500_000_000u64.to_le_bytes()); // virtual_sol_reserves

        let result = PumpSwapBondingCurveState::parse(&bonding_curve, &data);
        assert!(result.is_ok());

        let state = result.unwrap();
        assert_eq!(state.bonding_curve, bonding_curve);
    }
}
