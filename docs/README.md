# Shared Bot Infrastructure

Complete infrastructure for MEV and Arbitrage bots with ShredStream integration and Jupiter Ultra API rate limiting.

## 🏗️ Architecture

### Data Flow
```
ShredStream (Price Discovery) → Opportunity Detection → Fee Calculation → Jupiter Ultra (Execution)
```

### Components
- **ShredStream Client**: Real-time Solana transaction data
- **Jupiter Rate Limiter**: 50 req/10sec with exponential backoff
- **Dynamic Fee Model**: Tiered profit/fee calculations
- **Bot Coordinator**: Shared resource management
- **Jupiter Executor**: Execution-only (no price discovery)

## 📊 Dynamic Fee Model

### Profit Tiers
| Range | Multiplier | Gas % | Max Tip | Priority |
|-------|------------|-------|---------|----------|
| 0.5-1 SOL | 1.2x | 8% | 0.05 SOL | 3/10 |
| 1-3 SOL | 1.15x | 10% | 0.15 SOL | 5/10 |
| 3+ SOL | 1.1x | 12% | 0.5 SOL | 8/10 |

### Logic
- **Base Rule**: Net profit must be ≥ 1.2x total fees minimum
- **Gas/Tip**: Calculated as percentage of gross profit
- **Jito Tips**: Dynamically adjusted for congestion + competition
- **DEX Fees**: Added to gas costs for total fee calculation