//! Unified DEX Pool State Fetcher
//!
//! Routes pool state fetching to the correct DEX-specific parser based on program owner.
//! This solves the "Account not owned by Raydium V4" bug by checking actual program ownership.

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::str::FromStr;
use tracing::{info, warn};

// Import DEX-specific state modules
use crate::raydium_pool_state::RaydiumPoolState;
use crate::raydium_clmm_state::RaydiumClmmPoolState;
use crate::raydium_cpmm_state::RaydiumCpmmPoolState;
use crate::orca_whirlpool_state::OrcaWhirlpoolState;
use crate::meteora_dlmm_state::MeteoraDlmmPoolState;
use crate::pumpswap_state::PumpSwapBondingCurveState;

/// Supported DEX types (matches mev_sandwich_detector.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DexType {
    RaydiumAmmV4,
    RaydiumClmm,
    RaydiumCpmm,
    OrcaWhirlpools,
    MeteoraDlmm,
    PumpSwap,
    JupiterV6,  // Special handling needed
}

impl DexType {
    /// Get the program ID for this DEX type
    pub fn program_id(&self) -> Pubkey {
        match self {
            DexType::RaydiumAmmV4 => {
                Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap()
            }
            DexType::RaydiumClmm => {
                Pubkey::from_str("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK").unwrap()
            }
            DexType::RaydiumCpmm => {
                Pubkey::from_str("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C").unwrap()
            }
            DexType::OrcaWhirlpools => {
                Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap()
            }
            DexType::MeteoraDlmm => {
                Pubkey::from_str("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo").unwrap()
            }
            DexType::PumpSwap => {
                Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap()
            }
            DexType::JupiterV6 => {
                Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap()
            }
        }
    }

    /// Identify DEX type from program owner pubkey
    pub fn from_owner(owner: &Pubkey) -> Option<Self> {
        let owner_str = owner.to_string();
        match owner_str.as_str() {
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => Some(DexType::RaydiumAmmV4),
            "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => Some(DexType::RaydiumClmm),
            "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => Some(DexType::RaydiumCpmm),
            "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => Some(DexType::OrcaWhirlpools),
            "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => Some(DexType::MeteoraDlmm),
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => Some(DexType::PumpSwap),
            "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => Some(DexType::JupiterV6),
            _ => None,
        }
    }

    /// Identify DEX type from DEX name string (from sandwich detector)
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Raydium_AMM_V4" => Some(DexType::RaydiumAmmV4),
            "Raydium_CLMM" => Some(DexType::RaydiumClmm),
            "Raydium_CPMM" => Some(DexType::RaydiumCpmm),
            "Orca_Whirlpools" => Some(DexType::OrcaWhirlpools),
            "Meteora_DLMM" => Some(DexType::MeteoraDlmm),
            "PumpSwap" => Some(DexType::PumpSwap),
            "Jupiter_V6" => Some(DexType::JupiterV6),
            _ => None,
        }
    }
}

/// Unified pool state enum wrapping all DEX-specific states
#[derive(Debug)]
pub enum DexPoolState {
    RaydiumAmmV4(RaydiumPoolState),
    RaydiumClmm(RaydiumClmmPoolState),
    RaydiumCpmm(RaydiumCpmmPoolState),
    OrcaWhirlpools(OrcaWhirlpoolState),
    MeteoraDlmm(MeteoraDlmmPoolState),
    PumpSwap(PumpSwapBondingCurveState),
}

impl DexPoolState {
    /// Get the DEX type of this pool state
    pub fn dex_type(&self) -> DexType {
        match self {
            DexPoolState::RaydiumAmmV4(_) => DexType::RaydiumAmmV4,
            DexPoolState::RaydiumClmm(_) => DexType::RaydiumClmm,
            DexPoolState::RaydiumCpmm(_) => DexType::RaydiumCpmm,
            DexPoolState::OrcaWhirlpools(_) => DexType::OrcaWhirlpools,
            DexPoolState::MeteoraDlmm(_) => DexType::MeteoraDlmm,
            DexPoolState::PumpSwap(_) => DexType::PumpSwap,
        }
    }
}

/// Fetch pool state with automatic DEX-type routing
///
/// This function:
/// 1. Fetches the pool account from RPC
/// 2. Checks the account owner to determine DEX type
/// 3. Routes to the correct DEX-specific parser
///
/// This solves Bug #2: "Account not owned by Raydium V4"
pub async fn fetch_pool_state(rpc: &RpcClient, pool_addr: &Pubkey) -> Result<DexPoolState> {
    // Fetch account with commitment
    let account = rpc
        .get_account_with_commitment(pool_addr, CommitmentConfig::confirmed())
        .map_err(|e| anyhow!("Failed to fetch pool account: {}", e))?
        .value
        .ok_or_else(|| anyhow!("Pool account not found: {}", pool_addr))?;

    info!("📦 Fetched pool account | Address: {} | Owner: {} | Size: {} bytes",
          pool_addr, account.owner, account.data.len());

    // CRITICAL FIX: Validate pool account before parsing
    // Reject token accounts, wallets, etc. (owner must be DEX program + size >200 bytes)
    if account.data.len() < 200 {
        warn!("❌ Account too small to be a pool: {} bytes (min 200)", account.data.len());
        return Err(anyhow!("Account size {} bytes is too small for a pool (min 200 bytes required)", account.data.len()));
    }

    // Determine DEX type from account owner
    let dex_type = DexType::from_owner(&account.owner)
        .ok_or_else(|| anyhow!("Unknown DEX program owner: {} (likely a token account or wallet)", account.owner))?;

    info!("🔍 Identified DEX type: {:?}", dex_type);

    // Route to correct parser based on DEX type
    match dex_type {
        DexType::RaydiumAmmV4 => {
            let state = RaydiumPoolState::parse(pool_addr, &account.data)
                .map_err(|e| anyhow!("Failed to parse Raydium AMM V4 state: {}", e))?;
            Ok(DexPoolState::RaydiumAmmV4(state))
        }
        DexType::RaydiumClmm => {
            let state = RaydiumClmmPoolState::parse(pool_addr, &account.data)
                .map_err(|e| anyhow!("Failed to parse Raydium CLMM state: {}", e))?;
            Ok(DexPoolState::RaydiumClmm(state))
        }
        DexType::RaydiumCpmm => {
            let state = RaydiumCpmmPoolState::parse(pool_addr, &account.data)
                .map_err(|e| anyhow!("Failed to parse Raydium CPMM state: {}", e))?;
            Ok(DexPoolState::RaydiumCpmm(state))
        }
        DexType::OrcaWhirlpools => {
            let state = OrcaWhirlpoolState::parse(pool_addr, &account.data)
                .map_err(|e| anyhow!("Failed to parse Orca Whirlpool state: {}", e))?;
            Ok(DexPoolState::OrcaWhirlpools(state))
        }
        DexType::MeteoraDlmm => {
            let state = MeteoraDlmmPoolState::parse(pool_addr, &account.data)
                .map_err(|e| anyhow!("Failed to parse Meteora DLMM state: {}", e))?;
            Ok(DexPoolState::MeteoraDlmm(state))
        }
        DexType::PumpSwap => {
            let state = PumpSwapBondingCurveState::parse(pool_addr, &account.data)
                .map_err(|e| anyhow!("Failed to parse PumpSwap state: {}", e))?;
            Ok(DexPoolState::PumpSwap(state))
        }
        DexType::JupiterV6 => {
            // Jupiter is an aggregator, not a direct DEX
            // Pool addresses from Jupiter swaps should be the underlying DEX pool
            warn!("⚠️  Jupiter V6 detected - this should not happen");
            warn!("   Jupiter is an aggregator - pool should be from underlying DEX");
            Err(anyhow!("Jupiter V6: Use underlying DEX pool, not Jupiter program"))
        }
    }
}

/// Alternative: Fetch pool state when you already know the DEX type
/// (from sandwich opportunity detection)
pub async fn fetch_pool_state_by_dex(
    rpc: &RpcClient,
    pool_addr: &Pubkey,
    dex_type: DexType,
) -> Result<DexPoolState> {
    // Fetch account
    let account = rpc
        .get_account_with_commitment(pool_addr, CommitmentConfig::confirmed())
        .map_err(|e| anyhow!("Failed to fetch pool account: {}", e))?
        .value
        .ok_or_else(|| anyhow!("Pool account not found: {}", pool_addr))?;

    // Verify owner matches expected DEX type
    let expected_owner = dex_type.program_id();
    if account.owner != expected_owner {
        warn!("⚠️  Pool owner mismatch!");
        warn!("   Expected: {} ({:?})", expected_owner, dex_type);
        warn!("   Actual:   {}", account.owner);

        // Try to identify actual DEX
        if let Some(actual_dex) = DexType::from_owner(&account.owner) {
            warn!("   Pool is actually owned by: {:?}", actual_dex);
            warn!("   Falling back to automatic detection...");
            return fetch_pool_state(rpc, pool_addr).await;
        }

        return Err(anyhow!(
            "Pool owner mismatch: expected {:?} ({}), got {}",
            dex_type, expected_owner, account.owner
        ));
    }

    // Route to correct parser
    match dex_type {
        DexType::RaydiumAmmV4 => {
            let state = RaydiumPoolState::parse(pool_addr, &account.data)?;
            Ok(DexPoolState::RaydiumAmmV4(state))
        }
        DexType::RaydiumClmm => {
            let state = RaydiumClmmPoolState::parse(pool_addr, &account.data)?;
            Ok(DexPoolState::RaydiumClmm(state))
        }
        DexType::RaydiumCpmm => {
            let state = RaydiumCpmmPoolState::parse(pool_addr, &account.data)?;
            Ok(DexPoolState::RaydiumCpmm(state))
        }
        DexType::OrcaWhirlpools => {
            let state = OrcaWhirlpoolState::parse(pool_addr, &account.data)?;
            Ok(DexPoolState::OrcaWhirlpools(state))
        }
        DexType::MeteoraDlmm => {
            let state = MeteoraDlmmPoolState::parse(pool_addr, &account.data)?;
            Ok(DexPoolState::MeteoraDlmm(state))
        }
        DexType::PumpSwap => {
            let state = PumpSwapBondingCurveState::parse(pool_addr, &account.data)?;
            Ok(DexPoolState::PumpSwap(state))
        }
        DexType::JupiterV6 => {
            Err(anyhow!("Jupiter V6: Use underlying DEX pool"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dex_type_from_owner() {
        let raydium_v4 = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap();
        assert_eq!(DexType::from_owner(&raydium_v4), Some(DexType::RaydiumAmmV4));

        let orca = Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap();
        assert_eq!(DexType::from_owner(&orca), Some(DexType::OrcaWhirlpools));
    }

    #[test]
    fn test_dex_type_from_name() {
        assert_eq!(DexType::from_name("Raydium_AMM_V4"), Some(DexType::RaydiumAmmV4));
        assert_eq!(DexType::from_name("Orca_Whirlpools"), Some(DexType::OrcaWhirlpools));
        assert_eq!(DexType::from_name("Unknown_DEX"), None);
    }
}
