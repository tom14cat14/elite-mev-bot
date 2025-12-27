# 📚 MEV Bot - Elite MEV Bot v2.1 Production

**GitHub Repository**: https://github.com/tom14cat14/elite-mev-bot

---

## ⚡ CORE RULES (Non-Negotiable)

### **1. Never Use Fake Data**
- ✅ Real blockchain data ONLY (ShredStream, JITO, RPC)
- ❌ NO simulated prices, NO random data
- **If data unavailable → Stop, don't fake it**

### **2. Fix Errors, Don't Shortcut Them**
- ✅ Root cause fixes ONLY
- ✅ Proper async/await, correct trait bounds, fix borrow checker
- ❌ NO hacks, NO `unsafe` blocks, NO suppressing warnings
- **If you don't understand the error → Research, don't guess**

### **3. Safety First, Always**
- ✅ Paper trading FIRST, every time
- ✅ All safety mechanisms working perfectly
- ✅ Complete fee accounting (gas + tips + DEX fees)
- ❌ NO "good enough" for money
- **Financial code must be bulletproof**

### **4. Real Money = Extra Caution**
- ✅ Test paper trading extensively before live
- ✅ Start with minimum positions
- ✅ Monitor first 5-10 trades closely
- ✅ Circuit breakers must be tested
- **A single bug can cost significant money**

---

## 🎯 CURRENT STATUS

### **Production Bot - Elite MEV Bot v2.1**
- **Status**: Production ready with JITO best practices
- **Strategy**: Delayed sandwich (1-minute anti-rug delay)
- **Location**: `src/bin/elite_mev_bot_v2_1_production.rs`
- **Wallet**: `2MwDTydE7RshNSUmofotD9jD7MmaBtR7ZY26wVkJ3ZJ7` (1.686 SOL)
- **Build**: ✅ Compiles successfully

---

## 📖 Documentation

All essential documentation is in `/docs/current/`:

1. **BOT_SUMMARY.md** ⭐ - Complete bot overview and technical details
2. **DELAYED_SANDWICH_STRATEGY.md** - Core strategy explanation
3. **SANDWICH_STRATEGY_IMPLEMENTATION.md** - Implementation guide
4. **SANDWICH_STRATEGY_EXPLAINED.md** - Strategy deep dive
5. **DYNAMIC_POSITION_SIZING_COMPLETE.md** - Position sizing logic
6. **JITO_DYNAMIC_TIPPING.md** - JITO tipping strategy (99th percentile baseline)
7. **MEV_WALLET_SETUP.md** - Wallet configuration
8. **LIVE_TRADING_STATUS.md** - Current operational status
9. **SECURITY_AUDIT_REPORT.md** - Security audit results
10. **PRODUCTION_READINESS_AUDIT.md** - Production checklist

---

## 🚀 Quick Start

```bash
# Build
cargo build --release --bin elite_mev_bot_v2_1_production

# Paper Trading (safe)
ENABLE_REAL_TRADING=false PAPER_TRADING=true \
  cargo run --release --bin elite_mev_bot_v2_1_production

# Live Trading (caution!)
ENABLE_REAL_TRADING=true PAPER_TRADING=false \
  cargo run --release --bin elite_mev_bot_v2_1_production
```

---

## 🏗️ Architecture

### **Strategy: Delayed Sandwich Attack**
1. **Detect**: Monitor ShredStream for NEW token launches
2. **Wait**: Track token for 1 MINUTE (avoid rug pulls)
3. **Monitor**: After delay, watch for large BUY transactions
4. **Sandwich**: Front-run + back-run profitable buys
5. **Profit**: Capture price impact (5-20% per sandwich)

### **Key Features**
- **ShredStream**: 0.16ms latency (158μs)
- **Dynamic Position Sizing**: Scales with wallet balance & quality
- **Ultra-Aggressive JITO Tipping**: 99th percentile baseline, scales to 3.0x
- **Complete Fee Accounting**: Gas + Tip + DEX fees
- **Anti-Rug Protection**: 1-minute delay after launch
- **Safety First**: Circuit breakers, stop loss, daily limits

### **Performance Metrics**
- Detection Latency: <8.7ms avg (1.76ms best)
- Execution Speed: <5.4ms avg
- End-to-End Pipeline: <15ms total
- Bundle Success Rate: >75% JITO landing
- Target Returns: 5-20% per sandwich

---

## 🔧 Configuration

### **Trading Parameters** (`.env`)
```bash
# Core Strategy
NEW_COIN_QUALITY_THRESHOLD=8.5          # Min quality score
MAX_MARKET_CAP_USD=90000                # $90K pre-migration limit
MIN_VOLUME_USD_PER_MINUTE=5000          # $5K/min volume floor
COIN_LAUNCH_DELAY_SECONDS=60            # 1min anti-rug delay

# Position Sizing
MIN_NET_PROFIT_SOL=0.015                # Min profit after all fees

# Safety
ENABLE_REAL_TRADING=false               # Safety: disabled by default
PAPER_TRADING=true                      # Paper trading mode
```

---

## 🎉 Recent Major Improvements

### **Ultra-Aggressive JITO Tipping** (2025-10-08)
- **Strategy**: 99th percentile as BASELINE, scales UP to 3.0x
- **Fee Margin Scaling**:
  - High margin (< 5% fees) → 99th × 3.0x (capped at 0.005 SOL)
  - Medium margin (5-10% fees) → 99th × 1.5-2.0x
  - Low margin (> 10% fees) → 99th × 1.0x
- **Refresh**: Every 10 minutes (3x faster)
- **Documentation**: `docs/current/JITO_DYNAMIC_TIPPING.md`

### **Dynamic Position Sizing** (2025-10-07)
- Real-time wallet balance queries before every trade
- Quality-based allocation: 70% (Q9.5+), 50% (Q9.0+), 40% (Q8.5+)
- Always protects 0.1 SOL for fees
- Profit-based JITO fees (5-10% of expected profit)
- Complete fee accounting (Gas + Tip + DEX fees)
- **Documentation**: `docs/current/DYNAMIC_POSITION_SIZING_COMPLETE.md`

### **Real ShredStream Integration** (2025-10-06)
- Replaced fake ShredStreamProcessor with real implementation
- 3-layer filtering (same as Arb_Bot)
- Real bonding curve calculations
- Production-ready JITO bundle preparation

---

## 📁 Repository Structure

```
mev-bot/
├── src/
│   ├── bin/
│   │   ├── elite_mev_bot_v2_1_production.rs  ⭐ MAIN
│   │   ├── check_wallet_balance.rs
│   │   └── shredstream_mev_bot.rs
│   ├── arbitrage_engine.rs
│   ├── sandwich_engine.rs
│   ├── pumpfun_new_coin_detector.rs
│   ├── jito_bundle_manager.rs
│   ├── jito_submitter.rs
│   ├── realtime_price_monitor.rs
│   └── [other core modules]
├── docs/
│   ├── current/        # 11 essential docs
│   └── examples/       # Example .env files
├── examples/           # Rust code examples
├── scripts/            # Utility shell scripts
├── wallets/            # Encrypted wallets (gitignored)
├── .env
├── .env.example
├── CLAUDE.md           # This file
├── README.md
└── Cargo.toml
```

---

## ⚠️ IMPORTANT

- **Real money trading** requires extensive paper trading validation first
- **JITO rate limits** (1 bundle/~1s): Managed via high profit targets to reduce submission volume
- **Wallet encryption**: AES-256, stored in `wallets/` directory
- **Multi-market coverage**: Bot can detect both PumpFun and Multi-DEX opportunities simultaneously
- See documentation in `docs/current/` for complete details

---

## 🔗 Related Resources

- **JITO Official Docs**: https://jito-labs.gitbook.io/mev
- **ShredStream Docs**: https://docs.erpc.cloud/shredstream
- **Solana Docs**: https://docs.solana.com

---

**Last Updated**: 2025-11-06
**Status**: Production ready after major cleanup
**Build**: ✅ 0 errors, 11 warnings
