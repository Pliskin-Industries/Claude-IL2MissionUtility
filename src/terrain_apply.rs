//! terrain_apply.rs — put generated units on the measured terrain
//!
//! Export-time pass over a finished `.Group` tree: ground units (Vehicle,
//! Train), planes that start on the ground (`StartType` 1–3), their
//! `MCU_TR_Entity`s and the waypoints that command ground units get a Y
//! from `terrain::HeightStore` plus a safety margin; ships go to sea level.
//! It never moves anything in X/Z and never touches airborne planes, logic
//! MCUs or static objects (Block / Bridge / Ground — the game pins those).
//!
//! Rules:
//! * A unit already within `ON_GROUND_TOLERANCE_M` of the measured ground
//!   keeps its Y — it is dev-placed or was snapped in the editor.
//! * Where the terrain is not measured closely enough (no data, or only the
//!   800 m survey), the unit keeps its old Y and is listed in the report.
//! * Applying twice gives the same file (the pass is idempotent).
//!
//! ## Public API
//! * `apply_terrain_heights(&mut Il2Entity, &HeightStore) -> ApplyReport`
//! * `ApplyReport` (`summary`), `Unmeasured`, `UnitKind`, `ON_GROUND_TOLERANCE_M`
//!
//! ## Used by
//! * ui.rs — Template, Exclusive, Army, Map and Fighter Pack exports (not
//!   the Airfield export, whose objects are already dev-placed)

use std::collections::HashMap;

use crate::ast::Il2Entity;
use crate::terrain::{ground_margin_m, parked_plane_margin_m, HeightStore};

/// Within this of the measured ground a unit counts as already on it.
pub const ON_GROUND_TOLERANCE_M: f64 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    Ground,
    ParkedPlane,
    Ship,
}

impl UnitKind {
    fn of(e: &Il2Entity) -> Option<Self> {
        match e.block_type.as_str() {
            "Vehicle" | "Train" => Some(Self::Ground),
            "Ship" => Some(Self::Ship),
            "Plane" => matches!(e.property("StartType").map(str::trim), Some("1" | "2" | "3"))
                .then_some(Self::ParkedPlane),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Ground => "ground unit",
            Self::ParkedPlane => "parked plane",
            Self::Ship => "ship",
        }
    }
}

/// A unit left at its old height because the terrain there is not measured.
#[derive(Debug, Clone, PartialEq)]
pub struct Unmeasured {
    pub kind: UnitKind,
    pub name: String,
    pub x: f64,
    pub z: f64,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ApplyReport {
    /// Units moved onto the measured ground (+ margin).
    pub placed: usize,
    /// Units already on the measured ground, left as they were.
    pub already_on_ground: usize,
    /// Ships set to sea level.
    pub ships: usize,
    /// Ground-unit waypoints put on the ground.
    pub waypoints: usize,
    pub unmeasured: Vec<Unmeasured>,
}

impl ApplyReport {
    pub fn touched_anything(&self) -> bool {
        self.placed + self.already_on_ground + self.ships + self.waypoints + self.unmeasured.len() > 0
    }

    /// One status-bar sentence, e.g. "Terrain: 12 units placed on the ground,
    /// 3 already there; 2 outside measured terrain kept their height (T-34 [1], …)."
    pub fn summary(&self) -> String {
        if !self.touched_anything() {
            return String::new();
        }
        let mut parts = vec![format!("{} unit(s) placed on the ground", self.placed)];
        if self.already_on_ground > 0 {
            parts.push(format!("{} already there", self.already_on_ground));
        }
        if self.ships > 0 {
            parts.push(format!("{} ship(s) at sea level", self.ships));
        }
        let mut s = format!("Terrain: {}", parts.join(", "));
        if !self.unmeasured.is_empty() {
            let names: Vec<String> = self
                .unmeasured
                .iter()
                .take(3)
                .map(|u| format!("{} {}", u.kind.label(), u.name))
                .collect();
            let more = if self.unmeasured.len() > 3 { ", …" } else { "" };
            s.push_str(&format!(
                "; {} outside measured terrain kept their height — snap them in the editor ({}{more})",
                self.unmeasured.len(),
                names.join(", ")
            ));
        }
        s.push('.');
        s
    }
}

fn num(e: &Il2Entity, key: &str) -> Option<f64> {
    e.property(key)?.trim().parse().ok()
}

fn link_id(e: &Il2Entity) -> Option<i32> {
    e.property("LinkTrId")?.trim().parse().ok().filter(|&id| id > 0)
}

/// Ground height + margin for `kind` at (x, z), or `None` if not measured
/// closely enough to place that kind of unit.
fn target_y(store: &HeightStore, kind: UnitKind, x: f64, z: f64) -> Option<(f64, f64)> {
    let h = store.lookup(x, z)?;
    let margin = match kind {
        UnitKind::Ground => ground_margin_m(h.spacing_m)?,
        UnitKind::ParkedPlane => parked_plane_margin_m(h.spacing_m)?,
        UnitKind::Ship => return Some((0.0, 0.0)),
    };
    Some((h.y, h.y + margin))
}

/// Put the units in `root` on the measured terrain (see the module docs).
pub fn apply_terrain_heights(root: &mut Il2Entity, store: &HeightStore) -> ApplyReport {
    let mut rep = ApplyReport::default();
    // unit Index -> kind; entity Index -> unit Index
    let mut kinds: HashMap<i32, UnitKind> = HashMap::new();
    let mut owner: HashMap<i32, i32> = HashMap::new();
    root.for_each(&mut |e| {
        if let (Some(kind), Some(idx)) = (UnitKind::of(e), e.index) {
            kinds.insert(idx, kind);
            if let Some(ent) = link_id(e) {
                owner.insert(ent, idx);
            }
        }
    });

    // units: new Y, and how far each moved (for its entity)
    let mut shift: HashMap<i32, f64> = HashMap::new();
    root.for_each_mut(&mut |e| {
        let Some(kind) = e.index.and_then(|i| kinds.get(&i).copied()) else { return };
        let (Some(x), Some(y), Some(z)) = (num(e, "XPos"), num(e, "YPos"), num(e, "ZPos")) else { return };
        let new_y = match (kind, target_y(store, kind, x, z)) {
            (UnitKind::Ship, _) => {
                rep.ships += 1;
                0.0
            }
            (_, None) => {
                rep.unmeasured.push(Unmeasured { kind, name: e.name().unwrap_or("").to_string(), x, z });
                return;
            }
            (_, Some((ground, target))) => {
                if (y - ground).abs() <= ON_GROUND_TOLERANCE_M {
                    rep.already_on_ground += 1;
                    return;
                }
                rep.placed += 1;
                target
            }
        };
        if new_y != y {
            e.set_ypos(new_y);
            shift.insert(e.index.unwrap_or_default(), new_y - y);
        }
    });

    // entities follow their unit; ground-unit waypoints go on the ground
    let ground_entities: std::collections::HashSet<i32> = owner
        .iter()
        .filter(|(_, u)| kinds.get(u) == Some(&UnitKind::Ground))
        .map(|(ent, _)| *ent)
        .collect();
    root.for_each_mut(&mut |e| match e.block_type.as_str() {
        "MCU_TR_Entity" => {
            let moved = e.index.and_then(|i| owner.get(&i)).and_then(|u| shift.get(u)).copied();
            if let (Some(d), Some(y)) = (moved, num(e, "YPos")) {
                e.set_ypos(y + d);
            }
        }
        "MCU_Waypoint" if e.objects.iter().any(|o| ground_entities.contains(o)) => {
            let (Some(x), Some(y), Some(z)) = (num(e, "XPos"), num(e, "YPos"), num(e, "ZPos")) else { return };
            if let Some((ground, target)) = target_y(store, UnitKind::Ground, x, z)
                && (y - ground).abs() > ON_GROUND_TOLERANCE_M
            {
                e.set_ypos(target);
                rep.waypoints += 1;
            }
        }
        _ => {}
    });
    rep
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_group_file;
    use crate::serialize::serialize_group;
    use crate::terrain::{KOREA_MAP_ID, DEFAULT_GROUND_MARGIN_M, DEFAULT_PARKED_PLANE_MARGIN_M};

    /// 100 m cells around X 390,000–390,500 / Z 362,000–362,500, all at 400 m,
    /// except a hill node; nothing measured further away.
    fn store() -> HeightStore {
        let mut s = HeightStore::new(KOREA_MAP_ID);
        for i in 3900..=3905 {
            for j in 3620..=3625 {
                s.set_node(i, j, 400.0);
            }
        }
        s
    }

    fn block(kind: &str, index: i32, name: &str, x: f64, y: f64, z: f64, extra: &str) -> String {
        format!(
            "  {kind}\r\n  {{\r\n    Name = \"{name}\";\r\n    Index = {index};\r\n{extra}    XPos = {x:.3};\r\n    YPos = {y:.3};\r\n    ZPos = {z:.3};\r\n  }}\r\n\r\n"
        )
    }

    fn group(children: &[String]) -> Il2Entity {
        let text = format!("Group\r\n{{\r\n  Name = \"G\";\r\n  Index = 1;\r\n{}}}\r\n", children.concat());
        parse_group_file(&text).unwrap()
    }

    fn y(root: &Il2Entity, name: &str) -> f64 {
        root.find_by_name(name).unwrap().property("YPos").unwrap().parse().unwrap()
    }

    #[test]
    fn places_ground_units_parked_planes_and_ships() {
        let mut root = group(&[
            block("Vehicle", 10, "Tank", 390_150.0, 15.26, 362_250.0, "    LinkTrId = 11;\r\n"),
            block("MCU_TR_Entity", 11, "Tank entity", 390_150.0, 15.46, 362_250.0, "    MisObjID = 10;\r\n"),
            block("Plane", 12, "Parked", 390_250.0, 0.0, 362_350.0, "    StartType = 2;\r\n"),
            block("Plane", 13, "Airborne", 390_250.0, 1500.0, 362_350.0, "    StartType = 0;\r\n"),
            block("Ship", 14, "Boat", 390_250.0, 3.0, 362_350.0, ""),
            block("Block", 15, "House", 390_250.0, -2.526, 362_350.0, ""),
        ]);
        let rep = apply_terrain_heights(&mut root, &store());
        assert_eq!(y(&root, "Tank"), 400.0 + DEFAULT_GROUND_MARGIN_M);
        assert!((y(&root, "Tank entity") - (400.2 + DEFAULT_GROUND_MARGIN_M)).abs() < 1e-6, "entity keeps its 0.2 m offset");
        assert_eq!(y(&root, "Parked"), 400.0 + DEFAULT_PARKED_PLANE_MARGIN_M);
        assert_eq!(y(&root, "Airborne"), 1500.0, "airborne planes untouched");
        assert_eq!(y(&root, "Boat"), 0.0);
        assert_eq!(y(&root, "House"), -2.526, "static objects untouched");
        assert_eq!((rep.placed, rep.ships, rep.already_on_ground), (2, 1, 0));
        assert!(rep.unmeasured.is_empty());
        assert!(rep.summary().starts_with("Terrain: 2 unit(s) placed on the ground"));
    }

    #[test]
    fn keeps_units_already_on_the_ground() {
        let mut root = group(&[block("Vehicle", 10, "Snapped", 390_150.0, 403.2, 362_250.0, "")]);
        let rep = apply_terrain_heights(&mut root, &store());
        assert_eq!(y(&root, "Snapped"), 403.2);
        assert_eq!((rep.placed, rep.already_on_ground), (0, 1));
    }

    #[test]
    fn unmeasured_units_keep_their_height_and_are_listed() {
        let mut root = group(&[
            block("Vehicle", 10, "Far tank", 250_000.0, 15.0, 250_000.0, ""),
            block("Plane", 11, "Far plane", 250_000.0, 0.0, 250_000.0, "    StartType = 1;\r\n"),
        ]);
        let rep = apply_terrain_heights(&mut root, &store());
        assert_eq!(y(&root, "Far tank"), 15.0);
        assert_eq!(rep.unmeasured.len(), 2);
        assert!(rep.summary().contains("2 outside measured terrain kept their height"));
        assert!(rep.summary().contains("ground unit Far tank"));
    }

    #[test]
    fn survey_only_terrain_is_not_trusted_for_placement() {
        let mut s = HeightStore::new(KOREA_MAP_ID);
        for (i, j) in [(4000, 3000), (4000, 3008), (4008, 3000), (4008, 3008)] {
            s.set_node(i, j, 300.0);
        }
        let mut root = group(&[block("Vehicle", 10, "T", 400_400.0, 10.0, 300_400.0, "")]);
        let rep = apply_terrain_heights(&mut root, &s);
        assert_eq!(y(&root, "T"), 10.0);
        assert_eq!(rep.unmeasured.len(), 1);
    }

    #[test]
    fn ground_waypoints_follow_and_air_waypoints_do_not() {
        let wp = |idx: i32, name: &str, obj: i32| {
            block("MCU_Waypoint", idx, name, 390_350.0, 0.0, 362_150.0, &format!("    Objects = [{obj}];\r\n"))
        };
        let mut root = group(&[
            block("Vehicle", 10, "Tank", 390_150.0, 15.0, 362_250.0, "    LinkTrId = 11;\r\n"),
            block("MCU_TR_Entity", 11, "Tank entity", 390_150.0, 15.2, 362_250.0, ""),
            block("Plane", 12, "Fighter", 390_250.0, 1500.0, 362_350.0, "    StartType = 0;\r\n    LinkTrId = 13;\r\n"),
            block("MCU_TR_Entity", 13, "Fighter entity", 390_250.0, 1500.2, 362_350.0, ""),
            wp(20, "WP ground", 11),
            wp(21, "WP air", 13),
        ]);
        let rep = apply_terrain_heights(&mut root, &store());
        assert_eq!(y(&root, "WP ground"), 400.0 + DEFAULT_GROUND_MARGIN_M);
        assert_eq!(y(&root, "WP air"), 0.0);
        assert_eq!(y(&root, "Fighter entity"), 1500.2);
        assert_eq!(rep.waypoints, 1);
    }

    #[test]
    fn applying_twice_changes_nothing_more() {
        let mut root = group(&[
            block("Vehicle", 10, "Tank", 390_150.0, 15.0, 362_250.0, "    LinkTrId = 11;\r\n"),
            block("MCU_TR_Entity", 11, "Tank entity", 390_150.0, 15.2, 362_250.0, ""),
        ]);
        apply_terrain_heights(&mut root, &store());
        let once = serialize_group(&root);
        apply_terrain_heights(&mut root, &store());
        assert_eq!(serialize_group(&root), once);
    }

    #[test]
    fn empty_store_changes_nothing_but_reports() {
        let mut root = group(&[block("Vehicle", 10, "Tank", 390_150.0, 15.0, 362_250.0, "")]);
        let before = serialize_group(&root);
        let rep = apply_terrain_heights(&mut root, &HeightStore::new(KOREA_MAP_ID));
        assert_eq!(serialize_group(&root), before);
        assert_eq!(rep.unmeasured.len(), 1);
    }
}

#[cfg(test)]
mod real_data {
    use super::*;
    use crate::parser::parse_il2_document;
    use crate::terrain::{default_store_path, KOREA_MAP_ID};

    /// `cargo test --offline real_template -- --ignored --nocapture` (needs the
    /// user's store with the 0835 test grid).
    #[test]
    #[ignore]
    fn real_template_on_measured_terrain() {
        let store = HeightStore::load(&default_store_path(), KOREA_MAP_ID).unwrap();
        let text = std::fs::read_to_string("TemplateExamples/GroundUnits/DropIns/DPRK Tank Company.Group").unwrap();
        let mut root = parse_il2_document(&text).unwrap();
        let (x0, z0) = root.first_xz().unwrap();
        root.translate_xz(392_000.0 - x0, 376_000.0 - z0);
        let before: Vec<(String, String)> = collect(&root);
        let rep = apply_terrain_heights(&mut root, &store);
        for ((name, old), (_, new)) in before.iter().zip(collect(&root)) {
            if *old != new {
                println!("{name:28} {old:>10} -> {new:>10}");
            }
        }
        println!("{}", rep.summary());
    }

    fn collect(root: &Il2Entity) -> Vec<(String, String)> {
        let mut v = Vec::new();
        root.for_each(&mut |e| {
            if let Some(y) = e.property("YPos") {
                v.push((format!("{} {}", e.block_type, e.name().unwrap_or("")), y.to_string()));
            }
        });
        v
    }
}
