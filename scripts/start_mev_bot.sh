#!/bin/bash
# Start MEV Bot with dedicated wallet configuration

set -e

echo "🤖 Starting MEV Bot (Dedicated Wallet)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Load environment from dedicated MEV config
if [ -f .env.mev_production ]; then
    echo "✅ Loading MEV bot configuration..."
    set -a
    # shellcheck source=/dev/null
    . .env.mev_production
    set +a
else
    echo "❌ Error: .env.mev_production not found!"
    echo "   Run from /home/tom14cat14/MEV_Bot directory"
    exit 1
fi

# Show wallet info
echo ""
echo "🔐 Wallet Info:"
WALLET_ADDRESS=$(python3 get_wallet_address.py | grep "Wallet Address:" | cut -d' ' -f5)
echo "   Address: $WALLET_ADDRESS"
echo ""

# Show trading mode
echo "⚙️  Trading Mode:"
if [ "$PAPER_TRADING" = "true" ]; then
    echo "   ✅ PAPER TRADING (Safe - No real money)"
else
    echo "   ⚠️  LIVE TRADING (Real money at risk!)"
fi

if [ "$ENABLE_REAL_TRADING" = "true" ]; then
    echo "   ⚠️  REAL TRADING ENABLED"
else
    echo "   ✅ Real trading disabled"
fi

echo ""
echo "📊 Parameters:"
echo "   Max Position: $MAX_POSITION_SIZE_SOL SOL"
echo "   Min Profit: $MIN_PROFIT_THRESHOLD_SOL SOL"
echo "   Max Daily Trades: $MAX_DAILY_TRADES"
echo "   Max Daily Loss: $MAX_DAILY_LOSS_SOL SOL"
echo ""

# Confirmation if real trading
if [ "$ENABLE_REAL_TRADING" = "true" ]; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "⚠️  WARNING: You are about to start LIVE TRADING"
    echo "   Real money will be at risk!"
    echo "   Press Ctrl+C to cancel, or wait 10 seconds to continue..."
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    sleep 10
fi

# Build if needed
if [ ! -f "./target/release/elite_mev_bot_v2_1_production" ]; then
    echo "🔨 Building MEV bot..."
    ~/.cargo/bin/cargo build --release --bin elite_mev_bot_v2_1_production
fi

# Start the bot
echo "🚀 Starting MEV bot..."
echo "   Logs: /tmp/mev_bot_$(date +%Y%m%d_%H%M%S).log"
echo "   Press Ctrl+C to stop"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

~/.cargo/bin/cargo run --release --bin elite_mev_bot_v2_1_production
