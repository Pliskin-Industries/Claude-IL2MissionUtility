//! One-use Activate sortie shells (decisions 0017/0018).

use std::collections::{BTreeMap, BTreeSet};

use super::library::{SortieShape, by_id, nodes};
use super::{LossesImpl, SideLogic, ZoneOutImpl};
use crate::ast::Il2Entity;
use crate::{placement, recon, template};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellSpec {
    pub attack_time_s: f64,
    pub final_time_s: f64,
    pub extension_s: f64,
    pub target: (f64, f64),
    pub census_delay_s: f64,
    pub zone_out: ZoneOutImpl,
    pub losses: LossesImpl,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellIds {
    pub start: i32,
    pub done_hub: i32,
    pub started_on: i32,
    pub started_off: i32,
    pub hot: i32,
    pub cold: i32,
    pub flying: i32,
    pub landed: i32,
    pub delete_timer: i32,
    /// D11 exemptions, restricted to deactivated zero-second OUT relays.
    pub single_use_outs: Vec<i32>,
}

impl ShellIds {
    pub(crate) fn register(&self, area: usize, logic: &mut SideLogic) {
        logic.hot_consumers.entry(area).or_default().push(self.hot);
        logic
            .cold_consumers
            .entry(area)
            .or_default()
            .push(self.cold);
        logic.gates.push(self.start);
        logic.flags.push(self.flying);
        logic.budget_events.extend([self.start, self.delete_timer]);
        logic.init_off.extend([self.hot, self.flying]);
    }
}

pub(crate) fn by_id_mut(root: &mut Il2Entity, id: i32) -> Option<&mut Il2Entity> {
    if root.index == Some(id) {
        Some(root)
    } else {
        root.children
            .iter_mut()
            .find_map(|child| by_id_mut(child, id))
    }
}

pub(crate) fn group(name: &str, next_id: &mut i32) -> Il2Entity {
    let mut root = Il2Entity::new("Group");
    root.index = Some(*next_id);
    root.set_property("Index", next_id.to_string());
    *next_id += 1;
    root.set_name(name);
    root.set_property("Desc", "\"\"");
    root
}

fn remove_ids(root: &mut Il2Entity, removed: &BTreeSet<i32>) {
    root.children
        .retain(|node| !node.index.is_some_and(|id| removed.contains(&id)));
    if root.targets.iter().any(|id| removed.contains(id)) {
        root.set_targets(
            root.targets
                .iter()
                .copied()
                .filter(|id| !removed.contains(id))
                .collect(),
        );
    }
    if root.objects.iter().any(|id| removed.contains(id)) {
        root.set_objects(
            root.objects
                .iter()
                .copied()
                .filter(|id| !removed.contains(id))
                .collect(),
        );
    }
    for child in &mut root.children {
        remove_ids(child, removed);
    }
}

/// Cut first, then compute transitive reachability (including state links).
/// This removes unreachable cycles too, unlike an incoming-degree sweep.
fn strip(root: &mut Il2Entity, shape: &SortieShape) {
    recon::silence_clone_starts(root);
    // Decision 0018: keep the enemy-seen check, but cut its template start
    // and self-deactivation links before the orphan sweep.
    by_id_mut(root, shape.zone_in).unwrap()
        .set_targets(vec![shape.zone_out_activate, shape.zone_out_pulse]);
    by_id_mut(root, shape.zone_out).unwrap().set_targets(vec![]);
    root.for_each_mut(&mut |node| {
        if node.index.is_some_and(|id| shape.planes.contains(&id)) {
            node.children.retain(|child| !matches!(child.block_type.as_str(), "OnEvents" | "OnReports"));
        }
    });
    remove_ids(
        root,
        &shape.mission_complete_timers.iter().copied()
            .chain(shape.delete_timer).chain(shape.delete).collect(),
    );
    let all: BTreeMap<_, _> = nodes(root)
        .into_iter()
        .filter_map(|node| node.index.map(|id| (id, node)))
        .collect();
    let mut pending = vec![
        shape.bring_up_timer,
        shape.zone_in,
        shape.zone_out,
        shape.zone_out_activate,
        shape.zone_out_pulse,
        shape.end_timer,
    ];
    pending.extend(
        all.values()
            .filter(|node| node.block_type == "MCU_Waypoint")
            .filter_map(|node| node.index),
    );
    let mut reachable = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if reachable.insert(id)
            && let Some(node) = all.get(&id)
        {
            pending.extend(node.targets.iter().chain(&node.objects).copied());
        }
    }
    let removed = all
        .iter()
        .filter(|(id, node)| {
            !reachable.contains(id)
                && matches!(
                    node.block_type.as_str(),
                    "MCU_Timer"
                        | "MCU_Activate"
                        | "MCU_Deactivate"
                        | "MCU_Counter"
                        | "MCU_ModifierSetVal"
                        | "MCU_CheckZone"
                )
        })
        .map(|(&id, _)| id)
        .collect();
    remove_ids(root, &removed);
}

/// Wrap an inspected, placed copy. RTB insertion belongs to place_air_sortie;
/// this adds 17 MCUs, and assembly adds COPY r (19 including the new RTB timer).
/// A rejected wrap leaves both the input tree and the allocator unchanged.
pub fn wrap_sortie(
    root: &mut Il2Entity,
    prefix: &str,
    shape: &SortieShape,
    spec: &ShellSpec,
    next_id: &mut i32,
) -> Result<ShellIds, String> {
    if [
        spec.attack_time_s,
        spec.final_time_s,
        spec.extension_s,
        spec.census_delay_s,
    ]
    .iter()
    .any(|v| !v.is_finite() || *v < 0.0)
        || !spec.target.0.is_finite()
        || !spec.target.1.is_finite()
    {
        return Err(
            "Air Tasking needs finite, nonnegative shell timings and a finite target".into(),
        );
    }
    let present: BTreeSet<_> = nodes(root)
        .into_iter()
        .filter_map(|node| node.index)
        .collect();
    if [
        shape.bring_up_timer,
        shape.zone_in,
        shape.zone_out,
        shape.zone_out_activate,
        shape.zone_out_pulse,
        shape.end_timer,
        shape.delayed_end_timer,
        shape.navigation_attack_timer,
        shape.path_waypoints[&shape.navigation_lead],
    ]
    .iter()
    .chain(&shape.planes)
    .chain(shape.delete_timer.iter())
    .chain(shape.delete.iter())
    .any(|id| !present.contains(id))
    {
        return Err(format!("Air Tasking {prefix}: shell shape does not match this copy's indexes"));
    }
    let mut copy = root.clone();
    strip(&mut copy, shape);
    if nodes(&copy).iter().any(|node| {
        node.targets.contains(&shape.end_timer) || node.objects.contains(&shape.end_timer)
    }) {
        return Err("Air Tasking needs DONE HUB to be the only link into the end timer".into());
    }
    // Do not reuse indexes of the removed template Delete chain.
    let mut alloc = (*next_id).max(root.max_index() + 1);
    let mut added = Vec::new();
    let mut ids = BTreeMap::new();
    let (x, z) = by_id(&copy, shape.bring_up_timer)
        .pos_xz()
        .unwrap_or(spec.target);
    for (name, kind, time) in [
        ("START", "MCU_Timer", 0.0),
        ("HARD STOP", "MCU_Timer", spec.final_time_s),
        ("STARTED ON", "MCU_Activate", 0.0),
        ("STARTED OFF", "MCU_Deactivate", 0.0),
        ("ON STATION IN", "MCU_Timer", 0.0),
        ("ON STATION IN OFF", "MCU_Deactivate", 0.0),
        ("ON STATION", "MCU_Timer", spec.attack_time_s),
        ("HOT", "MCU_Timer", 0.0),
        ("COLD", "MCU_Timer", 0.0),
        ("EXTEND", "MCU_Timer", spec.extension_s),
        ("LOSSES", "MCU_Counter", 0.0),
        ("DONE HUB", "MCU_Timer", 0.0),
        ("DONE HUB OFF", "MCU_Deactivate", 0.0),
        ("FLYING", "MCU_Timer", spec.census_delay_s),
        ("LANDED", "MCU_Deactivate", 0.0),
    ] {
        let name_full = format!("{prefix} {name}");
        let node = match kind {
            "MCU_Timer" => template::timer(&name_full, time, &mut alloc, x, z),
            "MCU_Counter" => {
                template::counter(&name_full, shape.planes.len() as i32, 0, &mut alloc, x, z)
            }
            _ => template::mcu(kind, &name_full, &mut alloc, x, z),
        };
        ids.insert(name, node.index.unwrap());
        added.push(node);
    }
    // Use Template Builder's Spawn-mode Delete shapes, flown in flight 1L.
    let delay = template::timer(
        &format!("{prefix} DELETE DELAY"), template::DELETE_WAIT, &mut alloc, x, z,
    );
    let mut delete = template::mcu("MCU_Delete", &format!("{prefix} DELETE"), &mut alloc, x, z);
    delete.set_objects(shape.planes.clone());
    for (name, node) in [("DELETE DELAY", delay), ("DELETE", delete)] {
        ids.insert(name, node.index.unwrap());
        added.push(node);
    }
    let mut targets = BTreeMap::from([
        (
            ids["START"],
            vec![
                shape.bring_up_timer,
                ids["HARD STOP"],
                ids["STARTED ON"],
                ids["STARTED OFF"],
            ],
        ),
        (ids["HARD STOP"], vec![ids["DONE HUB"]]),
        (ids["STARTED ON"], vec![ids["FLYING"]]),
        (
            ids["ON STATION IN"],
            vec![ids["ON STATION IN OFF"], ids["ON STATION"]],
        ),
        (ids["ON STATION IN OFF"], vec![ids["ON STATION IN"]]),
        (ids["ON STATION"], vec![ids["HOT"], ids["COLD"]]),
        (ids["HOT"], vec![ids["EXTEND"]]),
        (ids["COLD"], vec![ids["DONE HUB"]]),
        (ids["EXTEND"], vec![ids["DONE HUB"]]),
        (ids["LOSSES"], vec![ids["DONE HUB"]]),
        (ids["DONE HUB"], vec![ids["DONE HUB OFF"], shape.end_timer]),
        (ids["DONE HUB OFF"], vec![ids["DONE HUB"]]),
        (ids["LANDED"], vec![ids["FLYING"]]),
        (ids["DELETE DELAY"], vec![ids["DELETE"], ids["LANDED"]]),
    ]);
    // Flight 0: J4 confirmed; relay latches stay as written.
    by_id_mut(&mut copy, shape.navigation_attack_timer)
        .unwrap()
        .append_target(ids["ON STATION IN"]);
    by_id_mut(&mut copy, shape.delayed_end_timer)
        .unwrap()
        .append_target(ids["DELETE DELAY"]);
    let wp = by_id_mut(&mut copy, shape.path_waypoints[&shape.navigation_lead]).unwrap();
    wp.append_target(shape.zone_in);
    let radius = by_id(&copy, shape.zone_out).property("Zone")
        .ok_or_else(|| format!("Air Tasking {prefix}: Zone Out has no Zone radius"))?
        .to_owned();
    for zone_id in [shape.zone_in, shape.zone_out] {
        let zone = by_id_mut(&mut copy, zone_id).unwrap();
        placement::set_coord(zone, "XPos", spec.target.0);
        placement::set_coord(zone, "ZPos", spec.target.1);
    }
    // Keep the template name, as for Zone Out: this is the same retained MCU.
    let zone_in = by_id_mut(&mut copy, shape.zone_in).unwrap();
    zone_in.set_property("Zone", radius);
    let zone = by_id_mut(&mut copy, shape.zone_out).unwrap();
    zone.set_targets(vec![ids["DONE HUB"]]);
    let mut single_use_outs = Vec::new();
    match spec.zone_out {
        ZoneOutImpl::CheckZone => {} // Flight 0, 0d: CheckZone remains default; J5 permits Watchdog.
        ZoneOutImpl::Watchdog => {
            // Reuse Zone Out as an inside check. A 5 s poll refreshes a 10 s
            // watchdog. This variant requires J5 = Restart; it is not selected
            // automatically. OUT gates stop both the poll and expiry at DONE.
            zone.set_property("Closer", "1");
            let mut watchdog =
                template::timer(&format!("{prefix} WATCHDOG"), 10.0, &mut alloc, x, z);
            let mut out = template::timer(&format!("{prefix} WATCHDOG OUT"), 0.0, &mut alloc, x, z);
            let mut poll =
                template::timer(&format!("{prefix} WATCHDOG POLL"), 5.0, &mut alloc, x, z);
            let mut poll_out = template::timer(
                &format!("{prefix} WATCHDOG POLL OUT"),
                0.0,
                &mut alloc,
                x,
                z,
            );
            let mut close = template::mcu(
                "MCU_Deactivate",
                &format!("{prefix} WATCHDOG OFF"),
                &mut alloc,
                x,
                z,
            );
            zone.set_targets(vec![watchdog.index.unwrap()]);
            watchdog.set_targets(vec![out.index.unwrap()]);
            out.set_targets(vec![ids["DONE HUB"]]);
            poll.set_targets(vec![poll_out.index.unwrap()]);
            poll_out.set_targets(vec![shape.zone_out, poll.index.unwrap()]);
            close.set_targets(vec![out.index.unwrap(), poll_out.index.unwrap()]);
            single_use_outs.extend([out.index.unwrap(), poll_out.index.unwrap()]);
            targets
                .get_mut(&ids["DONE HUB"])
                .unwrap()
                .push(close.index.unwrap());
            let wp = by_id_mut(&mut copy, shape.path_waypoints[&shape.navigation_lead]).unwrap();
            wp.append_target(watchdog.index.unwrap());
            wp.append_target(poll.index.unwrap());
            added.extend([watchdog, out, poll, poll_out, close]);
        }
    }
    // Flight 1G and decisions 0011/0017: one OnKilled (13) per aircraft.
    match spec.losses {
        LossesImpl::Type13 => {
            for &entity in &shape.planes {
                template::attach_event(
                    by_id_mut(&mut copy, entity).unwrap(),
                    template::EntityEvent::OnKilled.type_id(),
                    ids["LOSSES"],
                );
            }
        }
    }
    for node in &mut added {
        if let Some(links) = targets.remove(&node.index.unwrap()) {
            node.set_targets(links);
        }
    }
    fn append(root: &mut Il2Entity, sibling: i32, added: &mut Vec<Il2Entity>) -> bool {
        if root.children.iter().any(|node| node.index == Some(sibling)) {
            root.children.append(added);
            true
        } else {
            root.children
                .iter_mut()
                .any(|child| append(child, sibling, added))
        }
    }
    if !append(&mut copy, shape.bring_up_timer, &mut added) {
        return Err("Air Tasking bring-up timer has no containing group".into());
    }
    *root = copy;
    *next_id = alloc;
    Ok(ShellIds {
        start: ids["START"],
        done_hub: ids["DONE HUB"],
        started_on: ids["STARTED ON"],
        started_off: ids["STARTED OFF"],
        hot: ids["HOT"],
        cold: ids["COLD"],
        flying: ids["FLYING"],
        landed: ids["LANDED"],
        delete_timer: ids["DELETE DELAY"],
        single_use_outs,
    })
}

#[cfg(test)]
pub(crate) mod test_support {
    pub(crate) use super::tests::{active, configs, fixture, mcus, named, ticks, times};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airtask::assemble::emit_side_logic;
    use crate::airtask::library::tests::{builtin, generate, options, swap_flights};
    use crate::airtask::library::{TestedShape, inspect_sortie, number};
    use crate::airtask::place::{SortiePlacement, place_air_sortie};
    use crate::airtask::testkit::{self, *};
    use crate::airtask::{AirSide, Strategies};

    pub(crate) const TARGET: (f64, f64) = (150_000.0, 260_000.0);

    pub(crate) struct Fixture {
        pub(crate) root: Il2Entity,
        pub(crate) shape: SortieShape,
        pub(crate) ids: ShellIds,
        pub(crate) logic: SideLogic,
        pub(crate) next_id: i32,
        pub(crate) pointer: i32,
    }

    pub(crate) fn fixture(mut root: Il2Entity, strategies: Strategies) -> Fixture {
        let shape = inspect_sortie(&root).unwrap();
        place_air_sortie(
            &mut root,
            &SortiePlacement {
                target: TARGET,
                heading_deg: 0.0,
                spawn_dist_m: shape.wp_speed_kmh / 3.6 * 300.0,
                rtb: (60_000.0, 230_000.0),
            },
        )
        .unwrap();
        let shape = inspect_sortie(&root).unwrap();
        let prefix = format!("S{}01", shape.side.letter());
        let mut next_id = root.max_index() + 1;
        let ids = wrap_sortie(
            &mut root,
            &prefix,
            &shape,
            &ShellSpec {
                attack_time_s: shape.attack_time_s,
                final_time_s: shape.attack_time_s + 1500.0,
                extension_s: 600.0,
                target: TARGET,
                census_delay_s: 0.05,
                zone_out: strategies.zone_out,
                losses: strategies.losses,
            },
            &mut next_id,
        )
        .unwrap();
        let mut pointer = template::timer(
            &format!("E{}01 COPY 1", shape.side.letter()),
            0.0,
            &mut next_id,
            TARGET.0,
            TARGET.1,
        );
        pointer.set_targets(vec![ids.start]);
        let pointer_id = pointer.index.unwrap();
        root.children.push(pointer);
        let mut logic = SideLogic::default();
        ids.register(1, &mut logic);
        logic.links.push((ids.started_off, pointer_id));
        Fixture {
            root,
            shape,
            ids,
            logic,
            next_id,
            pointer: pointer_id,
        }
    }

    pub(crate) fn fixtures() -> Vec<Fixture> {
        (0..6)
            .map(|i| fixture(builtin(i), Strategies::default()))
            .collect()
    }

    pub(crate) fn configs() -> Vec<WalkerConfig> {
        [false, true]
            .into_iter()
            .flat_map(|reverse_targets| {
                [
                    TimerRetrigger::Restart,
                    TimerRetrigger::Ignore,
                    TimerRetrigger::Twice,
                ]
                .into_iter()
                .map(move |timer_retrigger| WalkerConfig {
                    reverse_targets,
                    timer_retrigger,
                    ..WalkerConfig::default()
                })
            })
            .collect()
    }

    pub(crate) fn ticks(seconds: f64) -> Tick {
        (seconds * 20.0).round() as Tick
    }
    pub(crate) fn times(log: &WalkerLog, id: i32) -> Vec<Tick> {
        log.pulses
            .iter()
            .filter(|p| p.index == id)
            .map(|p| p.t)
            .collect()
    }
    pub(crate) fn active(log: &WalkerLog, id: i32, t: Tick) -> bool {
        log.snapshots
            .iter()
            .rev()
            .find(|s| s.t <= t)
            .unwrap()
            .active[&id]
    }
    pub(crate) fn named<'a>(root: &'a Il2Entity, name: &str) -> &'a Il2Entity {
        root.find_by_name(name).unwrap()
    }
    pub(crate) fn mcus(root: &Il2Entity) -> usize {
        nodes(root)
            .iter()
            .filter(|n| n.block_type.starts_with("MCU_"))
            .count()
    }
    fn incoming(root: &Il2Entity, id: i32) -> BTreeSet<i32> {
        nodes(root)
            .iter()
            .filter(|n| {
                n.targets.contains(&id)
                    && !matches!(
                        n.block_type.as_str(),
                        "MCU_Activate" | "MCU_Deactivate" | "MCU_ModifierSetVal"
                    )
            })
            .filter_map(|n| n.index)
            .collect()
    }
    fn shell_id(f: &Fixture, suffix: &str) -> i32 {
        named(&f.root, &format!("S{}01 {suffix}", f.shape.side.letter()))
            .index
            .unwrap()
    }
    pub(crate) fn runtime(f: &Fixture, hot: bool) -> Il2Entity {
        let mut root = f.root.clone();
        let mut logic = f.logic.clone();
        if hot {
            logic.init_off.retain(|&id| id != f.ids.hot);
            logic.init_off.push(f.ids.cold);
        }
        let mut next = f.next_id;
        let init = emit_side_logic(
            f.shape.side,
            &logic,
            std::slice::from_mut(&mut root),
            &mut next,
        )
        .unwrap();
        root.children.extend(init);
        assert_links_resolve(&root);
        root
    }
    pub(crate) fn input(f: &Fixture, until: f64, arrival: Option<f64>) -> WalkerInput {
        WalkerInput {
            until: ticks(until),
            pulses: vec![(ticks(2.0), f.pointer.into())],
            arrivals: arrival
                .into_iter()
                .map(|t| {
                    (
                        ticks(t),
                        f.shape.path_waypoints[&f.shape.navigation_lead].into(),
                    )
                })
                .collect(),
            tracks: vec![ScriptedTrack {
                coalition: if f.shape.side == AirSide::Nato { 1 } else { 2 },
                country: if f.shape.side == AirSide::Nato { 501 } else { 601 },
                samples: vec![(0, TARGET.0, TARGET.1)],
            }],
            ..WalkerInput::default()
        }
    }

    #[test]
    fn shell_keeps_zone_in_and_strips_start() {
        for f in fixtures() {
            assert_eq!(by_id(&f.root, f.shape.zone_in).name(), Some("Zone IN"));
            for name in [
                "ENABLE / PULSE IN",
                "COOLDOWN",
                "Zone In ReActivate",
                "Self Deactivate",
            ] {
                assert!(f.root.find_by_name(name).is_none(), "{name}");
            }
            assert!(
                !nodes(&f.root)
                    .iter()
                    .any(|n| n.name().is_some_and(|s| s.starts_with("Mission Complete")))
            );
            let begin = by_id(&f.root, f.shape.mission_begin);
            assert_eq!(begin.property("Enabled"), Some("0"));
            assert!(begin.targets.is_empty());
            assert_links_resolve(&f.root);
        }
    }

    #[test]
    fn shell_end_timer_only_from_done_hub() {
        for f in fixtures() {
            assert_eq!(
                incoming(&f.root, f.shape.end_timer),
                BTreeSet::from([f.ids.done_hub])
            );
        }
    }

    #[test]
    fn shell_done_hub_inputs() {
        for f in fixtures() {
            let expected = [
                shell_id(&f, "HARD STOP"),
                f.ids.cold,
                shell_id(&f, "EXTEND"),
                shell_id(&f, "LOSSES"),
                f.shape.zone_out,
            ];
            assert_eq!(
                incoming(&f.root, f.ids.done_hub),
                expected.into_iter().collect()
            );
            let off = shell_id(&f, "DONE HUB OFF");
            assert!(by_id(&f.root, f.ids.done_hub).targets.contains(&off));
            assert_eq!(by_id(&f.root, off).targets, [f.ids.done_hub]);
        }
    }

    #[test]
    fn shell_delete_timer_releases_the_budget() {
        for f in fixtures() {
            let delete = shell_id(&f, "DELETE");
            assert_eq!(by_id(&f.root, f.ids.delete_timer).targets, [delete, f.ids.landed]);
            assert_eq!(by_id(&f.root, delete).block_type, "MCU_Delete");
            assert_eq!(by_id(&f.root, delete).objects, f.shape.planes);
            assert_eq!(by_id(&f.root, f.ids.landed).block_type, "MCU_Deactivate");
            assert_eq!(by_id(&f.root, f.ids.landed).targets, [f.ids.flying]);
            assert!(f.logic.budget_events.contains(&f.ids.delete_timer));
        }
    }

    #[test]
    fn shell_one_on_station_per_sortie() {
        for (f, duration) in fixtures()
            .into_iter()
            .zip([1080.0, 600.0, 900.0, 1200.0, 300.0, 600.0])
        {
            let station = shell_id(&f, "ON STATION");
            assert_eq!(number(by_id(&f.root, station), "Time").unwrap(), duration);
            assert_eq!(
                incoming(&f.root, station),
                BTreeSet::from([shell_id(&f, "ON STATION IN")])
            );
            assert_eq!(
                incoming(&f.root, shell_id(&f, "ON STATION IN")),
                BTreeSet::from([f.shape.navigation_attack_timer])
            );
            let attack = by_id(&f.root, f.shape.navigation_attack_timer);
            assert!(attack.targets.iter().any(|&id| {
                let n = by_id(&f.root, id);
                n.block_type == "MCU_CMD_AttackArea" && n.objects.contains(&f.shape.navigation_lead)
            }));
        }
    }

    #[test]
    fn shell_on_station_with_the_escort_first() {
        let mut root = builtin(3);
        swap_flights(&mut root);
        let f = fixture(root, Strategies::default());
        assert_eq!(f.shape.leads, [11, 7]);
        assert_eq!(f.shape.navigation_lead, 7);
        assert!(
            by_id(&f.root, 37)
                .targets
                .contains(&shell_id(&f, "ON STATION IN"))
        );
        for config in configs() {
            let log = walk(&runtime(&f, false), config, &input(&f, 1300.0, Some(10.0)));
            assert_eq!(times(&log, f.ids.done_hub), [ticks(1210.5)]);
        }
    }

    #[test]
    fn shell_wp1_pulses_zone_in() {
        for f in fixtures() {
            let wp = by_id(&f.root, f.shape.path_waypoints[&f.shape.navigation_lead]);
            assert!(wp.targets.contains(&f.shape.zone_in));
            assert!(!wp.targets.contains(&f.shape.zone_out_activate));
            assert!(!wp.targets.contains(&f.shape.zone_out_pulse));
            let zone_in = by_id(&f.root, f.shape.zone_in);
            assert_eq!(zone_in.targets, [f.shape.zone_out_activate, f.shape.zone_out_pulse]);
            assert_eq!(incoming(&f.root, f.shape.zone_in), BTreeSet::from([wp.index.unwrap()]));
            assert_eq!(incoming(&f.root, f.shape.zone_out_activate), BTreeSet::from([f.shape.zone_in]));
            assert_eq!(incoming(&f.root, f.shape.zone_out_pulse), BTreeSet::from([f.shape.zone_in]));
            assert_eq!(by_id(&f.root, f.shape.zone_out_activate).targets, [f.shape.zone_out]);
            assert_eq!(by_id(&f.root, f.shape.zone_out_pulse).targets, [f.shape.zone_out]);
            assert_eq!(number(by_id(&f.root, f.shape.zone_out_pulse), "Time").unwrap(), 0.1);
            assert!(!f.logic.init_off.contains(&f.shape.zone_in));
            let root = runtime(&f, false);
            assert!(!nodes(&root).iter().any(|node| node.block_type == "MCU_Deactivate"
                && (node.targets.contains(&f.shape.zone_in) || node.objects.contains(&f.shape.zone_in))));
            assert_eq!(f.root.count_block_type("MCU_CheckZone"), 2);
            for (zone, closer) in [(f.shape.zone_in, "1"), (f.shape.zone_out, "0")] {
                let zone = by_id(&f.root, zone);
                assert_eq!(zone.pos_xz(), Some(TARGET));
                assert_eq!(number(zone, "Zone").unwrap(), 35_000.0);
                assert_eq!(zone.property("Closer"), Some(closer));
                assert_eq!(zone.property("PlaneCoalitions"),
                    Some(if f.shape.side == AirSide::Nato { "[1]" } else { "[2]" }));
            }
        }
    }

    #[test]
    fn shell_losses_counts_every_plane() {
        for (f, count) in fixtures().into_iter().zip([4, 4, 4, 4, 4, 8]) {
            let losses = shell_id(&f, "LOSSES");
            assert_eq!(
                number(by_id(&f.root, losses), "Counter").unwrap(),
                f64::from(count)
            );
            assert_eq!(by_id(&f.root, losses).property("Dropcount"), Some("0"));
            let events: Vec<_> = nodes(&f.root)
                .into_iter()
                .filter(|n| n.block_type == "OnEvent")
                .collect();
            assert_eq!(events.len(), count as usize);
            for event in events {
                assert_eq!(event.property("Type"), Some("13"));
                assert_eq!(event.property("TarId"), Some(losses.to_string().as_str()));
            }
        }
    }

    #[test]
    fn shell_end_of_station_pair() {
        for f in fixtures() {
            assert_eq!(
                by_id(&f.root, shell_id(&f, "ON STATION")).targets,
                [f.ids.hot, f.ids.cold]
            );
            assert_eq!(by_id(&f.root, f.ids.hot).targets, [shell_id(&f, "EXTEND")]);
            for id in [f.ids.cold, shell_id(&f, "EXTEND")] {
                assert_eq!(by_id(&f.root, id).targets, [f.ids.done_hub]);
            }
            assert_eq!(f.logic.hot_consumers[&1], [f.ids.hot]);
            assert_eq!(f.logic.cold_consumers[&1], [f.ids.cold]);
            assert_eq!(f.root.count_block_type("MCU_CheckZone"), 2);
        }
    }

    #[test]
    fn shell_timing_values() {
        for (i, expected) in [(1, 2100.0), (3, 2700.0)] {
            let f = fixture(builtin(i), Strategies::default());
            assert_eq!(
                number(by_id(&f.root, shell_id(&f, "HARD STOP")), "Time").unwrap(),
                expected
            );
            assert_eq!(
                number(by_id(&f.root, shell_id(&f, "EXTEND")), "Time").unwrap(),
                600.0
            );
        }
    }

    #[test]
    fn shell_start_is_the_gate() {
        for f in fixtures() {
            assert_eq!(by_id(&f.root, f.pointer).targets, [f.ids.start]);
            assert!(
                !nodes(&f.root)
                    .iter()
                    .any(|n| n.name().is_some_and(|s| s.ends_with(" GATE")))
            );
            assert_eq!(f.logic.gates, [f.ids.start]);
            assert!(f.logic.budget_events.contains(&f.ids.start));
            assert_eq!(
                by_id(&f.root, f.ids.start).targets,
                [
                    f.shape.bring_up_timer,
                    shell_id(&f, "HARD STOP"),
                    f.ids.started_on,
                    f.ids.started_off
                ]
            );
            assert_eq!(by_id(&f.root, f.ids.started_on).targets, [f.ids.flying]);
            for node in nodes(&f.root).into_iter().filter(|n| {
                n.name()
                    .is_some_and(|name| name.starts_with("SN") || name.starts_with("SD"))
            }) {
                for &target in &node.targets {
                    assert!(!by_id(&f.root, target).name().is_some_and(|name| {
                        ["BUD", "EN", "ED", "ZN", "ZD"]
                            .iter()
                            .any(|p| name.starts_with(p))
                    }));
                }
            }
        }
    }

    #[test]
    fn shell_adds_19_mcus_per_copy() {
        let original = builtin(1);
        let shape = inspect_sortie(&original).unwrap();
        let mut stripped = original.clone();
        strip(&mut stripped, &shape);
        assert_eq!(mcus(&original) - mcus(&stripped), 8);
        let f = fixture(original, Strategies::default());
        assert_eq!(mcus(&f.root) - mcus(&stripped), 19 + 1);
        assert_eq!(mcus(&f.root), 43);
        let mut without_zone_in = f.root.clone();
        remove_ids(&mut without_zone_in, &BTreeSet::from([f.shape.zone_in]));
        assert_eq!(mcus(&without_zone_in), 42); // decision 0017
        assert!(f.root.find_by_name("RTB 1").is_some());
    }

    #[test]
    fn shell_keeps_template_rtb_timer() {
        let original = generate(&options(TestedShape::S7));
        let shape = inspect_sortie(&original).unwrap();
        let rtb = shape.rtb_timer.unwrap();
        let mut stripped = original.clone();
        strip(&mut stripped, &shape);
        assert_eq!(mcus(&original) - mcus(&stripped), 7);
        let f = fixture(original, Strategies::default());
        assert_eq!(f.shape.rtb_timer, Some(rtb));
        assert_eq!(mcus(&f.root) - mcus(&stripped), 18 + 1); // one lead lacked RTB
        assert_eq!(mcus(&f.root), 47);
        let mut without_zone_in = f.root.clone();
        remove_ids(&mut without_zone_in, &BTreeSet::from([f.shape.zone_in]));
        assert_eq!(mcus(&without_zone_in), 46); // decision 0017
        assert_eq!(
            nodes(&f.root)
                .iter()
                .filter(|n| n.name() == Some("RTB DELAY"))
                .count(),
            1
        );
        for config in configs() {
            let log = walk(&runtime(&f, false), config, &input(&f, 2000.0, Some(10.0)));
            assert_eq!(times(&log, rtb).len(), 1);
        }
    }

    #[test]
    fn shell_replaces_the_template_delete_chain() {
        for original in [builtin(1), generate(&options(TestedShape::S1))] {
            let f = fixture(original, Strategies::default());
            assert_eq!(f.root.count_block_type("MCU_Delete"), 1);
            let delete = named(&f.root, "SN01 DELETE");
            assert_eq!(delete.objects, f.shape.planes);
            assert_eq!(by_id(&f.root, f.ids.delete_timer).name(), Some("SN01 DELETE DELAY"));
            assert_eq!(number(by_id(&f.root, f.ids.delete_timer), "Time").unwrap(), 0.5);
            assert_eq!(by_id(&f.root, f.ids.delete_timer).targets, [delete.index.unwrap(), f.ids.landed]);
            assert!(by_id(&f.root, f.shape.delayed_end_timer).targets.contains(&f.ids.delete_timer));
            for old in [f.shape.delete, f.shape.delete_timer].into_iter().flatten() {
                assert!(!nodes(&f.root).iter().any(|node| node.index == Some(old)));
            }
            assert_links_resolve(&f.root);
        }
    }

    #[test]
    fn shell_does_not_reuse_removed_delete_chain_indexes() {
        let mut root = builtin(1);
        place_air_sortie(&mut root, &SortiePlacement {
            target: TARGET,
            heading_deg: 0.0,
            spawn_dist_m: 55_000.0,
            rtb: (60_000.0, 230_000.0),
        }).unwrap();
        let shape = inspect_sortie(&root).unwrap();
        // Put the removed chain above every retained index, then pass a stale
        // allocator so the shell must reserve the input's whole index range.
        let removed = [root.max_index() + 1, root.max_index() + 2];
        let remap = std::collections::HashMap::from([
            (shape.delete_timer.unwrap(), removed[0]),
            (shape.delete.unwrap(), removed[1]),
        ]);
        for (&old, &new) in &remap {
            let node = by_id_mut(&mut root, old).unwrap();
            node.index = Some(new);
            node.set_property("Index", new.to_string());
        }
        crate::duplicate::reconnect_pointers(&mut root, &remap);
        let shape = inspect_sortie(&root).unwrap();
        let mut retained = root.clone();
        strip(&mut retained, &shape);
        let retained: BTreeSet<_> = nodes(&retained).iter().filter_map(|node| node.index).collect();
        let mut next = 1;
        wrap_sortie(&mut root, "SN01", &shape, &ShellSpec {
            attack_time_s: 600.0,
            final_time_s: 2100.0,
            extension_s: 600.0,
            target: TARGET,
            census_delay_s: 0.05,
            zone_out: ZoneOutImpl::CheckZone,
            losses: LossesImpl::Type13,
        }, &mut next).unwrap();
        let added: Vec<_> = nodes(&root).iter().filter_map(|node| node.index)
            .filter(|index| !retained.contains(index)).collect();
        assert!(!added.is_empty());
        for index in added {
            assert!(!removed.contains(&index), "shell reused removed Delete-chain index {index}");
        }
        assert_links_resolve(&root);
    }

    #[test]
    fn shell_writes_only_the_losses_hooks() {
        for f in fixtures() {
            for &entity in &f.shape.planes {
                let events: Vec<_> = nodes(by_id(&f.root, entity)).into_iter()
                    .filter(|node| node.block_type == "OnEvent").collect();
                assert_eq!(events.len(), 1);
                assert_eq!(template::read_event(events[0]), Some((13, shell_id(&f, "LOSSES"))));
            }
            assert_eq!(f.root.count_block_type("OnEvent"), f.shape.planes.len());
            assert_eq!(f.root.count_block_type("OnReport"), 0);
        }
    }

    #[test]
    fn shell_deletion_frees_the_budget() {
        use crate::airtask::assemble::{assemble_side, tests::{context, finish, test_plan}};
        let mut plan = test_plan();
        plan.pool.truncate(1); // two HVAR copies, one budget slot
        let mut logic = SideLogic::default();
        let mut next = 1;
        let mut copies = assemble_side(AirSide::Nato, &plan, &context(), &mut logic, &mut next).unwrap();
        copies.trees[0].children.extend(crate::airtask::budget::build_side(
            AirSide::Nato, 1, &mut logic, &plan.strategies, &mut next,
        ));
        let root = finish(AirSide::Nato, &mut copies, &logic, &mut next);
        let first = &copies.entries[0].copies[0];
        let second = &copies.entries[0].copies[1];
        let delete = named(&root, "SN01 DELETE").index.unwrap();
        let script = WalkerInput {
            until: ticks(80.0),
            pulses: vec![
                (ticks(2.0), first.pointer.into()),
                (ticks(10.0), first.shell.done_hub.into()),
                (ticks(40.0), second.pointer.into()),
                (ticks(75.0), second.pointer.into()),
            ],
            ..WalkerInput::default()
        };
        for config in configs() {
            let log = walk(&root, config, &script);
            assert_eq!(times(&log, first.shell.start), [ticks(2.0)]);
            assert_eq!(times(&log, first.shell.done_hub), [ticks(10.0)]);
            assert_eq!(times(&log, first.shell.delete_timer), [ticks(70.6)]);
            assert_eq!(times(&log, delete), [ticks(70.6)]);
            assert!(active(&log, first.shell.flying, ticks(70.55)));
            assert!(!active(&log, first.shell.flying, ticks(70.6)));
            assert_eq!(times(&log, second.shell.start), [ticks(75.0)]);
        }
    }

    #[test]
    fn shell_orphan_sweep_keeps_everything_reachable() {
        for i in 0..6 {
            let original = builtin(i);
            let f = fixture(original.clone(), Strategies::default());
            let mut stripped = original;
            let shape = inspect_sortie(&stripped).unwrap();
            strip(&mut stripped, &shape);
            for node in nodes(&stripped)
                .into_iter()
                .filter(|n| n.block_type.starts_with("MCU_"))
            {
                assert!(nodes(&f.root).iter().any(|n| n.index == node.index));
            }
            for id in [
                shape.bring_up_timer,
                shape.end_timer,
                shape.end_orders_timer,
                shape.delayed_end_timer,
                shape.zone_out_activate,
                shape.zone_out_pulse,
            ] {
                assert!(nodes(&f.root).iter().any(|n| n.index == Some(id)));
            }
            assert_links_resolve(&f.root);
        }
    }

    #[test]
    fn shell_refuses_a_second_link_into_the_end_timer() {
        let mut root = builtin(1);
        let shape = inspect_sortie(&root).unwrap();
        by_id_mut(&mut root, shape.navigation_attack_timer)
            .unwrap()
            .append_target(shape.end_timer);
        let before = root.clone();
        let mut next = root.max_index() + 1;
        let initial = next;
        let error = wrap_sortie(
            &mut root,
            "SN01",
            &shape,
            &ShellSpec {
                attack_time_s: 600.0,
                final_time_s: 2100.0,
                extension_s: 600.0,
                target: TARGET,
                census_delay_s: 0.05,
                zone_out: ZoneOutImpl::CheckZone,
                losses: LossesImpl::Type13,
            },
            &mut next,
        )
        .unwrap_err();
        assert!(error.contains("only link"));
        assert_eq!(root, before);
        assert_eq!(next, initial);
    }

    #[test]
    fn shell_zone_out_watchdog_structure() {
        let f = fixture(
            builtin(1),
            Strategies {
                zone_out: ZoneOutImpl::Watchdog,
                ..Strategies::default()
            },
        );
        assert_eq!(
            by_id(&f.root, f.shape.zone_out).property("Closer"),
            Some("1")
        );
        assert_eq!(f.root.count_block_type("MCU_CheckZone"), 2);
        assert_eq!(
            by_id(&f.root, f.shape.zone_out).targets,
            [shell_id(&f, "WATCHDOG")]
        );
        let runtime = runtime(&f, false);
        testkit::walker_every_deactivated_out_has_a_rearm_or_is_single_use(
            &runtime,
            &f.ids.single_use_outs,
        )
        .unwrap();
        for reverse_targets in [false, true] {
            // Watchdog is explicitly gated on J5=Restart, unlike the primary shell.
            let mut script = input(&f, 60.0, Some(10.0));
            script.tracks[0].samples = (0..=30)
                .map(|s| {
                    (
                        ticks(f64::from(s)),
                        TARGET.0 + if s >= 30 { 40000.0 } else { 0.0 },
                        TARGET.1,
                    )
                })
                .collect();
            let log = walk(
                &runtime,
                WalkerConfig {
                    reverse_targets,
                    ..WalkerConfig::default()
                },
                &script,
            );
            assert_eq!(times(&log, f.ids.done_hub), [ticks(35.0)]);
            assert!(!active(
                &log,
                shell_id(&f, "WATCHDOG POLL OUT"),
                ticks(60.0)
            ));
        }
    }

    #[test]
    fn shell_done_fires_once_when_on_station_end_and_zone_out_coincide() {
        let f = fixture(builtin(1), Strategies::default());
        let mut script = input(&f, 800.0, None);
        script
            .pulses
            .push((ticks(10.0), f.shape.navigation_attack_timer.into()));
        // Zone is pulsed outside at the exact expiry of ON STATION.
        script.tracks.clear();
        script.pulses.push((ticks(610.5), f.shape.zone_out.into()));
        for config in configs() {
            let log = walk(&runtime(&f, false), config, &script);
            assert_eq!(times(&log, f.ids.done_hub), [ticks(610.5)]);
            assert_eq!(times(&log, f.shape.end_timer), [ticks(610.6)]);
        }
    }

    #[test]
    fn shell_losses_all_planes_triggers_done() {
        for f in fixtures() {
            let mut script = input(&f, 200.0, None);
            script.kills = f
                .shape
                .planes
                .iter()
                .enumerate()
                .map(|(i, &id)| (ticks(10.0 + i as f64), id.into()))
                .collect();
            for config in configs() {
                let log = walk(&runtime(&f, false), config, &script);
                assert_eq!(
                    times(&log, f.ids.done_hub),
                    [ticks(9.0 + f.shape.planes.len() as f64)]
                );
            }
        }
    }

    #[test]
    fn shell_cold_area_ends_at_on_station_end() {
        let f = fixture(builtin(1), Strategies::default());
        for config in configs() {
            let log = walk(&runtime(&f, false), config, &input(&f, 2200.0, Some(302.0)));
            assert_eq!(times(&log, f.ids.done_hub), [ticks(902.5)]);
            assert_eq!(times(&log, f.ids.delete_timer), [ticks(963.1)]);
            assert!(active(&log, f.ids.flying, ticks(963.05)));
            assert!(!active(&log, f.ids.flying, ticks(963.1)));
        }
    }

    #[test]
    fn shell_empty_target_stays_on_station() {
        let f = fixture(builtin(1), Strategies::default());
        let mut script = input(&f, 2200.0, Some(302.0));
        script.tracks.clear();
        for config in configs() {
            let log = walk(&runtime(&f, false), config, &script);
            assert!(times(&log, f.shape.zone_in).is_empty());
            assert!(times(&log, f.shape.zone_out).is_empty());
            assert_eq!(times(&log, f.ids.cold), [ticks(902.5)]);
            assert_eq!(times(&log, f.ids.done_hub), [ticks(902.5)]);
            assert_eq!(times(&log, f.ids.delete_timer), [ticks(963.1)]);
        }
    }

    fn enemy_window(f: &Fixture, first_s: u32, leave_s: u32) -> WalkerInput {
        let mut script = input(f, 2200.0, Some(302.0));
        // A track is absent before its first sample. Its last sample stays
        // outside the zone; samples in between are exactly one second apart.
        script.tracks[0].samples = (first_s..=leave_s).map(|second| {
            (ticks(f64::from(second)), TARGET.0 + if second == leave_s { 40_000.0 } else { 0.0 }, TARGET.1)
        }).collect();
        script
    }

    #[test]
    fn shell_enemy_present_at_arrival_then_leaves() {
        let f = fixture(builtin(1), Strategies::default());
        let script = enemy_window(&f, 0, 482);
        for config in configs() {
            let log = walk(&runtime(&f, false), config, &script);
            assert_eq!(times(&log, f.shape.zone_in), [ticks(302.0)]);
            assert_eq!(times(&log, f.shape.zone_out_pulse), [ticks(302.1)]);
            assert_eq!(times(&log, f.shape.zone_out), [ticks(482.0)]);
            assert_eq!(times(&log, f.ids.done_hub), [ticks(482.0)]);
            assert_eq!(times(&log, f.ids.cold), [ticks(902.5)]);
        }
    }

    #[test]
    fn shell_enemy_arrives_after_then_leaves() {
        let f = fixture(builtin(1), Strategies::default());
        let script = enemy_window(&f, 422, 602);
        for config in configs() {
            let log = walk(&runtime(&f, false), config, &script);
            assert_eq!(times(&log, f.shape.zone_in), [ticks(422.0)]);
            assert_eq!(times(&log, f.shape.zone_out_pulse), [ticks(422.1)]);
            assert_eq!(times(&log, f.shape.zone_out), [ticks(602.0)]);
            assert_eq!(times(&log, f.ids.done_hub), [ticks(602.0)]);
            assert_eq!(times(&log, f.ids.cold), [ticks(902.5)]);
        }
    }

    #[test]
    fn shell_hot_empty_target_extends_once() {
        let f = fixture(builtin(1), Strategies::default());
        let mut script = input(&f, 2200.0, Some(302.0));
        script.tracks.clear();
        for config in configs() {
            let log = walk(&runtime(&f, true), config, &script);
            assert!(times(&log, f.shape.zone_in).is_empty());
            assert!(times(&log, f.shape.zone_out).is_empty());
            assert!(times(&log, f.ids.cold).is_empty());
            assert_eq!(times(&log, f.ids.hot), [ticks(902.5)]);
            assert_eq!(times(&log, shell_id(&f, "EXTEND")), [ticks(1502.5)]);
            assert_eq!(times(&log, f.ids.done_hub), [ticks(1502.5)]);
            assert_eq!(times(&log, shell_id(&f, "HARD STOP")), [ticks(2102.0)]);
        }
    }

    #[test]
    fn shell_hot_area_extends_once_then_ends() {
        let f = fixture(builtin(1), Strategies::default());
        for config in configs() {
            let log = walk(&runtime(&f, true), config, &input(&f, 2200.0, Some(302.0)));
            assert_eq!(times(&log, f.ids.hot), [ticks(902.5)]);
            assert_eq!(times(&log, shell_id(&f, "EXTEND")), [ticks(1502.5)]);
            assert_eq!(times(&log, f.ids.done_hub), [ticks(1502.5)]);
            assert_eq!(times(&log, shell_id(&f, "HARD STOP")), [ticks(2102.0)]);
        }
    }

    #[test]
    fn shell_late_flight_ends_at_hard_stop() {
        for f in fixtures() {
            let final_time = 1502.0 + f.shape.attack_time_s;
            for config in configs() {
                let log = walk(
                    &runtime(&f, true),
                    config,
                    &input(&f, final_time + 100.0, None),
                );
                assert_eq!(times(&log, f.ids.done_hub), [ticks(final_time)]);
                assert!(times(&log, shell_id(&f, "ON STATION")).is_empty());
                assert!(times(&log, shell_id(&f, "EXTEND")).is_empty());
            }
        }
    }

    #[test]
    fn shell_airalert_on_station_starts_once() {
        let f = fixture(builtin(0), Strategies::default());
        assert_eq!(f.shape.navigation_attack_timer, 39);
        let wp = by_id(&f.root, f.shape.path_waypoints[&f.shape.navigation_lead]);
        assert!(wp.targets.contains(&39));
        assert!(by_id(&f.root, 37).targets.contains(&39));
        for config in configs() {
            let log = walk(&runtime(&f, false), config, &input(&f, 2700.0, Some(10.0)));
            assert_eq!(times(&log, shell_id(&f, "ON STATION IN")).len(), 1);
            assert_eq!(times(&log, shell_id(&f, "ON STATION")).len(), 1);
            assert_eq!(times(&log, f.ids.done_hub).len(), 1);
        }
    }

    #[test]
    fn shell_and_budget_pass_the_rearm_rule() {
        for f in fixtures() {
            let root = runtime(&f, false);
            testkit::walker_every_deactivated_out_has_a_rearm_or_is_single_use(
                &root,
                &f.ids.single_use_outs,
            )
            .unwrap();
        }
        for n in 1..=3 {
            let h =
                crate::airtask::budget::tests::harness(AirSide::Nato, n, 3, Strategies::default());
            testkit::walker_every_deactivated_out_has_a_rearm_or_is_single_use(&h.root, &[])
                .unwrap();
        }
    }
}
