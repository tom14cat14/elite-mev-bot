# PumpFun vs PumpSwap - Complete Clarification

**Date**: 2025-11-10
**Status**: Terminology Corrected + PumpSwap Integration Needed
**Grok Session**: 20251110_043649

---

## Executive Summary

We had **naming confusion** in our code:
- What we call "PumpSwap" is actually **PumpFun bonding curve** ✅ (working)
- The real **PumpSwap DEX** (post-migration) is **NOT implemented** ❌ (needs to be added)

---

## The Two Different Systems

### 1. PumpFun Bonding Curve (What We Have) ✅

**Program ID**: `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`
**Purpose**: Launches new memecoins via bonding curve mechanism
**Pool Index**: Account index **3** (bonding_curve account)
**Status in Our Bot**: ✅ **WORKING** - Currently labeled as "pumpswap" (should be renamed)

**What It Does**:
- New tokens launch here with bonding curve pricing
- Users buy/sell against the curve
- Tokens graduate when curve reaches 65% completion (~$69K market cap)
- **This is where sandwich attacks happen during launch phase**

**Our Code** (mev_sandwich_detector.rs):
```rust
// Line 36 - MISLABELED as "pumpswap" but is actually PumpFun bonding curve
pumpswap: Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap(),

// Line 535 - Extracts bonding curve from index 3
let bonding_curve = accounts.get(instruction.accounts[3] as usize)?;
info!("✅ EXTRACTED POOL: {} | DEX: PumpSwap | From ix accounts[3]", bonding_curve);
```

### 2. PumpSwap DEX (What We Need) ❌

**Program ID**: `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA`
**Launch Date**: March 2025
**Purpose**: Post-migration DEX where graduated tokens trade
**Pool Index**: Account index **0** (standard AMM)
**Status in Our Bot**: ❌ **NOT IMPLEMENTED**

**What It Does**:
- Tokens automatically migrate here after bonding curve graduation
- Constant product AMM (like Uniswap V2 / Raydium V4)
- Initial liquidity comes from bonding curve final state
- Zero migration fee, instant migration
- **This is where sandwich attacks happen post-migration**

---

## Migration Flow

```
Token Launch → PumpFun Bonding Curve (6EF8...)
               Users buy/sell on curve
               ↓ (reaches 65% / ~$69K cap)
            GRADUATION
               ↓
        PumpSwap DEX (pAMMBay...) ← WE NEED TO ADD THIS
        Post-migration trading
```

**Before March 2025**: Tokens migrated to Raydium
**After March 2025**: Tokens migrate to PumpSwap DEX

---

## PumpSwap DEX Integration Details (From Grok)

### Account Structure

| Index | Account | Description | Writable? | Signer? |
|-------|---------|-------------|-----------|---------|
| **0** | `pool` | Pool state account (PDA) | Yes | No |
| 1 | `pool_vault_a` | WSOL reserve | Yes | No |
| 2 | `pool_vault_b` | Token reserve | Yes | No |
| 3 | `pool_authority` | PDA authority | No | No |
| 4 | `user_source` | User input ATA | Yes | No |
| 5 | `user_destination` | User output ATA | Yes | No |
| 6 | `user` | Signer wallet | No | Yes |
| 7 | `token_program` | SPL Token | No | No |
| 8 | `associated_token_program` | ATA program | No | No |
| 9 | `system_program` | System program | No | No |
| 10 | `fee_recipient` | Protocol fee sink | Yes | No |

**Total**: 11 accounts (vs Raydium V4's 15+)

### Instruction Discriminator

**Buy (SOL → Token)**:
`[0x42, 0x3f, 0xa1, 0x12, 0x00, 0x00, 0x00, 0x00]`

**Sell (Token → SOL)**:
`[0x42, 0x3f, 0xa1, 0x12, 0x01, 0x00, 0x00, 0x00]`

### Data Format

```
Bytes 0-7:   Discriminator (see above)
Bytes 8-15:  u64 input_amount
Bytes 16-23: u64 min_output_amount
Byte 24:     u8 direction (0x00 = buy, 0x01 = sell)
```

### Pool PDA Derivation

```rust
let pool_pda = find_program_address(
    [b"pool", token_mint.as_ref()],
    PUMPSWAP_PROGRAM_ID
);
```

---

## Key Differences: PumpSwap vs Raydium V4

| Feature | PumpSwap DEX | Raydium V4 |
|---------|--------------|------------|
| **Serum Integration** | ❌ No (pure CPMM) | ✅ Yes (hybrid AMM/orderbook) |
| **Account Count** | 11 accounts | 15+ accounts |
| **Fee Structure** | 0.3% total (0.25% LP + 0.05% protocol) | 0.25% total |
| **Fee Recipient** | Required at index 10 | No separate recipient |
| **Liquidity** | Auto-bootstrapped from bonding curve | Manual pool creation |
| **Initial Price** | Matches bonding curve final price | Set by pool creator |
| **LP Lock** | 24h post-migration | No lock |
| **Compute Units** | ~20k less (no Serum) | Higher (Serum overhead) |

---

## What We Need To Do

### 1. Rename Existing Code (Fix Confusion)

**In `mev_sandwich_detector.rs`**:
```rust
// OLD (WRONG NAME):
pub pumpswap: Pubkey,  // Line 24

// NEW (CORRECT NAME):
pub pumpfun_bonding_curve: Pubkey,
```

**Update all references**:
- `parse_pumpswap_swap()` → `parse_pumpfun_bonding_curve_swap()`
- `"PumpSwap"` string literals → `"PumpFun_BondingCurve"`

### 2. Add PumpSwap DEX Support (New Code)

**Add to `DexPrograms` struct**:
```rust
pub pumpswap_dex: Pubkey,  // NEW - the actual post-migration DEX
```

**Initialize**:
```rust
pumpswap_dex: Pubkey::from_str("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA").unwrap(),
```

**Add parser function**:
```rust
fn parse_pumpswap_dex_swap(
    message: &VersionedMessage,
    instruction: &solana_sdk::instruction::CompiledInstruction,
) -> Option<(String, String, String, u64, u64)> {
    // Validate discriminator (buy or sell)
    if instruction.data.len() < 25 {
        return None;
    }

    let discriminator = &instruction.data[0..8];
    let is_buy = discriminator == [0x42, 0x3f, 0xa1, 0x12, 0x00, 0x00, 0x00, 0x00];
    let is_sell = discriminator == [0x42, 0x3f, 0xa1, 0x12, 0x01, 0x00, 0x00, 0x00];

    if !is_buy && !is_sell {
        return None;
    }

    // Validate account count (11 minimum)
    if instruction.accounts.len() < 11 {
        return None;
    }

    let accounts = message.static_account_keys();

    // ✅ GROK VERIFIED: Pool at index 0 (same as Raydium/Orca/etc)
    let pool_address = accounts.get(instruction.accounts[0] as usize)?;
    info!("✅ EXTRACTED POOL: {} | DEX: PumpSwap_DEX | From ix accounts[0]", pool_address);

    // Parse amounts from instruction data
    let amount_in = u64::from_le_bytes(instruction.data[8..16].try_into().ok()?);
    let min_amount_out = u64::from_le_bytes(instruction.data[16..24].try_into().ok()?);

    // Determine mints based on direction
    let input_mint = if is_buy {
        "WSOL" // SOL wrapped to WSOL
    } else {
        "TOKEN" // Base token being sold
    };

    let output_mint = if is_buy {
        "TOKEN" // Base token being bought
    } else {
        "WSOL" // Receiving SOL
    };

    Some((
        pool_address.to_string(),
        input_mint.to_string(),
        output_mint.to_string(),
        amount_in,
        min_amount_out,
    ))
}
```

**Add to DEX identification**:
```rust
else if program_id == &self.pumpswap_dex {
    Some("PumpSwap_DEX")
}
```

**Add to swap parsing**:
```rust
"PumpSwap_DEX" => parse_pumpswap_dex_swap(message, instruction),
```

### 3. Update Documentation

**Update `GROK_COLLABORATION_POOL_EXTRACTION.md`**:
- Fix terminology (PumpFun vs PumpSwap)
- Add PumpSwap DEX as 7th supported DEX type
- Update pool index table

---

## Expected Trading Volume

According to research:
- **PumpSwap DEX**: Launched March 2025, growing adoption
- **Graduated tokens**: ~$69K initial liquidity per pool
- **Migration frequency**: Continuous (tokens graduating 24/7)
- **Opportunity**: Catch post-migration sandwich attacks

**Combined MEV Coverage**:
- **PumpFun Bonding Curve**: Pre-migration trades (launch phase)
- **PumpSwap DEX**: Post-migration trades (established phase)
- **Total**: Complete lifecycle coverage

---

## Testing Plan

1. **Verify PumpSwap DEX on-chain**:
   ```bash
   solana account pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA
   ```

2. **Monitor for PumpSwap swaps** (ShredStream):
   - Look for program ID `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA`
   - Verify discriminator matches Grok's spec

3. **Paper test integration**:
   - Add PumpSwap DEX parser
   - Run bot in paper trading mode
   - Verify pool extraction logs

4. **Live deployment** (after testing):
   - Same wallet as other DEX types
   - Monitor initial 10-20 sandwich attempts
   - Validate profitability

---

## Summary

| Item | Status | Program ID |
|------|--------|------------|
| **PumpFun Bonding Curve** | ✅ Working (mislabeled) | `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P` |
| **PumpSwap DEX** | ❌ Not Implemented | `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA` |

**Next Steps**:
1. Rename "pumpswap" → "pumpfun_bonding_curve" (fix confusion)
2. Add PumpSwap DEX support (new DEX type)
3. Update documentation
4. Test in paper trading mode
5. Deploy to live trading

**Priority**: Medium-High
**Effort**: 2-3 hours
**Impact**: Expands MEV coverage to post-migration PumpFun tokens

---

**References**:
- Grok Session: `grok_answer_20251110_043649.md`
- PumpSwap Announcement: March 2025
- Pool Extraction Doc: `GROK_COLLABORATION_POOL_EXTRACTION.md`
