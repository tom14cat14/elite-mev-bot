# 🌊 ShredStream Setup Guide

## What is ShredStream?

**ShredStream** is ERPC's real-time blockchain data feed that provides ultra-low latency access to Solana transactions.

- **Latency**: 0.16-15ms (depends on implementation)
- **Protocol**: gRPC-over-HTTPS (NOT raw UDP for our setup)
- **Endpoint**: `https://shreds-ny6-1.erpc.global`
- **Authentication**: IP whitelist (151.243.244.130) - NO API tokens needed
- **SDK**: `solana-stream-sdk` v0.5.1

---

## ✅ Correct Implementation

### **Environment Variables**
```bash
# CORRECT endpoint (include https://)
SHREDS_ENDPOINT=https://shreds-ny6-1.erpc.global

# NO X_TOKEN needed (IP whitelist only)
# Your IP is already whitelisted: 151.243.244.130
```

### **Rust Implementation**
```rust
use solana_stream_sdk::{CommitmentLevel, ShredstreamClient};

// Connect to ShredStream
let mut client = ShredstreamClient::connect(
    "https://shreds-ny6-1.erpc.global"
).await?;

// Subscribe to all accounts/transactions
let stream = client.subscribe_with_commitment(
    CommitmentLevel::Processed  // Fastest commitment level
).await?;

// Process transactions
while let Some(update) = stream.next().await {
    // Parse DEX swaps, extract prices, etc.
}
```

### **Dependencies (Cargo.toml)**
```toml
[dependencies]
solana-stream-sdk = "0.5.1"
tokio = { version = "1", features = ["full"] }
```

---

## ❌ Common Mistakes

### **1. Wrong Protocol**
```rust
// WRONG - Don't use raw UDP sockets
use tokio::net::UdpSocket;
let socket = UdpSocket::bind("0.0.0.0:0").await?;
```
**Why**: ERPC ShredStream uses gRPC-over-HTTPS, not raw UDP.

### **2. Missing HTTPS Prefix**
```bash
# WRONG
SHREDS_ENDPOINT=shreds-ny6-1.erpc.global

# CORRECT
SHREDS_ENDPOINT=https://shreds-ny6-1.erpc.global
```

### **3. Trying to Use API Tokens**
```bash
# WRONG - Not needed for IP whitelist
X_TOKEN=507c3fff-6dc7-4d6d-8915-596be560814f
```
**Why**: Your IP (151.243.244.130) is whitelisted. No tokens needed.

---

## 📊 Performance Benchmarks

### **MEV_Bot Results**
- **Old (UDP socket)**: 52-60ms latency
- **New (gRPC-over-HTTPS)**: 0.16ms latency
- **Improvement**: 325x faster!

### **Arb_Bot Results**
- **Processing**: 4900+ swaps captured
- **Price cache**: 1000+ tokens
- **Latency**: <15ms end-to-end

---

## 🏗️ Architecture Patterns

### **Pattern 1: Direct Integration** (MEV_Bot)
```
Bot → ShredstreamClient → Parse swaps → Detect opportunities → Trade
```
**Pros**: Lowest latency (<1ms)
**Cons**: Code in every bot

### **Pattern 2: Service + REST API** (Arb_Bot)
```
ShredStream Service (port 8080) → REST API
                ↓
        Arb Bot consumes API
```
**Pros**: Reusable, cleaner separation
**Cons**: Extra network hop (~5-10ms)

---

## 🔍 Debugging

### **Test Connection**
```bash
cd /home/tom14cat14/MEV_Bot
cargo run --release --bin test_shredstream_live
# Should see: ✅ Connected successfully
```

### **Common Errors**

**Error**: "Transport error: transport error"
**Solution**: Check endpoint has `https://` prefix

**Error**: "Connection refused"
**Solution**: Verify IP whitelisting (151.243.244.130)

**Error**: "DNS lookup failed"
**Solution**: Check internet connection, try ping:
```bash
ping shreds-ny6-1.erpc.global
```

---

## 📁 Reference Implementations

### **MEV_Bot** (Direct Integration)
- **File**: `/home/tom14cat14/MEV_Bot/src/shredstream_processor.rs`
- **Performance**: 0.16ms latency
- **Status**: ✅ Production

### **Arb_Bot** (Service Pattern)
- **Service**: `/home/tom14cat14/Arb_Bot/shredstream_service/src/main.rs`
- **Consumer**: `/home/tom14cat14/Arb_Bot/clean_arb_bot/`
- **Performance**: <15ms end-to-end
- **Status**: ✅ Production

---

## 🚀 Quick Start

### **For New Bots**

1. **Add dependency**:
```toml
solana-stream-sdk = "0.5.1"
```

2. **Copy working implementation**:
```bash
# Use MEV_Bot as reference
cp /home/tom14cat14/MEV_Bot/src/shredstream_processor.rs your_bot/src/
```

3. **Configure endpoint**:
```bash
echo 'SHREDS_ENDPOINT=https://shreds-ny6-1.erpc.global' >> .env
```

4. **Test**:
```bash
cargo run --release
# Should see swap data flowing
```

---

## 📚 Related Documentation

- **MEV_Bot Implementation**: `/home/tom14cat14/MEV_Bot/BOT_SUMMARY.md`
- **Arb_Bot Implementation**: `/home/tom14cat14/Arb_Bot/SHREDSTREAM_SETUP.md`
- **Data Providers Guide**: `/home/tom14cat14/SOLANA_DATA_PROVIDERS_EXPLAINED.md`
- **ERPC Documentation**: https://docs.erpc.cloud/shredstream

---

## ⚙️ Advanced: 3-Layer Filtering

Used in Arb_Bot to eliminate junk prices:

```rust
// Layer 1: Volume filter (≥0.01 SOL/24h)
if token.volume_24h < 0.01 { continue; }

// Layer 2: Swap count (≥5 swaps/24h)
if token.swap_count_24h < 5 { continue; }

// Layer 3: Price deviation (≤50% from median)
let deviation = (price - median_price).abs() / median_price;
if deviation > 0.5 { continue; }
```

**Result**: 98% junk elimination (3,877 prices → 30-40 clean prices)

**Documentation**: `/home/tom14cat14/Arb_Bot/clean_arb_bot/VOLUME_FILTER_FINDINGS.md`

---

**Status**: ✅ Production-ready
**Last Updated**: 2025-10-08
**Contact**: Use working implementations as reference
