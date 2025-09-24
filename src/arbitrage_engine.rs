use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Signer,
};
use std::str::FromStr;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tracing::{debug, info, warn, error};

use crate::dex_registry::{DexRegistry, DexInfo};
use crate::dynamic_fee_model::DynamicFeeModel;
use crate::jupiter_executor::JupiterExecutor;
use crate::jito_bundle_manager::{JitoBundleManager, AtomicBundle};
use crate::wallet_manager::WalletManager;

/// High-performance arbitrage engine for cross-DEX price differences
/// Detects and executes profitable arbitrage opportunities across all supported DEXs
pub struct ArbitrageEngine {
    dex_registry: DexRegistry,
    fee_model: DynamicFeeModel,
    jupiter_executor: JupiterExecutor,
    bundle_manager: JitoBundleManager,
    wallet_manager: WalletManager,
    min_profit_sol: f64,
    max_position_size_sol: f64,
    price_cache: HashMap<String, TokenPrice>,
    stats: ArbitrageStats,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ArbitrageStats {
    pub opportunities_detected: u64,
    pub opportunities_executed: u64,
    pub total_profit_sol: f64,
    pub average_execution_time_ms: f64,
    pub failed_executions: u64,
    pub cross_dex_opportunities: u64,
    pub price_updates_processed: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArbitrageOpportunity {
    pub opportunity_id: String,
    pub token_pair: TokenPair,
    pub buy_dex: DexInfo,
    pub sell_dex: DexInfo,
    pub buy_price: f64,
    pub sell_price: f64,
    pub price_difference_percent: f64,
    pub optimal_amount: u64,
    pub estimated_profit_sol: f64,
    pub confidence_score: f64,
    pub execution_priority: u8,
    pub detected_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    pub base_mint: String,
    pub quote_mint: String,
    pub base_symbol: String,
    pub quote_symbol: String,
}

#[derive(Debug, Clone)]
struct TokenPrice {
    pub price: f64,
    pub liquidity: u64,
    pub last_updated: DateTime<Utc>,
    pub dex_name: String,
    pub volume_24h: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArbitrageExecution {
    pub opportunity_id: String,
    pub bundle_id: String,
    pub execution_time_ms: u64,
    pub actual_profit_sol: f64,
    pub success: bool,
    pub error_message: Option<String>,
    pub buy_signature: Option<String>,
    pub sell_signature: Option<String>,
    pub slippage_percent: f64,
}

impl ArbitrageEngine {
    /// Create new arbitrage engine targeting cross-DEX opportunities
    pub fn new(
        jupiter_api_key: String,
        jito_endpoint: String,
        min_profit_sol: f64,
        max_position_size_sol: f64,
    ) -> Result<Self> {
        let wallet_manager = WalletManager::from_env()?;

        Ok(Self {
            dex_registry: DexRegistry::new(),
            fee_model: DynamicFeeModel::new(),
            jupiter_executor: JupiterExecutor::new(jupiter_api_key),
            bundle_manager: JitoBundleManager::new(jito_endpoint),
            wallet_manager,
            min_profit_sol,
            max_position_size_sol,
            price_cache: HashMap::new(),
            stats: ArbitrageStats::default(),
        })
    }

    /// Update price data from market feeds (called continuously)
    pub fn update_price_data(&mut self, token_mint: &str, dex_name: &str, price: f64, liquidity: u64) {
        let price_entry = TokenPrice {
            price,
            liquidity,
            last_updated: Utc::now(),
            dex_name: dex_name.to_string(),
            volume_24h: 0.0, // Would be populated from real data
        };

        let key = format!("{}:{}", token_mint, dex_name);
        self.price_cache.insert(key, price_entry);
        self.stats.price_updates_processed += 1;

        // Note: Arbitrage scanning would happen asynchronously in production
        debug!("Price updated for {}: {} on {}", token_mint, price, dex_name);
    }

    /// Scan for arbitrage opportunities for a specific token
    async fn scan_for_arbitrage_opportunities(&mut self, token_mint: &str) -> Result<()> {
        let arbitrage_pairs = self.dex_registry.get_arbitrage_pairs();
        let mut opportunities = Vec::new();

        // Get all prices for this token across DEXs
        let token_prices: HashMap<String, &TokenPrice> = self.price_cache
            .iter()
            .filter(|(key, _)| key.starts_with(token_mint))
            .map(|(key, price)| (key.split(':').nth(1).unwrap().to_string(), price))
            .collect();

        if token_prices.len() < 2 {
            return Ok(()); // Need at least 2 DEXs for arbitrage
        }

        // Check all DEX pairs for arbitrage opportunities
        for (dex1, dex2) in arbitrage_pairs {
            if let (Some(price1), Some(price2)) = (
                token_prices.get(&dex1.name),
                token_prices.get(&dex2.name)
            ) {
                // Calculate price difference
                let price_diff_percent = ((price2.price - price1.price) / price1.price).abs() * 100.0;

                // Check if price difference is significant enough
                if price_diff_percent > 0.5 { // 0.5% minimum spread
                    let (buy_dex, sell_dex, buy_price, sell_price) = if price1.price < price2.price {
                        (dex1, dex2, price1.price, price2.price)
                    } else {
                        (dex2, dex1, price2.price, price1.price)
                    };

                    // Calculate optimal trade size and profit
                    if let Ok(opportunity) = self.calculate_arbitrage_opportunity(
                        token_mint,
                        buy_dex,
                        sell_dex,
                        buy_price,
                        sell_price,
                        price_diff_percent,
                    ).await {
                        if opportunity.estimated_profit_sol >= self.min_profit_sol {
                            opportunities.push(opportunity);
                        }
                    }
                }
            }
        }

        // Execute the most profitable opportunity
        if let Some(best_opportunity) = opportunities.into_iter()
            .max_by(|a, b| a.estimated_profit_sol.partial_cmp(&b.estimated_profit_sol).unwrap()) {

            self.stats.opportunities_detected += 1;
            info!("💰 Arbitrage opportunity detected: {:.4} SOL profit ({:.2}% spread) - {} -> {}",
                  best_opportunity.estimated_profit_sol,
                  best_opportunity.price_difference_percent,
                  best_opportunity.buy_dex.name,
                  best_opportunity.sell_dex.name);

            // Execute immediately for time-sensitive arbitrage
            tokio::spawn(async move {
                // Would execute the opportunity here
                // For now, just log it
                debug!("Would execute arbitrage opportunity: {}", best_opportunity.opportunity_id);
            });
        }

        Ok(())
    }

    /// Calculate arbitrage opportunity parameters
    async fn calculate_arbitrage_opportunity(
        &self,
        token_mint: &str,
        buy_dex: &DexInfo,
        sell_dex: &DexInfo,
        buy_price: f64,
        sell_price: f64,
        price_diff_percent: f64,
    ) -> Result<ArbitrageOpportunity> {
        // Calculate optimal trade size based on liquidity and price impact
        let optimal_amount = self.calculate_optimal_trade_size(buy_price, sell_price)?;

        // Check balance requirements
        let balance_info = self.wallet_manager.get_balance_info().await?;
        let required_sol = (optimal_amount as f64 / 1_000_000_000.0) * buy_price;

        if required_sol > balance_info.sol_balance * 0.8 {
            return Err(anyhow::anyhow!("Insufficient balance for arbitrage"));
        }

        // Calculate expected profit after fees
        let gross_profit = (optimal_amount as f64 / 1_000_000_000.0) * (sell_price - buy_price);
        let total_fees = self.calculate_arbitrage_fees(optimal_amount, buy_dex, sell_dex)?;
        let net_profit = gross_profit - total_fees;

        // Validate with fee model
        let fee_calculation = self.fee_model.calculate_fees(net_profit, 0.006)?; // Combined DEX fees
        if !fee_calculation.should_execute {
            return Err(anyhow::anyhow!("Below profitability threshold"));
        }

        let opportunity_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        // Create token pair info
        let token_pair = TokenPair {
            base_mint: token_mint.to_string(),
            quote_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL
            base_symbol: "TOKEN".to_string(), // Would be resolved from mint
            quote_symbol: "SOL".to_string(),
        };

        Ok(ArbitrageOpportunity {
            opportunity_id,
            token_pair,
            buy_dex: buy_dex.clone(),
            sell_dex: sell_dex.clone(),
            buy_price,
            sell_price,
            price_difference_percent: price_diff_percent,
            optimal_amount,
            estimated_profit_sol: fee_calculation.net_profit_sol,
            confidence_score: self.calculate_arbitrage_confidence(buy_dex, sell_dex, price_diff_percent),
            execution_priority: self.calculate_execution_priority(fee_calculation.net_profit_sol),
            detected_at: now,
            expires_at: now + chrono::Duration::milliseconds(2000), // 2 second window
        })
    }

    /// Execute arbitrage opportunity using atomic bundles
    pub async fn execute_arbitrage(
        &mut self,
        opportunity: ArbitrageOpportunity,
        recent_blockhash: solana_sdk::hash::Hash,
    ) -> Result<ArbitrageExecution> {
        let start_time = Instant::now();
        let opportunity_id = opportunity.opportunity_id.clone();

        info!("💸 Executing arbitrage: {} -> {} ({:.2}% spread)",
              opportunity.buy_dex.name, opportunity.sell_dex.name, opportunity.price_difference_percent);

        // Build buy instructions (on cheaper DEX)
        let buy_instructions = self.build_arbitrage_instructions(
            &opportunity.token_pair.base_mint,
            &opportunity.token_pair.quote_mint,
            opportunity.optimal_amount,
            &opportunity.buy_dex.name,
            true, // is_buy
        ).await?;

        // Build sell instructions (on more expensive DEX)
        let sell_instructions = self.build_arbitrage_instructions(
            &opportunity.token_pair.base_mint,
            &opportunity.token_pair.quote_mint,
            opportunity.optimal_amount,
            &opportunity.sell_dex.name,
            false, // is_sell
        ).await?;

        // Create atomic arbitrage bundle
        let bundle = self.bundle_manager.create_arbitrage_bundle(
            buy_instructions,
            sell_instructions,
            self.wallet_manager.get_main_keypair(),
            recent_blockhash,
            opportunity.buy_dex.name.clone(),
            opportunity.sell_dex.name.clone(),
            (opportunity.token_pair.base_symbol.clone(), opportunity.token_pair.quote_symbol.clone()),
        ).await?;

        info!("📦 Arbitrage bundle created in {}ms: {}",
              start_time.elapsed().as_millis(), bundle.bundle_id);

        // Submit bundle to Jito
        match self.bundle_manager.submit_bundle(&bundle).await {
            Ok(jito_bundle_id) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                self.stats.opportunities_executed += 1;
                self.stats.cross_dex_opportunities += 1;
                self.stats.total_profit_sol += opportunity.estimated_profit_sol;

                // Update average execution time
                let total_executions = self.stats.opportunities_executed as f64;
                self.stats.average_execution_time_ms =
                    (self.stats.average_execution_time_ms * (total_executions - 1.0) + execution_time as f64) / total_executions;

                info!("✅ Arbitrage executed successfully in {}ms: {} -> Jito: {}",
                      execution_time, opportunity_id, jito_bundle_id);

                Ok(ArbitrageExecution {
                    opportunity_id,
                    bundle_id: jito_bundle_id,
                    execution_time_ms: execution_time,
                    actual_profit_sol: opportunity.estimated_profit_sol,
                    success: true,
                    error_message: None,
                    buy_signature: None,
                    sell_signature: None,
                    slippage_percent: 0.5, // Estimated
                })
            }
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                self.stats.failed_executions += 1;

                error!("❌ Arbitrage execution failed in {}ms: {} - {}",
                       execution_time, opportunity_id, e);

                Ok(ArbitrageExecution {
                    opportunity_id,
                    bundle_id: bundle.bundle_id,
                    execution_time_ms: execution_time,
                    actual_profit_sol: 0.0,
                    success: false,
                    error_message: Some(e.to_string()),
                    buy_signature: None,
                    sell_signature: None,
                    slippage_percent: 0.0,
                })
            }
        }
    }

    /// Calculate optimal trade size for arbitrage
    fn calculate_optimal_trade_size(&self, buy_price: f64, sell_price: f64) -> Result<u64> {
        // Simple calculation - in production would consider liquidity depth
        let price_diff = sell_price - buy_price;
        let optimal_sol_amount = if price_diff > 0.1 { 5.0 } else { 2.0 }; // 2-5 SOL trades

        let max_sol = self.max_position_size_sol.min(optimal_sol_amount);
        Ok((max_sol * 1_000_000_000.0) as u64) // Convert to lamports
    }

    /// Calculate total fees for arbitrage trade
    fn calculate_arbitrage_fees(&self, amount: u64, buy_dex: &DexInfo, sell_dex: &DexInfo) -> Result<f64> {
        let amount_sol = amount as f64 / 1_000_000_000.0;
        let buy_fee = amount_sol * buy_dex.fee_rate;
        let sell_fee = amount_sol * sell_dex.fee_rate;

        // Add gas fees and Jito tips
        let gas_fees = 0.001; // ~0.001 SOL for transactions
        let jito_tip = 0.0001; // Small Jito tip

        Ok(buy_fee + sell_fee + gas_fees + jito_tip)
    }

    /// Calculate confidence score for arbitrage opportunity
    fn calculate_arbitrage_confidence(&self, buy_dex: &DexInfo, sell_dex: &DexInfo, price_diff: f64) -> f64 {
        let mut score: f64 = 0.7; // Base confidence

        // Higher confidence for larger price differences
        if price_diff > 2.0 { score += 0.2; }
        else if price_diff > 1.0 { score += 0.1; }

        // Higher confidence for high-liquidity DEXs
        if buy_dex.min_liquidity_threshold > 1_000_000 && sell_dex.min_liquidity_threshold > 1_000_000 {
            score += 0.1;
        }

        // Lower confidence for high-slippage DEXs
        if buy_dex.typical_slippage > 0.01 || sell_dex.typical_slippage > 0.01 {
            score -= 0.2;
        }

        score.min(1.0).max(0.1)
    }

    /// Calculate execution priority (1-10, 10 = highest)
    fn calculate_execution_priority(&self, profit_sol: f64) -> u8 {
        match profit_sol {
            p if p >= 5.0 => 10,
            p if p >= 2.0 => 8,
            p if p >= 1.0 => 6,
            p if p >= 0.5 => 4,
            _ => 2,
        }
    }

    /// Build arbitrage trade instructions
    async fn build_arbitrage_instructions(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        dex_name: &str,
        _is_buy: bool,
    ) -> Result<Vec<Instruction>> {
        // Check cache for route data
        if let Some(_cached_route) = self.jupiter_executor.get_route_from_cache(
            input_mint,
            output_mint,
            amount,
        ) {
            debug!("Using cached route for arbitrage instruction: {}", dex_name);
        }

        // For now, return simplified instruction
        // In production, this would build real DEX-specific swap instructions
        Ok(vec![
            solana_sdk::system_instruction::transfer(
                &self.wallet_manager.get_main_keypair().pubkey(),
                &self.wallet_manager.get_main_keypair().pubkey(),
                amount,
            ),
        ])
    }

    /// Detect arbitrage opportunities across DEXs (real-time scanning)
    pub async fn detect_opportunities(&mut self) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();

        // Get all unique token pairs from price cache
        let mut token_pairs: HashSet<String> = HashSet::new();
        for key in self.price_cache.keys() {
            if let Some(token) = key.split(':').next() {
                token_pairs.insert(token.to_string());
            }
        }

        // Check each token for arbitrage opportunities
        for token in token_pairs {
            if let Some(opportunity) = self.find_cross_dex_arbitrage(&token).await? {
                opportunities.push(opportunity);
                self.stats.opportunities_detected += 1;
            }
        }

        Ok(opportunities)
    }

    /// Find cross-DEX arbitrage opportunity for a specific token
    async fn find_cross_dex_arbitrage(&self, token_mint: &str) -> Result<Option<ArbitrageOpportunity>> {
        let mut prices = Vec::new();

        // Collect all prices for this token across DEXs
        for (key, price_data) in &self.price_cache {
            if key.starts_with(token_mint) {
                prices.push((price_data.dex_name.clone(), price_data.clone()));
            }
        }

        if prices.len() < 2 {
            return Ok(None); // Need at least 2 DEXs for arbitrage
        }

        // Find best buy and sell prices
        let (min_dex_name, min_price) = prices.iter()
            .min_by(|a, b| a.1.price.partial_cmp(&b.1.price).unwrap())
            .map(|(dex, price)| (dex.clone(), price.clone()))
            .unwrap();

        let (max_dex_name, max_price) = prices.iter()
            .max_by(|a, b| a.1.price.partial_cmp(&b.1.price).unwrap())
            .map(|(dex, price)| (dex.clone(), price.clone()))
            .unwrap();

        if min_dex_name == max_dex_name {
            return Ok(None); // Same DEX
        }

        // Get DexInfo objects for the DEXs
        let min_dex = self.dex_registry.get_dex_by_name(&min_dex_name)
            .ok_or_else(|| anyhow::anyhow!("DEX not found: {}", min_dex_name))?;
        let max_dex = self.dex_registry.get_dex_by_name(&max_dex_name)
            .ok_or_else(|| anyhow::anyhow!("DEX not found: {}", max_dex_name))?;

        // Calculate potential profit
        let price_diff = max_price.price - min_price.price;
        let percentage_diff = (price_diff / min_price.price) * 100.0;

        // Check if opportunity is profitable (>0.1% spread + fees)
        if percentage_diff > 0.2 { // 0.2% minimum for profitability after fees
            let estimated_amount = (self.max_position_size_sol / min_price.price).min(
                (min_price.liquidity as f64 / 4.0) // Use max 25% of liquidity
            );
            let estimated_profit = price_diff * estimated_amount * 0.95; // 95% efficiency

            if estimated_profit > self.min_profit_sol {
                return Ok(Some(ArbitrageOpportunity {
                    opportunity_id: format!("arb_{}_{}", fastrand::u64(..), token_mint),
                    token_pair: TokenPair {
                        base_mint: token_mint.to_string(),
                        quote_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL
                        base_symbol: "TOKEN".to_string(),
                        quote_symbol: "SOL".to_string(),
                    },
                    buy_dex: min_dex.clone(),
                    sell_dex: max_dex.clone(),
                    buy_price: min_price.price,
                    sell_price: max_price.price,
                    price_difference_percent: percentage_diff,
                    optimal_amount: (estimated_amount * 1_000_000_000.0) as u64, // Convert to lamports
                    estimated_profit_sol: estimated_profit,
                    confidence_score: 0.8, // High confidence for real price data
                    execution_priority: 3, // Medium priority
                    detected_at: Utc::now(),
                    expires_at: Utc::now() + chrono::Duration::seconds(5), // 5 second expiry
                }));
            }
        }

        Ok(None)
    }

    /// Get arbitrage engine statistics
    pub fn get_stats(&self) -> ArbitrageStats {
        self.stats.clone()
    }

    /// Get success rate percentage
    pub fn get_success_rate(&self) -> f64 {
        if self.stats.opportunities_detected == 0 {
            0.0
        } else {
            (self.stats.opportunities_executed as f64 / self.stats.opportunities_detected as f64) * 100.0
        }
    }

    /// Get current price cache size
    pub fn get_price_cache_size(&self) -> usize {
        self.price_cache.len()
    }

    /// Clean up old price data
    pub fn cleanup_old_prices(&mut self) {
        let cutoff = Utc::now() - chrono::Duration::seconds(30); // Remove prices older than 30s
        self.price_cache.retain(|_, price| price.last_updated > cutoff);
    }
}