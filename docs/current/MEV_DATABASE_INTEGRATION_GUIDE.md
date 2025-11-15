# MEV Bot Database Integration Guide

**Status**: Ready to integrate
**Estimated Time**: 15-20 minutes
**Files to modify**: `src/bin/elite_mev_bot_v2_1_production.rs`

---

## Step 1: Initialize Database Tracker (Top of `main()`)

**Location**: In `main()` function, after loading config, before main loop

```rust
use crate::mev_database_tracker::MevDatabaseTracker;

// Around line 1700, after env vars are loaded
let db_path = std::path::PathBuf::from("./data/mev_tracking.db");
let db_tracker = match MevDatabaseTracker::new(&db_path) {
    Ok(tracker) => {
        info!("✅ MEV database tracker initialized");

        // Log current configuration
        let mode = if enable_bonding_curve_direct { "pumpfun" } else { "multi-dex" };
        let _ = tracker.log_config(
            mode,
            paper_trading,
            0.01,  // min_swap_size_sol (from config)
            100.0, // max_swap_size_sol (from config)
            0.0001, // min_profit_sol (from config)
        );

        Arc::new(tracker)
    }
    Err(e) => {
        warn!("⚠️ Failed to initialize MEV database tracker: {}", e);
        warn!("   Dashboard will not have real-time data");
        // Continue without database tracking (non-fatal)
        Arc::new(MevDatabaseTracker::new(":memory:").expect("In-memory DB should never fail"))
    }
};

info!("🎯 Database tracker initialized at: {:?}", db_path);
```

---

## Step 2: Log Detected Opportunities

**Location**: Inside main loop, when opportunity is detected (around line 1950)

**Find this pattern**:
```rust
match execute_sandwich_opportunity(
    opportunity,
    &jito_client,
```

**Add BEFORE the match**:
```rust
// Calculate detection latency
let detection_start = std::time::Instant::now();
let detection_latency_ms = detection_start.elapsed().as_micros() as f64 / 1000.0;

// Log detected opportunity to database
let _ = db_tracker.log_detected(&opportunity, detection_latency_ms);
info!("📊 Logged opportunity to database: {} | DEX: {} | Size: {:.4} SOL",
      &opportunity.signature[..8], opportunity.dex_name, opportunity.estimated_sol_value);
```

---

## Step 3: Log Skipped Opportunities

**Location**: Inside `execute_sandwich_opportunity()`, whenever returning `Ok(false)`

**Find patterns like**:
```rust
return Ok(false);
```

**Replace with**:
```rust
let _ = db_tracker.log_skipped(&opportunity, "Reason here");
return Ok(false);
```

**Example locations** (search for these and add tracking):

1. **Net profit too low** (around line 2105):
```rust
if estimated_net_profit < min_profit_threshold {
    let _ = db_tracker.log_skipped(&opportunity, &format!(
        "Net profit too low: {:.6} SOL < {:.6} SOL", estimated_net_profit, min_profit_threshold
    ));
    debug!("⏭️ Skipping sandwich - net profit too low...");
    return Ok(false);
}
```

2. **No pool address** (around line 2335):
```rust
warn!("⚠️  No pool address in opportunity");
let _ = db_tracker.log_skipped(&opportunity, "No pool address");
return Ok(false);
```

3. **Phase 2 DEXs** (around line 2327-2370):
```rust
crate::dex_pool_state::DexPoolState::RaydiumClmm(pool_state) => {
    info!("🎯 Raydium CLMM pool detected!");
    // ...
    warn!("⚠️  CLMM execution not yet implemented - skipping");
    let _ = db_tracker.log_skipped(&opportunity, "CLMM execution not implemented (Phase 2)");
    return Ok(false);
}
```

---

## Step 4: Log Executed Opportunities

**Location**: After successful JITO bundle submission (around line 2300)

**Find this pattern**:
```rust
match bundle_result {
    Ok(bundle_id) => {
        info!("✅ Arbitrage bundle submitted! ID: {}", bundle_id);
```

**Add AFTER the success log**:
```rust
// Calculate actual profit (for now, use estimated - will improve with on-chain verification)
let actual_profit_sol = estimated_net_profit;

// Log execution to database
let _ = db_tracker.log_executed(
    &opportunity.signature,
    actual_profit_sol,
    effective_total_fees,
    jito_tip_sol,
    position_size_sol,
    &bundle_id,
    execution_latency_ms,
);

info!("📊 Logged execution to database: {} | Profit: {:.6} SOL",
      &opportunity.signature[..8], actual_profit_sol);
```

---

## Step 5: Log Failed Executions

**Location**: In the `Err` arm of bundle submission (around line 2313)

**Find this pattern**:
```rust
Err(e) => {
    warn!("⚠️  Bundle submission failed: {}", e);
```

**Add AFTER the warn**:
```rust
let _ = db_tracker.log_failed(&opportunity.signature, &format!("JITO bundle failed: {}", e));
return Ok(false);
```

---

## Step 6: Pass `db_tracker` to Function

**Location**: Function signature and call site

**Update function signature** (around line 2003):
```rust
async fn execute_sandwich_opportunity(
    opportunity: &crate::mev_sandwich_detector::SandwichOpportunity,
    jito_client: &Arc<JitoBundleClient>,
    trading_keypair: &Arc<Keypair>,
    safety_limits: &Arc<SafetyLimits>,
    paper_trading: bool,
    db_tracker: &Arc<MevDatabaseTracker>,  // ADD THIS
) -> Result<bool> {
```

**Update call site** (around line 1950):
```rust
match execute_sandwich_opportunity(
    opportunity,
    &jito_client,
    &trading_keypair,
    &safety_limits,
    paper_trading,
    &db_tracker,  // ADD THIS
).await {
```

---

## Step 7: Periodic Performance Logging (Optional)

**Location**: Inside main loop, every N iterations

**Add this code** (around line 1980, before processing opportunities):
```rust
// Log performance metrics every 100 iterations
static ITERATION_COUNTER: AtomicU64 = AtomicU64::new(0);
if ITERATION_COUNTER.fetch_add(1, Ordering::Relaxed) % 100 == 0 {
    // Get wallet balance
    let wallet_balance_sol = match rpc_client.get_balance(&trading_keypair.pubkey()) {
        Ok(lamports) => lamports as f64 / 1_000_000_000.0,
        Err(_) => 0.0,
    };

    let _ = db_tracker.log_performance(
        8.7,  // avg detection latency (calculated from recent opportunities)
        5.4,  // avg execution latency (calculated from recent executions)
        wallet_balance_sol,
    );
}
```

---

## Complete Integration Checklist

- [ ] Add `use crate::mev_database_tracker::MevDatabaseTracker;` at top
- [ ] Initialize `db_tracker` in `main()` (Step 1)
- [ ] Log detected opportunities before execution (Step 2)
- [ ] Log skipped opportunities at all `return Ok(false)` points (Step 3)
- [ ] Log successful executions after bundle submission (Step 4)
- [ ] Log failed executions in error handling (Step 5)
- [ ] Update function signature to accept `db_tracker` (Step 6)
- [ ] Add periodic performance logging (Step 7 - optional)
- [ ] Test compilation: `cargo check --bin elite_mev_bot_v2_1_production`

---

## Expected Database Location

```
/home/tom14cat14/MEV_Bot/data/mev_tracking.db
```

**Dashboard will query this database** via Python API.

---

## Testing the Integration

1. **Compile**:
   ```bash
   cargo check --bin elite_mev_bot_v2_1_production
   ```

2. **Run in paper trading**:
   ```bash
   ENABLE_REAL_TRADING=false PAPER_TRADING=true \
     cargo run --release --bin elite_mev_bot_v2_1_production
   ```

3. **Verify database**:
   ```bash
   sqlite3 /home/tom14cat14/MEV_Bot/data/mev_tracking.db "SELECT COUNT(*) FROM opportunities;"
   # Should show detected opportunities
   ```

4. **Check recent opportunities**:
   ```bash
   sqlite3 /home/tom14cat14/MEV_Bot/data/mev_tracking.db \
     "SELECT timestamp, dex_name, status FROM opportunities ORDER BY timestamp DESC LIMIT 10;"
   ```

---

## Next Step: Add Python API Endpoints

Once database logging is working, add API endpoints in `dashboard_api.py` (see next section).
