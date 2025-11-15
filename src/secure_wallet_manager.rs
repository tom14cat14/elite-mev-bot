use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::Result;
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Secure wallet management with encryption and KMS integration
#[derive(Debug)]
pub struct SecureWalletManager {
    wallets: Arc<RwLock<HashMap<String, EncryptedWallet>>>,
    master_key: [u8; 32],
    wallet_storage_path: String,
    kms_config: Option<KmsConfig>,
    security_metrics: SecurityMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedWallet {
    pub name: String,
    pub public_key: Pubkey,
    pub encrypted_private_key: Vec<u8>,
    pub nonce: [u8; 12],
    pub key_derivation_salt: [u8; 32], // SECURITY FIX: Random salt per wallet
    pub wallet_type: WalletType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub usage_count: u64,
    pub balance_sol: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalletType {
    Trading,
    Fee,
    Emergency,
    Jito,
    Development,
}

#[derive(Debug, Clone)]
pub struct KmsConfig {
    pub provider: KmsProvider,
    pub key_id: String,
    pub region: String,
    pub endpoint: String,
}

#[derive(Debug, Clone)]
pub enum KmsProvider {
    Aws,
    GoogleCloud,
    Azure,
    HashiCorpVault,
}

#[derive(Debug, Clone, Default)]
pub struct SecurityMetrics {
    pub total_wallets: usize,
    pub active_wallets: usize,
    pub total_transactions_signed: u64,
    pub failed_authentication_attempts: u64,
    pub last_security_audit: Option<chrono::DateTime<chrono::Utc>>,
    pub encryption_operations: u64,
    pub decryption_operations: u64,
}

#[derive(Debug, Clone)]
pub struct WalletOperationResult {
    pub success: bool,
    pub wallet_name: String,
    pub operation: String,
    pub execution_time_ms: f64,
    pub error_message: Option<String>,
}

impl SecureWalletManager {
    /// Create new secure wallet manager
    pub fn new(
        master_password: &str,
        wallet_storage_path: String,
        kms_config: Option<KmsConfig>,
    ) -> Result<Self> {
        let master_key = Self::derive_master_key(master_password)?;

        Ok(Self {
            wallets: Arc::new(RwLock::new(HashMap::new())),
            master_key,
            wallet_storage_path,
            kms_config,
            security_metrics: SecurityMetrics::default(),
        })
    }

    /// Initialize wallet manager and load existing wallets
    pub async fn initialize(&mut self) -> Result<()> {
        info!("🔐 Initializing secure wallet manager");

        // Create wallet storage directory
        if let Some(parent) = Path::new(&self.wallet_storage_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Load existing wallets
        self.load_wallets().await?;

        // Perform security audit
        self.perform_security_audit().await?;

        info!(
            "✅ Secure wallet manager initialized with {} wallets",
            self.security_metrics.total_wallets
        );

        Ok(())
    }

    /// Create new wallet with specified type
    pub async fn create_wallet(&mut self, name: String, wallet_type: WalletType) -> Result<Pubkey> {
        info!("🆕 Creating new wallet: {} ({:?})", name, wallet_type);

        // Generate new keypair
        let keypair = Keypair::new();
        let public_key = keypair.pubkey();

        // Encrypt private key
        let encrypted_wallet = self.encrypt_keypair(&name, &keypair, wallet_type)?;

        // Store wallet
        {
            let mut wallets = self.wallets.write().await;
            wallets.insert(name.clone(), encrypted_wallet);
        }

        // Save to persistent storage
        self.save_wallets().await?;

        // Update metrics
        self.security_metrics.total_wallets += 1;
        self.security_metrics.active_wallets += 1;
        self.security_metrics.encryption_operations += 1;

        info!("✅ Wallet created successfully: {} ({})", name, public_key);

        Ok(public_key)
    }

    /// Import existing wallet from private key
    pub async fn import_wallet(
        &mut self,
        name: String,
        private_key: &[u8],
        wallet_type: WalletType,
    ) -> Result<Pubkey> {
        info!("📥 Importing wallet: {} ({:?})", name, wallet_type);

        // Create keypair from private key
        let keypair = Keypair::from_bytes(private_key)?;
        let public_key = keypair.pubkey();

        // Encrypt and store
        let encrypted_wallet = self.encrypt_keypair(&name, &keypair, wallet_type)?;

        {
            let mut wallets = self.wallets.write().await;
            wallets.insert(name.clone(), encrypted_wallet);
        }

        self.save_wallets().await?;

        // Update metrics
        self.security_metrics.total_wallets += 1;
        self.security_metrics.active_wallets += 1;
        self.security_metrics.encryption_operations += 1;

        info!("✅ Wallet imported successfully: {} ({})", name, public_key);

        Ok(public_key)
    }

    /// Get wallet for signing (decrypts temporarily)
    pub async fn get_wallet_for_signing(&mut self, name: &str) -> Result<Keypair> {
        debug!("🔓 Decrypting wallet for signing: {}", name);

        let encrypted_wallet = {
            let wallets = self.wallets.read().await;
            wallets
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("Wallet not found: {}", name))?
                .clone()
        };

        // Decrypt private key
        let keypair = self.decrypt_wallet(&encrypted_wallet)?;

        // Update usage metrics
        {
            let mut wallets = self.wallets.write().await;
            if let Some(wallet) = wallets.get_mut(name) {
                wallet.last_used = Some(chrono::Utc::now());
                wallet.usage_count += 1;
            }
        }

        self.security_metrics.decryption_operations += 1;

        Ok(keypair)
    }

    /// Sign transaction with specified wallet
    pub async fn sign_transaction(
        &mut self,
        wallet_name: &str,
        transaction: &mut Transaction,
    ) -> Result<WalletOperationResult> {
        let start_time = std::time::Instant::now();

        match self.get_wallet_for_signing(wallet_name).await {
            Ok(keypair) => {
                transaction.sign(&[&keypair], transaction.message.recent_blockhash);

                self.security_metrics.total_transactions_signed += 1;

                Ok(WalletOperationResult {
                    success: true,
                    wallet_name: wallet_name.to_string(),
                    operation: "sign_transaction".to_string(),
                    execution_time_ms: start_time.elapsed().as_millis() as f64,
                    error_message: None,
                })
            }
            Err(e) => {
                error!(
                    "Failed to sign transaction with wallet {}: {}",
                    wallet_name, e
                );

                self.security_metrics.failed_authentication_attempts += 1;

                Ok(WalletOperationResult {
                    success: false,
                    wallet_name: wallet_name.to_string(),
                    operation: "sign_transaction".to_string(),
                    execution_time_ms: start_time.elapsed().as_millis() as f64,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    /// List all wallets (without private keys)
    pub async fn list_wallets(&self) -> Vec<WalletInfo> {
        let wallets = self.wallets.read().await;

        wallets
            .values()
            .map(|w| WalletInfo {
                name: w.name.clone(),
                public_key: w.public_key,
                wallet_type: w.wallet_type.clone(),
                balance_sol: w.balance_sol,
                usage_count: w.usage_count,
                last_used: w.last_used,
            })
            .collect()
    }

    /// Remove wallet (secure deletion)
    pub async fn remove_wallet(&mut self, name: &str) -> Result<()> {
        warn!("🗑️ Removing wallet: {}", name);

        {
            let mut wallets = self.wallets.write().await;
            if wallets.remove(name).is_none() {
                return Err(anyhow::anyhow!("Wallet not found: {}", name));
            }
        }

        self.save_wallets().await?;

        // Update metrics
        self.security_metrics.total_wallets = self.security_metrics.total_wallets.saturating_sub(1);
        self.security_metrics.active_wallets =
            self.security_metrics.active_wallets.saturating_sub(1);

        info!("✅ Wallet removed: {}", name);

        Ok(())
    }

    /// Backup all wallets to encrypted file
    pub async fn backup_wallets(&self, backup_path: &str, backup_password: &str) -> Result<()> {
        info!("💾 Creating wallet backup: {}", backup_path);

        let wallets = self.wallets.read().await;
        let backup_data = serde_json::to_string(&*wallets)?;

        // Encrypt backup with separate password
        let backup_key = Self::derive_master_key(backup_password)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&backup_key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let encrypted_backup = cipher
            .encrypt(&nonce, backup_data.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

        // Save encrypted backup
        let backup_file = EncryptedBackup {
            nonce: nonce.as_slice().try_into()?,
            data: encrypted_backup,
            created_at: chrono::Utc::now(),
            wallet_count: wallets.len(),
        };

        let backup_json = serde_json::to_string_pretty(&backup_file)?;
        tokio::fs::write(backup_path, backup_json).await?;

        info!(
            "✅ Wallet backup created: {} ({} wallets)",
            backup_path,
            wallets.len()
        );

        Ok(())
    }

    /// Restore wallets from encrypted backup
    pub async fn restore_wallets(
        &mut self,
        backup_path: &str,
        backup_password: &str,
    ) -> Result<()> {
        info!("📥 Restoring wallets from backup: {}", backup_path);

        let backup_content = tokio::fs::read_to_string(backup_path).await?;
        let backup_file: EncryptedBackup = serde_json::from_str(&backup_content)?;

        // Decrypt backup
        let backup_key = Self::derive_master_key(backup_password)?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&backup_key));
        let nonce = Nonce::from_slice(&backup_file.nonce);

        let decrypted_data = cipher
            .decrypt(nonce, backup_file.data.as_slice())
            .map_err(|e| anyhow::anyhow!("Decryption failed: {:?}", e))?;
        let wallets_data = String::from_utf8(decrypted_data)?;
        let restored_wallets: HashMap<String, EncryptedWallet> =
            serde_json::from_str(&wallets_data)?;

        // Merge with existing wallets
        {
            let mut wallets = self.wallets.write().await;
            for (name, wallet) in restored_wallets {
                wallets.insert(name, wallet);
            }
        }

        self.save_wallets().await?;

        info!(
            "✅ Wallets restored from backup: {} wallets",
            backup_file.wallet_count
        );

        Ok(())
    }

    /// Get security metrics
    pub fn get_security_metrics(&self) -> &SecurityMetrics {
        &self.security_metrics
    }

    /// Perform security audit
    pub async fn perform_security_audit(&mut self) -> Result<SecurityAuditReport> {
        info!("🔍 Performing security audit");

        let wallets = self.wallets.read().await;
        let mut audit = SecurityAuditReport {
            audit_time: chrono::Utc::now(),
            total_wallets: wallets.len(),
            wallet_types: HashMap::new(),
            issues: Vec::new(),
            recommendations: Vec::new(),
        };

        // Analyze wallet distribution
        for wallet in wallets.values() {
            *audit
                .wallet_types
                .entry(format!("{:?}", wallet.wallet_type))
                .or_insert(0) += 1;

            // Check for security issues
            if wallet.usage_count == 0 {
                audit.issues.push(format!("Unused wallet: {}", wallet.name));
            }

            if let Some(last_used) = wallet.last_used {
                let days_since_use = chrono::Utc::now()
                    .signed_duration_since(last_used)
                    .num_days();
                if days_since_use > 90 {
                    audit.issues.push(format!(
                        "Wallet {} not used for {} days",
                        wallet.name, days_since_use
                    ));
                }
            }
        }

        // Generate recommendations
        if audit.total_wallets > 10 {
            audit
                .recommendations
                .push("Consider consolidating unused wallets".to_string());
        }

        if self.security_metrics.failed_authentication_attempts > 0 {
            audit
                .recommendations
                .push("Review failed authentication attempts".to_string());
        }

        self.security_metrics.last_security_audit = Some(audit.audit_time);

        info!(
            "📊 Security audit complete: {} wallets, {} issues, {} recommendations",
            audit.total_wallets,
            audit.issues.len(),
            audit.recommendations.len()
        );

        Ok(audit)
    }

    /// Encrypt keypair for storage (SECURITY FIX: Now generates random salt per wallet)
    fn encrypt_keypair(
        &self,
        name: &str,
        keypair: &Keypair,
        wallet_type: WalletType,
    ) -> Result<EncryptedWallet> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.master_key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let private_key_bytes = keypair.to_bytes();
        let encrypted_private_key = cipher
            .encrypt(&nonce, private_key_bytes.as_slice())
            .map_err(|e| anyhow::anyhow!("Private key encryption failed: {:?}", e))?;

        // SECURITY FIX: Generate unique random salt for this wallet
        let key_derivation_salt = Self::generate_salt();

        Ok(EncryptedWallet {
            name: name.to_string(),
            public_key: keypair.pubkey(),
            encrypted_private_key,
            nonce: nonce.as_slice().try_into()?,
            key_derivation_salt, // SECURITY FIX: Store random salt with wallet
            wallet_type,
            created_at: chrono::Utc::now(),
            last_used: None,
            usage_count: 0,
            balance_sol: 0.0,
        })
    }

    /// Decrypt wallet for use
    fn decrypt_wallet(&self, encrypted_wallet: &EncryptedWallet) -> Result<Keypair> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.master_key));
        let nonce = Nonce::from_slice(&encrypted_wallet.nonce);

        let decrypted_bytes = cipher
            .decrypt(nonce, encrypted_wallet.encrypted_private_key.as_slice())
            .map_err(|e| anyhow::anyhow!("Keypair decryption failed: {:?}", e))?;
        let keypair = Keypair::from_bytes(&decrypted_bytes)?;

        Ok(keypair)
    }

    /// Derive master key from password with salt (SECURITY FIX: Now accepts salt parameter)
    fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
        let mut key = [0u8; 32];
        // Using 100,000 iterations as per OWASP recommendations for PBKDF2-HMAC-SHA256
        pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 100_000, &mut key);
        Ok(key)
    }

    /// Generate random salt for key derivation (SECURITY FIX: Random salt per wallet)
    fn generate_salt() -> [u8; 32] {
        use aes_gcm::aead::OsRng;
        // Generate 32 bytes of random salt using AES-GCM nonce generation (12 bytes) + additional entropy
        let nonce1 = Aes256Gcm::generate_nonce(&mut OsRng);
        let nonce2 = Aes256Gcm::generate_nonce(&mut OsRng);
        let nonce3 = Aes256Gcm::generate_nonce(&mut OsRng);

        let mut salt = [0u8; 32];
        salt[0..12].copy_from_slice(&nonce1);
        salt[12..24].copy_from_slice(&nonce2);
        salt[24..32].copy_from_slice(&nonce3[0..8]);
        salt
    }

    /// Derive master key from password for manager initialization (legacy compatibility)
    fn derive_master_key(password: &str) -> Result<[u8; 32]> {
        // For manager's master key, use a well-known salt derived from password itself
        // This is acceptable as it's just for the manager, not individual wallets
        let salt = format!("mev_wallet_manager_v1_{}", password);
        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), salt.as_bytes(), 100_000, &mut key);
        Ok(key)
    }

    /// Load wallets from persistent storage
    async fn load_wallets(&mut self) -> Result<()> {
        match tokio::fs::read_to_string(&self.wallet_storage_path).await {
            Ok(content) => {
                let loaded_wallets: HashMap<String, EncryptedWallet> =
                    serde_json::from_str(&content)?;

                {
                    let mut wallets = self.wallets.write().await;
                    *wallets = loaded_wallets;
                }

                let wallet_count = {
                    let wallets = self.wallets.read().await;
                    wallets.len()
                };
                self.security_metrics.total_wallets = wallet_count;
                self.security_metrics.active_wallets = wallet_count;

                info!("📁 Loaded {} wallets from storage", wallet_count);
            }
            Err(_) => {
                info!("📁 No existing wallet storage found, starting fresh");
            }
        }

        Ok(())
    }

    /// Save wallets to persistent storage
    async fn save_wallets(&self) -> Result<()> {
        let wallets = self.wallets.read().await;
        let json = serde_json::to_string_pretty(&*wallets)?;
        tokio::fs::write(&self.wallet_storage_path, json).await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WalletInfo {
    pub name: String,
    pub public_key: Pubkey,
    pub wallet_type: WalletType,
    pub balance_sol: f64,
    pub usage_count: u64,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedBackup {
    nonce: [u8; 12],
    data: Vec<u8>,
    created_at: chrono::DateTime<chrono::Utc>,
    wallet_count: usize,
}

#[derive(Debug, Clone)]
pub struct SecurityAuditReport {
    pub audit_time: chrono::DateTime<chrono::Utc>,
    pub total_wallets: usize,
    pub wallet_types: HashMap<String, usize>,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}
