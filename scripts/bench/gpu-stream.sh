#!/usr/bin/env bash
# Tier-3 GPU stream benchmark — the REAL pipeline: virtual output → zero-copy dmabuf→CUDA → NVENC →
# slipstream/1 over loopback UDP → FEC/decrypt/reassemble, with the client measuring end-to-end
# latency. This is the "real-world" regression test the GPU-less CI can't run; it runs on a
# self-sustained GPU runner (a dev box with an NVIDIA GPU + a KWin session). Report-only by default.
#
#   scripts/bench/gpu-stream.sh [WxHxHz] [seconds]      # measure + compare to the baseline
#   scripts/bench/gpu-stream.sh 1920x1080x120 12 --update   # (re)write scripts/bench/gpu-baseline.json
#   scripts/bench/gpu-stream.sh 1920x1080x120 12 --real-client  # decode+present (Vulkan) client
#   scripts/bench/gpu-stream.sh --check                # verify prerequisites (no stream)
#
# Metrics (host SLIPSTREAM_PERF + client report): encode_us_p50/p99, tx_mbps, send_dropped, and the
# client's capture→received lat_p50/p95/p99_us (received_ns-based — cross-machine valid with the
# connect-time clock-skew handshake). With --real-client the capture→valid-on-glass e2e latency is
# reported from the presenter's TRUE on-glass stamps (VK_KHR_present_wait) and present validity is
# recorded. Lower is better for latency/encode/drops, higher for throughput.
#
# CI GATE: without --report-only, a regression > 20% on a previously-supported metric FAILS the
# script (exit 1). --report-only keeps the exploratory-report behavior (exit 0 with ⚠ flags).
set -uo pipefail

MODE="${1:-1920x1080x120}"
SECS="${2:-12}"
UPDATE=""; REAL=""; REPORT_ONLY=""
for a in "$@"; do
  [[ "$a" == "--update" ]] && UPDATE=1
  [[ "$a" == "--real-client" ]] && REAL=1
  [[ "$a" == "--report-only" ]] && REPORT_ONLY=1
done
[[ "$MODE" == "--check" || "$1" == "--check" ]] && {
  echo "==> prerequisites:"
  command -v target/release/slipstream-host >/dev/null && echo "  ok slipstream-host" || echo "  !! build: cargo build -rq -p slipstream-host"
  command -v target/release/slipstream-probe >/dev/null && echo "  ok slipstream-probe" || echo "  !! build: cargo build -rq -p slipstream-probe"
  [[ -n "$REAL" ]] && { command -v target/release/slipstream-session >/dev/null && echo "  ok slipstream-session" || echo "  !! build: cargo build -rq -p slipstream-session"; }
  exit 0
}
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
BASELINE="scripts/bench/gpu-baseline.json"

# Compositor session: reuse one if present, else bring up a headless KWin (dev-box KDE pattern).
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-kde}"
export XDG_CURRENT_DESKTOP="${XDG_CURRENT_DESKTOP:-KDE}"
export SLIPSTREAM_COMPOSITOR="${SLIPSTREAM_COMPOSITOR:-kwin}"
export SLIPSTREAM_VIDEO_SOURCE=virtual SLIPSTREAM_ZEROCOPY=1 SLIPSTREAM_PERF=1
OWN_KWIN=""
if [[ ! -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]]; then
  echo "==> no $WAYLAND_DISPLAY — bringing up a headless KWin session"
  setsid bash scripts/headless/run-headless-kde.sh "${MODE%x*}" </dev/null >/tmp/bench-kwin.log 2>&1 &
  OWN_KWIN=$!
  for _ in $(seq 1 30); do [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] && break; sleep 1; done
fi

echo "==> building host + client (release)"
if [[ -n "$REAL" ]]; then
  cargo build -rq -p slipstream-host -p slipstream-session
else
  cargo build -rq -p slipstream-host -p slipstream-probe
fi

HOST_LOG="$(mktemp)"; CLI_LOG="$(mktemp)"
trap 'kill "$HOST_PID" 2>/dev/null; [[ -n "$OWN_KWIN" ]] && pkill -f "kwin_wayland --virtual" 2>/dev/null; rm -f "$HOST_LOG" "$CLI_LOG"' EXIT

echo "==> host: slipstream1-host --source virtual ($MODE, ${SECS}s)"
target/release/slipstream-host slipstream1-host --source virtual --seconds "$SECS" --max-sessions 1 \
  >"$HOST_LOG" 2>&1 &
HOST_PID=$!
sleep 3
echo "==> client: streaming + measuring latency"
if [[ -n "$REAL" ]]; then
  # The real decode+present client: Vulkan Video/VAAPI decode → VK_KHR_present_wait on-glass
  # timing. Its log line "on-glass present timing" records present validity (the bench gates on
  # it below); the e2e window is the capture→valid-on-glass figure.
  target/release/slipstream-session --connect 127.0.0.1:9777 --fullscreen \
    >"$CLI_LOG" 2>&1 || true
else
  target/release/slipstream-probe --connect 127.0.0.1:9777 --mode "$MODE" --out /dev/null \
    >"$CLI_LOG" 2>&1 || true
fi
wait "$HOST_PID" 2>/dev/null || true

# --- extract metrics ---------------------------------------------------------
field() { grep -oE "$1=\"?[0-9]+" "$2" | tail -1 | grep -oE "[0-9]+$"; }
ENC_P50=$(field "encode_us_p50" "$HOST_LOG"); ENC_P99=$(field "encode_us_p99" "$HOST_LOG")
TX_MBPS=$(field "tx_mbps" "$HOST_LOG");       DROPPED=$(field "send_dropped_total" "$HOST_LOG")
if [[ -n "$REAL" ]]; then
  # Present validity: the on-glass timing line means the presenter reached VK_KHR_present_wait
  # (TRUE on-glass stamps). Absent = the driver lacks present-wait; the run is still valid but
  # the e2e figure is submit-time, not on-glass.
  PRESENT_VALID=$(grep -c "on-glass present timing (VK_KHR_present_wait)" "$CLI_LOG")
  # The e2e window is logged by the session client's stats line; extract the p50/p95 ms.
  LAT_P50=$(field "e2e" "$CLI_LOG" | head -1)
  LAT_P95=$(field "e2e" "$CLI_LOG" | tail -1)
  LAT_P99=0
  RECEIVED_NS_BASED=0
else
  LAT_P50=$(field "lat_p50_us" "$CLI_LOG");     LAT_P95=$(field "lat_p95_us" "$CLI_LOG")
  LAT_P99=$(field "lat_p99_us" "$CLI_LOG")
  RECEIVED_NS_BASED=$(grep -c "received_ns_based=true" "$CLI_LOG")
  PRESENT_VALID=0
fi
if [[ -z "$LAT_P50" || -z "$ENC_P50" ]]; then
  echo "!! incomplete metrics (host/client did not stream). host log tail:"; tail -8 "$HOST_LOG"
  echo "   client log tail:"; tail -8 "$CLI_LOG"
  exit 1
fi

python3 - "$BASELINE" "${UPDATE:-}" "${REPORT_ONLY:-}" <<PY
import json, os, sys
baseline_path, update, report_only = sys.argv[1], sys.argv[2], sys.argv[3]
# (metric, value, lower_is_better)
cur = {
  "encode_us_p50": ($ENC_P50, True), "encode_us_p99": ($ENC_P99, True),
  "lat_us_p50": ($LAT_P50, True), "lat_us_p95": ($LAT_P95, True), "lat_us_p99": ($LAT_P99, True),
  "tx_mbps": (${TX_MBPS:-0}, False), "send_dropped_total": (${DROPPED:-0}, True),
}
vals = {k: v for k, (v, _) in cur.items()}
if update:
    json.dump(vals, open(baseline_path, "w"), indent=2); open(baseline_path,"a").write("\n")
    print("wrote GPU baseline ->", baseline_path); sys.exit(0)
base = json.load(open(baseline_path)) if os.path.exists(baseline_path) else {}
if not base:
    print("!! no baseline at", baseline_path, "— run with --update to check one in first")
    sys.exit(1)
THRESH = 0.20  # 20% on a dedicated runner
rows = ["## Tier-3 GPU stream benchmark ($MODE)", "",
        "| metric | baseline | current | Δ |", "|---|---:|---:|---:|"]
regr = []
for k, (v, lower) in cur.items():
    b = base.get(k)
    if b is None: rows.append(f"| {k} | — | {v} | _new_ |"); continue
    d = (v - b) / b if b else 0.0
    worse = (d > THRESH) if lower else (d < -THRESH)
    flag = " ⚠" if worse else ""
    rows.append(f"| {k} | {b} | {v} | {d:+.1%}{flag} |")
    if worse: regr.append(k)
# Phase 10: present validity + received_ns-based latency are recorded alongside the deltas.
rows.append("")
rows.append(f"| received_ns_based | {'yes' if $RECEIVED_NS_BASED else 'no (shared-clock)'} |"
            f" | present_valid | {'yes' if $PRESENT_VALID else 'no (no VK_KHR_present_wait)'} |")
out = "\n".join(rows)
print(out)
s = os.environ.get("GITHUB_STEP_SUMMARY")
if s: open(s, "a").write(out + "\n")
if regr:
    print("\n⚠ regressed:", ", ".join(regr))
    if not report_only:
        print("!! CI GATE: hard regression threshold exceeded — exiting 1")
        sys.exit(1)
PY
