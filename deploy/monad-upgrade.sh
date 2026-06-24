#!/usr/bin/env bash
# Operator-installed privileged upgrade wrapper for the Monad node.
# Invoked by Sentinel Console as: sudo -n /usr/local/bin/monad-upgrade.sh <version>
# Install: sudo install -o root -g root -m 750 deploy/monad-upgrade.sh /usr/local/bin/monad-upgrade.sh
set -euo pipefail

ver="${1:-}"
if [[ ! "$ver" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "invalid version: '$ver'" >&2
  exit 2
fi
ver="${ver#v}"   # apt wants 0.14.5, not v0.14.5

apt-get update
apt-get install --reinstall -y --allow-downgrades --allow-change-held-packages "monad=${ver}"
apt-mark hold monad
systemctl restart monad-bft monad-execution monad-rpc
monad-rpc -V
