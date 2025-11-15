#!/bin/bash

# =============================================================================
# MICRO-CAP MEV BOT STARTUP SCRIPT
# 4 SOL High-Impact Strategy for Pre-Migration Tokens <1M Market Cap
# =============================================================================

echo "💎 Starting Micro-Cap MEV Bot"
echo "🎯 Target: Pre-migration coins <1M market cap"
echo "💰 Capital: 4 SOL concentrated strategy"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Set working directory
cd "$(dirname "$0")"

# Load micro-cap specific environment
if [ -f ".env.microcap" ]; then
    echo "✅ Loading micro-cap configuration..."
    set -a
    # shellcheck source=/dev/null
    . .env.microcap
    set +a
else
    echo "❌ .env.microcap file not found!"
    exit 1
fi

# Check if binary exists
if [ ! -f "target/debug/microcap_mev_bot" ]; then
    echo "🔨 Building micro-cap MEV bot..."
    cargo build --bin microcap_mev_bot
    if [ "$?" -ne 0 ]; then
        echo "❌ Build failed!"
        exit 1
    fi
fi

# Display configuration
echo "📊 Micro-Cap MEV Configuration:"
echo "   • Max position size: ${MAX_TRADE_SIZE_SOL} SOL"
echo "   • Min position size: ${MIN_TRADE_SIZE_SOL} SOL"
echo "   • Max market cap: $$(echo \"scale=0; ${MAX_MARKET_CAP_USD}/1000\" | bc)K USD"
echo "   • Target price impact: ${TARGET_PRICE_IMPACT_PCT}%"
echo "   • Position timeout: ${POSITION_TIMEOUT_MS}ms"
echo "   • Paper trading: ${PAPER_TRADING}"

# Wallet balance check (if not paper trading)
if [ "${PAPER_TRADING}" = "false" ]; then
    echo "⚠️  LIVE TRADING MODE - Checking wallet balance..."
    # Add wallet balance check here if needed
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 Launching Micro-Cap MEV Bot..."
echo "💡 Press Ctrl+C to stop"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Launch the micro-cap MEV bot
exec ./target/debug/microcap_mev_bot