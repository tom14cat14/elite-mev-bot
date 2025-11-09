//! Pool Address Validator
//!
//! Validates pool addresses by checking owner and size via RPC.
//! This prevents extracting user wallets or token mints.

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    commitment_config::CommitmentConfig,
};
use std::str::FromStr;
use tracing::{debug, warn};

/// Known DEX program IDs for validation
pub struct DexProgramIds;

impl DexProgramIds {
    pub fn raydium_clmm() -> Pubkey {
        Pubkey::from_str("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK").unwrap()
    }

    pub fn orca_whirlpools() -> Pubkey {
        Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap()
    }

    pub fn meteora_dlmm() -> Pubkey {
        Pubkey::from_str("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo").unwrap()
    }

    pub fn raydium_cpmm() -> Pubkey {
        Pubkey::from_str("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C").unwrap()
    }

    pub fn raydium_amm_v4() -> Pubkey {
        Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap()
    }

    /// Get expected owner for a DEX type
    pub fn get_expected_owner(dex_name: &str) -> Option<Pubkey> {
        match dex_name {
            "Raydium_CLMM" => Some(Self::raydium_clmm()),
            "Orca_Whirlpools" => Some(Self::orca_whirlpools()),
            "Meteora_DLMM" => Some(Self::meteora_dlmm()),
            "Raydium_CPMM" => Some(Self::raydium_cpmm()),
            "Raydium_AMM_V4" => Some(Self::raydium_amm_v4()),
            _ => None,
        }
    }
}

/// Validates a pool address by checking:
/// 1. Account exists
/// 2. Owner matches expected DEX program
/// 3. Account size is large enough to be a pool (>200 bytes)
pub fn validate_pool_address(
    rpc: &RpcClient,
    candidate: &Pubkey,
    dex_name: &str,
) -> bool {
    // Get expected owner for this DEX
    let expected_owner = match DexProgramIds::get_expected_owner(dex_name) {
        Some(owner) => owner,
        None => {
            warn!("❌ Unknown DEX type: {}", dex_name);
            return false;
        }
    };

    // Fetch account data
    match rpc.get_account_with_commitment(candidate, CommitmentConfig::confirmed()) {
        Ok(response) => {
            if let Some(account) = response.value {
                let is_correct_owner = account.owner == expected_owner;
                let is_large_enough = account.data.len() > 200;

                if is_correct_owner && is_large_enough {
                    debug!(
                        "✅ Pool validated: {} | Owner: {} | Size: {} bytes | DEX: {}",
                        candidate, account.owner, account.data.len(), dex_name
                    );
                    true
                } else {
                    debug!(
                        "❌ Pool rejected: {} | Owner: {} (expected: {}) | Size: {} | DEX: {}",
                        candidate, account.owner, expected_owner, account.data.len(), dex_name
                    );
                    false
                }
            } else {
                debug!("❌ Account not found: {}", candidate);
                false
            }
        }
        Err(e) => {
            warn!("⚠️  RPC error validating {}: {}", candidate, e);
            false
        }
    }
}

/// Scans multiple account indices to find the pool address
/// Returns the first valid pool found
pub fn scan_for_pool(
    rpc: &RpcClient,
    accounts: &[Pubkey],
    instruction_accounts: &[u8],
    dex_name: &str,
) -> Option<Pubkey> {
    // Try accounts in order of likelihood (based on empirical data)
    // Most pools are in first 5 writable accounts
    let indices_to_try = [0, 1, 2, 3, 4, 5];

    for &idx in &indices_to_try {
        if let Some(&account_idx) = instruction_accounts.get(idx) {
            if let Some(candidate) = accounts.get(account_idx as usize) {
                debug!(
                    "🔍 Trying account index {} (account_idx={}) for {} pool: {}",
                    idx, account_idx, dex_name, candidate
                );

                if validate_pool_address(rpc, candidate, dex_name) {
                    debug!("✅ Found valid pool at index {}: {}", idx, candidate);
                    return Some(*candidate);
                }
            }
        }
    }

    debug!("❌ No valid pool found after scanning {} indices", indices_to_try.len());
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dex_program_ids() {
        let raydium_clmm = DexProgramIds::raydium_clmm();
        assert_ne!(raydium_clmm, Pubkey::default());

        let expected = DexProgramIds::get_expected_owner("Raydium_CLMM");
        assert!(expected.is_some());
        assert_eq!(expected.unwrap(), raydium_clmm);
    }
}
