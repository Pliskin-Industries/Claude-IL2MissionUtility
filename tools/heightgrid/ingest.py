"""Merge snapped editor files into the Korea height store and redraw the coverage map.

Probe tanks (named H<iiii>_<jjjj>, or the earlier HG_/HGA_/HGB_ test grids) land on the
100 m lattice. With --learn, other ground objects in the file (Vehicle / Block / Ground /
Train with PinToTerrain = 1 and a non-zero Y) are kept as extra measured points: use it
only on missions you have run "set to ground" on.

usage:
  py -3.12 tools/heightgrid/ingest.py <snapped.Group|.Mission> ... [--learn]
  py -3.12 tools/heightgrid/ingest.py            (just redraw the coverage map)
"""
import os, sys, re, csv
import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
HG = os.path.join(ROOT, "References", "HeightGrid")
STORE = os.path.join(HG, "store")
LATTICE = os.path.join(STORE, "heights_100m.npy")      # float32 [i, j], X = 100 i, Z = 100 j, NaN = unknown
LEARNED = os.path.join(STORE, "learned_points.csv")     # x, z, y, source
TILES = os.path.join(HG, "Land100m", "tiles.csv")
MAP = 499_200.0; STEP = 100; N = int(MAP // STEP) + 1

BLOCK = re.compile(r"(\w+)\s*\{([^{}]*)\}")
def num(body, key):
    m = re.search(r"\b" + key + r"\s*=\s*(-?[\d.]+)\s*;", body)
    return float(m.group(1)) if m else None


def load_store():
    os.makedirs(STORE, exist_ok=True)
    if os.path.exists(LATTICE):
        return np.load(LATTICE)
    return np.full((N, N), np.nan, np.float32)


def ingest(path, lat, learn, learned):
    txt = open(path, encoding="utf-8", errors="replace").read()
    n_lat = n_learn = n_zero = 0
    for m in BLOCK.finditer(txt):
        kind, body = m.group(1), m.group(2)
        x, y, z = num(body, "XPos"), num(body, "YPos"), num(body, "ZPos")
        if x is None or y is None or z is None:
            continue
        name = re.search(r'Name\s*=\s*"([^"]*)"', body)
        name = name.group(1) if name else ""
        probe = re.fullmatch(r"H(\d{4})_(\d{4})", name) or re.fullmatch(r"(HG|HGA|HGB)_(\d+)_(\d+)", name)
        if probe:
            if abs(x % STEP) < 0.5 and abs(z % STEP) < 0.5:
                lat[int(round(x / STEP)), int(round(z / STEP))] = y
                n_lat += 1
            else:
                learned.append((x, z, y, os.path.basename(path)))
                n_learn += 1
            n_zero += y == 0.0
        elif learn and kind in ("Vehicle", "Block", "Ground", "Train") and "PinToTerrain = 1" in body and y != 0.0:
            learned.append((x, z, y, os.path.basename(path)))
            n_learn += 1
    print(f"{os.path.basename(path)}: {n_lat:,} lattice probes, {n_learn:,} extra points, {n_zero:,} at 0 m (sea or unsnapped)")


def update_tiles(lat):
    if not os.path.exists(TILES):
        return []
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from gen_land_grid import land_lattice
    probe = land_lattice()                              # the same land mask the tiles were cut from
    rows = list(csv.DictReader(open(TILES)))
    for r in rows:
        i0, i1 = int(r["x_min"]) // STEP, int(r["x_max"]) // STEP
        j0, j1 = int(r["z_min"]) // STEP, int(r["z_max"]) // STEP
        known = (np.isfinite(lat[i0:i1 + 1, j0:j1 + 1]) & probe[i0:i1 + 1, j0:j1 + 1]).sum()
        r["done"] = "yes" if known >= 0.98 * int(r["probes"]) else (f"{100 * known / int(r['probes']):.0f}%" if known else "")
    with open(TILES, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(rows)
    return rows


def coverage_png(lat, rows):
    base = Image.open(os.path.join(ROOT, "assets", "DD052_en_map_01_LowQ.jpg")).convert("RGB")
    W = H = 1248
    base = base.resize((W, H))
    img = np.asarray(base).astype(float) * 0.55 + 255 * 0.45
    # downsample the lattice (every 4th point = 400 m) for display
    sub = lat[::4, ::4]
    sub = sub[::-1]                                   # row 0 = north
    ys = (np.arange(H) * sub.shape[0] / H).astype(int); xs = (np.arange(W) * sub.shape[1] / W).astype(int)
    e = sub[ys][:, xs]
    known = np.isfinite(e)
    stops = np.array([0, 150, 300, 450, 650, 850, 1150, 1800], float)
    cols = np.array([[40, 60, 200], [30, 150, 190], [60, 170, 90], [220, 210, 80], [235, 140, 50], [205, 50, 40], [250, 245, 240], [255, 255, 255]], float)
    rgb = np.stack([np.interp(np.nan_to_num(e), stops, cols[:, k]) for k in range(3)], -1)
    img[known] = rgb[known] * 0.8 + img[known] * 0.2
    out = Image.fromarray(img.astype(np.uint8)); d = ImageDraw.Draw(out)
    try: font = ImageFont.truetype("arial.ttf", 11)
    except Exception: font = ImageFont.load_default()
    for r in rows:
        x0 = int(r["z_min"]) / MAP * W; x1 = (int(r["z_max"]) + STEP) / MAP * W
        y0 = (MAP - int(r["x_max"]) - STEP) / MAP * H; y1 = (MAP - int(r["x_min"])) / MAP * H
        colr = (0, 140, 0) if r["done"] == "yes" else ((200, 120, 0) if r["done"] else (90, 90, 90))
        d.rectangle([x0, y0, x1, y1], outline=colr)
        d.text((x0 + 3, y0 + 2), f"{int(r['tile_row']):02d}_{int(r['tile_col']):02d}", fill=colr, font=font)
    path = os.path.join(HG, "coverage.png"); out.save(path)
    done = sum(r["done"] == "yes" for r in rows)
    print(f"known lattice points {np.isfinite(lat).sum():,}; tiles done {done}/{len(rows)}; map -> {path}")


def main():
    args = [a for a in sys.argv[1:] if a != "--learn"]
    learn = "--learn" in sys.argv
    lat = load_store()
    learned = []
    for p in args:
        ingest(p, lat, learn, learned)
    if args:
        np.save(LATTICE, lat)
        new = not os.path.exists(LEARNED)
        with open(LEARNED, "a", newline="") as f:
            w = csv.writer(f)
            if new: w.writerow(["x", "z", "y", "source"])
            w.writerows(learned)
    coverage_png(lat, update_tiles(lat))


if __name__ == "__main__":
    main()
