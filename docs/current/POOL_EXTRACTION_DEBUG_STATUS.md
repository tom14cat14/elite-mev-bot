# Pool Extraction Debug Status

**Date**: 2025-11-10 06:02 UTC - 09:15 UTC
**Status**: ⚠️ SUPERSEDED BY POOL_EXTRACTION_DEBUG_SESSION_20251110.md

**CRITICAL DISCOVERY**: Bot is NOT using mev_sandwich_detector.rs at all!
Real opportunities are being created by an unknown source without pool addresses.

**See**: `POOL_EXTRACTION_DEBUG_SESSION_20251110.md` for complete session notes.

---

## Original Session (2025-11-10 06:02 UTC)

## Current Status

### ✅ Bot Deployment
- **Status**: Running live (PIDs: 2251673, 2251701, 2251703)
- **Opportunities Detected**: 8.0/min
- **Execution Rate**: 0% (ALL pool extractions failing)

### ✅ Raydium CLMM Fix Applied
- **Fix**: Changed pool extraction from `accounts[0]` to `accounts[1]`
- **Reason**: Grok verified that account index 0 is `amm_config` PDA (~264 bytes), index 1 is actual `pool_state` PDA (~1,088 bytes)
- **Status**: Fix applied, rebuilt, deployed
- **Result**: No CLMM errors in current logs (bot detecting very few CLMM swaps currently)

### ❌ PumpSwap DEX - STILL BROKEN
- **Error Frequency**: Majority of pool extraction failures
- **Error Message**: "Unknown DEX program owner: pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA (likely a token account or wallet)"
- **Problem**: Extracting accounts owned by PumpSwap program, but they're NOT pool PDAs
- **Account Sizes**: 117 bytes (too small for a pool - min 200 bytes required)

---

## Error Analysis (Last 50 Errors)

### PumpSwap DEX Failures (~90% of errors)
```
[WARN] ⚠️  Failed to fetch pool state: Unknown DEX program owner: pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA (likely a token account or wallet)
[WARN]    - Pool address is incorrect

[WARN] ⚠️  Failed to fetch pool state: Account size 117 bytes is too small for a pool (min 200 bytes required)
[WARN]    - Pool address is incorrect
```

### Missing Pool Addresses (~10% of errors)
```
[WARN] ⚠️  No pool address in opportunity
```

---

## Root Cause

### PumpSwap DEX Problem
According to `PUMPFUN_VS_PUMPSWAP_CLARIFICATION.md`:
- **Program ID**: `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA`
- **Expected Account Structure** (from Grok):
  - Index 0 = `pool` PDA (~1,088 bytes)
  - Index 1 = `pool_vault_a` (WSOL reserve)
  - Index 2 = `pool_vault_b` (Token reserve)
  - ... (11 accounts minimum)

**Current Extraction** (`mev_sandwich_detector.rs`):
```rust
// Line 196-200 (PumpSwap DEX parser)
let pool_address = accounts.get(instruction.accounts[0] as usize)?;

info!("✅ EXTRACTED POOL: {} | DEX: PumpSwap_DEX | From ix accounts[0]",
      pool_address);
```

**Issue**: We're extracting from `accounts[0]`, which should be the pool PDA according to Grok. But the error messages show we're getting:
1. Accounts owned by PumpSwap program (correct program)
2. But they're token accounts/wallets (117 bytes), not pool PDAs

**Hypothesis**:
- Either the account index is wrong (not index 0)
- OR we need to derive the pool PDA instead of direct extraction
- OR PumpSwap swaps route through a wrapper (similar to Jupiter)

---

## Next Steps

1. **Verify PumpSwap DEX account structure** - Contact Grok again to confirm:
   - Is pool always at index 0 for PumpSwap DEX?
   - Or do we need to derive pool PDA from instruction data?
   - What's the account at index 0 if it's only 117 bytes?

2. **Check if swaps are routed through Jupiter**:
   - Many swaps might be going through Jupiter V6 aggregator
   - If so, we need to parse Jupiter instructions differently
   - Jupiter wraps DEX swaps, so pool is not at a fixed index

3. **Add diagnostic logging** to see what we're actually extracting:
   - Log all account addresses in the instruction
   - Log account sizes and owners
   - Identify which account is the actual pool

4. **Fix missing pool addresses**:
   - Some opportunities have NO pool address
   - Need to track down why pool extraction returns None

---

## Grok Contact History

### Previous Contact (2025-11-10 05:54 UTC)
**Question**: Raydium CLMM pool extraction failing (0-byte accounts)
**Answer**: Pool at index 1, not index 0
**Status**: ✅ FIX APPLIED

### Required Follow-up
**Question**: PumpSwap DEX pool extraction failing (117-byte token accounts instead of pools)
**Details Needed**:
- Confirm pool at index 0 for PumpSwap DEX
- Explain why we're extracting 117-byte accounts
- Provide PDA derivation if needed

---

## Code References

### Pool Extraction Logic
- **File**: `/home/tom14cat14/MEV_Bot/src/mev_sandwich_detector.rs`
- **Raydium CLMM Parser**: Lines 256-262 (✅ FIXED - uses accounts[1])
- **PumpSwap DEX Parser**: Lines 196-200 (❌ BROKEN - uses accounts[0], extracting wrong accounts)

### Documentation
- **PumpSwap Clarification**: `/home/tom14cat14/MEV_Bot/docs/current/PUMPFUN_VS_PUMPSWAP_CLARIFICATION.md`
- **Grok Response**: `/home/tom14cat14/MEV_Bot/grok_answer_20251110_055424.md` (CLMM fix)

---

## Observations

1. **Bot IS detecting opportunities** (8/min across multiple DEXs)
2. **Raydium CLMM fix appears correct** (no CLMM errors in logs)
3. **PumpSwap DEX is the main blocker** (~90% of failures)
4. **Opportunity detection working** but pool extraction failing

---

**Status**: Waiting for next steps - need to debug PumpSwap DEX pool extraction or contact Grok for clarification
