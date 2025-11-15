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

    // Raydium CLMM swap requires these accounts in specific order:
    // 0: pool_state (the CLMM pool)
    // 1: token_program (SPL Token program)
    // 2: user_source_token_account (user's input token account)
    // 3: user_destination_token_account (user's output token account)
    // 4: pool_vault_a (pool's vault for token A)
    // 5: pool_vault_b (pool's vault for token B)
    // 6: observation_state (price oracle account)
    // 7: user_owner (signer)

    let token_program = spl_token::id();

    // Determine which vault is input and which is output based on swap direction
    let (vault_input, vault_output) = if is_base_input {
        (pool_state.token_vault_a, pool_state.token_vault_b)
    } else {
        (pool_state.token_vault_b, pool_state.token_vault_a)
    };

    let accounts = vec![
        AccountMeta::new(pool_state.pool_id, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new(*user_source_token, false),
        AccountMeta::new(*user_dest_token, false),
        AccountMeta::new(vault_input, false),
        AccountMeta::new(vault_output, false),
        AccountMeta::new(pool_state.observation_key, false),
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
        // Create mock pool state
        let pool_id = Pubkey::new_unique();
        let pool_state = RaydiumClmmPoolState {
            pool_id,
            token_mint_a: Pubkey::new_unique(),
            token_mint_b: Pubkey::new_unique(),
            token_vault_a: Pubkey::new_unique(),
            token_vault_b: Pubkey::new_unique(),
            observation_key: Pubkey::new_unique(),
            sqrt_price_x64: 1000000000000000000, // Example price
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

        // Verify accounts count
        assert_eq!(instruction.accounts.len(), 8);

        // Verify last account is signer (user_owner)
        assert!(instruction.accounts[7].is_signer);

        // Verify pool accounts match pool_state
        assert_eq!(instruction.accounts[0].pubkey, pool_state.pool_id);
        assert_eq!(instruction.accounts[4].pubkey, pool_state.token_vault_a); // Input vault (base)
        assert_eq!(instruction.accounts[5].pubkey, pool_state.token_vault_b); // Output vault
        assert_eq!(instruction.accounts[6].pubkey, pool_state.observation_key);
    }

    #[test]
    fn test_swap_direction() {
        let pool_state = RaydiumClmmPoolState {
            pool_id: Pubkey::new_unique(),
            token_mint_a: Pubkey::new_unique(),
            token_mint_b: Pubkey::new_unique(),
            token_vault_a: Pubkey::new_unique(),
            token_vault_b: Pubkey::new_unique(),
            observation_key: Pubkey::new_unique(),
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

        // Vault A should be input (index 4), Vault B should be output (index 5)
        assert_eq!(ix_a_to_b.accounts[4].pubkey, pool_state.token_vault_a);
        assert_eq!(ix_a_to_b.accounts[5].pubkey, pool_state.token_vault_b);

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

        // Vault B should be input (index 4), Vault A should be output (index 5)
        assert_eq!(ix_b_to_a.accounts[4].pubkey, pool_state.token_vault_b);
        assert_eq!(ix_b_to_a.accounts[5].pubkey, pool_state.token_vault_a);
    }
}
