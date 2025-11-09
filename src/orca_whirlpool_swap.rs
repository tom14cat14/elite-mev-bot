//! Orca Whirlpools Swap Instruction Builder
//!
//! Builds swap instructions for Orca Whirlpool pools to enable sandwich attacks.
//! Reference: https://github.com/orca-so/whirlpools

use anyhow::{anyhow, Result};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::str::FromStr;
use crate::orca_whirlpool_state::OrcaWhirlpoolState;

/// Orca Whirlpools program ID
pub const ORCA_WHIRLPOOLS_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

/// Orca Whirlpools swap instruction discriminator (Anchor: sha256("global:swap")[:8])
pub const SWAP_INSTRUCTION_DISCRIMINATOR: [u8; 8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];

/// Build an Orca Whirlpool swap instruction
///
/// # Arguments
/// * `pool_state` - The Orca Whirlpool pool state containing all required accounts
/// * `user_source_token` - User's source token account (token being sold)
/// * `user_dest_token` - User's destination token account (token being bought)
/// * `user_owner` - The wallet performing the swap
/// * `amount` - Amount of tokens to swap (in lamports)
/// * `other_amount_threshold` - Minimum amount of tokens to receive (slippage protection)
/// * `sqrt_price_limit` - Price limit for the swap (use 0 for no limit)
/// * `amount_specified_is_input` - True if amount is input, false if amount is output
/// * `a_to_b` - True if swapping token A to B, false if B to A
///
/// # Returns
/// A Solana instruction ready to be added to a transaction
pub fn build_orca_whirlpool_swap_instruction(
    pool_state: &OrcaWhirlpoolState,
    user_source_token: &Pubkey,
    user_dest_token: &Pubkey,
    user_owner: &Pubkey,
    amount: u64,
    other_amount_threshold: u64,
    sqrt_price_limit: u128,
    amount_specified_is_input: bool,
    a_to_b: bool,
) -> Result<Instruction> {
    let program_id = Pubkey::from_str(ORCA_WHIRLPOOLS_PROGRAM_ID)
        .map_err(|e| anyhow!("Invalid Orca Whirlpools program ID: {}", e))?;

    // Build instruction data:
    // [discriminator (8 bytes), amount (8 bytes), other_amount_threshold (8 bytes),
    //  sqrt_price_limit (16 bytes), amount_specified_is_input (1 byte), a_to_b (1 byte)]
    let mut instruction_data = Vec::with_capacity(42);
    instruction_data.extend_from_slice(&SWAP_INSTRUCTION_DISCRIMINATOR);
    instruction_data.extend_from_slice(&amount.to_le_bytes());
    instruction_data.extend_from_slice(&other_amount_threshold.to_le_bytes());
    instruction_data.extend_from_slice(&sqrt_price_limit.to_le_bytes());
    instruction_data.push(amount_specified_is_input as u8);
    instruction_data.push(a_to_b as u8);

    // Derive tick arrays based on swap direction
    let (tick_array_0, tick_array_1, tick_array_2) = pool_state.derive_tick_arrays(a_to_b)?;

    // Orca Whirlpools swap requires these accounts in specific order:
    // 0: token_program (SPL Token program)
    // 1: token_authority (PDA authority for the program)
    // 2: whirlpool (the pool account)
    // 3: token_owner_account_a (user's token A account)
    // 4: token_vault_a (pool's vault for token A)
    // 5: token_owner_account_b (user's token B account)
    // 6: token_vault_b (pool's vault for token B)
    // 7: tick_array_0 (first tick array)
    // 8: tick_array_1 (second tick array)
    // 9: tick_array_2 (third tick array)
    // 10: oracle (price oracle account)
    // 11: user_owner (signer)

    let token_program = spl_token::id();

    // Derive token authority PDA (standard Orca pattern)
    let (token_authority, _bump) = Pubkey::find_program_address(
        &[b"vault"],
        &program_id,
    );

    // Determine which user account is A and which is B based on swap direction
    let (user_account_a, user_account_b) = if a_to_b {
        (*user_source_token, *user_dest_token)
    } else {
        (*user_dest_token, *user_source_token)
    };

    let accounts = vec![
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(token_authority, false),
        AccountMeta::new(pool_state.whirlpool, false),
        AccountMeta::new(user_account_a, false),
        AccountMeta::new(pool_state.token_vault_a, false),
        AccountMeta::new(user_account_b, false),
        AccountMeta::new(pool_state.token_vault_b, false),
        AccountMeta::new(tick_array_0, false),
        AccountMeta::new(tick_array_1, false),
        AccountMeta::new(tick_array_2, false),
        AccountMeta::new_readonly(pool_state.oracle, false),
        AccountMeta::new_readonly(*user_owner, true), // Signer
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
pub fn build_whirlpool_frontrun_instruction(
    pool_state: &OrcaWhirlpoolState,
    our_source_token: &Pubkey,
    our_dest_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    a_to_b: bool,
) -> Result<Instruction> {
    // Use conservative sqrt_price_limit (0 = no limit, allows max slippage)
    // In production, calculate based on current sqrt_price ± tolerance
    let sqrt_price_limit = 0u128;
    let amount_specified_is_input = true;

    build_orca_whirlpool_swap_instruction(
        pool_state,
        our_source_token,
        our_dest_token,
        our_wallet,
        amount_in,
        min_amount_out,
        sqrt_price_limit,
        amount_specified_is_input,
        a_to_b,
    )
}

/// Build a back-run swap instruction (sell after victim)
///
/// This sells the tokens we bought in the front-run, capturing the price impact
pub fn build_whirlpool_backrun_instruction(
    pool_state: &OrcaWhirlpoolState,
    our_source_token: &Pubkey,
    our_dest_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    a_to_b: bool,
) -> Result<Instruction> {
    // Use conservative sqrt_price_limit (0 = no limit)
    let sqrt_price_limit = 0u128;
    let amount_specified_is_input = true;

    build_orca_whirlpool_swap_instruction(
        pool_state,
        our_source_token,
        our_dest_token,
        our_wallet,
        amount_in,
        min_amount_out,
        sqrt_price_limit,
        amount_specified_is_input,
        a_to_b,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orca_whirlpool_state::OrcaWhirlpoolState;

    #[test]
    fn test_build_whirlpool_swap_instruction() {
        // Create mock pool state
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

        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        let result = build_orca_whirlpool_swap_instruction(
            &pool_state,
            &source,
            &dest,
            &owner,
            1_000_000_000, // 1 SOL
            900_000_000,   // 0.9 SOL min (10% slippage)
            0,             // No price limit
            true,          // Amount specified is input
            true,          // A → B
        );

        assert!(result.is_ok());
        let instruction = result.unwrap();

        // Verify instruction data format
        assert_eq!(&instruction.data[0..8], &SWAP_INSTRUCTION_DISCRIMINATOR);
        assert_eq!(instruction.data.len(), 42); // 8 + 8 + 8 + 16 + 1 + 1 bytes

        // Verify accounts count (should be 12)
        assert_eq!(instruction.accounts.len(), 12);

        // Verify last account is signer (user_owner)
        assert!(instruction.accounts[11].is_signer);

        // Verify pool accounts match pool_state
        assert_eq!(instruction.accounts[2].pubkey, pool_state.whirlpool);
        assert_eq!(instruction.accounts[4].pubkey, pool_state.token_vault_a);
        assert_eq!(instruction.accounts[6].pubkey, pool_state.token_vault_b);
        assert_eq!(instruction.accounts[10].pubkey, pool_state.oracle);
    }

    #[test]
    fn test_swap_direction() {
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

        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();

        // Test A → B swap
        let ix_a_to_b = build_orca_whirlpool_swap_instruction(
            &pool_state,
            &source,
            &dest,
            &Pubkey::new_unique(),
            1_000_000,
            900_000,
            0,
            true,
            true, // a_to_b = true
        )
        .unwrap();

        // When a_to_b=true, user_account_a should be source
        assert_eq!(ix_a_to_b.accounts[3].pubkey, source);
        assert_eq!(ix_a_to_b.accounts[5].pubkey, dest);

        // Test B → A swap
        let ix_b_to_a = build_orca_whirlpool_swap_instruction(
            &pool_state,
            &source,
            &dest,
            &Pubkey::new_unique(),
            1_000_000,
            900_000,
            0,
            true,
            false, // a_to_b = false
        )
        .unwrap();

        // When a_to_b=false, user_account_a should be dest, user_account_b should be source
        assert_eq!(ix_b_to_a.accounts[3].pubkey, dest);
        assert_eq!(ix_b_to_a.accounts[5].pubkey, source);
    }
}
