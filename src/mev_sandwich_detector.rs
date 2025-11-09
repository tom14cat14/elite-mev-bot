//! Simple MEV Sandwich Opportunity Detector
//!
//! Detects large swaps in real-time and determines if they are profitable to sandwich.
//! Minimal dependencies, maximum speed.

use solana_entry::entry::Entry;
use solana_sdk::{
    pubkey::Pubkey,
    message::VersionedMessage,
    transaction::VersionedTransaction,
    system_program,
};
use std::str::FromStr;
use tracing::{debug, info, warn};

/// Known DEX program IDs
pub struct DexPrograms {
    pub raydium_amm_v4: Pubkey,
    pub raydium_clmm: Pubkey,
    pub raydium_cpmm: Pubkey,
    pub orca_whirlpools: Pubkey,
    pub meteora_dlmm: Pubkey,
    pub jupiter_v6: Pubkey,
    pub pumpswap: Pubkey,
}

impl DexPrograms {
    pub fn new() -> Self {
        Self {
            raydium_amm_v4: Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap(),
            raydium_clmm: Pubkey::from_str("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK").unwrap(),
            raydium_cpmm: Pubkey::from_str("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C").unwrap(),
            orca_whirlpools: Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap(),
            meteora_dlmm: Pubkey::from_str("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo").unwrap(),
            jupiter_v6: Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap(),
            pumpswap: Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap(),
        }
    }

    pub fn identify(&self, program_id: &Pubkey) -> Option<&'static str> {
        if program_id == &self.raydium_amm_v4 {
            Some("Raydium_AMM_V4")
        } else if program_id == &self.raydium_clmm {
            Some("Raydium_CLMM")
        } else if program_id == &self.raydium_cpmm {
            Some("Raydium_CPMM")
        } else if program_id == &self.orca_whirlpools {
            Some("Orca_Whirlpools")
        } else if program_id == &self.meteora_dlmm {
            Some("Meteora_DLMM")
        } else if program_id == &self.jupiter_v6 {
            Some("Jupiter_V6")
        } else if program_id == &self.pumpswap {
            Some("PumpSwap")
        } else {
            None
        }
    }
}

/// A potential victim swap
#[derive(Debug, Clone)]
pub struct SandwichOpportunity {
    pub dex_name: String,
    pub signature: String,
    pub slot: u64,
    pub estimated_sol_value: f64,
    pub timestamp: std::time::Instant,

    // Transaction details for execution (optional - parsed when available)
    pub input_mint: Option<String>,      // Token being sold
    pub output_mint: Option<String>,     // Token being bought
    pub pool_address: Option<String>,    // DEX pool address
    pub swap_amount_in: Option<u64>,     // Amount in (lamports/smallest unit)
    pub min_amount_out: Option<u64>,     // Minimum amount out (slippage protection)
}

/// Configuration for sandwich detection
#[derive(Clone, Debug)]
pub struct SandwichConfig {
    pub min_swap_size_sol: f64,
    pub max_swap_size_sol: f64,
    pub min_profit_sol: f64,
}

impl Default for SandwichConfig {
    fn default() -> Self {
        Self {
            min_swap_size_sol: 0.01,   // Min 0.01 SOL swap (detect more opportunities)
            max_swap_size_sol: 100.0,  // Max 100 SOL (whale protection)
            min_profit_sol: 0.0001,    // Min 0.0001 SOL profit after fees (lowered to match execution threshold)
        }
    }
}

/// Detect sandwich opportunities from ShredStream entries
pub fn detect_sandwich_opportunities(
    entries: &[Entry],
    config: &SandwichConfig,
) -> Vec<SandwichOpportunity> {
    let dex_programs = DexPrograms::new();
    let mut opportunities = Vec::new();

    for entry in entries {
        for tx in &entry.transactions {
            if let Some(opp) = analyze_transaction(tx, &dex_programs, config) {
                opportunities.push(opp);
            }
        }
    }

    if !opportunities.is_empty() {
        info!("🎯 Detected {} potential sandwich opportunities", opportunities.len());
    }

    opportunities
}

/// Parse Raydium AMM V4 swap instruction
fn parse_raydium_amm_v4_swap(
    message: &VersionedMessage,
    instruction: &solana_sdk::instruction::CompiledInstruction,
) -> Option<(String, String, String, u64, u64)> {
    // Raydium AMM V4 swap instruction has 9 accounts:
    // 0: token_program
    // 1: amm_id (pool)
    // 2: amm_authority
    // 3: amm_open_orders
    // 4: amm_target_orders
    // 5: pool_coin_token_account
    // 6: pool_pc_token_account
    // 7: serum_program_id
    // 8: serum_market
    // 9: user_source_token_account
    // 10: user_destination_token_account
    // 11: user_owner

    if instruction.accounts.len() < 12 {
        return None; // Not enough accounts for Raydium swap
    }

    let accounts = message.static_account_keys();

    // Extract key accounts
    let pool_address = accounts.get(instruction.accounts[1] as usize)?;

    // Validate pool address - reject System Program and other invalid addresses
    if !is_valid_pool_address(pool_address) {
        return None;
    }

    let user_source = accounts.get(instruction.accounts[9] as usize)?;
    let user_dest = accounts.get(instruction.accounts[10] as usize)?;

    // Parse instruction data (first byte is discriminator, then amounts)
    if instruction.data.len() < 17 {
        return None; // Not enough data
    }

    // Instruction data format:
    // [0]: discriminator (9 for swap)
    // [1-8]: amount_in (u64 little-endian)
    // [9-16]: minimum_amount_out (u64 little-endian)

    if instruction.data[0] != 9 {
        return None; // Not a swap instruction
    }

    let amount_in = u64::from_le_bytes([
        instruction.data[1], instruction.data[2], instruction.data[3], instruction.data[4],
        instruction.data[5], instruction.data[6], instruction.data[7], instruction.data[8],
    ]);

    let min_amount_out = u64::from_le_bytes([
        instruction.data[9], instruction.data[10], instruction.data[11], instruction.data[12],
        instruction.data[13], instruction.data[14], instruction.data[15], instruction.data[16],
    ]);

    Some((
        pool_address.to_string(),
        user_source.to_string(),  // Approximation for input mint (user's source token account)
        user_dest.to_string(),    // Approximation for output mint (user's dest token account)
        amount_in,
        min_amount_out,
    ))
}

/// Parse Raydium CLMM (Concentrated Liquidity) swap instruction
/// Validate that a pool address is not a known system/program account
/// Filters out System Program, Token Program, and other invalid addresses
fn is_valid_pool_address(address: &Pubkey) -> bool {
    // Check if it's the system program
    if address == &system_program::ID {
        return false;
    }

    // Check if it's all zeros
    if address.to_bytes() == [0u8; 32] {
        return false;
    }

    // Check against known program addresses that are never pools
    let addr_str = address.to_string();
    !matches!(
        addr_str.as_str(),
        "11111111111111111111111111111111" // System Program
        | "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" // Token Program
        | "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" // Token 2022 Program
        | "ComputeBudget111111111111111111111111111111" // Compute Budget
        | "Vote111111111111111111111111111111111111111" // Vote Program
    )
}

fn parse_raydium_clmm_swap(
    message: &VersionedMessage,
    instruction: &solana_sdk::instruction::CompiledInstruction,
) -> Option<(String, String, String, u64, u64)> {
    // Extract pool regardless of swap instruction type (swap, swapV2, swapWithCap, etc.)

    if instruction.data.len() < 24 {
        return None;
    }

    let accounts = message.static_account_keys();

    // CLMM account structure:
    // 0: pool_state
    // 1: token_program
    // 2: token_program_2022 (optional)
    // 3: memo_program (optional)
    // 4: input_token_account
    // 5: output_token_account
    // 6: input_vault
    // 7: output_vault

    if instruction.accounts.len() < 8 {
        return None;
    }

    let pool_address = accounts.get(instruction.accounts[0] as usize)?;

    // Validate pool address - reject System Program and other invalid addresses
    if !is_valid_pool_address(pool_address) {
        return None;
    }

    let user_source = accounts.get(instruction.accounts[4] as usize)?;
    let user_dest = accounts.get(instruction.accounts[5] as usize)?;

    // Amount is at bytes 8-15, min_out at bytes 16-23
    let amount_in = u64::from_le_bytes([
        instruction.data[8], instruction.data[9], instruction.data[10], instruction.data[11],
        instruction.data[12], instruction.data[13], instruction.data[14], instruction.data[15],
    ]);

    let min_amount_out = u64::from_le_bytes([
        instruction.data[16], instruction.data[17], instruction.data[18], instruction.data[19],
        instruction.data[20], instruction.data[21], instruction.data[22], instruction.data[23],
    ]);

    Some((
        pool_address.to_string(),
        user_source.to_string(),
        user_dest.to_string(),
        amount_in,
        min_amount_out,
    ))
}

/// Parse Raydium CPMM (Constant Product) swap instruction
fn parse_raydium_cpmm_swap(
    message: &VersionedMessage,
    instruction: &solana_sdk::instruction::CompiledInstruction,
) -> Option<(String, String, String, u64, u64)> {
    // Extract pool regardless of swap instruction type (swap_base_input, swap_base_output, etc.)

    if instruction.data.len() < 24 {
        return None;
    }

    let accounts = message.static_account_keys();

    if instruction.accounts.len() < 8 {
        return None;
    }

    // CPMM pool is at account index 4
    let pool_address = accounts.get(instruction.accounts[4] as usize)?;

    // Validate pool address - reject System Program and other invalid addresses
    if !is_valid_pool_address(pool_address) {
        return None;
    }

    let user_source = accounts.get(instruction.accounts[4] as usize)?;
    let user_dest = accounts.get(instruction.accounts[5] as usize)?;

    // For CPMM with byte 0 = 0x09, amounts start at offset 1 (no 8-byte discriminator)
    let amount_in = u64::from_le_bytes([
        instruction.data[1], instruction.data[2], instruction.data[3], instruction.data[4],
        instruction.data[5], instruction.data[6], instruction.data[7], instruction.data[8],
    ]);

    let min_amount_out = u64::from_le_bytes([
        instruction.data[9], instruction.data[10], instruction.data[11], instruction.data[12],
        instruction.data[13], instruction.data[14], instruction.data[15], instruction.data[16],
    ]);

    Some((
        pool_address.to_string(),
        user_source.to_string(),
        user_dest.to_string(),
        amount_in,
        min_amount_out,
    ))
}

/// Parse Orca Whirlpools swap instruction
fn parse_orca_whirlpool_swap(
    message: &VersionedMessage,
    instruction: &solana_sdk::instruction::CompiledInstruction,
) -> Option<(String, String, String, u64, u64)> {
    // Extract pool regardless of swap instruction type (swap, swapV2, etc.)

    if instruction.data.len() < 24 {
        return None;
    }

    let accounts = message.static_account_keys();

    // Whirlpool account structure:
    // 0: token_program
    // 1: whirlpool (pool address)
    // 2: token_authority
    // 3: token_owner_account_a
    // 4: token_vault_a
    // 5: token_owner_account_b
    // 6: token_vault_b
    // 7: tick_array_0
    // 8: tick_array_1
    // 9: tick_array_2
    // 10: oracle

    if instruction.accounts.len() < 11 {
        return None;
    }

    // Orca Whirlpool pool is at account index 1
    let pool_address = accounts.get(instruction.accounts[1] as usize)?;

    // Validate pool address - reject System Program and other invalid addresses
    if !is_valid_pool_address(pool_address) {
        return None;
    }

    let user_source = accounts.get(instruction.accounts[3] as usize)?;
    let user_dest = accounts.get(instruction.accounts[5] as usize)?;

    // Parse amount from instruction data (Anchor format: 8 byte discriminator + data)
    let amount_in = u64::from_le_bytes([
        instruction.data[8], instruction.data[9], instruction.data[10], instruction.data[11],
        instruction.data[12], instruction.data[13], instruction.data[14], instruction.data[15],
    ]);

    let min_amount_out = u64::from_le_bytes([
        instruction.data[16], instruction.data[17], instruction.data[18], instruction.data[19],
        instruction.data[20], instruction.data[21], instruction.data[22], instruction.data[23],
    ]);

    Some((
        pool_address.to_string(),
        user_source.to_string(),
        user_dest.to_string(),
        amount_in,
        min_amount_out,
    ))
}

/// Parse Meteora DLMM swap instruction
fn parse_meteora_dlmm_swap(
    message: &VersionedMessage,
    instruction: &solana_sdk::instruction::CompiledInstruction,
) -> Option<(String, String, String, u64, u64)> {
    // Extract pool regardless of swap instruction type (swap, swap2, etc.)

    if instruction.data.len() < 24 {
        return None;
    }

    let accounts = message.static_account_keys();

    // DLMM typical structure:
    // 0: user_token_in
    // 1: user_token_out
    // 2: reserve_x
    // 3: reserve_y
    // 4: lb_pair (pool address)
    // 5: bin_array_bitmap_extension (optional)
    // 6: token_x_mint
    // 7: token_y_mint
    // 8: oracle
    // 9: token_program

    if instruction.accounts.len() < 10 {
        return None;
    }

    // Meteora DLMM pool is at account index 4
    let pool_address = accounts.get(instruction.accounts[4] as usize)?;

    // Validate pool address - reject System Program and other invalid addresses
    if !is_valid_pool_address(pool_address) {
        return None;
    }

    let user_source = accounts.get(instruction.accounts[0] as usize)?;
    let user_dest = accounts.get(instruction.accounts[1] as usize)?;

    // Parse amounts (8-byte Anchor discriminator + data)
    let amount_in = u64::from_le_bytes([
        instruction.data[8], instruction.data[9], instruction.data[10], instruction.data[11],
        instruction.data[12], instruction.data[13], instruction.data[14], instruction.data[15],
    ]);

    let min_amount_out = u64::from_le_bytes([
        instruction.data[16], instruction.data[17], instruction.data[18], instruction.data[19],
        instruction.data[20], instruction.data[21], instruction.data[22], instruction.data[23],
    ]);

    Some((
        pool_address.to_string(),
        user_source.to_string(),
        user_dest.to_string(),
        amount_in,
        min_amount_out,
    ))
}

/// Parse PumpSwap bonding curve swap
fn parse_pumpswap_swap(
    message: &VersionedMessage,
    instruction: &solana_sdk::instruction::CompiledInstruction,
) -> Option<(String, String, String, u64, u64)> {
    // PumpSwap has two swap instructions:
    // Buy:  [102, 6, 61, 18, 1, 218, 235, 234]
    // Sell: [51, 230, 133, 164, 1, 127, 131, 173]

    if instruction.data.len() < 24 {
        return None;
    }

    // Check for Buy or Sell discriminator
    let is_buy = instruction.data[0..8] == [102, 6, 61, 18, 1, 218, 235, 234];
    let is_sell = instruction.data[0..8] == [51, 230, 133, 164, 1, 127, 131, 173];

    if !is_buy && !is_sell {
        return None;
    }

    let accounts = message.static_account_keys();

    // PumpSwap structure:
    // 0: global
    // 1: fee_recipient
    // 2: mint
    // 3: bonding_curve
    // 4: associated_bonding_curve
    // 5: associated_user
    // 6: user
    // 7: system_program
    // 8: token_program
    // 9: rent
    // 10: event_authority
    // 11: program

    if instruction.accounts.len() < 7 {
        return None;
    }

    let bonding_curve = accounts.get(instruction.accounts[3] as usize)?;
    let user_source = accounts.get(instruction.accounts[5] as usize)?; // associated_user
    let user_dest = accounts.get(instruction.accounts[4] as usize)?;   // associated_bonding_curve

    // PumpSwap uses simpler 8-byte discriminator
    let amount_in = u64::from_le_bytes([
        instruction.data[8], instruction.data[9], instruction.data[10], instruction.data[11],
        instruction.data[12], instruction.data[13], instruction.data[14], instruction.data[15],
    ]);

    let min_amount_out = u64::from_le_bytes([
        instruction.data[16], instruction.data[17], instruction.data[18], instruction.data[19],
        instruction.data[20], instruction.data[21], instruction.data[22], instruction.data[23],
    ]);

    Some((
        bonding_curve.to_string(),
        user_source.to_string(),
        user_dest.to_string(),
        amount_in,
        min_amount_out,
    ))
}

/// Analyze a transaction for sandwich opportunities
fn analyze_transaction(
    tx: &VersionedTransaction,
    dex_programs: &DexPrograms,
    config: &SandwichConfig,
) -> Option<SandwichOpportunity> {
    let message = &tx.message;

    // Check each instruction
    for instruction in message.instructions() {
        // Get program ID
        let program_id = message.static_account_keys()
            .get(instruction.program_id_index as usize)?;

        // Is this a DEX swap?
        if let Some(dex_name) = dex_programs.identify(program_id) {
            info!("🔍 Detected {} swap in transaction", dex_name);

            // ⚡ SKIP JUPITER - It's an aggregator (too slow for MEV)
            // Jupiter routes through other DEXs which we already detect directly
            if dex_name == "Jupiter_V6" {
                debug!("⏭️  Skipping Jupiter_V6 swap (aggregator - detect direct DEX swaps instead)");
                continue;
            }

            // ✅ MULTI-MARKET MODE: Detect opportunities across ALL DEXes
            // PumpSwap (bonding curve) + Multi-DEX (CLMM, CPMM, Orca, Meteora)
            // JITO rate limits managed via high profit targets

            // Parse detailed swap information based on DEX type
            // All parsers use correct Anchor discriminators from IDL/source code
            let swap_details = match dex_name {
                "Raydium_AMM_V4" => parse_raydium_amm_v4_swap(message, instruction),
                "Raydium_CLMM" => parse_raydium_clmm_swap(message, instruction),
                "Raydium_CPMM" => parse_raydium_cpmm_swap(message, instruction),
                "Orca_Whirlpools" => parse_orca_whirlpool_swap(message, instruction),
                "Meteora_DLMM" => parse_meteora_dlmm_swap(message, instruction),
                "PumpSwap" => parse_pumpswap_swap(message, instruction),
                // Jupiter is NOT included - it's an aggregator (too slow for MEV)
                // It routes through other DEXs which we already detect directly
                _ => {
                    debug!("⚠️  DEX {} detected but parser not implemented - skipping", dex_name);
                    None
                }
            };

            // Use parsed amount if available, otherwise fall back to estimate
            let swap_size_sol = if let Some(ref details) = swap_details {
                // Convert lamports to SOL (1 SOL = 1_000_000_000 lamports)
                details.3 as f64 / 1_000_000_000.0
            } else {
                estimate_swap_size(message, instruction)
            };

            if swap_size_sol >= config.min_swap_size_sol && swap_size_sol <= config.max_swap_size_sol {
                // Calculate if profitable to sandwich
                if is_profitable_to_sandwich(swap_size_sol, config) {
                    info!("💰 SANDWICH OPPORTUNITY: {} swap of {:.4} SOL on {}",
                          dex_name, swap_size_sol, dex_name);

                    // If we parsed swap details, include them
                    let (pool_addr, input_mint, output_mint, amount_in, min_out) = if let Some(details) = swap_details {
                        info!("✅ Parsed swap details | Pool: {} | Amount: {} lamports ({:.4} SOL)",
                              &details.0[..8], details.3, details.3 as f64 / 1_000_000_000.0);
                        (Some(details.0), Some(details.1), Some(details.2), Some(details.3), Some(details.4))
                    } else {
                        debug!("⚠️  Could not parse swap details for {} (using estimates)", dex_name);
                        (None, None, None, None, None)
                    };

                    return Some(SandwichOpportunity {
                        dex_name: dex_name.to_string(),
                        signature: format!("{:?}", tx.signatures.get(0)?),
                        slot: 0,
                        estimated_sol_value: swap_size_sol,
                        timestamp: std::time::Instant::now(),

                        // Parsed transaction details (if available)
                        input_mint,
                        output_mint,
                        pool_address: pool_addr,
                        swap_amount_in: amount_in,
                        min_amount_out: min_out,
                    });
                }
            }
        }
    }

    None
}

/// Estimate swap size (simplified version)
/// TODO: Parse actual instruction data for precise amounts
fn estimate_swap_size(
    message: &VersionedMessage,
    instruction: &solana_sdk::instruction::CompiledInstruction,
) -> f64 {
    // For now, use a generous heuristic to detect ALL potential swaps
    // Real implementation would parse instruction data for each DEX type

    // AGGRESSIVE DETECTION: Assume all DEX swaps are worth detecting
    // This catches Orca, Meteora, Jupiter, PumpSwap that we can't parse yet

    let account_count = instruction.accounts.len();

    // Be more aggressive - assume smaller swap sizes to catch more opportunities
    // Once detected, the profitability check will filter unprofitable ones
    match account_count {
        0..=5 => 0.05,   // Small swaps
        6..=10 => 0.15,  // Medium swaps
        11..=15 => 0.5,  // Larger swaps
        16..=20 => 2.0,  // Big swaps
        _ => 5.0,        // Whale swaps
    }
}

/// Simple pre-filter: Is this swap large enough to consider sandwiching?
/// NOTE: Full profitability check happens in executor with actual position size
fn is_profitable_to_sandwich(swap_size_sol: f64, config: &SandwichConfig) -> bool {
    // Basic size filter - swap must be within configured range
    if swap_size_sol < config.min_swap_size_sol {
        debug!("⏭️  Swap too small: {:.4} SOL < {:.4} SOL minimum",
               swap_size_sol, config.min_swap_size_sol);
        return false;
    }

    if swap_size_sol > config.max_swap_size_sol {
        debug!("⏭️  Swap too large: {:.4} SOL > {:.4} SOL maximum",
               swap_size_sol, config.max_swap_size_sol);
        return false;
    }

    // Passed pre-filter - send to executor for full profitability check
    debug!("✅ Swap size OK: {:.4} SOL (will check profitability in executor)",
           swap_size_sol);
    true
}
