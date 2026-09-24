# Korea terrain height grid

The Mission Editor's heightmap is packed in the game archives, so we measure it. We place
T-34 probes on a 100 m lattice over all land, snap them with **set to ground**, and read the
heights back. Sea is skipped because it snaps to exactly 0 m.

Measured on the roughest 3 × 3 km (136–994 m): a 100 m lattice with a +20 m margin kept
every point at or above ground (mean error 5 m, 95% within 14 m).

## Files (`References/HeightGrid/`, git-ignored)

| Path | What |
|---|---|
| `Land100m/HG100_Trr_cc.Group` | One tile (22.4 × 22.4 km, ≤ 50,176 probes). `rr` counts north → south, `cc` west → east. |
| `Land100m/HG100_ALL_land.Group` | Every probe in one file. |
| `Land100m/Frame/` | Probes in the map's outer frame (X < 30.1 km or > 468.4 km, Z < 29.6 km or > 469.5 km). The terrain mask marks the frame as land, but only 34 of 34,571 game objects are there. Snap these only if a mission ever needs the frame. |
| `Land100m/tiles.csv` | Tile bounds, probe count, and a `done` column that `ingest.py` fills in. |
| `Snapped/` | Put snapped tiles here (any name works). |
| `store/heights_100m.npy` | The merged lattice: float32 `[i, j]` with X = 100·i and Z = 100·j; NaN means not measured yet. |
| `store/learned_points.csv` | Extra measured points from snapped missions (`--learn`). |
| `coverage.png` | Progress map: tiles outlined grey (to do), orange (partial), green (done). |

Probes are named `H<iiii>_<jjjj>`, which is their lattice position, so a snapped file needs
nothing else to be read back.

## One round

1. In an empty Korea mission, import a tile from `Land100m/`.
2. Select all → **set to ground**.
3. Save the group into `Snapped/`.
4. `py -3.12 tools/heightgrid/ingest.py References/HeightGrid/Snapped/<file>.Group`

## Learning from missions

After you snap a real mission, run
`py -3.12 tools/heightgrid/ingest.py <mission> --learn`. It keeps the snapped ground
objects (Vehicle / Block / Ground / Train with `PinToTerrain = 1` and a non-zero Y) as
measured points. Use `--learn` only on missions you have actually snapped.

## Regenerate

`py -3.12 tools/heightgrid/gen_land_grid.py` rebuilds `Land100m/` from
`assets/combined_terrain.bin`, keeping land inside the map frame plus a ~220 m coastal
buffer. A full run writes about 30 GB (tiles, the combined file and Frame/) and took several hours on this machine; `repartition.py` re-sorts an existing set in minutes.
