# Back-Running Strategy - JIT Arbitrage

**Status**: READY FOR IMPLEMENTATION
**Date**: 2025-11-08
**Strategy Type**: Just-In-Time (JIT) Arbitrage / Back-Running

---

## 🎯 Strategy Overview

### **What We're Building**

**Back-Running (JIT Arbitrage)**: Execute arbitrage trades immediately after detecting large swaps that create temporary price imbalances.

**NOT** traditional sandwich attacks:
- ❌ No front-running
- ❌ No victim transaction inclusion
- ❌ No mempool monitoring (Jito public mempool shut down March 2024)

**IS** profitable MEV extraction:
- ✅ Detect swaps via ShredStream (sub-ms latency)
- ✅ Identify profitable arbitrage opportunities
- ✅ Execute reverse trades to capture price impact
- ✅ Submit via JITO for priority execution

---

## 💰 How It Works

### **Example Trade Flow**

1. **Detection** (ShredStream)
   ```
   Victim swaps 10 SOL → TOKEN on Raydium Pool A
   Price impact: TOKEN price increases 2%
   ```

2. **Opportunity Analysis**
   ```
   Pool A: TOKEN overpriced (due to large buy)
   Pool B or Jupiter: TOKEN still at original price
   Arbitrage opportunity: ~2% profit
   ```

3. **Execution** (Our Trade)
   ```
   Buy TOKEN on Pool B/Jupiter (cheaper)
   Sell TOKEN on Pool A (expensive)
   Profit: Price difference - fees
   ```

4. **Submission** (JITO)
   ```
   Submit single transaction via JITO
   Pay competitive tip (0.001-0.01 SOL)
   Land in same block or next block
   ```

---

## 📊 Profitability Model

### **Revenue Sources**

1. **Price Impact Arbitrage**
   - Large swaps create temporary price differences
   - We exploit the difference before it normalizes
   - Typical profit: 0.5-5% per trade

2. **Cross-Pool Arbitrage**
   - Same token priced differently across DEXs
   - Execute triangular arbitrage
   - Lower competition than traditional arb

3. **Failed Transaction Opportunities**
   - Reverted swaps create temporary imbalances
   - Near-zero competition
   - High success rate

### **Cost Structure**

| Cost Item | Amount | % of Trade |
|-----------|--------|------------|
| JITO Tip | 0.001-0.01 SOL | 5-10% |
| Gas Fees | ~0.0001 SOL | <1% |
| DEX Fees | 0.25-1% | Variable |
| **Total** | ~0.0015-0.015 SOL | 6-12% |

**Minimum Viable Trade**: 0.05 SOL position, 2% profit = 0.001 SOL net profit

### **Expected Performance**

| Metric | Conservative | Aggressive |
|--------|-------------|------------|
| Opportunities/hour | 20 | 100 |
| Win rate | 60% | 80% |
| Avg profit/trade | $50 | $200 |
| Daily profit | $600 | $3,200+ |

---

## 🏗️ Technical Implementation

### **Infrastructure (All Ready ✅)**

- ✅ ShredStream detection (sub-ms latency)
- ✅ Transaction parsing (Raydium AMM V4)
- ✅ Pool state queries (all required accounts)
- ✅ Token account management (ATA creation)
- ✅ Swap instruction builder (Raydium)
- ✅ JITO client (bundle submission)

### **Code Changes Required**

**BEFORE (Sandwich)**:
```rust
// Build 3-tx bundle: [front-run, victim, back-run]
let frontrun_tx = build_frontrun_swap(...);
let backrun_tx = build_backrun_swap(...);
let bundle = vec![frontrun_tx, victim_tx, backrun_tx];
```

**AFTER (Back-Running)**:
```rust
// Build 1-tx arbitrage: [our trade only]
let arbitrage_tx = build_arbitrage_swap(...);
let bundle = vec![arbitrage_tx];
```

### **Execution Flow**

```
1. ShredStream detects large swap (sub-ms)
   ↓
2. Parse transaction details (pool, amounts, tokens)
   ↓
3. Calculate arbitrage opportunity
   - Check cross-pool prices
   - Calculate profit after fees
   - Validate minimum profit threshold
   ↓
4. Build arbitrage transaction
   - Buy on cheap pool
   - Sell on expensive pool
   - OR: Buy on Pool A, sell on Pool B
   ↓
5. Submit via JITO
   - Single transaction
   - Competitive tip based on profit
   - Fast confirmation
   ↓
6. Verify execution
   - Check balance change
   - Log actual vs expected profit
   - Update success metrics
```

---

## 🎮 Strategy Variants

### **1. Direct Back-Running (Primary)**
- Detect large buy → Execute reverse arbitrage
- Highest profit potential
- Requires fast execution

### **2. Cross-DEX Arbitrage**
- Same token, different prices across DEXs
- Lower competition
- Smaller profits but higher win rate

### **3. Momentum Copy-Trading**
- Whale buys → We buy immediately
- Ride the momentum
- Exit on next price movement

### **4. Failed TX Exploitation**
- Transaction reverts → Price returns to original
- Arbitrage the reversion
- Very low competition

---

## ⚠️ Risk Management

### **Circuit Breakers**

1. **Daily Loss Limit**: 1.0 SOL
2. **Per-Trade Maximum**: 2.0 SOL
3. **Minimum Net Profit**: 0.001 SOL after all fees
4. **Win Rate Monitoring**: Stop if <50% over 100 trades

### **Safety Features**

- All transactions signed and verified
- Balance checks before/after each trade
- JITO tip caps (max 0.01 SOL)
- Automatic position sizing based on wallet balance

---

## 🚀 Deployment Plan

### **Phase 1: Code Update (2-3 hours)**
- Remove sandwich logic
- Implement pure arbitrage
- Update profitability calculations
- Add cross-pool price checking

### **Phase 2: Testing (1-2 hours)**
- Backtest on last 24h of ShredStream data
- Simulate 100+ opportunities
- Verify profit calculations
- Check execution latency

### **Phase 3: Soft Launch (24 hours)**
- Deploy with 0.1 SOL capital
- Monitor first 50 trades
- Verify actual vs expected profits
- Adjust parameters based on results

### **Phase 4: Scale (ongoing)**
- Increase capital gradually
- Add more DEX support
- Optimize execution speed
- Tune tip bidding strategy

---

## 📈 Success Metrics

### **Must-Have**
- Win rate: >60%
- Avg profit per trade: >0.002 SOL
- Daily profit: >0.1 SOL
- Execution latency: <50ms

### **Nice-to-Have**
- Win rate: >80%
- Avg profit per trade: >0.01 SOL
- Daily profit: >1.0 SOL
- Execution latency: <20ms

---

## 🔄 Future Upgrades

### **Short-Term (1-2 weeks)**
- Add more DEX support (Orca, Meteora)
- Implement cross-pool arbitrage
- Optimize JITO tip bidding
- Add profit tracking dashboard

### **Medium-Term (1-2 months)**
- Multi-hop arbitrage routes
- Machine learning for opportunity prediction
- Automated parameter tuning
- Integration with Jupiter aggregator

### **Long-Term (3-6 months)**
- Private mempool access (if available)
- Validator partnerships
- Multi-token arbitrage
- Advanced MEV strategies

---

## 📚 References

- [Solana MEV Report - Helius](https://www.helius.dev/blog/solana-mev-report)
- [JITO Bundles Guide - QuickNode](https://blog.quicknode.com/solana-mev-economics-jito-bundles-liquid-staking-guide/)
- [Sandwich Attacks on Solana - Medium](https://medium.com/coinmonks/exploring-sandwich-attacks-on-solana-442034afc80e)

---

**Status**: Ready for implementation
**Risk Level**: Low (using existing infrastructure)
**Expected ROI**: High (proven strategy on Solana)
**Time to Deployment**: <24 hours
