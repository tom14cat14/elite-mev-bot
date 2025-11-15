//! PumpSwap Bonding Curve Swap Instruction Builder
//!
//! Builds swap instructions for PumpSwap bonding curve pools to enable sandwich attacks.
//! PumpSwap uses a simple bonding curve model similar to pump.fun.

use crate::pumpswap_state::PumpSwapBondingCurveState;
use anyhow::{anyhow, Result};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};
use std::str::FromStr;

/// PumpSwap program ID
pub const PUMPSWAP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// PumpSwap Buy instruction discriminator
pub const BUY_INSTRUCTION_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];

/// PumpSwap Sell instruction discriminator
pub const SELL_INSTRUCTION_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

/// Build a PumpSwap buy instruction
///
/// # Arguments
/// * `pool_state` - The PumpSwap bonding curve state containing all required accounts
/// * `associated_user` - User's associated token account for the token
/// * `user_wallet` - The user's wallet (signer)
/// * `amount_sol` - Amount of SOL to spend (in lamports)
/// * `min_tokens_out` - Minimum tokens to receive (slippage protection)
///
/// # Returns
/// A Solana instruction ready to be added to a transaction
pub fn build_pumpswap_buy_instruction(
    pool_state: &PumpSwapBondingCurveState,
    associated_user: &Pubkey,
    user_wallet: &Pubkey,
    amount_sol: u64,
    min_tokens_out: u64,
) -> Result<Instruction> {
    let program_id = Pubkey::from_str(PUMPSWAP_PROGRAM_ID)
        .map_err(|e| anyhow!("Invalid PumpSwap program ID: {}", e))?;

    // Build instruction data:
    // [discriminator (8 bytes), amount_in (8 bytes), min_amount_out (8 bytes)]
    let mut instruction_data = Vec::with_capacity(24);
    instruction_data.extend_from_slice(&BUY_INSTRUCTION_DISCRIMINATOR);
    instruction_data.extend_from_slice(&amount_sol.to_le_bytes());
    instruction_data.extend_from_slice(&min_tokens_out.to_le_bytes());

    // PumpSwap buy requires these accounts in specific order:
    // 0: global (global state account)
    // 1: fee_recipient (fee destination account)
    // 2: mint (token mint)
    // 3: bonding_curve (bonding curve state)
    // 4: associated_bonding_curve (bonding curve's token account)
    // 5: associated_user (user's token account)
    // 6: user (user wallet - signer)
    // 7: system_program
    // 8: token_program
    // 9: rent
    // 10: event_authority
    // 11: program

    let token_program = spl_token::id();
    let event_authority =
        Pubkey::from_str("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1").unwrap_or(program_id);

    let accounts = vec![
        AccountMeta::new(pool_state.global, false),
        AccountMeta::new(pool_state.fee_recipient, false),
        AccountMeta::new_readonly(pool_state.token_mint, false),
        AccountMeta::new(pool_state.bonding_curve, false),
        AccountMeta::new(pool_state.associated_bonding_curve, false),
        AccountMeta::new(*associated_user, false),
        AccountMeta::new(*user_wallet, true), // Signer
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(program_id, false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data: instruction_data,
    })
}

/// Build a PumpSwap sell instruction
///
/// # Arguments
/// * `pool_state` - The PumpSwap bonding curve state containing all required accounts
/// * `associated_user` - User's associated token account for the token
/// * `user_wallet` - The user's wallet (signer)
/// * `amount_tokens` - Amount of tokens to sell
/// * `min_sol_out` - Minimum SOL to receive (slippage protection, in lamports)
///
/// # Returns
/// A Solana instruction ready to be added to a transaction
pub fn build_pumpswap_sell_instruction(
    pool_state: &PumpSwapBondingCurveState,
    associated_user: &Pubkey,
    user_wallet: &Pubkey,
    amount_tokens: u64,
    min_sol_out: u64,
) -> Result<Instruction> {
    let program_id = Pubkey::from_str(PUMPSWAP_PROGRAM_ID)
        .map_err(|e| anyhow!("Invalid PumpSwap program ID: {}", e))?;

    // Build instruction data (same structure as buy, different discriminator)
    let mut instruction_data = Vec::with_capacity(24);
    instruction_data.extend_from_slice(&SELL_INSTRUCTION_DISCRIMINATOR);
    instruction_data.extend_from_slice(&amount_tokens.to_le_bytes());
    instruction_data.extend_from_slice(&min_sol_out.to_le_bytes());

    // Same account structure as buy instruction
    let token_program = spl_token::id();
    let event_authority =
        Pubkey::from_str("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1").unwrap_or(program_id);

    let accounts = vec![
        AccountMeta::new(pool_state.global, false),
        AccountMeta::new(pool_state.fee_recipient, false),
        AccountMeta::new_readonly(pool_state.token_mint, false),
        AccountMeta::new(pool_state.bonding_curve, false),
        AccountMeta::new(pool_state.associated_bonding_curve, false),
        AccountMeta::new(*associated_user, false),
        AccountMeta::new(*user_wallet, true), // Signer
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(program_id, false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data: instruction_data,
    })
}

/// Build a front-run swap instruction (buy before victim)
///
/// This buys tokens using SOL, driving up the price
pub fn build_pumpswap_frontrun_instruction(
    pool_state: &PumpSwapBondingCurveState,
    our_associated_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_sol: u64,
    min_tokens_out: u64,
) -> Result<Instruction> {
    build_pumpswap_buy_instruction(
        pool_state,
        our_associated_token,
        our_wallet,
        amount_sol,
        min_tokens_out,
    )
}

/// Build a back-run swap instruction (sell after victim)
///
/// This sells the tokens we bought in the front-run, capturing the price impact
pub fn build_pumpswap_backrun_instruction(
    pool_state: &PumpSwapBondingCurveState,
    our_associated_token: &Pubkey,
    our_wallet: &Pubkey,
    amount_tokens: u64,
    min_sol_out: u64,
) -> Result<Instruction> {
    build_pumpswap_sell_instruction(
        pool_state,
        our_associated_token,
        our_wallet,
        amount_tokens,
        min_sol_out,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pumpswap_state::PumpSwapBondingCurveState;

    #[test]
    fn test_build_buy_instruction() {
        let pool_state = PumpSwapBondingCurveState {
            bonding_curve: Pubkey::new_unique(),
            token_mint: Pubkey::new_unique(),
            associated_bonding_curve: Pubkey::new_unique(),
            global: Pubkey::new_unique(),
            fee_recipient: Pubkey::new_unique(),
        };

        let associated_user = Pubkey::new_unique();
        let user_wallet = Pubkey::new_unique();

        let result = build_pumpswap_buy_instruction(
            &pool_state,
            &associated_user,
            &user_wallet,
            1_000_000_000, // 1 SOL
            900_000,       // Min 0.9M tokens (10% slippage)
        );

        assert!(result.is_ok());
        let instruction = result.unwrap();

        // Verify instruction data format
        assert_eq!(&instruction.data[0..8], &BUY_INSTRUCTION_DISCRIMINATOR);
        assert_eq!(instruction.data.len(), 24); // 8 + 8 + 8 bytes

        // Verify accounts count (should be 12)
        assert_eq!(instruction.accounts.len(), 12);

        // Verify signer is user_wallet (account index 6)
        assert!(instruction.accounts[6].is_signer);
        assert_eq!(instruction.accounts[6].pubkey, user_wallet);

        // Verify pool accounts match pool_state
        assert_eq!(instruction.accounts[0].pubkey, pool_state.global);
        assert_eq!(instruction.accounts[1].pubkey, pool_state.fee_recipient);
        assert_eq!(instruction.accounts[2].pubkey, pool_state.token_mint);
        assert_eq!(instruction.accounts[3].pubkey, pool_state.bonding_curve);
        assert_eq!(
            instruction.accounts[4].pubkey,
            pool_state.associated_bonding_curve
        );
        assert_eq!(instruction.accounts[5].pubkey, associated_user);
    }

    #[test]
    fn test_build_sell_instruction() {
        let pool_state = PumpSwapBondingCurveState {
            bonding_curve: Pubkey::new_unique(),
            token_mint: Pubkey::new_unique(),
            associated_bonding_curve: Pubkey::new_unique(),
            global: Pubkey::new_unique(),
            fee_recipient: Pubkey::new_unique(),
        };

        let associated_user = Pubkey::new_unique();
        let user_wallet = Pubkey::new_unique();

        let result = build_pumpswap_sell_instruction(
            &pool_state,
            &associated_user,
            &user_wallet,
            1_000_000,   // 1M tokens
            900_000_000, // Min 0.9 SOL (10% slippage)
        );

        assert!(result.is_ok());
        let instruction = result.unwrap();

        // Verify instruction data format (different discriminator)
        assert_eq!(&instruction.data[0..8], &SELL_INSTRUCTION_DISCRIMINATOR);
        assert_eq!(instruction.data.len(), 24);

        // Verify accounts count (same as buy)
        assert_eq!(instruction.accounts.len(), 12);

        // Verify signer
        assert!(instruction.accounts[6].is_signer);
    }

    #[test]
    fn test_instruction_data_encoding() {
        let pool_state = PumpSwapBondingCurveState {
            bonding_curve: Pubkey::new_unique(),
            token_mint: Pubkey::new_unique(),
            associated_bonding_curve: Pubkey::new_unique(),
            global: Pubkey::new_unique(),
            fee_recipient: Pubkey::new_unique(),
        };

        let amount_in = 1_234_567_890u64;
        let min_amount_out = 987_654_321u64;

        let ix = build_pumpswap_buy_instruction(
            &pool_state,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            amount_in,
            min_amount_out,
        )
        .unwrap();

        // Verify discriminator
        assert_eq!(&ix.data[0..8], &BUY_INSTRUCTION_DISCRIMINATOR);

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
