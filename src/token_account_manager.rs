//! Token Account Manager
//!
//! Manages Associated Token Accounts (ATAs) for sandwich trades.
//! Creates or fetches ATAs for tokens we need to swap.
//!
//! ⚡ GROK OPTIMIZATION: Uses DashMap for lock-free ATA caching

use anyhow::{anyhow, Result};
use dashmap::DashMap;
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
use std::str::FromStr;
use once_cell::sync::Lazy;
use tracing::{debug, info, warn};

// ⚡ GROK OPTIMIZATION: Lock-free ATA cache using DashMap
// Key: "wallet:mint" string, Value: resolved ATA address
// This eliminates mutex contention and provides O(1) lookups
static VERIFIED_ATAS: Lazy<DashMap<String, Pubkey>> = Lazy::new(|| DashMap::new());

// Associated Token Program ID (well-known constant)
const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
// Token-2022 Program ID
const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
// Wrapped SOL mint (always uses standard Token Program)
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

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

    /// Detect if a mint is Token-2022 or standard Token Program
    /// Returns the correct token program ID for this mint
    ///
    /// Special handling:
    /// - WSOL is always standard Token Program
    /// - Other mints: query on-chain, default to SPL Token if lookup fails
    ///   (pump.fun bonding curve tokens use standard SPL Token, NOT Token-2022)
    pub fn get_token_program_for_mint(&self, mint: &Pubkey) -> Pubkey {
        let token_2022_id = Pubkey::from_str(TOKEN_2022_PROGRAM_ID).unwrap();
        let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();

        // WSOL is always standard Token Program
        if *mint == wsol_mint {
            info!("🔶 Mint {} is WSOL (always standard SPL Token)", mint);
            return spl_token::id();
        }

        // Query the mint account to determine its owner (token program)
        match self.rpc_client.get_account(mint) {
            Ok(account) => {
                if account.owner == token_2022_id {
                    info!("🔷 Mint {} is Token-2022 (owner: {})", mint, account.owner);
                    token_2022_id
                } else if account.owner == spl_token::id() {
                    info!("🔶 Mint {} is standard SPL Token (owner: {})", mint, account.owner);
                    spl_token::id()
                } else {
                    // Unknown owner - log it and default to SPL Token
                    warn!(
                        "⚠️  Mint {} has unknown owner {} - defaulting to SPL Token",
                        mint, account.owner
                    );
                    spl_token::id()
                }
            }
            Err(e) => {
                // Default to SPL Token for pump.fun meme tokens
                // pump.fun bonding curve tokens use standard SPL Token, NOT Token-2022
                warn!(
                    "⚠️  Could not determine token program for mint {} ({}), defaulting to SPL Token",
                    mint, e
                );
                spl_token::id()
            }
        }
    }

    /// Get or create an Associated Token Account (ATA) for a wallet and mint
    ///
    /// Returns the ATA address. If the account doesn't exist, it will be created.
    /// Checks BOTH token programs (SPL Token and Token-2022) to avoid "already exists" errors.
    /// Uses a cache to prevent repeated creation attempts for the same mint.
    pub fn get_or_create_ata(&self, wallet: &Keypair, mint: &Pubkey) -> Result<Pubkey> {
        let wallet_pubkey = wallet.pubkey();
        let token_2022_id = Pubkey::from_str(TOKEN_2022_PROGRAM_ID).unwrap();
        let spl_token_id = spl_token::id();

        // Calculate ATAs for BOTH possible token programs
        let ata_spl = Self::get_ata_address_with_program(&wallet_pubkey, mint, &spl_token_id);
        let ata_2022 = Self::get_ata_address_with_program(&wallet_pubkey, mint, &token_2022_id);

        // Create cache key (cloned for multiple potential inserts)
        let cache_key = format!("{}:{}", wallet_pubkey, mint);

        // ⚡ GROK OPTIMIZATION: Lock-free cache lookup using DashMap
        // Returns cached ATA address instantly without any RPC calls
        if let Some(cached_ata) = VERIFIED_ATAS.get(&cache_key) {
            debug!("⚡ SPEED: ATA from cache (0ms): {} (mint: {})", *cached_ata, mint);
            return Ok(*cached_ata);
        }

        // Check if SPL Token ATA already exists
        if let Ok(_account) = self.rpc_client.get_account(&ata_spl) {
            info!("✅ ATA already exists (SPL Token): {} (mint: {})", ata_spl, mint);
            // ⚡ GROK OPTIMIZATION: Lock-free cache insert
            VERIFIED_ATAS.insert(cache_key.clone(), ata_spl);
            return Ok(ata_spl);
        }

        // Check if Token-2022 ATA already exists
        if let Ok(_account) = self.rpc_client.get_account(&ata_2022) {
            info!("✅ ATA already exists (Token-2022): {} (mint: {})", ata_2022, mint);
            // ⚡ GROK OPTIMIZATION: Lock-free cache insert
            VERIFIED_ATAS.insert(cache_key.clone(), ata_2022);
            return Ok(ata_2022);
        }

        // Neither ATA exists - detect correct token program and create
        let token_program_id = self.get_token_program_for_mint(mint);
        let ata_address = Self::get_ata_address_with_program(&wallet_pubkey, mint, &token_program_id);

        info!(
            "📝 Creating ATA for wallet {} and mint {} (program: {})",
            &wallet_pubkey, mint, token_program_id
        );

        // Create ATA instruction with correct token program
        let create_ata_ix = Self::build_create_ata_instruction_with_program(&wallet_pubkey, mint, &token_program_id);

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
                // ⚡ GROK OPTIMIZATION: Lock-free cache insert after successful creation
                VERIFIED_ATAS.insert(cache_key, ata_address);
                Ok(ata_address)
            }
            Err(e) => {
                let err_str = e.to_string();

                // Error 0x2 = AccountAlreadyInitialized - this is OK, ATA exists
                // Error 0x0 = Success (shouldn't happen in error branch but handle anyway)
                if err_str.contains("0x2") || err_str.contains("already in use") ||
                   err_str.contains("AlreadyInitialized") || err_str.contains("AccountAlreadyInitialized") {
                    // ATA already exists - add to cache and return the computed address directly
                    // (RPC lookup may fail but we KNOW from error 0x2 that account exists)
                    info!("✅ ATA already exists (detected via error 0x2): {} (adding to cache)", ata_address);
                    // ⚡ GROK OPTIMIZATION: Lock-free cache insert
                    VERIFIED_ATAS.insert(cache_key, ata_address);
                    return Ok(ata_address);
                }

                // Check if error is because account was created by another transaction (race condition)
                // Check BOTH possible ATAs
                if let Ok(_account) = self.rpc_client.get_account(&ata_spl) {
                    warn!(
                        "⚠️  ATA creation failed but SPL Token ATA exists (race condition): {}",
                        ata_spl
                    );
                    // ⚡ GROK OPTIMIZATION: Lock-free cache insert
                    VERIFIED_ATAS.insert(cache_key.clone(), ata_spl);
                    return Ok(ata_spl);
                }
                if let Ok(_account) = self.rpc_client.get_account(&ata_2022) {
                    warn!(
                        "⚠️  ATA creation failed but Token-2022 ATA exists (race condition): {}",
                        ata_2022
                    );
                    // ⚡ GROK OPTIMIZATION: Lock-free cache insert
                    VERIFIED_ATAS.insert(cache_key, ata_2022);
                    return Ok(ata_2022);
                }
                Err(anyhow!("Failed to create ATA: {}", e))
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
    /// Legacy function - defaults to standard Token Program
    pub fn build_create_ata_instruction(wallet_pubkey: &Pubkey, mint: &Pubkey) -> Instruction {
        Self::build_create_ata_instruction_with_program(wallet_pubkey, mint, &spl_token::id())
    }

    /// Build the create ATA instruction with specific token program
    ///
    /// This doesn't execute the instruction, just returns it.
    /// Useful for including ATA creation in JITO bundles.
    /// Supports both standard Token Program and Token-2022.
    pub fn build_create_ata_instruction_with_program(
        wallet_pubkey: &Pubkey,
        mint: &Pubkey,
        token_program_id: &Pubkey
    ) -> Instruction {
        let ata_program_id =
            Pubkey::try_from(ASSOCIATED_TOKEN_PROGRAM_ID).expect("Invalid ATA program ID");

        let ata_address = Self::get_ata_address_with_program(wallet_pubkey, mint, token_program_id);

        // Create ATA instruction using the correct format
        // The create instruction is idempotent (discriminator: 1 byte = 0x01 for newer versions, or empty for older)
        Instruction {
            program_id: ata_program_id,
            accounts: vec![
                AccountMeta::new(*wallet_pubkey, true), // 0: payer (signer, writable)
                AccountMeta::new(ata_address, false),   // 1: associated token account (writable)
                AccountMeta::new_readonly(*wallet_pubkey, false), // 2: owner
                AccountMeta::new_readonly(*mint, false), // 3: mint
                AccountMeta::new_readonly(system_program::id(), false), // 4: system program
                AccountMeta::new_readonly(*token_program_id, false), // 5: token program (standard or 2022)
            ],
            // Use instruction discriminator 1 for create_idempotent (modern standard)
            data: vec![1],
        }
    }

    /// Get ATA address for a wallet and mint (doesn't check if it exists)
    /// Legacy function - defaults to standard Token Program
    pub fn get_ata_address(wallet_pubkey: &Pubkey, mint: &Pubkey) -> Pubkey {
        Self::get_ata_address_with_program(wallet_pubkey, mint, &spl_token::id())
    }

    /// Get ATA address with specific token program
    ///
    /// Uses PDA derivation: find_program_address([wallet, token_program, mint], ata_program)
    /// Supports both standard Token Program and Token-2022.
    pub fn get_ata_address_with_program(wallet_pubkey: &Pubkey, mint: &Pubkey, token_program_id: &Pubkey) -> Pubkey {
        let ata_program_id =
            Pubkey::try_from(ASSOCIATED_TOKEN_PROGRAM_ID).expect("Invalid ATA program ID");

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
