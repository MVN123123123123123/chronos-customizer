#!/system/bin/sh
MODDIR=${0%/*}

# Log start
exec 2>${MODDIR}/service.log
set -x

# 1. Wait for Android boot completion
while [ "$(getprop sys.boot_completed)" != "1" ]; do
    sleep 2
done

# 2. Whitelist package from Doze Mode & Standby limits
PKG="dev.chronos.customizer"
dumpsys deviceidle whitelist +$PKG 2>/dev/null || true
cmd appops set $PKG RUN_IN_BACKGROUND allow 2>/dev/null || true
cmd appops set $PKG RUN_ANY_IN_BACKGROUND allow 2>/dev/null || true
cmd appops set $PKG AUTO_REVOKE_PERMISSIONS_IF_UNUSED ignore 2>/dev/null || true
am set-standby-bucket $PKG active 2>/dev/null || true

# 3. Ensure customd binary has execution permissions
chmod 0755 "$MODDIR/bin/customd" 2>/dev/null || true

# 4. Launch native Rust daemon unconstrained by zygote cgroups (subshell backgrounding without nohup dependency)
if [ -f "$MODDIR/bin/customd" ]; then
    ( "$MODDIR/bin/customd" > /dev/null 2>&1 & )
fi
