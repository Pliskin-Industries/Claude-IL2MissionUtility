//! One-use sortie shells. Engine assumptions remain pending the P14 probes.

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
    by_id_mut(root, shape.zone_out).unwrap().set_targets(vec![]);
    root.for_each_mut(&mut |node| {
        if node.index.is_some_and(|id| shape.planes.contains(&id)) {
            node.children.retain(|child| child.block_type != "OnEvents");
        }
    });
    remove_ids(
        root,
        &shape.mission_complete_timers.iter().copied().collect(),
    );
    let all: BTreeMap<_, _> = nodes(root)
        .into_iter()
        .filter_map(|node| node.index.map(|id| (id, node)))
        .collect();
    let mut pending = vec![
        shape.bring_up_timer,
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
/// this adds 15 MCUs, and assembly adds COPY r (17 including the new RTB timer).
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
        shape.zone_out,
        shape.zone_out_activate,
        shape.zone_out_pulse,
        shape.end_timer,
        shape.delete_timer,
        shape.navigation_attack_timer,
        shape.path_waypoints[&shape.navigation_lead],
    ]
    .iter()
    .chain(&shape.planes)
    .any(|id| !present.contains(id))
    {
        return Err("Air Tasking shell shape does not match this copy's indexes".into());
    }
    let mut copy = root.clone();
    strip(&mut copy, shape);
    if nodes(&copy).iter().any(|node| {
        node.targets.contains(&shape.end_timer) || node.objects.contains(&shape.end_timer)
    }) {
        return Err("Air Tasking needs DONE HUB to be the only link into the end timer".into());
    }
    let mut alloc = (*next_id).max(copy.max_index() + 1);
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
    ]);
    // Relay single-use latches are the primary J4 hypothesis.
    // P14 G0: default pending flight 0
    by_id_mut(&mut copy, shape.navigation_attack_timer)
        .unwrap()
        .append_target(ids["ON STATION IN"]);
    by_id_mut(&mut copy, shape.delete_timer)
        .unwrap()
        .append_target(ids["LANDED"]);
    let wp = by_id_mut(&mut copy, shape.path_waypoints[&shape.navigation_lead]).unwrap();
    wp.append_target(shape.zone_out_activate);
    wp.append_target(shape.zone_out_pulse);
    let zone = by_id_mut(&mut copy, shape.zone_out).unwrap();
    placement::set_coord(zone, "XPos", spec.target.0);
    placement::set_coord(zone, "ZPos", spec.target.1);
    zone.set_targets(vec![ids["DONE HUB"]]);
    let mut single_use_outs = Vec::new();
    match spec.zone_out {
        ZoneOutImpl::CheckZone => {} // P14 G0: default pending flight 0
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
    for (n, &entity) in shape.planes.iter().enumerate() {
        let destination = match spec.losses {
            LossesImpl::Type4 => ids["LOSSES"], // P14 G0: default pending flight 0
            LossesImpl::Latch => {
                let mut latch = template::timer(
                    &format!("{prefix} LOSS {:02}", n + 1),
                    0.0,
                    &mut alloc,
                    x,
                    z,
                );
                let mut off = template::mcu(
                    "MCU_Deactivate",
                    &format!("{prefix} LOSS {:02} OFF", n + 1),
                    &mut alloc,
                    x,
                    z,
                );
                off.set_targets(vec![latch.index.unwrap()]);
                latch.set_targets(vec![off.index.unwrap(), ids["LOSSES"]]);
                let id = latch.index.unwrap();
                added.extend([latch, off]);
                id
            }
        };
        let entity = by_id_mut(&mut copy, entity).unwrap();
        template::attach_event(entity, 4, destination);
        if spec.losses == LossesImpl::Latch {
            template::attach_event(entity, 2, destination);
            template::attach_event(entity, 0, destination);
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
        delete_timer: shape.delete_timer,
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
                samples: vec![(0, TARGET.0, TARGET.1)],
            }],
            ..WalkerInput::default()
        }
    }

    #[test]
    fn shell_strips_zone_in_and_start() {
        for f in fixtures() {
            for name in [
                "Zone IN",
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
            assert!(
                by_id(&f.root, f.shape.delete_timer)
                    .targets
                    .contains(&f.ids.landed)
            );
            assert_eq!(by_id(&f.root, f.ids.landed).block_type, "MCU_Deactivate");
            assert_eq!(by_id(&f.root, f.ids.landed).targets, [f.ids.flying]);
            assert!(f.logic.budget_events.contains(&f.shape.delete_timer));
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
    fn shell_wp1_arms_zone_out() {
        for f in fixtures() {
            let wp = by_id(&f.root, f.shape.path_waypoints[&f.shape.navigation_lead]);
            assert!(wp.targets.contains(&f.shape.zone_out_activate));
            assert!(wp.targets.contains(&f.shape.zone_out_pulse));
            assert_eq!(by_id(&f.root, f.shape.zone_out).pos_xz(), Some(TARGET));
            assert_eq!(
                number(by_id(&f.root, f.shape.zone_out), "Zone").unwrap(),
                35000.0
            );
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
                assert_eq!(event.property("Type"), Some("4"));
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
            assert_eq!(f.root.count_block_type("MCU_CheckZone"), 1);
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
    fn shell_adds_17_mcus_per_copy() {
        let original = builtin(1);
        let shape = inspect_sortie(&original).unwrap();
        let mut stripped = original.clone();
        strip(&mut stripped, &shape);
        assert_eq!(mcus(&original) - mcus(&stripped), 7);
        let f = fixture(original, Strategies::default());
        assert_eq!(mcus(&f.root) - mcus(&stripped), 17 + 1);
        assert!(f.root.find_by_name("RTB 1").is_some());
    }

    #[test]
    fn shell_keeps_template_rtb_timer() {
        let original = generate(&options(TestedShape::S7));
        let shape = inspect_sortie(&original).unwrap();
        let rtb = shape.rtb_timer.unwrap();
        let mut stripped = original.clone();
        strip(&mut stripped, &shape);
        let f = fixture(original, Strategies::default());
        assert_eq!(f.shape.rtb_timer, Some(rtb));
        assert_eq!(mcus(&f.root) - mcus(&stripped), 16 + 1); // one lead lacked RTB
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
    fn shell_delete_timer_still_targets_delete() {
        for f in fixtures() {
            let targets = &by_id(&f.root, f.shape.delete_timer).targets;
            assert!(
                targets
                    .iter()
                    .any(|&id| by_id(&f.root, id).block_type == "MCU_Delete")
            );
        }
    }

    #[test]
    fn shell_wraps_a_spawn_mode_template() {
        let f = fixture(generate(&options(TestedShape::S5)), Strategies::default());
        assert_eq!(
            by_id(&f.root, f.shape.bring_up_timer).name(),
            Some("SPAWN UNITS")
        );
        assert!(
            by_id(&f.root, f.ids.start)
                .targets
                .contains(&f.shape.bring_up_timer)
        );
        for config in configs() {
            let log = walk(&runtime(&f, false), config, &input(&f, 2000.0, Some(10.0)));
            assert_eq!(
                times(&log, named(&f.root, "Trigger Spawner").index.unwrap()).len(),
                1
            );
            assert_eq!(
                times(&log, named(&f.root, "Trigger Delete").index.unwrap()).len(),
                1
            );
        }
    }

    #[test]
    fn shell_sweeps_repeat_mode_machinery() {
        let f = fixture(generate(&options(TestedShape::S6)), Strategies::default());
        for name in [
            "DeathCount",
            "DeathCount ReActivate",
            "DeathCount Deactivate",
            "Reset Counter",
            "Modifier Set Value",
            "COOLDOWN",
        ] {
            assert!(f.root.find_by_name(name).is_none(), "{name}");
        }
        for name in ["SpawnCount", "Trigger Spawner"] {
            assert!(f.root.find_by_name(name).is_some());
        }
        for event in nodes(&f.root)
            .into_iter()
            .filter(|n| n.block_type == "OnEvent")
        {
            assert_eq!(event.property("Type"), Some("4"));
            assert_eq!(
                event.property("TarId"),
                Some(shell_id(&f, "LOSSES").to_string().as_str())
            );
        }
        for config in configs() {
            let mut script = input(&f, 2000.0, None);
            script.kills.push((ticks(10.0), f.shape.planes[0].into()));
            let log = walk(&runtime(&f, false), config, &script);
            assert_eq!(
                times(&log, named(&f.root, "Trigger Spawner").index.unwrap()).len(),
                1
            );
            assert_eq!(times(&log, f.ids.done_hub), [ticks(10.0)]);
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
                shape.delete_timer,
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
        let f = fixture(builtin(1), Strategies::default());
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
                losses: LossesImpl::Type4,
            },
            &mut next,
        )
        .unwrap_err();
        assert!(error.contains("only link"));
        assert_eq!(root, before);
        assert_eq!(next, initial);
        assert_eq!(f.ids.delete_timer, shape.delete_timer);
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
        assert_eq!(f.root.count_block_type("MCU_CheckZone"), 1);
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
    fn shell_losses_latch_counts_each_plane_once_across_types_4_2_0() {
        let f = fixture(
            builtin(1),
            Strategies {
                losses: LossesImpl::Latch,
                ..Strategies::default()
            },
        );
        let mut script = input(&f, 100.0, None);
        for (i, &entity) in f.shape.planes.iter().enumerate() {
            let hooks: Vec<_> = nodes(by_id(&f.root, entity))
                .into_iter()
                .filter(|n| n.block_type == "OnEvent")
                .collect();
            assert_eq!(
                hooks
                    .iter()
                    .map(|n| n.property("Type").unwrap())
                    .collect::<Vec<_>>(),
                ["4", "2", "0"]
            );
            let latch: i32 = hooks[0].property("TarId").unwrap().parse().unwrap();
            assert!(
                hooks
                    .iter()
                    .all(|n| n.property("TarId").unwrap().parse::<i32>().unwrap() == latch)
            );
            for event in 0..3 {
                script.pulses.push((
                    ticks(10.0 + i as f64 + f64::from(event) * 0.1),
                    latch.into(),
                ));
            }
        }
        for config in configs() {
            let log = walk(&runtime(&f, false), config, &script);
            assert_eq!(times(&log, shell_id(&f, "LOSSES")), [ticks(13.0)]);
            assert_eq!(times(&log, f.ids.done_hub), [ticks(13.0)]);
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
            assert_eq!(times(&log, f.shape.delete_timer), [ticks(963.1)]);
            assert!(active(&log, f.ids.flying, ticks(963.05)));
            assert!(!active(&log, f.ids.flying, ticks(963.1)));
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

#[cfg(test)]
mod probe_writers {
    use super::*;
    use crate::airtask::assemble::{assemble_side, emit_side_logic};
    use crate::airtask::budget;
    use crate::airtask::library::{
        TestedShape, inspect_sortie,
        tests::{builtin, generate, options},
    };
    use crate::airtask::place::{SortiePlacement, place_air_sortie};
    use crate::airtask::probe::K14_ORIGIN;
    use crate::airtask::shell::test_support::{named, ticks, times};
    use crate::airtask::testkit::*;
    use crate::airtask::{
        AirBudget, AirContext, AirPlan, AirSide, PoolEntry, Strategies, TargetRef, TriggerMode,
        builtin_library,
    };
    use crate::{duplicate, locale, parser, serialize, trace};

    fn subtitle(root: &mut Il2Entity, source: i32, text: &str, duration: i32, next: &mut i32) {
        // Same SubtitleInfo layout as probe.rs::Probe::subtitle (private).
        // The shared locale helper below handles both probe generations.
        let mut node = template::mcu("MCU_TR_Subtitle", text, next, K14_ORIGIN.0, K14_ORIGIN.1);
        node.set_property("Enabled", "1");
        node.set_property("Coalitions", "[0,1,2]");
        let mut info = Il2Entity::new("SubtitleInfo");
        for (key, value) in [
            ("Duration", duration),
            ("FontSize", 20),
            ("HAlign", 1),
            ("VAlign", 2),
            ("RColor", 0),
            ("GColor", 100),
            ("BColor", 0),
            (
                "LCText",
                locale::P14_PROBE_TRACE_LCNAME + 10 + node.index.unwrap(),
            ),
        ] {
            info.set_property(key, value.to_string());
        }
        node.children.push(info);
        by_id_mut(root, source)
            .unwrap()
            .append_target(node.index.unwrap());
        root.children.push(node);
    }

    fn at(
        root: &mut Il2Entity,
        begin: i32,
        name: &str,
        seconds: f64,
        targets: Vec<i32>,
        next: &mut i32,
    ) -> i32 {
        let mut node = template::timer(name, seconds, next, K14_ORIGIN.0, K14_ORIGIN.1);
        node.set_targets(targets);
        let id = node.index.unwrap();
        by_id_mut(root, begin).unwrap().append_target(id);
        root.children.push(node);
        id
    }

    fn write_traced(stem: &str, mut root: Il2Entity, sources: Vec<i32>, mut next: i32) {
        assert_links_resolve(&root);
        walker_every_deactivated_out_has_a_rearm_or_is_single_use(&root, &[]).unwrap();
        for &id in &sources {
            assert!(
                matches!(
                    by_id(&root, id).block_type.as_str(),
                    "MCU_Timer" | "MCU_Counter" | "MCU_CheckZone"
                ),
                "trace the pulse feeding a state command, not the command's state links"
            );
        }
        let mut map = trace::TraceMap::default();
        let carrier = trace::TraceCarrier::default(); // P14 G0: default pending flight 0
        trace::instrument(
            &mut root,
            &trace::TraceSelect {
                indexes: sources,
                event_types: vec![4],
                ..trace::TraceSelect::default()
            },
            &mut next,
            carrier,
            &mut map,
        );
        let text = serialize::serialize_group(&root);
        let reparsed = parser::parse_group_file(&text).unwrap();
        assert_links_resolve(&reparsed);
        assert!(!map.entries.is_empty());
        let locale = locale::p14_probe_locale(&root);
        let english = locale::encode_locale_utf16le(&locale::serialize_locale(&locale));
        let dir = std::path::Path::new("target/p14");
        std::fs::create_dir_all(dir).unwrap();
        let group = dir.join(format!("{stem}_traced.Group"));
        let sidecar = group.with_extension("trace.json");
        let eng = group.with_extension("eng");
        std::fs::write(&group, text).unwrap();
        trace::write_trace_sidecar(&sidecar, &map).unwrap();
        std::fs::write(&eng, english).unwrap();
        assert_eq!(trace::read_trace_sidecar(&sidecar).unwrap(), map);
        use std::io::Write;
        let mut output = std::io::stdout().lock();
        for path in [group, sidecar, eng] {
            writeln!(
                output,
                "WROTE {} ({} bytes)",
                path.display(),
                std::fs::metadata(&path).unwrap().len()
            )
            .unwrap();
        }
    }

    fn budget_cell(
        label: &str,
        n: u8,
        t: f64,
        root: &mut Il2Entity,
        next: &mut i32,
        sources: &mut Vec<i32>,
    ) {
        let count = if n == 2 { 7 } else { 5 };
        let h = budget::tests::harness(AirSide::Nato, n, count, Strategies::default());
        let (mut cell, map) = duplicate::duplicate_template(&h.root, next);
        cell.set_name(label);
        cell.for_each_mut(&mut |node| {
            if node.block_type.starts_with("MCU_") {
                if let Some(name) = node.name() {
                    node.set_name(&format!("{label} {name}"));
                }
            }
        });
        let begin = named(&cell, &format!("{label} Translator Mission Begin"))
            .index
            .unwrap();
        let requests = if n == 2 {
            vec![0.0, 2.0, 4.0, 6.0, 32.0, 34.0, 36.0]
        } else {
            vec![0.0, 0.0, 5.0, 22.0, 32.0]
        };
        for (i, offset) in requests.into_iter().enumerate() {
            let request = at(
                &mut cell,
                begin,
                &format!("{label} request {}", i + 1),
                t + offset,
                vec![map[&h.requests[i]]],
                next,
            );
            sources.push(request);
            subtitle(
                &mut cell,
                request,
                &format!("{label}: request {}", i + 1),
                1,
                next,
            );
            let start = map[&h.gates[i]];
            sources.push(start);
            subtitle(
                &mut cell,
                start,
                &format!("{label}: copy {} START", i + 1),
                1,
                next,
            );
        }
        if n == 2 {
            at(
                &mut cell,
                begin,
                "T-k simultaneous deletions",
                t + 30.0,
                vec![map[&h.deletions[0]], map[&h.deletions[1]]],
                next,
            );
        } else {
            at(
                &mut cell,
                begin,
                "T-k1 first deletion",
                t + 20.0,
                vec![map[&h.deletions[0]]],
                next,
            );
            // Delete the other possible starters too: under DepthFirst the
            // fourth request can start after copy 1 has left the head count.
            at(
                &mut cell,
                begin,
                "T-k1 remaining deletions",
                t + 30.0,
                h.deletions[1..4].iter().map(|id| map[id]).collect(),
                next,
            );
        }
        for (i, &deletion) in h.deletions.iter().enumerate() {
            sources.push(map[&deletion]);
            subtitle(
                &mut cell,
                map[&deletion],
                &format!("{label}: copy {} deleted", i + 1),
                1,
                next,
            );
        }
        sources.extend(h.flags.iter().map(|id| map[id]));
        for suffix in ["CHECK", "CENSUS", "COUNT", "END", "REOPEN OUT", "AGAIN"] {
            if let Some(node) = cell.find_by_name(&format!("{label} BUD N {suffix}")) {
                sources.push(node.index.unwrap());
            }
        }
        for delivery in [SameTickDelivery::DepthFirst, SameTickDelivery::Batched] {
            let log = walk(
                &cell,
                WalkerConfig {
                    same_tick_delivery: delivery,
                    ..WalkerConfig::default()
                },
                &WalkerInput {
                    until: ticks(t + 40.0),
                    ..WalkerInput::default()
                },
            );
            if n == 2 {
                for (i, expected) in [1, 1, 0, 0, 1, 1, 0].into_iter().enumerate() {
                    assert_eq!(times(&log, map[&h.gates[i]]).len(), expected);
                }
            } else {
                assert_eq!(times(&log, map[&h.gates[0]]).len(), 1);
                assert_eq!(
                    times(&log, map[&h.gates[1]]).len(),
                    usize::from(delivery == SameTickDelivery::Batched)
                );
                assert!(times(&log, map[&h.gates[2]]).is_empty());
                assert_eq!(
                    times(&log, map[&h.gates[3]]).len(),
                    usize::from(delivery == SameTickDelivery::DepthFirst)
                );
                assert_eq!(times(&log, map[&h.gates[4]]).len(), 1);
            }
        }
        root.children.push(cell);
    }

    #[test]
    #[ignore = "writes target/p14 Probe 2 sortie, budget and trace files"]
    fn write_p14_sortie_probe() {
        let mut next = 1;
        let mut root = group("P14 Sortie Probe", &mut next);
        root.set_property("Desc", "\"Probe 2: join NATO at K14 and remain within 35 km of the targets. Probe-only Zone Out watches NATO so one ground observer can keep the sortie alive.\"");
        let mut logic = SideLogic::default();
        let mut sources = Vec::new();
        let mut observations = Vec::new();
        let mut arrivals = Vec::new();
        for (i, (source, start_s)) in [
            (builtin(1), 60.0),
            (builtin(0), 90.0),
            (generate(&options(TestedShape::S5)), 120.0),
        ]
        .into_iter()
        .enumerate()
        {
            let mut source = source;
            let target = (K14_ORIGIN.0 + 10_000.0, K14_ORIGIN.1 + i as f64 * 6000.0);
            let placed = place_air_sortie(
                &mut source,
                &SortiePlacement {
                    target,
                    heading_deg: 0.0,
                    spawn_dist_m: 55_000.0,
                    rtb: K14_ORIGIN,
                },
            )
            .unwrap();
            let (mut copy, _) = duplicate::duplicate_template(&source, &mut next);
            let shape = inspect_sortie(&copy).unwrap();
            let prefix = format!("SN{:02}", i + 1);
            copy.set_name(&format!("{prefix} Probe 2"));
            let ids = wrap_sortie(
                &mut copy,
                &prefix,
                &shape,
                &ShellSpec {
                    attack_time_s: placed.attack_time_s,
                    final_time_s: placed.attack_time_s + 1500.0,
                    extension_s: 600.0,
                    target,
                    census_delay_s: budget::census_delay(Strategies::default().census, i + 1),
                    zone_out: ZoneOutImpl::CheckZone, // P14 G0: default pending flight 0
                    losses: LossesImpl::Type4,        // P14 G0: default pending flight 0
                },
                &mut next,
            )
            .unwrap();
            ids.register(i + 1, &mut logic);
            // This is only the observation mission. Production copies keep
            // the original opposing coalition, verified by the shell tests.
            by_id_mut(&mut copy, shape.zone_out)
                .unwrap()
                .set_property("PlaneCoalitions", "[2]");
            let mut launch = template::timer(
                &format!("{prefix} launch at {start_s}"),
                start_s,
                &mut next,
                target.0,
                target.1,
            );
            launch.set_targets(vec![ids.start]);
            logic.mission_begin.push(launch.index.unwrap());
            sources.push(launch.index.unwrap());
            copy.children.push(launch);
            let mut hot = template::mcu(
                "MCU_Activate",
                &format!("PROBE {prefix} HOT ON"),
                &mut next,
                target.0,
                target.1,
            );
            hot.set_targets(vec![ids.hot]);
            let mut cold_off = template::mcu(
                "MCU_Deactivate",
                &format!("PROBE {prefix} COLD OFF"),
                &mut next,
                target.0,
                target.1,
            );
            cold_off.set_targets(vec![ids.cold]);
            let mut hot_time = template::timer(
                &format!("PROBE {prefix} HOT AT"),
                600.0,
                &mut next,
                target.0,
                target.1,
            );
            let mut cold_time = template::timer(
                &format!("PROBE {prefix} COLD OFF AT"),
                600.05,
                &mut next,
                target.0,
                target.1,
            );
            hot_time.set_targets(vec![hot.index.unwrap()]);
            cold_time.set_targets(vec![cold_off.index.unwrap()]);
            logic
                .mission_begin
                .extend([hot_time.index.unwrap(), cold_time.index.unwrap()]);
            copy.children.extend([hot, cold_off, hot_time, cold_time]);
            let extend = named(&copy, &format!("{prefix} EXTEND")).index.unwrap();
            let delete = named(&copy, "Trigger Delete").index.unwrap();
            // Commands are sinks in the walker; observe the timer in the same
            // tick as Trigger Delete, rather than pulse Targets on the command.
            for (id, label) in [
                (ids.start, "START"),
                (ids.done_hub, "DONE HUB"),
                (extend, "EXTEND expired"),
                (ids.hot, "EXTEND begins"),
                (ids.delete_timer, "Trigger Delete"),
            ] {
                sources.push(id);
                subtitle(&mut copy, id, &format!("{prefix}: {label}"), 1, &mut next);
            }
            assert!(by_id(&copy, ids.delete_timer).targets.contains(&delete));
            arrivals.push((
                ticks(start_s + 300.0),
                shape.path_waypoints[&shape.navigation_lead].into(),
            ));
            observations.push((ids, placed.attack_time_s, start_s));
            root.children.push(copy);
        }
        // Three timed airborne demonstrations, independently of T-k's N=2.
        root.children.extend(budget::build_side(
            AirSide::Nato,
            3,
            &mut logic,
            &Strategies::default(),
            &mut next,
        ));
        let init = emit_side_logic(
            AirSide::Nato,
            &logic,
            std::slice::from_mut(&mut root),
            &mut next,
        )
        .unwrap();
        root.children.extend(init);
        let log = walk(
            &root,
            WalkerConfig::default(),
            &WalkerInput {
                until: ticks(2600.0),
                arrivals,
                tracks: vec![ScriptedTrack {
                    coalition: 2,
                    samples: vec![(0, K14_ORIGIN.0, K14_ORIGIN.1)],
                }],
                ..WalkerInput::default()
            },
        );
        for (ids, attack, start) in observations {
            assert_eq!(times(&log, ids.start), [ticks(start)]);
            assert_eq!(times(&log, ids.done_hub).len(), 1);
            let done = times(&log, ids.done_hub)[0];
            assert!((done as f64 / 20.0 - (start + 300.5 + attack + 600.0)).abs() <= 0.5);
            assert_eq!(times(&log, ids.delete_timer), [done + ticks(60.6)]);
        }
        budget_cell("T-k", 2, 180.0, &mut root, &mut next, &mut sources);
        budget_cell("T-k1", 1, 240.0, &mut root, &mut next, &mut sources);
        write_traced("P14_Sortie_Probe", root, sources, next);
    }

    #[test]
    #[ignore = "writes target/p14 Probe 2 load file with 24 copies per side"]
    fn write_p14_load_probe() {
        let nato = [(150_000.0, 250_000.0), (155_000.0, 280_000.0)];
        let dprk = [(180_000.0, 250_000.0), (185_000.0, 280_000.0)];
        let ctx = AirContext {
            nato_objectives: &nato,
            east_objectives: &dprk,
            ..crate::airtask::assemble::tests::context()
        };
        let mut plan = AirPlan {
            library: builtin_library(),
            budget: AirBudget::default(),
            seed: 7,
            ..AirPlan::default()
        };
        // Step 11's six entry roles. NATO uses six runs; repeat the two DPRK
        // entries to reach the same 24-copy load without exceeding runs<=6.
        for (index, area, mode) in [
            (0, 0, TriggerMode::Clock),
            (1, 1, TriggerMode::Clock),
            (2, 0, TriggerMode::Zone),
            (3, 1, TriggerMode::Both),
            (4, 0, TriggerMode::Zone),
            (5, 1, TriggerMode::Both),
            (4, 0, TriggerMode::Zone),
            (5, 1, TriggerMode::Both),
        ] {
            let sortie = &plan.library[index];
            let target = if sortie.side == AirSide::Nato {
                nato[area]
            } else {
                dprk[area]
            };
            let mut entry = PoolEntry::new(
                sortie.id.clone(),
                TargetRef::Objective {
                    side: sortie.side,
                    xz: target,
                },
            );
            entry.mode = mode;
            entry.runs = 6;
            plan.pool.push(entry);
        }
        let mut next = 1;
        let mut root = group("P14 Load Probe", &mut next);
        let mut sources = Vec::new();
        for side in [AirSide::Dprk, AirSide::Nato] {
            let mut logic = SideLogic::default();
            let mut copies = assemble_side(side, &plan, &ctx, &mut logic, &mut next).unwrap();
            assert_eq!(logic.gates.len(), 24);
            for entry in &copies.entries {
                for copy in &entry.copies {
                    sources.extend([
                        copy.shell.start,
                        copy.shell.done_hub,
                        copy.shell.delete_timer,
                    ]);
                }
            }
            copies.trees[0].children.extend(budget::build_side(
                side,
                2,
                &mut logic,
                &plan.strategies,
                &mut next,
            ));
            let init = emit_side_logic(side, &logic, &mut copies.trees, &mut next).unwrap();
            copies.trees[0].children.extend(init);
            root.children.extend(copies.trees);
        }
        let mut begin = template::mcu(
            "MCU_TR_MissionBegin",
            "LOAD PROBE CUE",
            &mut next,
            K14_ORIGIN.0,
            K14_ORIGIN.1,
        );
        begin.set_property("Enabled", "1");
        let begin_id = begin.index.unwrap();
        root.children.push(begin);
        let cue = at(
            &mut root,
            begin_id,
            "LOAD PROBE READY",
            1.0,
            vec![],
            &mut next,
        );
        subtitle(
            &mut root,
            cue,
            "P14 load probe: 24 copies per side, idle. Record load time and server tick delay.",
            20,
            &mut next,
        );
        sources.push(cue);
        assert_eq!(root.count_block_type("Plane"), 240);
        let log = walk(
            &root,
            WalkerConfig::default(),
            &WalkerInput {
                until: ticks(10.0),
                ..WalkerInput::default()
            },
        );
        assert!(!log.pulses.iter().any(|p| p.name.ends_with(" START")));
        write_traced("P14_Load_Probe", root, sources, next);
    }
}
