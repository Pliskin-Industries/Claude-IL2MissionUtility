//! heightprobe.rs — terrain height probes: tile files out, snapped files in
//!
//! Builds the T-34 probe groups that measure the editor's terrain (one probe
//! per land node of the 100 m lattice, one `.Group` per 224 × 224-node tile)
//! and reads snapped files back into a `terrain::HeightStore`. Probes are
//! named `H<iiii>_<jjjj>` after their lattice node, so a snapped file needs
//! nothing else. The land test matches `tools/heightgrid/gen_land_grid.py`:
//! dry cells of `combined_terrain.bin` inside the map frame, grown 2 cells
//! into the sea. Sea is never probed; it snaps to exactly 0 m.
//!
//! A land probe still at Y = 0 was not snapped: it is reported and ignored,
//! never stored as sea level.
//!
//! ## Public API
//! * `CONTENT_X` / `CONTENT_Z` — map area inside the image frame
//! * `tile_probe_nodes`, `tile_probe_count`, `probe_tiles`
//! * `tile_name`, `probe_name`, `parse_probe_name`
//! * `probe_group`, `tile_probe_group` — build probe `.Group` trees
//! * `ingest` / `IngestReport` — merge a snapped tree into a store
//!
//! ## Used by
//! * tests only for now (the Map-tab Terrain panel wires it in later)

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::sync::OnceLock;

use crate::ast::Il2Entity;
use crate::geo::MAP_MAX;
use crate::parser::parse_il2_document;
use crate::terrain::{tile_node_range, tile_of_node, HeightStore, LATTICE_N, STEP_M, TILES_PER_SIDE};
use crate::watermap::TerrainMap;

/// Map content inside the image frame (the mask marks the frame as land).
pub const CONTENT_X: (f64, f64) = (30_100.0, 468_400.0);
pub const CONTENT_Z: (f64, f64) = (29_600.0, 469_500.0);
/// Probes reach this many mask cells (~110 m each) past the coastline.
const COAST_BUFFER_CELLS: usize = 2;
/// Catalog vehicle used as the probe (the editor only snaps units).
const PROBE_SCRIPT: &str = "t34-85.txt";
/// Ground objects whose snapped Y is kept by `ingest(.., learn = true)`.
const LEARN_TYPES: [&str; 4] = ["Vehicle", "Train", "Block", "Ground"];

fn in_content(x: f64, z: f64) -> bool {
    (CONTENT_X.0..=CONTENT_X.1).contains(&x) && (CONTENT_Z.0..=CONTENT_Z.1).contains(&z)
}

/// Mask cells that get probes: dry, inside the frame, plus the coast buffer.
fn probe_cells() -> Option<&'static (Vec<bool>, usize, usize)> {
    static CELLS: OnceLock<Option<(Vec<bool>, usize, usize)>> = OnceLock::new();
    CELLS
        .get_or_init(|| {
            let map = TerrainMap::builtin().ok()?;
            let (w, h) = (map.width as usize, map.height as usize);
            let mut land = vec![false; w * h];
            for y in 0..h {
                let x_world = MAP_MAX - (y as f64 + 0.5) / h as f64 * MAP_MAX;
                for x in 0..w {
                    let z_world = (x as f64 + 0.5) / w as f64 * MAP_MAX;
                    land[y * w + x] = in_content(x_world, z_world) && !map.is_water_cell(x as u32, y as u32);
                }
            }
            for _ in 0..COAST_BUFFER_CELLS {
                let prev = land.clone();
                for y in 0..h {
                    for x in 0..w {
                        let k = y * w + x;
                        if !prev[k]
                            && ((y > 0 && prev[k - w])
                                || (y + 1 < h && prev[k + w])
                                || (x > 0 && prev[k - 1])
                                || (x + 1 < w && prev[k + 1]))
                        {
                            land[k] = true;
                        }
                    }
                }
            }
            Some((land, w, h))
        })
        .as_ref()
}

fn keep_node(i: usize, j: usize) -> bool {
    let Some((cells, w, h)) = probe_cells() else { return false };
    let (xi, zj) = (i as f64 * STEP_M, j as f64 * STEP_M);
    if !in_content(xi, zj) {
        return false;
    }
    let gy = (((MAP_MAX - xi) / MAP_MAX * *h as f64) as usize).min(h - 1);
    let gx = ((zj / MAP_MAX * *w as f64) as usize).min(w - 1);
    cells[gy * w + gx]
}

/// Probe nodes of tile (ti, tj), north → south then west → east.
pub fn tile_probe_nodes(ti: usize, tj: usize) -> Vec<(usize, usize)> {
    let (i_lo, i_hi, j_lo, j_hi) = tile_node_range(ti, tj);
    (i_lo..=i_hi)
        .rev()
        .flat_map(|i| (j_lo..=j_hi).map(move |j| (i, j)))
        .filter(|&(i, j)| keep_node(i, j))
        .collect()
}

pub fn tile_probe_count(ti: usize, tj: usize) -> usize {
    let (i_lo, i_hi, j_lo, j_hi) = tile_node_range(ti, tj);
    (i_lo..=i_hi).map(|i| (j_lo..=j_hi).filter(|&j| keep_node(i, j)).count()).sum()
}

/// Every tile with at least one probe: (row, column, probe count).
pub fn probe_tiles() -> Vec<(usize, usize, usize)> {
    (0..TILES_PER_SIDE)
        .flat_map(|ti| (0..TILES_PER_SIDE).map(move |tj| (ti, tj)))
        .map(|(ti, tj)| (ti, tj, tile_probe_count(ti, tj)))
        .filter(|&(_, _, n)| n > 0)
        .collect()
}

pub fn tile_name(ti: usize, tj: usize) -> String {
    format!("HG100_T{ti:02}_{tj:02}")
}

pub fn probe_name(i: usize, j: usize) -> String {
    format!("H{i:04}_{j:04}")
}

/// `H<iiii>_<jjjj>` → lattice node.
pub fn parse_probe_name(name: &str) -> Option<(usize, usize)> {
    let b = name.as_bytes();
    if b.len() != 10 || b[0] != b'H' || b[5] != b'_' {
        return None;
    }
    let digits = |s: &str| s.bytes().all(|c| c.is_ascii_digit()).then(|| s.parse().ok()).flatten();
    let (i, j): (usize, usize) = (digits(&name[1..5])?, digits(&name[6..10])?);
    (i < LATTICE_N && j < LATTICE_N).then_some((i, j))
}

/// Earlier test grids (`HG_rrr_ccc`, `HGA_…`, `HGB_…`) — read by position.
fn is_legacy_probe(name: &str) -> bool {
    ["HG_", "HGA_", "HGB_"].iter().any(|p| name.starts_with(p))
}

/// The catalog T-34-85 vehicle block, parsed once from `assets/Models.Group`.
fn probe_prototype() -> Result<&'static Il2Entity, String> {
    static PROTO: OnceLock<Result<Il2Entity, String>> = OnceLock::new();
    PROTO
        .get_or_init(|| {
            let root = parse_il2_document(include_str!("../assets/Models.Group"))?;
            let mut found = None;
            root.for_each(&mut |e| {
                if found.is_none()
                    && e.block_type == "Vehicle"
                    && e.property("Script").is_some_and(|s| s.contains(PROBE_SCRIPT))
                {
                    found = Some(e.clone());
                }
            });
            found.ok_or_else(|| format!("Models.Group has no {PROBE_SCRIPT} vehicle"))
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// A `Group` of probe tanks at Y = 0 on the given lattice nodes.
pub fn probe_group(name: &str, nodes: &[(usize, usize)]) -> Result<Il2Entity, String> {
    let proto = probe_prototype()?;
    let mut group = Il2Entity::new("Group");
    group.set_name(name);
    group.index = Some(1);
    group.set_property("Index", "1");
    group.set_property("Desc", "\"Terrain height probe - select all, set to ground, save\"");
    for (k, &(i, j)) in nodes.iter().enumerate() {
        let mut v = proto.clone();
        v.set_name(&probe_name(i, j));
        let index = k as i32 + 2;
        v.index = Some(index);
        v.set_property("Index", index.to_string());
        v.set_property("LinkTrId", "0");
        v.set_property("XPos", format!("{:.3}", i as f64 * STEP_M));
        v.set_property("YPos", "0.000");
        v.set_property("ZPos", format!("{:.3}", j as f64 * STEP_M));
        for key in ["XOri", "YOri", "ZOri"] {
            v.set_property(key, "0");
        }
        v.set_existing_property("PinToTerrain", "1");
        group.children.push(v);
    }
    Ok(group)
}

/// Probe group for one tile, or `None` if the tile is all sea / frame.
pub fn tile_probe_group(ti: usize, tj: usize) -> Result<Option<Il2Entity>, String> {
    let nodes = tile_probe_nodes(ti, tj);
    if nodes.is_empty() {
        return Ok(None);
    }
    probe_group(&tile_name(ti, tj), &nodes).map(Some)
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IngestReport {
    /// Probe heights written to lattice nodes.
    pub lattice: usize,
    /// Off-lattice probe heights kept as measured points.
    pub probe_points: usize,
    /// Snapped mission objects kept as measured points (`learn`).
    pub learned: usize,
    /// Probes at exactly 0 m over sea (kept: sea level).
    pub sea: usize,
    /// Probes at exactly 0 m on land: not snapped, ignored.
    pub unsnapped: usize,
    /// Tiles that received lattice heights.
    pub tiles: BTreeSet<(usize, usize)>,
}

impl IngestReport {
    /// More than half the probes still at 0 m on land: the file was saved
    /// without running set to ground.
    pub fn looks_unsnapped(&self) -> bool {
        self.unsnapped > 0 && self.unsnapped * 2 > self.lattice + self.probe_points + self.unsnapped
    }
}

fn num(e: &Il2Entity, key: &str) -> Option<f64> {
    e.property(key)?.trim().parse().ok()
}

/// Merge every probe in `root` into `store`. With `learn`, snapped ground
/// objects (Vehicle / Train / Block / Ground with `PinToTerrain = 1` and a
/// non-zero Y) are kept as measured points too — only use it on files the
/// user has run set to ground on.
pub fn ingest(root: &Il2Entity, store: &mut HeightStore, learn: bool) -> IngestReport {
    let water = TerrainMap::builtin().ok();
    let mut rep = IngestReport::default();
    root.for_each(&mut |e| {
        let (Some(x), Some(y), Some(z)) = (num(e, "XPos"), num(e, "YPos"), num(e, "ZPos")) else { return };
        let name = e.name().unwrap_or("");
        let lattice = parse_probe_name(name);
        if lattice.is_none() && !is_legacy_probe(name) {
            let pinned = e.property("PinToTerrain").map(str::trim) == Some("1");
            if learn && pinned && y != 0.0 && LEARN_TYPES.contains(&e.block_type.as_str()) {
                store.add_point(x, z, y);
                rep.learned += 1;
            }
            return;
        }
        if y == 0.0 {
            if water.is_some_and(|w| w.is_water_xz(x, z)) {
                rep.sea += 1;
            } else {
                rep.unsnapped += 1;
                return;
            }
        }
        let (fi, fj) = (x / STEP_M, z / STEP_M);
        let on_node = (fi - fi.round()).abs() < 0.005 && (fj - fj.round()).abs() < 0.005;
        if on_node && fi >= 0.0 && fj >= 0.0 && (fi.round() as usize) < LATTICE_N && (fj.round() as usize) < LATTICE_N {
            let (i, j) = (fi.round() as usize, fj.round() as usize);
            store.set_node(i, j, y);
            rep.tiles.insert(tile_of_node(i, j));
            rep.lattice += 1;
        } else {
            store.add_point(x, z, y);
            rep.probe_points += 1;
        }
    });
    rep
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_group_file;
    use crate::serialize::serialize_group;
    use crate::terrain::KOREA_MAP_ID;

    /// Counts from the Python generator's tiles.csv (same mask and bounds).
    #[test]
    fn tile_counts_match_python_generator() {
        for (ti, tj, n) in [
            (5, 16, 18_106),
            (11, 4, 3),
            (20, 1, 16),
            (10, 10, 50_176),
            (1, 1, 21_280),
            (12, 18, 12_508),
        ] {
            assert_eq!(tile_probe_count(ti, tj), n, "tile {ti}_{tj}");
        }
        assert_eq!(tile_probe_count(0, 0), 0, "frame corner has no probes");
    }

    #[test]
    fn all_tiles_match_python_generator() {
        let tiles = probe_tiles();
        assert_eq!(tiles.len(), 310);
        assert_eq!(tiles.iter().map(|t| t.2).sum::<usize>(), 12_443_401);
    }

    #[test]
    fn probe_names_round_trip() {
        assert_eq!(probe_name(3905, 3628), "H3905_3628");
        assert_eq!(parse_probe_name("H3905_3628"), Some((3905, 3628)));
        for bad in ["H3905-3628", "H390_36280", "X3905_3628", "H9999_0001", "H12a4_0001", "HG_000_001"] {
            assert_eq!(parse_probe_name(bad), None, "{bad}");
        }
        assert_eq!(tile_name(5, 16), "HG100_T05_16");
    }

    #[test]
    fn probe_group_is_catalog_t34_on_the_lattice() {
        let g = tile_probe_group(11, 4).unwrap().unwrap();
        assert_eq!(g.name(), Some("HG100_T11_04"));
        assert_eq!(g.children.len(), 3);
        let v = &g.children[0];
        assert_eq!(v.block_type, "Vehicle");
        assert_eq!(v.name(), Some("H2305_1098"));
        assert_eq!(v.property("XPos"), Some("230500.000"));
        assert_eq!(v.property("ZPos"), Some("109800.000"));
        assert_eq!(v.property("YPos"), Some("0.000"));
        assert_eq!(v.property("LinkTrId"), Some("0"));
        assert_eq!(v.property("PinToTerrain"), Some("1"));
        assert!(v.property("Script").unwrap().contains("t34-85.txt"));
        assert_eq!(v.index, Some(2));
        assert!(tile_probe_group(0, 0).unwrap().is_none());
    }

    #[test]
    fn probe_file_round_trips_through_parser() {
        let g = tile_probe_group(11, 4).unwrap().unwrap();
        let text = serialize_group(&g);
        let back = parse_group_file(&text).unwrap();
        assert_eq!(back, g);
        assert_eq!(serialize_group(&back), text);
    }

    fn snapped(y_by_name: &[(&str, &str)], extra: &str) -> Il2Entity {
        let nodes = [(3905, 3628), (3905, 3629), (3906, 3628)];
        let mut text = serialize_group(&probe_group("T", &nodes).unwrap());
        for (name, y) in y_by_name {
            let at = text.find(&format!("\"{name}\"")).unwrap();
            let yline = at + text[at..].find("YPos = 0.000").unwrap();
            text.replace_range(yline..yline + 12, &format!("YPos = {y}"));
        }
        let close = text.rfind('}').unwrap();
        text.insert_str(close, extra);
        parse_group_file(&text).unwrap()
    }

    #[test]
    fn ingest_stores_snapped_probes_and_skips_unsnapped() {
        let root = snapped(&[("H3905_3628", "412.250"), ("H3905_3629", "398.100")], "");
        let mut store = HeightStore::new(KOREA_MAP_ID);
        let rep = ingest(&root, &mut store, false);
        assert_eq!(rep.lattice, 2);
        assert_eq!(rep.unsnapped, 1, "H3906_3628 still at 0 m on land");
        assert!(rep.tiles.contains(&tile_of_node(3905, 3628)));
        assert_eq!(store.node(3905, 3628), Some(412.3));
        assert_eq!(store.node(3905, 3629), Some(398.1));
        assert_eq!(store.node(3906, 3628), None);
        assert!(!rep.looks_unsnapped());
    }

    #[test]
    fn ingest_flags_a_file_saved_without_snapping() {
        let root = snapped(&[], "");
        let rep = ingest(&root, &mut HeightStore::new(KOREA_MAP_ID), false);
        assert_eq!(rep.unsnapped, 3);
        assert!(rep.looks_unsnapped());
    }

    #[test]
    fn ingest_learns_snapped_units_and_legacy_points() {
        let extra = "  Vehicle\r\n  {\r\n    Name = \"Tank\";\r\n    XPos = 390123.500;\r\n    YPos = 211.700;\r\n    ZPos = 362456.250;\r\n    PinToTerrain = 1;\r\n  }\r\n\r\n  \r\n\
                     \x20 Vehicle\r\n  {\r\n    Name = \"HGB_001_001\";\r\n    XPos = 393550.000;\r\n    YPos = 640.500;\r\n    ZPos = 375250.000;\r\n  }\r\n\r\n  \r\n\
                     \x20 Plane\r\n  {\r\n    Name = \"Flying\";\r\n    XPos = 390000.000;\r\n    YPos = 1500.000;\r\n    ZPos = 360000.000;\r\n    PinToTerrain = 1;\r\n  }\r\n\r\n  \r\n";
        let root = snapped(&[("H3905_3628", "412.000")], extra);
        let mut store = HeightStore::new(KOREA_MAP_ID);
        let off = ingest(&root, &mut store, false);
        assert_eq!((off.learned, off.probe_points), (0, 1));
        let mut store = HeightStore::new(KOREA_MAP_ID);
        let on = ingest(&root, &mut store, true);
        assert_eq!((on.learned, on.probe_points, on.lattice), (1, 1, 1));
        assert_eq!(store.height_at(390_123.5, 362_456.25), Some(211.7));
        assert_eq!(store.points().len(), 2, "planes are never learned");
    }
}

#[cfg(test)]
mod real_files {
    use super::*;
    use crate::terrain::KOREA_MAP_ID;

    /// `IL2_SNAPPED="a.Group;b.Group" cargo test --offline real_snapped -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_snapped_files_ingest() {
        let list = std::env::var("IL2_SNAPPED").expect("set IL2_SNAPPED");
        let mut store = HeightStore::new(KOREA_MAP_ID);
        for path in list.split(';') {
            let text = std::fs::read_to_string(path).unwrap();
            let root = parse_il2_document(&text).unwrap();
            let rep = ingest(&root, &mut store, false);
            println!(
                "{path}: lattice {} points {} sea {} unsnapped {} tiles {}",
                rep.lattice, rep.probe_points, rep.sea, rep.unsnapped, rep.tiles.len()
            );
        }
        println!("measured nodes {}, points {}", store.measured_nodes(), store.points().len());
        println!("height at 392,000 / 376,000: {:?}", store.height_at(392_000.0, 376_000.0));
    }
}
