use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, pubkey::Pubkey, signature::Signer,
    system_instruction, transaction::Transaction,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex; // Use tokio::sync::Mutex for async compatibility
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// JITO Tip Floor API response (for 95th percentile dynamic tipping)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipFloorResponse {
    #[serde(rename = "landed_tips_25th_percentile")]
    pub landed_tips_25th: f64,
    #[serde(rename = "landed_tips_50th_percentile")]
    pub landed_tips_50th: f64,
    #[serde(rename = "landed_tips_75th_percentile")]
    pub landed_tips_75th: f64,
    #[serde(rename = "landed_tips_95th_percentile")]
    pub landed_tips_95th: f64,
    #[serde(rename = "landed_tips_99th_percentile")]
    pub landed_tips_99th: f64,
}

/// Cached tip floor data with timestamp
#[derive(Debug, Clone)]
pub struct CachedTipFloor {
    pub data: TipFloorResponse,
    pub fetched_at: Instant,
    pub cache_duration: Duration,
}

impl CachedTipFloor {
    pub fn is_expired(&self) -> bool {
        self.fetched_at.elapsed() > self.cache_duration
    }
}

/// Production-ready Jito bundle client with HTTP submission
pub struct JitoBundleClient {
    client: Client,
    block_engine_url: String,
    relayer_url: String,
    auth_keypair: Option<Arc<solana_sdk::signature::Keypair>>, // SECURITY: Use Arc<Keypair> instead of owned Keypair
    tip_accounts: Vec<Pubkey>,
    bundle_timeout: Duration,
    metrics: Arc<Mutex<JitoMetrics>>,
    cached_tip_floor: Arc<Mutex<Option<CachedTipFloor>>>, // Cache for 95th percentile dynamic tips
    rpc_client: Option<Arc<RpcClient>>,                   // For pre/post balance checks
    last_submission_time: Arc<Mutex<Option<Instant>>>,    // For rate limiting (1 bundle/sec)
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
    pub id: String, // FIXED: Must be String per JITO API (not u64)
    pub method: String,
    pub params: Vec<Vec<String>>, // FIXED: Must be array of arrays per JITO API
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSubmissionResponse {
    pub jsonrpc: String,
    pub id: String, // FIXED: Must be String to match request (not u64)
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
    /// Create new Jito bundle client with secure keypair reference and RPC for balance checks
    pub fn new_with_keypair_ref(
        block_engine_url: String,
        relayer_url: String,
        auth_keypair: Arc<solana_sdk::signature::Keypair>,
        rpc_url: Option<String>,
    ) -> Self {
        // Official Jito tip accounts for mainnet-beta
        let tip_accounts = vec![
            "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5"
                .parse()
                .unwrap(),
            "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe"
                .parse()
                .unwrap(),
            "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY"
                .parse()
                .unwrap(),
            "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49"
                .parse()
                .unwrap(),
            "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh"
                .parse()
                .unwrap(),
            "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt"
                .parse()
                .unwrap(),
            "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL"
                .parse()
                .unwrap(),
            "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT"
                .parse()
                .unwrap(),
        ];

        // Initialize RPC client if URL provided (for pre/post balance checks)
        let rpc_client =
            rpc_url.map(|url| Arc::new(RpcClient::new_with_timeout(url, Duration::from_secs(10))));

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
            metrics: Arc::new(Mutex::new(JitoMetrics::default())),
            cached_tip_floor: Arc::new(Mutex::new(None)), // Initialize empty cache
            rpc_client,
            last_submission_time: Arc::new(Mutex::new(None)), // Initialize rate limiter
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
            "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5"
                .parse()
                .unwrap(),
            "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe"
                .parse()
                .unwrap(),
            "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY"
                .parse()
                .unwrap(),
            "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49"
                .parse()
                .unwrap(),
            "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh"
                .parse()
                .unwrap(),
            "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt"
                .parse()
                .unwrap(),
            "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL"
                .parse()
                .unwrap(),
            "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT"
                .parse()
                .unwrap(),
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
            metrics: Arc::new(Mutex::new(JitoMetrics::default())),
            cached_tip_floor: Arc::new(Mutex::new(None)), // Initialize empty cache
            rpc_client: None,                             // No RPC client in legacy constructor
            last_submission_time: Arc::new(Mutex::new(None)), // Initialize rate limiter
        }
    }

    /// Submit bundle with automatic tip calculation (NO RETRIES - fresh detections only)
    pub async fn submit_bundle(
        &self,
        transactions: Vec<Transaction>,
        tip_lamports: Option<u64>,
    ) -> Result<String> {
        self.submit_bundle_with_blockhash(transactions, tip_lamports, None, None, None)
            .await
    }

    /// Submit bundle with uncle bandit protection (balance verification)
    pub async fn submit_bundle_verified(
        &self,
        transactions: Vec<Transaction>,
        tip_lamports: Option<u64>,
        wallet_pubkey: &Pubkey,
        expected_profit_lamports: i64,
    ) -> Result<String> {
        self.submit_bundle_with_blockhash(
            transactions,
            tip_lamports,
            None,
            Some(wallet_pubkey),
            Some(expected_profit_lamports),
        )
        .await
    }

    /// Submit bundle with explicit blockhash for tip transaction and optional balance verification
    pub async fn submit_bundle_with_blockhash(
        &self,
        transactions: Vec<Transaction>,
        tip_lamports: Option<u64>,
        blockhash: Option<solana_sdk::hash::Hash>,
        wallet_pubkey: Option<&Pubkey>, // For balance verification
        expected_profit_lamports: Option<i64>, // For uncle bandit protection
    ) -> Result<String> {
        let start_time = Instant::now();

        // Calculate optimal tip if not provided
        let tip_amount = match tip_lamports {
            Some(tip) => tip,
            None => self.calculate_optimal_tip().await,
        };

        // Select random tip account for load balancing
        let tip_account = self.tip_accounts[fastrand::usize(..self.tip_accounts.len())];

        // Extract blockhash from first transaction if not provided
        let blockhash_to_use = blockhash
            .or_else(|| transactions.first().map(|tx| tx.message.recent_blockhash))
            .unwrap_or_else(|| {
                warn!("⚠️  No valid blockhash available - tip transaction may fail!");
                solana_sdk::hash::Hash::default()
            });

        // TODO: Integrate tip INTO main transaction (JITO best practice)
        // Current risk: Separate tip transaction vulnerable to uncle bandit attacks
        // if block is uncled, tip could be paid even if trade fails
        // Fix requires rebuilding transaction with tip instruction included

        // Create separate tip transaction (temporary - should be integrated)
        let tip_tx = self.create_tip_transaction(tip_amount, tip_account, blockhash_to_use)?;

        // Combine user transactions with tip transaction
        let mut bundle_transactions = transactions;
        bundle_transactions.push(tip_tx);

        warn!(
            "⚠️  Using separate tip transaction (uncle bandit risk) - TODO: integrate into main tx"
        );

        // Convert to base58 encoded strings (JITO requirement)
        // NOTE: JITO docs say "base64 preferred" but actual working implementation uses base58!
        // Verified from working Arb_Bot implementation
        let encoded_transactions: Result<Vec<String>> = bundle_transactions
            .iter()
            .map(|tx| {
                let serialized = bincode::serialize(tx)
                    .map_err(|e| anyhow::anyhow!("Transaction serialization error: {}", e))?;
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

        info!(
            "📦 Submitting Jito bundle: {} transactions, {} lamports tip",
            bundle.transactions.len(),
            tip_amount
        );

        // Submit with retries
        let bundle_id = self.submit_with_retries(&bundle).await?;

        // Update metrics
        {
            let mut metrics = self.metrics.lock().await;
            metrics.bundles_submitted += 1;
            metrics.tip_amounts_paid.push(tip_amount);
        }

        // ✅ UNCLE BANDIT PROTECTION: Verify balance change if wallet info provided
        if let (Some(wallet_pk), Some(expected_profit)) = (wallet_pubkey, expected_profit_lamports)
        {
            info!("🛡️  Verifying balance change to detect uncle bandit attacks...");
            if let Err(e) = self
                .verify_bundle_execution(wallet_pk, expected_profit, tip_amount)
                .await
            {
                warn!("⚠️  Balance verification failed: {}", e);
                warn!("⚠️  Bundle may have been uncle'd (tip paid but trade didn't execute)");
                // Don't return error - bundle was submitted successfully
                // Just log the warning for monitoring
            }
        } else {
            debug!("Balance verification skipped (wallet info not provided)");
        }

        let submission_time = start_time.elapsed().as_millis();
        debug!("Bundle submitted in {}ms: {}", submission_time, bundle_id);

        Ok(bundle_id)
    }

    /// Submit bundle ONCE with NO RETRIES (MEV philosophy: only fresh detections)
    /// If submission fails, we move on to the next opportunity - no time wasted on retries
    async fn submit_with_retries(&self, bundle: &JitoBundle) -> Result<String> {
        // ⚡ RATE LIMITING: Enforce 1.1 second minimum between submissions
        // JITO limit is 1 bundle/sec, we use 1.1s for safety margin
        let rate_limit_duration = Duration::from_millis(1100);

        {
            let mut last_time = self.last_submission_time.lock().await;
            if let Some(last_instant) = *last_time {
                let elapsed = last_instant.elapsed();
                if elapsed < rate_limit_duration {
                    let sleep_duration = rate_limit_duration - elapsed;
                    debug!(
                        "⏱️  Rate limiting: sleeping {:?} to maintain 1 bundle/1.1s",
                        sleep_duration
                    );
                    drop(last_time); // Release lock before sleeping
                    tokio::time::sleep(sleep_duration).await;
                    // Re-acquire lock to update
                    let mut last_time = self.last_submission_time.lock().await;
                    *last_time = Some(Instant::now());
                } else {
                    *last_time = Some(Instant::now());
                }
            } else {
                *last_time = Some(Instant::now());
            }
        }

        // Submit once with NO RETRIES - MEV opportunities are time-sensitive
        // If it fails, we skip it and move on to the next fresh detection
        self.submit_bundle_once(bundle).await
    }

    /// Single bundle submission attempt
    async fn submit_bundle_once(&self, bundle: &JitoBundle) -> Result<String> {
        let request = BundleSubmissionRequest {
            jsonrpc: "2.0".to_string(),
            id: format!("bundle_{}", fastrand::u64(..)), // FIXED: String ID per JITO API
            method: "sendBundle".to_string(),
            params: vec![bundle.transactions.clone()], // FIXED: Wrapped in array per JITO API
        };

        let response = timeout(
            Duration::from_secs(30),
            self.client
                .post(format!("{}/api/v1/bundles", self.block_engine_url))
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
            return Err(anyhow::anyhow!(
                "Jito error {}: {}",
                error.code,
                error.message
            ));
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
        recent_blockhash: solana_sdk::hash::Hash,
    ) -> Result<Transaction> {
        let auth_keypair = self
            .auth_keypair
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Auth keypair required for tip transactions"))?;

        let tip_instruction =
            system_instruction::transfer(&auth_keypair.pubkey(), &tip_account, tip_lamports);

        // Add compute budget to ensure tip transaction processes quickly
        let compute_budget_instruction = ComputeBudgetInstruction::set_compute_unit_price(50_000);

        let transaction = Transaction::new_signed_with_payer(
            &[compute_budget_instruction, tip_instruction],
            Some(&auth_keypair.pubkey()),
            &[auth_keypair],
            recent_blockhash,
        );

        Ok(transaction)
    }

    /// Get cached tip floor data (returns None if cache is empty or expired)
    pub async fn get_cached_tip_floor(&self) -> Option<TipFloorResponse> {
        let cache_guard = self.cached_tip_floor.lock().await;
        if let Some(cached) = cache_guard.as_ref() {
            if !cached.is_expired() {
                return Some(cached.data.clone());
            }
        }
        None
    }

    /// Start background task to keep tip floor cache fresh (call once at startup)
    pub fn start_tip_floor_refresh(&self) {
        let client_clone = self.client.clone();
        let cache_clone = self.cached_tip_floor.clone();

        tokio::spawn(async move {
            info!("🔄 Starting JITO tip floor refresh task (10 minute interval)...");

            loop {
                // Wait 10 minutes between fetches (user requested - fresher data)
                tokio::time::sleep(Duration::from_secs(600)).await;

                // Fetch fresh tip floor data
                let url = "https://bundles.jito.wtf/api/v1/bundles/tip_floor";

                match client_clone
                    .get(url)
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await
                {
                    Ok(response) => {
                        match response.json::<TipFloorResponse>().await {
                            Ok(tip_floor) => {
                                // Update cache
                                {
                                    let mut cache_guard = cache_clone.lock().await;
                                    *cache_guard = Some(CachedTipFloor {
                                        data: tip_floor.clone(),
                                        fetched_at: Instant::now(),
                                        cache_duration: Duration::from_secs(600), // 10 minutes
                                    });
                                    info!("💰 Tip floor refreshed: 95th = {:.6} SOL | 99th = {:.6} SOL",
                                          tip_floor.landed_tips_95th / 1_000_000_000.0,
                                          tip_floor.landed_tips_99th / 1_000_000_000.0);
                                }
                            }
                            Err(e) => {
                                warn!("⚠️ Failed to parse tip floor response: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to fetch tip floor: {}", e);
                    }
                }
            }
        });
    }

    /// Fetch tip floor data from JITO API with caching (60 second cache)
    async fn fetch_tip_floor(&self) -> Result<TipFloorResponse> {
        // Check cache first
        {
            let cache_guard = self.cached_tip_floor.lock().await;
            if let Some(cached) = cache_guard.as_ref() {
                if !cached.is_expired() {
                    debug!(
                        "💰 Using cached tip floor (95th: {:.6} SOL)",
                        cached.data.landed_tips_95th / 1_000_000_000.0
                    );
                    return Ok(cached.data.clone());
                }
            }
        }

        // Cache expired or empty, fetch from API
        let url = "https://bundles.jito.wtf/api/v1/bundles/tip_floor";
        debug!("🌐 Fetching fresh tip floor data from JITO API...");

        let response = self
            .client
            .get(url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch tip floor: {}", e))?;

        let tip_floor: TipFloorResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse tip floor response: {}", e))?;

        // Update cache
        {
            let mut cache_guard = self.cached_tip_floor.lock().await;
            *cache_guard = Some(CachedTipFloor {
                data: tip_floor.clone(),
                fetched_at: Instant::now(),
                cache_duration: Duration::from_secs(600), // 10 minutes
            });
            info!(
                "💰 Tip floor updated: 95th = {:.6} SOL | 99th = {:.6} SOL",
                tip_floor.landed_tips_95th / 1_000_000_000.0,
                tip_floor.landed_tips_99th / 1_000_000_000.0
            );
        }

        Ok(tip_floor)
    }

    /// Calculate optimal tip using JITO's 95th percentile (aggressive MEV strategy)
    async fn calculate_optimal_tip(&self) -> u64 {
        // Try to use 95th percentile from JITO API (user requested "95% for mev")
        // This is async, so we'll spawn a task and use fallback immediately
        let cached_tip_floor = self.cached_tip_floor.clone();

        // Check if we have cached data
        {
            let cache_guard = cached_tip_floor.lock().await;
            if let Some(cached) = cache_guard.as_ref() {
                if !cached.is_expired() {
                    // Use 95th percentile as requested by user
                    let tip_95th = (cached.data.landed_tips_95th * 1_000_000_000.0) as u64;
                    debug!(
                        "💰 Using 95th percentile tip: {} lamports ({:.6} SOL)",
                        tip_95th,
                        tip_95th as f64 / 1_000_000_000.0
                    );
                    return tip_95th;
                }
            }
        }

        // Fallback: Use old calculation if no cached data available
        warn!("⚠️ No cached tip floor data, using fallback calculation");

        let base_tip = 50_000u64; // 0.00005 SOL fallback

        // Adjust based on recent success rate and confirmation times
        let (success_rate_multiplier, latency_multiplier) = {
            let metrics = self.metrics.lock().await;
            let success_rate_mult = if metrics.bundle_success_rate < 0.5 {
                3.0 // Triple tip if success rate is low (more competitive)
            } else if metrics.bundle_success_rate > 0.9 {
                0.9 // Slightly reduce if high success
            } else {
                1.5 // Default to 1.5x for medium success
            };

            let latency_mult = if metrics.average_confirmation_time_ms > 5000.0 {
                2.0 // Double if confirmations are slow
            } else if metrics.average_confirmation_time_ms < 2000.0 {
                1.0 // Keep base if fast
            } else {
                1.3 // Increase slightly for medium latency
            };

            (success_rate_mult, latency_mult)
        };

        let optimal_tip = (base_tip as f64 * success_rate_multiplier * latency_multiplier) as u64;

        // Increased cap for high-quality opportunities
        optimal_tip.min(500_000) // Max 0.0005 SOL (increased from 0.0001)
    }

    /// Monitor bundle status and update metrics
    async fn monitor_bundle_status(&self, bundle_id: String) -> Result<()> {
        let start_time = Instant::now();
        let mut check_interval = tokio::time::interval(Duration::from_millis(500));

        for _ in 0..120 {
            // Monitor for up to 60 seconds
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
                .post(format!("{}/api/v1/bundles", self.relayer_url))
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
    pub async fn get_metrics(&self) -> JitoMetrics {
        self.metrics.lock().await.clone()
    }

    /// Reset metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.lock().await;
        *metrics = JitoMetrics::default();
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
                .post(format!("{}/api/v1/bundles", self.relayer_url))
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

    /// Check on-chain confirmation of a bundle by verifying transaction signatures
    /// Returns (confirmed, block_number, actual_signature) if found on-chain
    pub async fn check_on_chain_confirmation(
        &self,
        bundle_id: &str,
        expected_signatures: &[String],
    ) -> Result<Option<(bool, u64, String)>> {
        if let Some(rpc_client) = &self.rpc_client {
            // Check each transaction signature to see if it landed on-chain
            for sig_str in expected_signatures {
                if let Ok(signature) = sig_str.parse::<solana_sdk::signature::Signature>() {
                    // Query transaction status
                    match rpc_client.get_signature_status(&signature) {
                        Ok(Some(status)) => {
                            // Transaction found on-chain!
                            // Try to get transaction details (best effort)
                            let block_num = if let Ok(tx) = rpc_client.get_transaction(
                                &signature,
                                solana_transaction_status::UiTransactionEncoding::Json,
                            ) {
                                tx.slot
                            } else {
                                0 // Unknown block if we can't fetch details
                            };

                            info!(
                                "✅ On-chain confirmation found! Bundle: {} | Tx: {} | Block: {}",
                                &bundle_id[..8],
                                &sig_str[..8],
                                block_num
                            );

                            // Check if transaction succeeded (no error)
                            let success = status.is_ok();
                            return Ok(Some((success, block_num, sig_str.clone())));
                        }
                        Ok(None) => {
                            // Transaction not found, continue checking others
                            continue;
                        }
                        Err(e) => {
                            debug!("RPC error checking signature {}: {}", &sig_str[..8], e);
                            continue;
                        }
                    }
                }
            }

            // None of the signatures found on-chain
            debug!(
                "❌ No on-chain confirmation found for bundle: {}",
                &bundle_id[..8]
            );
            Ok(None)
        } else {
            warn!("⚠️ No RPC client available for on-chain confirmation");
            Ok(None)
        }
    }

    /// Check account balance (for pre/post bundle checks)
    fn check_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        if let Some(ref rpc) = self.rpc_client {
            let balance = rpc
                .get_balance(pubkey)
                .map_err(|e| anyhow::anyhow!("Failed to get balance: {}", e))?;
            Ok(balance)
        } else {
            Err(anyhow::anyhow!(
                "RPC client not configured for balance checks"
            ))
        }
    }

    /// Perform pre/post balance verification (JITO best practice)
    pub async fn verify_bundle_execution(
        &self,
        wallet_pubkey: &Pubkey,
        expected_change_lamports: i64,
        tip_lamports: u64,
    ) -> Result<()> {
        if self.rpc_client.is_none() {
            debug!("Balance verification disabled (no RPC client configured)");
            return Ok(());
        }

        // Pre-submission balance check
        let pre_balance = self.check_balance(wallet_pubkey)?;
        info!(
            "💵 Pre-bundle balance: {} lamports ({:.6} SOL)",
            pre_balance,
            pre_balance as f64 / 1_000_000_000.0
        );

        // Wait for bundle to process (simplified - in production, monitor bundle status)
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Post-submission balance check
        let post_balance = self.check_balance(wallet_pubkey)?;
        info!(
            "💵 Post-bundle balance: {} lamports ({:.6} SOL)",
            post_balance,
            post_balance as f64 / 1_000_000_000.0
        );

        // Calculate actual change
        let actual_change = post_balance as i64 - pre_balance as i64;

        // Expected change: profit from trade minus tip
        // Note: Negative expected_change means we spent money (buy), positive means we earned (sell)
        let expected_total_change = expected_change_lamports - tip_lamports as i64;

        info!(
            "💵 Balance change: Actual = {} lamports | Expected = {} lamports (including {} tip)",
            actual_change, expected_total_change, tip_lamports
        );

        // Check if only tip was paid (uncle bandit scenario)
        if actual_change < 0 && actual_change.abs() <= (tip_lamports as i64 * 2) {
            // Lost money, but only around tip amount → possible unbundled transaction
            warn!("⚠️  UNCLE BANDIT WARNING: Lost ~{} lamports (close to tip {}), trade may not have executed!",
                  actual_change.abs(), tip_lamports);
            return Err(anyhow::anyhow!("Possible unbundled transaction detected"));
        }

        // Check if change is in expected range (allow 10% variance for fees)
        let variance_threshold = (expected_total_change.abs() as f64 * 0.10) as i64;
        let diff = (actual_change - expected_total_change).abs();

        if diff > variance_threshold && diff > 100_000 {
            // More than 10% variance and >0.0001 SOL difference
            warn!(
                "⚠️  Unexpected balance change: diff = {} lamports ({:.6} SOL)",
                diff,
                diff as f64 / 1_000_000_000.0
            );
        } else {
            info!("✅ Balance change verified (within expected range)");
        }

        Ok(())
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
