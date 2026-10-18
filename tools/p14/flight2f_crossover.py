"""Wingman slot relative to its lead around the turn reversal after WP 1 (flight 2F runs).

Usage: python tools/p14/flight2f_crossover.py <run dir>. Prints, per cell and pair, the wingman's
forward/right/up offset from its lead before WP 1 and every crossing of the
lead's track (sign change of 'right') from WP 1 - 5 s to WP 1 + 60 s, with the
fore-aft and vertical offsets and the distance at the crossing.
"""
import sys, re, glob, math

CELLS = ["PD", "PS", "PL", "WD", "WS", "WL"]
STARTS = [60, 390, 720, 1050, 1380, 1710]
TRANSIT = 264.1
GTIME = (12, 30, 0)
run = sys.argv[1]
acmi = glob.glob(run + "/*.acmi")[0]
logs = glob.glob(run + "/missionReport*.txt")
text = "".join(open(p, encoding="utf-8", errors="replace").read() for p in logs)
act = {int(m.group(2)): int(m.group(1)) for m in re.finditer(r"T:(\d+) AType:28 OBJID:(\d+)", text)}
starts = sorted(set(act.values()))
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
    if "yaw" in s: hist.setdefault(o, []).append((round(t, 1), dict(s)))

by_name = {}
for o, h in hist.items():
    by_name[(cell_of[o], h[-1][1]["name"].split(" - ")[-1])] = dict(h)

def rel(lead, wing):
    n = (wing["lat"] - lead["lat"]) * 111320.0
    e = (wing["lon"] - lead["lon"]) * 111320.0 * math.cos(math.radians(lead["lat"]))
    hd = math.radians(lead["yaw"])
    fwd = n * math.cos(hd) + e * math.sin(hd)
    right = e * math.cos(hd) - n * math.sin(hd)
    return fwd, right, wing["alt"] - lead["alt"], math.hypot(n, e, wing["alt"] - lead["alt"])

for c, s0 in zip(CELLS, STARTS):
    wp1 = s0 + TRANSIT
    for lead_n, wing_n in (("Red 11", "Red 12"), ("Blue 21", "Blue 22")):
        L, W = by_name.get((c, lead_n)), by_name.get((c, wing_n))
        if not L or not W: continue
        ts = sorted(tt for tt in L if tt in W and wp1 - 5 <= tt <= wp1 + 60)
        if not ts: continue
        f0, r0, u0, d0 = rel(L[ts[0]], W[ts[0]])
        out = [f"{c} {lead_n[:-3]}: slot at WP1-5 s fwd {f0:+.0f} right {r0:+.0f} up {u0:+.0f} m"]
        prev = None; dmin = (1e9, 0)
        for tt in ts:
            f, r, u, d = rel(L[tt], W[tt])
            if d < dmin[0]: dmin = (d, tt)
            if prev is not None and (prev > 0) != (r > 0):
                out.append(f"  crosses at WP1{tt - wp1:+.1f} s: fwd {f:+.0f} up {u:+.0f} m, dist {d:.0f} m")
            prev = r
        out.append(f"  closest {dmin[0]:.1f} m at WP1{dmin[1] - wp1:+.1f} s")
        print("\n".join(out))
