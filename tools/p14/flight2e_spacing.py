"""Flight 2E: spacing per flight and every loss, from one run folder.

Usage: python tools/p14/flight2e_spacing.py <run dir>
The run dir holds one Tacview .acmi (unzipped) and the run's missionReport
logs. Cells are told apart by activation time (AType:28, in START order);
flights by the Tacview Name suffix (Red 1x / Blue 2x). Sampled once a second.
"Same flight" is the nearest aircraft of the aircraft's own flight; "other
flight" the nearest aircraft of the other flight in the same cell. "<20 m"
counts aircraft-seconds. Phases use the measured WP 1 times (replay, all three
runs within 0.1 s): transit before WP 1, descent the next 180 s, then station.
Results: handoff/P14-probe-results.md, "Flight 2E".
"""
import sys, re, glob, math

CELLS = ["EA", "EY", "EI", "EC"]          # START order: 60, 120, 180, 240 s
WP1 = {"EA": 378.5, "EY": 384.1, "EI": 463.1, "EC": 553.1}
GTIME = (12, 30, 0)                        # the mission's start time of day

run = sys.argv[1]
acmi = glob.glob(run + "/*.acmi")[0]
logs = sorted(glob.glob(run + "/missionReport*.txt"),
              key=lambda p: int(re.search(r"\[(\d+)\]", p).group(1)))
text = "".join(open(p, encoding="utf-8", errors="replace").read() for p in logs).replace("\r", "")
secs = lambda t: (int(t) - 3) / 50

act = {int(m.group(2)): int(m.group(1)) for m in re.finditer(r"T:(\d+) AType:28 OBJID:(\d+)", text)}
starts = sorted(set(act.values()))
assert len(starts) == len(CELLS), f"expected {len(CELLS)} activation times, got {len(starts)}"
cell_of = {o: CELLS[starts.index(t)] for o, t in act.items()}
kills = [(secs(m.group(1)), int(m.group(2))) for m in re.finditer(r"T:(\d+) AType:3 AID:-1 TID:(\d+)", text)]
attacked = re.findall(r"AType:3 AID:(?!-1)\d+", text)

head = open(acmi, encoding="utf-8", errors="replace").read(400)
rt = re.search(r"ReferenceTime=\d+-\d+-\d+T(\d+):(\d+):(\d+)", head)
off = (int(rt.group(1)) - GTIME[0]) * 3600 + (int(rt.group(2)) - GTIME[1]) * 60 + int(rt.group(3)) - GTIME[2]

st = {}                                    # object -> state
hist = {}                                  # object -> [(t, x, y, z, agl)]
t = 0.0; last = -1
nn = {c: {f: {ph: [] for ph in ("transit", "descent", "station")} for f in ("same", "other")} for c in CELLS}
close = {c: {"same": 0, "other": 0} for c in CELLS}
mins = {c: {"same": (1e9, None), "other": (1e9, None)} for c in CELLS}

def dist(a, b):
    # Scale the longitude difference, not each longitude: at 127 E,
    # lon * cos(lat) per aircraft adds about 1.4 m east per metre north.
    north = (b["lat"] - a["lat"]) * 111320.0
    east = (b["lon"] - a["lon"]) * 111320.0 * math.cos(math.radians((a["lat"] + b["lat"]) / 2))
    return math.hypot(north, east, b["alt"] - a["alt"])

def snap():
    live = [(o, s) for o, s in st.items() if "lat" in s and not s.get("gone")]
    for o, a in live:
        c = cell_of[o]
        ph = "transit" if t < WP1[c] else ("descent" if t < WP1[c] + 180 else "station")
        for kind in ("same", "other"):
            ds = [(dist(a, b), b["name"]) for p, b in live
                  if p != o and cell_of[p] == c and (b["flight"] == a["flight"]) == (kind == "same")]
            if ds:
                d, who = min(ds)
                nn[c][kind][ph].append(d)
                if d < 20: close[c][kind] += 1
                if d < mins[c][kind][0]: mins[c][kind] = (d, f"{a['name']} / {who} at {t:.0f} s")

for line in open(acmi, encoding="utf-8", errors="replace"):
    line = line.rstrip("\n")
    if line.startswith("#"):
        t = float(line[1:]) + off
        if int(t) != last:
            last = int(t); snap()
        continue
    if line.startswith("-"):
        try:
            o = int(line[1:], 16)
            if o in st: st[o]["gone"] = True
        except ValueError:
            pass
        continue
    m = re.match(r"([0-9a-f]+),T=([^,]*)(.*)", line)
    if not m: continue
    o = int(m.group(1), 16)
    if o not in cell_of: continue
    s = st.setdefault(o, {})
    nm = re.search(r",Name=([^,]*)", m.group(3))
    if nm:
        s["name"] = nm.group(1); s["flight"] = "Red" if " Red " in nm.group(1) + " " else "Blue"
    agl = re.search(r",AGL=([-0-9.]+)", m.group(3))
    if agl: s["agl"] = float(agl.group(1))
    p = m.group(2).split("|")
    for k, i in (("lon", 0), ("lat", 1), ("alt", 2)):
        if len(p) > i and p[i] != "": s[k] = float(p[i])
    if "lat" in s:
        hist.setdefault(o, []).append((t, dict(s)))

def q(v, p):
    v = sorted(v); return v[int(p * (len(v) - 1))] if v else float("nan")

print("Spacing (m): median and 5th percentile per phase; aircraft-seconds under 20 m; closest pass")
for c in CELLS:
    for kind in ("same", "other"):
        if not any(nn[c][kind].values()): continue
        r = "  ".join("%s %.0f/%.0f" % (ph, q(v, .5), q(v, .05)) for ph, v in nn[c][kind].items() if v)
        print(f"{c} {kind:5} {r} | <20 m: {close[c][kind]} | min {mins[c][kind][0]:.0f} m ({mins[c][kind][1]})")

def state_at(o, when):
    best = None
    for tt, s in hist.get(o, []):
        if tt <= when: best = (tt, s)
        else: break
    return best

print("\nLosses (no attacker); state 0.5 s before")
for when, o in kills:
    if o not in cell_of: continue
    prev = state_at(o, when - 0.5)
    if not prev: print(f"{when:.1f} s {cell_of[o]} id {o}: no track"); continue
    _, s = prev
    near = []
    for p in hist:
        if p == o: continue
        ps = state_at(p, when - 0.5)
        if ps and not ps[1].get("gone") and abs(ps[0] - (when - 0.5)) < 2:
            near.append((dist(s, ps[1]), ps[1]["name"], cell_of[p]))
    d, who, wc = min(near) if near else (float("nan"), "-", "-")
    print(f"{when:.1f} s {cell_of[o]} {s['name']} alt {s['alt']:.0f} m AGL {s.get('agl', float('nan')):.0f} m"
          f" | WP 1 {when - WP1[cell_of[o]]:+.0f} s | nearest {d:.0f} m: {who} ({wc})")

print("\nKills with an attacker:", len(attacked))
end = [l for l in text.splitlines() if "BUL:" in l]
print("End-of-mission entries with BUL:", len(end), "| any BUL below full:",
      sorted({re.search(r"BUL:(\d+)", l).group(1) for l in end}))
