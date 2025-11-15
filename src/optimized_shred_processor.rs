use anyhow::Result;
use solana_entry::entry::Entry;
use solana_sdk::transaction::VersionedTransaction;
use std::time::Instant;
use tracing::{info, warn};

use crate::market_cap_filter::{MarketCapThresholds, ShredStreamTokenFilter, TokenMetrics};
use crate::simd_bincode::SafeSimdBincode;

/// High-performance ShredStream processor with SIMD and upfront filtering
/// Implements Grok's recommendations for 1-3ms performance gains
pub struct OptimizedShredProcessor {
    token_filter: ShredStreamTokenFilter,
    processed_count: u64,
    filtered_count: u64,
    simd_enabled: bool,
    processing_times: Vec<u128>, // microseconds
}

impl OptimizedShredProcessor {
    pub fn new() -> Self {
        let thresholds = MarketCapThresholds {
            minimum_market_cap_usd: 50_000.0, // $50K minimum for processing
            minimum_volume_24h_usd: 10_000.0, // $10K daily volume
            minimum_liquidity_usd: 5_000.0,   // $5K liquidity
            minimum_holder_count: 50,         // 50+ holders
            maximum_age_minutes: 30,          // 30 minute cache
        };

        let simd_enabled = crate::simd_bincode::SimdBincode::is_simd_supported();

        if simd_enabled {
            info!(
                "🚀 SIMD OPTIMIZATION: Enabled ({}) for ~5ms boost",
                crate::simd_bincode::SimdBincode::get_simd_capabilities()
            );
        } else {
            warn!("⚠️  SIMD OPTIMIZATION: Not available - using fallback");
        }

        Self {
            token_filter: ShredStreamTokenFilter::new(thresholds),
            processed_count: 0,
            filtered_count: 0,
            simd_enabled,
            processing_times: Vec::with_capacity(1000),
        }
    }

    /// Process ShredStream entry with maximum optimization
    /// Target: 1-3ms savings through SIMD + upfront filtering
    pub fn process_entry(&mut self, entry_data: &[u8]) -> Result<ProcessingResult> {
        let start_time = Instant::now();

        // 1. SIMD-optimized deserialization
        let entry: Entry = if self.simd_enabled {
            SafeSimdBincode::deserialize(entry_data)?
        } else {
            bincode::deserialize(entry_data)?
        };

        let mut opportunities = Vec::new();
        let mut filtered_transactions = 0;

        // 2. Process transactions with upfront filtering
        for transaction in &entry.transactions {
            // Extract mint from transaction (fast path)
            let mint = self.extract_mint_from_transaction(transaction);

            // Upfront filter using SIMD + market cap
            if !self
                .token_filter
                .should_process_entry(entry_data, mint.as_deref())
            {
                filtered_transactions += 1;
                continue;
            }

            // Process the transaction (only if it passes filters)
            if let Some(opportunity) = self.analyze_transaction(transaction, mint)? {
                opportunities.push(opportunity);
            }
        }

        self.processed_count += 1;
        self.filtered_count += filtered_transactions;

        let processing_time = start_time.elapsed().as_micros();
        self.processing_times.push(processing_time);

        // Keep only last 1000 measurements
        if self.processing_times.len() > 1000 {
            self.processing_times.remove(0);
        }

        Ok(ProcessingResult {
            opportunities,
            transactions_processed: entry.transactions.len() as u64,
            transactions_filtered: filtered_transactions,
            processing_time_us: processing_time,
            simd_used: self.simd_enabled,
        })
    }

    /// Fast mint extraction from transaction data
    #[inline(always)]
    fn extract_mint_from_transaction(&self, transaction: &VersionedTransaction) -> Option<String> {
        // Quick mint extraction for PumpFun transactions
        // This is a simplified version - in practice, you'd parse the instruction data
        if let Ok(tx_data) = SafeSimdBincode::serialize(transaction) {
            // Use SIMD to find PumpFun program ID
            if self
                .token_filter
                .market_cap_filter()
                .is_pumpfun_transaction(&tx_data)
            {
                // Extract mint from instruction accounts (first account after program)
                if let Some(message) = transaction.message.static_account_keys().get(1) {
                    return Some(message.to_string());
                }
            }
        }
        None
    }

    /// Analyze transaction for MEV opportunities
    fn analyze_transaction(
        &self,
        transaction: &VersionedTransaction,
        mint: Option<String>,
    ) -> Result<Option<MEVOpportunity>> {
        if let Some(mint_str) = mint {
            // Check if this is a high-value migration candidate
            if self
                .token_filter
                .market_cap_filter()
                .passes_pre_migration_filter(&mint_str)
            {
                return Ok(Some(MEVOpportunity {
                    mint: mint_str,
                    opportunity_type: OpportunityType::PreMigration,
                    estimated_profit_sol: 0.1, // Placeholder
                    confidence_score: 0.8,
                    processing_priority: 1,
                }));
            }

            // Check bonding curve completion
            if let Some(true) = self
                .token_filter
                .market_cap_filter()
                .is_bonding_curve_complete(&mint_str)
            {
                return Ok(Some(MEVOpportunity {
                    mint: mint_str,
                    opportunity_type: OpportunityType::BondingCurveComplete,
                    estimated_profit_sol: 0.05,
                    confidence_score: 0.9,
                    processing_priority: 2,
                }));
            }
        }

        Ok(None)
    }

    /// Update token metrics in the filter cache
    pub fn update_token_metrics(&self, mint: String, metrics: TokenMetrics) {
        self.token_filter.update_metrics(mint, metrics);
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        let avg_processing_time = if !self.processing_times.is_empty() {
            self.processing_times.iter().sum::<u128>() / self.processing_times.len() as u128
        } else {
            0
        };

        let filter_stats = self.token_filter.get_filter_stats();

        PerformanceStats {
            processed_count: self.processed_count,
            filtered_count: self.filtered_count,
            average_processing_time_us: avg_processing_time,
            simd_enabled: self.simd_enabled,
            filter_efficiency: filter_stats.filter_ratio,
            cache_hit_rate: filter_stats.cache_hit_ratio,
            estimated_time_saved_ms: (self.filtered_count as f64 * 2.0) / 1000.0, // 2μs per filtered transaction
        }
    }

    /// Log performance analysis
    pub fn log_performance_analysis(&self) {
        let stats = self.get_performance_stats();

        info!("⚡ OPTIMIZED SHRED PROCESSOR PERFORMANCE:");
        info!(
            "  • Processed: {} entries | Filtered: {} transactions",
            stats.processed_count, stats.filtered_count
        );
        info!(
            "  • Avg Processing: {}μs | SIMD: {}",
            stats.average_processing_time_us,
            if stats.simd_enabled { "✅" } else { "❌" }
        );
        info!(
            "  • Filter Efficiency: {:.1}% | Cache Hit Rate: {:.1}%",
            stats.filter_efficiency * 100.0,
            stats.cache_hit_rate * 100.0
        );
        info!(
            "  • Est. Time Saved: {:.2}ms",
            stats.estimated_time_saved_ms
        );

        if stats.simd_enabled && stats.filter_efficiency > 0.5 {
            info!("  • Status: 🔥 MAXIMUM OPTIMIZATION ACTIVE");
        } else if stats.simd_enabled || stats.filter_efficiency > 0.3 {
            info!("  • Status: 🎯 GOOD OPTIMIZATION");
        } else {
            warn!("  • Status: ⚠️  OPTIMIZATION NEEDS TUNING");
        }

        // Log filter statistics
        self.token_filter.get_filter_stats().log_performance();
    }
}

#[derive(Debug, Clone)]
pub struct ProcessingResult {
    pub opportunities: Vec<MEVOpportunity>,
    pub transactions_processed: u64,
    pub transactions_filtered: u64,
    pub processing_time_us: u128,
    pub simd_used: bool,
}

#[derive(Debug, Clone)]
pub struct MEVOpportunity {
    pub mint: String,
    pub opportunity_type: OpportunityType,
    pub estimated_profit_sol: f64,
    pub confidence_score: f64,
    pub processing_priority: u8,
}

#[derive(Debug, Clone)]
pub enum OpportunityType {
    PreMigration,
    BondingCurveComplete,
    HighVolume,
    Arbitrage,
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub processed_count: u64,
    pub filtered_count: u64,
    pub average_processing_time_us: u128,
    pub simd_enabled: bool,
    pub filter_efficiency: f64,
    pub cache_hit_rate: f64,
    pub estimated_time_saved_ms: f64,
}

impl Default for OptimizedShredProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let processor = OptimizedShredProcessor::new();
        println!("SIMD Enabled: {}", processor.simd_enabled);

        let stats = processor.get_performance_stats();
        assert_eq!(stats.processed_count, 0);
    }

    #[test]
    fn test_performance_stats() {
        let processor = OptimizedShredProcessor::new();
        let stats = processor.get_performance_stats();

        assert!(stats.average_processing_time_us == 0); // No processing yet
        println!("Initial stats: {:?}", stats);
    }
}
