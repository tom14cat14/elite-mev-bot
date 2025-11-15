//! Raydium CPMM (Constant Product Market Maker) Pool State
//!
//! Fetches CPMM pool state for sandwich execution.
//! CPMM is a simpler constant product AMM (x * y = k) compared to CLMM.

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Raydium CPMM program ID
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

/// Raydium CPMM pool state - essential fields for swap execution
#[derive(Debug, Clone)]
pub struct RaydiumCpmmPoolState {
    pub pool_id: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_1_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub authority: Pubkey,
}

impl RaydiumCpmmPoolState {
    /// Parse CPMM pool state from account data
    ///
    /// Raydium CPMM uses a simpler structure than CLMM.
    /// The pool account contains vault addresses and other essentials.
    ///
    /// Basic layout (estimated offsets):
    /// - bump: [u8; 1] at offset 0
    /// - authority: Pubkey (32 bytes) at offset 1
    /// - token_0_mint: Pubkey (32 bytes) at offset 33
    /// - token_1_mint: Pubkey (32 bytes) at offset 65
    /// - token_0_vault: Pubkey (32 bytes) at offset 97
    /// - token_1_vault: Pubkey (32 bytes) at offset 129
    /// - lp_mint: Pubkey (32 bytes) at offset 161
    /// ... (additional fields)
    pub fn parse(pool_pubkey: &Pubkey, data: &[u8]) -> Result<Self> {
        // Minimum size check
        if data.len() < 193 {
            return Err(anyhow!(
                "CPMM pool account data too small: {} bytes (expected at least 193)",
                data.len()
            ));
        }

        // Parse essential pubkeys
        let authority = Self::parse_pubkey(data, 1)?;
        let token_0_mint = Self::parse_pubkey(data, 33)?;
        let token_1_mint = Self::parse_pubkey(data, 65)?;
        let token_0_vault = Self::parse_pubkey(data, 97)?;
        let token_1_vault = Self::parse_pubkey(data, 129)?;
        let lp_mint = Self::parse_pubkey(data, 161)?;

        Ok(RaydiumCpmmPoolState {
            pool_id: *pool_pubkey,
            token_0_mint,
            token_1_mint,
            token_0_vault,
            token_1_vault,
            lp_mint,
            authority,
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

    /// Fetch CPMM pool state from RPC
    pub fn fetch(rpc_client: &RpcClient, pool_address: &Pubkey) -> Result<Self> {
        // Get pool account data
        let account = rpc_client
            .get_account(pool_address)
            .map_err(|e| anyhow!("Failed to fetch CPMM pool account: {}", e))?;

        // Verify owner is Raydium CPMM program
        let cpmm_program = Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID)?;
        if account.owner != cpmm_program {
            return Err(anyhow!(
                "Account is not owned by Raydium CPMM program. Owner: {}, Expected: {}",
                account.owner,
                cpmm_program
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

        let result = RaydiumCpmmPoolState::parse_pubkey(&data, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_pubkey);
    }

    #[test]
    fn test_parse_pool_state_too_small() {
        let pool = Pubkey::new_unique();
        let data = vec![0u8; 100]; // Too small

        let result = RaydiumCpmmPoolState::parse(&pool, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too small"));
    }
}
