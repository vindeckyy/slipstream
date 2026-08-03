#!/usr/bin/env bash
# netem-latency.sh — real `tc netem` network-latency sweep over the slipstream/1 path (plan §10).
#
# Shapes a chosen interface with ONE `tc netem` root qdisc, then drives the full pipeline
# (virtual source → encode → slipstream/1 over UDP → FEC → probe client) under that shaping and
# extracts the client's capture→received latency percentiles plus the host's encode/send stats.
#
#   scripts/bench/netem-latency.sh [scenario] [run|analyze]   # run: 30 s warmup + 120 s collect × 5
#   scripts/bench/netem-latency.sh --list                      # print the profile table
#   scripts/bench/netem-latency.sh --check                     # verify prerequisites (no shaping)
#   scripts/bench/netem-latency.sh analyze [scenario]          # aggregate last run's summary.jsonl
#
# PROFILES (one-sided netem — one qdisc, one direction; `delay` is ONE-WAY, rtt = 2 × delay):
#   clean-lan  no shaping (baseline)
#   wan-20     delay 20ms loss 0.1% rate 1000mbit
#   wan-50     delay 50ms loss 0.5% rate 500mbit
#   jitter-2   delay 10ms jitter 2ms loss 0.1%            (jitter = normal distribution)
#   jitter-10  delay 20ms jitter 10ms loss 0.1%
#   loss-01    loss 0.1% delay 2ms
#   loss-05    loss 0.5% delay 2ms
#   loss-10    loss 1%   delay 2ms
#   loss-20    loss 2%   delay 2ms
#   cap-12x    rate = 1.2 × NETEM_BITRATE_MBPS (default 50 → 60mbit), delay 2ms
#   cap-20x    rate = 2 × NETEM_BITRATE_MBPS (default 50 → 100mbit), delay 2ms
#
# ONE-SIDED CONTRACT: a single netem qdisc shapes EGRESS on one machine — the direction that
# leaves it. With NETEM_SIDE=host (default) the video path host→client is shaped; with
# NETEM_SIDE=client the ACK/feedback path client→host is shaped instead. Two-sided RTT shaping
# is out of scope by design.
#
# LOOPBACK CAVEAT: netem does NOT shape `lo` on most kernels, and traffic to 127.0.0.1 never
# crosses another interface anyway. The default client target is 127.0.0.1:9777 (mirrors
# gpu-stream.sh's loopback invocation, so the harness runs anywhere), but for REAL shaped
# results you must stream across a machine boundary:
#
#   host machine:   NETEM_IFACE=<lan-iface> NETEM_CLIENT_CONNECT=<host-lan-ip>:9777 \
#                   sudo scripts/bench/netem-latency.sh wan-20
#                   (probe on the client machine reaches the host's LAN IP — netem on the
#                    host's egress then shapes the video packets)
#   client machine: NETEM_SIDE=client NETEM_IFACE=<client-iface> \
#                   NETEM_CLIENT_CONNECT=<host-ip>:9777 sudo scripts/bench/netem-latency.sh wan-20
#                   (runs the probe only; start the host on the host machine with a long enough
#                    session, e.g. --seconds 800 for a full sweep of one scenario)
#
# SAFETY: refuses to start if the chosen interface already carries a non-default root qdisc
# (htb/tbf/cake/prio/… — a real config; never nuked). Safe defaults (noqueue, fq, fq_codel,
# pfifo, pfifo_fast, bfifo, mq, mqprio) are replaced for the run and RESTORED afterwards; a
# leftover netem qdisc is treated as ours and removed. Inspect any time with: tc qdisc show dev
# <iface>  (remove with: sudo tc qdisc del dev <iface> root).
#
# PROTOCOL per scenario: for each of NETEM_REPS (5) repetitions: 30 s warmup → 5 s quiet →
# 120 s collection → 5 s quiet. Each repetition appends one JSON line to
# scripts/bench/results/netem/<scenario>/summary.jsonl:
#   {scenario, repetition, rtt_ms (=2×one-way delay), jitter_ms, loss_pct, rate_mbps,
#    lat_p50_us, lat_p95_us, lat_p99_us, lat_max_us, samples (=client frames), drops
#    (=host send_dropped_total), encode_us_p50, tx_mbps}
# latency/encode/throughput come from the host SLIPSTREAM_PERF + probe report via the same
# `field()` grep extraction gpu-stream.sh uses; the per-rep host/client logs and the
# SLIPSTREAM_LATENCY_ARTIFACT / SLIPSTREAM_CLIENT_ARTIFACT files land under the same results dir.
#
# Env overrides: NETEM_IFACE (default: first non-loopback interface), NETEM_CLIENT_CONNECT
# (default 127.0.0.1:9777), NETEM_SIDE (host|client, default host), NETEM_BITRATE_MBPS (default
# 50), NETEM_REPS, NETEM_WARMUP_SECS, NETEM_RUN_SECS, NETEM_QUIET_SECS, NETEM_RESULTS_DIR.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

RESULTS_DIR="${NETEM_RESULTS_DIR:-scripts/bench/results/netem}"
REPS="${NETEM_REPS:-5}"
WARMUP_SECS="${NETEM_WARMUP_SECS:-30}"
RUN_SECS="${NETEM_RUN_SECS:-120}"
QUIET_SECS="${NETEM_QUIET_SECS:-5}"
BITRATE_MBPS="${NETEM_BITRATE_MBPS:-50}"
CONNECT="${NETEM_CLIENT_CONNECT:-127.0.0.1:9777}"
SIDE="${NETEM_SIDE:-host}"
MODE_RES="${NETEM_MODE:-1920x1080x60}"

# name|label|delay_ms|jitter_ms|loss_pct|rate_mbit("X<n>" = n/10 × BITRATE_MBPS)|reorder_pct|reorder_gap
PROFILE_TABLE="$(cat <<'EOF'
clean-lan|no shaping (baseline)|0|0|0|0|0|1
wan-20|20 ms one-way delay, 0.1% loss, 1000 mbit|20|0|0.1|1000|0|1
wan-50|50 ms one-way delay, 0.5% loss, 500 mbit|50|0|0.5|500|0|1
jitter-2|10 ms delay + 2 ms jitter (normal), 0.1% loss|10|2|0.1|0|0|1
jitter-10|20 ms delay + 10 ms jitter (normal), 0.1% loss|20|10|0.1|0|0|1
loss-01|0.1% loss, 2 ms delay|2|0|0.1|0|0|1
loss-05|0.5% loss, 2 ms delay|2|0|0.5|0|0|1
loss-10|1% loss, 2 ms delay|2|0|1|0|0|1
loss-20|2% loss, 2 ms delay|2|0|2|0|0|1
cap-12x|rate = 1.2 x NETEM_BITRATE_MBPS, 2 ms delay|2|0|0|X12|0|1
cap-20x|rate = 2 x NETEM_BITRATE_MBPS, 2 ms delay|2|0|0|X20|0|1
EOF
)"
PROFILE_NAMES="$(awk -F'|' '{print $1}' <<<"$PROFILE_TABLE")"

# --- environment -------------------------------------------------------------
HOST_PID=""
OWN_KWIN=""
NETEM_APPLIED=0
PRIOR_SAVED="none"

pick_iface() {
  # Prefer the first interface that is operationally UP (the `show up` flag filter matches
  # NO-CARRIER links too — it is the admin flag, not the carrier state); fall back to any
  # non-loopback interface, never `lo`.
  local line i state
  while read -r line; do
    i="$(awk -F': ' '{print $2}' <<<"$line" | sed 's/@.*//')"
    state="$(grep -o 'state [A-Za-z]*' <<<"$line" | awk '{print $2}')"
    [[ -n "$i" && "$i" != lo && "$state" == UP ]] && { echo "$i"; return 0; }
  done < <(ip -o link show 2>/dev/null)
  ip -o link show 2>/dev/null | awk -F': ' '$2!="lo"{gsub(/@.*/,"",$2); print $2; exit}'
}
IFACE="${NETEM_IFACE:-$(pick_iface)}"

pfield() { awk -F'|' -v n="$1" -v i="$2" '$1==n{print $i; exit}' <<<"$PROFILE_TABLE"; }
qdisc_kind() { tc qdisc show dev "$IFACE" 2>/dev/null | awk '/^qdisc/ {print $2; exit}'; }

field() { grep -oE "$1=\"?[0-9]+" "$2" | tail -1 | grep -oE "[0-9]+$"; }

usage() {
  sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
  echo "profiles:"
  awk -F'|' -v b="$BITRATE_MBPS" '{
    r = $6;
    if (r ~ /^X/) { m = substr(r, 2); r = int(b * m / 10); }
    p = "delay " $3 "ms";
    if ($4 + 0 > 0) p = p " " $4 "ms distribution normal";
    if ($5 != 0) p = p " loss " $5 "%";
    if (r != 0) p = p " rate " r "mbit";
    if ($7 != 0) p = p " reorder " $7 "% " $8;
    printf "  %-10s %-54s %s\n", $1, p, $2
  }' <<<"$PROFILE_TABLE"
}

list_profiles() {
  echo "netem-latency profiles (tc netem params; delay is one-way, rtt = 2 x delay):"
  awk -F'|' -v b="$BITRATE_MBPS" '{
    r = $6;
    if (r ~ /^X/) { m = substr(r, 2); r = int(b * m / 10); }
    p = "delay " $3 "ms";
    if ($4 + 0 > 0) p = p " " $4 "ms distribution normal";
    if ($5 != 0) p = p " loss " $5 "%";
    if (r != 0) p = p " rate " r "mbit";
    if ($7 != 0) p = p " reorder " $7 "% " $8;
    printf "%-10s %-54s %s\n", $1, p, $2
  }' <<<"$PROFILE_TABLE"
  echo
  echo "env: NETEM_IFACE=$IFACE (default: first non-loopback)  NETEM_CLIENT_CONNECT=$CONNECT  NETEM_SIDE=$SIDE  NETEM_BITRATE_MBPS=$BITRATE_MBPS"
  echo "protocol: $REPS reps x ($WARMUP_SECS s warmup + $QUIET_SECS s quiet + $RUN_SECS s collection) per scenario"
  echo "results: $RESULTS_DIR/<scenario>/summary.jsonl"
}

loopback_warn() {
  case "$CONNECT" in
    127.0.0.1*|localhost*|::1*|\[::1\]*)
      if [[ "$IFACE" == lo ]]; then
        echo "!! connect target is loopback AND netem on lo is a no-op on most kernels — this run measures"
        echo "   the pipeline UNSHAPED. Point NETEM_IFACE + NETEM_CLIENT_CONNECT at a real machine boundary."
      else
        echo "!! client connects to $CONNECT (loopback) — that traffic never crosses $IFACE, so netem will NOT"
        echo "   affect this run. For real shaping set NETEM_CLIENT_CONNECT=<host-lan-ip>:9777 and run the"
        echo "   probe on another machine (see header)."
      fi ;;
  esac
}

# --- tc qdisc handling -------------------------------------------------------
build_params() {
  local name="$1" d j l r p g
  d="$(pfield "$name" 3)"; j="$(pfield "$name" 4)"; l="$(pfield "$name" 5)"
  r="$(pfield "$name" 6)"; p="$(pfield "$name" 7)"; g="$(pfield "$name" 8)"
  case "$r" in X*) r=$(( BITRATE_MBPS * ${r#X} / 10 )) ;; esac
  local params="delay ${d}ms"
  if (( j > 0 )); then params+=" ${j}ms distribution normal"; fi
  if [[ -n "$l" && "$l" != 0 ]]; then params+=" loss ${l}%"; fi
  if [[ -n "$r" && "$r" != 0 ]]; then params+=" rate ${r}mbit"; fi
  if [[ -n "$p" && "$p" != 0 ]]; then params+=" reorder ${p}% ${g:-1}"; fi
  echo "$params"
}

netem_up() {
  local name="$1"
  NETEM_APPLIED=0
  [[ "$name" == clean-lan ]] && return 0   # baseline: no shaping at all
  local prior
  prior="$(qdisc_kind)"
  case "$prior" in
    ""|noqueue)        PRIOR_SAVED=none ;;
    netem)             PRIOR_SAVED=netem ;;           # leftover from a previous run — ours
    fq|fq_codel|pfifo|pfifo_fast|bfifo|mq|mqprio) PRIOR_SAVED="$prior" ;;  # safe default, restored later
    *)
      echo "!! refusing to shape $IFACE: it already carries a '${prior}' root qdisc — that is a real"
      echo "   config, not a safe default, and this harness will not nuke it."
      echo "   inspect: tc qdisc show dev $IFACE"
      echo "   remove if you own it: sudo tc qdisc del dev $IFACE root"
      echo "   or pick another interface: NETEM_IFACE=<iface> $0 $name"
      exit 1 ;;
  esac
  local params
  params="$(build_params "$name")"
  [[ "$prior" == netem ]] && tc qdisc del dev "$IFACE" root 2>/dev/null
  if ! tc qdisc add dev "$IFACE" root handle 1: netem $params; then
    echo "!! tc qdisc add failed (see error above) — nothing was changed on $IFACE"
    exit 1
  fi
  NETEM_APPLIED=1
  echo "==> shaping $IFACE: netem $params (prior qdisc: ${PRIOR_SAVED:-none})"
}

netem_down() {
  [[ "$NETEM_APPLIED" == 1 ]] || return 0
  tc qdisc del dev "$IFACE" root 2>/dev/null || true
  case "${PRIOR_SAVED:-}" in
    ""|none|netem) ;;
    fq|fq_codel|pfifo|pfifo_fast|bfifo|mq|mqprio) tc qdisc add dev "$IFACE" root "$PRIOR_SAVED" 2>/dev/null || true ;;
  esac
  echo "==> qdisc on $IFACE removed (restored: ${PRIOR_SAVED:-none})"
  NETEM_APPLIED=0
}

cleanup() {
  [[ -n "$HOST_PID" ]] && kill "$HOST_PID" 2>/dev/null
  [[ -n "$OWN_KWIN" ]] && pkill -f "kwin_wayland --virtual" 2>/dev/null
  netem_down
}
trap cleanup EXIT

# --- prerequisites -----------------------------------------------------------
require_prereqs() {
  [[ "$(id -u)" == 0 ]] || {
    echo "!! netem-latency needs root for tc/ip — rerun under sudo: sudo $0 $*"; exit 1; }
  command -v tc >/dev/null 2>&1 || { echo "!! tc not found (iproute2)"; exit 1; }
  command -v ip >/dev/null 2>&1 || { echo "!! ip not found (iproute2)"; exit 1; }
  command -v python3 >/dev/null 2>&1 || { echo "!! python3 not found"; exit 1; }
  ip link show dev "$IFACE" >/dev/null 2>&1 || {
    echo "!! interface $IFACE does not exist — override with NETEM_IFACE"; exit 1; }
  [[ "$SIDE" == client ]] || [[ -x target/release/slipstream-host ]] || {
    echo "!! target/release/slipstream-host missing — run: cargo build -rq -p slipstream-host -p slipstream-probe"
    exit 1; }
  [[ -x target/release/slipstream-probe ]] || {
    echo "!! target/release/slipstream-probe missing — run: cargo build -rq -p slipstream-host -p slipstream-probe"
    exit 1; }
}

check_env() {
  local fail=0 tool bin qd
  echo "== netem-latency harness — environment check"
  if [[ "$(id -u)" == 0 ]]; then
    echo "[PASS] running as root (uid 0) — run mode can shape $IFACE"
  else
    echo "[WARN] not root (uid $(id -u)) — run mode needs root for tc/ip; rerun under sudo"
  fi
  for tool in tc ip python3; do
    if command -v "$tool" >/dev/null 2>&1; then echo "[PASS] $tool at $(command -v "$tool")"
    else echo "[FAIL] $tool not found (iproute2 / python3)"; fail=1; fi
  done
  [[ "$SIDE" == client ]] || {
    if [[ -x target/release/slipstream-host ]]; then echo "[PASS] target/release/slipstream-host"
    else echo "[FAIL] target/release/slipstream-host missing — run: cargo build -rq -p slipstream-host -p slipstream-probe"; fail=1; fi; }
  if [[ -x target/release/slipstream-probe ]]; then echo "[PASS] target/release/slipstream-probe"
  else echo "[FAIL] target/release/slipstream-probe missing — run: cargo build -rq -p slipstream-host -p slipstream-probe"; fail=1; fi
  if ip link show dev "$IFACE" >/dev/null 2>&1; then
    echo "[PASS] interface $IFACE exists (NETEM_IFACE; default: first non-loopback)"
  else echo "[FAIL] interface $IFACE does not exist — override with NETEM_IFACE"; fail=1; fi
  qd="$(qdisc_kind)"
  case "$qd" in
    ""|noqueue)  echo "[INFO] $IFACE root qdisc: ${qd:-none} — no real shaping; netem removed after the run" ;;
    netem)       echo "[INFO] $IFACE already carries a netem qdisc (leftover from a previous run?) — replaced, then removed" ;;
    fq|fq_codel|pfifo|pfifo_fast|bfifo|mq|mqprio)
                 echo "[INFO] $IFACE root qdisc: $qd (safe default — replaced for the run, restored after)" ;;
    *)           echo "[FAIL] $IFACE root qdisc: $qd — harness refuses to replace non-default shaping. Remove with: sudo tc qdisc del dev $IFACE root"
                 fail=1 ;;
  esac
  case "$CONNECT" in
    127.0.0.1*|localhost*|::1*|\[::1\]*) echo "[INFO] connect target $CONNECT is loopback — netem on $IFACE will NOT shape it; set NETEM_CLIENT_CONNECT to a LAN IP for a cross-machine run" ;;
  esac
  echo
  if (( fail )); then echo "!! fix the FAIL lines above, then rerun --check"; exit 1; fi
  echo "== all hard prerequisites present — run mode is go (root permitting)"
}

# --- run machinery -----------------------------------------------------------
bring_up_session() {
  export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
  export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-kde}"
  export XDG_CURRENT_DESKTOP="${XDG_CURRENT_DESKTOP:-KDE}"
  export SLIPSTREAM_COMPOSITOR="${SLIPSTREAM_COMPOSITOR:-kwin}"
  export SLIPSTREAM_VIDEO_SOURCE=virtual SLIPSTREAM_PERF=1
  export SLIPSTREAM_ZEROCOPY="${SLIPSTREAM_ZEROCOPY:-1}"
  if [[ ! -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]]; then
    echo "==> no $WAYLAND_DISPLAY — bringing up a headless KWin session for the virtual output"
    setsid bash scripts/headless/run-headless-kde.sh "${MODE_RES%x*}" </dev/null >/tmp/netem-bench-kwin.log 2>&1 &
    OWN_KWIN=$!
    for _ in $(seq 1 30); do [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] && break; sleep 1; done
    [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] ||
      echo "!! headless KWin did not come up — host capture may fail; see /tmp/netem-bench-kwin.log"
  fi
}

# One warmup or collection run of the pipeline into $dir (logs + latency artifacts).
run_pair() {
  local secs="$1" dir="$2"
  mkdir -p "$dir"
  if [[ "$SIDE" == host ]]; then
    SLIPSTREAM_PERF=1 \
    SLIPSTREAM_LATENCY_ARTIFACT="$dir/host_artifact.jsonl" \
      target/release/slipstream-host slipstream1-host --source virtual --seconds "$secs" --max-sessions 1 \
      >"$dir/host.log" 2>&1 &
    HOST_PID=$!
  fi
  sleep 3
  SLIPSTREAM_CLIENT_ARTIFACT="$dir/client_artifact.jsonl" \
    target/release/slipstream-probe --connect "$CONNECT" --mode "$MODE_RES" --out /dev/null \
    >"$dir/client.log" 2>&1 || true
  if [[ "$SIDE" == host ]]; then
    wait "$HOST_PID" 2>/dev/null || true
    HOST_PID=""
  fi
}

summarize() {
  local scenario="$1" rep="$2" dir="$3"
  local rtt jitter loss rate lat_p50 lat_p95 lat_p99 lat_max samples enc tx drops line
  rtt=$(( $(pfield "$scenario" 3) * 2 ))
  jitter="$(pfield "$scenario" 4)"; loss="$(pfield "$scenario" 5)"; rate="$(pfield "$scenario" 6)"
  case "$rate" in X*) rate=$(( BITRATE_MBPS * ${rate#X} / 10 )) ;; esac
  lat_p50="$(field lat_p50_us "$dir/client.log")"; lat_p95="$(field lat_p95_us "$dir/client.log")"
  lat_p99="$(field lat_p99_us "$dir/client.log")"; lat_max="$(field lat_max_us "$dir/client.log")"
  samples="$(field frames "$dir/client.log")"
  enc="$(field encode_us_p50 "$dir/host.log")"; tx="$(field tx_mbps "$dir/host.log")"
  drops="$(field send_dropped_total "$dir/host.log")"
  if [[ -z "$lat_p50" ]]; then
    echo "!! $scenario rep $rep: no client latency metrics (pipeline likely failed). client.log tail:"
    tail -6 "$dir/client.log" 2>/dev/null | sed 's/^/   /'
    echo "   host.log tail:"; tail -6 "$dir/host.log" 2>/dev/null | sed 's/^/   /'
  fi
  line="$(python3 - "$scenario" "$rep" "$rtt" "$jitter" "$loss" "$rate" \
    "$lat_p50" "$lat_p95" "$lat_p99" "$lat_max" "$samples" "$drops" "$enc" "$tx" <<'PY'
import json, sys
sc, rep, rtt, jit, loss, rate, p50, p95, p99, mx, samples, drops, enc, tx = sys.argv[1:]
def num(s):
    if s == "":
        return None
    try:
        return int(s)
    except ValueError:
        pass
    try:
        return float(s)
    except ValueError:
        return None
print(json.dumps(dict(scenario=sc, repetition=int(rep), rtt_ms=num(rtt), jitter_ms=num(jit),
                      loss_pct=num(loss), rate_mbps=num(rate), lat_p50_us=num(p50),
                      lat_p95_us=num(p95), lat_p99_us=num(p99), lat_max_us=num(mx),
                      samples=num(samples), drops=num(drops), encode_us_p50=num(enc),
                      tx_mbps=num(tx))))
PY
)"
  echo "$line" >>"$RESULTS_DIR/$scenario/summary.jsonl"
  echo "==>   $scenario rep $rep: rtt=${rtt}ms p50=${lat_p50:-?}us p95=${lat_p95:-?}us p99=${lat_p99:-?}us max=${lat_max:-?}us frames=${samples:-0} drops=${drops:-0} enc_p50=${enc:-?}us tx=${tx:-?}mbps"
}

run_scenario() {
  local scenario="$1" rep
  netem_up "$scenario"
  echo "==> scenario: $scenario — $(pfield "$scenario" 2)"
  for rep in $(seq 1 "$REPS"); do
    echo "==>   rep $rep/$REPS: warmup ${WARMUP_SECS}s (netem live)"
    run_pair "$WARMUP_SECS" "$RESULTS_DIR/$scenario/warmup-$rep"
    sleep "$QUIET_SECS"
    echo "==>   rep $rep/$REPS: collection ${RUN_SECS}s"
    run_pair "$RUN_SECS" "$RESULTS_DIR/$scenario/$rep"
    summarize "$scenario" "$rep" "$RESULTS_DIR/$scenario/$rep"
    if (( rep < REPS )); then echo "==>   quiet ${QUIET_SECS}s"; sleep "$QUIET_SECS"; fi
  done
  netem_down
}

run_all() {
  local scenario list n
  require_prereqs "$@"
  bring_up_session
  loopback_warn
  if [[ "$SCENARIO" == all ]]; then list="$PROFILE_NAMES"; else list="$SCENARIO"; fi
  n="$(echo "$list" | sed '/^$/d' | wc -l)"
  echo "==> $n scenario(s) x $REPS reps x (${WARMUP_SECS}s warmup + ${QUIET_SECS}s quiet + ${RUN_SECS}s collect) ~ $(( n * REPS * (WARMUP_SECS + RUN_SECS + QUIET_SECS * 2) / 60 )) min"
  echo "==> results: $RESULTS_DIR/<scenario>/summary.jsonl"
  while read -r scenario; do
    [[ -n "$scenario" ]] || continue
    run_scenario "$scenario"
  done <<<"$list"
  echo "==> sweep complete. aggregate with: scripts/bench/netem-latency.sh analyze"
  exit 0
}

# --- main --------------------------------------------------------------------
SCENARIO="${1:-all}"
MODE="${2:-run}"
case "$SCENARIO" in
  --list|-l)      list_profiles; exit 0 ;;
  --check|-c)     check_env; exit 0 ;;
  --help|-h)      usage; exit 0 ;;
  analyze)        MODE=analyze; SCENARIO=all ;;
esac
if [[ "$MODE" == analyze ]]; then
  dir="$RESULTS_DIR"
  [[ "$SCENARIO" != all ]] && dir="$dir/$SCENARIO"
  exec python3 scripts/bench/netem-analyze.py "$dir"
fi
[[ "$MODE" == run ]] || { echo "!! unknown mode '$MODE' (run|analyze)"; usage; exit 1; }
if [[ "$SCENARIO" != all ]] && ! grep -qx "$SCENARIO" <<<"$PROFILE_NAMES"; then
  echo "!! unknown scenario '$SCENARIO' — see --list"; exit 1
fi
[[ -n "$IFACE" ]] || { echo "!! no usable interface found (ip link show); set NETEM_IFACE"; exit 1; }
run_all "$@"
