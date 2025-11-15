# Log Bloat Issue - Fixed (2025-11-08)

## 🚨 Issue Summary

**Problem**: MEV bot generated 270GB of logs in a single run, filling disk to 100%.

**Root Cause**: Excessive logging frequency (every 100 cycles) at high processing speed.

**Impact**:
- Disk filled to 100% (386GB of 387GB)
- Required immediate intervention
- No log rotation configured

---

## ✅ Fixes Applied

### 1. **Reduced Logging Frequency**
- **Before**: Log every 100 cycles (`info!` level)
- **After**: Log every 50,000 cycles (`debug!` level)
- **Reduction**: 500x less frequent
- **File**: `src/bin/elite_mev_bot_v2_1_production.rs:1975`

### 2. **Automatic Log Rotation**
Created two scripts in `scripts/`:

#### `rotate_logs.sh`
- Rotates logs when they exceed 100MB
- Keeps last 5 rotations
- Auto-deletes old backups
- Run manually or via cron

#### `start_with_rotation.sh`
- Starts bot with automatic log rotation (every 5 minutes)
- Graceful shutdown handling
- Cleans up rotation process on exit

---

## 📊 Before vs After

| Metric | Before | After |
|--------|--------|-------|
| Log frequency | Every 100 cycles | Every 50,000 cycles |
| Log level | `info!` | `debug!` |
| Log rotation | None | Every 5 min |
| Max log size | Unlimited | 100MB before rotation |
| Rotations kept | N/A | Last 5 |
| Disk usage | 270GB in hours | ~100MB max |

---

## 🚀 Usage

### Start with automatic rotation:
```bash
cd /home/tom14cat14/MEV_Bot
./scripts/start_with_rotation.sh
```

### Manual log rotation:
```bash
./scripts/rotate_logs.sh
```

### Standard start (no rotation):
```bash
./target/release/elite_mev_bot_v2_1_production > /tmp/mev_multidex.log 2>&1 &
```

---

## ⚠️ Monitoring

Watch log sizes:
```bash
du -sh /tmp/mev*.log
```

Check rotation status:
```bash
ls -lh /tmp/mev*.log*
```

View live logs (reduced verbosity):
```bash
tail -f /tmp/mev_multidex.log
```

---

## 🔒 Prevention

1. **Always use startup script** - Built-in rotation prevents bloat
2. **Monitor disk space** - Set up alerts at 80% usage
3. **Debug mode only when needed** - Use `RUST_LOG=info` in production
4. **Regular cleanup** - Old rotations auto-deleted after 5 versions

---

**Status**: ✅ Fixed and deployed (2025-11-08)
**Disk Recovery**: 270GB freed
**Production Ready**: Yes (with rotation enabled)
