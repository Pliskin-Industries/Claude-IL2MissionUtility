//! mapfighters.rs — park fighter packs in one coalition's influence area
//!
//! Places NodeGates packs inside the eastern or western AO. Spots stay at
//! least a Zone In radius apart, cover the whole influence polygon
//! (including narrow parts), and split evenly across waves. Fill keeps
//! that tight spacing and adds packs up to `MAX_PACKS` (8). Picks RTB
//! corners on the friendly-rear edge of the AABB and maps a GUI country
//! setpoint onto 500-series (east) or 601 (NATO). It does not clone
//! groups or rebuild NodeGates — `pack::generate_pack_at` consumes the
//! spots.
//!
//! ## Public API
//! * `MAX_PACKS` — 8
//! * `struct FighterSpot` / `struct MapFighterLayout`
//! * `fn place_in_coalition` — waves × groups, spacing, optional AO fill
//! * `fn full_pack_spots` — keep only complete N-group copies, split evenly
//! * `fn rtb_ao_point` — rear-corner RTB (east parks north, NATO south)
//! * `fn country_for_coalition` — 500-series or 501 east; 601 west
//!
//! ## Used by
//! * ui.rs (Map) — fighter-pack preview pins and generate_pack_at positions

use std::collections::HashMap;

use geo::{BoundingRect, Contains, MultiPolygon, Point};

use crate::frontlines::{AOI_GAP, densify};
use crate::mapclip::{WorldAabb, apply_salients, influence_minus_salients};

/// Hard cap on NodeGates packs written into one base map.
pub const MAX_PACKS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FighterSpot {
    pub x: f64,
    pub z: f64,
    /// 1-based wave (preview only; not written as a map icon).
    pub wave: u32,
    /// Which generated N-pack this group belongs to.
    pub pack: u32,
    /// 1-based `Group N` index inside that pack.
    pub slot: u32,
    /// AttackArea center relative to this spot (the group's Zone IN).
    pub area_dx: f64,
    pub area_dz: f64,
    /// AttackArea radius in metres. Zero draws no circle.
    pub area_r: f64,
}

#[derive(Clone, Debug)]
pub struct MapFighterLayout {
    pub eastern: bool,
    pub spots: Vec<FighterSpot>,
}

/// Country used when placing that coalition. Eastern keeps a 500-series setpoint
/// (or 501 if the Fighter Pack page is on USA). NATO is always 601.
pub fn country_for_coalition(eastern: bool, selected: i32) -> i32 {
    if eastern {
        if selected / 100 == 5 { selected } else { 501 }
    } else {
        601
    }
}

/// Friendly-rear corner of the AO for an RTB waypoint.
///
/// Eastern parks on the north edge (`x_max`), NATO on the south (`x_min`).
/// Of those two rear corners, the one closer to the group is used (west vs
/// east). Equal distance prefers west.
pub fn rtb_ao_point(eastern: bool, group_x: f64, group_z: f64, aabb: WorldAabb) -> (f64, f64) {
    let rear_x = if eastern { aabb.x_max } else { aabb.x_min };
    let west = (rear_x, aabb.z_min);
    let east = (rear_x, aabb.z_max);
    let d_west = dist2(group_x, group_z, west.0, west.1);
    let d_east = dist2(group_x, group_z, east.0, east.1);
    if d_east < d_west { east } else { west }
}

fn dist2(x0: f64, z0: f64, x1: f64, z1: f64) -> f64 {
    let dx = x0 - x1;
    let dz = z0 - z1;
    dx * dx + dz * dz
}

/// Checkerboard fighter pins in the coalition half of the AO.
pub fn place_in_coalition(
    eastern: bool,
    front_xz: &[(f64, f64)],
    aabb: WorldAabb,
    salients: &[Vec<(f64, f64)>],
    stretch_east: bool,
    min_spacing: f64,
    waves: usize,
    groups_per_wave: usize,
    fill: bool,
) -> Result<MapFighterLayout, String> {
    let waves = waves.clamp(1, MAX_PACKS);
    let groups_per_wave = groups_per_wave.max(1);
    let min_spacing = min_spacing.max(1_000.0);
    let dense = densify(front_xz, 4_000.0);
    if dense.len() < 2 {
        return Err("draw or select a front before placing fighters.".into());
    }
    let (_, patches) = apply_salients(dense.clone(), salients);
    let Some((north, south)) =
        influence_minus_salients(&dense, aabb, AOI_GAP, true, stretch_east, &patches)
    else {
        return Err("could not build areas of influence for this front.".into());
    };
    let mp = if eastern { north } else { south };
    let spots = checkerboard_spots(&mp, min_spacing, waves, groups_per_wave, fill);
    if spots.is_empty() {
        return Err(
            "coalition area is too small for Zone In spacing. Enlarge the AO or lower linked groups."
                .into(),
        );
    }
    Ok(MapFighterLayout { eastern, spots })
}

/// Keep only complete packs of `group_count` spots, split evenly across waves.
///
/// A loaded fighter pack is already wired for a fixed number of groups, so a
/// short pack cannot be stamped onto it. Leftover spots that do not fill a
/// copy are dropped. When at least `waves` copies fit, each wave gets the
/// same number of copies.
pub fn full_pack_spots(
    spots: &[FighterSpot],
    eastern: bool,
    waves: usize,
    group_count: usize,
) -> MapFighterLayout {
    let waves = waves.clamp(1, MAX_PACKS);
    let group_count = group_count.max(1);
    let pts: Vec<(f64, f64)> = spots.iter().map(|s| (s.x, s.z)).collect();
    let mut copies = (pts.len() / group_count).min(MAX_PACKS);
    if copies >= waves {
        copies = (copies / waves) * waves;
    }
    if copies == 0 {
        return MapFighterLayout {
            eastern,
            spots: Vec::new(),
        };
    }
    let use_n = copies * group_count;
    let pts = if pts.len() > use_n {
        farthest(&pts, use_n, 0.0)
    } else {
        pts
    };
    let wave_n = if copies >= waves { waves } else { copies };
    MapFighterLayout {
        eastern,
        spots: deal_spots(pts, wave_n, group_count),
    }
}

fn checkerboard_spots(
    mp: &MultiPolygon<f64>,
    min_step: f64,
    waves: usize,
    groups_per_wave: usize,
    fill: bool,
) -> Vec<FighterSpot> {
    let waves = waves.max(1);
    let groups_per_wave = groups_per_wave.max(1);
    let want = if fill {
        max_even_spots(waves, groups_per_wave)
    } else {
        waves.saturating_mul(groups_per_wave).max(1)
    };
    let mut pts = if fill {
        fill_points(mp, min_step, want)
    } else {
        spread_points(mp, min_step, want)
    };
    if pts.len() > want {
        pts = farthest(&pts, want, 0.0);
    }
    if waves > 1 && pts.len() >= waves {
        let n = (pts.len() / waves) * waves;
        if pts.len() > n {
            pts = farthest(&pts, n, 0.0);
        }
    }
    deal_spots(pts, waves, groups_per_wave)
}

/// Most spots that still split evenly and stay within `MAX_PACKS`.
fn max_even_spots(waves: usize, groups_per_wave: usize) -> usize {
    let waves = waves.max(1);
    let packs_per_wave = (MAX_PACKS / waves).max(1);
    packs_per_wave * waves * groups_per_wave
}

fn spread_points(mp: &MultiPolygon<f64>, min_step: f64, want: usize) -> Vec<(f64, f64)> {
    farthest(&interior_candidates(mp, min_step), want, min_step)
}

fn fill_points(mp: &MultiPolygon<f64>, min_step: f64, cap: usize) -> Vec<(f64, f64)> {
    let packed = pack_dense(mp, min_step);
    if packed.len() <= cap {
        packed
    } else {
        farthest(&packed, cap, 0.0)
    }
}

/// Fine lattice inside the polygon so a strip that falls between a coarse
/// Zone In grid still gets a candidate.
fn interior_candidates(mp: &MultiPolygon<f64>, min_step: f64) -> Vec<(f64, f64)> {
    let Some(rect) = mp.bounding_rect() else {
        return Vec::new();
    };
    let step = (min_step / 8.0).clamp(1_000.0, 4_000.0);
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return Vec::new();
    }
    let x0 = rect.min().x;
    let z0 = rect.min().y;
    let x1 = rect.max().x;
    let z1 = rect.max().y;
    let mut out = Vec::new();
    let mut x = x0 + step * 0.5;
    while x < x1 {
        let mut z = z0 + step * 0.5;
        while z < z1 {
            if mp_contains(mp, x, z) {
                out.push((x, z));
            }
            z += step;
        }
        x += step;
    }
    out
}

fn pack_dense(mp: &MultiPolygon<f64>, min_step: f64) -> Vec<(f64, f64)> {
    let mut grid = SpacingGrid::new(min_step);
    let mut out = Vec::new();
    for (x, z) in interior_candidates(mp, min_step) {
        if grid.fits(x, z, min_step) {
            grid.insert(x, z);
            out.push((x, z));
        }
    }
    out
}

/// Pick up to `k` points. `min_sep` is the closest two selected points may be
/// (0 keeps a subset of an already spaced set). The first point is near the
/// centroid; each later point is the one farthest from those already chosen,
/// so the set covers the polygon instead of stacking on one edge.
fn farthest(cands: &[(f64, f64)], k: usize, min_sep: f64) -> Vec<(f64, f64)> {
    if cands.is_empty() || k == 0 {
        return Vec::new();
    }
    let (cx, cz) = centroid(cands);
    let mut seed = 0usize;
    let mut seed_d = f64::MAX;
    for (i, p) in cands.iter().enumerate() {
        let d = dist2(p.0, p.1, cx, cz);
        if d < seed_d {
            seed_d = d;
            seed = i;
        }
    }
    let mut selected = vec![cands[seed]];
    let mut used = vec![false; cands.len()];
    used[seed] = true;
    while selected.len() < k {
        let mut best_i = None;
        let mut best_d = -1.0;
        for (i, p) in cands.iter().enumerate() {
            if used[i] {
                continue;
            }
            let d = selected
                .iter()
                .map(|s| (p.0 - s.0).hypot(p.1 - s.1))
                .fold(f64::MAX, f64::min);
            if d + 1.0 >= min_sep && d > best_d {
                best_d = d;
                best_i = Some(i);
            }
        }
        match best_i {
            Some(i) => {
                used[i] = true;
                selected.push(cands[i]);
            }
            None => break,
        }
    }
    selected
}

fn centroid(pts: &[(f64, f64)]) -> (f64, f64) {
    let n = pts.len() as f64;
    let (sx, sz) = pts
        .iter()
        .fold((0.0, 0.0), |(sx, sz), p| (sx + p.0, sz + p.1));
    (sx / n, sz / n)
}

struct SpacingGrid {
    cell: f64,
    bins: HashMap<(i32, i32), Vec<(f64, f64)>>,
}

impl SpacingGrid {
    fn new(min_step: f64) -> Self {
        Self {
            cell: min_step.max(1.0),
            bins: HashMap::new(),
        }
    }

    fn fits(&self, x: f64, z: f64, min_step: f64) -> bool {
        let (cx, cz) = self.key(x, z);
        for ix in (cx - 1)..=(cx + 1) {
            for iz in (cz - 1)..=(cz + 1) {
                if let Some(pts) = self.bins.get(&(ix, iz)) {
                    for p in pts {
                        if (x - p.0).hypot(z - p.1) + 1.0 < min_step {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    fn insert(&mut self, x: f64, z: f64) {
        let key = self.key(x, z);
        self.bins.entry(key).or_default().push((x, z));
    }

    fn key(&self, x: f64, z: f64) -> (i32, i32) {
        (
            (x / self.cell).floor() as i32,
            (z / self.cell).floor() as i32,
        )
    }
}

/// Round-robin in spatial order so each wave gets the same count, then cut
/// each wave into packs of `groups_per_wave` (the last pack in a wave may be
/// short when the zone cannot hold a full set).
fn deal_spots(mut pts: Vec<(f64, f64)>, waves: usize, groups_per_wave: usize) -> Vec<FighterSpot> {
    if pts.is_empty() || waves == 0 || groups_per_wave == 0 {
        return Vec::new();
    }
    pts.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut buckets: Vec<Vec<(f64, f64)>> = vec![Vec::new(); waves];
    for (i, p) in pts.into_iter().enumerate() {
        buckets[i % waves].push(p);
    }
    let mut spots = Vec::new();
    let mut pack = 0u32;
    let chunks = buckets
        .iter()
        .map(|b| b.len().div_ceil(groups_per_wave))
        .max()
        .unwrap_or(0);
    for ci in 0..chunks {
        for (wi, bucket) in buckets.iter().enumerate() {
            if pack as usize >= MAX_PACKS {
                return spots;
            }
            let start = ci * groups_per_wave;
            if start >= bucket.len() {
                continue;
            }
            let end = (start + groups_per_wave).min(bucket.len());
            let wave = (wi + 1) as u32;
            for (slot, p) in bucket[start..end].iter().enumerate() {
                spots.push(FighterSpot {
                    x: p.0,
                    z: p.1,
                    wave,
                    pack,
                    slot: (slot + 1) as u32,
                    area_dx: 0.0,
                    area_dz: 0.0,
                    area_r: 0.0,
                });
            }
            pack += 1;
        }
    }
    spots
}

fn mp_contains(mp: &MultiPolygon<f64>, x: f64, z: f64) -> bool {
    mp.contains(&Point::new(x, z))
}

#[cfg(test)]
mod tests {
    use geo::{Coord, LineString, Polygon};

    use super::*;

    fn rect_mp(x0: f64, z0: f64, x1: f64, z1: f64) -> MultiPolygon<f64> {
        let ring = LineString::from(vec![
            Coord { x: x0, y: z0 },
            Coord { x: x1, y: z0 },
            Coord { x: x1, y: z1 },
            Coord { x: x0, y: z1 },
            Coord { x: x0, y: z0 },
        ]);
        MultiPolygon(vec![Polygon::new(ring, vec![])])
    }

    fn min_pair_dist(spots: &[FighterSpot]) -> f64 {
        let mut best = f64::MAX;
        for i in 0..spots.len() {
            for j in (i + 1)..spots.len() {
                let d = (spots[i].x - spots[j].x).hypot(spots[i].z - spots[j].z);
                if d < best {
                    best = d;
                }
            }
        }
        best
    }

    #[test]
    fn country_split_is_semi_automatic() {
        assert_eq!(country_for_coalition(true, 502), 502);
        assert_eq!(country_for_coalition(true, 601), 501);
        assert_eq!(country_for_coalition(false, 501), 601);
        assert_eq!(country_for_coalition(false, 601), 601);
    }

    #[test]
    fn rtb_uses_coalition_ao_corner() {
        let aabb = WorldAabb::from_corners(80_000.0, 200_000.0, 160_000.0, 300_000.0);
        let nato = rtb_ao_point(false, 120_000.0, 250_000.0, aabb);
        assert!((nato.0 - 80_000.0).abs() < 0.1);
        assert!((nato.1 - 200_000.0).abs() < 0.1);
        let east = rtb_ao_point(true, 120_000.0, 250_000.0, aabb);
        assert!((east.0 - 160_000.0).abs() < 0.1);
        assert!((east.1 - 200_000.0).abs() < 0.1);
        let near_east = rtb_ao_point(false, 100_000.0, 298_000.0, aabb);
        assert!((near_east.0 - 80_000.0).abs() < 0.1);
        assert!((near_east.1 - 300_000.0).abs() < 0.1);
        let east_east = rtb_ao_point(true, 140_000.0, 290_000.0, aabb);
        assert!((east_east.0 - 160_000.0).abs() < 0.1);
        assert!((east_east.1 - 300_000.0).abs() < 0.1);
    }

    #[test]
    fn checkerboard_respects_zone_in_and_mixes_waves() {
        let mp = rect_mp(100_000.0, 100_000.0, 164_000.0, 164_000.0);
        let spots = checkerboard_spots(&mp, 16_000.0, 2, 5, false);
        assert!(!spots.is_empty());
        assert!(spots.len() <= 10);
        assert!(min_pair_dist(&spots) + 1.0 >= 16_000.0);
        let w1 = spots.iter().filter(|s| s.wave == 1).count();
        let w2 = spots.iter().filter(|s| s.wave == 2).count();
        assert!(w1 >= 1 && w2 >= 1, "waves should mix, got {w1} and {w2}");
        let z1: f64 = spots
            .iter()
            .filter(|s| s.wave == 1)
            .map(|s| s.z)
            .sum::<f64>()
            / w1 as f64;
        let z2: f64 = spots
            .iter()
            .filter(|s| s.wave == 2)
            .map(|s| s.z)
            .sum::<f64>()
            / w2 as f64;
        assert!(
            (z1 - z2).abs() < 24_000.0,
            "waves should interleave, not sit in opposite corners ({z1} vs {z2})"
        );
        for s in &spots {
            assert!(mp_contains(&mp, s.x, s.z));
        }
    }

    #[test]
    fn two_waves_are_separate_packs() {
        let mp = rect_mp(80_000.0, 80_000.0, 176_000.0, 176_000.0);
        let spots = checkerboard_spots(&mp, 16_000.0, 2, 5, false);
        let packs: std::collections::BTreeSet<u32> = spots.iter().map(|s| s.pack).collect();
        assert_eq!(packs.len(), 2);
        for p in packs {
            let n = spots.iter().filter(|s| s.pack == p).count();
            assert!(n >= 1 && n <= 5);
            let waves: std::collections::BTreeSet<u32> = spots
                .iter()
                .filter(|s| s.pack == p)
                .map(|s| s.wave)
                .collect();
            assert_eq!(waves.len(), 1);
        }
    }

    #[test]
    fn spread_opens_up_when_the_zone_is_large() {
        let mp = rect_mp(50_000.0, 50_000.0, 250_000.0, 250_000.0);
        let tight = checkerboard_spots(&mp, 16_000.0, 2, 4, true);
        let spread = checkerboard_spots(&mp, 16_000.0, 2, 4, false);
        assert!(spread.len() <= 8);
        assert!(tight.len() >= spread.len());
        if spread.len() >= 2 && tight.len() >= 2 {
            assert!(min_pair_dist(&spread) + 50.0 >= min_pair_dist(&tight));
        }
    }

    #[test]
    fn fill_caps_at_max_packs() {
        let mp = rect_mp(0.0, 0.0, 400_000.0, 400_000.0);
        let spots = checkerboard_spots(&mp, 16_000.0, 2, 5, true);
        let packs: std::collections::BTreeSet<u32> = spots.iter().map(|s| s.pack).collect();
        assert!(packs.len() <= MAX_PACKS);
        assert!(spots.len() <= MAX_PACKS * 5);
    }

    #[test]
    fn tiny_zone_returns_what_fits() {
        let mp = rect_mp(100_000.0, 100_000.0, 118_000.0, 118_000.0);
        let spots = checkerboard_spots(&mp, 16_000.0, 2, 5, false);
        assert!(!spots.is_empty() && spots.len() <= 4, "got {}", spots.len());
        if spots.len() >= 2 {
            assert!(min_pair_dist(&spots) + 1.0 >= 16_000.0);
            let (w1, w2) = wave_counts(&spots);
            assert_eq!(w1, w2, "tiny zone split {w1} and {w2}");
        }
    }

    fn wave_counts(spots: &[FighterSpot]) -> (usize, usize) {
        (
            spots.iter().filter(|s| s.wave == 1).count(),
            spots.iter().filter(|s| s.wave == 2).count(),
        )
    }

    #[test]
    fn two_waves_of_a_five_pack_place_ten() {
        let mp = rect_mp(0.0, 0.0, 200_000.0, 200_000.0);
        let spots = checkerboard_spots(&mp, 16_000.0, 2, 5, false);
        assert_eq!(wave_counts(&spots), (5, 5));
        assert_eq!(spots.len(), 10);
        let (min_x, max_x, min_z, max_z) = extent(&spots);
        assert!(
            max_x - min_x > 100_000.0,
            "spots stay in one band of x ({min_x}..{max_x})"
        );
        assert!(
            max_z - min_z > 100_000.0,
            "spots stay in one band of z ({min_z}..{max_z})"
        );
    }

    #[test]
    fn fill_splits_two_waves_evenly() {
        let mp = rect_mp(0.0, 0.0, 120_000.0, 120_000.0);
        let spots = checkerboard_spots(&mp, 16_000.0, 2, 5, true);
        let (w1, w2) = wave_counts(&spots);
        assert_eq!(w1, w2, "fill handed the waves {w1} and {w2}");
        assert!(w1 >= 4, "fill left only {w1} in a 120 km box");
        let packs: std::collections::BTreeSet<u32> = spots.iter().map(|s| s.pack).collect();
        assert!(packs.len() <= MAX_PACKS);
    }

    #[test]
    fn fill_covers_a_strip_a_coarse_grid_misses() {
        // 200 km by 10 km. A 16 km grid inset from the box never lands inside.
        let mp = rect_mp(0.0, 10_000.0, 200_000.0, 20_000.0);
        let spots = checkerboard_spots(&mp, 16_000.0, 2, 1, true);
        assert!(spots.len() >= 8, "thin strip got {} spots", spots.len());
        let (w1, w2) = wave_counts(&spots);
        assert_eq!(w1, w2, "thin strip split {w1} and {w2}");
        let (min_x, max_x, _, _) = extent(&spots);
        assert!(
            max_x - min_x > 100_000.0,
            "strip was filled from one end ({min_x}..{max_x})"
        );
        for s in &spots {
            assert!(mp_contains(&mp, s.x, s.z));
        }
    }

    #[test]
    fn spread_reaches_the_narrow_end_of_a_wedge() {
        // Wide along the north edge (high X), a point at the south.
        let ring = LineString::from(vec![
            Coord {
                x: 30_000.0,
                y: 100_000.0,
            },
            Coord {
                x: 170_000.0,
                y: 20_000.0,
            },
            Coord {
                x: 170_000.0,
                y: 180_000.0,
            },
            Coord {
                x: 30_000.0,
                y: 100_000.0,
            },
        ]);
        let mp = MultiPolygon(vec![Polygon::new(ring, vec![])]);
        assert!(mp_contains(&mp, 120_000.0, 100_000.0));
        let spots = checkerboard_spots(&mp, 16_000.0, 2, 4, false);
        let (w1, w2) = wave_counts(&spots);
        assert_eq!(w1, w2);
        assert_eq!(spots.len(), 8);
        let min_x = spots.iter().map(|s| s.x).fold(f64::MAX, f64::min);
        assert!(
            min_x < 100_000.0,
            "every spot sat in the wide end (min x {min_x})"
        );
    }

    fn extent(spots: &[FighterSpot]) -> (f64, f64, f64, f64) {
        let min_x = spots.iter().map(|s| s.x).fold(f64::MAX, f64::min);
        let max_x = spots.iter().map(|s| s.x).fold(f64::MIN, f64::max);
        let min_z = spots.iter().map(|s| s.z).fold(f64::MAX, f64::min);
        let max_z = spots.iter().map(|s| s.z).fold(f64::MIN, f64::max);
        (min_x, max_x, min_z, max_z)
    }
}
