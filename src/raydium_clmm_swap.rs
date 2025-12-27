//! Raydium CLMM (Concentrated Liquidity Market Maker) Swap Instruction Builder
//!
//! Builds swap instructions for Raydium CLMM pools to enable sandwich attacks.
//! Reference: https://github.com/raydium-io/raydium-clmm

use crate::raydium_clmm_state::RaydiumClmmPoolState;
use anyhow::{anyhow, Result};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::str::FromStr;

/// Raydium CLMM program ID
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

/// Raydium CLMM swap instruction discriminator (Anchor: sha256("global:swap")[:8])
pub const SWAP_INSTRUCTION_DISCRIMINATOR: [u8; 8] =
    [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];

/// Number of ticks per tick array (Raydium CLMM constant)
pub const TICK_ARRAY_SIZE: i32 = 60;

/// Calculate the tick array start index for a given tick
pub fn get_tick_array_start_index(tick: i32, tick_spacing: u16) -> i32 {
    let ticks_in_array = TICK_ARRAY_SIZE * (tick_spacing as i32);
    let start = (tick / ticks_in_array) * ticks_in_array;
    // Handle negative ticks
    if tick < 0 && tick % ticks_in_array != 0 {
        start - ticks_in_array
    } else {
        start
    }
}

/// Derive the tick array PDA for a given pool and start index
/// NOTE: Raydium CLMM uses BIG-ENDIAN for start_index (confirmed by on-chain verification)
pub fn derive_tick_array_pda(pool_id: &Pubkey, start_index: i32, program_id: &Pubkey) -> Pubkey {
    let seeds = &[
        b"tick_array",
        pool_id.as_ref(),
        &start_index.to_be_bytes(),  // Big-endian (verified 2025-12-07)
    ];
    let (pda, _bump) = Pubkey::find_program_address(seeds, program_id);
    pda
}

/// Verify that a tick_array account exists and is owned by Raydium CLMM
/// Returns Ok(tick_array_pubkey) if valid, Err if not initialized
pub fn verify_tick_array_exists(
    rpc_client: &solana_client::rpc_client::RpcClient,
    pool_id: &Pubkey,
    tick: i32,
    tick_spacing: u16,
) -> Result<Pubkey> {
    let program_id = Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID)
        .map_err(|e| anyhow!("Invalid Raydium CLMM program ID: {}", e))?;

    let start_index = get_tick_array_start_index(tick, tick_spacing);
    let tick_array_pda = derive_tick_array_pda(pool_id, start_index, &program_id);

    // Debug: show the start_index calculation details
    let ticks_in_array = TICK_ARRAY_SIZE * (tick_spacing as i32);
    tracing::info!(
        "🔢 tick_array derivation: tick={}, tick_spacing={}, ticks_in_array={}, start_index={}, PDA={}",
        tick, tick_spacing, ticks_in_array, start_index, tick_array_pda
    );

    // Check if account exists and is owned by CLMM program
    match rpc_client.get_account(&tick_array_pda) {
        Ok(account) => {
            if account.owner == program_id {
                tracing::info!(
                    "✅ tick_array verified: {} (start_index={}, account_data_len={})",
                    tick_array_pda, start_index, account.data.len()
                );
                Ok(tick_array_pda)
            } else {
                Err(anyhow!(
                    "tick_array {} exists but owned by {} (expected {})",
                    tick_array_pda, account.owner, program_id
                ))
            }
        }
        Err(e) => {
            Err(anyhow!(
                "tick_array {} not initialized (start_index={}, tick={}, tick_spacing={}): {}",
                tick_array_pda, start_index, tick, tick_spacing, e
            ))
        }
    }
}

/// Build a Raydium CLMM swap instruction
///
/// # Arguments
/// * `pool_state` - The Raydium CLMM pool state containing all required accounts
/// * `user_source_token` - User's source token account (token being sold)
/// * `user_dest_token` - User's destination token account (token being bought)
/// * `user_owner` - The wallet performing the swap
/// * `amount_in` - Amount of tokens to swap (in lamports)
/// * `min_amount_out` - Minimum amount of tokens to receive (slippage protection)
/// * `sqrt_price_limit` - Price limit for the swap (use 0 for no limit, or conservative value)
/// * `is_base_input` - True if swapping token A to B, false if B to A
///
/// # Returns
/// A Solana instruction ready to be added to a transaction
pub fn build_raydium_clmm_swap_instruction(
    pool_state: &RaydiumClmmPoolState,
    user_source_token: &Pubkey,
    user_dest_token: &Pubkey,
    user_owner: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    sqrt_price_limit: u128,
    is_base_input: bool,
) -> Result<Instruction> {
    let program_id = Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID)
        .map_err(|e| anyhow!("Invalid Raydium CLMM program ID: {}", e))?;

    // Build instruction data:
    // [discriminator (8 bytes), amount_in (8 bytes), min_amount_out (8 bytes),
    //  sqrt_price_limit (16 bytes), is_base_input (1 byte)]
    let mut instruction_data = Vec::with_capacity(41);
    instruction_data.extend_from_slice(&SWAP_INSTRUCTION_DISCRIMINATOR);
    instruction_data.extend_from_slice(&amount_in.to_le_bytes());
    instruction_data.extend_from_slice(&min_amount_out.to_le_bytes());
    instruction_data.extend_from_slice(&sqrt_price_limit.to_le_bytes());
    instruction_data.push(is_base_input as u8);

    // Raydium CLMM SwapV2 account layout (verified from on-chain transactions):
    // 0: payer (signer)
    // 1: amm_config
    // 2: pool_state
    // 3: input_token_account
    // 4: output_token_account
    // 5: input_vault
    // 6: output_vault
    // 7: observation_state
    // 8: token_program (SPL Token)
    // 9: token_program_2022 (Token-2022)
    // 10: memo_program
    // 11: input_vault_mint
    // 12: output_vault_mint
    // remaining: tick_arrays (1 or more)

    let token_program = spl_token::id();
    let token_program_2022 = Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
        .expect("Token-2022 program ID");
    let memo_program = Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
        .expect("Memo program ID");

    // Determine which vault/mint is input and which is output based on swap direction
    let (vault_input, vault_output, mint_input, mint_output) = if is_base_input {
        (pool_state.token_vault_a, pool_state.token_vault_b, pool_state.token_mint_a, pool_state.token_mint_b)
    } else {
        (pool_state.token_vault_b, pool_state.token_vault_a, pool_state.token_mint_b, pool_state.token_mint_a)
    };

    // Calculate tick_array PDA based on current tick and tick spacing
    let tick_array_start_index = get_tick_array_start_index(pool_state.tick_current, pool_state.tick_spacing);
    let tick_array = derive_tick_array_pda(&pool_state.pool_id, tick_array_start_index, &program_id);

    // DEBUG: tick_array derivation - using info! to ensure it appears
    tracing::info!(
        "🎯 CLMM tick_array: tick_current={}, tick_spacing={}, start_index={}, tick_array={}",
        pool_state.tick_current, pool_state.tick_spacing, tick_array_start_index, tick_array
    );

    let accounts = vec![
        AccountMeta::new(*user_owner, true),              // 0: payer (SIGNER)
        AccountMeta::new_readonly(pool_state.amm_config, false), // 1: amm_config
        AccountMeta::new(pool_state.pool_id, false),      // 2: pool_state
        AccountMeta::new(*user_source_token, false),      // 3: input_token_account
        AccountMeta::new(*user_dest_token, false),        // 4: output_token_account
        AccountMeta::new(vault_input, false),             // 5: input_vault
        AccountMeta::new(vault_output, false),            // 6: output_vault
        AccountMeta::new(pool_state.observation_key, false), // 7: observation_state
        AccountMeta::new_readonly(token_program, false),  // 8: token_program (SPL Token)
        AccountMeta::new_readonly(token_program_2022, false), // 9: token_program_2022
        AccountMeta::new_readonly(memo_program, false),   // 10: memo_program
        AccountMeta::new_readonly(mint_input, false),     // 11: input_vault_mint
        AccountMeta::new_readonly(mint_output, false),    // 12: output_vault_mint
        AccountMeta::new(tick_array, false),              // 13+: tick_array (remaining accounts)
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data: instruction_data,
    })
}

/// Build a front-run swap instruction (buy before victim)
///
/// This buys the same token the victim is buying, driving up the price
pub fn build_clmm_frontrun_instruction(
    pool_state: &RaydiumClmmPoolState,
    our_source_token: &Pubkey,
    our_dest_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    is_base_input: bool,
) -> Result<Instruction> {
    // Use conservative sqrt_price_limit (0 = no limit, allows max slippage)
    // In production, calculate based on current sqrt_price_x64 ± tolerance
    let sqrt_price_limit = 0u128;

    build_raydium_clmm_swap_instruction(
        pool_state,
        our_source_token,
        our_dest_token,
        our_wallet,
        amount_in,
        min_amount_out,
        sqrt_price_limit,
        is_base_input,
    )
}

/// Build a back-run swap instruction (sell after victim)
///
/// This sells the tokens we bought in the front-run, capturing the price impact
pub fn build_clmm_backrun_instruction(
    pool_state: &RaydiumClmmPoolState,
    our_source_token: &Pubkey,
    our_dest_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    is_base_input: bool,
) -> Result<Instruction> {
    // Use conservative sqrt_price_limit (0 = no limit)
    let sqrt_price_limit = 0u128;

    build_raydium_clmm_swap_instruction(
        pool_state,
        our_source_token,
        our_dest_token,
        our_wallet,
        amount_in,
        min_amount_out,
        sqrt_price_limit,
        is_base_input,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raydium_clmm_state::RaydiumClmmPoolState;

    #[test]
    fn test_build_clmm_swap_instruction() {
        // Create mock pool state with all required fields
        let pool_id = Pubkey::new_unique();
        let pool_state = RaydiumClmmPoolState {
            pool_id,
            amm_config: Pubkey::new_unique(),
            token_mint_a: Pubkey::new_unique(),
            token_mint_b: Pubkey::new_unique(),
            token_vault_a: Pubkey::new_unique(),
            token_vault_b: Pubkey::new_unique(),
            observation_key: Pubkey::new_unique(),
            tick_spacing: 10,
            sqrt_price_x64: 1000000000000000000,
            tick_current: 0,
        };

        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        let result = build_raydium_clmm_swap_instruction(
            &pool_state,
            &source,
            &dest,
            &owner,
            1_000_000_000, // 1 SOL
            900_000_000,   // 0.9 SOL min (10% slippage)
            0,             // No price limit
            true,          // Base input (A → B)
        );

        assert!(result.is_ok());
        let instruction = result.unwrap();

        // Verify instruction data format
        assert_eq!(&instruction.data[0..8], &SWAP_INSTRUCTION_DISCRIMINATOR);
        assert_eq!(instruction.data.len(), 41); // 8 + 8 + 8 + 16 + 1 bytes

        // Verify accounts count (14 total: 13 fixed + 1 tick_array)
        assert_eq!(instruction.accounts.len(), 14);

        // Verify first account is signer (payer/user_owner)
        assert!(instruction.accounts[0].is_signer);

        // Verify pool accounts match pool_state
        assert_eq!(instruction.accounts[2].pubkey, pool_state.pool_id);
        assert_eq!(instruction.accounts[5].pubkey, pool_state.token_vault_a); // Input vault (base)
        assert_eq!(instruction.accounts[6].pubkey, pool_state.token_vault_b); // Output vault
        assert_eq!(instruction.accounts[7].pubkey, pool_state.observation_key);
    }

    #[test]
    fn test_swap_direction() {
        let pool_state = RaydiumClmmPoolState {
            pool_id: Pubkey::new_unique(),
            amm_config: Pubkey::new_unique(),
            token_mint_a: Pubkey::new_unique(),
            token_mint_b: Pubkey::new_unique(),
            token_vault_a: Pubkey::new_unique(),
            token_vault_b: Pubkey::new_unique(),
            observation_key: Pubkey::new_unique(),
            tick_spacing: 10,
            sqrt_price_x64: 1000000000000000000,
            tick_current: 0,
        };

        // Test A → B swap
        let ix_a_to_b = build_raydium_clmm_swap_instruction(
            &pool_state,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1_000_000,
            900_000,
            0,
            true, // is_base_input = true
        )
        .unwrap();

        // Vault A should be input (index 5), Vault B should be output (index 6)
        assert_eq!(ix_a_to_b.accounts[5].pubkey, pool_state.token_vault_a);
        assert_eq!(ix_a_to_b.accounts[6].pubkey, pool_state.token_vault_b);

        // Test B → A swap
        let ix_b_to_a = build_raydium_clmm_swap_instruction(
            &pool_state,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1_000_000,
            900_000,
            0,
            false, // is_base_input = false
        )
        .unwrap();

        // Vault B should be input (index 5), Vault A should be output (index 6)
        assert_eq!(ix_b_to_a.accounts[5].pubkey, pool_state.token_vault_b);
        assert_eq!(ix_b_to_a.accounts[6].pubkey, pool_state.token_vault_a);
    }
}
