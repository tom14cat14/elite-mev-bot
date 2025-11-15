#!/bin/bash
# MEV Bot Startup Script with Automatic Log Rotation
# Starts bot and schedules automatic log rotation

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MEV_BOT_DIR="/home/tom14cat14/MEV_Bot"

echo "=== MEV Bot Startup Script ==="
echo ""

# Check if bot is already running
if pgrep -f elite_mev_bot_v2_1_production > /dev/null; then
    echo "⚠️  MEV bot is already running (PID: $(pgrep -f elite_mev_bot_v2_1_production))"
    echo "Stop it first with: killall elite_mev_bot_v2_1_production"
    exit 1
fi

# Check if rotation script exists
if [ ! -f "$SCRIPT_DIR/rotate_logs.sh" ]; then
    echo "❌ Log rotation script not found: $SCRIPT_DIR/rotate_logs.sh"
    exit 1
fi

# Add log rotation to crontab (runs every hour)
CRON_LINE="0 * * * * $SCRIPT_DIR/rotate_logs.sh >> /tmp/mev_log_rotation.log 2>&1"

# Check if cron job already exists
if ! crontab -l 2>/dev/null | grep -q "rotate_logs.sh"; then
    echo "📅 Adding hourly log rotation to crontab..."
    (crontab -l 2>/dev/null; echo "$CRON_LINE") | crontab -
    echo "✅ Cron job added (runs every hour)"
else
    echo "✅ Log rotation cron job already exists"
fi

# Show crontab
echo ""
echo "Current crontab for MEV bot:"
crontab -l 2>/dev/null | grep -E "(rotate_logs|MEV)" || echo "No MEV-related cron jobs"

echo ""
echo "=== Starting MEV Bot ==="
cd "$MEV_BOT_DIR" || exit 1

# Start bot with multi-DEX config
bash -c "set -a && source .env.multidex && set +a && ./target/release/elite_mev_bot_v2_1_production > /tmp/mev_multidex.log 2>&1 &"

# Wait for startup
sleep 3

# Verify bot is running
if pgrep -f elite_mev_bot_v2_1_production > /dev/null; then
    echo "✅ MEV bot started successfully (PID: $(pgrep -f elite_mev_bot_v2_1_production))"
    echo ""
    echo "📊 Log file: /tmp/mev_multidex.log"
    echo "📅 Log rotation: Automatic (every hour, 100MB max, keeps 5 files)"
    echo ""
    echo "View logs: tail -f /tmp/mev_multidex.log"
    echo "Stop bot: killall elite_mev_bot_v2_1_production"
    echo "Manual rotation: $SCRIPT_DIR/rotate_logs.sh"
else
    echo "❌ Failed to start MEV bot"
    exit 1
fi
