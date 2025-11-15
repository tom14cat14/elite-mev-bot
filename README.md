# Solana PumpFun MEV Bot

Production-ready MEV bot for Solana, executing delayed sandwich attacks on PumpFun tokens.

## 📊 Current Status

**Production Ready** - Elite MEV Bot v2.1 with dynamic position sizing and JITO best practices

## 🎯 Strategy

**Delayed Sandwich Attack** - Anti-rug protection with 1-minute safety delay

**How It Works:**
1. **Detect**: Monitor ShredStream for NEW token launches on PumpFun
2. **Wait**: Track token for 1 MINUTE (avoid rug pulls)
3. **Monitor**: After 1 minute, watch for large BUY transactions
4. **Sandwich**: Front-run + back-run profitable victim buys
5. **Profit**: Capture price impact from victim's transaction

**Why Delayed:**
- Rugs typically happen in first 30-60 seconds
- Only sandwich tokens that survive initial launch
- Higher success rate, lower risk

## 🚀 Quick Start

```bash
# Build
cargo build --release --bin elite_mev_bot_v2_1_production

# Paper Trading (safe testing)
ENABLE_REAL_TRADING=false PAPER_TRADING=true \
  cargo run --release --bin elite_mev_bot_v2_1_production

# Live Trading (requires funded wallet)
ENABLE_REAL_TRADING=true PAPER_TRADING=false \
  cargo run --release --bin elite_mev_bot_v2_1_production
```

## 📁 Repository Structure

```
mev-bot/
├── src/
│   ├── bin/
│   │   └── elite_mev_bot_v2_1_production.rs  # Main production bot
│   └── [core modules]
├── docs/
│   ├── current/        # Essential documentation (11 docs)
│   └── examples/       # Example .env configurations
├── examples/           # Rust code examples
├── scripts/            # Utility shell scripts
└── Cargo.toml
```

## 📖 Documentation

All essential documentation is in `docs/current/`:

- **BOT_SUMMARY.md** - Complete bot overview
- **CLAUDE.md** - Development log & instructions
- **DELAYED_SANDWICH_STRATEGY.md** - Core strategy explanation
- **DYNAMIC_POSITION_SIZING_COMPLETE.md** - Position sizing logic
- **JITO_DYNAMIC_TIPPING.md** - JITO tipping strategy
- **MEV_WALLET_SETUP.md** - Wallet configuration
- **LIVE_TRADING_STATUS.md** - Current operational status
- **SECURITY_AUDIT_REPORT.md** - Security audit results
- **PRODUCTION_READINESS_AUDIT.md** - Production checklist

## ⚡ Key Features

- **ShredStream Integration** - 0.16ms latency (158μs)
- **Dynamic Position Sizing** - Scales with wallet balance & quality
- **Ultra-Aggressive JITO Tipping** - 99th percentile baseline
- **Profit-Based Fees** - 5-10% JITO tips based on expected returns
- **Complete Fee Accounting** - Gas + Tip + DEX fees
- **Anti-Rug Protection** - 1-minute delay after token launch
- **Safety First** - Comprehensive circuit breakers

## 🔧 Technical Specifications

### Performance Metrics
- **Detection Latency**: <8.7ms avg (1.76ms best)
- **Execution Speed**: <5.4ms avg
- **End-to-End Pipeline**: <15ms total
- **Bundle Success Rate**: >75% JITO landing
- **Target Returns**: 5-20% per successful sandwich

### Data Pipeline
```
ShredStream gRPC → Parse Entries → Detect NEW Token Launches →
Track Token for 1 Minute → Detect Large BUY Txs →
Calculate Sandwich Profit → Build 3-Tx Bundle → Submit to JITO
```

### Configuration
- **Market Cap Limit**: <$90K (pre-migration)
- **Volume Floor**: $5K/min minimum
- **Max Monitoring**: 15-minute window per token
- **Min Net Profit**: 0.015 SOL after all fees

## ⚠️ Important Notes

- **Real money trading** requires extensive paper trading validation first
- **JITO rate limits** (1 bundle/~1s) are shared across all bots
- **Wallet**: Encrypted with AES-256, stored in `wallets/` (gitignored)
- See documentation for complete safety guidelines

## 📜 License

Private repository - All rights reserved

---

**Last Updated**: 2025-11-06
**Status**: Production ready with full JITO best practices
