# PumpFun Sandwich Attack Strategy - Complete Explanation

## 🎯 What is a Sandwich Attack?

A sandwich attack is an MEV (Maximal Extractable Value) strategy where you profit by "sandwiching" a victim's transaction between your own front-run and back-run transactions.

### The 3-Step Process

```
BEFORE (Bonding Curve Price: 1.0 SOL per token)
├─ Our wallet: 2.0 SOL
└─ Victim's wallet: 1.0 SOL to spend

STEP 1: Front-Run BUY ⬆️
├─ We buy 2.0 SOL worth of tokens
├─ Price moves: 1.0 → 1.15 SOL per token
└─ We hold tokens bought at avg 1.075 SOL

STEP 2: Victim's BUY ⬆️
├─ Victim buys 1.0 SOL worth
├─ Price moves: 1.15 → 1.25 SOL per token
└─ Victim gets fewer tokens (paid higher price)

STEP 3: Back-Run SELL ⬇️
├─ We sell all our tokens at higher price
├─ Price moves back: 1.25 → 1.10 SOL per token
├─ We sell at avg 1.20 SOL per token
└─ Profit: (1.20 - 1.075) × token_amount = ~0.25 SOL

RESULT:
├─ Our profit: 0.25 SOL (12.5% return)
├─ Victim's loss: Got tokens at worse price
└─ Net impact: We extracted value from victim's trade
```

## 🔍 Detection: Finding Profitable Victims

### Target Criteria
- **Minimum Size**: >0.1 SOL buy transactions
- **Expected Price Impact**: >2% move on bonding curve
- **Token**: PumpFun tokens (pre-Raydium migration)
- **Liquidity**: Enough depth for our back-run sell

### PumpFun Program Signature
```rust
const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

// BUY instruction discriminators (first 8 bytes)
const BUY_DISCRIMINATOR: &[u8] = &[0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea];
```

### Detection Flow
```
ShredStream → Parse Entry → Deserialize Transactions →
Check Program ID → Decode Buy Instruction → Extract Amount →
Calculate Price Impact → Assess Profitability → Accept/Reject
```

## 💰 Profitability Calculation

### Formula
```rust
// 1. Calculate victim's price impact on bonding curve
let price_before = bonding_curve.get_current_price();
let price_after = bonding_curve.simulate_buy(victim_amount);
let price_impact = (price_after - price_before) / price_before;

// 2. Front-run sizing (2-3x victim size for optimal extraction)
let front_run_size = if price_impact > 0.05 {
    victim_amount * 2.0  // Large impact = smaller front-run
} else {
    victim_amount * 3.0  // Small impact = larger front-run
};

// 3. Expected profit
let buy_avg_price = simulate_buy(front_run_size);
let sell_avg_price = simulate_sell_after_victim(front_run_size, victim_amount);
let gross_profit = (sell_avg_price - buy_avg_price) * tokens_acquired;

// 4. Subtract fees
let dex_fee = gross_profit * 0.025;  // 2.5% PumpFun fee
let jito_tip = gross_profit * 0.40;  // 40% to win MEV auction
let gas_cost = 0.001;  // ~0.001 SOL for 3 transactions
let net_profit = gross_profit - dex_fee - jito_tip - gas_cost;

// 5. Accept if profitable
return net_profit >= 0.015;  // Min 0.015 SOL profit threshold
```

## 📦 JITO Bundle Structure

### Atomic 3-Transaction Bundle
```rust
Bundle {
    transactions: [
        // Tx #1: Front-run (our buy)
        Transaction {
            instructions: [
                create_associated_token_account(),  // If needed
                pumpfun_buy_instruction(
                    amount: front_run_size,
                    min_tokens: calculated_with_slippage,
                ),
            ],
            signers: [our_wallet],
        },

        // Tx #2: Victim's original transaction (from mempool)
        victim_transaction,

        // Tx #3: Back-run (our sell)
        Transaction {
            instructions: [
                pumpfun_sell_instruction(
                    amount: all_tokens_from_frontrun,
                    min_sol: calculated_with_slippage,
                ),
            ],
            signers: [our_wallet],
        },
    ],
    tip_amount: (net_profit * 0.40) in lamports,  // 40% of profit
}
```

### Why JITO is Critical
1. **Atomicity**: All 3 txs execute or none execute (no partial risk)
2. **Ordering**: Guaranteed execution order (front → victim → back)
3. **MEV Auction**: Tip determines priority vs other bots
4. **No Slippage**: No chance of victim's tx landing before our front-run

## ⚡ Speed Requirements

### Critical Timing
- **Detection to Submission**: <20ms
- **JITO Bundle Acceptance**: <10ms
- **Total Latency Budget**: ~30ms

### Why Speed Matters
```
Block Time: ~400ms per slot
Opportunity Window: ~200ms (before victim tx included)

If we're 50ms slower than competitors:
├─ Their bundle submitted at t=0ms
├─ Our bundle submitted at t=50ms
└─ Result: They win MEV auction, we get nothing
```

### Optimization Points
- **ShredStream**: 0.16ms latency (gRPC streaming)
- **Detection**: <1ms (pattern matching, not full deserialization)
- **Calculation**: <5ms (bonding curve math)
- **Bundle Build**: <10ms (pre-signed templates)
- **JITO Submit**: <5ms (persistent gRPC connection)

## 🛡️ Risk Management

### Safety Checks
1. **Balance Check**: Ensure sufficient SOL for front-run
2. **Position Limits**: Max 80% of wallet per sandwich
3. **Minimum Profit**: 0.015 SOL net profit threshold
4. **Circuit Breaker**: Stop after 5 consecutive failures
5. **Rate Limiting**: 1 bundle per second to JITO

### Failure Scenarios
- **Bundle Rejected**: No loss (just wasted gas)
- **Victim TX Reverts**: Entire bundle reverts (atomicity)
- **Back-run Fails**: We're stuck with tokens (bad scenario!)
- **Outbid by Competitor**: No execution, no loss

## 📊 Expected Performance

### Win Rate Targets
- **Detection Rate**: 100+ victims per minute
- **Submission Rate**: 50-60 bundles/min (after rate limit)
- **Bundle Landing**: 10-30% (depends on tip competitiveness)
- **Profitable Sandwiches**: 5-15 per hour

### Profitability
- **Average Profit**: 0.05-0.15 SOL per sandwich
- **Hourly Target**: 0.5-2.0 SOL
- **Daily Target**: 12-48 SOL
- **ROI per Trade**: 5-20% on capital deployed

## 🚨 Current Issues

### CRITICAL: Fake Data Generation
**Problem**: Bot is generating fake victim transactions instead of parsing real ShredStream data.

**Location**: `src/pumpfun_new_coin_detector.rs:147-197`

**Current Code** (WRONG):
```rust
pub async fn process_shred_data(&mut self, shred_data: &[u8]) -> Result<Vec<NewTokenEvent>> {
    if shred_data.len() > 100 {
        let random_chance = fastrand::u8(..);  // ❌ FAKE
        if random_chance < 50 {
            let test_token = NewTokenEvent {
                mint: Pubkey::new_unique(),  // ❌ FAKE (creates "11111..." addresses)
                quality_score: fastrand::f64() * 3.0 + 7.0,  // ❌ FAKE
                // ...
            };
            opportunities.push(test_token);
        }
    }
    Ok(opportunities)
}
```

**Needed Fix**: Parse real PumpFun BUY transactions:
```rust
pub async fn process_shred_data(&mut self, shred_data: &[u8]) -> Result<Vec<VictimTransaction>> {
    // 1. Deserialize Entry from shred_data
    let entry: Entry = bincode::deserialize(shred_data)?;

    // 2. Parse transactions
    let mut victims = Vec::new();
    for tx in entry.transactions {
        // 3. Check for PumpFun program
        if tx.message.account_keys.contains(&PUMPFUN_PROGRAM_ID) {
            // 4. Decode BUY instruction
            if let Some(buy_data) = parse_buy_instruction(&tx) {
                // 5. Check if worth sandwiching
                if buy_data.amount >= MIN_VICTIM_SIZE {
                    victims.push(VictimTransaction {
                        signature: tx.signature,
                        amount: buy_data.amount,
                        token_mint: buy_data.mint,
                        // ...
                    });
                }
            }
        }
    }
    Ok(victims)
}
```

## 🔧 Implementation Status

### ✅ Working Components
- ShredStream gRPC connection (0.16ms latency)
- JITO bundle encoding (bs58 format)
- Rate limiting (1 req/sec)
- Sandwich engine logic (profit calculation, bundle building)
- PumpFun bonding curve math

### ❌ Broken Components
- **Victim detection**: Generating fake data instead of parsing real transactions
- **Bundle landing rate**: Low (1-5%) due to insufficient tips
- **RPC rate limits**: Many 429 errors

### 🚧 Needed Fixes
1. **PRIORITY 1**: Replace fake data generation with real PumpFun TX parsing
2. **PRIORITY 2**: Increase JITO tips (0.02 → 0.05 SOL max)
3. **PRIORITY 3**: Add multiple RPC endpoints for rate limit mitigation

---

**Last Updated**: 2025-10-07
**Status**: STRATEGY DOCUMENTED, IMPLEMENTATION BROKEN (using fake data)
**Next Step**: Fix victim detection to parse real ShredStream transactions
