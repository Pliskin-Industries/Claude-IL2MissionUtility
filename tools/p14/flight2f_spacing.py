"""Flight 2F: spacing per flight, losses, attitude jumps and server speed, from one run folder.

Usage: python tools/p14/flight2f_spacing.py <run dir>
The run dir holds one Tacview .acmi (unzipped) and the run's missionReport
logs (copied with their modification times, cp -p). Cells are told apart by
activation time (AType:28, in START order); flights by the Tacview Name
suffix (Red 1x / Blue 2x). Sampled once a second. "Same flight" is the
nearest aircraft of the aircraft's own flight; "other flight" the nearest
aircraft of the other flight in the same cell. "<20 m" counts
aircraft-seconds. Phases: transit before WP 1, descent the next 180 s, then
station. Distances: coordinate differences first (see flight2e_spacing.py).

Attitude jumps (the server test): every Tacview step of an aircraft where its
roll, pitch or yaw changed by 60 degrees or more within 0.4 s. Jumps of one
aircraft less than 30 s apart are one episode (an aircraft tumbling after a
contact keeps jumping); each episode is reported with the nearest other
aircraft at its first jump. A contact shows as a jump at a close pass; erratic server behaviour would show
jumps with no aircraft near.

Server speed: mission seconds per wall second, from each log file's last T:
and its modification time.
Results: handoff/P14-probe-results.md, "Flight 2F".
"""
import sys, re, glob, math, os, bisect

CELLS = ["PD", "PS", "PL", "WD", "WS", "WL"]   # START order: 60, 390, 720, 1050, 1380, 1710 s
STARTS = [60, 390, 720, 1050, 1380, 1710]
TRANSIT = 264.1                                # flight 2E's measured Yak transit
GTIME = (12, 30, 0)                            # the mission's start time of day
JUMP_DEG, JUMP_S, EPISODE_S = 60.0, 0.4, 30.0

run = sys.argv[1]
acmi = glob.glob(run + "/*.acmi")[0]
logs = sorted(glob.glob(run + "/missionReport*.txt"),
              key=lambda p: int(re.search(r"\[(\d+)\]", p).group(1)))
texts = [open(p, encoding="utf-8", errors="replace").read().replace("\r", "") for p in logs]
text = "".join(texts)
secs = lambda t: (int(t) - 3) / 50

# Server speed from the log files.
pts = []
for p, tx in zip(logs, texts):
    ts = re.findall(r"^T:(\d+)", tx, re.M)
    if ts: pts.append((os.path.getmtime(p), secs(ts[-1])))
if len(pts) > 2:
    (w0, m0), (w1, m1) = pts[1], pts[-1]
    print(f"Server speed: {(m1 - m0) / (w1 - w0):.2f} mission s per wall s "
          f"(log files {len(pts)}, mission {m0:.0f} to {m1:.0f} s)")

act = {int(m.group(2)): int(m.group(1)) for m in re.finditer(r"T:(\d+) AType:28 OBJID:(\d+)", text)}
starts = sorted(set(act.values()))
assert len(starts) == len(CELLS), f"expected {len(CELLS)} activation times, got {len(starts)}"
cell_of = {o: CELLS[starts.index(t)] for o, t in act.items()}
wp1 = {c: s + TRANSIT for c, s in zip(CELLS, STARTS)}
kills = [(secs(m.group(1)), int(m.group(2))) for m in re.finditer(r"T:(\d+) AType:3 AID:-1 TID:(\d+)", text)]
attacked = re.findall(r"AType:3 AID:(?!-1)\d+", text)

head = open(acmi, encoding="utf-8", errors="replace").read(400)
rt = re.search(r"ReferenceTime=\d+-\d+-\d+T(\d+):(\d+):(\d+)", head)
off = (int(rt.group(1)) - GTIME[0]) * 3600 + (int(rt.group(2)) - GTIME[1]) * 60 + int(rt.group(3)) - GTIME[2]

st = {}                                        # object -> state
hist = {}                                      # object -> [(t, state)]
t = 0.0; last = -1
nn = {c: {f: {ph: [] for ph in ("transit", "descent", "station")} for f in ("same", "other")} for c in CELLS}
close = {c: {"same": 0, "other": 0} for c in CELLS}
mins = {c: {"same": (1e9, None), "other": (1e9, None)} for c in CELLS}

def dist(a, b):
    north = (b["lat"] - a["lat"]) * 111320.0
    east = (b["lon"] - a["lon"]) * 111320.0 * math.cos(math.radians((a["lat"] + b["lat"]) / 2))
    return math.hypot(north, east, b["alt"] - a["alt"])

def snap():
    live = [(o, s) for o, s in st.items() if "lat" in s and not s.get("gone")]
    for o, a in live:
        c = cell_of[o]
        ph = "transit" if t < wp1[c] else ("descent" if t < wp1[c] + 180 else "station")
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
    keys = ("lon", "lat", "alt", "roll", "pitch", "yaw") if len(p) >= 6 else ("lon", "lat", "alt")
    for i, k in enumerate(keys):
        if p[i] != "": s[k] = float(p[i])
    if "lat" in s:
        hist.setdefault(o, []).append((t, dict(s)))

def q(v, p):
    v = sorted(v); return v[int(p * (len(v) - 1))] if v else float("nan")

print("\nSpacing (m): median and 5th percentile per phase; aircraft-seconds under 20 m; closest pass")
for c in CELLS:
    for kind in ("same", "other"):
        if not any(nn[c][kind].values()): continue
        r = "  ".join("%s %.0f/%.0f" % (ph, q(v, .5), q(v, .05)) for ph, v in nn[c][kind].items() if v)
        print(f"{c} {kind:5} {r} | <20 m: {close[c][kind]} | min {mins[c][kind][0]:.1f} m ({mins[c][kind][1]})")

times = {o: [tt for tt, _ in h] for o, h in hist.items()}

def state_at(o, when):
    i = bisect.bisect_right(times.get(o, []), when) - 1
    return hist[o][i] if i >= 0 else None

def nearest(o, s, when):
    near = []
    for p in hist:
        if p == o: continue
        ps = state_at(p, when)
        if ps and not ps[1].get("gone") and abs(ps[0] - when) < 2:
            near.append((dist(s, ps[1]), ps[1]["name"], cell_of[p]))
    return min(near) if near else (float("nan"), "-", "-")

print("\nClosest same-flight pass at Tacview frame rate (each pair, both aircraft updated within 0.1 s)")
pairs = {}
for o in hist:
    for p in hist:
        if o < p and cell_of[o] == cell_of[p] and hist[o][0][1].get("flight") == hist[p][0][1].get("flight"):
            pairs.setdefault(cell_of[o], []).append((o, p))
for c in CELLS:
    best = (1e9, None)
    for o, p in pairs.get(c, []):
        for tt, s in hist[o]:
            ps = state_at(p, tt)
            if ps and abs(ps[0] - tt) < 0.1 and not ps[1].get("gone") and not s.get("gone"):
                d = dist(s, ps[1])
                if d < best[0]: best = (d, f"{s['name']} / {ps[1]['name']} at {tt:.1f} s, WP 1 {tt - wp1[c]:+.1f} s, alt {s['alt']:.0f} m")
    print(f"{c} {best[0]:.1f} m ({best[1]})")

print("\nLosses (no attacker); state 0.5 s before")
for when, o in kills:
    if o not in cell_of: continue
    prev = state_at(o, when - 0.5)
    if not prev: print(f"{when:.1f} s {cell_of[o]} id {o}: no track"); continue
    _, s = prev
    d, who, wc = nearest(o, s, when - 0.5)
    print(f"{when:.1f} s {cell_of[o]} {s['name']} alt {s['alt']:.0f} m AGL {s.get('agl', float('nan')):.0f} m"
          f" | WP 1 {when - wp1[cell_of[o]]:+.0f} s | nearest {d:.0f} m: {who} ({wc})")

def ang(a, b):
    return abs((b - a + 180) % 360 - 180)

print(f"\nAttitude-jump episodes (>= {JUMP_DEG:.0f} deg in {JUMP_S} s; jumps under {EPISODE_S:.0f} s apart merged)")
njump = {c: [0, 0] for c in CELLS}             # [nearest < 30 m, farther]
for o, h in hist.items():
    episode = None                             # [first t, last t, jumps, first-jump line, near]
    def report(e):
        print(e[3] + f" | {e[2]} jumps to {e[1]:.1f} s")
        njump[cell_of[o]][0 if e[4] else 1] += 1
    for i, (tt, s) in enumerate(h):
        if "roll" not in s: continue
        j = bisect.bisect_left(times[o], tt - JUMP_S)
        ps = h[j][1]
        if j >= i or "roll" not in ps: continue
        d_att = max(ang(ps["roll"], s["roll"]), ang(ps["pitch"], s["pitch"]), ang(ps["yaw"], s["yaw"]))
        if d_att < JUMP_DEG: continue
        if episode and tt - episode[1] < EPISODE_S:
            if tt - episode[1] > JUMP_S: episode[2] += 1
            episode[1] = tt
            continue
        if episode: report(episode)
        d, who, wc = nearest(o, s, tt)
        c = cell_of[o]
        episode = [tt, tt, 1, f"{tt:.1f} s {c} {s['name']} {d_att:.0f} deg | WP 1 {tt - wp1[c]:+.0f} s"
                   f" | alt {s['alt']:.0f} m | nearest {d:.1f} m: {who} ({wc})", d < 30]
    if episode: report(episode)
print("Episodes per cell, first jump nearest < 30 m / farther:", {c: tuple(v) for c, v in njump.items()})

print("\nKills with an attacker:", len(attacked))
end = [l for l in text.splitlines() if "BUL:" in l]
print("End-of-mission entries with BUL:", len(end), "| any BUL below full:",
      sorted({re.search(r"BUL:(\d+)", l).group(1) for l in end}))
