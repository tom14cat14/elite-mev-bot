use anyhow::Result;
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Elite MEV Bot - Wallet Balance Checker");
    println!("═══════════════════════════════════════════");

    // Load environment variables
    dotenvy::dotenv().ok();

    // Get wallet configuration
    let wallet_private_key = std::env::var("WALLET_PRIVATE_KEY")
        .map_err(|_| anyhow::anyhow!("WALLET_PRIVATE_KEY not found in environment"))?;

    // Use standard RPC endpoint for balance queries (ShredStream is UDP-only for data feeds)
    let rpc_endpoint = "https://api.mainnet-beta.solana.com".to_string();

    // Create keypair from private key
    println!("🔑 Loading wallet from private key...");
    let decoded = bs58::decode(&wallet_private_key)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Failed to decode private key: {}", e))?;

    if decoded.len() != 64 {
        return Err(anyhow::anyhow!("Invalid private key length: expected 64 bytes, got {}", decoded.len()));
    }

    let keypair = Keypair::from_bytes(&decoded)
        .map_err(|e| anyhow::anyhow!("Failed to create keypair: {}", e))?;

    let wallet_pubkey = keypair.pubkey();
    println!("📍 Wallet Address: {}", wallet_pubkey);

    // This is the actual wallet derived from the private key
    println!("✅ WALLET LOADED: Using wallet derived from WALLET_PRIVATE_KEY");

    // Connect to RPC and check balance
    println!("\n🌐 Connecting to RPC: {}", rpc_endpoint);
    let rpc_client = RpcClient::new(rpc_endpoint);

    match rpc_client.get_balance(&wallet_pubkey) {
        Ok(balance_lamports) => {
            let balance_sol = balance_lamports as f64 / 1_000_000_000.0;
            println!("💰 SOL Balance: {:.9} SOL ({} lamports)", balance_sol, balance_lamports);

            // Check if we have sufficient capital
            let required_capital = 4.0; // From .env config
            if balance_sol >= required_capital {
                println!("✅ SUFFICIENT CAPITAL: {:.3} SOL >= {:.1} SOL required", balance_sol, required_capital);
            } else {
                println!("⚠️  INSUFFICIENT CAPITAL: {:.3} SOL < {:.1} SOL required", balance_sol, required_capital);
                println!("   Need to add: {:.3} SOL", required_capital - balance_sol);
            }
        }
        Err(e) => {
            println!("❌ Failed to fetch balance: {}", e);
            println!("   This might be due to RPC connection issues");
        }
    }

    // Check trading mode
    let enable_real_trading = std::env::var("ENABLE_REAL_TRADING")
        .unwrap_or_default()
        .parse::<bool>()
        .unwrap_or(false);

    let paper_trading = std::env::var("PAPER_TRADING")
        .unwrap_or_default()
        .parse::<bool>()
        .unwrap_or(true);

    println!("\n⚙️  Trading Configuration:");
    println!("   Real Trading: {}", if enable_real_trading { "✅ ENABLED" } else { "🔒 DISABLED" });
    println!("   Paper Trading: {}", if paper_trading { "📝 ENABLED" } else { "❌ DISABLED" });

    if !enable_real_trading && paper_trading {
        println!("🛡️  SAFE MODE: Bot will simulate trades without spending real SOL");
    } else if enable_real_trading && !paper_trading {
        println!("🚨 LIVE MODE: Bot will execute real trades with real SOL");
    } else {
        println!("⚠️  WARNING: Inconsistent trading mode configuration");
    }

    println!("\n═══════════════════════════════════════════");
    println!("✅ Wallet check complete!");

    Ok(())
}