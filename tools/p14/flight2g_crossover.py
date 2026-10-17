"""Every crossing of the lead's track by its wingman, flight 2G runs.

Usage: python tools/p14/flight2g_crossover.py <run dir>
Per cell and pair: the wingman's slot (forward, right, up from its lead, in the
lead's heading frame) averaged over WP 1 - 30 s to WP 1 - 5 s; the lead's bank
on arrival (each change of bank sign from WP 1 to + 40 s, the reversal); the
lead's ground speed before and after WP 1; and every crossing of the lead's
track (sign change of 'right') from WP 1 - 5 s to the delete (WP 1 + 361 s),
with the time after WP 1, fore-aft and vertical offset, the distance, and the
closest pass within 3 s of it (crossings 300 m or more away are counted, not listed). Positions at Tacview's frame rate; both
aircraft must have a frame within 0.1 s. Adapted from flight2f_crossover.py.
Results: handoff/P14-probe-results.md, "Flight 2G".
"""
import sys, re, glob, math, bisect

CELLS = ["GC", "G3", "G4", "GX", "GA"]
STARTS = [60, 390, 720, 1050, 1380]
TRANSIT = 264.1                                # flight 2E's measured Yak transit
DONE_AFTER_WP1, DELETE_AFTER_WP1 = 300.5, 361.1
GTIME = (12, 30, 0)
run = sys.argv[1]
acmi = glob.glob(run + "/*.acmi")[0]
logs = glob.glob(run + "/missionReport*.txt")
text = "".join(open(p, encoding="utf-8", errors="replace").read() for p in logs)
act = {int(m.group(2)): int(m.group(1)) for m in re.finditer(r"T:(\d+) AType:28 OBJID:(\d+)", text)}
starts = sorted(set(act.values()))
assert len(starts) == len(CELLS), f"expected {len(CELLS)} activation times, got {len(starts)}"
cell_of = {o: CELLS[starts.index(t)] for o, t in act.items()}
head = open(acmi, encoding="utf-8", errors="replace").read(400)
rt = re.search(r"ReferenceTime=\d+-\d+-\d+T(\d+):(\d+):(\d+)", head)
off = (int(rt.group(1)) - GTIME[0]) * 3600 + (int(rt.group(2)) - GTIME[1]) * 60 + int(rt.group(3)) - GTIME[2]

st, hist, t = {}, {}, 0.0
for line in open(acmi, encoding="utf-8", errors="replace"):
    line = line.rstrip("\n")
    if line.startswith("#"):
        t = float(line[1:]) + off; continue
    m = re.match(r"([0-9a-f]+),T=([^,]*)(.*)", line)
    if not m: continue
    o = int(m.group(1), 16)
    if o not in cell_of: continue
    s = st.setdefault(o, {})
    nm = re.search(r",Name=([^,]*)", m.group(3))
    if nm: s["name"] = nm.group(1)
    p = m.group(2).split("|")
    for i, k in enumerate(("lon", "lat", "alt", "roll", "pitch", "yaw")[:len(p)]):
        if p[i] != "": s[k] = float(p[i])
    if "yaw" in s and "lat" in s: hist.setdefault(o, []).append((t, dict(s)))

by_name = {}
for o, h in hist.items():
    by_name[(cell_of[o], h[-1][1]["name"].split(" - ")[-1])] = h

def at(h, ts, when, tol=0.1):
    i = bisect.bisect_left(ts, when - tol)
    best = None
    for j in (i, i + 1, i + 2):
        if j < len(h) and abs(h[j][0] - when) <= tol and (best is None or abs(h[j][0] - when) < abs(best[0] - when)):
            best = h[j]
    return best[1] if best else None

def rel(lead, wing):
    n = (wing["lat"] - lead["lat"]) * 111320.0
    e = (wing["lon"] - lead["lon"]) * 111320.0 * math.cos(math.radians((lead["lat"] + wing["lat"]) / 2))
    hd = math.radians(lead["yaw"])
    fwd = n * math.cos(hd) + e * math.sin(hd)
    right = e * math.cos(hd) - n * math.sin(hd)
    up = wing["alt"] - lead["alt"]
    return fwd, right, up, math.hypot(n, e, up)

def speed(h, ts, a, b):
    sa, sb = at(h, ts, a, 0.3), at(h, ts, b, 0.3)
    if not sa or not sb: return float("nan")
    n = (sb["lat"] - sa["lat"]) * 111320.0
    e = (sb["lon"] - sa["lon"]) * 111320.0 * math.cos(math.radians(sa["lat"]))
    return math.hypot(n, e) / (b - a) * 3.6

summary = []
for c, s0 in zip(CELLS, STARTS):
    wp1 = s0 + TRANSIT
    for lead_n, wing_n in (("Red 11", "Red 12"), ("Blue 21", "Blue 22")):
        L, W = by_name.get((c, lead_n)), by_name.get((c, wing_n))
        if not L or not W:
            print(f"{c} {lead_n[:-3]}: no track for the pair"); continue
        lt, wt = [x[0] for x in L], [x[0] for x in W]
        slot = [rel(a, b) for a, b in ((at(L, lt, tt), at(W, wt, tt)) for tt, _ in L if wp1 - 30 <= tt <= wp1 - 5) if a and b]
        if slot:
            f0, r0, u0 = (sum(v[i] for v in slot) / len(slot) for i in range(3))
            print(f"{c} {lead_n[:-3]}: slot before WP 1: {-f0:.0f} behind, {r0:.0f} right, {-u0:.0f} below (m)")
        banks = [(tt, s["roll"]) for tt, s in L if wp1 <= tt <= wp1 + 40]
        rev = [f"{tt - wp1:+.1f} s ({'right' if r > 0 else 'left'})" for (tp, rp), (tt, r) in zip(banks, banks[1:])
               if (rp > 10 and r < -10) or (rp < -10 and r > 10) or (abs(rp) <= 10 < abs(r))]
        print(f"  lead bank on arrival (into a bank or through level, WP 1 to + 40 s): {', '.join(rev[:6]) or 'none'}"
              + (f" and {len(rev) - 6} more" if len(rev) > 6 else ""))
        print(f"  lead ground speed: WP 1 - 20 to - 5 s {speed(L, lt, wp1 - 20, wp1 - 5):.0f} km/h,"
              f" + 5 to + 20 s {speed(L, lt, wp1 + 5, wp1 + 20):.0f} km/h")
        prev, dmin, n20, crossings = None, (1e9, 0.0), 0, []
        for tt, ls in L:
            if not (wp1 - 5 <= tt <= wp1 + DELETE_AFTER_WP1): continue
            ws = at(W, wt, tt)
            if not ws: continue
            f, r, u, d = rel(ls, ws)
            if d < dmin[0]: dmin = (d, tt)
            if prev is not None and (prev > 0) != (r > 0):
                near = min((rel(a, b)[3] for a, b in ((at(L, lt, x), at(W, wt, x)) for x, _ in L if abs(x - tt) <= 3) if a and b), default=d)
                crossings.append((tt, f, u, d, near))
            prev = r
        for tt, f, u, d, near in crossings:
            n20 += near < 20
            if near >= 300: continue
            mark = "  <20 m" if near < 20 else ""
            phase = "after DONE HUB" if tt - wp1 > DONE_AFTER_WP1 else ""
            print(f"  crosses at WP 1 {tt - wp1:+.1f} s: {'ahead' if f > 0 else 'behind'} {abs(f):.0f} m,"
                  f" {'above' if u > 0 else 'below'} {abs(u):.0f} m, dist {d:.0f} m, closest within 3 s {near:.1f} m {phase}{mark}")
        print(f"  {len(crossings)} crossings, {n20} within 20 m; closest {dmin[0]:.1f} m at WP 1 {dmin[1] - wp1:+.1f} s")
        summary.append((c, lead_n[:-3], len(crossings), n20, dmin[0]))
print("\nSummary (cell, pair, crossings, within 20 m, closest m):")
for row in summary: print("  %s %-4s %3d %3d %6.1f" % row)
