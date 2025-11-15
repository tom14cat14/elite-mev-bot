use crate::token_decimal_cache::{calculate_adjusted_price, TokenDecimalCache};
use chrono::{DateTime, Utc};
use solana_sdk::{message::VersionedMessage, pubkey::Pubkey, transaction::VersionedTransaction};
use std::str::FromStr;
use tracing::{debug, warn};

/// Safe string truncation to prevent panics on short strings
fn truncate_safe(s: &str, max_len: usize) -> String {
    s.chars().take(max_len).collect()
}

/// Information extracted from a DEX swap transaction
#[derive(Debug, Clone)]
pub struct SwapInfo {
    pub signature: String,
    pub slot: u64,
    pub dex_name: String,
    pub dex_program_id: String,
    pub token_mint: String,
    pub pool_address: String,
    pub amount_in: u64,
    pub amount_out: u64,
    pub price_sol: f64, // Decimal-adjusted price
    pub is_buy: bool,
    pub timestamp: DateTime<Utc>,
    pub user_wallet: String,
    pub decimals_in: u8,  // Token decimals for amount_in
    pub decimals_out: u8, // Token decimals for amount_out
}

/// DEX program information
#[derive(Clone)]
struct DexProgramInfo {
    name: String,
    program_id: Pubkey,
    swap_discriminator: Vec<u8>,
}

/// Parser for extracting DEX swap information from Solana transactions
pub struct DexSwapParser {
    dex_programs: Vec<DexProgramInfo>,
    decimal_cache: TokenDecimalCache,
}

impl DexSwapParser {
    pub fn new(decimal_cache: TokenDecimalCache) -> Result<Self, String> {
        // Helper macro to parse pubkey with better error messages
        macro_rules! parse_pubkey {
            ($addr:expr) => {
                Pubkey::from_str($addr).map_err(|e| format!("Invalid pubkey '{}': {}", $addr, e))?
            };
        }

        let dex_programs = vec![
            // Raydium - Multiple Pool Types
            DexProgramInfo {
                name: "Raydium_AMM_V4".to_string(),
                program_id: parse_pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"),
                swap_discriminator: vec![143, 190, 90, 218, 196, 30, 51, 222],
            },
            DexProgramInfo {
                name: "Raydium_CLMM".to_string(),
                program_id: parse_pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"),
                swap_discriminator: vec![143, 190, 90, 218, 196, 30, 51, 222],
            },
            DexProgramInfo {
                name: "Raydium_CPMM".to_string(),
                program_id: parse_pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"),
                swap_discriminator: vec![143, 190, 90, 218, 196, 30, 51, 222],
            },
            DexProgramInfo {
                name: "Raydium_Stable".to_string(),
                program_id: parse_pubkey!("5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h"),
                swap_discriminator: vec![143, 190, 90, 218, 196, 30, 51, 222],
            },
            // Orca
            DexProgramInfo {
                name: "Orca_Legacy".to_string(),
                program_id: parse_pubkey!("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP"),
                swap_discriminator: vec![248, 198, 158, 145, 225, 117, 135, 200],
            },
            DexProgramInfo {
                name: "Orca_Whirlpools".to_string(),
                program_id: parse_pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"),
                swap_discriminator: vec![248, 198, 158, 145, 225, 117, 135, 200],
            },
            // Jupiter
            DexProgramInfo {
                name: "Jupiter".to_string(),
                program_id: parse_pubkey!("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"),
                swap_discriminator: vec![229, 23, 203, 151, 122, 227, 173, 42],
            },
            // Meteora
            DexProgramInfo {
                name: "Meteora_DAMM_V1".to_string(),
                program_id: parse_pubkey!("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB"),
                swap_discriminator: vec![248, 198, 158, 145, 225, 117, 135, 200],
            },
            DexProgramInfo {
                name: "Meteora_DLMM".to_string(),
                program_id: parse_pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo"),
                swap_discriminator: vec![248, 198, 158, 145, 225, 117, 135, 200],
            },
            DexProgramInfo {
                name: "Meteora_DAMM_V2".to_string(),
                program_id: parse_pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG"),
                swap_discriminator: vec![248, 198, 158, 145, 225, 117, 135, 200],
            },
            // PumpSwap
            DexProgramInfo {
                name: "PumpSwap".to_string(),
                program_id: parse_pubkey!("GMk6j2defJhS7F194toqmJNFNhAkbDXhYJo5oR3Rpump"),
                swap_discriminator: vec![102, 6, 61, 18, 1, 218, 235, 234],
            },
        ];

        Ok(Self {
            dex_programs,
            decimal_cache,
        })
    }

    /// Parse a transaction and extract swap information
    /// Now async to support decimal fetching from chain
    pub async fn parse_transaction(
        &self,
        tx: &VersionedTransaction,
        signature: String,
        slot: u64,
    ) -> Option<SwapInfo> {
        let (instructions, account_keys) = match &tx.message {
            VersionedMessage::Legacy(msg) => (&msg.instructions, &msg.account_keys),
            VersionedMessage::V0(msg) => (&msg.instructions, &msg.account_keys),
        };

        for instruction in instructions.iter() {
            let program_id_index = instruction.program_id_index as usize;
            if program_id_index >= account_keys.len() {
                continue;
            }
            let program_id = account_keys[program_id_index];

            for dex_info in &self.dex_programs {
                if program_id == dex_info.program_id
                    && instruction.data.len() >= 8 {
                        let discriminator = &instruction.data[0..8];
                        if discriminator == dex_info.swap_discriminator.as_slice() {
                            return self
                                .parse_swap_instruction(
                                    account_keys,
                                    instruction,
                                    &dex_info.name,
                                    &signature,
                                    slot,
                                )
                                .await;
                        }
                    }
            }
        }

        None
    }

    async fn parse_swap_instruction(
        &self,
        account_keys: &[Pubkey],
        instruction: &solana_sdk::instruction::CompiledInstruction,
        dex_name: &str,
        signature: &str,
        slot: u64,
    ) -> Option<SwapInfo> {
        if instruction.data.len() < 24 {
            return None;
        }

        let amount_in = u64::from_le_bytes(instruction.data[8..16].try_into().ok()?);
        let amount_out = u64::from_le_bytes(instruction.data[16..24].try_into().ok()?);

        let token_mint = if instruction.accounts.len() > 2 {
            let mint_index = instruction.accounts[2] as usize;
            if mint_index < account_keys.len() {
                account_keys[mint_index].to_string()
            } else {
                return None;
            }
        } else {
            return None;
        };

        let user_wallet = if !instruction.accounts.is_empty() {
            let user_index = instruction.accounts[0] as usize;
            if user_index < account_keys.len() {
                account_keys[user_index].to_string()
            } else {
                "unknown".to_string()
            }
        } else {
            "unknown".to_string()
        };

        // 🔍 DIAGNOSTIC: Log ALL accounts to understand structure
        debug!(
            "🔍 DEX Parser DIAGNOSTIC | Total accounts: {}",
            instruction.accounts.len()
        );
        for (i, &account_idx) in instruction.accounts.iter().enumerate().take(10) {
            if let Some(account_key) = account_keys.get(account_idx as usize) {
                debug!("   Account[{}] = {} (idx={})", i, account_key, account_idx);
            }
        }

        let pool_address = if instruction.accounts.len() > 1 {
            let pool_index = instruction.accounts[1] as usize;
            if pool_index < account_keys.len() {
                let extracted = account_keys[pool_index].to_string();
                debug!("🔍 DEX Parser extracted pool from index 1: {}", extracted);
                extracted
            } else {
                "unknown".to_string()
            }
        } else {
            "unknown".to_string()
        };

        const MIN_SWAP_SIZE_LAMPORTS: u64 = 1_000_000; // 0.001 SOL
        if amount_in < MIN_SWAP_SIZE_LAMPORTS && amount_out < MIN_SWAP_SIZE_LAMPORTS {
            return None;
        }

        // CRITICAL FIX: Fetch token decimals for accurate price calculations
        // Without this, prices are wrong by 100x-1000x!
        let decimals_in = self
            .decimal_cache
            .get_decimals(&token_mint)
            .await
            .unwrap_or_else(|e| {
                warn!(
                    "⚠️ Failed to fetch decimals for {}: {}, defaulting to 9",
                    truncate_safe(&token_mint, 8),
                    e
                );
                9 // Default to SOL decimals if fetch fails
            });

        // SOL has 9 decimals - this is our base currency
        let decimals_out = 9;

        // Calculate decimal-adjusted price (the CRITICAL fix!)
        let price_sol = if amount_in > 0 && amount_out > 0 {
            let adjusted_price =
                calculate_adjusted_price(amount_in, amount_out, decimals_in, decimals_out);

            const MIN_REALISTIC_PRICE: f64 = 0.0000001;
            const MAX_REALISTIC_PRICE: f64 = 10_000.0;

            if !(MIN_REALISTIC_PRICE..=MAX_REALISTIC_PRICE).contains(&adjusted_price) {
                return None;
            }

            adjusted_price
        } else {
            0.0
        };

        let is_buy = amount_in < amount_out;

        Some(SwapInfo {
            signature: signature.to_string(),
            slot,
            dex_name: dex_name.to_string(),
            dex_program_id: instruction.program_id_index.to_string(),
            token_mint,
            pool_address,
            amount_in,
            amount_out,
            price_sol, // Now decimal-adjusted!
            is_buy,
            timestamp: Utc::now(),
            user_wallet,
            decimals_in,
            decimals_out,
        })
    }
}
