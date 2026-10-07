import json, re, sys
S = sys.argv[1]; OFF = 45.5
files = sys.argv[2].split(",")
seen = set()
rows = []
for p in files:
    for l in open(f"{S}/{p}", encoding="utf-8"):
        f = json.loads(l)
        for i in f["items"]:
            t = i["t"]
            if re.search(r"(?i)fire|start|hit|\bout|reach|T-[a-z]|done|object|marker|note|stay|within|mission|K14|J[1-5]|E[1-9]|F[12]", t) \
               and not re.search(r"(?i)starting|startin|before|exhaust|ready|fire[^d]|to ?fir|press", t):
                key = (round(f["s"], 1), t)
                if key in seen: continue
                seen.add(key)
                rows.append((f["s"], t, i["x"], i["y"]))
rows.sort()
mm = lambda s: f"{int(s)//60:02d}:{s%60:04.1f}"
for s, t, x, y in rows:
    print(f"v {mm(s)}  m {mm(s + OFF)}  {t}  @{x},{y}")
