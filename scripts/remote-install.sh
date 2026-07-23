#!/usr/bin/env bash
# Runs on the deploy host as root (via `sudo bash -s`).
# Installs the staged binary + config as a new timestamped release, then
# activates it. The previous releases are kept for rollback (see remote-rollback.sh).
#
# Staged inputs (uploaded by the `deploy` recipe before this runs):
#   /tmp/gomesh-broker.next          - new binary
#   /tmp/gomesh-broker-config.toml   - new config
set -euo pipefail

BASE=/opt/gomesh
REL=$BASE/releases
KEEP=10                       # number of releases to retain
OWNER=gomesh
GROUP=users

SRC_BIN=/tmp/gomesh-broker.next
SRC_CFG=/tmp/gomesh-broker-config.toml

[ -f "$SRC_BIN" ] || { echo "error: missing staged binary $SRC_BIN" >&2; exit 1; }
[ -f "$SRC_CFG" ] || { echo "error: missing staged config $SRC_CFG" >&2; exit 1; }

TS=$(date -u +%Y%m%d-%H%M%S)
DEST="$REL/$TS"

# 1. Record the new release
install -d -o "$OWNER" -g "$GROUP" "$DEST"
install -o "$OWNER" -g "$GROUP" -m 0755 "$SRC_BIN" "$DEST/gomesh-broker"
install -o "$OWNER" -g "$GROUP" -m 0644 "$SRC_CFG" "$DEST/config.toml"

# 2. Activate atomically (write-then-rename so a running exec never sees a partial file)
install -o "$OWNER" -g "$GROUP" -m 0755 "$DEST/gomesh-broker" "$BASE/bin/gomesh-broker.next"
mv -f "$BASE/bin/gomesh-broker.next" "$BASE/bin/gomesh-broker"
install -o "$OWNER" -g "$GROUP" -m 0644 "$DEST/config.toml" "$BASE/etc/gomesh-broker/config.toml.next"
mv -f "$BASE/etc/gomesh-broker/config.toml.next" "$BASE/etc/gomesh-broker/config.toml"

# 3. Mark this release current
ln -sfn "$TS" "$REL/current"
chown -h "$OWNER":"$GROUP" "$REL/current"

# 4. Prune old releases (keep newest $KEEP), never touching `current`
find "$REL" -mindepth 1 -maxdepth 1 -type d -name '20*' -printf '%f\n' \
  | sort | head -n "-$KEEP" | while read -r old; do
    [ "$old" = "$(readlink "$REL/current")" ] && continue
    rm -rf "${REL:?}/$old"
  done

rm -f "$SRC_BIN" "$SRC_CFG"

# 5. Restart the service onto the new binary
systemctl restart gomesh-broker
sleep 1
systemctl is-active --quiet gomesh-broker \
  && echo "Deployed release $TS and restarted gomesh-broker (active)." \
  || { echo "error: gomesh-broker failed to start after deploy" >&2; systemctl --no-pager -l status gomesh-broker >&2; exit 1; }
