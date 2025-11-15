# Profit/Loss Tracking - Complete Event Logging

**Date**: 2025-11-09
**Status**: ✅ Comprehensive P&L tracking implemented

---

## Overview

The MEV bot has complete profit/loss tracking for every event (sandwich opportunity). Each execution is logged with detailed metrics to track performance and identify when margin adjustments are needed.

---

## Per-Event Tracking (Logged for Each Opportunity)

### **1. Profitability Analysis (Before Execution)**
Located: `elite_mev_bot_v2_1_production.rs:2098-2119`

```
💡 PROFITABILITY ANALYSIS:
  • Position size: X.XXXXXX SOL
  • Victim swap: X.XXXX SOL
  • Estimated price impact: X.XX%
  • Gross profit: X.XXXXXX SOL
  • Total fees: X.XXXXXX SOL (gas + JITO + DEX + volatility buffer)
  • Net profit: X.XXXXXX SOL (after all fees)
  • Min threshold: X.XXXXXX SOL

✅ Sandwich profitable | Net profit: X.XXXXXX SOL (after X.XXXXXX SOL total fees)
```

### **2. Position Sizing Details**
Located: `elite_mev_bot_v2_1_production.rs:2584-2590`

```
📊 Position sizing:
  • Quality score: X.X
  • Allocation: XX%
  • Position: X.XXXX SOL
  💰 Expected profit: X.XXXX SOL (X.X% return)
```

### **3. Execution Details (Real Trades)**
Located: `elite_mev_bot_v2_1_production.rs:2328-2329`

```
🎯 EXECUTING SANDWICH | DEX: [name] | Position: X.XXXX SOL | Expected profit: X.XXXXXX SOL
```

### **4. Fee Breakdown**
Located: `elite_mev_bot_v2_1_production.rs:2279`

```
📊 Fee summary:
   Gas fee: X.XXXXXX SOL
   JITO tip: X.XXXXXX SOL
   DEX fees: X.XXXXXX SOL
   Volatility buffer: X.XXXXXX SOL
   Total fees: X.XXXXXX SOL
   Expected profit: X.XXXXXX SOL (XX.XX%)
```

### **5. Post-Execution Balance Verification**
Located: `elite_mev_bot_v2_1_production.rs:739-748`

```rust
// Monitor bundle for confirmation and calculate real profit
let profit_sol = if balance_change > 0.0 {
    Some(balance_change)  // Positive profit
} else {
    Some(balance_change)  // Negative profit (loss)
};
```

### **6. Transaction Profit Calculation**
Located: `elite_mev_bot_v2_1_production.rs:808-887`

```
🔍 Analyzing transaction logs for real profit calculation...
💰 Calculated real profit: X.XXXXXX SOL
```

---

## Aggregate Metrics (Tracked Over Time)

### **1. Running Totals**
Tracked in `BotStatistics` struct:

```rust
pub struct BotStatistics {
    pub opportunities_detected: u64,
    pub opportunities_executed: u64,
    pub opportunities_failed: u64,
    pub total_profit_sol: f64,
    pub total_loss_sol: f64,
    pub win_rate: f64,
    pub average_profit_per_trade: f64,
}
```

### **2. Safety Limits Tracking**
```rust
pub struct SafetyLimits {
    pub max_daily_loss_sol: f64,
    pub daily_loss_counter: Arc<AtomicU64>,  // Persisted to disk every 5 minutes
}
```

**Persistence**: Daily loss counter saved to `.mev_daily_loss.state` every 5 minutes
**Reset**: Automatically resets at 00:00 UTC daily

### **3. Periodic Statistics Display**
Located: `elite_mev_bot_v2_1_production.rs:1638-1649`

```
📊 LIVE TRADING | Trades: X | Success: X | Profit: X.XXXXXX SOL
   Average profit per trade: X.XXXXXX SOL
   Win rate: XX.X%
   Total volume: X.XX SOL
```

---

## Key Metrics for Margin Adjustment

### **What's Logged for Each Event:**

1. **Gross Profit**: Raw profit before fees
2. **Total Fees**: Complete fee accounting (gas + JITO + DEX + buffer)
3. **Net Profit**: Final profit after ALL fees
4. **Profit Margin**: Net profit as % of position size
5. **Fee Ratio**: Total fees as % of gross profit
6. **Expected vs Actual**: Comparison of estimated vs realized profit

### **Failure Reasons Tracked:**

```rust
pub enum FailureReason {
    InsufficientBalance,
    RateLimitHit,
    TransactionFailed,    // On-chain execution failed
    SlippageExceeded,     // Trade executed with excessive slippage
    InsufficientMargin,   // Net profit below required margin
}
```

---

## Configuration for Margin Tuning

### **Adjustable Thresholds** (`.env.multidex`):

```bash
# Minimum net profit after all fees
MIN_NET_PROFIT_SOL=0.015

# Profit margin multiplier (net profit must be X times total fees)
MIN_PROFIT_MARGIN_MULTIPLIER=1.5

# Maximum position size
MAX_POSITION_SIZE_SOL=2.0

# Daily loss limit
MAX_DAILY_LOSS_SOL=5.0

# JITO fee percentage (of expected profit)
JITO_FEE_PERCENTAGE=10.0
```

### **When to Increase Margins:**

**Monitor these in logs:**
- Frequent `⏭️ Skipping sandwich - net profit too low` → Opportunities not profitable enough
- `⚠️ Actual profit lower than expected` → Margin too thin, increase MIN_PROFIT_MARGIN_MULTIPLIER
- High failure rate (`SlippageExceeded`) → Increase slippage tolerance or reduce position size
- Daily loss limit hit → Reduce position sizes or increase MIN_NET_PROFIT_SOL

---

## Log File Locations

- **Live Trading Log**: `/tmp/mev_multidex.log` (Multi-DEX mode)
- **PumpFun Log**: `/tmp/mev_pumpfun.log` (PumpFun-only mode)
- **Daily Loss State**: `.mev_daily_loss.state` (persisted counter)

---

## Example Log Output (Per Event)

```
[INFO] 💡 PROFITABILITY ANALYSIS:
[INFO]   • Position size: 2.000000 SOL
[INFO]   • Victim swap: 5.2500 SOL
[INFO]   • Estimated price impact: 3.15%
[INFO]   • Gross profit: 0.063000 SOL
[INFO]   • Total fees: 0.018500 SOL (gas + JITO + DEX + volatility buffer)
[INFO]   • Net profit: 0.044500 SOL (after all fees)
[INFO]   • Min threshold: 0.015000 SOL

[INFO] ✅ Sandwich profitable | Net profit: 0.044500 SOL (after 0.018500 SOL total fees)

[INFO] 🎯 EXECUTING SANDWICH | DEX: Raydium_CLMM | Position: 2.0000 SOL | Expected profit: 0.044500 SOL

[INFO] ✅ Arbitrage bundle submitted! ID: abc123xyz

[INFO] 💰 Calculated real profit: 0.042300 SOL
```

**Result**: 
- **Expected**: 0.044500 SOL profit
- **Actual**: 0.042300 SOL profit
- **Difference**: -0.002200 SOL (4.9% lower than expected)

**Action**: If this pattern repeats (actual < expected), increase `MIN_PROFIT_MARGIN_MULTIPLIER` from 1.5 to 2.0

---

## Status

✅ **All metrics implemented and logging**
✅ **Per-event profitability tracking**
✅ **Aggregate statistics**
✅ **Safety limits with persistence**
✅ **Failure reason tracking**
✅ **Margin adjustment configuration**

**Ready to monitor live performance and adjust margins as needed.**
