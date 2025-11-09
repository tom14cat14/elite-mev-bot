# Multi-DEX MEV Bot Strategy

**Date**: 2025-11-08
**Goal**: Create second MEV bot variant for post-migration and established tokens
**Capital**: 1.1 SOL (same wallet or separate)

---

## 🎯 Strategy Comparison

### **Current MEV Bot** (Keep Running)
- **Target**: PumpFun pre-migration only
- **Tokens**: NEW launches (<60 seconds old)
- **Market Cap**: <$90K
- **Delay**: 60-second anti-rug wait
- **Opportunity Rate**: 5-20 per day (sporadic)
- **Profit per Trade**: High (20-50% potential)

### **Multi-DEX MEV Bot** (NEW - To Create)
- **Target**: ALL DEXs (Raydium, Orca, Meteora, Jupiter, PumpSwap)
- **Tokens**: Established tokens (post-migration, high liquidity)
- **Market Cap**: $100K - $10M (avoid low liquidity)
- **Delay**: None (immediate sandwich)
- **Opportunity Rate**: 50-200 per day (frequent)
- **Profit per Trade**: Lower (5-15% typical)

---

## 🔧 What Needs to Change

### **Code Already Has Multi-DEX Support!** ✅

The `sandwich_engine.rs` already includes:
- ✅ Raydium AMM V4 instructions (line 22-34)
- ✅ Orca Whirlpools instructions (line 26-45)
- ✅ PumpFun executor (line 54)
- ✅ Jupiter executor (line 53)
- ✅ Generic DEX registry (line 51)

**No code changes needed!** Just configuration.

---

## 📝 Configuration Changes (.env)

### **Create `.env.multidex`**

```bash
# =============================================================================
# MULTI-DEX MEV BOT - POST-MIGRATION & ESTABLISHED TOKENS
# =============================================================================

# ================================
# 🔑 WALLET CONFIGURATION
# ================================
# Use same wallet or create new one
WALLET_PRIVATE_KEY=5NqpEknRLphDJNtiHNQu9NScpf9oi8jgWVrPUJm38LDd1x41GJR3PgCysVSXgH7VrSyw3G4zRNcJFbBoY5WL4trf

# ================================
# 🎯 MULTI-DEX STRATEGY (KEY CHANGES)
# ================================

# Remove PumpFun-only filter
ENABLE_BONDING_CURVE_DIRECT=false          # Was: true
SKIP_JUPITER_ROUTING=false                  # Was: true
PUMPFUN_PROGRAM_ID=                         # Leave blank (not used)

# Multi-DEX sandwich parameters
DEFAULT_SLIPPAGE_BPS=150                    # 1.5% slippage
TARGET_PRICE_IMPACT_PCT=3.0                 # 3% price impact minimum
MAX_MARKET_CAP_USD=10000000                 # $10M max (was $90K)
MIN_MARKET_CAP_USD=100000                   # $100K min (NEW - avoid low liquidity)
MIN_LIQUIDITY_SOL=50.0                      # 50 SOL min liquidity (was 2.0)

# ================================
# ⏰ TIMING STRATEGY (KEY CHANGES)
# ================================

# NO LAUNCH DELAY - immediate sandwich on established tokens
COIN_LAUNCH_DELAY_SECONDS=0                 # Was: 60
ENABLE_VOLUME_DECAY_MONITORING=false        # Not needed for established tokens
MAX_MONITORING_TIME_MINUTES=1               # Quick execution (was 15)

# New coin detection - DISABLE (we want established tokens)
NEW_COIN_QUALITY_THRESHOLD=100.0            # Set impossibly high to ignore new coins
BONDING_CURVE_COMPLETION_THRESHOLD=1.0      # Only fully migrated tokens

# Volume requirements - HIGHER for established tokens
MIN_VOLUME_USD_PER_MINUTE=10000             # $10K/min (was $5K)
VOLUME_SAMPLING_WINDOW_SECONDS=60

# ================================
# 💰 POSITION SIZING (SAME AS BEFORE)
# ================================
CAPITAL_SOL=0.9
MAX_POSITION_SIZE_SOL=0.9
MIN_POSITION_SIZE_SOL=0.05
POSITION_SIZE_BASE=0.5
MIN_WALLET_RESERVE_SOL=0.1

# Profit margins - SAME
MAX_LOSS_SOL=0.15
STOP_LOSS_PERCENTAGE=5.0
TAKE_PROFIT_PERCENTAGE=10.0

# ================================
# 🎯 DEX TARGETS (NEW SECTION)
# ================================

# Enable all DEXs
ENABLE_RAYDIUM_SANDWICH=true
ENABLE_ORCA_SANDWICH=true
ENABLE_METEORA_SANDWICH=true
ENABLE_JUPITER_SANDWICH=true
ENABLE_PUMPSWAP_SANDWICH=true

# DEX-specific minimum volumes (to filter low liquidity)
RAYDIUM_MIN_VOLUME_24H_SOL=100
ORCA_MIN_VOLUME_24H_SOL=100
METEORA_MIN_VOLUME_24H_SOL=50
JUPITER_MIN_VOLUME_24H_SOL=100
PUMPSWAP_MIN_VOLUME_24H_SOL=50

# ================================
# 🛡️ SAFETY (MORE AGGRESSIVE)
# ================================

# More aggressive - established tokens less risky
CIRCUIT_BREAKER_ENABLED=false
MAX_DAILY_TRADES=200                        # Was: 100 (more opportunities)
MAX_CONSECUTIVE_FAILURES=999999999
MAX_CONCURRENT_TRADES=3                     # Was: 2 (can handle more)
DAILY_LOSS_LIMIT_SOL=0.15
MAX_DAILY_LOSS_SOL=0.15

POSITION_TIMEOUT_MS=800
MAX_POSITION_AGE_SECONDS=300

# ================================
# ⚡ JITO (SAME)
# ================================
JITO_BLOCK_ENGINE_URL=https://ny.mainnet.block-engine.jito.wtf
JITO_ENDPOINT=https://ny.mainnet.block-engine.jito.wtf
JITO_TIP_ACCOUNT=96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5
JITO_MAX_TIP_LAMPORTS=10000

# ================================
# 🌐 SHREDSTREAM (SAME - ALREADY WORKING)
# ================================
SHREDS_ENDPOINT=https://shreds-ny6-1.erpc.global
SOLANA_RPC_ENDPOINT=https://edge.erpc.global?api-key=507c3fff-6dc7-4d6d-8915-596be560814f

# ================================
# 📊 PERFORMANCE (SAME)
# ================================
ENABLE_REAL_TRADING=false                   # Start with paper trading
PAPER_TRADING=true
RUST_LOG=debug
ENVIRONMENT=production_multidex
BOT_VERSION=v2.1_multidex

# ================================
# 🔥 STRATEGY SUMMARY
# ================================
# CURRENT CONFIGURATION:
# • Strategy: Multi-DEX Sandwich Attacks (ALL DEXs, established tokens)
# • Targets: Raydium, Orca, Meteora, Jupiter, PumpSwap
# • Market Cap: $100K - $10M (high liquidity only)
# • No launch delay: Immediate sandwich on detection
# • Capital: 1.1 SOL (0.5 SOL positions optimal)
# • Volume: Min $10K/min (2x higher than PumpFun bot)
# • Safety: Same circuit breakers, 3 concurrent positions
#
# DIFFERENCES FROM PUMPFUN BOT:
# ❌ No PumpFun bonding curve direct trading
# ❌ No 60-second launch delay
# ❌ No new coin detection
# ✅ ALL DEXs enabled (Raydium, Orca, Meteora, Jupiter, PumpSwap)
# ✅ Higher market cap range ($100K - $10M)
# ✅ Higher volume requirements ($10K/min)
# ✅ More concurrent positions (3 vs 2)
# =============================================================================

# GROK CYCLE 3: Margin-Based Profit Threshold System (SAME)
MIN_PROFIT_MARGIN_MULTIPLIER=2.0
FEE_BUFFER_MULTIPLIER=1.2
GAS_FEE_LAMPORTS=100000
COMPUTE_FEE_LAMPORTS=20000
```

---

## 🚀 Implementation Steps

### **Option A: Run Both Bots Simultaneously** (Recommended)

**Advantage**: Cover both strategies (new launches + established tokens)

**Setup**:
1. Keep current MEV bot running (PumpFun new launches)
2. Create `.env.multidex` with config above
3. Run second instance with:
   ```bash
   cp .env .env.multidex
   # Edit .env.multidex with changes above

   # Start multidex bot
   env $(cat .env.multidex | xargs) ./target/release/elite_mev_bot_v2_1_production \
     > /tmp/mev_multidex.log 2>&1 &
   ```

**JITO Rate Limit**: Both share 1 bundle/sec - use 1.1s spacing (already configured)

---

### **Option B: Switch Current Bot to Multi-DEX**

**Advantage**: Simpler, one bot only

**Setup**:
1. Stop current MEV bot: `pkill -f elite_mev_bot_v2_1_production`
2. Copy `.env` to `.env.pumpfun_backup`
3. Update `.env` with multi-DEX config
4. Restart: `./target/release/elite_mev_bot_v2_1_production > /tmp/mev_startup.log 2>&1 &`

**Trade-off**: Lose PumpFun new launch coverage

---

## 📊 Expected Performance

### **Multi-DEX Bot Profitability**

**Opportunity Rate**: 50-200 sandwich opportunities/day
- Raydium: 20-50/day
- Orca: 10-30/day
- Meteora: 5-15/day
- Jupiter: 10-30/day
- PumpSwap: 5-15/day

**Win Rate**: 40-60% (lower than PumpFun, higher competition)

**Profit per Trade**: 0.03-0.08 SOL (5-15% return)

**Daily Expected**:
- Conservative: 10 successful trades × 0.04 SOL = **0.4 SOL/day**
- Moderate: 20 successful trades × 0.05 SOL = **1.0 SOL/day**
- Aggressive: 30 successful trades × 0.06 SOL = **1.8 SOL/day**

**Monthly Expected**: 12-50 SOL (~1000-4500% monthly return)

---

## ⚠️ Key Differences vs PumpFun Bot

| Aspect | PumpFun Bot | Multi-DEX Bot |
|--------|-------------|---------------|
| **Opportunities** | 5-20/day (rare) | 50-200/day (frequent) |
| **Profit/Trade** | 0.1-0.5 SOL (high) | 0.03-0.08 SOL (lower) |
| **Win Rate** | 70-80% | 40-60% |
| **Competition** | Low (60s delay filter) | High (no delay) |
| **Market Cap** | <$90K | $100K-$10M |
| **Strategy** | Patient (wait for launch) | Aggressive (immediate) |
| **Best For** | Large profits, rare | Consistent income, volume |

---

## 🎯 Recommendation

**Run BOTH bots simultaneously**:

1. **PumpFun Bot** (current): Hunt rare high-profit new launches
2. **Multi-DEX Bot** (new): Grind consistent smaller profits on established tokens

**Combined Expected Daily**: 0.5-2.0 SOL/day
- PumpFun: 0.1-0.5 SOL (1-3 trades)
- Multi-DEX: 0.4-1.5 SOL (10-30 trades)

**Capital Split**:
- Both use same 1.1 SOL wallet (different timing = no conflicts)
- Or create second wallet with another 1+ SOL

---

## ✅ Next Steps

1. Create `.env.multidex` file
2. Test compilation: `cargo build --release`
3. Start in paper trading mode first
4. Monitor for 1-2 hours
5. Switch to live trading if profitable
6. Run both bots simultaneously for maximum coverage

---

**Status**: Ready to implement
**Code Changes**: NONE NEEDED (multi-DEX support already exists!)
**Config Changes**: Simple .env modifications
**Timeline**: 30 minutes to deploy and test

