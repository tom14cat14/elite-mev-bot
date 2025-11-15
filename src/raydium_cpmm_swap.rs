//! Raydium CPMM (Constant Product Market Maker) Swap Instruction Builder
//!
//! Builds swap instructions for Raydium CPMM pools to enable sandwich attacks.
//! CPMM is a simple constant product AMM (x * y = k).

use crate::raydium_cpmm_state::RaydiumCpmmPoolState;
use anyhow::{anyhow, Result};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::str::FromStr;

/// Raydium CPMM program ID
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

/// Raydium CPMM swap_base_input instruction discriminator (byte 0 = 0x09)
pub const SWAP_BASE_INPUT_DISCRIMINATOR: u8 = 0x09;

/// Build a Raydium CPMM swap instruction
///
/// # Arguments
/// * `pool_state` - The Raydium CPMM pool state containing all required accounts
/// * `user_source_token` - User's source token account (token being sold)
/// * `user_dest_token` - User's destination token account (token being bought)
/// * `user_owner` - The wallet performing the swap
/// * `amount_in` - Amount of tokens to swap (in lamports)
/// * `min_amount_out` - Minimum amount of tokens to receive (slippage protection)
///
/// # Returns
/// A Solana instruction ready to be added to a transaction
pub fn build_raydium_cpmm_swap_instruction(
    pool_state: &RaydiumCpmmPoolState,
    user_source_token: &Pubkey,
    user_dest_token: &Pubkey,
    user_owner: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction> {
    let program_id = Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID)
        .map_err(|e| anyhow!("Invalid Raydium CPMM program ID: {}", e))?;

    // Build instruction data: [discriminator (1 byte), amount_in (8 bytes), min_amount_out (8 bytes)]
    // Note: CPMM uses byte discriminator (0x09), NOT 8-byte Anchor discriminator
    let mut instruction_data = Vec::with_capacity(17);
    instruction_data.push(SWAP_BASE_INPUT_DISCRIMINATOR);
    instruction_data.extend_from_slice(&amount_in.to_le_bytes());
    instruction_data.extend_from_slice(&min_amount_out.to_le_bytes());

    // Raydium CPMM swap requires these accounts in specific order:
    // 0: pool (the CPMM pool account)
    // 1: authority (PDA authority for the pool)
    // 2: token_program (SPL Token program)
    // 3: token_0_vault (pool's vault for token 0)
    // 4: token_1_vault (pool's vault for token 1)
    // 5: user_source_token (user's input token account)
    // 6: user_dest_token (user's output token account)
    // 7: user_owner (signer)

    let token_program = spl_token::id();

    let accounts = vec![
        AccountMeta::new(pool_state.pool_id, false),
        AccountMeta::new_readonly(pool_state.authority, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new(pool_state.token_0_vault, false),
        AccountMeta::new(pool_state.token_1_vault, false),
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
pub fn build_cpmm_frontrun_instruction(
    pool_state: &RaydiumCpmmPoolState,
    our_source_token: &Pubkey,
    our_dest_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction> {
    build_raydium_cpmm_swap_instruction(
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
pub fn build_cpmm_backrun_instruction(
    pool_state: &RaydiumCpmmPoolState,
    our_source_token: &Pubkey,
    our_dest_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction> {
    build_raydium_cpmm_swap_instruction(
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
    use crate::raydium_cpmm_state::RaydiumCpmmPoolState;

    #[test]
    fn test_build_cpmm_swap_instruction() {
        // Create mock pool state
        let pool_id = Pubkey::new_unique();
        let pool_state = RaydiumCpmmPoolState {
            pool_id,
            token_0_mint: Pubkey::new_unique(),
            token_1_mint: Pubkey::new_unique(),
            token_0_vault: Pubkey::new_unique(),
            token_1_vault: Pubkey::new_unique(),
            lp_mint: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
        };

        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        let result = build_raydium_cpmm_swap_instruction(
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
        assert_eq!(instruction.data[0], SWAP_BASE_INPUT_DISCRIMINATOR);
        assert_eq!(instruction.data.len(), 17); // 1 + 8 + 8 bytes

        // Verify accounts count
        assert_eq!(instruction.accounts.len(), 8);

        // Verify last account is signer (user_owner)
        assert!(instruction.accounts[7].is_signer);

        // Verify pool accounts match pool_state
        assert_eq!(instruction.accounts[0].pubkey, pool_state.pool_id);
        assert_eq!(instruction.accounts[1].pubkey, pool_state.authority);
        assert_eq!(instruction.accounts[3].pubkey, pool_state.token_0_vault);
        assert_eq!(instruction.accounts[4].pubkey, pool_state.token_1_vault);

        // Verify user token accounts
        assert_eq!(instruction.accounts[5].pubkey, source);
        assert_eq!(instruction.accounts[6].pubkey, dest);
    }

    #[test]
    fn test_instruction_data_format() {
        let pool_state = RaydiumCpmmPoolState {
            pool_id: Pubkey::new_unique(),
            token_0_mint: Pubkey::new_unique(),
            token_1_mint: Pubkey::new_unique(),
            token_0_vault: Pubkey::new_unique(),
            token_1_vault: Pubkey::new_unique(),
            lp_mint: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
        };

        let amount_in = 1_234_567_890u64;
        let min_amount_out = 987_654_321u64;

        let ix = build_raydium_cpmm_swap_instruction(
            &pool_state,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            amount_in,
            min_amount_out,
        )
        .unwrap();

        // Verify discriminator
        assert_eq!(ix.data[0], 0x09);

        // Verify amount_in encoding (little-endian u64 at bytes 1-8)
        let parsed_amount_in = u64::from_le_bytes([
            ix.data[1], ix.data[2], ix.data[3], ix.data[4], ix.data[5], ix.data[6], ix.data[7],
            ix.data[8],
        ]);
        assert_eq!(parsed_amount_in, amount_in);

        // Verify min_amount_out encoding (little-endian u64 at bytes 9-16)
        let parsed_min_out = u64::from_le_bytes([
            ix.data[9],
            ix.data[10],
            ix.data[11],
            ix.data[12],
            ix.data[13],
            ix.data[14],
            ix.data[15],
            ix.data[16],
        ]);
        assert_eq!(parsed_min_out, min_amount_out);
    }
}
