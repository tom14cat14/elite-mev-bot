//! Meteora DLMM (Dynamic Liquidity Market Maker) Swap Instruction Builder
//!
//! Builds swap instructions for Meteora DLMM pools to enable sandwich attacks.
//! Reference: https://github.com/MeteoraAg/dlmm-sdk

use crate::meteora_dlmm_state::MeteoraDlmmPoolState;
use anyhow::{anyhow, Result};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::str::FromStr;

/// Meteora DLMM program ID
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

/// Meteora DLMM swap instruction discriminator (Anchor: sha256("global:swap")[:8])
pub const SWAP_INSTRUCTION_DISCRIMINATOR: [u8; 8] =
    [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];

/// Build a Meteora DLMM swap instruction
///
/// # Arguments
/// * `pool_state` - The Meteora DLMM pool state containing all required accounts
/// * `user_source_token` - User's source token account (token being sold)
/// * `user_dest_token` - User's destination token account (token being bought)
/// * `user_owner` - The wallet performing the swap
/// * `amount_in` - Amount of tokens to swap (in lamports)
/// * `min_amount_out` - Minimum amount of tokens to receive (slippage protection)
///
/// # Returns
/// A Solana instruction ready to be added to a transaction
pub fn build_meteora_dlmm_swap_instruction(
    pool_state: &MeteoraDlmmPoolState,
    user_source_token: &Pubkey,
    user_dest_token: &Pubkey,
    user_owner: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction> {
    let program_id = Pubkey::from_str(METEORA_DLMM_PROGRAM_ID)
        .map_err(|e| anyhow!("Invalid Meteora DLMM program ID: {}", e))?;

    // Build instruction data:
    // [discriminator (8 bytes), amount_in (8 bytes), min_amount_out (8 bytes)]
    let mut instruction_data = Vec::with_capacity(24);
    instruction_data.extend_from_slice(&SWAP_INSTRUCTION_DISCRIMINATOR);
    instruction_data.extend_from_slice(&amount_in.to_le_bytes());
    instruction_data.extend_from_slice(&min_amount_out.to_le_bytes());

    // Meteora DLMM swap requires these accounts in specific order:
    // 0: lb_pair (the DLMM pool)
    // 1: bin_array_bitmap_extension (optional, can be same as lb_pair)
    // 2: reserve_x (pool's reserve for token X)
    // 3: reserve_y (pool's reserve for token Y)
    // 4: user_token_in (user's input token account)
    // 5: user_token_out (user's output token account)
    // 6: token_x_mint
    // 7: token_y_mint
    // 8: oracle
    // 9: token_program (SPL Token program)

    let token_program = spl_token::id();

    let accounts = vec![
        AccountMeta::new(pool_state.lb_pair, false),
        AccountMeta::new(pool_state.lb_pair, false), // bin_array_bitmap_extension (use lb_pair as fallback)
        AccountMeta::new(pool_state.reserve_x, false),
        AccountMeta::new(pool_state.reserve_y, false),
        AccountMeta::new(*user_source_token, false),
        AccountMeta::new(*user_dest_token, false),
        AccountMeta::new_readonly(pool_state.token_x_mint, false),
        AccountMeta::new_readonly(pool_state.token_y_mint, false),
        AccountMeta::new_readonly(pool_state.oracle, false),
        AccountMeta::new_readonly(token_program, false),
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
pub fn build_dlmm_frontrun_instruction(
    pool_state: &MeteoraDlmmPoolState,
    our_source_token: &Pubkey,
    our_dest_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction> {
    build_meteora_dlmm_swap_instruction(
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
pub fn build_dlmm_backrun_instruction(
    pool_state: &MeteoraDlmmPoolState,
    our_source_token: &Pubkey,
    our_dest_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction> {
    build_meteora_dlmm_swap_instruction(
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
    use crate::meteora_dlmm_state::MeteoraDlmmPoolState;

    #[test]
    fn test_build_dlmm_swap_instruction() {
        // Create mock pool state
        let lb_pair = Pubkey::new_unique();
        let pool_state = MeteoraDlmmPoolState {
            lb_pair,
            token_x_mint: Pubkey::new_unique(),
            token_y_mint: Pubkey::new_unique(),
            reserve_x: Pubkey::new_unique(),
            reserve_y: Pubkey::new_unique(),
            oracle: Pubkey::new_unique(),
            active_id: 0,
            bin_step: 10,
        };

        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        let result = build_meteora_dlmm_swap_instruction(
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
        assert_eq!(&instruction.data[0..8], &SWAP_INSTRUCTION_DISCRIMINATOR);
        assert_eq!(instruction.data.len(), 24); // 8 + 8 + 8 bytes

        // Verify accounts count (should be 10)
        assert_eq!(instruction.accounts.len(), 10);

        // Verify pool accounts match pool_state
        assert_eq!(instruction.accounts[0].pubkey, pool_state.lb_pair);
        assert_eq!(instruction.accounts[2].pubkey, pool_state.reserve_x);
        assert_eq!(instruction.accounts[3].pubkey, pool_state.reserve_y);
        assert_eq!(instruction.accounts[6].pubkey, pool_state.token_x_mint);
        assert_eq!(instruction.accounts[7].pubkey, pool_state.token_y_mint);
        assert_eq!(instruction.accounts[8].pubkey, pool_state.oracle);

        // Verify user token accounts
        assert_eq!(instruction.accounts[4].pubkey, source);
        assert_eq!(instruction.accounts[5].pubkey, dest);
    }

    #[test]
    fn test_instruction_data_format() {
        let pool_state = MeteoraDlmmPoolState {
            lb_pair: Pubkey::new_unique(),
            token_x_mint: Pubkey::new_unique(),
            token_y_mint: Pubkey::new_unique(),
            reserve_x: Pubkey::new_unique(),
            reserve_y: Pubkey::new_unique(),
            oracle: Pubkey::new_unique(),
            active_id: 0,
            bin_step: 10,
        };

        let amount_in = 1_234_567_890u64;
        let min_amount_out = 987_654_321u64;

        let ix = build_meteora_dlmm_swap_instruction(
            &pool_state,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            amount_in,
            min_amount_out,
        )
        .unwrap();

        // Verify discriminator
        assert_eq!(&ix.data[0..8], &SWAP_INSTRUCTION_DISCRIMINATOR);

        // Verify amount_in encoding (little-endian u64 at bytes 8-15)
        let parsed_amount_in = u64::from_le_bytes([
            ix.data[8],
            ix.data[9],
            ix.data[10],
            ix.data[11],
            ix.data[12],
            ix.data[13],
            ix.data[14],
            ix.data[15],
        ]);
        assert_eq!(parsed_amount_in, amount_in);

        // Verify min_amount_out encoding (little-endian u64 at bytes 16-23)
        let parsed_min_out = u64::from_le_bytes([
            ix.data[16],
            ix.data[17],
            ix.data[18],
            ix.data[19],
            ix.data[20],
            ix.data[21],
            ix.data[22],
            ix.data[23],
        ]);
        assert_eq!(parsed_min_out, min_amount_out);
    }
}
