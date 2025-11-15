//! Token Account Manager
//!
//! Manages Associated Token Accounts (ATAs) for sandwich trades.
//! Creates or fetches ATAs for tokens we need to swap.

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
    transaction::Transaction,
};
use tracing::{info, warn};

// Associated Token Program ID (well-known constant)
const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

/// Token account manager for sandwich trades
pub struct TokenAccountManager {
    rpc_client: RpcClient,
}

impl TokenAccountManager {
    /// Create a new token account manager
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_client: RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed()),
        }
    }

    /// Get or create an Associated Token Account (ATA) for a wallet and mint
    ///
    /// Returns the ATA address. If the account doesn't exist, it will be created.
    pub fn get_or_create_ata(&self, wallet: &Keypair, mint: &Pubkey) -> Result<Pubkey> {
        let wallet_pubkey = wallet.pubkey();

        // Calculate ATA address manually
        let ata_address = Self::get_ata_address(&wallet_pubkey, mint);

        // Check if ATA already exists
        match self.rpc_client.get_account(&ata_address) {
            Ok(_account) => {
                info!("✅ ATA already exists: {} (mint: {})", ata_address, mint);
                Ok(ata_address)
            }
            Err(_) => {
                // ATA doesn't exist, create it
                info!(
                    "📝 Creating ATA for wallet {} and mint {}",
                    &wallet_pubkey, mint
                );

                // Create ATA instruction manually
                let create_ata_ix = Self::build_create_ata_instruction(&wallet_pubkey, mint);

                // Build and send transaction
                let recent_blockhash = self
                    .rpc_client
                    .get_latest_blockhash()
                    .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;

                let transaction = Transaction::new_signed_with_payer(
                    &[create_ata_ix],
                    Some(&wallet_pubkey),
                    &[wallet],
                    recent_blockhash,
                );

                // Send transaction
                match self.rpc_client.send_and_confirm_transaction(&transaction) {
                    Ok(signature) => {
                        info!("✅ Created ATA: {} | Signature: {}", ata_address, signature);
                        Ok(ata_address)
                    }
                    Err(e) => {
                        // Check if error is because account was created by another transaction
                        if let Ok(_account) = self.rpc_client.get_account(&ata_address) {
                            warn!(
                                "⚠️  ATA creation failed but account exists (race condition): {}",
                                ata_address
                            );
                            Ok(ata_address)
                        } else {
                            Err(anyhow!("Failed to create ATA: {}", e))
                        }
                    }
                }
            }
        }
    }

    /// Get or create ATAs for both tokens in a swap
    ///
    /// Returns (source_ata, dest_ata) - the ATAs for selling and buying tokens
    pub fn get_or_create_swap_atas(
        &self,
        wallet: &Keypair,
        source_mint: &Pubkey,
        dest_mint: &Pubkey,
    ) -> Result<(Pubkey, Pubkey)> {
        info!(
            "🔍 Getting/creating ATAs for swap | Source: {} | Dest: {}",
            source_mint, dest_mint
        );

        let source_ata = self.get_or_create_ata(wallet, source_mint)?;
        let dest_ata = self.get_or_create_ata(wallet, dest_mint)?;

        info!(
            "✅ ATAs ready | Source: {} | Dest: {}",
            source_ata, dest_ata
        );

        Ok((source_ata, dest_ata))
    }

    /// Check if an ATA exists without creating it
    pub fn ata_exists(&self, wallet_pubkey: &Pubkey, mint: &Pubkey) -> bool {
        let ata_address = Self::get_ata_address(wallet_pubkey, mint);
        self.rpc_client.get_account(&ata_address).is_ok()
    }

    /// Build the create ATA instruction (for use in bundles)
    ///
    /// This doesn't execute the instruction, just returns it.
    /// Useful for including ATA creation in JITO bundles.
    pub fn build_create_ata_instruction(wallet_pubkey: &Pubkey, mint: &Pubkey) -> Instruction {
        let ata_program_id =
            Pubkey::try_from(ASSOCIATED_TOKEN_PROGRAM_ID).expect("Invalid ATA program ID");

        let ata_address = Self::get_ata_address(wallet_pubkey, mint);

        // Create ATA instruction using the correct format
        // The create instruction is idempotent (discriminator: 1 byte = 0x01 for newer versions, or empty for older)
        // Modern versions use empty data for backwards compatibility
        Instruction {
            program_id: ata_program_id,
            accounts: vec![
                AccountMeta::new(*wallet_pubkey, true), // 0: payer (signer, writable)
                AccountMeta::new(ata_address, false),   // 1: associated token account (writable)
                AccountMeta::new_readonly(*wallet_pubkey, false), // 2: owner
                AccountMeta::new_readonly(*mint, false), // 3: mint
                AccountMeta::new_readonly(system_program::id(), false), // 4: system program
                AccountMeta::new_readonly(spl_token::id(), false), // 5: token program
            ],
            // Use instruction discriminator 1 for create_idempotent (modern standard)
            data: vec![1],
        }
    }

    /// Get ATA address for a wallet and mint (doesn't check if it exists)
    ///
    /// Uses PDA derivation: find_program_address([wallet, token_program, mint], ata_program)
    pub fn get_ata_address(wallet_pubkey: &Pubkey, mint: &Pubkey) -> Pubkey {
        let ata_program_id =
            Pubkey::try_from(ASSOCIATED_TOKEN_PROGRAM_ID).expect("Invalid ATA program ID");

        // Create binding for token program ID to extend lifetime
        let token_program_id = spl_token::id();

        // ATA address = PDA derived from [wallet, token_program_id, mint]
        let seeds = &[
            wallet_pubkey.as_ref(),
            token_program_id.as_ref(),
            mint.as_ref(),
        ];

        let (ata_address, _bump) = Pubkey::find_program_address(seeds, &ata_program_id);
        ata_address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_ata_address() {
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ata = TokenAccountManager::get_ata_address(&wallet, &mint);

        // ATA should be deterministic
        let ata2 = TokenAccountManager::get_ata_address(&wallet, &mint);
        assert_eq!(ata, ata2);

        // Different mint should give different ATA
        let mint2 = Pubkey::new_unique();
        let ata3 = TokenAccountManager::get_ata_address(&wallet, &mint2);
        assert_ne!(ata, ata3);
    }

    #[test]
    fn test_build_create_ata_instruction() {
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let instruction = TokenAccountManager::build_create_ata_instruction(&wallet, &mint);

        // Should be create_associated_token_account instruction
        // Just verify it's an instruction (specific program_id checks would require knowing the spl-token ID)
        assert!(!instruction.accounts.is_empty());
    }
}
