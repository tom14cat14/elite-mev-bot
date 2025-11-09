//! Orca Whirlpools (Concentrated Liquidity Market Maker) Pool State
//!
//! Fetches Whirlpool pool state for sandwich execution.
//! Reference: https://github.com/orca-so/whirlpools

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Orca Whirlpools program ID
pub const ORCA_WHIRLPOOLS_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

/// Orca Whirlpool pool state - essential fields for swap execution
#[derive(Debug, Clone)]
pub struct OrcaWhirlpoolState {
    pub whirlpool: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub tick_spacing: u16,
    pub sqrt_price: u128,
    pub tick_current_index: i32,
    pub oracle: Pubkey,
}

impl OrcaWhirlpoolState {
    /// Parse Whirlpool pool state from account data
    ///
    /// Layout (from Orca Whirlpools SDK):
    /// - whirlpools_config: Pubkey (32 bytes) at offset 8
    /// - whirlpool_bump: [u8; 1] at offset 40
    /// - tick_spacing: u16 (2 bytes) at offset 41
    /// - tick_spacing_seed: [u8; 2] at offset 43
    /// - fee_rate: u16 at offset 45
    /// - protocol_fee_rate: u16 at offset 47
    /// - liquidity: u128 (16 bytes) at offset 49
    /// - sqrt_price: u128 (16 bytes) at offset 65
    /// - tick_current_index: i32 (4 bytes) at offset 81
    /// - protocol_fee_owed_a: u64 at offset 85
    /// - protocol_fee_owed_b: u64 at offset 93
    /// - token_mint_a: Pubkey (32 bytes) at offset 101
    /// - token_vault_a: Pubkey (32 bytes) at offset 133
    /// - fee_growth_global_a: u128 at offset 165
    /// - token_mint_b: Pubkey (32 bytes) at offset 181
    /// - token_vault_b: Pubkey (32 bytes) at offset 213
    /// - fee_growth_global_b: u128 at offset 245
    /// - reward_last_updated_timestamp: u64 at offset 261
    /// - reward_infos: [RewardInfo; 3] at offset 269 (each 128 bytes)
    pub fn parse(whirlpool_pubkey: &Pubkey, data: &[u8]) -> Result<Self> {
        // Minimum size check (at least 400 bytes for basic fields)
        if data.len() < 400 {
            return Err(anyhow!(
                "Whirlpool account data too small: {} bytes (expected at least 400)",
                data.len()
            ));
        }

        // Skip 8-byte Anchor discriminator, parse essential fields
        let tick_spacing = u16::from_le_bytes(
            data[41..43]
                .try_into()
                .map_err(|e| anyhow!("Failed to parse tick_spacing: {:?}", e))?,
        );

        let sqrt_price = u128::from_le_bytes(
            data[65..81]
                .try_into()
                .map_err(|e| anyhow!("Failed to parse sqrt_price: {:?}", e))?,
        );

        let tick_current_index = i32::from_le_bytes(
            data[81..85]
                .try_into()
                .map_err(|e| anyhow!("Failed to parse tick_current_index: {:?}", e))?,
        );

        let token_mint_a = Self::parse_pubkey(data, 101)?;
        let token_vault_a = Self::parse_pubkey(data, 133)?;
        let token_mint_b = Self::parse_pubkey(data, 181)?;
        let token_vault_b = Self::parse_pubkey(data, 213)?;

        // Oracle is derived from whirlpool address (standard Orca pattern)
        // For now, use the whirlpool address itself (will derive properly in swap builder)
        let oracle = *whirlpool_pubkey;

        Ok(OrcaWhirlpoolState {
            whirlpool: *whirlpool_pubkey,
            token_mint_a,
            token_mint_b,
            token_vault_a,
            token_vault_b,
            tick_spacing,
            sqrt_price,
            tick_current_index,
            oracle,
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

    /// Fetch Whirlpool pool state from RPC
    pub fn fetch(rpc_client: &RpcClient, whirlpool_address: &Pubkey) -> Result<Self> {
        // Get pool account data
        let account = rpc_client
            .get_account(whirlpool_address)
            .map_err(|e| anyhow!("Failed to fetch Whirlpool account: {}", e))?;

        // Verify owner is Orca Whirlpools program
        let whirlpools_program = Pubkey::from_str(ORCA_WHIRLPOOLS_PROGRAM_ID)?;
        if account.owner != whirlpools_program {
            return Err(anyhow!(
                "Account is not owned by Orca Whirlpools program. Owner: {}, Expected: {}",
                account.owner,
                whirlpools_program
            ));
        }

        // Parse pool state
        Self::parse(whirlpool_address, &account.data)
    }

    /// Get the current tick array index for a given tick
    pub fn get_tick_array_start_index(&self, tick_index: i32) -> i32 {
        let tick_spacing = self.tick_spacing as i32;
        let tick_array_size = 88; // Orca Whirlpools standard tick array size
        let ticks_in_array = tick_array_size * tick_spacing;

        // Round down to nearest tick array boundary
        (tick_index / ticks_in_array) * ticks_in_array
    }

    /// Derive tick array PDAs for the current price
    /// Returns (tick_array_0, tick_array_1, tick_array_2) - the 3 tick arrays needed for swaps
    pub fn derive_tick_arrays(&self, a_to_b: bool) -> Result<(Pubkey, Pubkey, Pubkey)> {
        let program_id = Pubkey::from_str(ORCA_WHIRLPOOLS_PROGRAM_ID)?;
        let current_start = self.get_tick_array_start_index(self.tick_current_index);

        // For A→B swaps, we need current and lower tick arrays
        // For B→A swaps, we need current and higher tick arrays
        let tick_spacing = self.tick_spacing as i32;
        let tick_array_size = 88;
        let ticks_in_array = tick_array_size * tick_spacing;

        let (start_0, start_1, start_2) = if a_to_b {
            // Swapping A→B (price decreasing), need lower ticks
            (current_start, current_start - ticks_in_array, current_start - 2 * ticks_in_array)
        } else {
            // Swapping B→A (price increasing), need higher ticks
            (current_start, current_start + ticks_in_array, current_start + 2 * ticks_in_array)
        };

        let tick_array_0 = Self::derive_tick_array_pda(&program_id, &self.whirlpool, start_0)?;
        let tick_array_1 = Self::derive_tick_array_pda(&program_id, &self.whirlpool, start_1)?;
        let tick_array_2 = Self::derive_tick_array_pda(&program_id, &self.whirlpool, start_2)?;

        Ok((tick_array_0, tick_array_1, tick_array_2))
    }

    /// Derive a tick array PDA
    fn derive_tick_array_pda(program_id: &Pubkey, whirlpool: &Pubkey, start_index: i32) -> Result<Pubkey> {
        let seeds = &[
            b"tick_array",
            whirlpool.as_ref(),
            &start_index.to_le_bytes(),
        ];

        let (pda, _bump) = Pubkey::find_program_address(seeds, program_id);
        Ok(pda)
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

        let result = OrcaWhirlpoolState::parse_pubkey(&data, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_pubkey);
    }

    #[test]
    fn test_parse_pool_state_too_small() {
        let whirlpool = Pubkey::new_unique();
        let data = vec![0u8; 100]; // Too small

        let result = OrcaWhirlpoolState::parse(&whirlpool, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too small"));
    }

    #[test]
    fn test_tick_array_derivation() {
        let whirlpool = Pubkey::new_unique();
        let pool_state = OrcaWhirlpoolState {
            whirlpool,
            token_mint_a: Pubkey::new_unique(),
            token_mint_b: Pubkey::new_unique(),
            token_vault_a: Pubkey::new_unique(),
            token_vault_b: Pubkey::new_unique(),
            tick_spacing: 64,
            sqrt_price: 1000000000000000000,
            tick_current_index: 0,
            oracle: whirlpool,
        };

        // Test A→B swap (descending ticks)
        let result = pool_state.derive_tick_arrays(true);
        assert!(result.is_ok());
        let (tick_0, tick_1, tick_2) = result.unwrap();

        // All should be valid pubkeys
        assert_ne!(tick_0, Pubkey::default());
        assert_ne!(tick_1, Pubkey::default());
        assert_ne!(tick_2, Pubkey::default());

        // Test B→A swap (ascending ticks)
        let result = pool_state.derive_tick_arrays(false);
        assert!(result.is_ok());
    }
}
