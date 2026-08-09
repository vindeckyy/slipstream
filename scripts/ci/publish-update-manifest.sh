#!/usr/bin/env bash
# Build, sign, upload, and self-verify the host update manifest for one channel.
#
# Upload target (operator-provided package registry; no public default):
#   https://<REGISTRY>/api/packages/<OWNER>/generic/slipstream-update/<channel>/manifest.json
#   https://<REGISTRY>/api/packages/<OWNER>/generic/slipstream-update/<channel>/manifest.json.sig
#
# The signature is a raw 64-byte Ed25519 over the EXACT manifest bytes, base64 in the .sig —
# the same format the plugin index uses and `store::index::verify_signature` checks. The
# public half is pinned in the shared checker (OFFICIAL_UPDATE_KEYS in crates/ss-update-check);
# before signing, this script cross-checks the signing key against that constant and refuses
# on mismatch — the most likely deploy mistake is signing with a key no host trusts (the
# sysext publisher's fingerprint-crosscheck drill).
#
# Upload order is manifest THEN signature: a client caught in the replace window sees a
# mismatched pair and refuses (fail-closed), never a stale-signed document.
#
# Environment:
#   CHANNEL              stable | canary                                   (required)
#   VERSION              the announced host version string                 (required)
#   CI_RUN               CI run number                                     (required for canary)
#   NOTES_URL            release-notes link (optional; prefer GitHub blob URLs)
#   UPDATE_MANIFEST_KEY  PKCS#8 PEM, the Ed25519 private key               (required to sign)
#   REQUIRE_KEY=1        missing key is a hard failure (announce/stable)   (optional)
#   REGISTRY_TOKEN       package-registry PAT with write:package           (required)
#   REGISTRY / OWNER     REGISTRY required (no public default); OWNER defaults to unom
# Docs: https://github.com/vindeckyy/slipstream
set -euo pipefail

CHANNEL="${CHANNEL:?set CHANNEL=stable|canary}"
VERSION="${VERSION:?set VERSION}"
REGISTRY="${REGISTRY:?set REGISTRY to your package-registry host}"
OWNER="${OWNER:-unom}"
BASE="https://${REGISTRY}/api/packages/${OWNER}/generic/slipstream-update/${CHANNEL}"

case "$CHANNEL" in stable|canary) ;; *) echo "CHANNEL must be stable or canary" >&2; exit 1 ;; esac
if [ "$CHANNEL" = canary ] && [ -z "${CI_RUN:-}" ]; then
  echo "canary manifests need CI_RUN (the definitive newer-than axis)" >&2; exit 1
fi
# ---- key handling (fail-closed where it matters) --------------------------------------------
if [ -z "${UPDATE_MANIFEST_KEY:-}" ]; then
  if [ "${REQUIRE_KEY:-0}" = 1 ] || case "${GITHUB_REF:-}" in refs/tags/v*) true ;; *) false ;; esac; then
    echo "ERROR: UPDATE_MANIFEST_KEY is not set — refusing to publish an unsigned manifest" >&2
    exit 1
  fi
  echo "WARN: UPDATE_MANIFEST_KEY not set — skipping the ${CHANNEL} update manifest" >&2
  exit 0
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
KEY="$WORK/key.pem"
# Tolerate secret stores that mangle newlines into literal \n (the plugin-index signer's fix).
if printf '%s' "$UPDATE_MANIFEST_KEY" | grep -q '\\n' && ! printf '%s' "$UPDATE_MANIFEST_KEY" | grep -q '^-----BEGIN.*-----$'; then
  printf '%s' "$UPDATE_MANIFEST_KEY" | sed 's/\\n/\n/g' > "$KEY"
else
  printf '%s\n' "$UPDATE_MANIFEST_KEY" > "$KEY"
fi

# Cross-check: the key we are about to sign with must be one the host binary pins.
PUB="ed25519:$(openssl pkey -in "$KEY" -pubout -outform DER | tail -c 32 | base64)"
# The pin list moved to the shared checker when the Linux client started verifying the same
# manifest (crates/ss-update-check/src/lib.rs, OFFICIAL_UPDATE_KEYS) — one list, so the host
# and the client can never disagree about who may announce a release.
KEYS_FILE="crates/ss-update-check/src/lib.rs"
# A MISSING file is fatal, not a warning. This check is the guard against signing with a key
# no build trusts; if its path ever goes stale the old `else` branch would have skipped it
# silently and published an unverifiable manifest — the exact failure it exists to prevent.
if [ ! -f "$KEYS_FILE" ]; then
  echo "ERROR: $KEYS_FILE not in this checkout — refusing to sign without the pinned-key cross-check" >&2
  exit 1
fi
if ! grep -qF "\"$PUB\"" "$KEYS_FILE"; then
  echo "ERROR: signing key $PUB is not pinned in $KEYS_FILE (OFFICIAL_UPDATE_KEYS) — wrong key?" >&2
  exit 1
fi

# ---- build the manifest ---------------------------------------------------------------------
SERIAL="$(date +%s)"
PUBLISHED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
MANIFEST="$WORK/manifest.json"

# Keep optional metadata in jq arguments so omitted values do not add manifest fields.
jq -n \
  --arg channel "$CHANNEL" \
  --arg version "$VERSION" \
  --arg published_at "$PUBLISHED_AT" \
  --arg notes_url "${NOTES_URL:-}" \
  --argjson serial "$SERIAL" \
  --arg ci_run "${CI_RUN:-}" \
  '
  {schema: 1, channel: $channel, serial: $serial, published_at: $published_at, version: $version}
  + (if $notes_url != "" then {notes_url: $notes_url} else {} end)
  + (if $ci_run != "" then {ci_run: ($ci_run | tonumber)} else {} end)
  ' > "$MANIFEST"
echo "manifest:"; cat "$MANIFEST"

# ---- sign + local verify --------------------------------------------------------------------
SIG_BIN="$WORK/sig.bin"
openssl pkeyutl -sign -inkey "$KEY" -rawin -in "$MANIFEST" -out "$SIG_BIN"
[ "$(wc -c < "$SIG_BIN")" -eq 64 ] || { echo "ERROR: signature is not 64 bytes" >&2; exit 1; }
SIG="$WORK/manifest.json.sig"
base64 < "$SIG_BIN" | tr -d '\n' > "$SIG"; printf '\n' >> "$SIG"

PUBPEM="$WORK/pub.pem"
openssl pkey -in "$KEY" -pubout -out "$PUBPEM"
openssl pkeyutl -verify -pubin -inkey "$PUBPEM" -rawin -in "$MANIFEST" -sigfile "$SIG_BIN" >/dev/null

# ---- upload (manifest first, then signature) ------------------------------------------------
: "${REGISTRY_TOKEN:?set REGISTRY_TOKEN}"
# Credentials ride in a netrc file, not curl argv, so the token never shows up in /proc/<pid>/cmdline.
NETRC="$WORK/.netrc"
printf 'machine %s login %s password %s\n' "$REGISTRY" "enricobuehler" "$REGISTRY_TOKEN" > "$NETRC"
chmod 600 "$NETRC"
upload() { # file url
  curl -fsS -o /dev/null --netrc-file "$NETRC" -X DELETE "$2" 2>/dev/null || true
  curl -fsS -o /dev/null --netrc-file "$NETRC" --upload-file "$1" "$2"
  echo "published: $2"
}
upload "$MANIFEST" "$BASE/manifest.json"
upload "$SIG"      "$BASE/manifest.json.sig"

# ---- self-verify the LIVE feed (bytes must round-trip; -L follows the 303) ------------------
curl -fsSL "$BASE/manifest.json" -o "$WORK/live.json"
curl -fsSL "$BASE/manifest.json.sig" -o "$WORK/live.sig"
cmp -s "$MANIFEST" "$WORK/live.json" || { echo "ERROR: live manifest differs from what was uploaded" >&2; exit 1; }
cmp -s "$SIG" "$WORK/live.sig" || { echo "ERROR: live signature differs from what was uploaded" >&2; exit 1; }
openssl pkeyutl -verify -pubin -inkey "$PUBPEM" -rawin -in "$WORK/live.json" -sigfile "$SIG_BIN" >/dev/null
echo "OK: ${CHANNEL} update manifest ${VERSION} (serial ${SERIAL}) is live and verifies"
