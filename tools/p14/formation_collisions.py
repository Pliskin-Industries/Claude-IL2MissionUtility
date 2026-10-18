"""Flights 2C to 2F: for every loss with no attacker in one run: the two aircraft involved and their geometry before contact.

Usage: python tools/p14/formation_collisions.py <run dir with .acmi and missionReport logs>
The 'other' aircraft is the nearest aircraft of the same flight (same activation
time and colour) at 0.5 s before the loss. Positions are in the frame of the
lower-numbered aircraft (the leader of the two): fwd, right, up in metres; bank in degrees.
A 'crossing' is a sign change of 'right' from 20 s before to 1 s after the closest point.
Assumes the mission starts at 12:30:00 (every probe so far). Results: handoff/P14-probe-results.md,
"Flight 2F", "Every collision is a crossing in a turn".
"""
import sys, re, glob, math, bisect

run = sys.argv[1]
acmi = glob.glob(run + "/*.acmi")[0]
text = "".join(open(p, encoding="utf-8", errors="replace").read() for p in glob.glob(run + "/missionReport*.txt"))
secs = lambda t: (int(t) - 3) / 50
act = {int(m.group(2)): int(m.group(1)) for m in re.finditer(r"T:(\d+) AType:28 OBJID:(\d+)", text)}
kills = [(secs(m.group(1)), int(m.group(2))) for m in re.finditer(r"T:(\d+) AType:3 AID:-1 TID:(\d+)", text)]
head = open(acmi, encoding="utf-8", errors="replace").read(400)
rt = re.search(r"ReferenceTime=\d+-\d+-\d+T(\d+):(\d+):(\d+)", head)
off = (int(rt.group(1)) - 12) * 3600 + (int(rt.group(2)) - 30) * 60 + int(rt.group(3))

names, st, hist, t = {}, {}, {}, 0.0
for line in open(acmi, encoding="utf-8", errors="replace"):
    line = line.rstrip()
    if line.startswith("#"):
        t = float(line[1:]) + off; continue
    m = re.match(r"([0-9a-f]+),T=([^,]*)(.*)", line)
    if not m: continue
    o = int(m.group(1), 16)
    if o not in act: continue
    nm = re.search(r",Name=([^,\n]*)", m.group(3))
    if nm: names[o] = nm.group(1).strip()
    p = m.group(2).split("|"); s = st.setdefault(o, {})
    for i, k in enumerate(("lon", "lat", "alt", "roll", "pitch", "yaw")[:len(p)]):
        if p[i] != "": s[k] = float(p[i])
    if "yaw" in s: hist.setdefault(o, []).append((t, dict(s)))
times = {o: [x for x, _ in h] for o, h in hist.items()}

def at(o, when):
    i = bisect.bisect_right(times.get(o, []), when) - 1
    return hist[o][i][1] if i >= 0 else None

def colour(o): return "Red" if " Red " in names.get(o, "") + " " else "Blue"
def num(o):
    m = re.search(r"(\d+)\s*$", names.get(o, "")); return int(m.group(1)) if m else 0

def rel(a, b):
    n = (b["lat"] - a["lat"]) * 111320.0
    e = (b["lon"] - a["lon"]) * 111320.0 * math.cos(math.radians(a["lat"]))
    h = math.radians(a["yaw"])
    return n * math.cos(h) + e * math.sin(h), e * math.cos(h) - n * math.sin(h), b["alt"] - a["alt"], math.hypot(n, e, b["alt"] - a["alt"])

starts = sorted(set(act.values()))
seen = set()
for when, o in kills:
    if o not in hist or o in seen: continue
    s0 = at(o, when - 0.5)
    mates = [p for p in hist if p != o and act[p] == act[o] and colour(p) == colour(o) and at(p, when - 0.5)]
    if not mates or not s0: print(f"{when:.1f} s {names.get(o)}: no mate"); continue
    p = min(mates, key=lambda q: rel(s0, at(q, when - 0.5))[3])
    a, b = sorted([o, p], key=num)
    # contact: closest point in the 30 s before the loss
    best = min(((rel(at(a, x), at(b, x))[3], x) for x in times[a] if when - 30 <= x <= when and at(b, x)), default=(None, None))
    d, tc = best
    seen.update([o])
    if d is None or d > 30:
        print(f"{when:.1f} s lost {names.get(o)}: no same-flight aircraft within 30 m in the 30 s before"); continue
    line = [f"{when:.1f} s lost {names.get(o)} | cell start {starts.index(act[o]) + 1} | pair {names.get(a)} (lead) / {names.get(b)} | closest {d:.1f} m at {tc:.1f} s"]
    prev = None; cross = []
    for x in [x for x in times[a] if tc - 20 <= x <= tc + 1]:
        sa, sb = at(a, x), at(b, x)
        if not sb: continue
        f, r, u, dd = rel(sa, sb)
        if prev is not None and (prev > 0) != (r > 0): cross.append(f"{x - tc:+.1f} s (fwd {f:+.0f} up {u:+.0f})")
        prev = r
    for dt in (-15, -8, -4, -1, 0):
        sa, sb = at(a, tc + dt), at(b, tc + dt)
        f, r, u, dd = rel(sa, sb)
        line.append(f"   t{dt:+d} s: {names.get(b)} fwd {f:+.0f} right {r:+.0f} up {u:+.0f} | banks lead {sa['roll']:+.0f} wing {sb['roll']:+.0f}")
    line.append("   crossings of the lead's track from 20 s before to 1 s after the closest point: " + (", ".join(cross) if cross else "none"))
    print("\n".join(line))
