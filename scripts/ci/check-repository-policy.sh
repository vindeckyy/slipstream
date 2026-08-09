#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

failures=0
fail() {
  printf 'repository policy: %s\n' "$1" >&2
  failures=$((failures + 1))
}

tracked_text() {
  git grep -I -n -i -E "$1" -- \
    ':!docs/releases/**' \
    ':!THIRD-PARTY-NOTICES.txt' \
    ':!scripts/ci/check-repository-policy.sh' 2>/dev/null || true
}

stale_references="$(tracked_text 'punktfunk|testflight|ko-?fi|reddit\.com')"
if [[ -n "$stale_references" ]]; then
  printf '%s\n' "$stale_references" >&2
  fail "stale product or third-party references remain in tracked text"
fi

if (git grep -I -n -E 'gh[pousr]_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16}' -- 2>/dev/null || true) | grep -q .; then
  fail "credential-shaped material is present in tracked files"
fi

if ! python3 - <<'PY'
import pathlib
import re
import subprocess
import sys

paths = subprocess.check_output(["git", "ls-files", "-z"]).split(b"\0")
pem = re.compile(
    rb"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----\n"
    rb"[A-Za-z0-9+/=\n]{80,}\n"
    rb"-----END (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"
)
found = []
for raw_path in paths:
    if not raw_path:
        continue
    path = pathlib.Path(raw_path.decode())
    try:
        data = path.read_bytes()
    except OSError:
        continue
    if pem.search(data):
        found.append(str(path))
if found:
    print("\n".join(found), file=sys.stderr)
    raise SystemExit(1)
PY
then
  fail "a PEM private key is present in tracked files"
fi

if git diff --check; then
  :
else
  fail "whitespace errors are present"
fi

while IFS= read -r action_line; do
  action_ref="${action_line##*@}"
  action_ref="${action_ref%%[[:space:]]*}"
  if [[ ! "$action_ref" =~ ^[0-9a-fA-F]{40}$ ]]; then
    fail "a workflow action is referenced by a mutable version tag"
    break
  fi
done < <(
  find .github/workflows -type f \( -name '*.yml' -o -name '*.yaml' \) -print0 |
    xargs -0 grep -hE '^\s*-?\s*uses: [^ ]+@[^ ]+' 2>/dev/null || true
)

if command -v appstreamcli >/dev/null 2>&1; then
  if ! appstreamcli validate packaging/flatpak/io.slipstream.metainfo.xml >/dev/null; then
    fail "Flatpak AppStream metadata does not validate"
  fi
fi

if [ "$failures" -ne 0 ]; then
  exit 1
fi

echo "repository policy checks passed"
