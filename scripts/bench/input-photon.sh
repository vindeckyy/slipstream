#!/usr/bin/env bash
# input-photon.sh — input-to-photon latency test (latency plan §8): a physical button or LED
# marker measures the full input loop, while the host's own injection-delay log measures the
# software leg (host receive → uinput/libei injection). The two are never conflated: video
# latency is a separate statistic.
#
#   scripts/bench/input-photon.sh [--led] [seconds]   # software-leg measurement (default)
#   scripts/bench/input-photon.sh --check             # verify prerequisites
#
# SOFTWARE LEG (always measured):
#   - streams the slipstream/1 virtual source for `seconds` (default 20)
#   - reads the host log's "input injection delay (host receive→inject)" percentile line,
#     which the host emits every 5 s while input flows (see ss-inject::input::service)
#   - reports inject_p50/p95/max µs — the host-side leg of input-to-photon
#
# PHYSICAL MARKER (--led): a button wired to GPIO (or a LED you watch) that the OPERATOR presses
#   the moment a test pattern flashes on the client display; the script times the loop
#   operator-input → client-photon and reports the FULL input-to-photon latency. This is a
#   human-in-the-loop measurement (stopwatch-grade); it exists to prove the loop end-to-end,
#   not to replace the software-leg percentiles.
#
# The software leg needs the host's injection-delay telemetry: it runs only when input actually
# flows (a gamepad/pointer client connected). The probe client streams video; the injection
# delay line appears once the host injects a real event.
set -uo pipefail

MODE="${1:-1280x720x60}"
if [[ "$1" == "--check" || "$1" == "-c" ]]; then
  echo "==> checking prerequisites"
  command -v target/release/slipstream-host >/dev/null || { echo "!! build the host first: cargo build -rq -p slipstream-host"; exit 1; }
  command -v target/release/slipstream-probe >/dev/null || { echo "!! build the probe first: cargo build -rq -p slipstream-probe"; exit 1; }
  echo "ok"
  exit 0
fi
LED=0
[[ "${1:-}" == "--led" ]] && { LED=1; MODE="${2:-1280x720x60}"; }
SECS="${2:-20}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-kde}"
export XDG_CURRENT_DESKTOP="${XDG_CURRENT_DESKTOP:-KDE}"
export SLIPSTREAM_COMPOSITOR="${SLIPSTREAM_COMPOSITOR:-kwin}"
export SLIPSTREAM_VIDEO_SOURCE=virtual SLIPSTREAM_PERF=1

echo "==> building host + probe (release)"
cargo build -rq -p slipstream-host -p slipstream-probe

HOST_LOG="$(mktemp)"; CLI_LOG="$(mktemp)"
trap 'kill "$HOST_PID" 2>/dev/null; rm -f "$HOST_LOG" "$CLI_LOG"' EXIT

echo "==> host: slipstream1-host --source virtual ($MODE, ${SECS}s)"
target/release/slipstream-host slipstream1-host --source virtual --seconds "$SECS" --max-sessions 1 \
  >"$HOST_LOG" 2>&1 &
HOST_PID=$!
sleep 3
echo "==> client: streaming"
target/release/slipstream-probe --connect 127.0.0.1:9777 --mode "$MODE" --out /dev/null \
  >"$CLI_LOG" 2>&1 || true

# Physical marker: the operator presses a button / flashes a LED the moment the client's test
# pattern appears, and the script times the loop (best-effort; see the header).
if [[ "$LED" == "1" ]]; then
  echo "==> press the marker button the instant the client display lights up"
  T0=$(date +%s%N)
  read -r -p "press ENTER when the client shows the pattern" _
  T1=$(date +%s%N)
  echo "operator-loop (input→photon) = $(( (T1 - T0) / 1000000 )) ms (human-in-the-loop, not a substitute for the software leg)"
fi

wait "$HOST_PID" 2>/dev/null || true

# --- extract the host injection-delay metric (software leg) ------------------
INJ=$(grep -oE 'inject_p50_us=[0-9]+|inject_p95_us=[0-9]+|inject_max_us=[0-9]+' "$HOST_LOG" | tail -3)
if [[ -z "$INJ" ]]; then
  echo "!! no injection-delay line in the host log — input did not flow (no pointer/gamepad client?)"
  echo "   host log tail:"; tail -6 "$HOST_LOG"
  exit 1
fi
echo "==> host injection delay (receive→inject, software leg):"
echo "$INJ" | sed 's/^/   /'
echo "==> full input-to-photon requires the --led marker on a real client; the software leg above"
echo "    is the host's share. Video latency is a separate statistic (see gpu-stream.sh)."
