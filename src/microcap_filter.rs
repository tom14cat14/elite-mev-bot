use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Adaptive parameters for different token market cap ranges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveParams {
    pub timeout_ms: u64,
    pub max_concurrent: usize,
    pub min_liquidity_sol: f64,
    pub target_impact_pct: f64,
    pub position_multiplier: f64,
    pub enable_liquidations: bool,
}

/// Pre-migration tuning parameters (speed + volume focused)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreMigrationParams {
    pub volatility_threshold: f64,     // Min price change % in 24h (can be disabled)
    pub age_threshold_hours: i64,      // Max age for pre-migration detection
    pub liquidity_aggressiveness: f64, // Multiplier for position sizing
    pub impact_multiplier: f64,        // Expected impact enhancement
    pub metadata_keywords: Vec<String>, // Keywords indicating pre-migration (can be empty)
    pub min_volume_usd_24h: f64,       // NEW: Minimum volume requirement for trading activity
    pub min_profit_margin_sol: f64,    // NEW: Minimum profit margin for risk compensation
    pub launch_window_minutes: i64,    // NEW: Only monitor first X minutes after launch
    pub max_age_minutes: i64,          // NEW: Maximum age in minutes (ultra-precise timing)
}

/// PumpFun-specific gas and tip optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunConfig {
    pub base_gas_fee: f64,             // Base gas fee in SOL
    pub priority_tip_sol: f64,         // Priority tip for faster execution
    pub max_tip_sol: f64,              // Maximum tip we're willing to pay
    pub gas_multiplier_premigration: f64, // Gas multiplier for pre-migration urgency
}

/// Specialized filter for micro-cap tokens (< 1M market cap)
/// Optimized for adaptive strategy with pre-migration parameter tuning
pub struct MicroCapFilter {
    min_liquidity_sol: f64,
    max_market_cap_usd: f64,
    max_position_size_sol: f64,
    target_price_impact_pct: f64,
    known_premigration_tokens: HashMap<String, TokenInfo>,
    // Adaptive parameters for different market cap ranges
    premigration_params: AdaptiveParams,    // <100K tokens (ultra-aggressive)
    standard_params: AdaptiveParams,        // 100K-1M tokens (balanced)
    premigration_tuning: PreMigrationParams, // Tunable pre-migration detection
    pumpfun_config: PumpFunConfig,          // PumpFun gas/tip optimization
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub mint: String,
    pub symbol: String,
    pub market_cap_usd: f64,
    pub liquidity_sol: f64,
    pub is_premigration: bool,
    pub last_updated: i64,
    pub volatility_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MicroCapOpportunity {
    pub token_mint: String,
    pub symbol: String,
    pub current_price: f64,
    pub market_cap_usd: f64,
    pub liquidity_sol: f64,
    pub expected_price_impact_pct: f64,
    pub recommended_position_sol: f64,
    pub estimated_profit_sol: f64,     // NEW: For profit margin validation
    pub is_premigration: bool,
    pub confidence_score: f64,
    pub volatility_score: f64,
}

impl MicroCapFilter {
    /// Create new unified micro-cap filter with adaptive parameters
    pub fn new() -> Self {
        Self {
            min_liquidity_sol: 2.0,        // Minimum 2 SOL liquidity
            max_market_cap_usd: 1_000_000.0, // Max 1M market cap (hard limit for efficiency)
            max_position_size_sol: 1.5,    // Max 1.5 SOL per position
            target_price_impact_pct: 3.0,  // Target 3%+ price impact
            known_premigration_tokens: HashMap::new(),

            // Pre-migration parameters (<100K tokens) - HIGHLY TUNABLE
            premigration_params: AdaptiveParams {
                timeout_ms: 700,           // Ultra-fast execution
                max_concurrent: 4,         // Lower concurrency for high impact
                min_liquidity_sol: 0.8,    // Aggressive liquidity requirement
                target_impact_pct: 8.0,    // High impact target
                position_multiplier: 1.2,  // Larger positions for max impact
                enable_liquidations: false, // Sandwich only for volatility
            },

            // Standard micro-cap parameters (100K-1M tokens)
            standard_params: AdaptiveParams {
                timeout_ms: 1200,          // Balanced execution speed
                max_concurrent: 8,         // Higher concurrency
                min_liquidity_sol: 2.0,    // Standard liquidity requirement
                target_impact_pct: 4.0,    // Moderate impact target
                position_multiplier: 1.0,  // Standard position sizing
                enable_liquidations: true, // Full strategy suite
            },

            // Pre-migration detection tuning - SPEED + VOLUME FOCUSED
            premigration_tuning: PreMigrationParams {
                volatility_threshold: 30.0,     // 30%+ price change in 24h
                age_threshold_hours: 48,        // <48 hours old
                liquidity_aggressiveness: 1.5,  // 50% more aggressive sizing
                impact_multiplier: 2.0,         // 2x expected impact
                metadata_keywords: vec![
                    "migration".to_string(),
                    "migrate".to_string(),
                    "v2".to_string(),
                    "upgrade".to_string(),
                    "launch".to_string(),
                    "presale".to_string(),
                ],
                min_volume_usd_24h: 1000.0,     // $1K minimum volume for trading activity
                min_profit_margin_sol: 0.05,    // 0.05 SOL minimum profit (standard)
                launch_window_minutes: 60,      // 60 minutes window (standard)
                max_age_minutes: 1440,          // 24 hours in minutes
            },

            // PumpFun optimization for pre-migration targeting
            pumpfun_config: PumpFunConfig {
                base_gas_fee: 0.001,            // 0.001 SOL base gas
                priority_tip_sol: 0.003,        // 0.003 SOL priority tip (speed)
                max_tip_sol: 0.01,              // 0.01 SOL maximum tip
                gas_multiplier_premigration: 1.5, // 1.5x gas for pre-migration urgency
            },
        }
    }

    /// Create new micro-cap filter with custom market cap limit
    pub fn new_with_limit(max_market_cap_usd: f64) -> Self {
        let base = Self::new();
        Self {
            max_market_cap_usd,
            ..base
        }
    }

    /// Create filter optimized for pure pre-migration speed + volume strategy
    pub fn new_premigration() -> Self {
        let mut base = Self::new();
        base.max_market_cap_usd = 90_000.0; // Max 90K (PumpFun migration at 98.8K)
        base.premigration_params = AdaptiveParams {
            timeout_ms: 400,           // ULTRA-FAST for speed advantage
            max_concurrent: 3,         // Quality over quantity
            min_liquidity_sol: 1.0,    // Ensure executability
            target_impact_pct: 8.0,    // High impact for pre-migration
            position_multiplier: 1.0,  // Standard position (risk managed)
            enable_liquidations: false, // Sandwich ONLY for speed
        };
        base.premigration_tuning = PreMigrationParams {
            volatility_threshold: 30.0,     // Moderate volatility (volume more important)
            age_threshold_hours: 6,         // <6 hours old (brand new focus)
            liquidity_aggressiveness: 1.5,  // 1.5x more aggressive sizing
            impact_multiplier: 2.0,         // 2x expected impact
            metadata_keywords: vec![], // REMOVED: Focus on behavioral data only
            min_volume_usd_24h: 3000.0,     // $3K minimum volume (moderate for new tokens)
            min_profit_margin_sol: 0.10,    // 0.10 SOL minimum (moderate for new tokens)
            launch_window_minutes: 5,       // 5 minutes window (Grok's recommendation: 1-5 min post-launch)
            max_age_minutes: 360,           // 6 hours in minutes
        };
        base
    }

    /// Create filter for pure speed + volume pre-migration targeting
    pub fn new_speed_volume_premigration() -> Self {
        Self {
            min_liquidity_sol: 1.0,        // Minimum 1.0 SOL liquidity
            max_market_cap_usd: 1_000_000.0, // Max 1M (catch any new launch)
            max_position_size_sol: 1.5,    // Max 1.5 SOL per position
            target_price_impact_pct: 8.0,  // High impact target
            known_premigration_tokens: HashMap::new(),

            // Speed-focused parameters
            premigration_params: AdaptiveParams {
                timeout_ms: 400,           // ULTRA-FAST execution
                max_concurrent: 3,         // Quality focus
                min_liquidity_sol: 1.0,    // Ensure execution
                target_impact_pct: 8.0,    // High impact
                position_multiplier: 1.0,  // Risk-managed position
                enable_liquidations: false, // Sandwich ONLY
            },

            // Standard parameters (not used in pure pre-migration mode)
            standard_params: AdaptiveParams {
                timeout_ms: 1200,
                max_concurrent: 8,
                min_liquidity_sol: 2.0,
                target_impact_pct: 4.0,
                position_multiplier: 1.0,
                enable_liquidations: true,
            },

            // Speed + Volume tuning - PURE PRE-MIGRATION FOCUS
            premigration_tuning: PreMigrationParams {
                volatility_threshold: 0.0,       // DISABLED: Volume matters more
                age_threshold_hours: 6,          // <6 hours old (brand new)
                liquidity_aggressiveness: 1.5,   // 1.5x aggressive
                impact_multiplier: 2.0,          // 2x impact
                metadata_keywords: vec![],       // DISABLED: No keyword checking
                min_volume_usd_24h: 1000.0,      // $1K minimum volume (catch early activity)
                min_profit_margin_sol: 0.15,     // 0.15 SOL minimum (3x risk premium)
                launch_window_minutes: 5,        // 5 MINUTE WINDOW (Grok: 1-5 min post-launch optimal)
                max_age_minutes: 1,              // EXACTLY 1 minute maximum
            },

            // PumpFun optimization for ULTRA-FAST pre-migration execution
            pumpfun_config: PumpFunConfig {
                base_gas_fee: 0.0005,           // 0.0005 SOL base gas (minimal)
                priority_tip_sol: 0.005,        // 0.005 SOL priority tip (aggressive)
                max_tip_sol: 0.02,              // 0.02 SOL maximum tip (speed at all costs)
                gas_multiplier_premigration: 2.0, // 2x gas for pre-migration urgency
            },
        }
    }

    /// EARLY FILTER: Quick market cap check to save processing (efficiency optimization)
    pub fn quick_market_cap_filter(&self, tx_data: &Value) -> Result<bool> {
        // Fast market cap estimation for early filtering
        if let Some(market_cap) = tx_data.get("market_cap").and_then(|mc| mc.as_f64()) {
            return Ok(market_cap <= self.max_market_cap_usd);
        }

        // If no market cap data, estimate from supply and price (if available)
        if let Some(supply) = tx_data.get("supply").and_then(|s| s.as_f64()) {
            if let Some(price) = tx_data.get("price_usd").and_then(|p| p.as_f64()) {
                let estimated_cap = supply * price;
                return Ok(estimated_cap <= self.max_market_cap_usd);
            }
        }

        // If we can't determine market cap quickly, allow through for full analysis
        // This prevents missing opportunities due to incomplete data
        Ok(true)
    }

    /// Get adaptive parameters for a given market cap
    pub fn get_adaptive_params(&self, market_cap_usd: f64, is_premigration: bool) -> &AdaptiveParams {
        if is_premigration || market_cap_usd < 100_000.0 {
            &self.premigration_params
        } else {
            &self.standard_params
        }
    }

    /// Get pre-migration tuning parameters for optimization
    pub fn get_premigration_tuning(&self) -> &PreMigrationParams {
        &self.premigration_tuning
    }

    /// Update pre-migration parameters for live tuning
    pub fn update_premigration_params(&mut self, params: AdaptiveParams) {
        info!("🔧 Updated pre-migration parameters: timeout={}ms, concurrent={}, impact={:.1}%",
              params.timeout_ms, params.max_concurrent, params.target_impact_pct);
        self.premigration_params = params;
    }

    /// Update pre-migration tuning for live optimization
    pub fn update_premigration_tuning(&mut self, tuning: PreMigrationParams) {
        info!("🎯 Updated pre-migration tuning: volatility={:.1}%, age={}h, aggressiveness={:.1}x",
              tuning.volatility_threshold, tuning.age_threshold_hours, tuning.liquidity_aggressiveness);
        self.premigration_tuning = tuning;
    }

    /// Analyze if a token is suitable for micro-cap MEV strategy with adaptive parameters
    pub async fn analyze_token(&self, tx_data: &Value) -> Result<Option<MicroCapOpportunity>> {
        // Extract token information from transaction
        let token_info = match self.extract_token_info(tx_data) {
            Ok(info) => info,
            Err(_) => return Ok(None),
        };

        // Check market cap filter
        if token_info.market_cap_usd > self.max_market_cap_usd {
            debug!("Token {} exceeds market cap limit: ${:.0}M",
                   token_info.symbol, token_info.market_cap_usd / 1_000_000.0);
            return Ok(None);
        }

        // Check liquidity requirements
        if token_info.liquidity_sol < self.min_liquidity_sol {
            debug!("Token {} insufficient liquidity: {:.2} SOL",
                   token_info.symbol, token_info.liquidity_sol);
            return Ok(None);
        }

        // Calculate expected price impact with 4 SOL
        let price_impact = self.calculate_price_impact(
            &token_info,
            self.max_position_size_sol
        );

        // Filter for meaningful price impact
        if price_impact < self.target_price_impact_pct {
            debug!("Token {} insufficient price impact: {:.1}%",
                   token_info.symbol, price_impact);
            return Ok(None);
        }

        // Calculate optimal position size for target impact
        let optimal_position = self.calculate_optimal_position_size(
            &token_info,
            self.target_price_impact_pct
        );

        // Bonus scoring for pre-migration tokens
        let premigration_bonus = if token_info.is_premigration { 0.3 } else { 0.0 };

        // Calculate confidence score
        let confidence_score = self.calculate_confidence_score(
            &token_info,
            price_impact,
            optimal_position
        ) + premigration_bonus;

        // Calculate estimated profit for validation
        let estimated_profit = self.calculate_estimated_profit(&token_info, optimal_position);

        // CRITICAL: Validate profit margin for risk compensation (pure pre-migration focus)
        if !self.validate_profit_margin(estimated_profit) {
            return Ok(None); // REJECT: Insufficient profit margin for high-risk strategy
        }

        let opportunity = MicroCapOpportunity {
            token_mint: token_info.mint.clone(),
            symbol: token_info.symbol.clone(),
            current_price: self.estimate_current_price(&token_info),
            market_cap_usd: token_info.market_cap_usd,
            liquidity_sol: token_info.liquidity_sol,
            expected_price_impact_pct: price_impact,
            recommended_position_sol: optimal_position.min(self.max_position_size_sol),
            estimated_profit_sol: estimated_profit,
            is_premigration: token_info.is_premigration,
            confidence_score,
            volatility_score: token_info.volatility_score,
        };

        info!("🎯 MICRO-CAP OPPORTUNITY: {} | MC: ${:.0}K | Impact: {:.1}% | Position: {:.2} SOL",
              opportunity.symbol,
              opportunity.market_cap_usd / 1000.0,
              opportunity.expected_price_impact_pct,
              opportunity.recommended_position_sol);

        Ok(Some(opportunity))
    }

    /// Extract token information from transaction data
    fn extract_token_info(&self, tx_data: &Value) -> Result<TokenInfo> {
        // Parse transaction to extract token mint and trading data
        let mint = tx_data
            .get("mint")
            .and_then(|m| m.as_str())
            .ok_or_else(|| anyhow::anyhow!("No token mint found"))?;

        // Get symbol from transaction or use mint
        let symbol = tx_data
            .get("symbol")
            .and_then(|s| s.as_str())
            .unwrap_or(mint)
            .to_string();

        // Estimate market cap from transaction data
        let market_cap = self.estimate_market_cap(tx_data)?;

        // Estimate liquidity from pool data
        let liquidity = self.estimate_liquidity(tx_data)?;

        // Check if token appears to be pre-migration
        let is_premigration = self.detect_premigration_characteristics(tx_data);

        // Calculate volatility score from recent price action
        let volatility_score = self.calculate_volatility_score(tx_data);

        Ok(TokenInfo {
            mint: mint.to_string(),
            symbol,
            market_cap_usd: market_cap,
            liquidity_sol: liquidity,
            is_premigration,
            last_updated: chrono::Utc::now().timestamp(),
            volatility_score,
        })
    }

    /// Calculate expected price impact for given position size
    fn calculate_price_impact(&self, token_info: &TokenInfo, position_sol: f64) -> f64 {
        // Simple price impact model: impact = position / liquidity * 100
        // More sophisticated models would consider order book depth, slippage curves, etc.
        let base_impact = (position_sol / token_info.liquidity_sol) * 100.0;

        // Apply micro-cap multiplier (less efficient markets)
        let microcap_multiplier = if token_info.market_cap_usd < 100_000.0 {
            2.5 // 250% multiplier for <100K market cap
        } else if token_info.market_cap_usd < 500_000.0 {
            1.8 // 180% multiplier for <500K market cap
        } else {
            1.4 // 140% multiplier for <1M market cap
        };

        // Apply pre-migration bonus (higher volatility)
        let premigration_multiplier = if token_info.is_premigration { 1.5 } else { 1.0 };

        base_impact * microcap_multiplier * premigration_multiplier
    }

    /// Calculate optimal position size for target price impact
    fn calculate_optimal_position_size(&self, token_info: &TokenInfo, target_impact_pct: f64) -> f64 {
        // Reverse engineer position size from target impact
        let base_position = (target_impact_pct * token_info.liquidity_sol) / 100.0;

        // Apply efficiency adjustments
        let microcap_adjustment = if token_info.market_cap_usd < 100_000.0 {
            0.4 // Need less capital for same impact
        } else if token_info.market_cap_usd < 500_000.0 {
            0.55
        } else {
            0.7
        };

        let optimal = base_position * microcap_adjustment;

        // Ensure within reasonable bounds for 4 SOL strategy
        optimal.max(0.3).min(self.max_position_size_sol)
    }

    /// Calculate confidence score for opportunity
    fn calculate_confidence_score(&self, token_info: &TokenInfo, price_impact: f64, position_size: f64) -> f64 {
        let mut score = 0.0;

        // Price impact score (higher impact = higher score, up to a point)
        score += if price_impact > 10.0 {
            0.9 // Excellent impact
        } else if price_impact > 5.0 {
            0.7 // Good impact
        } else if price_impact > 2.0 {
            0.4 // Moderate impact
        } else {
            0.1 // Low impact
        };

        // Market cap score (smaller = better for our strategy)
        score += if token_info.market_cap_usd < 100_000.0 {
            0.8 // Micro micro-cap
        } else if token_info.market_cap_usd < 500_000.0 {
            0.6 // Small micro-cap
        } else {
            0.3 // Large micro-cap
        };

        // Liquidity score (enough to trade but not too much)
        let liquidity_ratio = position_size / token_info.liquidity_sol;
        score += if liquidity_ratio > 0.3 && liquidity_ratio < 0.8 {
            0.7 // Sweet spot
        } else if liquidity_ratio > 0.1 && liquidity_ratio < 1.0 {
            0.4 // Acceptable
        } else {
            0.1 // Too little or too much impact
        };

        // Volatility bonus
        score += token_info.volatility_score * 0.3;

        score.min(1.0) // Cap at 1.0
    }

    /// Estimate market cap from transaction data
    fn estimate_market_cap(&self, tx_data: &Value) -> Result<f64> {
        // Try to extract market cap from various sources in transaction
        if let Some(market_cap) = tx_data.get("market_cap").and_then(|mc| mc.as_f64()) {
            return Ok(market_cap);
        }

        // Estimate from supply and price if available
        if let Some(supply) = tx_data.get("supply").and_then(|s| s.as_f64()) {
            if let Some(price) = tx_data.get("price_usd").and_then(|p| p.as_f64()) {
                return Ok(supply * price);
            }
        }

        // Default estimation for unknown tokens (assume micro-cap)
        Ok(250_000.0)
    }

    /// Estimate liquidity from transaction data
    fn estimate_liquidity(&self, tx_data: &Value) -> Result<f64> {
        // Try to extract liquidity from pool data
        if let Some(liquidity) = tx_data.get("liquidity_sol").and_then(|l| l.as_f64()) {
            return Ok(liquidity);
        }

        // Estimate from reserves if available
        if let Some(sol_reserve) = tx_data.get("sol_reserve").and_then(|r| r.as_f64()) {
            return Ok(sol_reserve);
        }

        // Default conservative estimate
        Ok(5.0)
    }

    /// Detect tokens in 1-MINUTE launch window using PRECISE timing + volume
    fn detect_premigration_characteristics(&self, tx_data: &Value) -> bool {
        let tuning = &self.premigration_tuning;
        let now = chrono::Utc::now().timestamp();

        // PRIMARY: Check volume requirement - MANDATORY for trading activity
        let volume_24h = if let Some(vol) = tx_data.get("volume_usd_24h").and_then(|v| v.as_f64()) {
            if vol < tuning.min_volume_usd_24h {
                debug!("1-MIN WINDOW REJECTED: Volume ${:.0} < ${:.0} minimum",
                       vol, tuning.min_volume_usd_24h);
                return false; // REJECT: Insufficient volume
            }
            vol
        } else {
            debug!("1-MIN WINDOW REJECTED: No volume data available");
            return false; // REJECT: No volume data
        };

        // CRITICAL: Ultra-precise timing check - ONLY first X minutes after launch
        if let Some(created) = tx_data.get("created_timestamp").and_then(|t| t.as_i64()) {
            let age_minutes = (now - created) / 60;

            if age_minutes > tuning.max_age_minutes {
                debug!("1-MIN WINDOW REJECTED: Age {}m > {}m maximum (RUG AVOIDANCE)",
                       age_minutes, tuning.max_age_minutes);
                return false; // REJECT: Outside safe window
            }

            // GROK'S RECOMMENDATION: 1-5 minute post-launch window (pre-migration PumpFun)
            if age_minutes >= 1 && age_minutes <= tuning.launch_window_minutes {
                debug!("PUMPFUN WINDOW ACCEPTED: Age {}m in 1-{}m post-launch window + Volume ${:.0}",
                       age_minutes, tuning.launch_window_minutes, volume_24h);
                return true; // ACCEPT: In 1-5 minute post-launch window + has volume
            } else if age_minutes < 1 {
                debug!("PUMPFUN WINDOW REJECTED: Age {}m < 1m (too early, avoid initial frenzy)",
                       age_minutes);
                return false; // REJECT: Too early (< 1 minute)
            } else {
                debug!("PUMPFUN WINDOW REJECTED: Age {}m > {}m launch window (avoid post-pump rugs)",
                       age_minutes, tuning.launch_window_minutes);
                return false; // REJECT: Outside launch window (> 5 minutes)
            }
        }

        // FALLBACK: If no timestamp but very high volume, allow with caution
        if let Some(volume_24h) = tx_data.get("volume_usd_24h").and_then(|v| v.as_f64()) {
            if volume_24h > tuning.min_volume_usd_24h * 5.0 {
                debug!("1-MIN WINDOW ACCEPTED: Extreme volume ${:.0} (no timestamp)", volume_24h);
                return true; // ACCEPT: Extreme volume suggests very new
            }
        }

        debug!("1-MIN WINDOW REJECTED: No timestamp and insufficient volume");
        false
    }

    /// Validate minimum profit margin for risk compensation
    fn validate_profit_margin(&self, estimated_profit_sol: f64) -> bool {
        let tuning = &self.premigration_tuning;
        if estimated_profit_sol >= tuning.min_profit_margin_sol {
            debug!("Profit margin ACCEPTED: {:.4} SOL >= {:.4} SOL minimum",
                   estimated_profit_sol, tuning.min_profit_margin_sol);
            true
        } else {
            debug!("Profit margin REJECTED: {:.4} SOL < {:.4} SOL minimum (risk compensation)",
                   estimated_profit_sol, tuning.min_profit_margin_sol);
            false
        }
    }

    /// Calculate estimated profit from position size and expected impact (PumpFun optimized)
    fn calculate_estimated_profit(&self, token_info: &TokenInfo, position_sol: f64) -> f64 {
        // Basic profit calculation: position_size * expected_impact * profit_multiplier
        let base_profit = position_sol * (self.target_price_impact_pct / 100.0) * 0.75; // 75% capture rate (better on PumpFun)

        // Pre-migration bonus
        let premigration_multiplier = if token_info.is_premigration {
            self.premigration_tuning.impact_multiplier
        } else {
            1.0
        };

        // PumpFun-specific fee structure (optimized for pre-migration)
        let gross_profit = base_profit * premigration_multiplier;
        let pumpfun_fees = position_sol * 0.001; // ~0.1% PumpFun fees (lower than others)

        // Dynamic gas + tip calculation based on pre-migration urgency
        let base_gas_tip = self.pumpfun_config.base_gas_fee + self.pumpfun_config.priority_tip_sol;
        let gas_and_tip = if token_info.is_premigration {
            (base_gas_tip * self.pumpfun_config.gas_multiplier_premigration).min(self.pumpfun_config.max_tip_sol)
        } else {
            base_gas_tip
        };

        let total_costs = pumpfun_fees + gas_and_tip;

        let net_profit = gross_profit - total_costs;

        debug!("Profit calc: gross={:.4}, pumpfun_fees={:.4}, gas_tip={:.4}, net={:.4} SOL",
               gross_profit, pumpfun_fees, gas_and_tip, net_profit);

        net_profit.max(0.0) // Ensure non-negative
    }

    /// Calculate volatility score from transaction data
    fn calculate_volatility_score(&self, tx_data: &Value) -> f64 {
        let mut volatility_score = 0.0;

        // Price volatility
        if let Some(price_change) = tx_data.get("price_change_24h_pct").and_then(|p| p.as_f64()) {
            volatility_score += (price_change.abs() / 100.0).min(1.0); // Cap at 100%
        }

        // Volume volatility
        if let Some(volume_change) = tx_data.get("volume_change_24h_pct").and_then(|v| v.as_f64()) {
            volatility_score += (volume_change.abs() / 200.0).min(0.5); // Cap at 50% contribution
        }

        volatility_score.min(1.0)
    }

    /// Estimate current price from transaction data
    fn estimate_current_price(&self, token_info: &TokenInfo) -> f64 {
        // Simple estimation - in practice would use latest trade data
        if token_info.market_cap_usd > 0.0 && token_info.liquidity_sol > 0.0 {
            // Rough price estimate based on market cap and liquidity ratio
            token_info.market_cap_usd / (token_info.liquidity_sol * 150.0) // 150 SOL price estimate
        } else {
            0.000001 // Default micro price
        }
    }

    /// Add known pre-migration token to tracking
    pub fn add_premigration_token(&mut self, mint: String, info: TokenInfo) {
        info!("🎯 Added pre-migration token to tracking: {} ({})", info.symbol, mint);
        self.known_premigration_tokens.insert(mint, info);
    }

    /// Check if token is in known pre-migration list
    pub fn is_known_premigration(&self, mint: &str) -> bool {
        self.known_premigration_tokens.contains_key(mint)
    }

    /// Get filter statistics
    pub fn get_filter_stats(&self) -> HashMap<String, Value> {
        let mut stats = HashMap::new();
        stats.insert("min_liquidity_sol".to_string(), Value::from(self.min_liquidity_sol));
        stats.insert("max_market_cap_usd".to_string(), Value::from(self.max_market_cap_usd));
        stats.insert("max_position_size_sol".to_string(), Value::from(self.max_position_size_sol));
        stats.insert("target_price_impact_pct".to_string(), Value::from(self.target_price_impact_pct));
        stats.insert("known_premigration_count".to_string(), Value::from(self.known_premigration_tokens.len()));
        stats
    }
}