#!/usr/bin/env bash
# Runs on the deploy host as root (via `sudo bash -s`).
# Installs HAProxy if needed, refreshes its config and certificates, validates
# the result, and reloads. A config or certificate that fails validation is
# never activated.
#
# Staged inputs (uploaded by the `install-haproxy` recipe before this runs):
#   /tmp/haproxy.cfg                       - HAProxy config
#   /tmp/cloudflare-origin-pull-ca.pem     - Cloudflare Authenticated Origin Pulls CA
set -euo pipefail

CERTS=/etc/haproxy/certs
SRC_CFG=/tmp/haproxy.cfg
SRC_CA=/tmp/cloudflare-origin-pull-ca.pem

[ -f "$SRC_CFG" ] || { echo "error: missing staged config $SRC_CFG" >&2; exit 1; }
[ -f "$SRC_CA" ] || { echo "error: missing staged CA $SRC_CA" >&2; exit 1; }

if ! command -v haproxy >/dev/null; then
  DEBIAN_FRONTEND=noninteractive apt-get update -q
  DEBIAN_FRONTEND=noninteractive apt-get install -y -q haproxy
fi

# Stage everything as .next and validate before touching the live files, so a
# bad cert or config can't break the next restart.
# Server certificate: the broker's existing origin cert + key, combined into
# the single PEM HAProxy expects.
install -d -o root -g haproxy -m 0750 "$CERTS"
umask 077
cat /opt/gomesh/cert.pem /opt/gomesh/key.pem > "$CERTS/gomesh.pem.next"
chown root:haproxy "$CERTS/gomesh.pem.next"
chmod 0640 "$CERTS/gomesh.pem.next"
install -o root -g haproxy -m 0644 "$SRC_CA" "$CERTS/cloudflare-origin-pull-ca.pem.next"
install -o root -g root -m 0644 "$SRC_CFG" /etc/haproxy/haproxy.cfg.next

# Validate a copy of the new config that points at the staged certs.
sed -e "s#$CERTS/gomesh.pem#$CERTS/gomesh.pem.next#g" \
    -e "s#$CERTS/cloudflare-origin-pull-ca.pem#$CERTS/cloudflare-origin-pull-ca.pem.next#g" \
    /etc/haproxy/haproxy.cfg.next > /etc/haproxy/haproxy.cfg.check
if ! haproxy -c -q -f /etc/haproxy/haproxy.cfg.check; then
  rm -f /etc/haproxy/haproxy.cfg.check /etc/haproxy/haproxy.cfg.next \
    "$CERTS/gomesh.pem.next" "$CERTS/cloudflare-origin-pull-ca.pem.next"
  echo "error: haproxy config or certificates failed validation; leaving the current ones in place" >&2
  exit 1
fi
rm -f /etc/haproxy/haproxy.cfg.check
mv -f "$CERTS/gomesh.pem.next" "$CERTS/gomesh.pem"
mv -f "$CERTS/cloudflare-origin-pull-ca.pem.next" "$CERTS/cloudflare-origin-pull-ca.pem"
mv -f /etc/haproxy/haproxy.cfg.next /etc/haproxy/haproxy.cfg
rm -f "$SRC_CFG" "$SRC_CA"

systemctl enable --quiet haproxy
if systemctl is-active --quiet haproxy; then
  systemctl reload haproxy
else
  systemctl start haproxy
fi
sleep 1
systemctl is-active --quiet haproxy \
  && echo "HAProxy config installed and active." \
  || { echo "error: haproxy is not running" >&2; systemctl --no-pager -l status haproxy >&2; exit 1; }
