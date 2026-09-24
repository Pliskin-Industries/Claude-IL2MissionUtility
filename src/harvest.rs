//! harvest.rs — Task Editor `_gen.mission` → airfield database
//!
//! Freeflight writes the whole generated mission to `data/Missions/_gen.mission`
//! whenever you start a flight from an airfield. This module cuts the start
//! airfield out of that file (no mission editor step), cleans it with
//! `airfield::clean_airfield`, and files it into a database folder:
//!
//! ```text
//! <db>/raw/<utc stamp>_gen.Mission   untouched copy + language files
//! <db>/<Airfield>_<country>.Group     cleaned airfield + language files
//! <db>/catalog.Group                  one AirfieldRecord per field/country
//! <db>/models.tsv                     every Model/Script the game used (+ raw file)
//! ```
//!
//! ## Cutting one airfield out of a mission
//! 1. Every indexed object within `radius_m` of the `Airfield` block is
//!    kept, unless another `Airfield` is closer (nearest-field split).
//! 2. Logic reachable from those objects through links (`Targets`,
//!    `Objects`, `OnEvents`, …) is pulled in up to `link_reach_m` away:
//!    K13's approach icons sit 13–15 km out. World objects (anything with a
//!    `Model`) and entities are never pulled in by links.
//! 3. Links to anything left behind are scrubbed.
//! 4. The player plane and its SP logic go (`clean_airfield`); remaining AI
//!    planes go too unless `keep_ai_planes` (`strip_ai_planes`).
//!
//! The raw file is archived before parsing, so a parse failure loses
//! nothing and later extractors can mine the archive.
//!
//! ## Runway axis
//! Taxi-graph nodes do not mark the runway (`Runway = 0` everywhere in the
//! K13–K15 exports). `AxisHeading` / `AxisLength` are the principal axis of
//! the `MCU_TR_TaxiGraph` nodes — an approximation of the main runway, grid
//! north. Older maps keep an `Airfield { Chart { Point } }` in airfield-local
//! coordinates instead; those points are counted in `TaxiNodes` but give no axis.
//!
//! ## Public API
//! * `GEN_FILE`, `DEFAULT_RADIUS_M`, `DEFAULT_LINK_REACH_M`, `DEFAULT_DB_DIR`
//! * `struct HarvestConfig`, `struct AirfieldRecord`, `struct HarvestedAirfield`,
//!   `struct HarvestOutcome`
//! * `fn default_missions_dir` / `fn find_gen_file`
//! * `fn harvest_root` — cut + clean airfields from a parsed mission (pure)
//! * `fn harvest_file` — archive, harvest, write group/catalog/models (I/O)
//! * `struct GenWatcher` — polls `_gen.mission` and reports stable rewrites
//!
//! ## Used by
//! * ui.rs (Airfield) — watcher toggle, Harvest now, Harvest a file

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::airfield::{
    clean_airfield, node_link_ids, scrub_deleted, strip_ai_planes, CleanReport,
    EASTERN_PLANE_COALITIONS, WESTERN_PLANE_COALITIONS,
};
use crate::ast::Il2Entity;
use crate::locale::{merge_template_sidecars, write_sidecars, LocaleTable};
use crate::parser::{parse_group_file, parse_il2_document};
use crate::serialize::serialize_group;

pub const GEN_FILE: &str = "_gen.mission";
pub const DEFAULT_RADIUS_M: f64 = 4000.0;
pub const DEFAULT_LINK_REACH_M: f64 = 20000.0;
pub const DEFAULT_DB_DIR: &str = "References/Airfields";

const CATALOG_FILE: &str = "catalog.Group";
const MODELS_FILE: &str = "models.tsv";
const RAW_DIR: &str = "raw";
const RECORD_BLOCK: &str = "AirfieldRecord";
/// `_gen.mission` must stop changing for this long before it is read.
const STABLE_FOR: Duration = Duration::from_millis(1500);
/// Sidecars archived with the raw mission (`pol` is not in `LANG_EXTS`).
const RAW_SIDECAR_EXTS: &[&str] = &["eng", "chs", "fra", "ger", "rus", "spa", "pol"];

const STEAM_MISSIONS: &str =
    r"C:\Program Files (x86)\Steam\steamapps\common\IL-2 Sturmovik Battle of Stalingrad\data\Missions";
const STANDALONE_MISSIONS: &str =
    r"C:\Program Files\IL-2 Sturmovik Great Battles\data\Missions";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarvestConfig {
    pub radius_m: f64,
    pub link_reach_m: f64,
    pub keep_ai_planes: bool,
}

impl Default for HarvestConfig {
    fn default() -> Self {
        Self {
            radius_m: DEFAULT_RADIUS_M,
            link_reach_m: DEFAULT_LINK_REACH_M,
            keep_ai_planes: false,
        }
    }
}

/// One catalog row. Written as an `AirfieldRecord` block in `catalog.Group`.
#[derive(Debug, Clone, PartialEq)]
pub struct AirfieldRecord {
    pub name: String,
    pub file: String,
    pub country: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Farthest kept world object from the airfield block (m).
    pub footprint_m: f64,
    pub taxi_nodes: usize,
    pub axis_heading_deg: Option<f64>,
    pub axis_length_m: Option<f64>,
    pub vehicles: usize,
    pub blocks: usize,
    pub logic: usize,
    pub map: String,
    pub date: String,
    pub source: String,
    pub captured: String,
}

#[derive(Debug, Clone)]
pub struct HarvestedAirfield {
    pub group: Il2Entity,
    pub record: AirfieldRecord,
    pub cleaned: CleanReport,
    pub ai_planes_removed: usize,
    /// Logic nodes kept only because a link reached them from inside the radius.
    pub via_links: usize,
}

#[derive(Debug, Clone)]
pub struct HarvestOutcome {
    pub archived: PathBuf,
    pub airfields: Vec<HarvestedAirfield>,
    pub models_added: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct MissionHeader {
    map: String,
    date: String,
}

#[derive(Debug, Clone)]
struct Site {
    index: i32,
    name: String,
    country: i32,
    x: f64,
    y: f64,
    z: f64,
}

pub fn default_missions_dir() -> Option<PathBuf> {
    [STEAM_MISSIONS, STANDALONE_MISSIONS]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_dir())
}

/// `_gen.mission` in `dir`, matched case-insensitively.
pub fn find_gen_file(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case(GEN_FILE))
        })
}

/// Parse `.Mission` text. `#` comment lines and the `Options` header are
/// dropped: `WindLayers` rows (`0 : 143 : 1.5;`) are not `.Group` syntax.
/// The header's `GuiMap` and `Date` are returned for the catalog.
fn parse_mission_with_header(text: &str) -> Result<(Il2Entity, MissionHeader), String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let body: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\r\n");
    let (body, options) = cut_top_level_block(&body, "Options");
    let header = options.map(|o| read_header(&o)).unwrap_or_default();
    let root = parse_il2_document(&body).map_err(|err| format!("mission parse failed: {err}"))?;
    Ok((root, header))
}

/// Remove the first top-level `name { … }` block. Returns the rest and the block text.
fn cut_top_level_block(text: &str, name: &str) -> (String, Option<String>) {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut in_quote = false;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if in_quote {
            in_quote = c != b'"';
            i += 1;
            continue;
        }
        match c {
            b'"' => in_quote = true,
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            _ if depth == 0 && bytes[i..].starts_with(name.as_bytes()) => {
                let at_word = i == 0 || !is_ident_byte(bytes[i - 1]);
                let after = &text[i + name.len()..];
                let ends_word = after.bytes().next().is_none_or(|b| !is_ident_byte(b));
                if at_word && ends_word && after.trim_start().starts_with('{') {
                    if let Some(end) = matching_brace(text, i + name.len()) {
                        let block = text[i..=end].to_string();
                        let rest = format!("{}{}", &text[..i], &text[end + 1..]);
                        return (rest, Some(block));
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    (text.to_string(), None)
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Index of the `}` closing the first `{` at or after `from`.
fn matching_brace(text: &str, from: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_quote = false;
    for (off, c) in text[from..].bytes().enumerate() {
        if in_quote {
            in_quote = c != b'"';
            continue;
        }
        match c {
            b'"' => in_quote = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(from + off);
                }
            }
            _ => {}
        }
    }
    None
}

fn read_header(options: &str) -> MissionHeader {
    let mut header = MissionHeader::default();
    for line in options.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_end_matches(';').trim().trim_matches('"');
        match key.trim() {
            "GuiMap" => header.map = value.to_string(),
            "Date" => header.date = value.to_string(),
            _ => {}
        }
    }
    header
}

/// Cut and clean airfields from a parsed mission. With a player plane only
/// the start field (nearest the player) is taken; otherwise every field.
pub fn harvest_root(root: &Il2Entity, cfg: &HarvestConfig) -> Vec<HarvestedAirfield> {
    let sites = airfield_sites(root);
    let wanted: Vec<&Site> = match player_xz(root) {
        Some((px, pz)) => sites
            .iter()
            .min_by(|a, b| dist2(a.x, a.z, px, pz).total_cmp(&dist2(b.x, b.z, px, pz)))
            .into_iter()
            .collect(),
        None => sites.iter().collect(),
    };
    wanted
        .into_iter()
        .map(|site| harvest_site(root, site, &sites, cfg))
        .collect()
}

fn harvest_site(
    root: &Il2Entity,
    site: &Site,
    sites: &[Site],
    cfg: &HarvestConfig,
) -> HarvestedAirfield {
    let (mut group, via_links) = extract_site(root, site, sites, cfg);
    let coalitions = if site.country / 100 == 5 {
        EASTERN_PLANE_COALITIONS
    } else {
        WESTERN_PLANE_COALITIONS
    };
    let cleaned = clean_airfield(&mut group, coalitions).unwrap_or(CleanReport {
        stripped: 0,
        unlinked_checkzones: 0,
        plane_coalitions: coalitions.to_string(),
    });
    let mut ai_planes_removed = 0;
    if !cfg.keep_ai_planes {
        ai_planes_removed = group.count_block_type("Plane");
        if ai_planes_removed > 0 {
            let _ = strip_ai_planes(&mut group, coalitions);
        }
    }
    let record = describe(&group, site);
    HarvestedAirfield {
        group,
        record,
        cleaned,
        ai_planes_removed,
        via_links,
    }
}

fn airfield_sites(root: &Il2Entity) -> Vec<Site> {
    let mut sites = Vec::new();
    root.for_each(&mut |e| {
        if e.block_type != "Airfield" {
            return;
        }
        let (Some(index), Some((x, z))) = (e.index, e.pos_xz()) else {
            return;
        };
        sites.push(Site {
            index,
            name: e.name().unwrap_or("Airfield").to_string(),
            country: int_prop(e, "Country").unwrap_or(0),
            x,
            y: float_prop(e, "YPos").unwrap_or(0.0),
            z,
        });
    });
    sites
}

fn player_xz(root: &Il2Entity) -> Option<(f64, f64)> {
    let mut found = None;
    root.for_each(&mut |e| {
        if found.is_none() && e.block_type == "Plane" && e.property("AILevel") == Some("0") {
            found = e.pos_xz();
        }
    });
    found
}

/// Clone the site's objects into a `Group` named after the airfield.
fn extract_site(
    root: &Il2Entity,
    site: &Site,
    sites: &[Site],
    cfg: &HarvestConfig,
) -> (Il2Entity, usize) {
    let by_index = index_map(root);
    let in_zone = |x: f64, z: f64| {
        let d = dist2(x, z, site.x, site.z);
        d <= cfg.radius_m * cfg.radius_m
            && sites
                .iter()
                .filter(|s| s.index != site.index)
                .all(|s| d <= dist2(x, z, s.x, s.z))
    };

    let mut keep: HashSet<i32> = HashSet::new();
    for (&id, e) in &by_index {
        if e.block_type == "Group" || (e.block_type == "Airfield" && id != site.index) {
            continue;
        }
        if e.pos_xz().is_some_and(|(x, z)| in_zone(x, z)) {
            keep.insert(id);
        }
    }

    let mut via_links = 0usize;
    let reach2 = cfg.link_reach_m * cfg.link_reach_m;
    let mut queue: Vec<i32> = keep.iter().copied().collect();
    while let Some(id) = queue.pop() {
        let Some(node) = by_index.get(&id) else {
            continue;
        };
        for next in node_link_ids(node) {
            if keep.contains(&next) {
                continue;
            }
            let Some(n) = by_index.get(&next) else {
                continue;
            };
            if n.block_type == "Group" || is_world_object(n) || n.block_type == "MCU_TR_Entity" {
                continue;
            }
            if n
                .pos_xz()
                .is_some_and(|(x, z)| dist2(x, z, site.x, site.z) > reach2)
            {
                continue;
            }
            keep.insert(next);
            queue.push(next);
            via_links += 1;
        }
    }

    // Entities follow their object: keep the kept objects' entities, drop orphans.
    let world_kept: Vec<i32> = keep
        .iter()
        .copied()
        .filter(|id| by_index.get(id).is_some_and(|e| is_world_object(e)))
        .collect();
    for id in world_kept {
        if let Some(link) = by_index.get(&id).and_then(|e| int_prop(e, "LinkTrId")) {
            if link > 0 && by_index.contains_key(&link) {
                keep.insert(link);
            }
        }
    }
    keep.retain(|id| {
        let Some(e) = by_index.get(id) else {
            return false;
        };
        if e.block_type != "MCU_TR_Entity" {
            return true;
        }
        match int_prop(e, "MisObjID") {
            Some(owner) if owner > 0 => keep_contains_owner(&by_index, owner, site, &in_zone),
            _ => true,
        }
    });

    let mut group = Il2Entity::new("Group");
    group.set_name(&site.name);
    group.set_property("Desc", "\"\"");
    group.children = filter_children(&root.children, &keep, &|x, z| in_zone(x, z));

    let dropped: HashSet<i32> = by_index.keys().copied().filter(|id| !keep.contains(id)).collect();
    scrub_deleted(&mut group, &dropped);
    (group, via_links)
}

/// An entity survives when its owner is inside the zone (owners are never
/// pulled in by links, so the zone test is the whole rule).
fn keep_contains_owner(
    by_index: &HashMap<i32, &Il2Entity>,
    owner: i32,
    site: &Site,
    in_zone: &dyn Fn(f64, f64) -> bool,
) -> bool {
    let Some(obj) = by_index.get(&owner) else {
        return false;
    };
    if obj.block_type == "Airfield" {
        return obj.index == Some(site.index);
    }
    obj.pos_xz().is_some_and(|(x, z)| in_zone(x, z))
}

fn filter_children(
    children: &[Il2Entity],
    keep: &HashSet<i32>,
    in_zone: &dyn Fn(f64, f64) -> bool,
) -> Vec<Il2Entity> {
    let mut out = Vec::new();
    for child in children {
        if child.block_type == "Group" {
            let inner = filter_children(&child.children, keep, in_zone);
            if !inner.is_empty() {
                let mut g = child.clone();
                g.children = inner;
                out.push(g);
            }
        } else if let Some(id) = child.index {
            if keep.contains(&id) {
                out.push(child.clone());
            }
        } else if child.pos_xz().is_some_and(|(x, z)| in_zone(x, z)) {
            out.push(child.clone());
        }
    }
    out
}

fn is_world_object(e: &Il2Entity) -> bool {
    e.property("Model").is_some()
}

fn index_map(root: &Il2Entity) -> HashMap<i32, &Il2Entity> {
    let mut map = HashMap::new();
    fill_index_map(root, &mut map);
    map
}

fn fill_index_map<'a>(e: &'a Il2Entity, map: &mut HashMap<i32, &'a Il2Entity>) {
    if let Some(id) = e.index {
        map.entry(id).or_insert(e);
    }
    for child in &e.children {
        fill_index_map(child, map);
    }
}

fn describe(group: &Il2Entity, site: &Site) -> AirfieldRecord {
    let mut footprint2: f64 = 0.0;
    let (mut vehicles, mut blocks, mut logic) = (0usize, 0usize, 0usize);
    let mut nodes: Vec<(f64, f64)> = Vec::new();
    let mut chart_points = 0usize;
    group.for_each(&mut |e| {
        match e.block_type.as_str() {
            "Vehicle" | "Ship" => vehicles += 1,
            "Block" | "Ground" | "Bridge" => blocks += 1,
            t if t.starts_with("MCU_") => logic += 1,
            _ => {}
        }
        if is_world_object(e) {
            if let Some((x, z)) = e.pos_xz() {
                footprint2 = footprint2.max(dist2(x, z, site.x, site.z));
            }
        }
        if e.block_type == "MCU_TR_TaxiGraph" {
            collect_taxi_nodes(e, &mut nodes);
        }
        if e.block_type == "Airfield" {
            chart_points += e
                .children
                .iter()
                .filter(|c| c.block_type == "Chart")
                .map(|c| c.count_block_type("Point"))
                .sum::<usize>();
        }
    });
    let axis = principal_axis(&nodes);
    AirfieldRecord {
        name: site.name.clone(),
        file: airfield_file_name(&site.name, site.country),
        country: site.country,
        x: site.x,
        y: site.y,
        z: site.z,
        footprint_m: footprint2.sqrt(),
        taxi_nodes: nodes.len() + chart_points,
        axis_heading_deg: axis.map(|a| a.0),
        axis_length_m: axis.map(|a| a.1),
        vehicles,
        blocks,
        logic,
        map: String::new(),
        date: String::new(),
        source: String::new(),
        captured: String::new(),
    }
}

fn collect_taxi_nodes(e: &Il2Entity, out: &mut Vec<(f64, f64)>) {
    for child in &e.children {
        if child.block_type == "Node" {
            if let (Some(x), Some(z)) = (float_prop(child, "X"), float_prop(child, "Z")) {
                out.push((x, z));
            }
        }
        collect_taxi_nodes(child, out);
    }
}

/// Principal axis of `points`: grid heading in [0, 180) (0 = +X, 90 = +Z)
/// and the length of the points projected onto it.
fn principal_axis(points: &[(f64, f64)]) -> Option<(f64, f64)> {
    if points.len() < 2 {
        return None;
    }
    let n = points.len() as f64;
    let mx = points.iter().map(|p| p.0).sum::<f64>() / n;
    let mz = points.iter().map(|p| p.1).sum::<f64>() / n;
    let (mut sxx, mut szz, mut sxz) = (0.0, 0.0, 0.0);
    for (x, z) in points {
        let (dx, dz) = (x - mx, z - mz);
        sxx += dx * dx;
        szz += dz * dz;
        sxz += dx * dz;
    }
    let theta = 0.5 * (2.0 * sxz).atan2(sxx - szz);
    let (c, s) = (theta.cos(), theta.sin());
    let proj = points.iter().map(|(x, z)| (x - mx) * c + (z - mz) * s);
    let (lo, hi) = proj.fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)));
    let heading = theta.to_degrees().rem_euclid(180.0);
    Some((heading, hi - lo))
}

pub fn airfield_file_name(name: &str, country: i32) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{safe}_{country}.Group")
}

/// Archive `source`, harvest it, and write groups, catalog and model list into `db`.
pub fn harvest_file(
    source: &Path,
    db: &Path,
    cfg: &HarvestConfig,
) -> Result<HarvestOutcome, String> {
    let bytes = std::fs::read(source).map_err(|err| format!("{}: {err}", source.display()))?;
    let stamp = utc_stamp(SystemTime::now());
    let archived = archive_raw(source, &bytes, db, &stamp.file)?;
    let text = String::from_utf8_lossy(&bytes);
    let (root, header) = parse_mission_with_header(&text)?;

    let mut airfields = harvest_root(&root, cfg);
    if airfields.is_empty() {
        return Err(format!(
            "no Airfield block in {} (raw copy kept at {})",
            source.display(),
            archived.display()
        ));
    }
    let source_rel = format!(
        "{RAW_DIR}/{}",
        archived.file_name().and_then(|n| n.to_str()).unwrap_or("")
    );
    let tables = merge_template_sidecars(&[source.to_path_buf()]);
    for af in &mut airfields {
        af.record.map = header.map.clone();
        af.record.date = header.date.clone();
        af.record.source = source_rel.clone();
        af.record.captured = stamp.display.clone();
        let path = db.join(&af.record.file);
        std::fs::write(&path, serialize_group(&af.group))
            .map_err(|err| format!("{}: {err}", path.display()))?;
        write_sidecars(&path, &used_tables(&af.group, &tables))?;
    }
    let records: Vec<AirfieldRecord> = airfields.iter().map(|a| a.record.clone()).collect();
    upsert_catalog(&db.join(CATALOG_FILE), &records)?;
    let raw_name = archived.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let models_added = append_models(&db.join(MODELS_FILE), &root, raw_name)?;
    Ok(HarvestOutcome {
        archived,
        airfields,
        models_added,
    })
}

fn archive_raw(source: &Path, bytes: &[u8], db: &Path, stamp: &str) -> Result<PathBuf, String> {
    let raw_dir = db.join(RAW_DIR);
    std::fs::create_dir_all(&raw_dir).map_err(|err| format!("{}: {err}", raw_dir.display()))?;
    let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("mission");
    let dest = raw_dir.join(format!("{stamp}{stem}.Mission"));
    std::fs::write(&dest, bytes).map_err(|err| format!("{}: {err}", dest.display()))?;
    for ext in RAW_SIDECAR_EXTS {
        let side = source.with_extension(ext);
        if side.is_file() {
            let _ = std::fs::copy(&side, dest.with_extension(ext));
        }
    }
    Ok(dest)
}

/// Language tables trimmed to the LC ids the harvested group uses.
fn used_tables(
    group: &Il2Entity,
    tables: &HashMap<String, LocaleTable>,
) -> HashMap<String, LocaleTable> {
    let mut ids = HashSet::new();
    group.for_each(&mut |e| {
        for (k, v) in &e.properties {
            if k.starts_with("LC") {
                if let Ok(id) = v.parse::<i32>() {
                    ids.insert(id);
                }
            }
        }
    });
    tables
        .iter()
        .map(|(ext, table)| {
            let mut out = LocaleTable::default();
            for &id in &ids {
                if let Some(text) = table.get(id) {
                    out.insert(id, text);
                }
            }
            (ext.clone(), out)
        })
        .collect()
}

fn record_entity(r: &AirfieldRecord) -> Il2Entity {
    let mut e = Il2Entity::new(RECORD_BLOCK);
    let q = |s: &str| format!("\"{}\"", s.replace('"', "'"));
    e.set_property("Name", q(&r.name));
    e.set_property("File", q(&r.file));
    e.set_property("Country", r.country.to_string());
    e.set_property("XPos", format!("{:.3}", r.x));
    e.set_property("YPos", format!("{:.3}", r.y));
    e.set_property("ZPos", format!("{:.3}", r.z));
    e.set_property("Footprint", format!("{:.0}", r.footprint_m));
    e.set_property("TaxiNodes", r.taxi_nodes.to_string());
    if let (Some(h), Some(l)) = (r.axis_heading_deg, r.axis_length_m) {
        e.set_property("AxisHeading", format!("{h:.1}"));
        e.set_property("AxisLength", format!("{l:.0}"));
    }
    e.set_property("Vehicles", r.vehicles.to_string());
    e.set_property("Blocks", r.blocks.to_string());
    e.set_property("Logic", r.logic.to_string());
    e.set_property("Map", q(&r.map));
    e.set_property("Date", q(&r.date));
    e.set_property("Source", q(&r.source));
    e.set_property("Captured", q(&r.captured));
    e
}

fn record_from_entity(e: &Il2Entity) -> Option<AirfieldRecord> {
    let text = |k: &str| e.property(k).unwrap_or("").trim_matches('"').to_string();
    Some(AirfieldRecord {
        name: e.name()?.to_string(),
        file: text("File"),
        country: int_prop(e, "Country").unwrap_or(0),
        x: float_prop(e, "XPos").unwrap_or(0.0),
        y: float_prop(e, "YPos").unwrap_or(0.0),
        z: float_prop(e, "ZPos").unwrap_or(0.0),
        footprint_m: float_prop(e, "Footprint").unwrap_or(0.0),
        taxi_nodes: int_prop(e, "TaxiNodes").unwrap_or(0).max(0) as usize,
        axis_heading_deg: float_prop(e, "AxisHeading"),
        axis_length_m: float_prop(e, "AxisLength"),
        vehicles: int_prop(e, "Vehicles").unwrap_or(0).max(0) as usize,
        blocks: int_prop(e, "Blocks").unwrap_or(0).max(0) as usize,
        logic: int_prop(e, "Logic").unwrap_or(0).max(0) as usize,
        map: text("Map"),
        date: text("Date"),
        source: text("Source"),
        captured: text("Captured"),
    })
}

/// Records in `catalog.Group` (empty when the file does not exist yet).
pub fn read_catalog(path: &Path) -> Result<Vec<AirfieldRecord>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let root = parse_group_file(&text).map_err(|err| format!("{}: {err}", path.display()))?;
    Ok(root
        .children
        .iter()
        .filter(|c| c.block_type == RECORD_BLOCK)
        .filter_map(record_from_entity)
        .collect())
}

/// Replace records with the same name and country; keep the rest sorted by name.
fn upsert_catalog(path: &Path, fresh: &[AirfieldRecord]) -> Result<(), String> {
    let mut records = read_catalog(path)?;
    for r in fresh {
        records.retain(|old| !(old.name == r.name && old.country == r.country));
        records.push(r.clone());
    }
    records.sort_by(|a, b| a.name.cmp(&b.name).then(a.country.cmp(&b.country)));
    let mut root = Il2Entity::new("Group");
    root.set_name("AirfieldDB");
    root.set_property("Desc", "\"Harvested by IL-2 Mission Utility; not an editor group\"");
    root.children = records.iter().map(record_entity).collect();
    std::fs::write(path, serialize_group(&root)).map_err(|err| format!("{}: {err}", path.display()))
}

/// Append Model/Script pairs not yet in `models.tsv`. Returns how many were new.
fn append_models(path: &Path, root: &Il2Entity, seen_at: &str) -> Result<usize, String> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let mut known: HashSet<(String, String)> = existing
        .lines()
        .skip(1)
        .filter_map(|l| {
            let mut cols = l.split('\t');
            Some((cols.next()?.to_string(), cols.next()?.to_string()))
        })
        .collect();
    let mut out = if existing.is_empty() {
        "Type\tScript\tModel\tFirstSeenIn\r\n".to_string()
    } else {
        existing
    };
    let mut added = 0usize;
    root.for_each(&mut |e| {
        let (Some(model), Some(script)) = (e.property("Model"), e.property("Script")) else {
            return;
        };
        let script = script.trim_matches('"').to_string();
        if known.insert((e.block_type.clone(), script.clone())) {
            out.push_str(&format!(
                "{}\t{script}\t{}\t{seen_at}\r\n",
                e.block_type,
                model.trim_matches('"')
            ));
            added += 1;
        }
    });
    if added > 0 {
        std::fs::write(path, out).map_err(|err| format!("{}: {err}", path.display()))?;
    }
    Ok(added)
}

struct Stamp {
    /// `2026-09-24_153012Z_` — file-name prefix.
    file: String,
    /// `2026-09-24 15:30:12 UTC`
    display: String,
}

fn utc_stamp(t: SystemTime) -> Stamp {
    let secs = t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (y, m, d) = civil_from_days(days);
    let (hh, mm, ss) = (rem / 3600, rem % 3600 / 60, rem % 60);
    Stamp {
        file: format!("{y:04}-{m:02}-{d:02}_{hh:02}{mm:02}{ss:02}Z"),
        display: format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02} UTC"),
    }
}

/// Days since 1970-01-01 → (year, month, day). Howard Hinnant's algorithm.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

fn dist2(ax: f64, az: f64, bx: f64, bz: f64) -> f64 {
    (ax - bx).powi(2) + (az - bz).powi(2)
}

fn int_prop(e: &Il2Entity, key: &str) -> Option<i32> {
    e.property(key)?.trim().parse().ok()
}

fn float_prop(e: &Il2Entity, key: &str) -> Option<f64> {
    e.property(key)?.trim().parse().ok()
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FileSig {
    modified: SystemTime,
    len: u64,
}

fn file_sig(path: &Path) -> Option<FileSig> {
    let meta = std::fs::metadata(path).ok()?;
    Some(FileSig {
        modified: meta.modified().ok()?,
        len: meta.len(),
    })
}

/// Polls a Missions folder for `_gen.mission` rewrites. A file that already
/// exists when the watcher starts is ignored (use Harvest now for it).
#[derive(Debug, Clone)]
pub struct GenWatcher {
    dir: PathBuf,
    done: Option<FileSig>,
    pending: Option<(FileSig, Instant)>,
}

impl GenWatcher {
    pub fn new(dir: PathBuf) -> Self {
        let done = find_gen_file(&dir).and_then(|p| file_sig(&p));
        Self {
            dir,
            done,
            pending: None,
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Treat the current `_gen.mission` as handled (after a manual harvest).
    pub fn acknowledge_current(&mut self) {
        self.done = find_gen_file(&self.dir).and_then(|p| file_sig(&p));
        self.pending = None;
    }

    /// The `_gen.mission` path once a new version has been stable for
    /// `STABLE_FOR`; otherwise `None`.
    pub fn poll(&mut self, now: Instant) -> Option<PathBuf> {
        let path = find_gen_file(&self.dir)?;
        let sig = file_sig(&path)?;
        if self.done == Some(sig) {
            self.pending = None;
            return None;
        }
        match self.pending {
            Some((p, since)) if p == sig => {
                if now.duration_since(since) >= STABLE_FOR {
                    self.done = Some(sig);
                    self.pending = None;
                    return Some(path);
                }
            }
            _ => self.pending = Some((sig, now)),
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_il2_document;

    fn k13() -> Il2Entity {
        parse_group_file(include_str!("../References/K13 AFB_mp.Group")).expect("K13")
    }

    fn seoul_flat() -> Il2Entity {
        parse_il2_document(include_str!("../TemplateExamples/Seoul AFB.Group")).expect("Seoul")
    }

    /// A fake `_gen.mission`: comment, Options with WindLayers, then objects.
    fn mission_text(body: &str) -> String {
        format!(
            "# Mission File Version = 1.0;\r\n\r\nOptions\r\n{{\r\n  LCName = 0;\r\n  Time = 7:56:34;\r\n  Date = 29.7.1951;\r\n  GuiMap = \"korea-summer\";\r\n  WindLayers\r\n  {{\r\n    0 :     143 :     1.5;\r\n    500 :     0 :     1;\r\n  }}\r\n  Countries\r\n  {{\r\n    Country\r\n    {{\r\n      ID = 601;\r\n      Coalition = 2;\r\n    }}\r\n  }}\r\n}}\r\n\r\n{body}\r\n\r\n# end of file"
        )
    }

    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "il2_harvest_{tag}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn block(kind: &str, index: i32, x: f64, z: f64, extra: &str) -> String {
        format!(
            "{kind}\r\n{{\r\n  Index = {index};\r\n  Name = \"{kind}{index}\";\r\n  XPos = {x:.3};\r\n  YPos = 10.000;\r\n  ZPos = {z:.3};\r\n{extra}}}\r\n"
        )
    }

    #[test]
    fn parse_mission_drops_comments_and_options() {
        let text = mission_text(&serialize_group(&k13()));
        let (root, header) = parse_mission_with_header(&text).expect("parse");
        assert_eq!(header.map, "korea-summer");
        assert_eq!(header.date, "29.7.1951");
        assert_eq!(root.name(), Some("K13 AFB"));
        assert!(root.find_by_name("K-13_Suwon_AF").is_some());
    }

    #[test]
    fn cut_block_ignores_names_inside_quotes_and_words() {
        let text = "Group\r\n{\r\n  Name = \"Options {\";\r\n}\r\nOptionsX\r\n{\r\n}\r\nOptions\r\n{\r\n  A { B = 1; }\r\n}\r\nTail\r\n{\r\n}";
        let (rest, block) = cut_top_level_block(text, "Options");
        let block = block.expect("Options block");
        assert!(block.starts_with("Options"));
        assert!(block.ends_with('}'));
        assert!(rest.contains("OptionsX"));
        assert!(rest.contains("Tail"));
        assert!(rest.contains("\"Options {\""));
    }

    #[test]
    fn k13_harvests_whole_package() {
        let root = k13();
        let before = root.count_block_type("MCU_Timer");
        let got = harvest_root(&root, &HarvestConfig::default());
        assert_eq!(got.len(), 1);
        let af = &got[0];
        assert_eq!(af.record.name, "K-13_Suwon_AF");
        assert_eq!(af.record.country, 601);
        assert_eq!(af.record.file, "K-13_Suwon_AF_601.Group");
        assert_eq!(af.group.count_block_type("MCU_Timer"), before);
        assert_eq!(af.group.count_block_type("MCU_TR_TaxiGraph"), 1);
        assert_eq!(af.record.taxi_nodes, 205);
        assert!(af.group.count_block_type("MCU_Icon") >= 30, "icons via links");
        let len = af.record.axis_length_m.unwrap();
        assert!((1500.0..4000.0).contains(&len), "axis length {len}");
        let text = serialize_group(&af.group);
        parse_group_file(&text).expect("reparse harvested group");
    }

    #[test]
    fn seoul_freeflight_drops_player_and_keeps_field() {
        let root = seoul_flat();
        let got = harvest_root(&root, &HarvestConfig::default());
        assert_eq!(got.len(), 1);
        let af = &got[0];
        let mut players = 0;
        af.group.for_each(&mut |e| {
            if e.block_type == "Plane" {
                players += 1;
            }
        });
        assert_eq!(players, 0);
        assert!(af.cleaned.stripped > 10);
        assert!(af.group.find_by_name("VERTICAL_SearchLightArea").is_some());
        assert_eq!(af.group.count_block_type("MCU_TR_TaxiGraph"), 1);
    }

    #[test]
    fn nearest_field_split_and_player_picks_start_field() {
        let a = block("Airfield", 1, 0.0, 0.0, "  Country = 601;\r\n");
        let b = block("Airfield", 2, 3000.0, 0.0, "  Country = 501;\r\n");
        let near_a = block("Block", 3, 1000.0, 0.0, "  Model = \"a.mgm\";\r\n  Script = \"a.txt\";\r\n");
        let near_b = block("Block", 4, 2000.0, 0.0, "  Model = \"b.mgm\";\r\n  Script = \"b.txt\";\r\n");
        let timer = block("MCU_Timer", 5, 100.0, 0.0, "  Targets = [4,6];\r\n  Objects = [];\r\n");
        let far_icon = block("MCU_Icon", 6, 14000.0, 0.0, "  Targets = [];\r\n  Objects = [];\r\n");
        let body = [a, b, near_a, near_b, timer, far_icon].join("\r\n");
        let root = parse_mission_with_header(&mission_text(&body)).map(|(r, _)| r).unwrap();

        let all = harvest_root(&root, &HarvestConfig::default());
        assert_eq!(all.len(), 2, "no player: every field");
        let fa = all.iter().find(|h| h.record.country == 601).unwrap();
        assert!(fa.group.find_by_name("Block3").is_some());
        assert!(fa.group.find_by_name("Block4").is_none(), "closer to field 2");
        assert!(fa.group.find_by_name("MCU_Icon6").is_some(), "pulled in by link");
        assert_eq!(fa.via_links, 1);
        let t = fa.group.find_by_name("MCU_Timer5").unwrap();
        assert_eq!(t.targets, vec![6], "link to the other field scrubbed");

        let player = block("Plane", 7, 2900.0, 0.0, "  AILevel = 0;\r\n  Country = 501;\r\n  Model = \"p.mgm\";\r\n  Script = \"p.txt\";\r\n");
        let body = format!("{body}\r\n{player}");
        let root = parse_mission_with_header(&mission_text(&body)).map(|(r, _)| r).unwrap();
        let start = harvest_root(&root, &HarvestConfig::default());
        assert_eq!(start.len(), 1);
        assert_eq!(start[0].record.country, 501);
        assert_eq!(start[0].group.count_block_type("Plane"), 0);
    }

    #[test]
    fn ai_planes_stripped_unless_kept() {
        let a = block("Airfield", 1, 0.0, 0.0, "  Country = 601;\r\n");
        let ai = block("Plane", 2, 200.0, 0.0, "  AILevel = 2;\r\n  LinkTrId = 3;\r\n  Model = \"p.mgm\";\r\n  Script = \"p.txt\";\r\n");
        let ent = block("MCU_TR_Entity", 3, 200.0, 0.0, "  Targets = [];\r\n  Objects = [];\r\n  MisObjID = 2;\r\n");
        let body = [a, ai, ent].join("\r\n");
        let root = parse_mission_with_header(&mission_text(&body)).map(|(r, _)| r).unwrap();
        let stripped = harvest_root(&root, &HarvestConfig::default());
        assert_eq!(stripped[0].ai_planes_removed, 1);
        assert_eq!(stripped[0].group.count_block_type("Plane"), 0);
        assert_eq!(stripped[0].group.count_block_type("MCU_TR_Entity"), 0);
        let cfg = HarvestConfig {
            keep_ai_planes: true,
            ..HarvestConfig::default()
        };
        let kept = harvest_root(&root, &cfg);
        assert_eq!(kept[0].group.count_block_type("Plane"), 1);
        assert_eq!(kept[0].group.count_block_type("MCU_TR_Entity"), 1);
    }

    #[test]
    fn principal_axis_of_a_line() {
        let pts: Vec<(f64, f64)> = (0..=10).map(|i| (i as f64 * 100.0, i as f64 * 100.0)).collect();
        let (h, l) = principal_axis(&pts).unwrap();
        assert!((h - 45.0).abs() < 1e-6, "heading {h}");
        assert!((l - 1000.0 * 2f64.sqrt()).abs() < 1e-6, "length {l}");
        let (h, _) = principal_axis(&[(0.0, 0.0), (0.0, 500.0)]).unwrap();
        assert!((h - 90.0).abs() < 1e-6);
    }

    #[test]
    fn utc_stamp_formats_known_instant() {
        let t = UNIX_EPOCH + Duration::from_secs(1_790_263_812); // 2026-09-24 15:30:12 UTC
        let s = utc_stamp(t);
        assert_eq!(s.display, "2026-09-24 15:30:12 UTC");
        assert_eq!(s.file, "2026-09-24_153012Z");
    }

    #[test]
    fn harvest_file_writes_db_and_upserts_catalog() {
        let game = scratch_dir("game");
        let db = scratch_dir("db");
        let gen_path = game.join("_gen.Mission");
        std::fs::write(&gen_path, mission_text(&serialize_group(&k13()))).unwrap();
        let eng = std::fs::read("References/K13 AFB_mp.eng").unwrap();
        std::fs::write(game.join("_gen.eng"), eng).unwrap();

        let out = harvest_file(&gen_path, &db, &HarvestConfig::default()).unwrap();
        assert!(out.archived.is_file());
        assert!(out.archived.with_extension("eng").is_file());
        assert!(db.join("K-13_Suwon_AF_601.Group").is_file());
        assert!(db.join("K-13_Suwon_AF_601.eng").is_file());
        assert!(out.models_added > 10);
        let cat = read_catalog(&db.join(CATALOG_FILE)).unwrap();
        assert_eq!(cat.len(), 1);
        assert_eq!(cat[0].map, "korea-summer");
        assert_eq!(cat[0].taxi_nodes, 205);
        assert!(cat[0].source.starts_with("raw/"));

        let again = harvest_file(&gen_path, &db, &HarvestConfig::default()).unwrap();
        assert_eq!(again.models_added, 0);
        assert_eq!(read_catalog(&db.join(CATALOG_FILE)).unwrap().len(), 1, "upsert, not append");

        let _ = std::fs::remove_dir_all(&game);
        let _ = std::fs::remove_dir_all(&db);
    }

    #[test]
    fn watcher_waits_for_stable_rewrite() {
        let dir = scratch_dir("watch");
        let gen_path = dir.join("_gen.mission");
        std::fs::write(&gen_path, "old").unwrap();
        let mut w = GenWatcher::new(dir.clone());
        let t0 = Instant::now();
        assert_eq!(w.poll(t0), None, "existing file ignored");

        std::fs::write(&gen_path, "new mission text").unwrap();
        assert_eq!(w.poll(t0), None, "first sighting only arms");
        assert_eq!(w.poll(t0 + Duration::from_millis(500)), None, "not stable yet");
        assert_eq!(w.poll(t0 + STABLE_FOR), Some(gen_path.clone()));
        assert_eq!(w.poll(t0 + STABLE_FOR * 2), None, "reported once");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Smoke test on a real game mission: `IL2_MISSION=<path> cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_mission_from_env() {
        let path = std::env::var("IL2_MISSION").expect("set IL2_MISSION");
        let bytes = std::fs::read(&path).unwrap();
        let (root, _) =
            parse_mission_with_header(&String::from_utf8_lossy(&bytes)).expect("parse real mission");
        let got = harvest_root(&root, &HarvestConfig::default());
        for af in &got {
            eprintln!(
                "{} c{} nodes={} logic={} veh={} blocks={} via_links={} axis={:?}/{:?}",
                af.record.name,
                af.record.country,
                af.record.taxi_nodes,
                af.record.logic,
                af.record.vehicles,
                af.record.blocks,
                af.via_links,
                af.record.axis_heading_deg,
                af.record.axis_length_m
            );
            parse_group_file(&serialize_group(&af.group)).expect("reparse");
        }
    }
}
