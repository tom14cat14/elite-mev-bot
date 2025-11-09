#!/bin/bash
# Start MEV Bot with automatic log rotation
# Rotates logs every 5 minutes to prevent disk bloat

cd "$(dirname "$0")/.." || exit 1

# Load environment
set -a
source .env.multidex 2>/dev/null || source .env
set +a

# Ensure rotation script exists
ROTATION_SCRIPT="./scripts/rotate_logs.sh"
if [ ! -f "$ROTATION_SCRIPT" ]; then
    echo "Error: Rotation script not found at $ROTATION_SCRIPT"
    exit 1
fi

# Start log rotation in background (every 5 minutes)
(
    while true; do
        sleep 300  # 5 minutes
        "$ROTATION_SCRIPT"
    done
) &
ROTATION_PID=$!

echo "Log rotation started (PID: $ROTATION_PID)"
echo "Starting MEV Bot..."

# Start the bot
./target/release/elite_mev_bot_v2_1_production > /tmp/mev_multidex.log 2>&1 &
BOT_PID=$!

echo "MEV Bot started (PID: $BOT_PID)"
echo "Logs: /tmp/mev_multidex.log"
echo "Press Ctrl+C to stop"

# Cleanup function
cleanup() {
    echo "Stopping MEV Bot..."
    kill $BOT_PID 2>/dev/null
    kill $ROTATION_PID 2>/dev/null
    wait $BOT_PID 2>/dev/null
    wait $ROTATION_PID 2>/dev/null
    echo "Stopped"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Wait for bot to finish
wait $BOT_PID
