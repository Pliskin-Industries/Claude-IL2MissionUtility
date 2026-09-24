"""Generate T-34 height-probe .Group files on a 100 m lattice over all Korea land.

Every probe sits on the global lattice  X = i * STEP,  Z = j * STEP  and is named
"H<iiii>_<jjjj>", so any snapped file maps back to the lattice without extra data.
Probes are written per 224x224-point tile (<= 50,176 tanks, the size proven in the
editor) plus one combined file. Sea is skipped: it snaps to exactly 0 m.

usage: py -3.12 tools/heightgrid/gen_land_grid.py [out_dir]
"""
import os, sys, csv
import numpy as np

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
MASK = os.path.join(ROOT, "assets", "combined_terrain.bin")
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "References", "HeightGrid", "Land100m")

MAP = 499_200.0
STEP = 100
TILE = 224                      # lattice points per tile side
COAST_BUFFER_CELLS = 2          # keep probes within ~2 mask cells (~220 m) of land
# The terrain mask marks the map image's tan frame as land. Keep only the content inside it:
# the first/last mask rows and columns that carry any water/road/open flag.
CONTENT_X = (30_100, 468_400)
CONTENT_Z = (29_600, 469_500)

BODY = (
    "  Vehicle\n  {{\n    Name = \"H{i:04d}_{j:04d}\";\n    Index = {idx};\n    LinkTrId = 0;\n"
    "    XPos = {x}.000;\n    YPos = 0.000;\n    ZPos = {z}.000;\n    XOri = 0;\n    YOri = 0;\n    ZOri = 0;\n"
    "    Script = \"LuaScripts\\WorldObjects\\vehicles\\t34-85.txt\";\n"
    "    Model = \"graphics\\vehicles\\t34-85\\t34-85.mgm\";\n"
    "    Desc = \"\";\n    Country = 503;\n    NumberInFormation = 0;\n    Vulnerable = 1;\n    Engageable = 1;\n"
    "    LimitAmmo = 1;\n    AILevel = 2;\n    DamageReport = 50;\n    DamageThreshold = 1;\n    DeleteAfterDeath = 1;\n"
    "    CoopStart = 0;\n    Spotter = -1;\n    BeaconChannel = 0;\n    Callsign = 0;\n    PayloadId = 0;\n    ModMask = 1;\n"
    "    Fuel = 1;\n    Callnum = 0;\n    Skin = \"\";\n    BotSkin = \"\";\n    RepairTimeMultiplier = 0;\n"
    "    RehealTimeMultiplier = 0;\n    RearmTimeMultiplier = 0;\n    RefuelTimeMultiplier = 0;\n    MaintenanceRadius = 10;\n"
    "    TCode = \"\";\n    TCodeColor = \"\";\n    TrailerAtStart = 1;\n    PinToTerrain = 1;\n  }}\n\n"
)


def land_lattice():
    raw = open(MASK, "rb").read()
    assert raw[:4] == b"WMAP"
    w = int.from_bytes(raw[4:8], "little"); h = int.from_bytes(raw[8:12], "little")
    g = np.frombuffer(raw, np.uint8, offset=12).reshape(h, w)
    land = (g & 1) == 0
    # treat the frame as sea, so the coastal buffer below cannot grow it into the map
    rx = MAP - (np.arange(h) + 0.5) / h * MAP; cz = (np.arange(w) + 0.5) / w * MAP
    land &= ((rx >= CONTENT_X[0]) & (rx <= CONTENT_X[1]))[:, None] & ((cz >= CONTENT_Z[0]) & (cz <= CONTENT_Z[1]))[None, :]
    for _ in range(COAST_BUFFER_CELLS):          # grow land into the sea a little
        land = land | np.roll(land, 1, 0) | np.roll(land, -1, 0) | np.roll(land, 1, 1) | np.roll(land, -1, 1)
    n = int(MAP // STEP) + 1
    k = np.arange(n) * STEP
    gy = np.clip(((MAP - k) / MAP * h).astype(int), 0, h - 1)   # row in mask for lattice i (X, north)
    gx = np.clip((k / MAP * w).astype(int), 0, w - 1)           # col in mask for lattice j (Z, east)
    inx = (k >= CONTENT_X[0]) & (k <= CONTENT_X[1])
    inz = (k >= CONTENT_Z[0]) & (k <= CONTENT_Z[1])
    return land[gy][:, gx] & inx[:, None] & inz[None, :]         # [i, j]


def main():
    os.makedirs(OUT, exist_ok=True)
    keep = land_lattice()
    n = keep.shape[0]
    tiles_per_side = (n + TILE - 1) // TILE
    total = int(keep.sum())
    print(f"lattice {n}x{n}, land probes {total:,}, tiles per side {tiles_per_side}")

    all_path = os.path.join(OUT, "HG100_ALL_land.Group")
    idx = 2
    rows = []
    with open(all_path, "w", newline="\r\n", encoding="ascii") as fall:
        fall.write('Group\n{\n  Name = "HG100_ALL_land";\n  Index = 1;\n  Desc = "Terrain height probe";\n')
        # tiles ordered north -> south, west -> east; tile row 0 is the northern edge
        for ti in range(tiles_per_side):
            i_hi = n - 1 - ti * TILE
            i_lo = max(i_hi - TILE + 1, 0)
            for tj in range(tiles_per_side):
                j_lo = tj * TILE
                j_hi = min(j_lo + TILE - 1, n - 1)
                sub = keep[i_lo:i_hi + 1, j_lo:j_hi + 1]
                cnt = int(sub.sum())
                if cnt == 0:
                    continue
                name = f"HG100_T{ti:02d}_{tj:02d}"
                parts = [f'Group\n{{\n  Name = "{name}";\n  Index = 1;\n  Desc = "Terrain height probe";\n']
                for i in range(i_hi, i_lo - 1, -1):
                    js = np.nonzero(keep[i, j_lo:j_hi + 1])[0] + j_lo
                    for j in js:
                        parts.append(BODY.format(i=i, j=j, idx=idx, x=i * STEP, z=j * STEP))
                        idx += 1
                block = "".join(parts[1:])
                with open(os.path.join(OUT, f"{name}.Group"), "w", newline="\r\n", encoding="ascii") as ft:
                    ft.write(parts[0]); ft.write(block); ft.write("}\n")
                fall.write(block)
                rows.append([name, ti, tj, cnt, i_lo * STEP, i_hi * STEP, j_lo * STEP, j_hi * STEP])
                print(f"  {name}: {cnt:6,} probes")
        fall.write("}\n")

    with open(os.path.join(OUT, "tiles.csv"), "w", newline="") as f:
        wcsv = csv.writer(f)
        wcsv.writerow(["tile", "tile_row", "tile_col", "probes", "x_min", "x_max", "z_min", "z_max", "done"])
        for r in rows:
            wcsv.writerow(r + [""])
    print(f"{len(rows)} tile files + HG100_ALL_land.Group, {total:,} probes -> {OUT}")


if __name__ == "__main__":
    main()
