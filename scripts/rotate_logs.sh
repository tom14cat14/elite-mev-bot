#!/bin/bash
# Log Rotation Script for MEV Bot
# Rotates logs when they exceed 100MB, keeps last 5 rotations
# Also deletes rotations older than 1 minute (opportunities are gone)

LOG_DIR="/tmp"
LOG_PREFIX="mev"
MAX_SIZE_MB=100
MAX_ROTATIONS=5
MAX_AGE_MINUTES=1

rotate_log() {
    local log_file=$1
    local base_name=$(basename "$log_file")

    # Check if file exists and size
    if [ ! -f "$log_file" ]; then
        return
    fi

    local size_mb=$(du -m "$log_file" | cut -f1)

    if [ "$size_mb" -gt "$MAX_SIZE_MB" ]; then
        echo "Rotating $log_file (size: ${size_mb}MB)"

        # Rotate existing backups
        for i in $(seq $((MAX_ROTATIONS-1)) -1 1); do
            if [ -f "${log_file}.${i}" ]; then
                mv "${log_file}.${i}" "${log_file}.$((i+1))"
            fi
        done

        # Move current log to .1
        mv "$log_file" "${log_file}.1"
        touch "$log_file"

        # Delete old rotations beyond max count
        for i in $(seq $((MAX_ROTATIONS+1)) 10); do
            rm -f "${log_file}.${i}"
        done

        echo "Rotation complete. Kept last $MAX_ROTATIONS backups."
    fi

    # Delete rotations older than MAX_AGE_MINUTES (opportunities expired)
    find "${log_file}".* -type f -mmin +${MAX_AGE_MINUTES} -delete 2>/dev/null
}

# Rotate all MEV bot logs
for log in "${LOG_DIR}/${LOG_PREFIX}_"*.log; do
    if [ -f "$log" ]; then
        rotate_log "$log"
    fi
done
