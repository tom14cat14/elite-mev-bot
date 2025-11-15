#!/bin/bash
# MEV Bot Log Rotation Script
# Rotates /tmp/mev_multidex.log when it gets large
# Keeps last 5 rotated logs, compresses old ones

# Configuration
LOG_FILE="/tmp/mev_multidex.log"
MAX_SIZE_MB=100  # Rotate when log exceeds this size
KEEP_ROTATIONS=5  # Keep this many old logs
COMPRESS=true     # Compress old logs (saves 90%+ space)

# Get log file size in MB
if [ ! -f "$LOG_FILE" ]; then
    echo "Log file not found: $LOG_FILE"
    exit 0
fi

FILE_SIZE_MB=$(du -m "$LOG_FILE" | cut -f1)
echo "Current log size: ${FILE_SIZE_MB}MB (max: ${MAX_SIZE_MB}MB)"

# Check if rotation needed
if [ "$FILE_SIZE_MB" -lt "$MAX_SIZE_MB" ]; then
    echo "No rotation needed."
    exit 0
fi

echo "Log size exceeded ${MAX_SIZE_MB}MB - rotating..."

# Create timestamped backup
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${LOG_FILE}.${TIMESTAMP}"

# Copy current log to backup (don't move - bot is still writing)
cp "$LOG_FILE" "$BACKUP_FILE"
echo "Created backup: $BACKUP_FILE"

# Truncate original log (clears it while bot keeps file handle)
> "$LOG_FILE"
echo "Truncated original log file"

# Compress backup if enabled
if [ "$COMPRESS" = true ]; then
    gzip "$BACKUP_FILE"
    echo "Compressed backup: ${BACKUP_FILE}.gz"
    BACKUP_FILE="${BACKUP_FILE}.gz"
fi

# Delete old rotations (keep only KEEP_ROTATIONS most recent)
OLD_LOGS=$(ls -t ${LOG_FILE}.* 2>/dev/null | tail -n +$((KEEP_ROTATIONS + 1)))
if [ -n "$OLD_LOGS" ]; then
    echo "Deleting old logs:"
    echo "$OLD_LOGS"
    echo "$OLD_LOGS" | xargs rm -f
fi

# Show remaining logs
echo ""
echo "Rotated logs (keeping ${KEEP_ROTATIONS}):"
ls -lh ${LOG_FILE}.* 2>/dev/null || echo "No rotated logs yet"

echo ""
echo "✅ Log rotation complete!"
