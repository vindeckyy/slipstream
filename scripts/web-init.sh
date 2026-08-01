#!/bin/sh
# First-run setup for the slipstream web console (run by slipstream-web-init.service as the user).
# The browser creates the login password on the first visit. This service only prepares the private
# config directory; the mgmt token is owned by the host.
set -eu

DIR="${XDG_CONFIG_HOME:-$HOME/.config}/slipstream"
mkdir -p "$DIR"
chmod 700 "$DIR" 2>/dev/null || true
echo "slipstream web console ready; open https://<host-ip>:47992 to choose a login password"
