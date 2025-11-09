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
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub observation_key: Pubkey,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
}

impl RaydiumClmmPoolState {
    /// Parse CLMM pool state from account data
    ///
    /// Layout (from Raydium CLMM IDL):
    /// - bump: [u8; 1] at offset 0
    /// - amm_config: Pubkey (32 bytes) at offset 1
    /// - owner: Pubkey (32 bytes) at offset 33
    /// - token_mint_0: Pubkey (32 bytes) at offset 65
    /// - token_mint_1: Pubkey (32 bytes) at offset 97
    /// - token_vault_0: Pubkey (32 bytes) at offset 129
    /// - token_vault_1: Pubkey (32 bytes) at offset 161
    /// - observation_key: Pubkey (32 bytes) at offset 193
    /// - tick_spacing: u16 (2 bytes) at offset 225
    /// - liquidity: u128 (16 bytes) at offset 227
    /// - sqrt_price_x64: u128 (16 bytes) at offset 243
    /// - tick_current: i32 (4 bytes) at offset 259
    /// ... (rest of fields not needed for basic swap)
    pub fn parse(pool_pubkey: &Pubkey, data: &[u8]) -> Result<Self> {
        // Minimum size check
        if data.len() < 263 {
            return Err(anyhow!(
                "CLMM pool account data too small: {} bytes (expected at least 263)",
                data.len()
            ));
        }

        // Parse pubkeys at their offsets
        let token_mint_a = Self::parse_pubkey(data, 65)?;
        let token_mint_b = Self::parse_pubkey(data, 97)?;
        let token_vault_a = Self::parse_pubkey(data, 129)?;
        let token_vault_b = Self::parse_pubkey(data, 161)?;
        let observation_key = Self::parse_pubkey(data, 193)?;

        // Parse sqrt_price_x64 (u128 at offset 243)
        let sqrt_price_x64 = u128::from_le_bytes(
            data[243..259]
                .try_into()
                .map_err(|e| anyhow!("Failed to parse sqrt_price_x64: {:?}", e))?,
        );

        // Parse tick_current (i32 at offset 259)
        let tick_current = i32::from_le_bytes(
            data[259..263]
                .try_into()
                .map_err(|e| anyhow!("Failed to parse tick_current: {:?}", e))?,
        );

        Ok(RaydiumClmmPoolState {
            pool_id: *pool_pubkey,
            token_mint_a,
            token_mint_b,
            token_vault_a,
            token_vault_b,
            observation_key,
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
