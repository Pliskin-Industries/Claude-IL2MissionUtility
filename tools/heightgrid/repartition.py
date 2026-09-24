"""Re-sort an existing Land100m set after the land mask / frame bounds change, without regenerating.

Reads every probe from Land100m/*.Group and Land100m/Frame/*.Group, then rewrites each tile:
probes on the current land mask go to HG100_Trr_cc.Group, the rest to Frame/HG100_Trr_cc_frame.Group.
tiles.csv and HG100_ALL_land.Group are rebuilt. Snapped data is never touched.
"""
import os, re, csv, glob, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_land_grid import OUT, land_lattice, STEP, TILE

FRAME = os.path.join(OUT, "Frame")
os.makedirs(FRAME, exist_ok=True)
keep = land_lattice()
n = keep.shape[0]
NAME = re.compile(r'Name = "H(\d{4})_(\d{4})"')
HEAD = 'Group\r\n{{\r\n  Name = "{}";\r\n  Index = 1;\r\n  Desc = "Terrain height probe";\r\n'

def tile_of(path):
    m = re.search(r"HG100_T(\d\d)_(\d\d)", os.path.basename(path))
    return (int(m.group(1)), int(m.group(2))) if m else None

files = {}
for p in glob.glob(os.path.join(OUT, "HG100_T*.Group")) + glob.glob(os.path.join(FRAME, "HG100_T*.Group")):
    files.setdefault(tile_of(p), []).append(p)

rows, kept_total, frame_total = [], 0, 0
for (ti, tj) in sorted(files):
    blocks = []
    for p in files[(ti, tj)]:
        text = open(p, newline="").read()
        if "  Vehicle\r\n" in text:
            blocks += ["  Vehicle\r\n" + b for b in text[text.index("  Vehicle"):text.rindex("}")].split("  Vehicle\r\n")[1:]]
        os.remove(p)
    main, frame = [], []
    for b in blocks:
        m = NAME.search(b)
        (main if keep[int(m.group(1)), int(m.group(2))] else frame).append(b)
    name = f"HG100_T{ti:02d}_{tj:02d}"
    if main:
        with open(os.path.join(OUT, name + ".Group"), "w", newline="") as f:
            f.write(HEAD.format(name) + "".join(main) + "}\r\n")
        i_hi = n - 1 - ti * TILE; i_lo = max(i_hi - TILE + 1, 0); j_lo = tj * TILE; j_hi = min(j_lo + TILE - 1, n - 1)
        rows.append([name, ti, tj, len(main), i_lo * STEP, i_hi * STEP, j_lo * STEP, j_hi * STEP, ""])
    if frame:
        with open(os.path.join(FRAME, name + "_frame.Group"), "w", newline="") as f:
            f.write(HEAD.format(name + "_frame") + "".join(frame) + "}\r\n")
    kept_total += len(main); frame_total += len(frame)

with open(os.path.join(OUT, "tiles.csv"), "w", newline="") as f:
    w = csv.writer(f); w.writerow(["tile", "tile_row", "tile_col", "probes", "x_min", "x_max", "z_min", "z_max", "done"]); w.writerows(rows)
with open(os.path.join(OUT, "HG100_ALL_land.Group"), "w", newline="") as fa:
    fa.write(HEAD.format("HG100_ALL_land"))
    for r in rows:
        text = open(os.path.join(OUT, r[0] + ".Group"), newline="").read()
        fa.write(text[text.index("  Vehicle"):text.rindex("}")])
    fa.write("}\r\n")
print(f"{len(rows)} tiles, {kept_total:,} map probes, {frame_total:,} frame probes (expected land mask total {int(keep.sum()):,})")
