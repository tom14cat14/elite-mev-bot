use tokio::sync::mpsc;
use tokio::time::{self, Duration, Instant};
use once_cell::sync::Lazy;
use anyhow::Result;
use solana_sdk::transaction::Transaction;

// Simple logging macros (matching main bot)
macro_rules! info {
    ($($arg:tt)*) => {
        println!("[INFO] {}", format!($($arg)*));
    };
}

macro_rules! warn {
    ($($arg:tt)*) => {
        println!("[WARN] {}", format!($($arg)*));
    };
}

macro_rules! error {
    ($($arg:tt)*) => {
        println!("[ERROR] {}", format!($($arg)*));
    };
}

/// Queue item: (bundle, token_mint, position_size, expected_profit_sol)
type QueueItem = (Vec<Transaction>, String, f64, f64);

/// Global JITO submitter instance
pub static JITO_SUBMITTER: Lazy<JitoSubmitter> = Lazy::new(|| JitoSubmitter::new());

/// Queue-based JITO bundle submitter with rate limiting and retry logic
pub struct JitoSubmitter {
    queue_tx: mpsc::UnboundedSender<QueueItem>,
}

impl JitoSubmitter {
    /// Create new submitter with dedicated queue processing task
    pub fn new() -> Self {
        let (queue_tx, mut queue_rx) = mpsc::unbounded_channel::<QueueItem>();

        // Clone queue_tx for use inside the async task (for re-queuing)
        let queue_tx_clone = queue_tx.clone();

        // Dedicated task to submit bundles at exactly 1 per 1.1 seconds
        tokio::spawn(async move {
            let mut last_submit = Instant::now();
            let mut client: Option<crate::jito_bundle_client::JitoBundleClient> = None;

            info!("🚀 JITO Queue Processor started - Rate: 1 bundle per 1.1s");

            while let Some((bundle, token_mint, position_size, expected_profit_sol)) = queue_rx.recv().await {
                // Rate limiting: ensure at least 1.1s between submissions
                let elapsed = last_submit.elapsed();
                if elapsed < Duration::from_millis(1100) {
                    let sleep_duration = Duration::from_millis(1100) - elapsed;
                    info!("⏱️  Rate limit: Sleeping {:?} before submission", sleep_duration);
                    time::sleep(sleep_duration).await;
                }

                // Initialize JITO client lazily (reuse for performance)
                if client.is_none() {
                    let endpoint = std::env::var("JITO_ENDPOINT")
                        .unwrap_or_else(|_| "https://ny.mainnet.block-engine.jito.wtf".to_string());

                    // Get wallet keypair from environment for signing
                    let wallet_key = std::env::var("WALLET_PRIVATE_KEY")
                        .expect("WALLET_PRIVATE_KEY must be set");
                    let wallet_keypair = solana_sdk::signature::Keypair::from_base58_string(&wallet_key);

                    info!("📡 Initializing JITO client: {}", endpoint);
                    let jito_client = crate::jito_bundle_client::JitoBundleClient::new(
                        endpoint.clone(),
                        endpoint,
                        Some(wallet_keypair),
                    );

                    // Start background task to fetch tip floor data (10 minute refresh)
                    jito_client.start_tip_floor_refresh();
                    info!("💰 Started JITO tip floor refresh (99th percentile + scaling, 10min interval)");

                    client = Some(jito_client);
                }

                // Submit bundle
                let jito_client = client.as_ref().unwrap();

                // ULTRA-AGGRESSIVE JITO TIPPING STRATEGY (2025-10-08)
                // Always use 99th percentile as base, scale UP based on profit margin
                // - Base: 99th percentile (beats 99% of bundles)
                // - High margin (fees <5%): 3.0x multiplier (ultra aggressive)
                // - Medium margin (5-10%): 1.5-2.0x scaling (very aggressive)
                // - Low margin (>10%): 1.0x (99th only)
                // - Hard cap: 0.005 SOL maximum

                // Estimate fees
                let dex_fees_sol = position_size * 0.025; // 2.5% DEX fees
                let gas_fees_sol = 0.0001; // 0.0001 SOL estimate for gas

                // Get dynamic tip from JITO API
                let tip_lamports = if let Some(cached_data) = jito_client.get_cached_tip_floor() {
                    // ALWAYS use 99th percentile as base
                    let tip_99th_sol = cached_data.landed_tips_99th / 1_000_000_000.0;
                    let base_tip_99_lamports = (cached_data.landed_tips_99th * 1_000_000_000.0) as u64;

                    // Calculate fee margin with 99th percentile base
                    let total_fees_base = dex_fees_sol + gas_fees_sol + tip_99th_sol;
                    let fee_percentage = if expected_profit_sol > 0.0 {
                        (total_fees_base / expected_profit_sol) * 100.0
                    } else {
                        100.0 // Default to conservative if no profit
                    };

                    // PROFIT-BASED SCALING: Scale tip above 99th percentile based on margin
                    // High margin (fees <5%) → 3.0x multiplier (ultra aggressive)
                    // Medium margin (5-10%) → 1.5-2.0x scaling (very aggressive)
                    // Low margin (>10%) → 1.0x (99th percentile only)
                    let tip_multiplier = if fee_percentage >= 10.0 {
                        1.0  // Stick to 99th percentile
                    } else if fee_percentage >= 5.0 {
                        // Medium margin: Scale from 1.5x to 2.0x
                        1.5 + ((10.0 - fee_percentage) / 5.0) * 0.5
                    } else {
                        // High margin: Scale from 2.0x to 3.0x
                        2.0 + ((5.0 - fee_percentage) / 5.0) * 1.0
                    };

                    let scaled_tip = (base_tip_99_lamports as f64 * tip_multiplier) as u64;

                    info!("💎 Aggressive tip: Fee margin {:.1}% → Multiplier {:.2}x → Base 99th {:.6} SOL × {:.2} = {:.6} SOL",
                          fee_percentage, tip_multiplier,
                          base_tip_99_lamports as f64 / 1e9, tip_multiplier,
                          scaled_tip as f64 / 1e9);

                    // Minimum: 100,000 lamports (JITO competitive baseline)
                    let min_tip = 100_000_u64;

                    // Cap at 17% of profit (safety limit)
                    let max_tip_profit = ((expected_profit_sol * 0.17) * 1_000_000_000.0) as u64;

                    // Hard cap: 0.005 SOL (5,000,000 lamports)
                    let hard_cap = 5_000_000_u64;

                    scaled_tip.max(min_tip).min(max_tip_profit).min(hard_cap)
                } else {
                    // Fallback: Conservative tip calculation (if JITO API data not available yet)
                    warn!("⚠️  No JITO tip floor data, using fallback calculation");

                    // Use conservative 5% of profit for fallback
                    let total_fee_budget = expected_profit_sol * 0.05;
                    let tip_budget = total_fee_budget * 0.40; // 40% for tip, 60% for gas
                    ((tip_budget * 1_000_000_000.0) as u64).max(100_000).min(5_000_000)
                };

                // RATE LIMITING: Enforce 1.1 seconds per request (JITO best practice)
                // Ensures we stay under 1 bundle/second limit with safety margin
                let elapsed_since_last = last_submit.elapsed();
                let min_interval = Duration::from_millis(1100); // 1.1 seconds
                if elapsed_since_last < min_interval {
                    let wait_time = min_interval - elapsed_since_last;
                    info!("⏱️  Rate limiting: waiting {:?} before next submission", wait_time);
                    tokio::time::sleep(wait_time).await;
                }

                info!("📦 Submitting bundle: Token {} | Size: {:.3} SOL | Expected Profit: {:.4} SOL | Tip: {} lamports ({:.6} SOL)",
                      token_mint, position_size, expected_profit_sol, tip_lamports, tip_lamports as f64 / 1_000_000_000.0);

                // Add timeout to prevent infinite hangs (Grok's fix)
                let submit_timeout = Duration::from_secs(15);
                match time::timeout(submit_timeout, jito_client.submit_bundle(bundle.clone(), Some(tip_lamports))).await {
                    Ok(Ok(bundle_id)) => {
                        info!("✅ Bundle submitted successfully: {} | Token: {} | Amount: {:.3} SOL",
                              bundle_id, token_mint, position_size);
                        last_submit = Instant::now();
                    }
                    Ok(Err(e)) if e.to_string().contains("429") => {
                        // NO RETRIES: PumpFun MEV is time-sensitive, opportunity is gone after first miss
                        warn!("⚠️  429 Rate Limited - SKIPPING (opportunity expired) | Token: {}", token_mint);
                    }
                    Ok(Err(e)) => {
                        error!("❌ Bundle submission failed: {} | Token: {}", e, token_mint);
                    }
                    Err(_timeout) => {
                        error!("❌ Bundle submission TIMED OUT after 15s | Token: {}", token_mint);
                    }
                }
            }

            warn!("⚠️  JITO Queue Processor stopped (channel closed)");
        });

        Self { queue_tx }
    }

    /// Submit a bundle to the queue (non-blocking)
    pub fn submit(&self, bundle: Vec<Transaction>, token_mint: String, position_size: f64, expected_profit_sol: f64) -> Result<()> {
        self.queue_tx.send((bundle, token_mint, position_size, expected_profit_sol))
            .map_err(|_| anyhow::anyhow!("Queue send failed - processor may be stopped"))?;
        Ok(())
    }

    /// Get queue depth (for monitoring)
    pub fn queue_depth(&self) -> usize {
        // Note: mpsc doesn't expose len(), so we'd need a separate atomic counter
        // For now, return 0 (could be enhanced with Arc<AtomicUsize> if needed)
        0
    }
}

/// Helper to get the global submitter instance
pub fn get_submitter() -> &'static JitoSubmitter {
    &JITO_SUBMITTER
}
