use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    signature::Signer,
    transaction::Transaction,
};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn, error};
use uuid::Uuid;

/// High-performance atomic bundle manager for Jito MEV execution
/// Target: 58ms bundle creation (85% below 400ms Solana block time)
#[derive(Clone)]
pub struct JitoBundleManager {
    jito_endpoint: String,
    client: reqwest::Client,
    bundle_stats: BundleStats,
    max_bundle_size: usize,
    priority_fee_calculator: PriorityFeeCalculator,
}

#[derive(Debug, Clone)]
pub struct BundleStats {
    pub total_bundles_created: u64,
    pub successful_submissions: u64,
    pub failed_submissions: u64,
    pub average_creation_time_ms: f64,
    pub below_target_percentage: f64, // % of bundles created in <58ms
}

#[derive(Debug, Clone)]
pub struct PriorityFeeCalculator {
    base_priority_fee: u64,
    congestion_multiplier: f64,
    max_priority_fee: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AtomicBundle {
    pub bundle_id: String,
    pub transactions: Vec<String>, // Base58 encoded transactions
    pub created_at: DateTime<Utc>,
    pub bundle_type: BundleType,
    pub estimated_profit: f64,
    pub priority_fee: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize)]
pub enum BundleType {
    SandwichAttack {
        front_run_tx: String,
        victim_tx: String,
        back_run_tx: String,
    },
    Arbitrage {
        buy_dex: String,
        sell_dex: String,
        token_pair: (String, String),
    },
    Liquidation {
        protocol: String,
        position_id: String,
    },
}

#[derive(Debug, Deserialize)]
struct JitoBundleResponse {
    bundle_id: String,
    status: String,
    error: Option<String>,
}

impl JitoBundleManager {
    /// Create new Jito bundle manager optimized for <58ms bundle creation
    pub fn new(jito_endpoint: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(500)) // Fast timeout for MEV speed
            .build()
            .expect("Failed to create HTTP client");

        Self {
            jito_endpoint,
            client,
            bundle_stats: BundleStats::default(),
            max_bundle_size: 5, // Jito limit
            priority_fee_calculator: PriorityFeeCalculator::new(),
        }
    }

    /// Create atomic sandwich attack bundle - target <58ms
    pub async fn create_sandwich_bundle(
        &mut self,
        front_run_instructions: Vec<Instruction>,
        victim_tx_hash: String,
        back_run_instructions: Vec<Instruction>,
        wallet_keypair: &solana_sdk::signature::Keypair,
        recent_blockhash: solana_sdk::hash::Hash,
    ) -> Result<AtomicBundle> {
        let start_time = Instant::now();

        // Calculate optimal priority fee based on profit and congestion
        let priority_fee = self.priority_fee_calculator.calculate_optimal_fee().await;

        // Build front-run transaction
        let front_run_tx = self.build_transaction(
            front_run_instructions,
            wallet_keypair,
            recent_blockhash,
            priority_fee,
        )?;

        // Build back-run transaction
        let back_run_tx = self.build_transaction(
            back_run_instructions,
            wallet_keypair,
            recent_blockhash,
            priority_fee,
        )?;

        let bundle = AtomicBundle {
            bundle_id: Uuid::new_v4().to_string(),
            transactions: vec![
                bs58::encode(bincode::serialize(&front_run_tx)?).into_string(),
                victim_tx_hash.clone(), // User's transaction we're sandwiching
                bs58::encode(bincode::serialize(&back_run_tx)?).into_string(),
            ],
            created_at: Utc::now(),
            bundle_type: BundleType::SandwichAttack {
                front_run_tx: bs58::encode(bincode::serialize(&front_run_tx)?).into_string(),
                victim_tx: victim_tx_hash,
                back_run_tx: bs58::encode(bincode::serialize(&back_run_tx)?).into_string(),
            },
            estimated_profit: 0.0, // Will be calculated by caller
            priority_fee,
            max_retries: 3,
        };

        let creation_time = start_time.elapsed().as_millis();
        self.update_bundle_stats(creation_time);

        debug!("🥪 Sandwich bundle created in {}ms (target: 58ms): {}",
               creation_time, bundle.bundle_id);

        if creation_time > 58 {
            warn!("⚠️ Bundle creation took {}ms (above 58ms target)", creation_time);
        }

        Ok(bundle)
    }

    /// Create atomic arbitrage bundle - target <58ms
    pub async fn create_arbitrage_bundle(
        &mut self,
        buy_instructions: Vec<Instruction>,
        sell_instructions: Vec<Instruction>,
        wallet_keypair: &solana_sdk::signature::Keypair,
        recent_blockhash: solana_sdk::hash::Hash,
        buy_dex: String,
        sell_dex: String,
        token_pair: (String, String),
    ) -> Result<AtomicBundle> {
        let start_time = Instant::now();

        let priority_fee = self.priority_fee_calculator.calculate_optimal_fee().await;

        // Build buy transaction
        let buy_tx = self.build_transaction(
            buy_instructions,
            wallet_keypair,
            recent_blockhash,
            priority_fee,
        )?;

        // Build sell transaction
        let sell_tx = self.build_transaction(
            sell_instructions,
            wallet_keypair,
            recent_blockhash,
            priority_fee,
        )?;

        let bundle = AtomicBundle {
            bundle_id: Uuid::new_v4().to_string(),
            transactions: vec![
                bs58::encode(bincode::serialize(&buy_tx)?).into_string(),
                bs58::encode(bincode::serialize(&sell_tx)?).into_string(),
            ],
            created_at: Utc::now(),
            bundle_type: BundleType::Arbitrage {
                buy_dex: buy_dex.clone(),
                sell_dex: sell_dex.clone(),
                token_pair: token_pair.clone(),
            },
            estimated_profit: 0.0,
            priority_fee,
            max_retries: 3,
        };

        let creation_time = start_time.elapsed().as_millis();
        self.update_bundle_stats(creation_time);

        debug!("💰 Arbitrage bundle created in {}ms: {} -> {} ({})",
               creation_time, buy_dex, sell_dex, bundle.bundle_id);

        Ok(bundle)
    }

    /// Create liquidation bundle for atomic execution
    pub async fn create_liquidation_bundle(
        &mut self,
        liquidation_instructions: Vec<Instruction>,
        wallet_keypair: &solana_sdk::signature::Keypair,
        recent_blockhash: solana_sdk::hash::Hash,
    ) -> Result<AtomicBundle> {
        let start_time = Instant::now();
        let bundle_id = Uuid::new_v4().to_string();

        // Calculate optimal priority fee
        let priority_fee = self.priority_fee_calculator.calculate_optimal_fee().await;

        // Build transaction with liquidation instructions
        let transaction = self.build_transaction(
            liquidation_instructions,
            wallet_keypair,
            recent_blockhash,
            priority_fee,
        )?;

        let transaction_b58 = bs58::encode(bincode::serialize(&transaction)?).into_string();

        let bundle = AtomicBundle {
            bundle_id: bundle_id.clone(),
            transactions: vec![transaction_b58],
            created_at: Utc::now(),
            bundle_type: BundleType::Liquidation {
                protocol: "Unknown".to_string(), // Will be updated with actual protocol
                position_id: "Unknown".to_string(), // Will be updated with actual position
            },
            estimated_profit: 0.1, // Will be updated with actual profit calculation
            priority_fee,
            max_retries: 3,
        };

        let creation_time = start_time.elapsed().as_millis();
        self.update_bundle_stats(creation_time);

        debug!("💧 Liquidation bundle created in {}ms: {}",
               creation_time, bundle.bundle_id);

        Ok(bundle)
    }

    /// Submit bundle to Jito for execution
    pub async fn submit_bundle(&mut self, bundle: &AtomicBundle) -> Result<String> {
        let submit_start = Instant::now();

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendBundle",
            "params": [{
                "transactions": bundle.transactions
            }]
        });

        let response = self.client
            .post(&format!("{}/api/v1/bundles", self.jito_endpoint))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let submit_time = submit_start.elapsed().as_millis();

        if response.status().is_success() {
            let bundle_response: JitoBundleResponse = response.json().await?;

            info!("✅ Bundle submitted successfully in {}ms: {} -> Jito ID: {}",
                  submit_time, bundle.bundle_id, bundle_response.bundle_id);

            self.bundle_stats.successful_submissions += 1;
            Ok(bundle_response.bundle_id)
        } else {
            let error_text = response.text().await?;
            error!("❌ Bundle submission failed: {}", error_text);

            self.bundle_stats.failed_submissions += 1;
            Err(anyhow::anyhow!("Jito bundle submission failed: {}", error_text))
        }
    }

    /// Build optimized transaction for bundle inclusion
    fn build_transaction(
        &self,
        instructions: Vec<Instruction>,
        wallet_keypair: &solana_sdk::signature::Keypair,
        recent_blockhash: solana_sdk::hash::Hash,
        priority_fee: u64,
    ) -> Result<Transaction> {
        // Add priority fee instruction for MEV competitiveness
        let mut all_instructions = vec![
            solana_sdk::system_instruction::transfer(
                &wallet_keypair.pubkey(),
                &wallet_keypair.pubkey(),
                priority_fee,
            ),
        ];
        all_instructions.extend(instructions);

        let message = Message::new(&all_instructions, Some(&wallet_keypair.pubkey()));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.sign(&[wallet_keypair], recent_blockhash);

        Ok(transaction)
    }

    /// Update performance statistics
    fn update_bundle_stats(&mut self, creation_time_ms: u128) {
        self.bundle_stats.total_bundles_created += 1;

        // Update rolling average
        let total = self.bundle_stats.total_bundles_created as f64;
        let current_avg = self.bundle_stats.average_creation_time_ms;
        self.bundle_stats.average_creation_time_ms =
            (current_avg * (total - 1.0) + creation_time_ms as f64) / total;

        // Track percentage below 58ms target
        let below_target = if creation_time_ms <= 58 { 1.0 } else { 0.0 };
        self.bundle_stats.below_target_percentage =
            (self.bundle_stats.below_target_percentage * (total - 1.0) + below_target) / total;
    }

    /// Get bundle performance statistics
    pub fn get_bundle_stats(&self) -> BundleStats {
        self.bundle_stats.clone()
    }

    /// Get bundle efficiency report for monitoring
    pub fn get_performance_report(&self) -> BundlePerformanceReport {
        BundlePerformanceReport {
            total_bundles: self.bundle_stats.total_bundles_created,
            success_rate: if self.bundle_stats.total_bundles_created > 0 {
                (self.bundle_stats.successful_submissions as f64 /
                 self.bundle_stats.total_bundles_created as f64) * 100.0
            } else { 0.0 },
            average_creation_time_ms: self.bundle_stats.average_creation_time_ms,
            below_target_percentage: self.bundle_stats.below_target_percentage * 100.0,
            target_creation_time_ms: 58.0,
            target_success_rate: 85.0, // From user's previous bot metrics
        }
    }
}

impl PriorityFeeCalculator {
    fn new() -> Self {
        Self {
            base_priority_fee: 1000, // 1000 lamports base
            congestion_multiplier: 1.5,
            max_priority_fee: 100_000, // 100k lamports max
        }
    }

    /// Calculate optimal priority fee based on network congestion
    async fn calculate_optimal_fee(&self) -> u64 {
        // In production, this would check mempool congestion
        // For now, use base fee with some randomization for competitiveness
        let congestion_factor = fastrand::f64() * self.congestion_multiplier;
        let calculated_fee = (self.base_priority_fee as f64 * (1.0 + congestion_factor)) as u64;

        calculated_fee.min(self.max_priority_fee)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundlePerformanceReport {
    pub total_bundles: u64,
    pub success_rate: f64,
    pub average_creation_time_ms: f64,
    pub below_target_percentage: f64,
    pub target_creation_time_ms: f64,
    pub target_success_rate: f64,
}

impl Default for BundleStats {
    fn default() -> Self {
        Self {
            total_bundles_created: 0,
            successful_submissions: 0,
            failed_submissions: 0,
            average_creation_time_ms: 0.0,
            below_target_percentage: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Keypair;

    #[test]
    fn test_bundle_manager_creation() {
        let manager = JitoBundleManager::new("https://jito-test.com".to_string());
        assert_eq!(manager.max_bundle_size, 5);
        assert_eq!(manager.bundle_stats.total_bundles_created, 0);
    }

    #[test]
    fn test_priority_fee_calculation() {
        let calculator = PriorityFeeCalculator::new();
        assert_eq!(calculator.base_priority_fee, 1000);
        assert_eq!(calculator.max_priority_fee, 100_000);
    }

    #[tokio::test]
    async fn test_bundle_stats_update() {
        let mut manager = JitoBundleManager::new("https://jito-test.com".to_string());

        // Simulate fast bundle creation (below target)
        manager.update_bundle_stats(45);
        assert_eq!(manager.bundle_stats.total_bundles_created, 1);
        assert_eq!(manager.bundle_stats.below_target_percentage, 1.0);

        // Simulate slow bundle creation (above target)
        manager.update_bundle_stats(75);
        assert_eq!(manager.bundle_stats.total_bundles_created, 2);
        assert_eq!(manager.bundle_stats.below_target_percentage, 0.5);
    }
}