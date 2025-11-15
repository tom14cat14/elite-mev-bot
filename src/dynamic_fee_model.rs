use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicFeeModel {
    pub profit_tiers: Vec<ProfitTier>,
    pub base_multiplier: f64,        // 1.2x minimum profit
    pub default_gas_percentage: f64, // 10% of profit for gas/tip
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitTier {
    pub min_profit_sol: f64,
    pub max_profit_sol: Option<f64>,
    pub profit_multiplier: f64,
    pub gas_percentage: f64,
    pub max_tip_sol: f64,
    pub priority_boost: u8, // Higher = more aggressive
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCalculation {
    pub gross_profit_sol: f64,
    pub net_profit_sol: f64,
    pub total_fees_sol: f64,
    pub gas_tip_sol: f64,
    pub dex_fees_sol: f64,
    pub profit_multiplier: f64,
    pub should_execute: bool,
    pub tier_name: String,
}

impl DynamicFeeModel {
    pub fn new() -> Self {
        Self {
            base_multiplier: 1.2,         // Minimum 1.2x profit
            default_gas_percentage: 0.10, // 10% of profit for gas/tip
            profit_tiers: vec![
                // Small profits: 0.5-1 SOL
                ProfitTier {
                    min_profit_sol: 0.5,
                    max_profit_sol: Some(1.0),
                    profit_multiplier: 1.2,
                    gas_percentage: 0.08, // 8% for smaller profits
                    max_tip_sol: 0.05,    // Max 0.05 SOL tip
                    priority_boost: 3,
                },
                // Medium profits: 1-3 SOL
                ProfitTier {
                    min_profit_sol: 1.0,
                    max_profit_sol: Some(3.0),
                    profit_multiplier: 1.15,
                    gas_percentage: 0.10, // 10% standard
                    max_tip_sol: 0.15,    // Max 0.15 SOL tip
                    priority_boost: 5,
                },
                // Large profits: 3+ SOL
                ProfitTier {
                    min_profit_sol: 3.0,
                    max_profit_sol: None,
                    profit_multiplier: 1.1, // More aggressive on large profits
                    gas_percentage: 0.12,   // 12% for priority
                    max_tip_sol: 0.5,       // Max 0.5 SOL tip
                    priority_boost: 8,
                },
            ],
        }
    }

    /// Calculate fees and determine if opportunity should be executed
    pub fn calculate_fees(
        &self,
        estimated_profit_sol: f64,
        dex_fees_sol: f64,
    ) -> Result<FeeCalculation> {
        let tier = self.get_profit_tier(estimated_profit_sol);

        debug!(
            "Calculating fees for {:.4} SOL profit using tier: {}",
            estimated_profit_sol,
            tier.tier_name()
        );

        // Calculate gas/tip based on tier
        let raw_gas_tip = estimated_profit_sol * tier.gas_percentage;
        let gas_tip_sol = raw_gas_tip.min(tier.max_tip_sol);

        // Total fees
        let total_fees_sol = gas_tip_sol + dex_fees_sol;

        // Net profit after fees
        let net_profit_sol = estimated_profit_sol - total_fees_sol;

        // Check if net profit meets minimum multiplier
        let required_min_profit = total_fees_sol * tier.profit_multiplier;
        let should_execute = net_profit_sol >= required_min_profit;

        let calculation = FeeCalculation {
            gross_profit_sol: estimated_profit_sol,
            net_profit_sol,
            total_fees_sol,
            gas_tip_sol,
            dex_fees_sol,
            profit_multiplier: tier.profit_multiplier,
            should_execute,
            tier_name: tier.tier_name(),
        };

        if should_execute {
            info!(
                "✅ Profitable trade: {:.4} SOL gross → {:.4} SOL net ({}x after fees)",
                calculation.gross_profit_sol,
                calculation.net_profit_sol,
                net_profit_sol / total_fees_sol
            );
        } else {
            debug!(
                "❌ Below threshold: {:.4} SOL net < {:.4} SOL required ({}x)",
                net_profit_sol, required_min_profit, tier.profit_multiplier
            );
        }

        Ok(calculation)
    }

    /// Get the appropriate profit tier for a given profit amount
    fn get_profit_tier(&self, profit_sol: f64) -> &ProfitTier {
        for tier in &self.profit_tiers {
            if profit_sol >= tier.min_profit_sol {
                if let Some(max) = tier.max_profit_sol {
                    if profit_sol <= max {
                        return tier;
                    }
                } else {
                    // No max limit, this is the highest tier
                    return tier;
                }
            }
        }

        // Fallback to smallest tier if below minimum
        &self.profit_tiers[0]
    }

    /// Calculate optimal Jito tip based on competition and profit
    pub fn calculate_jito_tip(
        &self,
        profit_sol: f64,
        current_network_congestion: f64, // 0.0 - 1.0
        competition_level: u8,           // 1-10
    ) -> f64 {
        let tier = self.get_profit_tier(profit_sol);
        let base_tip = profit_sol * tier.gas_percentage;

        // Adjust for network congestion (higher congestion = higher tip)
        let congestion_multiplier = 1.0 + (current_network_congestion * 0.5);

        // Adjust for competition (more competition = higher tip)
        let competition_multiplier = 1.0 + (competition_level as f64 * 0.1);

        let adjusted_tip = base_tip * congestion_multiplier * competition_multiplier;

        // Cap at tier maximum
        adjusted_tip.min(tier.max_tip_sol)
    }

    /// Get priority boost for coordination between bots
    pub fn get_priority_boost(&self, profit_sol: f64) -> u8 {
        self.get_profit_tier(profit_sol).priority_boost
    }
}

impl ProfitTier {
    fn tier_name(&self) -> String {
        match (self.min_profit_sol, self.max_profit_sol) {
            (min, Some(max)) => format!("{:.1}-{:.1} SOL", min, max),
            (min, None) => format!("{:.1}+ SOL", min),
        }
    }
}

impl Default for DynamicFeeModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_profit_calculation() {
        let fee_model = DynamicFeeModel::new();
        let result = fee_model.calculate_fees(0.8, 0.02).unwrap();

        assert_eq!(result.tier_name, "0.5-1.0 SOL");
        assert!(result.gas_tip_sol <= 0.05); // Max tip for small tier
        assert_eq!(result.profit_multiplier, 1.2);
    }

    #[test]
    fn test_large_profit_calculation() {
        let fee_model = DynamicFeeModel::new();
        let result = fee_model.calculate_fees(5.0, 0.05).unwrap();

        assert_eq!(result.tier_name, "3.0+ SOL");
        assert_eq!(result.profit_multiplier, 1.1);
        assert!(result.should_execute);
    }

    #[test]
    fn test_jito_tip_calculation() {
        let fee_model = DynamicFeeModel::new();
        let tip = fee_model.calculate_jito_tip(2.0, 0.8, 7);

        // Base tip is 0.2 (10% of 2.0 SOL)
        // With congestion 0.8 and competition 7, multipliers apply
        // But capped at tier max of 0.15 SOL for medium profits (1-3 SOL)
        assert!(tip > 0.1); // Should be significant
        assert!(tip <= 0.15); // Capped at medium tier max

        // Test with larger profit that allows higher tips
        let high_tip = fee_model.calculate_jito_tip(5.0, 0.8, 7);
        assert!(high_tip > 0.2); // Large profit tier allows higher tips
        assert!(high_tip <= 0.5); // Capped at large tier max
    }
}
