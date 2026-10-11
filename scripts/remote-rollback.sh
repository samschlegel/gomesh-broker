#!/usr/bin/env bash
# Runs on the deploy host as root (via `sudo bash -s -- [VERSION]`).
# Activates a previously-deployed release and restarts the service.
#
#   no arg   -> roll back to the most recent release that is NOT current
#   VERSION  -> roll back to that specific release (a releases/ subdir name)
set -euo pipefail

BASE=/opt/gomesh
REL=$BASE/releases
OWNER=gomesh
GROUP=users
TARGET="${1:-}"

mapfile -t VERSIONS < <(
  find "$REL" -mindepth 1 -maxdepth 1 -type d -name '20*' -printf '%f\n' | sort
)
[ "${#VERSIONS[@]}" -gt 0 ] || { echo "error: no releases found in $REL" >&2; exit 1; }

CURRENT=$(readlink "$REL/current" 2>/dev/null || true)

if [ -z "$TARGET" ]; then
  # Pick the newest release that isn't the current one.
  for v in "${VERSIONS[@]}"; do
    [ "$v" != "$CURRENT" ] && TARGET="$v"
  done
  [ -n "$TARGET" ] || { echo "error: no earlier release to roll back to (only $CURRENT exists)" >&2; exit 1; }
fi

[ -d "$REL/$TARGET" ] || {
  echo "error: release '$TARGET' not found. Available:" >&2
  printf '  %s\n' "${VERSIONS[@]}" >&2
  exit 1
}

echo "Rolling back: $CURRENT -> $TARGET"

install -o "$OWNER" -g "$GROUP" -m 0755 "$REL/$TARGET/gomesh-broker" "$BASE/bin/gomesh-broker.next"
mv -f "$BASE/bin/gomesh-broker.next" "$BASE/bin/gomesh-broker"
install -o "$OWNER" -g "$GROUP" -m 0644 "$REL/$TARGET/config.toml" "$BASE/etc/gomesh-broker/config.toml.next"
mv -f "$BASE/etc/gomesh-broker/config.toml.next" "$BASE/etc/gomesh-broker/config.toml"

ln -sfn "$TARGET" "$REL/current"
chown -h "$OWNER":"$GROUP" "$REL/current"

systemctl restart gomesh-broker
sleep 1
systemctl is-active --quiet gomesh-broker \
  && echo "Rolled back to $TARGET and restarted gomesh-broker (active)." \
  || { echo "error: gomesh-broker failed to start after rollback" >&2; systemctl --no-pager -l status gomesh-broker >&2; exit 1; }
