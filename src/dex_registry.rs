use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexInfo {
    pub name: String,
    pub program_id: Pubkey,
    pub fee_rate: f64,
    pub supports_arbitrage: bool,
    pub supports_sandwich: bool,
    pub min_liquidity_threshold: u64,
    pub typical_slippage: f64,
}

#[derive(Debug, Clone)]
pub struct DexRegistry {
    pub dexs: HashMap<String, DexInfo>,
    pub program_id_to_name: HashMap<Pubkey, String>,
}

impl DexRegistry {
    pub fn new() -> Self {
        let mut dexs = HashMap::new();
        let mut program_id_to_name = HashMap::new();

        // Core DEXs with multiple pool types
        let dex_configs = vec![
            // Raydium - Multiple Pool Types
            (
                "Raydium_AMM_V4",
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
                0.0025,
                true,
                true,
                1_000_000,
                0.001,
            ), // Legacy AMM v4
            (
                "Raydium_CLMM",
                "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
                0.0025,
                true,
                true,
                1_000_000,
                0.001,
            ), // Concentrated Liquidity
            (
                "Raydium_CPMM",
                "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
                0.0025,
                true,
                true,
                1_000_000,
                0.001,
            ), // New Standard AMM
            (
                "Raydium_Stable",
                "5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h",
                0.001,
                true,
                true,
                10_000_000,
                0.0005,
            ), // Stable AMM
            // Orca - Multiple Pool Types
            (
                "Orca_Legacy",
                "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP",
                0.003,
                true,
                true,
                5_000_000,
                0.002,
            ), // Legacy CPMM (placeholder)
            (
                "Orca_Whirlpools",
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
                0.003,
                true,
                true,
                5_000_000,
                0.002,
            ), // Concentrated Liquidity
            // Jupiter
            (
                "Jupiter",
                "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",
                0.001,
                true,
                false,
                0,
                0.001,
            ),
            // Serum
            (
                "Serum",
                "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
                0.0022,
                true,
                true,
                10_000_000,
                0.003,
            ),
            // Meteora - Multiple Pool Types
            (
                "Meteora_DAMM_V1",
                "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB",
                0.003,
                true,
                true,
                2_000_000,
                0.002,
            ), // Dynamic AMM v1
            (
                "Meteora_DLMM",
                "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
                0.003,
                true,
                true,
                2_000_000,
                0.001,
            ), // Dynamic Liquidity Market Maker
            (
                "Meteora_DAMM_V2",
                "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG",
                0.003,
                true,
                true,
                2_000_000,
                0.002,
            ), // Dynamic AMM v2
            // Lending/Liquidation Protocols
            (
                "Drift",
                "dRiftyHA39MWEi3m9aunc5MjRF1JYuBsbn6VPcn33UH",
                0.001,
                false,
                false,
                100_000,
                0.005,
            ),
            (
                "Solend",
                "So1endDq2YkqhipRh3WViPaJ8LEs9MDCP2xuU4jBEAAg",
                0.001,
                false,
                false,
                100_000,
                0.005,
            ),
            (
                "Kamino",
                "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD",
                0.001,
                false,
                false,
                100_000,
                0.005,
            ),
            (
                "Hubble",
                "HEvSKofvBgfaexv23kMabbYqxasxU3mQ4ibBMEmJWHny",
                0.002,
                false,
                false,
                50_000,
                0.01,
            ),
            // Meme Token Platforms - MEV yes, arbitrage no until migration
            (
                "PumpFun",
                "PumpFunP4PfMpqd7KsAEL7NKPhpq6M4yDmMRr2tH6gN",
                0.01,
                false,
                true,
                100_000,
                0.015,
            ), // MEV sandwich ok, no arb
            // Additional DEXs for comprehensive coverage
            (
                "Aldrin",
                "AMM55ShdkoGRB5jVYPjWziwk8m5MpwyDgsMWHaMSQWH6",
                0.003,
                true,
                true,
                1_000_000,
                0.003,
            ),
            (
                "Saros",
                "SSwpkEEWHvCXCNWnMYXVW7gCYDXkF4aQMxKdpEqrZks",
                0.0025,
                true,
                true,
                500_000,
                0.003,
            ),
            (
                "Crema",
                "6MLxLqiXaaSUpkgMnWDTuejNZEz3kE7k2woyHGVFw319",
                0.003,
                true,
                true,
                1_000_000,
                0.002,
            ),
            (
                "Cropper",
                "CTMAxxk34HjKWxQ3QLZQA1EQdxtjbYGP4Qjrw7nTn8bM",
                0.003,
                true,
                true,
                500_000,
                0.004,
            ),
            (
                "Lifinity",
                "EewxydAPCCVuNEyrVN68PuSYdQ7wKn27V9Gjeoi8dy3S",
                0.0025,
                true,
                true,
                2_000_000,
                0.002,
            ),
            (
                "Marinade",
                "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
                0.001,
                false,
                false,
                10_000_000,
                0.001,
            ),
            (
                "Fluxbeam",
                "FLUXBmPhT3Fd1EDVFdg46YREqHBeNypn1h4EbnTzWERX",
                0.003,
                true,
                true,
                200_000,
                0.005,
            ),
            // Additional DEXs from user
            (
                "Humidifi",
                "9H6tua7jkLhdm3w8BvgpTn5LZNU7g4ZynDmCiNN3q6Rp",
                0.003,
                true,
                true,
                1_000_000,
                0.003,
            ),
            (
                "TesseraV",
                "TessVdML9pBGgG9yGks7o4HewRaXVAMuoVj4x83GLQH",
                0.003,
                true,
                true,
                1_000_000,
                0.003,
            ),
            (
                "PumpFun_V2",
                "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
                0.01,
                false,
                true,
                100_000,
                0.015,
            ), // MEV sandwich ok, no arb
            (
                "PumpSwap",
                "GMk6j2defJhS7F194toqmJNFNhAkbDXhYJo5oR3Rpump",
                0.003,
                true,
                true,
                100_000,
                0.01,
            ), // MIGRATED tokens: both MEV & arb ok
            // Major Launchpad Competitors (2025) - MEV yes, arbitrage no until migration
            (
                "Moonshot",
                "MOONSHOT_PROGRAM_ID_PLACEHOLDER",
                0.01,
                false,
                true,
                100_000,
                0.015,
            ), // MEV sandwich ok, no arb
            (
                "BonkFun",
                "BONKFUN_PROGRAM_ID_PLACEHOLDER",
                0.01,
                false,
                true,
                100_000,
                0.015,
            ), // MEV sandwich ok, no arb
        ];

        for (
            name,
            program_id_str,
            fee_rate,
            supports_arbitrage,
            supports_sandwich,
            min_liquidity,
            typical_slippage,
        ) in dex_configs
        {
            if let Ok(program_id) = Pubkey::from_str(program_id_str) {
                let dex_info = DexInfo {
                    name: name.to_string(),
                    program_id,
                    fee_rate,
                    supports_arbitrage,
                    supports_sandwich,
                    min_liquidity_threshold: min_liquidity,
                    typical_slippage,
                };

                dexs.insert(name.to_string(), dex_info);
                program_id_to_name.insert(program_id, name.to_string());
            }
        }

        Self {
            dexs,
            program_id_to_name,
        }
    }

    pub fn get_dex_by_program_id(&self, program_id: &Pubkey) -> Option<&DexInfo> {
        if let Some(name) = self.program_id_to_name.get(program_id) {
            self.dexs.get(name)
        } else {
            None
        }
    }

    pub fn get_dex_by_name(&self, name: &str) -> Option<&DexInfo> {
        self.dexs.get(name)
    }

    pub fn is_dex_program(&self, program_id: &Pubkey) -> bool {
        self.program_id_to_name.contains_key(program_id)
    }

    pub fn is_dex_program_str(&self, program_id_str: &str) -> bool {
        if let Ok(program_id) = Pubkey::from_str(program_id_str) {
            self.is_dex_program(&program_id)
        } else {
            false
        }
    }

    pub fn get_arbitrage_pairs(&self) -> Vec<(&DexInfo, &DexInfo)> {
        let arbitrage_dexs: Vec<&DexInfo> = self
            .dexs
            .values()
            .filter(|dex| dex.supports_arbitrage)
            .collect();

        let mut pairs = Vec::new();
        for i in 0..arbitrage_dexs.len() {
            for j in (i + 1)..arbitrage_dexs.len() {
                pairs.push((arbitrage_dexs[i], arbitrage_dexs[j]));
            }
        }
        pairs
    }

    pub fn get_sandwich_targets(&self) -> Vec<&DexInfo> {
        self.dexs
            .values()
            .filter(|dex| dex.supports_sandwich)
            .collect()
    }

    pub fn get_all_program_ids(&self) -> Vec<String> {
        self.program_id_to_name
            .keys()
            .map(|pubkey| pubkey.to_string())
            .collect()
    }

    pub fn calculate_total_fees(&self, dex1_name: &str, dex2_name: &str, amount: f64) -> f64 {
        let fee1 = self
            .dexs
            .get(dex1_name)
            .map(|d| d.fee_rate)
            .unwrap_or(0.003);
        let fee2 = self
            .dexs
            .get(dex2_name)
            .map(|d| d.fee_rate)
            .unwrap_or(0.003);
        amount * (fee1 + fee2)
    }

    pub fn get_optimal_route(&self, input_amount: f64, target_profit: f64) -> Vec<String> {
        // Simple routing logic - prioritize low-fee DEXs for arbitrage
        let mut low_fee_dexs: Vec<_> = self
            .dexs
            .iter()
            .filter(|(_, info)| info.supports_arbitrage && info.fee_rate < 0.003)
            .collect();

        low_fee_dexs.sort_by(|a, b| a.1.fee_rate.partial_cmp(&b.1.fee_rate).unwrap());

        low_fee_dexs
            .into_iter()
            .take(3)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl Default for DexRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dex_registry_creation() {
        let registry = DexRegistry::new();
        assert!(registry.dexs.len() > 10);
        assert!(registry.get_dex_by_name("Raydium_AMM_V4").is_some());
        assert!(registry.get_dex_by_name("Orca_Legacy").is_some());
    }

    #[test]
    fn test_program_id_lookup() {
        let registry = DexRegistry::new();
        let raydium_id = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap();
        assert!(registry.is_dex_program(&raydium_id));
        assert_eq!(
            registry.get_dex_by_program_id(&raydium_id).unwrap().name,
            "Raydium_AMM_V4"
        );
    }

    #[test]
    fn test_arbitrage_pairs() {
        let registry = DexRegistry::new();
        let pairs = registry.get_arbitrage_pairs();
        assert!(pairs.len() > 0);
    }
}
