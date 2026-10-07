import json, re, sys
S, a, b = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
pat = sys.argv[4] if len(sys.argv) > 4 else r"(?i)fired|T5|Z1|\bout|start|stay|done|J\d"
for l in open(f"{S}/verify.jsonl", encoding="utf-8"):
    f = json.loads(l); m = f["s"] + 45.5
    if not (a <= m <= b):
        continue
    for i in f["items"]:
        t = i["t"]
        if re.search(pat, t) and not re.search(r"(?i)starting|before|instr|exhaust", t):
            print(f"m {int(m)//60:02d}:{m%60:04.1f}  {t!r} @{i['x']},{i['y']} c{i['c']}")
