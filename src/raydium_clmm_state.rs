//! Raydium CLMM (Concentrated Liquidity Market Maker) Pool State
//!
//! Fetches CLMM pool state for sandwich execution.
//! Reference: https://github.com/raydium-io/raydium-clmm

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Raydium CLMM program ID
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

/// Raydium CLMM pool state - essential fields for swap execution
#[derive(Debug, Clone)]
pub struct RaydiumClmmPoolState {
    pub pool_id: Pubkey,
    pub amm_config: Pubkey,      // Added: Required for swap instruction
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub observation_key: Pubkey,
    pub tick_spacing: u16,       // Added: Required for tick_array calculation
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
}

impl RaydiumClmmPoolState {
    /// Parse CLMM pool state from account data
    ///
    /// ✅ CORRECTED (2025-12-07): Layout includes 8-byte discriminator + mint_decimals fields
    /// Layout (from Raydium CLMM PoolState struct):
    /// - discriminator: [u8; 8] at offset 0 (Anchor account discriminator)
    /// - bump: [u8; 1] at offset 8
    /// - amm_config: Pubkey (32 bytes) at offset 9
    /// - owner: Pubkey (32 bytes) at offset 41
    /// - token_mint_0: Pubkey (32 bytes) at offset 73
    /// - token_mint_1: Pubkey (32 bytes) at offset 105
    /// - token_vault_0: Pubkey (32 bytes) at offset 137
    /// - token_vault_1: Pubkey (32 bytes) at offset 169
    /// - observation_key: Pubkey (32 bytes) at offset 201
    /// - mint_decimals_0: u8 at offset 233  ← MISSED BEFORE!
    /// - mint_decimals_1: u8 at offset 234  ← MISSED BEFORE!
    /// - tick_spacing: u16 (2 bytes) at offset 235  ← CORRECTED (+2)
    /// - liquidity: u128 (16 bytes) at offset 237
    /// - sqrt_price_x64: u128 (16 bytes) at offset 253  ← CORRECTED (+2)
    /// - tick_current: i32 (4 bytes) at offset 269  ← CORRECTED (+2)
    /// ... (rest of fields not needed for basic swap)
    pub fn parse(pool_pubkey: &Pubkey, data: &[u8]) -> Result<Self> {
        // Minimum size check (with 8-byte discriminator + mint_decimals)
        if data.len() < 273 {
            return Err(anyhow!(
                "CLMM pool account data too small: {} bytes (expected at least 273)",
                data.len()
            ));
        }

        // Parse pubkeys at their offsets (all +8 from original due to discriminator)
        let amm_config = Self::parse_pubkey(data, 9)?;  // offset 9 (after discriminator + bump)
        let token_mint_a = Self::parse_pubkey(data, 73)?;
        let token_mint_b = Self::parse_pubkey(data, 105)?;
        let token_vault_a = Self::parse_pubkey(data, 137)?;
        let token_vault_b = Self::parse_pubkey(data, 169)?;
        let observation_key = Self::parse_pubkey(data, 201)?;

        // Parse tick_spacing (u16 at offset 235 - after mint_decimals_0 and mint_decimals_1)
        let tick_spacing = u16::from_le_bytes(
            data[235..237]
                .try_into()
                .map_err(|e| anyhow!("Failed to parse tick_spacing: {:?}", e))?,
        );

        // Parse sqrt_price_x64 (u128 at offset 253)
        let sqrt_price_x64 = u128::from_le_bytes(
            data[253..269]
                .try_into()
                .map_err(|e| anyhow!("Failed to parse sqrt_price_x64: {:?}", e))?,
        );

        // Parse tick_current (i32 at offset 269)
        let tick_current = i32::from_le_bytes(
            data[269..273]
                .try_into()
                .map_err(|e| anyhow!("Failed to parse tick_current: {:?}", e))?,
        );

        // DEBUG: pool parsing - using info! to ensure it appears
        tracing::info!(
            "📊 CLMM parsed: amm_config={}, tick_current={}, tick_spacing={}",
            amm_config, tick_current, tick_spacing
        );

        Ok(RaydiumClmmPoolState {
            pool_id: *pool_pubkey,
            amm_config,
            token_mint_a,
            token_mint_b,
            token_vault_a,
            token_vault_b,
            observation_key,
            tick_spacing,
            sqrt_price_x64,
            tick_current,
        })
    }

    /// Helper to parse a pubkey from data at an offset
    fn parse_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
        if data.len() < offset + 32 {
            return Err(anyhow!(
                "Not enough data to parse pubkey at offset {}: {} bytes available",
                offset,
                data.len()
            ));
        }

        let pubkey_bytes: [u8; 32] = data[offset..offset + 32]
            .try_into()
            .map_err(|e| anyhow!("Failed to parse pubkey: {:?}", e))?;

        Ok(Pubkey::new_from_array(pubkey_bytes))
    }

    /// Fetch CLMM pool state from RPC
    pub fn fetch(rpc_client: &RpcClient, pool_address: &Pubkey) -> Result<Self> {
        // Get pool account data
        let account = rpc_client
            .get_account(pool_address)
            .map_err(|e| anyhow!("Failed to fetch CLMM pool account: {}", e))?;

        // Verify owner is Raydium CLMM program
        let clmm_program = Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID)?;
        if account.owner != clmm_program {
            return Err(anyhow!(
                "Account is not owned by Raydium CLMM program. Owner: {}, Expected: {}",
                account.owner,
                clmm_program
            ));
        }

        // Parse pool state
        Self::parse(pool_address, &account.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pubkey() {
        let mut data = vec![0u8; 64];
        let test_pubkey = Pubkey::new_unique();
        data[0..32].copy_from_slice(test_pubkey.as_ref());

        let result = RaydiumClmmPoolState::parse_pubkey(&data, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_pubkey);
    }

    #[test]
    fn test_parse_pool_state_too_small() {
        let pool = Pubkey::new_unique();
        let data = vec![0u8; 100]; // Too small

        let result = RaydiumClmmPoolState::parse(&pool, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too small"));
    }
}
