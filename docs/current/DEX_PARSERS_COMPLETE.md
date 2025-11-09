# DEX Parsers Implementation - Complete

**Date**: 2025-11-09
**Status**: ✅ ALL PARSERS IMPLEMENTED

---

## Overview

All DEX binary transaction parsers are now implemented with correct Anchor discriminators based on official IDL files and documentation.

## Implemented Parsers

### 1. **Raydium AMM V4** (Already working)
- **Program ID**: `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`
- **Type**: Non-Anchor (byte discriminator)
- **Discriminator**: Byte 0 = `0x09` (SwapBaseIn)
- **Status**: ✅ Fully functional

### 2. **Raydium CLMM** (Concentrated Liquidity)
- **Program ID**: `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK`
- **Type**: Anchor
- **Discriminator**: `[0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]` (sha256("global:swap")[:8])
- **Status**: ✅ Implemented

### 3. **Raydium CPMM** (Constant Product)
- **Program ID**: `CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C`
- **Type**: Non-Anchor (byte discriminator)
- **Discriminator**: Byte 0 = `0x09` (swap_base_input)
- **Data offset**: Amounts start at offset 1 (no 8-byte Anchor discriminator)
- **Status**: ✅ Implemented

### 4. **Orca Whirlpools**
- **Program ID**: `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`
- **Type**: Anchor
- **Discriminator**: `[0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]` (sha256("global:swap")[:8])
- **Status**: ✅ Implemented

### 5. **Meteora DLMM**
- **Program ID**: `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo`
- **Type**: Anchor
- **Discriminator**: `[0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]` (sha256("global:swap")[:8])
- **Note**: Also has "swap2" variant with different discriminator (not implemented)
- **Status**: ✅ Implemented

### 6. **PumpSwap** (Bonding Curve)
- **Program ID**: `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`
- **Type**: Anchor
- **Discriminators**:
  - Buy: `[102, 6, 61, 18, 1, 218, 235, 234]`
  - Sell: `[51, 230, 133, 164, 1, 127, 131, 173]`
- **Status**: ✅ Implemented

---

## Key Insight: Anchor Discriminators

**Three DEXs use the SAME discriminator** because they all use Anchor and name their swap instruction "swap":
- Raydium CLMM: `sha256("global:swap")[:8]`
- Orca Whirlpools: `sha256("global:swap")[:8]`
- Meteora DLMM: `sha256("global:swap")[:8]`

This is standard Anchor behavior - the discriminator is a hash of the instruction name.

---

## Jupiter V6 (NOT IMPLEMENTED)

**Reason**: Jupiter is an aggregator (too slow for MEV)
- Routes through other DEXs (which we already detect directly)
- Adds routing latency incompatible with same-block MEV execution
- **User feedback**: "you cant use jup for mev, it is too slow"

---

## Implementation Details

### Parser Functions
All parsers are in `src/mev_sandwich_detector.rs`:
- `parse_raydium_amm_v4_swap()` - Line 147-179
- `parse_raydium_clmm_swap()` - Line 181-236
- `parse_raydium_cpmm_swap()` - Line 238-284
- `parse_orca_whirlpool_swap()` - Line 286-344
- `parse_meteora_dlmm_swap()` - Line 346-404
- `parse_pumpswap_swap()` - Line 406-454

### Match Statement
Parsers are called in `analyze_transaction()` - Line 560-573:
```rust
let swap_details = match dex_name {
    "Raydium_AMM_V4" => parse_raydium_amm_v4_swap(message, instruction),
    "Raydium_CLMM" => parse_raydium_clmm_swap(message, instruction),
    "Raydium_CPMM" => parse_raydium_cpmm_swap(message, instruction),
    "Orca_Whirlpools" => parse_orca_whirlpool_swap(message, instruction),
    "Meteora_DLMM" => parse_meteora_dlmm_swap(message, instruction),
    "PumpSwap" => parse_pumpswap_swap(message, instruction),
    _ => None
};
```

---

## Sources

### Research
- **Raydium IDL**: https://github.com/raydium-io/raydium-idl
- **Orca Whirlpools**: https://github.com/orca-so/whirlpools
- **Meteora DLMM SDK**: https://github.com/MeteoraAg/dlmm-sdk
- **PumpSwap IDL**: https://gist.github.com/Taylor123/dcd9f3285ca105efdcdf98089a2b3198

### User Guide
- Comprehensive discriminator extraction guide provided by user
- Anchor discriminator calculation: `sha256("global:instruction_name")[:8]`
- Direct DEX instruction parsing (no routing/aggregation delay)

---

## Build Status

**Latest Build**: 2025-11-09
**Status**: ✅ Successful (19.39s)
**Warnings**: 12 (non-critical, mainly unused variables)
**Errors**: 0

---

## Next Steps

1. **Test parsers with live data** - Run bot and verify parsing works
2. **Monitor logs** - Check for "✅ Parsed swap details" messages
3. **Validate account structures** - Ensure account indices match actual transactions
4. **Test execution** - Verify bot can execute on all DEX types

---

## Expected Behavior

When a swap is detected:
1. Bot identifies DEX type (e.g., "Raydium_CLMM")
2. Calls corresponding parser function
3. Parser checks discriminator (returns `None` if wrong)
4. Parser extracts: pool address, user accounts, amounts
5. Bot logs: `✅ Parsed swap details | Pool: XXX | Amount: YYY lamports`
6. If profitable: Attempts sandwich execution

If parser returns `None`:
- Bot logs: `⚠️ Could not parse swap details for {dex_name} (using estimates)`
- Profitability check still runs with estimated amounts
- Execution may be skipped if details are required

---

**Status**: Ready for live testing with real ShredStream data.
