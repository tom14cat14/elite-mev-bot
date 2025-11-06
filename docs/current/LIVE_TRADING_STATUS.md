# MEV Bot - Live Trading Status

**Date**: 2025-10-06
**Status**: ✅ LIVE TRADING ACTIVE
**Wallet**: `CWfwucpmfQveUY8D14SEuc5YK6BbVt4EhATroznU7ktT`
**Balance**: 1.0 SOL
**Mode**: Real money trading (NOT paper trading)

---

## 🎯 Current Configuration

### **Trading Parameters**
- **Position Size**: 0.1 SOL per trade (10% of capital)
- **Min Profit**: 0.08 SOL per trade (8% return)
- **Daily Loss Limit**: 0.2 SOL (circuit breaker)
- **Daily Trade Limit**: 20 trades
- **Slippage Tolerance**: 50 bps (0.5%)
- **JITO Tip**: 10,000 lamports

### **Data Source**
- **ShredStream**: `https://shreds-ny6-1.erpc.global`
- **RPC**: `https://api.mainnet-beta.solana.com`
- **Latency**: ~15ms (ShredStream)

### **Safety Mechanisms**
- ✅ Circuit breaker at 0.2 SOL loss
- ✅ Position size limits (0.1 SOL max)
- ✅ Profit threshold enforcement (0.08 SOL min)
- ✅ Daily trade limits (20 max)
- ✅ JITO MEV protection enabled

---

## 🚨 CRITICAL FIX: Profit Threshold Unification

**Problem**: User reported same issue as Arb bot - profit thresholds inconsistent across codebase

**Before Fix**:
```
Location                                          | Value
--------------------------------------------------|--------
.env.mev_production                               | 0.05 SOL ❌
src/bin/elite_mev_bot_v2_1_production.rs:188     | 0.08 SOL ❌ HARDCODED
detect_arbitrage_opportunities() spread filter    | 0.5% (no profit check) ❌
```

**After Fix**:
```
Location                                          | Value
--------------------------------------------------|--------
.env.mev_production                               | 0.08 SOL ✅
src/bin/elite_mev_bot_v2_1_production.rs:191-194 | Read from env ✅
detect_arbitrage_opportunities() line 1630-1678   | Filters by profit ✅
```

**Result**: All profit thresholds now unified and read from single source (`.env.mev_production`)

**Files Modified**:
1. `src/bin/elite_mev_bot_v2_1_production.rs` (Lines 187-194, 1629-1678)
2. `.env.mev_production` (Updated to 0.08 SOL, LIVE mode)

**Full Details**: See `PROFIT_THRESHOLD_UNIFIED.md`

---

## 📊 Expected Performance

### **Risk/Reward Analysis**
- **Max Risk**: 0.2 SOL (circuit breaker)
- **Expected Trades**: 1-10 per day
- **Profit per Trade**: 0.08-0.15 SOL
- **Win Rate Needed**: >70% for profitability
- **Max Positions**: ~10 concurrent (0.1 SOL each)

### **First Day Goals**
- ✅ Connect to ShredStream
- ✅ Parse clean prices (30-100 tokens)
- 🎯 Detect 1-5 arbitrage opportunities
- 🎯 Execute 1-3 profitable trades
- 🎯 Net profit: >0 SOL

---

## 🔍 Monitoring

### **Log File**
```bash
tail -f /tmp/mev_live.log | grep -E "(🎯 FOUND|Est Profit|EXECUTING|FAILED)"
```

### **Wallet Balance**
```bash
curl -s -X POST https://api.mainnet-beta.solana.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getBalance","params":["CWfwucpmfQveUY8D14SEuc5YK6BbVt4EhATroznU7ktT"]}' \
  | python3 -c "import sys, json; data=json.load(sys.stdin); print(f'Balance: {data[\"result\"][\"value\"]/1e9:.4f} SOL')"
```

### **Process Status**
```bash
ps aux | grep elite_mev | grep -v grep
```

---

## ⚠️ WARNING: LIVE TRADING

This bot is trading with REAL MONEY on mainnet. All safety mechanisms are active, but losses are possible.

**Circuit Breaker**: Bot will automatically stop if it loses 0.2 SOL (20% of capital).

**Emergency Stop**:
```bash
pkill -f elite_mev_bot_v2_1_production
```

---

## 📁 Related Documentation

- `PROFIT_THRESHOLD_UNIFIED.md` - Details on profit threshold fix
- `MEV_WALLET_SETUP.md` - Wallet setup and funding
- `.env.mev_production` - Configuration file
- `/home/tom14cat14/CLAUDE.md` - Main context documentation

---

**Last Updated**: 2025-10-06
**Next Review**: After first trade or 24 hours
