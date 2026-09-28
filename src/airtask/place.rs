//! Place an inspected sortie without regenerating its aircraft or order graph.

use std::collections::BTreeSet;

use super::library::{SortieShape, by_id, inspect_sortie, number};
use crate::ast::Il2Entity;
use crate::{mapnet, placement, template, weapon_range};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SortiePlacement {
    pub target: (f64, f64),
    pub heading_deg: f64,
    pub spawn_dist_m: f64,
    pub rtb: (f64, f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacedSortie {
    pub spawn: (f64, f64),
    pub leads: Vec<i32>,
    pub wp_speed_kmh: f64,
    pub attack_time_s: f64,
    /// Plane entity ids, including wingmen, for the later shell's kill hooks.
    pub planes: Vec<i32>,
    pub warnings: Vec<String>,
}

pub fn place_air_sortie(root: &mut Il2Entity, p: &SortiePlacement) -> Result<PlacedSortie, String> {
    // All fallible validation precedes mutation, including the speed fallback.
    let shape = inspect_sortie(root).map_err(|errors| errors.join("; "))?;
    if [
        p.target.0,
        p.target.1,
        p.heading_deg,
        p.spawn_dist_m,
        p.rtb.0,
        p.rtb.1,
    ]
    .iter()
    .any(|value| !value.is_finite())
        || p.spawn_dist_m < 0.0
    {
        return Err(
            "Air Tasking needs finite placement coordinates and a nonnegative spawn distance"
                .into(),
        );
    }
    let (sin, cos) = p.heading_deg.rem_euclid(360.0).to_radians().sin_cos();
    let spawn = (
        p.target.0 - p.spawn_dist_m * cos,
        p.target.1 - p.spawn_dist_m * sin,
    );
    if !spawn.0.is_finite() || !spawn.1.is_finite() {
        return Err("Air Tasking spawn coordinates overflow".into());
    }
    let missing: Vec<_> = shape
        .leads
        .iter()
        .copied()
        .filter(|lead| !shape.rtb_waypoints.contains_key(lead))
        .collect();
    let added = missing.len() + usize::from(shape.rtb_timer.is_none());
    root.max_index()
        .checked_add(added as i32 + 1)
        .ok_or("Air Tasking has no free indexes for RTB")?;
    let mut next_id = root.max_index() + 1;
    let mut new_rtb = Vec::new();
    for (n, &lead) in shape.leads.iter().enumerate() {
        if !missing.contains(&lead) {
            continue;
        }
        let altitude_node = shape
            .path_waypoints
            .get(&lead)
            .copied()
            .unwrap_or(shape.plane_objects[&lead]);
        let altitude = number(by_id(root, altitude_node), "YPos")?;
        let mut wp = template::mcu(
            "MCU_Waypoint",
            &format!("RTB {}", n + 1),
            &mut next_id,
            p.rtb.0,
            p.rtb.1,
        );
        wp.set_property("Area", "1000");
        wp.set_property("Speed", format!("{:.0}", shape.wp_speed_kmh));
        wp.set_property("Priority", "2");
        wp.set_objects(vec![lead]);
        wp.set_ypos(altitude);
        new_rtb.push(wp);
    }
    let delayed_time = number(by_id(root, shape.delayed_end_timer), "Time")?;

    rotate_to_spawn(root, &shape, p.heading_deg, spawn);
    let paths: BTreeSet<_> = shape.path_waypoints.values().copied().collect();
    mapnet::park_path_waypoints(root, &vec![p.target; paths.len()]);
    // park_path_waypoints has a legacy RTB-name filter. The inspected ids also
    // handle renamed path/RTB nodes; existing RTBs are moved to p.rtb below.
    root.for_each_mut(&mut |node| {
        if node.index.is_some_and(|index| paths.contains(&index)) {
            placement::set_coord(node, "XPos", p.target.0);
            placement::set_coord(node, "ZPos", p.target.1);
            if number(node, "Speed").expect("inspected speed") <= 150.0 {
                node.set_property("Speed", format!("{:.0}", shape.wp_speed_kmh));
            }
        }
    });
    weapon_range::snap_ground_attack_areas(root, p.target.0, p.target.1);
    weapon_range::snap_air_attack_areas(root, p.target.0, p.target.1);

    let mut rtbs: BTreeSet<_> = shape.rtb_waypoints.values().copied().collect();
    root.for_each_mut(&mut |node| {
        if node.index.is_some_and(|index| rtbs.contains(&index)) {
            placement::set_coord(node, "XPos", p.rtb.0);
            placement::set_coord(node, "ZPos", p.rtb.1);
            node.set_property("Priority", "2");
        }
        if node.index == Some(shape.delayed_end_timer) {
            node.set_property("Time", delayed_time.max(60.0).to_string());
        }
    });
    let path_id = shape.path_waypoints[&shape.navigation_lead];
    for wp in new_rtb {
        rtbs.insert(wp.index.expect("new waypoint index"));
        append_sibling(root, path_id, wp);
    }
    let rtb_timer = if let Some(index) = shape.rtb_timer {
        index
    } else {
        let at = by_id(root, shape.end_orders_timer)
            .pos_xz()
            .unwrap_or(spawn);
        let timer = template::timer("RTB DELAY", 0.5, &mut next_id, at.0, at.1);
        let index = timer.index.expect("new RTB timer index");
        append_sibling(root, shape.end_orders_timer, timer);
        index
    };
    root.for_each_mut(&mut |node| {
        if node.index == Some(rtb_timer) {
            node.set_targets(rtbs.iter().copied().collect());
        }
        if node.index == Some(shape.end_orders_timer) {
            node.append_target(rtb_timer);
        }
    });
    Ok(PlacedSortie {
        spawn,
        leads: shape.leads,
        wp_speed_kmh: shape.wp_speed_kmh,
        attack_time_s: shape.attack_time_s,
        planes: shape.planes,
        warnings: shape.warnings,
    })
}

/// Separate geometric stage so the bearing can be checked before snapping.
fn rotate_to_spawn(root: &mut Il2Entity, shape: &SortieShape, heading: f64, spawn: (f64, f64)) {
    let pivot = by_id(root, shape.plane_objects[&shape.navigation_lead])
        .pos_xz()
        .expect("inspected plane position");
    let wp = by_id(root, shape.path_waypoints[&shape.navigation_lead])
        .pos_xz()
        .expect("inspected waypoint position");
    let theta = heading.rem_euclid(360.0) - placement::heading_toward(pivot, wp);
    placement::rotate_tree(root, pivot, theta);
    placement::move_anchor_to(root, pivot, spawn);
}

fn append_sibling(root: &mut Il2Entity, sibling: i32, node: Il2Entity) {
    fn insert(root: &mut Il2Entity, sibling: i32, node: &mut Option<Il2Entity>) -> bool {
        if root
            .children
            .iter()
            .any(|child| child.index == Some(sibling))
        {
            root.children.push(node.take().expect("insert once"));
            return true;
        }
        root.children
            .iter_mut()
            .any(|child| insert(child, sibling, node))
    }
    assert!(
        insert(root, sibling, &mut Some(node)),
        "inspected MCU has a parent"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airtask::library::tests::{builtin, generate, options, order, swap_flights};
    use crate::airtask::library::{MULTIPLE_WAYPOINTS, NO_WAYPOINT, TestedShape, nodes};
    use crate::airtask::testkit::assert_links_resolve;
    use crate::template::{OrderKind, TemplateOptions};

    const PLACEMENT: SortiePlacement = SortiePlacement {
        target: (100_000.0, 150_000.0),
        heading_deg: 45.0,
        spawn_dist_m: 30_000.0,
        rtb: (80_000.0, 120_000.0),
    };

    fn expected_spawn() -> (f64, f64) {
        let distance = 30_000.0 / 2.0_f64.sqrt();
        (100_000.0 - distance, 150_000.0 - distance)
    }

    fn near(actual: (f64, f64), expected: (f64, f64), tolerance: f64) {
        assert!(
            (actual.0 - expected.0).abs() <= tolerance
                && (actual.1 - expected.1).abs() <= tolerance,
            "{actual:?} != {expected:?} (+/- {tolerance})"
        );
    }

    fn own_rtbs(root: &Il2Entity, lead: i32) -> Vec<&Il2Entity> {
        nodes(root)
            .into_iter()
            .filter(|node| {
                node.block_type == "MCU_Waypoint"
                    && mapnet::is_rtb_waypoint(node)
                    && node.objects.contains(&lead)
            })
            .collect()
    }

    #[test]
    fn place_all_builtins_lead_at_spawn() {
        for i in 0..6 {
            let mut root = builtin(i);
            let shape = inspect_sortie(&root).unwrap();
            let placed = place_air_sortie(&mut root, &PLACEMENT).unwrap();
            near(placed.spawn, expected_spawn(), 0.001);
            near(
                by_id(&root, shape.plane_objects[&shape.navigation_lead])
                    .pos_xz()
                    .unwrap(),
                expected_spawn(),
                1.0,
            );
            assert_eq!(placed.leads, shape.leads);
            assert_eq!(placed.planes, shape.planes);
            assert_eq!(placed.attack_time_s, shape.attack_time_s);
            assert_eq!(placed.wp_speed_kmh, shape.wp_speed_kmh);
        }
        let mut escort_first = builtin(3);
        swap_flights(&mut escort_first);
        let shape = inspect_sortie(&escort_first).unwrap();
        place_air_sortie(&mut escort_first, &PLACEMENT).unwrap();
        near(
            by_id(&escort_first, shape.plane_objects[&7])
                .pos_xz()
                .unwrap(),
            expected_spawn(),
            1.0,
        );
    }

    #[test]
    fn place_all_builtins_bearing() {
        for i in 0..6 {
            for original_heading in [0.0, 123.0] {
                let mut root = builtin(i);
                placement::rotate_tree(&mut root, (40_000.0, 40_000.0), original_heading);
                let shape = inspect_sortie(&root).unwrap();
                rotate_to_spawn(&mut root, &shape, PLACEMENT.heading_deg, expected_spawn());
                let lead = by_id(&root, shape.plane_objects[&shape.navigation_lead])
                    .pos_xz()
                    .unwrap();
                let wp = by_id(&root, shape.path_waypoints[&shape.navigation_lead])
                    .pos_xz()
                    .unwrap();
                let bearing = (wp.1 - lead.1)
                    .atan2(wp.0 - lead.0)
                    .to_degrees()
                    .rem_euclid(360.0);
                assert!((bearing - 45.0).abs() <= 0.01, "{i}: {bearing}");
            }
        }
    }

    #[test]
    fn place_all_builtins_offsets_rotate_rigidly() {
        for i in 0..6 {
            let mut original = builtin(i);
            original.for_each_mut(&mut |node| {
                if node.pos_xz().is_some() {
                    node.set_property("XOri", "12.500");
                    node.set_property("ZOri", "7.250");
                }
            });
            let shape = inspect_sortie(&original).unwrap();
            let pivot = by_id(&original, shape.plane_objects[&shape.navigation_lead])
                .pos_xz()
                .unwrap();
            let wp = by_id(&original, shape.path_waypoints[&shape.navigation_lead])
                .pos_xz()
                .unwrap();
            let beta = (wp.1 - pivot.1).atan2(wp.0 - pivot.0);
            let (sin, cos) = (45.0_f64.to_radians() - beta).sin_cos();
            let mut placed = original.clone();
            place_air_sortie(&mut placed, &PLACEMENT).unwrap();
            let lead = by_id(&placed, shape.plane_objects[&shape.navigation_lead])
                .pos_xz()
                .unwrap();
            for old in nodes(&original) {
                let Some((x, z)) = old.pos_xz() else {
                    continue;
                };
                let new = by_id(&placed, old.index.unwrap());
                for key in ["YPos", "XOri", "ZOri"] {
                    assert_eq!(new.property(key), old.property(key));
                }
                if old.block_type == "MCU_Waypoint" || old.block_type == "MCU_CMD_AttackArea" {
                    continue;
                }
                let (nx, nz) = new.pos_xz().unwrap();
                let (dx, dz) = (x - pivot.0, z - pivot.1);
                near(
                    (nx - lead.0, nz - lead.1),
                    (dx * cos - dz * sin, dx * sin + dz * cos),
                    0.01,
                );
                if old.block_type != "Plane" && old.block_type != "MCU_TR_Entity" {
                    assert_eq!(new.property("YOri"), old.property("YOri"));
                }
            }
        }
    }

    #[test]
    fn place_all_builtins_plane_yori() {
        for i in 0..6 {
            let mut original = builtin(i);
            original.for_each_mut(&mut |node| {
                if node.block_type == "Plane" || node.block_type == "MCU_TR_Entity" {
                    node.set_property("YOri", "350.000");
                }
            });
            let mut placed = original.clone();
            place_air_sortie(&mut placed, &PLACEMENT).unwrap();
            for old in nodes(&original)
                .into_iter()
                .filter(|node| node.block_type == "Plane" || node.block_type == "MCU_TR_Entity")
            {
                let expected = (number(old, "YOri").unwrap() + 45.0).rem_euclid(360.0);
                assert_eq!(
                    number(by_id(&placed, old.index.unwrap()), "YOri").unwrap(),
                    expected
                );
            }
        }
    }

    #[test]
    fn place_all_builtins_waypoints_and_attack_areas_on_target() {
        for i in 0..6 {
            let original = builtin(i);
            let mut placed = original.clone();
            place_air_sortie(&mut placed, &PLACEMENT).unwrap();
            let mut snapped = 0;
            for node in nodes(&placed) {
                if (node.block_type == "MCU_Waypoint" && !mapnet::is_rtb_waypoint(node))
                    || node.block_type == "MCU_CMD_AttackArea"
                {
                    near(node.pos_xz().unwrap(), PLACEMENT.target, 0.001);
                    assert_eq!(
                        node.property("YPos"),
                        by_id(&original, node.index.unwrap()).property("YPos")
                    );
                    snapped += 1;
                }
            }
            assert_eq!(snapped, [3, 2, 2, 2, 3, 3][i]);
        }
    }

    #[test]
    fn place_all_builtins_rtb_per_lead() {
        for i in 0..6 {
            let mut root = builtin(i);
            let shape = inspect_sortie(&root).unwrap();
            place_air_sortie(&mut root, &PLACEMENT).unwrap();
            let timer = root.find_by_name("RTB DELAY").unwrap();
            assert_eq!(number(timer, "Time").unwrap(), 0.5);
            assert_eq!(timer.targets.len(), shape.leads.len());
            assert!(
                by_id(&root, shape.end_orders_timer)
                    .targets
                    .contains(&timer.index.unwrap())
            );
            for (n, &lead) in shape.leads.iter().enumerate() {
                let rtbs = own_rtbs(&root, lead);
                assert_eq!(rtbs.len(), 1);
                let rtb = rtbs[0];
                assert_eq!(rtb.name().unwrap(), format!("RTB {}", n + 1));
                near(rtb.pos_xz().unwrap(), PLACEMENT.rtb, 0.001);
                assert_eq!(rtb.property("Priority"), Some("2"));
                assert_eq!(rtb.property("Area"), Some("1000"));
                assert_eq!(rtb.objects, [lead]);
                assert_eq!(number(rtb, "Speed").unwrap(), shape.wp_speed_kmh);
                let alt_source = shape
                    .path_waypoints
                    .get(&lead)
                    .copied()
                    .unwrap_or(shape.plane_objects[&lead]);
                assert_eq!(
                    number(rtb, "YPos").unwrap(),
                    number(by_id(&root, alt_source), "YPos").unwrap()
                );
                if i == 3 && lead == 11 {
                    assert_eq!(number(rtb, "YPos").unwrap(), 2400.0);
                }
                assert!(timer.targets.contains(&rtb.index.unwrap()));
            }
            assert_eq!(
                number(by_id(&root, shape.delayed_end_timer), "Time").unwrap(),
                60.0
            );
        }
    }

    #[test]
    fn place_all_builtins_links_resolve() {
        for i in 0..6 {
            let mut root = builtin(i);
            place_air_sortie(&mut root, &PLACEMENT).unwrap();
            assert_links_resolve(&root);
            let all = nodes(&root);
            let node_count = all.len();
            let ids: Vec<_> = all.iter().filter_map(|node| node.index).collect();
            assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
            let shape = inspect_sortie(&root).unwrap();
            assert_eq!(shape.kind, TestedShape::S7);
            let serialized = crate::serialize::serialize_group(&root);
            assert_eq!(crate::parser::parse_group_file(&serialized).unwrap(), root);
            let again = place_air_sortie(&mut root, &PLACEMENT).unwrap();
            assert_eq!(again.leads, shape.leads);
            assert_eq!(nodes(&root).len(), node_count);
            assert_links_resolve(&root);
        }
    }

    #[test]
    fn place_rejects_two_path_waypoints_per_lead() {
        let mut opts = options(TestedShape::S1);
        let mut second = order(OrderKind::GotoWaypoint);
        second.waypoint = 2;
        opts.seats[0].orders.insert(2, second);
        let mut root = generate(&opts);
        let before = root.clone();
        assert_eq!(
            place_air_sortie(&mut root, &PLACEMENT).unwrap_err(),
            MULTIPLE_WAYPOINTS
        );
        assert_eq!(root, before);
    }

    #[test]
    fn place_rejects_template_without_waypoint() {
        let mut opts = options(TestedShape::S1);
        opts.seats[0]
            .orders
            .retain(|order| order.kind != OrderKind::GotoWaypoint);
        let mut root = generate(&opts);
        let before = root.clone();
        assert_eq!(
            place_air_sortie(&mut root, &PLACEMENT).unwrap_err(),
            NO_WAYPOINT
        );
        assert_eq!(root, before);
    }

    #[test]
    fn place_low_speed_uses_model_spec_and_warns() {
        for speed in [100.0, 150.0] {
            let opts = TemplateOptions {
                waypoint_speed: speed,
                ..options(TestedShape::S1)
            };
            let expected = f64::from(crate::model_spec::suggested_waypoint_speed_kmh(
                opts.seats.iter().map(|seat| seat.unit.script.as_str()),
            ));
            let mut root = generate(&opts);
            let shape = inspect_sortie(&root).unwrap();
            let placed = place_air_sortie(&mut root, &PLACEMENT).unwrap();
            assert!(expected > 150.0);
            assert_eq!(placed.wp_speed_kmh, expected);
            assert_eq!(
                number(
                    by_id(&root, shape.path_waypoints[&shape.navigation_lead]),
                    "Speed"
                )
                .unwrap(),
                expected
            );
            assert_eq!(
                number(own_rtbs(&root, shape.navigation_lead)[0], "Speed").unwrap(),
                expected
            );
            assert_eq!(
                placed.warnings,
                [format!(
                    "Waypoint speed {speed} km/h replaced with model suggestion {expected} km/h"
                )]
            );
            assert_links_resolve(&root);
        }
    }

    #[test]
    fn place_low_speed_with_unknown_models_is_refused() {
        let opts = TemplateOptions {
            waypoint_speed: 100.0,
            ..options(TestedShape::S1)
        };
        let mut root = generate(&opts);
        root.for_each_mut(&mut |node| {
            if node.block_type == "Plane" {
                node.set_property("Script", "\"unknown-p14-model.txt\"");
            }
        });
        let before = root.clone();
        assert_eq!(
            place_air_sortie(&mut root, &PLACEMENT).unwrap_err(),
            "Air Tasking needs a waypoint speed above 150 km/h; no usable model speed for unknown-p14-model"
        );
        assert_eq!(root, before);
    }

    #[test]
    fn place_rtb_per_lead_keeps_and_adds() {
        for rename in [false, true] {
            for delayed_time in [2.0_f64, 90.0] {
                let mut root = generate(&options(TestedShape::S7));
                let before = inspect_sortie(&root).unwrap();
                let lead = before.leads[0];
                let existing = before.rtb_waypoints[&lead];
                root.for_each_mut(&mut |node| {
                    if node.index == Some(existing) {
                        node.set_ypos(4321.0);
                        node.set_property("Speed", "470");
                    }
                    if node.index == Some(before.delayed_end_timer) {
                        node.set_property("Time", delayed_time.to_string());
                    }
                    if rename {
                        node.set_name("X");
                    }
                });
                let old_nodes = nodes(&root).len();
                place_air_sortie(&mut root, &PLACEMENT).unwrap();
                let after = inspect_sortie(&root).unwrap();
                assert_eq!(after.rtb_waypoints[&lead], existing);
                assert_eq!(after.rtb_timer, before.rtb_timer);
                assert_eq!(nodes(&root).len(), old_nodes + 1);
                let kept = by_id(&root, existing);
                assert_eq!(number(kept, "YPos").unwrap(), 4321.0);
                assert_eq!(number(kept, "Speed").unwrap(), 470.0);
                assert_eq!(after.rtb_waypoints.len(), 2);
                let timer = by_id(&root, after.rtb_timer.unwrap());
                assert_eq!(timer.targets.len(), 2);
                for &rtb in after.rtb_waypoints.values() {
                    assert!(timer.targets.contains(&rtb));
                    near(by_id(&root, rtb).pos_xz().unwrap(), PLACEMENT.rtb, 0.001);
                }
                assert_eq!(
                    number(by_id(&root, after.delayed_end_timer), "Time").unwrap(),
                    delayed_time.max(60.0)
                );
                assert_links_resolve(&root);
            }
        }
    }
}
