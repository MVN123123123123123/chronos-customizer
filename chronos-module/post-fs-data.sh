#!/system/bin/sh
MODDIR=${0%/*}

# Early boot systemless configuration
# Strictly in-memory (tmpfs) - NEVER modify physical block partitions (/system, /boot)
# Safe for locked bootloaders and temporary root setups

# Ensure runtime directories exist with safe permissions
mkdir -p /data/local/tmp/chronos
chmod 0777 /data/local/tmp/chronos

# Verify Zygisk library permissions
if [ -f "$MODDIR/zygisk/arm64-v8a.so" ]; then
    chmod 0644 "$MODDIR/zygisk/arm64-v8a.so"
fi
