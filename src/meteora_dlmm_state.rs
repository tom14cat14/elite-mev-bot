//! Meteora DLMM (Dynamic Liquidity Market Maker) Pool State
//!
//! Fetches DLMM pool state for sandwich execution.
//! DLMM uses a bin-based liquidity distribution model similar to Uniswap V3.
//! Reference: https://github.com/MeteoraAg/dlmm-sdk

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Meteora DLMM program ID
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

/// Meteora DLMM pool state - essential fields for swap execution
#[derive(Debug, Clone)]
pub struct MeteoraDlmmPoolState {
    pub lb_pair: Pubkey,          // Pool ID
    pub token_x_mint: Pubkey,     // Token X mint
    pub token_y_mint: Pubkey,     // Token Y mint
    pub reserve_x: Pubkey,        // Reserve X account
    pub reserve_y: Pubkey,        // Reserve Y account
    pub oracle: Pubkey,           // Oracle account
    pub active_id: i32,           // Current active bin ID
    pub bin_step: u16,            // Bin step (price precision)
}

impl MeteoraDlmmPoolState {
    /// Parse DLMM pool state from account data
    ///
    /// Meteora DLMM uses a complex bin-based structure.
    /// Essential fields layout (estimated offsets from Anchor discriminator):
    /// - bin_step: u16 at offset 8
    /// - active_id: i32 at offset 10
    /// - reserve_x: Pubkey (32 bytes) at offset 16
    /// - reserve_y: Pubkey (32 bytes) at offset 48
    /// - token_x_mint: Pubkey (32 bytes) at offset 80
    /// - token_y_mint: Pubkey (32 bytes) at offset 112
    /// - oracle: Pubkey (32 bytes) at offset 144
    pub fn parse(lb_pair_pubkey: &Pubkey, data: &[u8]) -> Result<Self> {
        // Minimum size check
        if data.len() < 176 {
            return Err(anyhow!(
                "DLMM pool account data too small: {} bytes (expected at least 176)",
                data.len()
            ));
        }

        // Parse bin_step (u16 at offset 8)
        let bin_step = u16::from_le_bytes(
            data[8..10]
                .try_into()
                .map_err(|e| anyhow!("Failed to parse bin_step: {:?}", e))?,
        );

        // Parse active_id (i32 at offset 10)
        let active_id = i32::from_le_bytes(
            data[10..14]
                .try_into()
                .map_err(|e| anyhow!("Failed to parse active_id: {:?}", e))?,
        );

        // Parse essential pubkeys
        let reserve_x = Self::parse_pubkey(data, 16)?;
        let reserve_y = Self::parse_pubkey(data, 48)?;
        let token_x_mint = Self::parse_pubkey(data, 80)?;
        let token_y_mint = Self::parse_pubkey(data, 112)?;
        let oracle = Self::parse_pubkey(data, 144)?;

        Ok(MeteoraDlmmPoolState {
            lb_pair: *lb_pair_pubkey,
            token_x_mint,
            token_y_mint,
            reserve_x,
            reserve_y,
            oracle,
            active_id,
            bin_step,
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

    /// Fetch DLMM pool state from RPC
    pub fn fetch(rpc_client: &RpcClient, lb_pair_address: &Pubkey) -> Result<Self> {
        // Get pool account data
        let account = rpc_client
            .get_account(lb_pair_address)
            .map_err(|e| anyhow!("Failed to fetch DLMM pool account: {}", e))?;

        // Verify owner is Meteora DLMM program
        let dlmm_program = Pubkey::from_str(METEORA_DLMM_PROGRAM_ID)?;
        if account.owner != dlmm_program {
            return Err(anyhow!(
                "Account is not owned by Meteora DLMM program. Owner: {}, Expected: {}",
                account.owner,
                dlmm_program
            ));
        }

        // Parse pool state
        Self::parse(lb_pair_address, &account.data)
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

        let result = MeteoraDlmmPoolState::parse_pubkey(&data, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_pubkey);
    }

    #[test]
    fn test_parse_pool_state_too_small() {
        let lb_pair = Pubkey::new_unique();
        let data = vec![0u8; 100]; // Too small

        let result = MeteoraDlmmPoolState::parse(&lb_pair, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too small"));
    }
}
