//! Raydium AMM V4 Swap Instruction Builder
//!
//! Builds swap instructions for Raydium AMM V4 pools to enable sandwich attacks.

use anyhow::{anyhow, Result};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::str::FromStr;
use crate::raydium_pool_state::RaydiumPoolState;

/// Raydium AMM V4 program ID
pub const RAYDIUM_AMM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Raydium AMM V4 swap instruction discriminator
pub const SWAP_INSTRUCTION_DISCRIMINATOR: u8 = 9;

/// Build a Raydium AMM V4 swap instruction
///
/// # Arguments
/// * `pool_state` - The Raydium pool state containing all required accounts
/// * `user_source_token` - User's source token account (token being sold)
/// * `user_dest_token` - User's destination token account (token being bought)
/// * `user_owner` - The wallet performing the swap
/// * `amount_in` - Amount of tokens to swap (in lamports)
/// * `min_amount_out` - Minimum amount of tokens to receive (slippage protection)
///
/// # Returns
/// A Solana instruction ready to be added to a transaction
pub fn build_raydium_swap_instruction(
    pool_state: &RaydiumPoolState,
    user_source_token: &Pubkey,
    user_dest_token: &Pubkey,
    user_owner: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction> {
    let program_id = Pubkey::from_str(RAYDIUM_AMM_V4_PROGRAM_ID)
        .map_err(|e| anyhow!("Invalid Raydium program ID: {}", e))?;

    // Build instruction data: [discriminator (1 byte), amount_in (8 bytes), min_amount_out (8 bytes)]
    let mut instruction_data = Vec::with_capacity(17);
    instruction_data.push(SWAP_INSTRUCTION_DISCRIMINATOR);
    instruction_data.extend_from_slice(&amount_in.to_le_bytes());
    instruction_data.extend_from_slice(&min_amount_out.to_le_bytes());

    // Raydium AMM V4 swap requires 12 accounts in specific order:
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

    let token_program = spl_token::id();

    let accounts = vec![
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new(pool_state.amm_id, false),
        AccountMeta::new_readonly(pool_state.amm_authority, false),
        AccountMeta::new(pool_state.amm_open_orders, false),
        AccountMeta::new(pool_state.amm_target_orders, false),
        AccountMeta::new(pool_state.pool_coin_token_account, false),
        AccountMeta::new(pool_state.pool_pc_token_account, false),
        AccountMeta::new_readonly(pool_state.serum_program_id, false),
        AccountMeta::new(pool_state.serum_market, false),
        AccountMeta::new(*user_source_token, false),
        AccountMeta::new(*user_dest_token, false),
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
pub fn build_frontrun_instruction(
    pool_state: &RaydiumPoolState,
    our_source_token: &Pubkey,
    our_dest_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction> {
    build_raydium_swap_instruction(
        pool_state,
        our_source_token,
        our_dest_token,
        our_wallet,
        amount_in,
        min_amount_out,
    )
}

/// Build a back-run swap instruction (sell after victim)
///
/// This sells the tokens we bought in the front-run, capturing the price impact
pub fn build_backrun_instruction(
    pool_state: &RaydiumPoolState,
    our_source_token: &Pubkey,
    our_dest_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction> {
    build_raydium_swap_instruction(
        pool_state,
        our_source_token,
        our_dest_token,
        our_wallet,
        amount_in,
        min_amount_out,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raydium_pool_state::RaydiumPoolState;

    #[test]
    fn test_build_swap_instruction() {
        // Create mock pool state
        let pool_id = Pubkey::new_unique();
        let pool_state = RaydiumPoolState {
            amm_id: pool_id,
            amm_authority: Pubkey::new_unique(),
            amm_open_orders: Pubkey::new_unique(),
            amm_target_orders: Pubkey::new_unique(),
            pool_coin_token_account: Pubkey::new_unique(),
            pool_pc_token_account: Pubkey::new_unique(),
            pool_withdraw_queue: Pubkey::new_unique(),
            pool_temp_lp_token_account: Pubkey::new_unique(),
            serum_program_id: Pubkey::new_unique(),
            serum_market: Pubkey::new_unique(),
            coin_mint: Pubkey::new_unique(),
            pc_mint: Pubkey::new_unique(),
        };

        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        let result = build_raydium_swap_instruction(
            &pool_state,
            &source,
            &dest,
            &owner,
            1_000_000_000, // 1 SOL
            900_000_000,   // 0.9 SOL min (10% slippage)
        );

        assert!(result.is_ok());
        let instruction = result.unwrap();

        // Verify instruction data format
        assert_eq!(instruction.data[0], SWAP_INSTRUCTION_DISCRIMINATOR);
        assert_eq!(instruction.data.len(), 17); // 1 + 8 + 8 bytes

        // Verify accounts count
        assert_eq!(instruction.accounts.len(), 12);

        // Verify last account is signer (user_owner)
        assert!(instruction.accounts[11].is_signer);

        // Verify pool accounts match pool_state
        assert_eq!(instruction.accounts[1].pubkey, pool_state.amm_id);
        assert_eq!(instruction.accounts[2].pubkey, pool_state.amm_authority);
        assert_eq!(instruction.accounts[3].pubkey, pool_state.amm_open_orders);
    }
}
