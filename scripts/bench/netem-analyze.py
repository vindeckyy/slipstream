#!/usr/bin/env python3
"""Aggregate a `tc netem` latency sweep written by scripts/bench/netem-latency.sh.

Reads every `<scenario>/summary.jsonl` under the results dir (arg; default
scripts/bench/results/netem) and prints a markdown table: one row per
(scenario, repetition) with p50/p95/p99/max latency, frame count and drop count,
plus a per-scenario median row across the repetitions. Scenarios whose median p95
sits more than 5% above the clean-lan median p95 are flagged ⚠.

Report-only: always exits 0, even when regressions are found — the gate decision
stays with the bench driver / a human.

Aggregation rule: percentiles are per-repetition statistics. The per-scenario
aggregate takes the MEDIAN across repetitions of each column; raw samples from
independent runs are never pooled and percentiles are never recomputed from
combined stage data (pooling independent stage percentiles would be wrong).

  python3 scripts/bench/netem-analyze.py [results-dir]
"""
import argparse
import glob
import json
import os
import statistics
import sys

COLS = ("lat_p50_us", "lat_p95_us", "lat_p99_us", "lat_max_us", "samples", "drops")
REG_THRESH = 0.05  # >5% p95 deviation vs clean-lan flags ⚠


def med(vals):
    vals = [v for v in vals if v is not None]
    return statistics.median(vals) if vals else None


def fmt(v):
    return "—" if v is None else f"{v:g}"


def pct(part, base):
    if part is None or base is None or base == 0:
        return "—"
    return f"{(part - base) / base:+.1%}"


def load(dirpath):
    """scenario -> {repetition: row}, last row wins per (scenario, repetition)."""
    by = {}
    for f in sorted(glob.glob(os.path.join(dirpath, "**", "summary.jsonl"), recursive=True)):
        scenario = os.path.basename(os.path.dirname(f))
        try:
            with open(f) as fh:
                for line in fh:
                    line = line.strip()
                    if not line:
                        continue
                    row = json.loads(line)
                    by.setdefault(scenario, {})[int(row.get("repetition", 0))] = row
        except (OSError, ValueError) as e:
            print(f"note: skipping unreadable {f}: {e}", file=sys.stderr)
    return by


def main():
    ap = argparse.ArgumentParser(description="Aggregate netem-latency.sh sweep results (report-only).")
    ap.add_argument("dir", nargs="?", default=None,
                    help="results dir (default: repo scripts/bench/results/netem)")
    args = ap.parse_args()

    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    d = args.dir or os.path.join(root, "scripts", "bench", "results", "netem")
    if not os.path.isdir(d):
        print(f"no results dir: {d} — run scripts/bench/netem-latency.sh <scenario> first")
        return 0
    by = load(d)
    if not by:
        print(f"no summary.jsonl under {d} — nothing to aggregate")
        return 0

    scenarios = sorted(by)
    if "clean-lan" in scenarios:
        scenarios.remove("clean-lan")
        scenarios.insert(0, "clean-lan")
    base = med([r.get("lat_p95_us") for r in by.get("clean-lan", {}).values()])

    out = [f"## netem latency sweep — {len(scenarios)} scenario(s) from `{d}` (report-only)", ""]
    out.append("One row per (scenario, repetition); `median` rows aggregate the repetitions "
               "(median of each column — raw samples are never pooled). Δp95 is vs the "
               "clean-lan median p95; ⚠ = scenario median p95 more than 5% above it.")
    out.append("")
    out.append("| scenario | rep | p50 µs | p95 µs | p99 µs | max µs | samples | drops | Δp95 |")
    out.append("|---|---|---:|---:|---:|---:|---:|---:|---:|")

    flagged = []
    for sc in scenarios:
        rows = by[sc]
        for rep in sorted(rows):
            r = rows[rep]
            cells = " | ".join(fmt(r.get(c)) for c in COLS)
            out.append(f"| {sc} | {rep} | {cells} | {pct(r.get('lat_p95_us'), base)} |")
        meds = {c: med([r.get(c) for r in rows.values()]) for c in COLS}
        cells = " | ".join(fmt(meds[c]) for c in COLS)
        flag = ""
        if sc != "clean-lan" and meds["lat_p95_us"] is not None and base is not None:
            dev = (meds["lat_p95_us"] - base) / base
            if dev > REG_THRESH:
                flag = " ⚠"
                flagged.append((sc, dev))
        out.append(f"| **{sc}** | median({len(rows)}) | {cells} | {pct(meds['lat_p95_us'], base)}{flag} |")

    table = "\n".join(out)
    print(table)

    if base is None:
        print("\nnote: no clean-lan rows — baseline missing, regressions not flagged")
    if flagged:
        print("\n⚠ p95 >5% above clean-lan median:")
        for sc, dev in flagged:
            print(f"   {sc}: {dev:+.1%}")

    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a") as f:
            f.write(table + "\n")
    return 0  # report-only; the CI gate lives in the bench driver


if __name__ == "__main__":
    sys.exit(main())
