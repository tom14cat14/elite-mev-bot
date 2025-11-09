# MEV Bot Profit Math Analysis

**Date**: 2025-11-08
**Status**: 0.1 SOL wallet - Analyzing minimum profitable trade requirements

---

## 💰 Complete Fee Structure

### **Fixed Fees (per trade)**
| Fee Type | Amount (SOL) | Amount (lamports) | Notes |
|----------|-------------|-------------------|-------|
| **Gas Fee** | 0.0001 | 100,000 | Base transaction fee |
| **Compute Fee** | 0.00002 | 20,000 | Priority compute units |
| **JITO Tip** | 0.001 - 0.005 | 1M - 5M | 99th percentile, capped at 0.005 SOL |

### **Variable Fees (% of position)**
| Fee Type | Percentage | On 0.05 SOL | On 0.1 SOL | On 0.5 SOL |
|----------|-----------|-------------|-----------|------------|
| **DEX Fees** | 2.5% | 0.00125 SOL | 0.0025 SOL | 0.0125 SOL |

### **Fee Buffers & Multipliers**
- **Fee Buffer**: 1.2x (20% safety margin on all estimates)
- **Profit Margin Required**: 2.0x total fees (net profit must be 2x fees)

---

## 📊 Minimum Profitable Trade Analysis

### **Scenario 1: 0.05 SOL Position (Current Minimum)**

**Total Fees**:
- DEX fees: 0.05 × 0.025 = **0.00125 SOL**
- Gas + Compute: **0.00012 SOL**
- JITO tip (avg): **0.003 SOL**
- **Subtotal**: 0.00437 SOL
- **With 1.2x buffer**: 0.00524 SOL

**Profit Requirements**:
- Required margin (2x fees): **0.01048 SOL**
- **Minimum gross profit needed**: 0.01572 SOL
- **Required return**: 31.4% on position

**Reality Check**: ❌
- To profit 0.01572 SOL on a 0.05 SOL position requires **31.4% price movement**
- PumpFun sandwich attacks rarely yield >10-15% per trade
- **CONCLUSION**: 0.05 SOL positions are NOT profitable with current fee structure

---

### **Scenario 2: 0.1 SOL Position**

**Total Fees**:
- DEX fees: 0.1 × 0.025 = **0.0025 SOL**
- Gas + Compute: **0.00012 SOL**
- JITO tip (avg): **0.003 SOL**
- **Subtotal**: 0.00562 SOL
- **With 1.2x buffer**: 0.00674 SOL

**Profit Requirements**:
- Required margin (2x fees): **0.01348 SOL**
- **Minimum gross profit needed**: 0.02022 SOL
- **Required return**: 20.2% on position

**Reality Check**: ⚠️
- To profit 0.02022 SOL on 0.1 SOL requires **20% price movement**
- Marginal - requires very good sandwich opportunities
- **CONCLUSION**: 0.1 SOL positions are BARELY profitable, high risk

---

### **Scenario 3: 0.2 SOL Position**

**Total Fees**:
- DEX fees: 0.2 × 0.025 = **0.005 SOL**
- Gas + Compute: **0.00012 SOL**
- JITO tip (avg): **0.003 SOL**
- **Subtotal**: 0.00812 SOL
- **With 1.2x buffer**: 0.00974 SOL

**Profit Requirements**:
- Required margin (2x fees): **0.01948 SOL**
- **Minimum gross profit needed**: 0.02922 SOL
- **Required return**: 14.6% on position

**Reality Check**: ✅
- To profit 0.02922 SOL on 0.2 SOL requires **14.6% price movement**
- Achievable with good sandwich opportunities (10-20% typical)
- **CONCLUSION**: 0.2 SOL is the MINIMUM truly profitable position size

---

### **Scenario 4: 0.5 SOL Position (Recommended)**

**Total Fees**:
- DEX fees: 0.5 × 0.025 = **0.0125 SOL**
- Gas + Compute: **0.00012 SOL**
- JITO tip (avg): **0.003 SOL**
- **Subtotal**: 0.01562 SOL
- **With 1.2x buffer**: 0.01874 SOL

**Profit Requirements**:
- Required margin (2x fees): **0.03748 SOL**
- **Minimum gross profit needed**: 0.05622 SOL
- **Required return**: 11.2% on position

**Reality Check**: ✅✅
- To profit 0.05622 SOL on 0.5 SOL requires **11.2% price movement**
- Comfortably achievable with typical sandwich opportunities
- Multiple 0.5 SOL positions possible with 1+ SOL wallet
- **CONCLUSION**: 0.5 SOL is OPTIMAL for consistent profitability

---

### **Scenario 5: 1.0 SOL Position (Aggressive)**

**Total Fees**:
- DEX fees: 1.0 × 0.025 = **0.025 SOL**
- Gas + Compute: **0.00012 SOL**
- JITO tip (high): **0.005 SOL**
- **Subtotal**: 0.03012 SOL
- **With 1.2x buffer**: 0.03614 SOL

**Profit Requirements**:
- Required margin (2x fees): **0.07228 SOL**
- **Minimum gross profit needed**: 0.10842 SOL
- **Required return**: 10.8% on position

**Reality Check**: ✅✅✅
- To profit 0.10842 SOL on 1.0 SOL requires **10.8% price movement**
- Easily achievable with good sandwich opportunities
- Higher absolute profits per trade
- **CONCLUSION**: 1.0 SOL positions are HIGHLY profitable when opportunities arise

---

## 🎯 Recommendations by Wallet Balance

### **Current Balance: 0.1 SOL** ❌ NOT VIABLE
- **Problem**: Cannot reach 0.2 SOL minimum profitable position
- **Max position**: 0.1 SOL (with 0 reserve) or 0.05 SOL (with 0.05 reserve)
- **Required return**: 20-31% per trade (unrealistic)
- **Recommendation**: **Add at least 0.5 SOL to wallet**

### **Wallet Balance: 0.6 SOL** ✅ MINIMUM VIABLE
- **Usable capital**: 0.5 SOL (keep 0.1 SOL reserve)
- **Position size**: 0.2-0.5 SOL per trade
- **Required return**: 11-15% per trade
- **Trades per cycle**: 1 position at a time
- **Expected profit**: 0.02-0.05 SOL per successful trade
- **Recommendation**: Good starting point for testing

### **Wallet Balance: 1.5 SOL** ✅✅ OPTIMAL
- **Usable capital**: 1.4 SOL (keep 0.1 SOL reserve)
- **Position size**: 0.5 SOL per trade
- **Required return**: 11% per trade
- **Trades per cycle**: 2-3 positions simultaneously
- **Expected profit**: 0.05+ SOL per successful trade
- **Recommendation**: **RECOMMENDED - Sweet spot for profitability**

### **Wallet Balance: 3.0 SOL** ✅✅✅ AGGRESSIVE
- **Usable capital**: 2.9 SOL (keep 0.1 SOL reserve)
- **Position size**: 0.5-1.0 SOL per trade
- **Required return**: 10-11% per trade
- **Trades per cycle**: 3-6 positions simultaneously
- **Expected profit**: 0.05-0.10 SOL per successful trade
- **Recommendation**: Maximum profitability with diversification

---

## 💡 Key Insights

### **Fixed Costs Dominate Small Trades**
- JITO tips (0.001-0.005 SOL) are relatively large compared to small positions
- Gas fees (0.00012 SOL) become significant overhead on tiny positions
- **Conclusion**: Bigger positions = better efficiency (fees as % of position decrease)

### **The 2x Margin Rule is STRICT**
- Net profit must be **2x total fees** (50% margin)
- This is conservative but protects against:
  - Slippage
  - Price volatility
  - Failed transactions
  - Uncle bundles (JITO tip paid, trade failed)

### **Minimum Viable Position: 0.2 SOL**
- Below 0.2 SOL, required returns are too high (>15%)
- At 0.2 SOL, need 14.6% return (achievable but tight)
- At 0.5 SOL, need 11.2% return (comfortable)
- **Sweet spot**: 0.5-1.0 SOL positions

### **JITO Tips Scale with Competition**
- During high activity: 99th percentile could be 0.005 SOL
- During low activity: Could be 0.001 SOL
- Fee calculations use 0.003 SOL average (conservative)

---

## 🚨 Bottom Line

**With 0.1 SOL wallet**: ❌ **NOT PROFITABLE**
- Cannot make minimum 0.2 SOL positions
- Fees will eat all profits
- **Action needed**: Add at least 0.5 SOL (preferably 1.5 SOL)

**Recommended funding**:
- **Minimum**: 0.6 SOL (0.5 tradeable + 0.1 reserve)
- **Optimal**: 1.5 SOL (1.4 tradeable + 0.1 reserve)
- **Aggressive**: 3.0 SOL (2.9 tradeable + 0.1 reserve)

---

**Last Updated**: 2025-11-08 07:15 UTC
**Next Review**: After funding wallet
