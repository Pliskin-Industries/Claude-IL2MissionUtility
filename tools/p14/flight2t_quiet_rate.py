"""Flight 2T: counts quiet slots and jitter steps from the mission text logs.

Usage: python tools/p14/flight2t_quiet_rate.py <log folder> [last tick s]

Each clock ticks at 60, 120, ... s (`CLK x FIRST` and `PERIOD` are 60 s; TICK has no
breadcrumb). A slot is quiet when no `SLOT OPEN` breadcrumb comes before the next
tick; otherwise its step is SLOT OPEN - TICK (0.1, 15.1 or 30.1 s). Each `JITTER k`
breadcrumb logs when that random timer passes. Run sheet:
handoff/sessions/2026-11-05-p14-flights-a4-2t.md.
"""
import math
import re
import sys
from pathlib import Path

CRUMB = re.compile(r"^T:(\d+) AType:8 .*POS\(([\d.]+),[\d.]+,5000\.0+\)")
# Breadcrumb x positions from handoff/p14-probes/flight-2t/P14_Probe_2T.trace.json.
SIDES = {
    "DPRK": {"open": 5000, "jitter": (5080, 5090, 5100), "go": 5170, "inner": 5190},
    "NATO": {"open": 5310, "jitter": (5390, 5400, 5410), "go": 5480, "inner": 5500},
}
STARTS = range(5630, 6211, 100)  # SD01..SN03 START: 5630, 5730, ... 6130
STEPS = (0.1, 15.1, 30.1)
PASS_PCT = (27, 36, 57)
PERIOD = 60.0


def read(folder):
    crumbs = []
    for f in sorted(Path(folder).glob("missionReport*.txt")):
        for line in f.read_text(encoding="utf-8", errors="replace").splitlines():
            m = CRUMB.match(line)
            if m:
                crumbs.append(((int(m.group(1)) - 3) / 50, int(float(m.group(2)))))
    return sorted(crumbs)


def binom_range(n, p):
    sd = math.sqrt(n * p * (1 - p))
    return round(n * p - 1.96 * sd), round(n * p + 1.96 * sd)


def main():
    folder = sys.argv[1]
    crumbs = read(folder)
    last = max(t for t, _ in crumbs) if crumbs else 0
    last_tick = float(sys.argv[2]) if len(sys.argv) > 2 else PERIOD * math.floor((last - 31) / PERIOD)
    ticks = [PERIOD * k for k in range(1, int(last_tick // PERIOD) + 1)]
    print(f"crumbs {len(crumbs)}, last {last:.2f} s, ticks 60 to {ticks[-1]:.0f} s ({len(ticks)} per side)")
    total = {"quiet": 0, "slots": 0, "steps": [0, 0, 0], "jitter": [0, 0, 0]}
    for side, x in SIDES.items():
        opens = [t for t, p in crumbs if p == x["open"]]
        quiet, steps, odd, run, longest = 0, [0, 0, 0], [], 0, 0
        for tick in ticks:
            inside = [t - tick for t in opens if tick <= t < tick + PERIOD]
            if not inside:
                quiet += 1
                run += 1
                longest = max(longest, run)
                continue
            run = 0
            if len(inside) > 1:
                odd.append((tick, inside))
            k = min(range(3), key=lambda i: abs(inside[0] - STEPS[i]))
            if abs(inside[0] - STEPS[k]) > 0.05:
                odd.append((tick, inside))
            steps[k] += 1
        jitter = []
        for k, jx in enumerate(x["jitter"]):
            jitter.append(sum(1 for t, p in crumbs if p == jx and ticks[0] <= t < ticks[-1] + PERIOD))
        print(f"{side}: quiet {quiet}/{len(ticks)}, steps {steps}, longest quiet run {longest}, "
              f"JITTER passes {jitter}, GO {sum(1 for _, p in crumbs if p == x['go'])}, "
              f"INNER {sum(1 for _, p in crumbs if p == x['inner'])}, off-step or double {odd[:5]}")
        total["quiet"] += quiet
        total["slots"] += len(ticks)
        total["steps"] = [a + b for a, b in zip(total["steps"], steps)]
        total["jitter"] = [a + b for a, b in zip(total["jitter"], jitter)]
    n = total["slots"]
    print(f"ALL: quiet {total['quiet']}/{n} = {100 * total['quiet'] / n:.1f} % (95 % range {binom_range(n, 0.2)})")
    p_steps = (0.27, 0.73 * 0.36, 0.73 * 0.64 * 0.57)
    for k in range(3):
        print(f"  step {k + 1}: {total['steps'][k]} (expected {n * p_steps[k]:.1f}, range {binom_range(n, p_steps[k])}); "
              f"JITTER {k + 1} passes {total['jitter'][k]} (expected {n * PASS_PCT[k] / 100:.1f}, range {binom_range(n, PASS_PCT[k] / 100)})")
    starts = sum(1 for _, p in crumbs if p in STARTS)
    print(f"copy STARTs: {starts}")


if __name__ == "__main__":
    main()
