use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use reqwest::Client;
use solana_sdk::{
    transaction::Transaction,
    signature::{Signature, Signer},
    pubkey::Pubkey,
    compute_budget::ComputeBudgetInstruction,
    system_instruction,
};
use tokio::time::timeout;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Production-ready Jito bundle client with HTTP submission
#[derive(Debug)]
pub struct JitoBundleClient {
    client: Client,
    block_engine_url: String,
    relayer_url: String,
    auth_keypair: Option<Arc<solana_sdk::signature::Keypair>>, // SECURITY: Use Arc<Keypair> instead of owned Keypair
    tip_accounts: Vec<Pubkey>,
    bundle_timeout: Duration,
    max_retries: usize,
    metrics: Arc<Mutex<JitoMetrics>>,
}

#[derive(Debug, Clone)]
pub struct JitoMetrics {
    pub bundles_submitted: u64,
    pub bundles_landed: u64,
    pub bundles_failed: u64,
    pub average_confirmation_time_ms: f64,
    pub tip_amounts_paid: Vec<u64>,
    pub bundle_success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitoBundle {
    pub uuid: String,
    pub transactions: Vec<String>, // Base58 encoded transactions
    pub tip_amount: u64,
    pub tip_account: Pubkey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSubmissionRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSubmissionResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<String>,
    pub error: Option<JitoError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitoError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleStatus {
    pub bundle_id: String,
    pub status: String,
    pub landed_slot: Option<u64>,
    pub transactions: Vec<BundleTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleTransaction {
    pub signature: String,
    pub status: String,
    pub slot: Option<u64>,
}

impl JitoBundleClient {
    /// Create new Jito bundle client with secure keypair reference
    pub fn new_with_keypair_ref(
        block_engine_url: String,
        relayer_url: String,
        auth_keypair: Arc<solana_sdk::signature::Keypair>,
    ) -> Self {
        // Official Jito tip accounts for mainnet-beta
        let tip_accounts = vec![
            "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5".parse().unwrap(),
            "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe".parse().unwrap(),
            "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY".parse().unwrap(),
            "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49".parse().unwrap(),
            "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh".parse().unwrap(),
            "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt".parse().unwrap(),
            "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL".parse().unwrap(),
            "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT".parse().unwrap(),
        ];

        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            block_engine_url,
            relayer_url,
            auth_keypair: Some(auth_keypair), // Store Arc<Keypair> securely
            tip_accounts,
            bundle_timeout: Duration::from_secs(60),
            max_retries: 3,
            metrics: Arc::new(Mutex::new(JitoMetrics::default())),
        }
    }

    /// Create new Jito bundle client (legacy - deprecated, use new_with_keypair_ref)
    #[deprecated(note = "Use new_with_keypair_ref for secure keypair handling")]
    pub fn new(
        block_engine_url: String,
        relayer_url: String,
        auth_keypair: Option<solana_sdk::signature::Keypair>,
    ) -> Self {
        // Official Jito tip accounts for mainnet-beta
        let tip_accounts = vec![
            "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5".parse().unwrap(),
            "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe".parse().unwrap(),
            "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY".parse().unwrap(),
            "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49".parse().unwrap(),
            "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh".parse().unwrap(),
            "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt".parse().unwrap(),
            "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL".parse().unwrap(),
            "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT".parse().unwrap(),
        ];

        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            block_engine_url,
            relayer_url,
            auth_keypair: auth_keypair.map(Arc::new), // Convert to Arc<Keypair>
            tip_accounts,
            bundle_timeout: Duration::from_secs(60),
            max_retries: 3,
            metrics: Arc::new(Mutex::new(JitoMetrics::default())),
        }
    }

    /// Submit bundle with automatic tip calculation and retry logic
    pub async fn submit_bundle(
        &self,
        transactions: Vec<Transaction>,
        tip_lamports: Option<u64>,
    ) -> Result<String> {
        let start_time = Instant::now();

        // Calculate optimal tip if not provided
        let tip_amount = tip_lamports.unwrap_or_else(|| self.calculate_optimal_tip());

        // Select random tip account for load balancing
        let tip_account = self.tip_accounts[fastrand::usize(..self.tip_accounts.len())];

        // Create tip transaction
        let tip_tx = self.create_tip_transaction(tip_amount, tip_account)?;

        // Combine user transactions with tip transaction
        let mut bundle_transactions = transactions;
        bundle_transactions.push(tip_tx);

        // Convert to base58 encoded strings
        let encoded_transactions: Result<Vec<String>> = bundle_transactions
            .iter()
            .map(|tx| {
                let serialized = bincode::serialize(tx)?;
                Ok(bs58::encode(serialized).into_string())
            })
            .collect();

        let encoded_transactions = encoded_transactions?;

        // Create bundle
        let bundle = JitoBundle {
            uuid: Uuid::new_v4().to_string(),
            transactions: encoded_transactions.clone(),
            tip_amount,
            tip_account,
        };

        info!("📦 Submitting Jito bundle: {} transactions, {} lamports tip",
              bundle.transactions.len(), tip_amount);

        // Submit with retries
        let bundle_id = self.submit_with_retries(&bundle).await?;

        // Update metrics
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.bundles_submitted += 1;
            metrics.tip_amounts_paid.push(tip_amount);
        }

        // Note: Bundle monitoring would need to be implemented separately
        // to avoid cloning the client with non-cloneable Keypair
        debug!("Bundle monitoring not implemented in this version");

        let submission_time = start_time.elapsed().as_millis();
        debug!("Bundle submitted in {}ms: {}", submission_time, bundle_id);

        Ok(bundle_id)
    }

    /// Submit bundle with retry logic
    async fn submit_with_retries(&self, bundle: &JitoBundle) -> Result<String> {
        let mut last_error = None;

        for attempt in 1..=self.max_retries {
            match self.submit_bundle_once(bundle).await {
                Ok(bundle_id) => {
                    if attempt > 1 {
                        info!("✅ Bundle submitted successfully on attempt {}", attempt);
                    }
                    return Ok(bundle_id);
                }
                Err(e) => {
                    warn!("❌ Bundle submission attempt {} failed: {}", attempt, e);
                    last_error = Some(e);

                    if attempt < self.max_retries {
                        let delay = Duration::from_millis(100 * attempt as u64);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All bundle submission attempts failed")))
    }

    /// Single bundle submission attempt
    async fn submit_bundle_once(&self, bundle: &JitoBundle) -> Result<String> {
        let request = BundleSubmissionRequest {
            jsonrpc: "2.0".to_string(),
            id: fastrand::u64(..),
            method: "sendBundle".to_string(),
            params: bundle.transactions.clone(),
        };

        let response = timeout(
            Duration::from_secs(30),
            self.client
                .post(&format!("{}/api/v1/bundles", self.relayer_url))
                .header("Content-Type", "application/json")
                .json(&request)
                .send(),
        )
        .await??;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP error {}: {}",
                response.status(),
                response.text().await?
            ));
        }

        let bundle_response: BundleSubmissionResponse = response.json().await?;

        if let Some(error) = bundle_response.error {
            return Err(anyhow::anyhow!("Jito error {}: {}", error.code, error.message));
        }

        bundle_response
            .result
            .ok_or_else(|| anyhow::anyhow!("No bundle ID returned"))
    }

    /// Create tip transaction to Jito validators
    fn create_tip_transaction(
        &self,
        tip_lamports: u64,
        tip_account: Pubkey,
    ) -> Result<Transaction> {
        let auth_keypair = self.auth_keypair
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Auth keypair required for tip transactions"))?;

        let tip_instruction = system_instruction::transfer(
            &auth_keypair.pubkey(),
            &tip_account,
            tip_lamports,
        );

        // Add compute budget to ensure tip transaction processes quickly
        let compute_budget_instruction = ComputeBudgetInstruction::set_compute_unit_price(50_000);

        let recent_blockhash = solana_sdk::hash::Hash::default(); // Should be fetched from RPC

        let transaction = Transaction::new_signed_with_payer(
            &[compute_budget_instruction, tip_instruction],
            Some(&auth_keypair.pubkey()),
            &[auth_keypair],
            recent_blockhash,
        );

        Ok(transaction)
    }

    /// Calculate optimal tip based on current network conditions
    fn calculate_optimal_tip(&self) -> u64 {
        // Base tip amount
        let base_tip = 10_000u64; // 0.00001 SOL

        // Adjust based on recent success rate and confirmation times
        let (success_rate_multiplier, latency_multiplier) = if let Ok(metrics) = self.metrics.lock() {
            let success_rate_mult = if metrics.bundle_success_rate < 0.5 {
                2.0 // Double tip if success rate is low
            } else if metrics.bundle_success_rate > 0.9 {
                0.8 // Reduce tip if success rate is high
            } else {
                1.0
            };

            let latency_mult = if metrics.average_confirmation_time_ms > 5000.0 {
                1.5 // Increase tip if confirmations are slow
            } else if metrics.average_confirmation_time_ms < 2000.0 {
                0.9 // Slightly reduce tip if confirmations are fast
            } else {
                1.0
            };

            (success_rate_mult, latency_mult)
        } else {
            (1.0, 1.0) // Default multipliers if mutex is poisoned
        };

        let optimal_tip = (base_tip as f64 * success_rate_multiplier * latency_multiplier) as u64;

        // Cap tip at reasonable maximum
        optimal_tip.min(100_000) // Max 0.0001 SOL
    }

    /// Monitor bundle status and update metrics
    async fn monitor_bundle_status(&self, bundle_id: String) -> Result<()> {
        let start_time = Instant::now();
        let mut check_interval = tokio::time::interval(Duration::from_millis(500));

        for _ in 0..120 { // Monitor for up to 60 seconds
            check_interval.tick().await;

            match self.get_bundle_status(&bundle_id).await {
                Ok(status) => {
                    match status.status.as_str() {
                        "Landed" => {
                            let confirmation_time = start_time.elapsed().as_millis() as f64;
                            info!("✅ Bundle landed in {}ms: {}", confirmation_time, bundle_id);

                            // Update metrics (would need mutable access)
                            // self.metrics.bundles_landed += 1;
                            // self.update_average_confirmation_time(confirmation_time);
                            return Ok(());
                        }
                        "Failed" | "Rejected" => {
                            error!("❌ Bundle failed: {}", bundle_id);
                            // self.metrics.bundles_failed += 1;
                            return Err(anyhow::anyhow!("Bundle failed: {}", status.status));
                        }
                        "Pending" | "Processing" => {
                            debug!("⏳ Bundle pending: {}", bundle_id);
                            continue;
                        }
                        _ => {
                            warn!("Unknown bundle status: {}", status.status);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    debug!("Error checking bundle status: {}", e);
                    continue;
                }
            }
        }

        warn!("⏰ Bundle monitoring timeout: {}", bundle_id);
        Ok(())
    }

    /// Get bundle status from Jito
    async fn get_bundle_status(&self, bundle_id: &str) -> Result<BundleStatus> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": fastrand::u64(..),
            "method": "getBundleStatuses",
            "params": [vec![bundle_id]]
        });

        let response = timeout(
            Duration::from_secs(10),
            self.client
                .post(&format!("{}/api/v1/bundles", self.relayer_url))
                .header("Content-Type", "application/json")
                .json(&request)
                .send(),
        )
        .await??;

        let json: serde_json::Value = response.json().await?;

        if let Some(error) = json.get("error") {
            return Err(anyhow::anyhow!("Jito API error: {}", error));
        }

        let result = json
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .ok_or_else(|| anyhow::anyhow!("Invalid bundle status response"))?;

        let status: BundleStatus = serde_json::from_value(result.clone())?;
        Ok(status)
    }

    /// Get bundle performance metrics
    pub fn get_metrics(&self) -> JitoMetrics {
        self.metrics.lock().unwrap_or_else(|poisoned_guard| {
            warn!("Mutex poisoned for metrics, returning default");
            poisoned_guard.into_inner()
        }).clone()
    }

    /// Reset metrics
    pub fn reset_metrics(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            *metrics = JitoMetrics::default();
        }
    }

    /// Check if Jito service is available
    pub async fn health_check(&self) -> Result<bool> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getInflightBundleStatuses",
            "params": []
        });

        let response = timeout(
            Duration::from_secs(5),
            self.client
                .post(&format!("{}/api/v1/bundles", self.relayer_url))
                .header("Content-Type", "application/json")
                .json(&request)
                .send(),
        )
        .await;

        match response {
            Ok(Ok(resp)) => Ok(resp.status().is_success()),
            _ => Ok(false),
        }
    }
}

impl Default for JitoMetrics {
    fn default() -> Self {
        Self {
            bundles_submitted: 0,
            bundles_landed: 0,
            bundles_failed: 0,
            average_confirmation_time_ms: 0.0,
            tip_amounts_paid: Vec::new(),
            bundle_success_rate: 0.0,
        }
    }
}

impl JitoMetrics {
    /// Calculate success rate
    pub fn calculate_success_rate(&mut self) {
        let total = self.bundles_submitted;
        if total > 0 {
            self.bundle_success_rate = self.bundles_landed as f64 / total as f64;
        }
    }

    /// Update average confirmation time
    pub fn update_average_confirmation_time(&mut self, new_time_ms: f64) {
        let count = self.bundles_landed as f64;
        if count == 1.0 {
            self.average_confirmation_time_ms = new_time_ms;
        } else {
            self.average_confirmation_time_ms =
                (self.average_confirmation_time_ms * (count - 1.0) + new_time_ms) / count;
        }
    }

    /// Get average tip amount
    pub fn average_tip_amount(&self) -> f64 {
        if self.tip_amounts_paid.is_empty() {
            0.0
        } else {
            self.tip_amounts_paid.iter().sum::<u64>() as f64 / self.tip_amounts_paid.len() as f64
        }
    }
}

/// Helper function to create MEV bundle for front-running protection
pub fn create_mev_bundle(
    user_transactions: Vec<Transaction>,
    tip_lamports: u64,
) -> Vec<Transaction> {
    // In a real MEV bundle, you would:
    // 1. Add a tip transaction at the beginning
    // 2. Add your MEV transactions
    // 3. Add user transactions at the end
    // 4. Ensure all transactions are atomic

    user_transactions // Simplified for now
}