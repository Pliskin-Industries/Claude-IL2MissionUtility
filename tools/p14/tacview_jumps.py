"""Flight 2E: every abrupt step in a run's Tacview: attitude or position steps no
aircraft can fly, with the nearest other aircraft at that moment.
Usage: python tools/p14/tacview_jumps.py <run dir> (the unzipped .acmi)"""
import sys, glob, re, math

run = sys.argv[1]
acmi = glob.glob(run + "/*.acmi")[0]
head = open(acmi, encoding="utf-8").read(400)
rt = re.search(r"ReferenceTime=\S+T(\d+):(\d+):(\d+)Z", head)
off = (int(rt.group(1)) - 12) * 3600 + (int(rt.group(2)) - 30) * 60 + int(rt.group(3))

def ang(a, b):
    d = (b - a + 180) % 360 - 180
    return abs(d)

names, st, prev = {}, {}, {}
t = None
frames = []
events = []
air = set()

def dist(a, b):
    n = (b["lat"] - a["lat"]) * 111320
    e = (b["lon"] - a["lon"]) * 111320 * math.cos(math.radians(a["lat"]))
    return math.hypot(n, e, b["alt"] - a["alt"])

def check(tt):
    for o in air:
        s = st.get(o)
        if not s or "lat" not in s or s.get("gone"):
            continue
        p = prev.get(o)
        if p and p[0] < tt:
            dt = tt - p[0]
            ps = p[1]
            step = dist(ps, s)
            v = step / dt
            jumps = [k for k in ("roll", "yaw") if k in s and k in ps and ang(ps[k], s[k]) > 40]
            if (jumps or v > 300) and s.get("agl", 999) > 100:
                near = min((dist(s, st[q]), names.get(q, "?")) for q in air
                           if q != o and "lat" in st.get(q, {}) and not st[q].get("gone"))
                events.append((tt, names.get(o, o), jumps, round(v * 3.6), round(near[0], 1), near[1],
                               {k: (round(ps.get(k, 0)), round(s.get(k, 0))) for k in ("roll", "yaw")}))
        prev[o] = (tt, dict(s))

for line in open(acmi, encoding="utf-8", errors="replace"):
    line = line.strip()
    if line.startswith("#"):
        if t is not None:
            check(t)
        t = float(line[1:]) + off
        frames.append(t)
        continue
    if line.startswith("-"):
        try:
            o = line[1:]
            if o in st:
                st[o]["gone"] = True
        except ValueError:
            pass
        continue
    m = re.match(r"([0-9a-f]+),T=([^,]*)(.*)", line)
    if not m:
        continue
    o = m.group(1)
    nm = re.search(r",Name=([^,]*)", m.group(3))
    if nm:
        names[o] = nm.group(1)
    if re.search(r"Type=Air\+FixedWing", m.group(3)):
        air.add(o)
    s = st.setdefault(o, {})
    p = m.group(2).split("|")
    for i, k in enumerate(["lon", "lat", "alt", "roll", "pitch", "yaw"]):
        if i < len(p) and p[i] != "":
            s[k] = float(p[i])
    a = re.search(r"AGL=([-0-9.]+)", m.group(3))
    if a:
        s["agl"] = float(a.group(1))

gaps = [(a, b - a) for a, b in zip(frames, frames[1:]) if b - a > 0.3]
steps = sorted({round(b - a, 2) for a, b in zip(frames, frames[1:])})
print(f"frames {len(frames)} from {frames[0]:.1f} to {frames[-1]:.1f} s; step sizes {steps[:6]}; gaps > 0.3 s: {len(gaps)} {gaps[:5]}")
print(f"aircraft tracked: {len(air)}; abrupt steps above 100 m AGL: {len(events)}")
for e in events:
    print(f"  {e[0]:7.1f} s {e[1]:22} jump {e[2]} speed {e[3]} km/h, nearest {e[4]} m ({e[5]}), {e[6]}")
