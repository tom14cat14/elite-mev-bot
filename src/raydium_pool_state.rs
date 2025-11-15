//! Raydium AMM V4 Pool State Query
//!
//! Fetches pool state from on-chain to get all required accounts for swap instructions.

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Raydium AMM V4 program ID
pub const RAYDIUM_AMM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Token program ID
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// Serum DEX V3 program ID
pub const SERUM_PROGRAM_ID: &str = "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin";

/// Raydium pool state - simplified version with just what we need
#[derive(Debug)]
pub struct RaydiumPoolState {
    pub amm_id: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_target_orders: Pubkey,
    pub pool_coin_token_account: Pubkey,
    pub pool_pc_token_account: Pubkey,
    pub pool_withdraw_queue: Pubkey,
    pub pool_temp_lp_token_account: Pubkey,
    pub serum_program_id: Pubkey,
    pub serum_market: Pubkey,
    pub coin_mint: Pubkey,
    pub pc_mint: Pubkey,
}

/// Raydium pool state layout (offsets from Raydium SDK)
/// Reference: https://github.com/raydium-io/raydium-sdk
///
/// The pool account data structure (simplified):
/// - Status: u64 (8 bytes) at offset 0
/// - Nonce: u64 (8 bytes) at offset 8
/// - Order num: u64 (8 bytes) at offset 16
/// - Depth: u64 (8 bytes) at offset 24
/// - Coin decimals: u64 (8 bytes) at offset 32
/// - PC decimals: u64 (8 bytes) at offset 40
/// - State: u64 (8 bytes) at offset 48
/// - Reset flag: u64 (8 bytes) at offset 56
/// - Min size: u64 (8 bytes) at offset 64
/// - Vol max cut ratio: u64 (8 bytes) at offset 72
/// - Amount wave: u64 (8 bytes) at offset 80
/// - Coin lot size: u64 (8 bytes) at offset 88
/// - PC lot size: u64 (8 bytes) at offset 96
/// - Min price multiplier: u64 (8 bytes) at offset 104
/// - Max price multiplier: u64 (8 bytes) at offset 112
/// - System decimals value: u64 (8 bytes) at offset 120
///
/// Then the account pubkeys start at offset 128:
/// - amm_target_orders: Pubkey (32 bytes) at offset 128
/// - pool_coin_token_account: Pubkey (32 bytes) at offset 160
/// - pool_pc_token_account: Pubkey (32 bytes) at offset 192
/// - coin_mint: Pubkey (32 bytes) at offset 224
/// - pc_mint: Pubkey (32 bytes) at offset 256
/// - lp_mint: Pubkey (32 bytes) at offset 288
/// - amm_open_orders: Pubkey (32 bytes) at offset 320
/// - serum_market: Pubkey (32 bytes) at offset 352
/// - serum_program_id: Pubkey (32 bytes) at offset 384
/// - amm_target_orders (duplicate): Pubkey (32 bytes) at offset 416
/// - pool_withdraw_queue: Pubkey (32 bytes) at offset 448
/// - pool_temp_lp_token_account: Pubkey (32 bytes) at offset 480
/// - amm_owner: Pubkey (32 bytes) at offset 512
/// - pnl_owner: Pubkey (32 bytes) at offset 544
impl RaydiumPoolState {
    /// Parse pool state from account data
    pub fn parse(pool_pubkey: &Pubkey, data: &[u8]) -> Result<Self> {
        // Minimum size check (need at least 576 bytes for all fields)
        if data.len() < 576 {
            return Err(anyhow!(
                "Pool account data too small: {} bytes (expected at least 576)",
                data.len()
            ));
        }

        // Parse pubkeys at their offsets
        let amm_target_orders = Self::parse_pubkey(data, 128)?;
        let pool_coin_token_account = Self::parse_pubkey(data, 160)?;
        let pool_pc_token_account = Self::parse_pubkey(data, 192)?;
        let coin_mint = Self::parse_pubkey(data, 224)?;
        let pc_mint = Self::parse_pubkey(data, 256)?;
        let amm_open_orders = Self::parse_pubkey(data, 320)?;
        let serum_market = Self::parse_pubkey(data, 352)?;
        let serum_program_id = Self::parse_pubkey(data, 384)?;
        let pool_withdraw_queue = Self::parse_pubkey(data, 448)?;
        let pool_temp_lp_token_account = Self::parse_pubkey(data, 480)?;

        // Derive AMM authority (PDA derived from pool address)
        let amm_authority = Self::derive_amm_authority(pool_pubkey)?;

        Ok(RaydiumPoolState {
            amm_id: *pool_pubkey,
            amm_authority,
            amm_open_orders,
            amm_target_orders,
            pool_coin_token_account,
            pool_pc_token_account,
            pool_withdraw_queue,
            pool_temp_lp_token_account,
            serum_program_id,
            serum_market,
            coin_mint,
            pc_mint,
        })
    }

    /// Derive AMM authority PDA from pool address
    ///
    /// Raydium uses a Program Derived Address (PDA) for the pool authority.
    /// The seeds are: [b"amm authority", pool_address.as_ref()]
    fn derive_amm_authority(pool_address: &Pubkey) -> Result<Pubkey> {
        let program_id = Pubkey::from_str(RAYDIUM_AMM_V4_PROGRAM_ID)
            .map_err(|e| anyhow!("Invalid Raydium program ID: {}", e))?;

        let (authority, _bump) =
            Pubkey::find_program_address(&[b"amm authority", pool_address.as_ref()], &program_id);

        Ok(authority)
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

    /// Fetch pool state from RPC
    pub fn fetch(rpc_client: &RpcClient, pool_address: &Pubkey) -> Result<Self> {
        // Get pool account data
        let account = rpc_client
            .get_account(pool_address)
            .map_err(|e| anyhow!("Failed to fetch pool account: {}", e))?;

        // Verify owner is Raydium program
        let raydium_program = Pubkey::from_str(RAYDIUM_AMM_V4_PROGRAM_ID)?;
        if account.owner != raydium_program {
            return Err(anyhow!(
                "Account is not owned by Raydium program. Owner: {}, Expected: {}",
                account.owner,
                raydium_program
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
    fn test_derive_amm_authority() {
        let pool = Pubkey::new_unique();
        let result = RaydiumPoolState::derive_amm_authority(&pool);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_pubkey() {
        let mut data = vec![0u8; 64];
        let test_pubkey = Pubkey::new_unique();
        data[0..32].copy_from_slice(test_pubkey.as_ref());

        let result = RaydiumPoolState::parse_pubkey(&data, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_pubkey);
    }

    #[test]
    fn test_parse_pool_state_too_small() {
        let pool = Pubkey::new_unique();
        let data = vec![0u8; 100]; // Too small

        let result = RaydiumPoolState::parse(&pool, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too small"));
    }
}
