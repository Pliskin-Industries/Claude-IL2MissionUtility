"""Compare logged firings (replay timeline) with subtitles OCR'd from the video."""
import json, re, sys
from difflib import SequenceMatcher

S = sys.argv[1]
OFFSET = float(sys.argv[2]) if len(sys.argv) > 2 else 45.5   # mission = video + OFFSET
OCR_FILES = sys.argv[3].split(",")

# --- vocabulary (subtitle texts) -------------------------------------------
vocab = {}
for line in open(f"{S}/p14/to.eng", encoding="utf-8-sig"):
    if ":" in line:
        k, v = line.rstrip("\n").split(":", 1)
        if v.strip() and v.strip() not in ("T-a1", "T-z"):
            vocab[v.strip()] = int(k)
norm = lambda t: re.sub(r"[^a-z0-9+]", "", t.lower())
NV = {v: norm(v) for v in vocab}

def best(text):
    n = norm(text)
    if len(n) < 4:
        return None, 0.0, 0.0
    sc = sorted(((SequenceMatcher(None, n, nv).ratio(), v) for v, nv in NV.items()), reverse=True)
    return sc[0][1], sc[0][0], sc[0][0] - sc[1][0]

# --- source breadcrumb -> subtitle it drives --------------------------------
SUB = {"F1": "F1 hit", "READ F1 reached 12": "F1 reached 12", "READ F1 reached 29": "F1 reached 29",
       "F2 OUT 1": "F2 out 1", "F2 OUT 2": "F2 out 2", "F2 OUT 3": "F2 out 3", "F2 OUT 4": "F2 out 4",
       "T-t T1": "T-t T1 fired", "T-t T2": "T-t T2 fired", "T-t T3a": "T-t T3a fired",
       "T-t T3b": "T-t T3b fired", "T-t T7 SAME TICK": "T-t T7 fired", "J4": "J4 out", "J5": "J5 out",
       "ZA1": "ZA1 fired", "E1": "E1 fired", "E2": "E2 fired", "READ E2 fired 2+": "E2 fired 2+",
       "E3": "E3 fired", "E6": "E6 fired", "READ E6 fired 2+": "E6 fired 2+", "E7": "E7 fired",
       "E8": "E8 fired", "E9": "E9 fired",
       "CUE 01": "T-o O1 fired", "CUE 02": "T-o O2 fired", "CUE 03": "T-o O3 fired",
       "CUE 04": "T-o O4 fired", "CUE 05": "T-o O5 fired",
       "CUE 06": "Flight 0: stay within 20 km of K14 until 11:00", "CUE 07": "T-e start",
       "CUE 08": "T-f F1 start"}
expected = []
for line in open(f"{S}/p14/P14-probe-replay.md", encoding="utf-8-sig"):
    m = re.match(r"- P14_Probe_0 / (?:[^/]+ / )?(.+?) \(MCU_\w+ #\d+\): \d+ observed firing\(s\), at (.+) s\.", line)
    if m and m.group(1) in SUB:
        for t in m.group(2).split(","):
            expected.append((float(t), m.group(1), SUB[m.group(1)]))
expected.sort()

# --- OCR hits ----------------------------------------------------------------
hits = []
for p in OCR_FILES:
    for line in open(p, encoding="utf-8"):
        fr = json.loads(line)
        for it in fr["items"]:
            if re.search(r"(?i)load|fov|bloc|usa|test|starting|restart|startin", it["t"]):
                continue
            lab, sc, mg = best(it["t"])
            if lab and len(norm(it["t"])) < 0.6 * len(NV[lab]):
                continue
            if lab and sc >= 0.72 and mg >= 0.03:
                hits.append({"v": fr["s"], "m": fr["s"] + OFFSET, "label": lab, "raw": it["t"], "sc": round(sc, 2)})
hits.sort(key=lambda h: h["v"])

def mm(s): return f"{int(s)//60:02d}:{s%60:05.2f}"
used = set()
rows = []
for t, src, sub in expected:
    win = [i for i, h in enumerate(hits) if t - 0.6 <= h["m"] <= t + (20.5 if src.startswith("CUE") else 2.5)]
    labels = sorted({hits[i]["label"] for i in win})
    used.update(win)
    rows.append({"mission_s": t, "video": mm(t - OFFSET), "source": src, "subtitle": sub,
                 "seen": labels, "match": sub in labels})
extra = [h for i, h in enumerate(hits) if i not in used]

out = {"offset_s": OFFSET, "rows": rows, "unexplained": extra}
json.dump(out, open(f"{S}/compare.json", "w", encoding="utf-8"), ensure_ascii=False, indent=1)
for r in rows:
    if r["mission_s"] - OFFSET < 0:
        continue
    flag = "SEEN " if r["match"] else ("OTHER" if r["seen"] else "-    ")
    print(f"m {mm(r['mission_s'])}  v {r['video']}  {flag} {r['source']:<18} {r['subtitle'][:40]:<40} {r['seen']}")
print("\nunexplained OCR subtitle hits:")
for h in extra:
    print(f"v {mm(h['v'])}  m {mm(h['m'])}  {h['label']:<40} raw={h['raw']!r} {h['sc']}")

