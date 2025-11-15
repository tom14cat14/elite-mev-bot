//! Dynamic Slippage Calculator for PumpFun Trades
//! Queries recent transactions to calculate actual slippage instead of using hardcoded values
//! GROK CYCLE 1 FIX #3

use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;

/// Calculate dynamic slippage based on recent PumpFun transactions
/// Returns slippage percentage (e.g., 0.025 for 2.5%)
/// Minimum return value is 0.025 (2.5%) for safety
pub fn calculate_dynamic_slippage(
    rpc_client: &RpcClient,
    token_mint: &Pubkey,
) -> Result<f64> {
    let program_id = Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")?;
    
    // Get last 10 transactions for PumpFun program
    let signatures = match rpc_client.get_signatures_for_address(&program_id) {
        Ok(sigs) => sigs,
        Err(e) => {
            tracing::warn!("⚠️ Failed to fetch signatures for slippage calculation: {}", e);
            return Ok(0.025); // Return default on error
        }
    };
    
    let recent_signatures: Vec<_> = signatures.into_iter().take(10).collect();
    let mut swap_data = Vec::new();

    for sig_info in recent_signatures {
        let tx = match rpc_client.get_transaction(
            &sig_info.signature,
            UiTransactionEncoding::JsonParsed
        ) {
            Ok(t) => t,
            Err(_) => continue, // Skip failed fetches
        };
        
        if let Some(meta) = tx.transaction.meta {
            // Get SOL balance changes (simplified - just use first two accounts)
            let pre_sol = meta.pre_balances.get(1).copied().unwrap_or(0);
            let post_sol = meta.post_balances.get(1).copied().unwrap_or(0);
            let sol_change = post_sol as i64 - pre_sol as i64;

            // Get token balance change for the token_mint
            // Simplified: just check if token balance increased while SOL decreased
            let mut token_change = 0i64;

            // Check pre_token_balances
            let pre_token_balances = match &meta.pre_token_balances {
                solana_transaction_status::option_serializer::OptionSerializer::Some(balances) => balances,
                _ => continue,
            };

            let post_token_balances = match &meta.post_token_balances {
                solana_transaction_status::option_serializer::OptionSerializer::Some(balances) => balances,
                _ => continue,
            };

            for pre_token in pre_token_balances {
                if pre_token.mint == token_mint.to_string() {
                    let pre_amount = pre_token.ui_token_amount.ui_amount.unwrap_or(0.0);
                    if let Some(post_token) = post_token_balances.iter()
                        .find(|pt| pt.mint == token_mint.to_string()) {
                        let post_amount = post_token.ui_token_amount.ui_amount.unwrap_or(0.0);
                        token_change = ((post_amount - pre_amount) * 1_000_000.0) as i64; // Scale up for precision
                    }
                    break;
                }
            }

            // Identify buy: SOL decreases, token increases
            if sol_change < 0 && token_change > 0 {
                let amount_in = (-sol_change) as f64 / 1_000_000_000.0; // Lamports to SOL
                let amount_out = token_change as f64 / 1_000_000.0; // Scale back down
                if amount_in > 0.0 && amount_out > 0.0 {
                    let price = amount_out / amount_in;
                    swap_data.push((sig_info.slot, price));
                }
            }
        }
    }

    if swap_data.is_empty() {
        tracing::debug!("No swap data found for slippage calculation, using default 2.5%");
        return Ok(0.025); // Default if no data
    }

    // Sort by slot (chronological order)
    swap_data.sort_by_key(|&(slot, _)| slot);

    // Calculate average price change percentage (volatility indicator)
    let mut total_slippage = 0.0;
    let mut count = 0;
    for i in 0..swap_data.len().saturating_sub(1) {
        let (_, price1) = swap_data[i];
        let (_, price2) = swap_data[i + 1];
        if price1 > 0.0 {
            let price_change = ((price2 - price1).abs() / price1);
            total_slippage += price_change;
            count += 1;
        }
    }

    let avg_slippage = if count > 0 { 
        total_slippage / count as f64 
    } else { 
        0.025 
    };
    
    // Return max(calculated, 2.5%) for safety
    let final_slippage = avg_slippage.max(0.025);
    
    tracing::debug!("📊 Dynamic slippage calculated: {:.2}% (from {} swaps)", 
                   final_slippage * 100.0, swap_data.len());
    
    Ok(final_slippage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slippage_minimum() {
        // Even if calculated slippage is 0, should return 2.5% minimum
        // This is a placeholder test - real test would mock RPC calls
    }
}
