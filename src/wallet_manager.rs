use anyhow::Result;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use tracing::{debug, info, warn};

/// Wallet management for trading operations
pub struct WalletManager {
    main_wallet: Keypair,
    jito_wallet: Option<Keypair>,
    paper_trading: bool,
}

impl WalletManager {
    /// Create new wallet manager from environment configuration
    pub fn from_env() -> Result<Self> {
        let wallet_key = std::env::var("WALLET_PRIVATE_KEY")
            .map_err(|_| anyhow::anyhow!("WALLET_PRIVATE_KEY not found in environment"))?;

        let paper_trading = std::env::var("PAPER_TRADING")
            .unwrap_or_default()
            .parse::<bool>()
            .unwrap_or(true);

        let main_wallet = Self::keypair_from_base58(&wallet_key)?;

        info!(
            "Wallet manager initialized - Main wallet: {}",
            main_wallet.pubkey()
        );
        info!("Paper trading mode: {}", paper_trading);

        Ok(Self {
            main_wallet,
            jito_wallet: None,
            paper_trading,
        })
    }

    /// Create new wallet manager with specific keypair
    pub fn new(main_wallet: Keypair, paper_trading: bool) -> Self {
        info!(
            "Wallet manager initialized - Main wallet: {}",
            main_wallet.pubkey()
        );

        Self {
            main_wallet,
            jito_wallet: None,
            paper_trading,
        }
    }

    /// Get main wallet public key
    pub fn get_main_pubkey(&self) -> Pubkey {
        self.main_wallet.pubkey()
    }

    /// Get main wallet public key as string
    pub fn get_main_pubkey_string(&self) -> String {
        self.main_wallet.pubkey().to_string()
    }

    /// Get main wallet keypair reference
    pub fn get_main_keypair(&self) -> &Keypair {
        &self.main_wallet
    }

    /// Sign transaction with main wallet
    pub fn sign_transaction(&self, transaction: &mut Transaction) -> Result<()> {
        if self.paper_trading {
            debug!("Paper trading mode: skipping transaction signing");
            return Ok(());
        }

        transaction.sign(&[&self.main_wallet], transaction.message.recent_blockhash);
        debug!("Transaction signed with main wallet");
        Ok(())
    }

    /// Set up separate Jito wallet if needed
    pub fn setup_jito_wallet(&mut self, jito_keypair: Option<Keypair>) {
        self.jito_wallet = jito_keypair;
        if let Some(ref jito_wallet) = self.jito_wallet {
            info!("Jito wallet configured: {}", jito_wallet.pubkey());
        }
    }

    /// Get Jito wallet or fall back to main wallet
    pub fn get_jito_wallet(&self) -> &Keypair {
        self.jito_wallet.as_ref().unwrap_or(&self.main_wallet)
    }

    /// Check if paper trading mode is enabled
    pub fn is_paper_trading(&self) -> bool {
        self.paper_trading
    }

    /// Set paper trading mode
    pub fn set_paper_trading(&mut self, enabled: bool) {
        self.paper_trading = enabled;
        info!("Paper trading mode: {}", enabled);
    }

    /// Create keypair from base58 private key string
    fn keypair_from_base58(private_key: &str) -> Result<Keypair> {
        let decoded = bs58::decode(private_key)
            .into_vec()
            .map_err(|e| anyhow::anyhow!("Failed to decode base58 private key: {}", e))?;

        if decoded.len() != 64 {
            return Err(anyhow::anyhow!(
                "Invalid private key length: expected 64 bytes, got {}",
                decoded.len()
            ));
        }

        Keypair::from_bytes(&decoded)
            .map_err(|e| anyhow::anyhow!("Failed to create keypair from bytes: {}", e))
    }

    /// Generate a new random keypair (for testing or additional wallets)
    pub fn generate_new_keypair() -> Keypair {
        Keypair::new()
    }

    /// Get wallet balance info (placeholder for future RPC integration)
    pub async fn get_balance_info(&self) -> Result<WalletBalanceInfo> {
        // This would integrate with Solana RPC in the future
        // For now, return placeholder data
        Ok(WalletBalanceInfo {
            sol_balance: 0.0,
            token_balances: vec![],
        })
    }
}

/// Wallet balance information
#[derive(Debug, Clone)]
pub struct WalletBalanceInfo {
    pub sol_balance: f64,
    pub token_balances: Vec<TokenBalance>,
}

/// Token balance for a specific mint
#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub mint: Pubkey,
    pub amount: u64,
    pub decimals: u8,
    pub ui_amount: f64,
}

impl Default for WalletManager {
    fn default() -> Self {
        // Generate a temporary keypair for testing
        let test_keypair = Keypair::new();
        warn!("Using default test wallet: {}", test_keypair.pubkey());

        Self {
            main_wallet: test_keypair,
            jito_wallet: None,
            paper_trading: true, // Always paper trading for default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_manager_creation() {
        let keypair = Keypair::new();
        let pubkey = keypair.pubkey();

        let wallet_manager = WalletManager::new(keypair, true);
        assert_eq!(wallet_manager.get_main_pubkey(), pubkey);
        assert!(wallet_manager.is_paper_trading());
    }

    #[test]
    fn test_keypair_from_base58() {
        let test_keypair = Keypair::new();
        let private_key_bytes = test_keypair.to_bytes();
        let private_key_base58 = bs58::encode(&private_key_bytes).into_string();

        let decoded_keypair = WalletManager::keypair_from_base58(&private_key_base58).unwrap();
        assert_eq!(test_keypair.pubkey(), decoded_keypair.pubkey());
    }
}
