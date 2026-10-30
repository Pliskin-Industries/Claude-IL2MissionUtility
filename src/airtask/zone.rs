//! Threshold PASS subscribers: one chance per arming, then a cooldown (0030 C).

use std::collections::BTreeMap;

use super::assemble::{EntryCopies, SideCopies};
use super::shell::{by_id_mut, group};
use super::{
    AirIssue, AirPlan, AirSide, IssueSeverity, SideLogic, TriggerMode, budget, clock, timing,
};
use crate::ast::Il2Entity;
use crate::template;

const CHANCE_BASE_S: f64 = 0.1;
const MISS_WAIT_BASE_S: f64 = 1.0;
const FIRE_DELAY_S: f64 = 0.05;
const DENY_WAIT_S: f64 = 1.0;

fn zone_entries(entries: &[EntryCopies]) -> impl Iterator<Item = &EntryCopies> {
    entries
        .iter()
        .filter(|entry| matches!(entry.mode, TriggerMode::Zone | TriggerMode::Both))
}

// Construction and validation share the per-area stagger, including all copies.
fn zone_staggers(side: AirSide, plan: &AirPlan, entries: &[EntryCopies]) -> Vec<f64> {
    let count = entries.iter().map(|entry| entry.copies.len()).sum();
    let spacing = budget::recount_time(plan.strategies.census, count) + 0.2;
    let eligible: Vec<_> = zone_entries(entries).collect();
    let mut positions = BTreeMap::new();
    let mut staggers = vec![0.0; eligible.len()];
    for i in clock::waterfall_order(plan.seed, side, eligible.len()) {
        let position = positions.entry(eligible[i].area).or_insert(0);
        staggers[i] = *position as f64 * spacing;
        *position += 1;
    }
    staggers
}

/// Zone diagnostics use assembly's eligible entries and shared areas.
/// Step 7's validate will call this per side before building the mission.
pub(crate) fn validate_zone(
    side: AirSide,
    plan: &AirPlan,
    entries: &[EntryCopies],
) -> Vec<AirIssue> {
    if zone_entries(entries).next().is_none() {
        return Vec::new();
    }
    let error = |message: &str| AirIssue {
        severity: IssueSeverity::Error,
        text: format!("Air Tasking {} zone: {message}", side.label()),
        rail_only: false,
    };
    let mut issues = Vec::new();
    if !(1..=100).contains(&plan.zone.chance_pct) {
        issues.push(error("chance must be 1 to 100 %"));
    }
    let cooldown = plan.zone.cooldown_min * 60.0;
    if !plan.zone.cooldown_min.is_finite()
        || plan.zone.cooldown_min <= 0.0
        || !cooldown.is_finite()
        || cooldown <= 0.0
    {
        issues.push(error(
            "cooldown must be a positive number of minutes, finite in both minutes and seconds",
        ));
    }
    let longest_stagger = zone_staggers(side, plan, entries)
        .into_iter()
        .fold(0.0, f64::max);
    let minimum = MISS_WAIT_BASE_S + longest_stagger + 1.0;
    let hold = timing::derived_timing(&plan.threshold).hold_s;
    if !hold.is_finite() {
        issues.push(error(&format!(
            "the threshold hold time ({hold:.1} s) must be a finite number of seconds",
        )));
    } else if hold <= minimum {
        issues.push(error(&format!(
            "the threshold hold time ({hold:.1} s) must be longer than {minimum:.1} s, the zone's longest wait on one area plus 1 s",
        )));
    }
    issues
}

/// Return one Zone group for copies.trees[0]. Stage allocation and wiring;
/// copy pointers, lifecycle MCUs and exhaustion belong to assemble_side.
pub(crate) fn build_side(
    side: AirSide,
    plan: &AirPlan,
    copies: &SideCopies,
    logic: &mut SideLogic,
    next_id: &mut i32,
) -> Result<Vec<Il2Entity>, String> {
    let entries: Vec<_> = zone_entries(&copies.entries).collect();
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(issue) = validate_zone(side, plan, &copies.entries)
        .into_iter()
        .find(|issue| issue.severity == IssueSeverity::Error)
    {
        return Err(issue.text);
    }
    let cooldown = plan.zone.cooldown_min * 60.0;
    let staggers = zone_staggers(side, plan, &copies.entries);

    let mut alloc = *next_id;
    let mut registry = logic.clone();
    let mut root = group("Zone", &mut alloc);
    for (entry, stagger) in entries.iter().zip(staggers) {
        let mut ids = BTreeMap::new();
        for (role, kind, time) in [
            ("PASS IN", "MCU_Timer", 0.0),
            ("CHANCE", "MCU_Timer", CHANCE_BASE_S + stagger),
            ("MISS WAIT", "MCU_Timer", MISS_WAIT_BASE_S + stagger),
            ("MISS WAIT OUT", "MCU_Timer", 0.0),
            ("DISARM", "MCU_Deactivate", 0.0),
            ("FIRE ARM", "MCU_Activate", 0.0),
            ("FIRE", "MCU_Timer", FIRE_DELAY_S),
            ("DENY WAIT", "MCU_Timer", DENY_WAIT_S),
            ("GRANTED", "MCU_Deactivate", 0.0),
            ("WAITING", "MCU_Timer", 0.0),
            ("WAITING OFF", "MCU_Deactivate", 0.0),
            ("COOLDOWN", "MCU_Timer", cooldown),
            ("ARM", "MCU_Activate", 0.0),
        ] {
            let name = format!("Z{}{:02} {role}", side.letter(), entry.number);
            let node = if role == "CHANCE" {
                template::timer_random(&name, time, plan.zone.chance_pct, &mut alloc, 0.0, 0.0)
            } else if kind == "MCU_Timer" {
                template::timer(&name, time, &mut alloc, 0.0, 0.0)
            } else {
                template::mcu(kind, &name, &mut alloc, 0.0, 0.0)
            };
            ids.insert(role, node.index.unwrap());
            root.children.push(node);
        }
        // Start the refusal timer before requesting; START's GRANTED switches
        // it off, which pauses it. The next FIRE ARM resumes it, and FIRE
        // restarts it 0.05 s later, long before the paused second runs out.
        // Last-first pointers cannot consume the next run activated by START.
        let mut fire = vec![ids["DENY WAIT"]];
        fire.extend(entry.copies.iter().rev().map(|copy| copy.pointer));
        // Switching a running timer off only pauses it (flight 2K), so CHANCE
        // and MISS WAIT are never switched. PASS IN drops passes while the zone
        // is disarmed; MISS WAIT runs out into MISS WAIT OUT, which is off
        // after a won roll (the D11 relay, decision 0044).
        let gates = vec![ids["PASS IN"], ids["MISS WAIT OUT"]];
        for (from, to) in [
            ("PASS IN", vec![ids["CHANCE"], ids["MISS WAIT"]]),
            ("CHANCE", vec![ids["FIRE ARM"], ids["DISARM"], ids["FIRE"]]),
            ("MISS WAIT", vec![ids["MISS WAIT OUT"]]),
            ("MISS WAIT OUT", vec![ids["COOLDOWN"], ids["DISARM"]]),
            ("DISARM", gates.clone()),
            ("FIRE ARM", vec![ids["WAITING"], ids["DENY WAIT"]]),
            ("FIRE", fire),
            ("DENY WAIT", vec![ids["WAITING"]]),
            ("GRANTED", vec![ids["DENY WAIT"]]),
            ("WAITING", vec![ids["WAITING OFF"], ids["COOLDOWN"]]),
            ("WAITING OFF", vec![ids["WAITING"]]),
            ("COOLDOWN", vec![ids["ARM"]]),
            ("ARM", gates),
        ] {
            by_id_mut(&mut root, ids[from]).unwrap().set_targets(to);
        }
        registry
            .pass_subscribers
            .entry(entry.area)
            .or_default()
            .push(ids["PASS IN"]);
        registry.init_off.push(ids["WAITING"]);
        for copy in &entry.copies {
            registry.links.extend([
                (copy.shell.start, ids["GRANTED"]),
                (copy.shell.done_hub, ids["WAITING"]),
            ]);
        }
        registry.links.extend(
            ["PASS IN", "MISS WAIT OUT", "COOLDOWN"].map(|role| (entry.exhausted, ids[role])),
        );
    }
    *logic = registry;
    *next_id = alloc;
    Ok(vec![root])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airtask::assemble::{assemble_side, tests::context};
    use crate::airtask::clock::tests::{
        Harness, assembled, configs, entry_id, harness, loiter, starts,
    };
    use crate::airtask::library::{by_id, nodes, number};
    use crate::airtask::shell::test_support::{active, mcus, named, ticks, times};
    use crate::airtask::testkit::*;
    use crate::airtask::{
        CensusImpl, NATO, PoolEntry, Schedule, TargetRef, builtin_library, threshold,
    };
    use std::collections::BTreeSet;

    const PASS: f64 = 61.1; // Entry at 1 s, default dwell 60 s, INNER PULSE 0.1 s.
    // Later passes come exactly HOLD apart: CHECK ON switches the armed INNER
    // on, and that is a check (flight 2K, K5A).
    const HOLD: f64 = 120.0;
    const COOLDOWN: f64 = 900.0;

    fn zone_plan(n: usize, mode: TriggerMode) -> AirPlan {
        let mut plan = AirPlan {
            library: builtin_library(),
            seed: 7,
            ..AirPlan::default()
        };
        for _ in 0..n {
            let mut entry = PoolEntry::new(
                plan.library[1].id.clone(),
                TargetRef::Objective {
                    side: AirSide::Nato,
                    xz: (150_000.0, 260_000.0),
                },
            );
            entry.mode = mode;
            plan.pool.push(entry);
        }
        plan
    }

    fn certain_plan(n: usize, mode: TriggerMode) -> AirPlan {
        let mut plan = zone_plan(n, mode);
        plan.zone.chance_pct = 100;
        // Retain the real clock and its GO paths; scenarios script its requests.
        plan.schedule[NATO].first_slot_min = 1000.0;
        plan
    }

    fn id(h: &Harness, name: &str) -> i32 {
        named(&h.root, name).index.unwrap()
    }

    fn zone_id(h: &Harness, entry: usize, role: &str) -> i32 {
        id(h, &format!("ZN{entry:02} {role}"))
    }

    fn run(
        h: &Harness,
        config: WalkerConfig,
        seed: u64,
        until: f64,
        pulses: &[(f64, i32)],
    ) -> WalkerLog {
        walk(
            &h.root,
            config,
            &WalkerInput {
                seed,
                until: ticks(until),
                tracks: loiter(h, &(1..=h.copies.areas.len()).collect::<Vec<_>>()),
                pulses: pulses
                    .iter()
                    .map(|&(t, id)| (ticks(t), id.into()))
                    .collect(),
                ..WalkerInput::default()
            },
        )
    }

    fn assert_times(h: &Harness, log: &WalkerLog, name: &str, expected: &[f64]) {
        assert_eq!(
            times(log, id(h, name)),
            expected.iter().map(|&t| ticks(t)).collect::<Vec<_>>(),
            "{name}"
        );
    }

    fn assert_zone_times(h: &Harness, log: &WalkerLog, entry: usize, role: &str, expected: &[f64]) {
        assert_times(h, log, &format!("ZN{entry:02} {role}"), expected);
    }

    #[test]
    fn zone_structure() {
        let plan = zone_plan(1, TriggerMode::Zone);
        let (copies, mut logic, mut next) = assembled(&plan);
        let before = logic.clone();
        let root = build_side(AirSide::Nato, &plan, &copies, &mut logic, &mut next)
            .unwrap()
            .remove(0);
        assert_eq!(root.name(), Some("Zone"));
        let zid = |role: &str| named(&root, &format!("ZN01 {role}")).index.unwrap();
        for (role, time, random) in [
            ("PASS IN", 0.0, 100.0),
            ("CHANCE", 0.1, 50.0),
            ("MISS WAIT", 1.0, 100.0),
            ("MISS WAIT OUT", 0.0, 100.0),
            ("FIRE", 0.05, 100.0),
            ("DENY WAIT", 1.0, 100.0),
            ("WAITING", 0.0, 100.0),
            ("COOLDOWN", 900.0, 100.0),
        ] {
            let node = by_id(&root, zid(role));
            assert_eq!(node.block_type, "MCU_Timer");
            assert_eq!(number(node, "Time"), Ok(time));
            assert_eq!(number(node, "Random"), Ok(random));
        }
        let mut fire = vec![zid("DENY WAIT")];
        fire.extend(
            copies.entries[0]
                .copies
                .iter()
                .rev()
                .map(|copy| copy.pointer),
        );
        let gates = vec![zid("PASS IN"), zid("MISS WAIT OUT")];
        for (role, kind, targets) in [
            (
                "PASS IN",
                "MCU_Timer",
                vec![zid("CHANCE"), zid("MISS WAIT")],
            ),
            (
                "CHANCE",
                "MCU_Timer",
                vec![zid("FIRE ARM"), zid("DISARM"), zid("FIRE")],
            ),
            ("MISS WAIT", "MCU_Timer", vec![zid("MISS WAIT OUT")]),
            (
                "MISS WAIT OUT",
                "MCU_Timer",
                vec![zid("COOLDOWN"), zid("DISARM")],
            ),
            ("DISARM", "MCU_Deactivate", gates.clone()),
            (
                "FIRE ARM",
                "MCU_Activate",
                vec![zid("WAITING"), zid("DENY WAIT")],
            ),
            ("FIRE", "MCU_Timer", fire),
            ("DENY WAIT", "MCU_Timer", vec![zid("WAITING")]),
            ("GRANTED", "MCU_Deactivate", vec![zid("DENY WAIT")]),
            (
                "WAITING",
                "MCU_Timer",
                vec![zid("WAITING OFF"), zid("COOLDOWN")],
            ),
            ("WAITING OFF", "MCU_Deactivate", vec![zid("WAITING")]),
            ("COOLDOWN", "MCU_Timer", vec![zid("ARM")]),
            ("ARM", "MCU_Activate", gates),
        ] {
            let node = by_id(&root, zid(role));
            assert_eq!(node.block_type, kind, "{role}");
            assert_eq!(node.targets, targets, "{role}");
            assert!(node.objects.is_empty());
            assert_eq!(node.pos_xz(), Some((0.0, 0.0)));
        }
        assert_eq!(logic.pass_subscribers[&1], [zid("PASS IN")]);
        assert_eq!(&logic.init_off[before.init_off.len()..], [zid("WAITING")]);
        assert_eq!(logic.mission_begin, before.mission_begin);
        for role in ["PASS IN", "CHANCE", "MISS WAIT", "MISS WAIT OUT"] {
            assert!(!logic.init_off.contains(&zid(role)), "{role}");
        }
        let cells = threshold::build_side(
            AirSide::Nato,
            &copies.areas,
            &plan.threshold,
            &plan.strategies,
            &mut logic,
            &mut next,
        )
        .unwrap();
        let pass = cells
            .iter()
            .find(|node| node.name() == Some("THN01 INNER"))
            .unwrap();
        assert_eq!(&pass.targets[4..], [zid("PASS IN")]);
    }

    #[test]
    fn zone_has_13_mcus_per_entry() {
        for side in [AirSide::Nato, AirSide::Dprk] {
            for n in 1..=4 {
                let mut plan = zone_plan(n, TriggerMode::Both);
                if side == AirSide::Dprk {
                    for entry in &mut plan.pool {
                        entry.sortie_id = plan.library[5].id.clone();
                        entry.target = TargetRef::Objective {
                            side,
                            xz: (180_000.0, 260_000.0),
                        };
                    }
                }
                for debug in [false, true] {
                    plan.debug_subtitles = debug;
                    let mut logic = SideLogic::default();
                    let mut next = 1;
                    let copies =
                        assemble_side(side, &plan, &context(), &mut logic, &mut next).unwrap();
                    let start = next;
                    let output = build_side(side, &plan, &copies, &mut logic, &mut next).unwrap();
                    assert_eq!(output.len(), 1);
                    let root = &output[0];
                    assert_eq!(mcus(root), 13 * n); // No additional per-side MCU.
                    assert_eq!(root.children.len(), 13 * n);
                    assert_eq!(next - start, 1 + 13 * n as i32); // One group ID.
                    for entry in &copies.entries {
                        let prefix = format!("Z{}{:02} ", side.letter(), entry.number);
                        assert_eq!(
                            root.children
                                .iter()
                                .filter(|node| node.name().unwrap().starts_with(&prefix))
                                .count(),
                            13
                        );
                    }
                    assert!(root.children.iter().all(|node| matches!(
                        node.block_type.as_str(),
                        "MCU_Timer" | "MCU_Activate" | "MCU_Deactivate"
                    )));
                    assert!(
                        root.children
                            .iter()
                            .all(|node| !node.name().unwrap().contains("EXHAUSTED"))
                    );
                }
            }
        }
    }

    #[test]
    fn zone_stagger_per_area() {
        let mut plan = zone_plan(3, TriggerMode::Zone);
        let (copies, mut logic, mut next) = assembled(&plan);
        let root = build_side(AirSide::Nato, &plan, &copies, &mut logic, &mut next)
            .unwrap()
            .remove(0);
        assert_eq!(budget::recount_time(plan.strategies.census, 9), 0.6);
        for (p, i) in clock::waterfall_order(plan.seed, AirSide::Nato, 3)
            .into_iter()
            .enumerate()
        {
            for (role, expected) in [
                ("CHANCE", [0.1, 0.9, 1.7][p]),
                ("MISS WAIT", [1.0, 1.8, 2.6][p]),
            ] {
                assert_eq!(
                    number(
                        named(&root, &format!("ZN{:02} {role}", copies.entries[i].number)),
                        "Time"
                    ),
                    Ok(expected)
                );
            }
        }
        // Area positions reset, and Clock-only copies still contribute to C.
        plan.pool[1].mode = TriggerMode::Clock;
        plan.pool.push(plan.pool[0].clone());
        plan.pool[2].target = TargetRef::Objective {
            side: AirSide::Nato,
            xz: (155_000.0, 280_000.0),
        };
        for (census, expected) in [
            (CensusImpl::Spaced, [(0.1, 1.0), (0.1, 1.0), (1.05, 1.95)]),
            (CensusImpl::SameTick, [(0.1, 1.0), (0.1, 1.0), (0.5, 1.4)]),
        ] {
            plan.strategies.census = census;
            let (copies, mut logic, mut next) = assembled(&plan);
            let root = build_side(AirSide::Nato, &plan, &copies, &mut logic, &mut next)
                .unwrap()
                .remove(0);
            let eligible = [&copies.entries[0], &copies.entries[2], &copies.entries[3]];
            // Seed 7 orders eligible indices [1, 0, 2]: entry 3 is on its
            // own area; entries 1 and 4 share the other area's two positions.
            for (entry, (chance, miss)) in eligible.into_iter().zip(expected) {
                for (role, expected) in [("CHANCE", chance), ("MISS WAIT", miss)] {
                    let actual = number(
                        named(&root, &format!("ZN{:02} {role}", entry.number)),
                        "Time",
                    )
                    .unwrap();
                    assert!((actual - expected).abs() < 1e-9);
                }
            }
            assert!(root.find_by_name("ZN02 CHANCE").is_none());
        }
    }

    #[test]
    fn zone_fire_lists_copy_pointers_last_run_first() {
        let mut plan = zone_plan(1, TriggerMode::Both);
        for runs in 1..=6 {
            plan.pool[0].runs = runs;
            let h = harness(&plan, 60.0);
            let mut expected = vec![zone_id(&h, 1, "DENY WAIT")];
            expected.extend(
                h.copies.entries[0]
                    .copies
                    .iter()
                    .rev()
                    .map(|copy| copy.pointer),
            );
            assert_eq!(named(&h.root, "ZN01 FIRE").targets, expected);
        }
    }

    #[test]
    fn zone_last_copy_deactivates_its_gates_and_cooldown() {
        let h = harness(&zone_plan(2, TriggerMode::Both), 60.0);
        for entry in &h.copies.entries {
            for role in ["PASS IN", "MISS WAIT OUT", "COOLDOWN"] {
                let target = zone_id(&h, entry.number, role);
                assert!(h.logic.links.contains(&(entry.exhausted, target)));
                assert!(by_id(&h.root, entry.exhausted).targets.contains(&target));
            }
            for copy in &entry.copies {
                for (source, role) in [
                    (copy.shell.start, "GRANTED"),
                    (copy.shell.done_hub, "WAITING"),
                ] {
                    let target = zone_id(&h, entry.number, role);
                    assert!(h.logic.links.contains(&(source, target)));
                    assert!(by_id(&h.root, source).targets.contains(&target));
                }
            }
        }
    }

    #[test]
    fn zone_ignores_clock_only_entries() {
        let mut plan = zone_plan(3, TriggerMode::Zone);
        plan.pool[0].mode = TriggerMode::Clock;
        plan.pool[2].mode = TriggerMode::Both;
        let h = harness(&plan, 60.0);
        assert!(h.root.find_by_name("ZN01 CHANCE").is_none());
        assert_eq!(mcus(named(&h.root, "Zone")), 26);
        assert_eq!(
            h.logic.pass_subscribers[&1],
            [zone_id(&h, 2, "PASS IN"), zone_id(&h, 3, "PASS IN")]
        );
    }

    #[test]
    fn zone_side_without_zone_entries_emits_nothing() {
        for n in [0, 2] {
            let mut plan = zone_plan(n, TriggerMode::Clock);
            // Unused zone settings do not prevent a Clock-only side building.
            plan.zone.chance_pct = 0;
            plan.zone.cooldown_min = f64::NAN;
            plan.threshold.hold_s = Some(f64::NAN);
            let (copies, mut logic, mut next) = assembled(&plan);
            assert!(validate_zone(AirSide::Nato, &plan, &copies.entries).is_empty());
            let before = (logic.clone(), next);
            assert!(
                build_side(AirSide::Nato, &plan, &copies, &mut logic, &mut next)
                    .unwrap()
                    .is_empty()
            );
            assert_eq!((logic, next), before);
        }
    }

    #[test]
    fn zone_refuses_invalid_input() {
        let plan = zone_plan(3, TriggerMode::Both);
        let (copies, original_logic, original_next) = assembled(&plan);
        let mut invalid = Vec::new();
        for chance in [0, 101] {
            let mut bad = plan.clone();
            bad.zone.chance_pct = chance;
            invalid.push((bad, "chance must be 1 to 100 %"));
        }
        for cooldown in [
            0.0,
            -0.0,
            -1.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
        ] {
            let mut bad = plan.clone();
            bad.zone.cooldown_min = cooldown;
            invalid.push((
                bad,
                "cooldown must be a positive number of minutes, finite in both minutes and seconds",
            ));
        }
        for (hold, expected) in [
            (
                3.5,
                "the threshold hold time (3.5 s) must be longer than 3.6 s, the zone's longest wait on one area plus 1 s",
            ),
            (
                3.6,
                "the threshold hold time (3.6 s) must be longer than 3.6 s, the zone's longest wait on one area plus 1 s",
            ),
            (
                f64::NAN,
                "the threshold hold time (NaN s) must be a finite number of seconds",
            ),
            (
                f64::INFINITY,
                "the threshold hold time (inf s) must be a finite number of seconds",
            ),
            (
                f64::NEG_INFINITY,
                "the threshold hold time (-inf s) must be a finite number of seconds",
            ),
        ] {
            let mut bad = plan.clone();
            bad.threshold.hold_s = Some(hold);
            invalid.push((bad, expected));
        }
        for side in [AirSide::Nato, AirSide::Dprk] {
            assert!(validate_zone(side, &plan, &copies.entries).is_empty());
            for (bad, expected) in &invalid {
                let issues = validate_zone(side, bad, &copies.entries);
                assert_eq!(issues.len(), 1);
                assert_eq!(issues[0].severity, IssueSeverity::Error);
                assert!(!issues[0].rail_only);
                assert_eq!(
                    issues[0].text,
                    format!("Air Tasking {} zone: {expected}", side.label())
                );
                let mut logic = original_logic.clone();
                let mut next = original_next;
                let message = build_side(side, bad, &copies, &mut logic, &mut next).unwrap_err();
                assert_eq!(message, issues[0].text);
                assert_eq!(logic, original_logic);
                assert_eq!(next, original_next);
            }
            // Report every invalid input; construction refuses the first error.
            let mut bad = plan.clone();
            bad.zone.chance_pct = 0;
            bad.zone.cooldown_min = 0.0;
            bad.threshold.hold_s = Some(3.6);
            let issues = validate_zone(side, &bad, &copies.entries);
            assert_eq!(issues.len(), 3);
            assert!(
                issues
                    .iter()
                    .all(|issue| { issue.severity == IssueSeverity::Error && !issue.rail_only })
            );
            assert_eq!(
                build_side(
                    side,
                    &bad,
                    &copies,
                    &mut original_logic.clone(),
                    &mut original_next.clone()
                )
                .unwrap_err(),
                issues[0].text
            );
        }
        let mut good = plan;
        good.threshold.hold_s = Some(3.65);
        assert!(validate_zone(AirSide::Nato, &good, &copies.entries).is_empty());
        assert!(
            build_side(
                AirSide::Nato,
                &good,
                &copies,
                &mut original_logic.clone(),
                &mut original_next.clone()
            )
            .is_ok()
        );
    }

    #[test]
    fn both_entry_copies_use_zone_spawn_distance() {
        let mut plan = zone_plan(2, TriggerMode::Both);
        plan.pool[1].mode = TriggerMode::Clock;
        let (copies, _, _) = assembled(&plan);
        for entry in &copies.entries {
            let (x, z) = copies.areas[entry.area - 1];
            for copy in &entry.copies {
                let distance = (copy.spawn.0 - x).hypot(copy.spawn.1 - z);
                if entry.mode == TriggerMode::Both {
                    assert!((distance - 30_000.0).abs() <= 1.0);
                } else {
                    assert!((distance - 30_000.0).abs() > 1.0);
                }
                let placed = by_id(&copies.trees[1], copy.root);
                let lead = nodes(placed)
                    .into_iter()
                    .find(|node| {
                        node.block_type == "Plane" && number(node, "NumberInFormation") == Ok(0.0)
                    })
                    .unwrap();
                let (lx, lz) = lead.pos_xz().unwrap();
                assert!(((lx - x).hypot(lz - z) - distance).abs() <= 1.0);
            }
        }
    }

    #[test]
    fn zone_does_not_edit_other_builders() {
        let plan = zone_plan(2, TriggerMode::Both);
        let (mut copies, mut logic, mut next) = assembled(&plan);
        let clock =
            clock::build_side(AirSide::Nato, &plan, &copies, &mut logic, &mut next).unwrap();
        copies.trees[0].children.extend(clock);
        let before = copies.clone();
        let registry = logic.clone();
        let mut debug_plan = plan.clone();
        debug_plan.debug_subtitles = true;
        let mut debug_logic = logic.clone();
        let mut debug_next = next;
        let normal = build_side(AirSide::Nato, &plan, &copies, &mut logic, &mut next).unwrap();
        let debug = build_side(
            AirSide::Nato,
            &debug_plan,
            &copies,
            &mut debug_logic,
            &mut debug_next,
        )
        .unwrap();
        assert_eq!(copies, before);
        assert_eq!(normal, debug);
        assert_eq!(logic, debug_logic);
        assert_eq!(next, debug_next);
        assert_eq!(logic.mission_begin, registry.mission_begin);
        assert_eq!(logic.hot_consumers, registry.hot_consumers);
        assert_eq!(logic.cold_consumers, registry.cold_consumers);
        assert_eq!(logic.gates, registry.gates);
        assert_eq!(logic.flags, registry.flags);
        assert_eq!(logic.budget_events, registry.budget_events);
        assert!(logic.links.starts_with(&registry.links));
        assert!(logic.init_off.starts_with(&registry.init_off));
    }

    #[test]
    fn zone_links_resolve_and_rearm_rule() {
        let h = harness(&zone_plan(3, TriggerMode::Both), 60.0);
        assert_links_resolve(&h.root);
        let single_use: Vec<_> = h
            .copies
            .entries
            .iter()
            .flat_map(|entry| &entry.copies)
            .flat_map(|copy| copy.shell.single_use_outs.iter().copied())
            .collect();
        assert_eq!(
            walker_every_deactivated_out_has_a_rearm_or_is_single_use(&h.root, &single_use),
            Ok(())
        );
        for entry in &h.copies.entries {
            assert_eq!(
                named(&h.root, &format!("ZN{:02} ARM", entry.number)).targets,
                [
                    zone_id(&h, entry.number, "PASS IN"),
                    zone_id(&h, entry.number, "MISS WAIT OUT")
                ]
            );
            assert_eq!(
                named(&h.root, &format!("ZN{:02} FIRE ARM", entry.number)).targets,
                [
                    zone_id(&h, entry.number, "WAITING"),
                    zone_id(&h, entry.number, "DENY WAIT")
                ]
            );
        }
    }

    #[test]
    fn zone_budget_full_refused_rearmed_after_cooldown_then_started() {
        let mut plan = certain_plan(2, TriggerMode::Zone);
        plan.pool[0].mode = TriggerMode::Clock;
        plan.budget.per_side[NATO] = 1;
        let h = harness(&plan, 60.0);
        for config in configs() {
            let log = run(&h, config, 0, 1050.0, &[(2.0, entry_id(&h, 1, "GO"))]);
            let passes = times(&log, id(&h, "THN01 INNER"));
            assert_eq!(passes[0], ticks(PASS));
            assert!(
                passes
                    .windows(2)
                    .all(|pair| pair[1] - pair[0] == ticks(HOLD))
            );
            let next_pass = PASS + 8.0 * HOLD;
            assert_zone_times(&h, &log, 2, "CHANCE", &[PASS + 0.1, next_pass + 0.1]);
            assert_zone_times(&h, &log, 2, "FIRE", &[PASS + 0.15, next_pass + 0.15]);
            assert_zone_times(&h, &log, 2, "DENY WAIT", &[PASS + 1.15]);
            assert_zone_times(&h, &log, 2, "WAITING", &[PASS + 1.15]);
            assert_zone_times(&h, &log, 2, "COOLDOWN", &[PASS + 1.15 + COOLDOWN]);
            assert_zone_times(&h, &log, 2, "ARM", &[PASS + 1.15 + COOLDOWN]);
            assert_eq!(
                times(&log, h.copies.entries[0].copies[0].shell.start),
                [ticks(2.0)]
            );
            assert_eq!(
                times(&log, h.copies.entries[1].copies[0].shell.start),
                [ticks(next_pass + 0.15)]
            );
            assert_eq!(starts(&log, &h).len(), 2);
        }
    }

    #[test]
    fn zone_clock_launch_before_fire_does_not_strand_the_zone() {
        let mut plan = certain_plan(2, TriggerMode::Both);
        plan.budget.per_side[NATO] = 1;
        for entry in &mut plan.pool {
            entry.runs = 2;
        }
        let second = clock::waterfall_order(plan.seed, AirSide::Nato, 2)[1];
        let h = harness(&plan, 60.0);
        let entry = &h.copies.entries[second];
        let copy = &entry.copies[0];
        let go_name = format!("EN{:02} GO", entry.number);
        for config in configs() {
            // The requested +0.01 s is +0.05 s on the walker's tick grid.
            // Pulsing the real 0.5 s delete timer here deletes at PASS +0.55.
            let log = run(
                &h,
                config,
                0,
                PASS + COOLDOWN + 2.0,
                &[
                    (PASS + 0.05, entry_id(&h, entry.number, "GO")),
                    (PASS + 0.05, copy.shell.delete_timer),
                ],
            );
            assert_times(
                &h,
                &log,
                "THN01 INNER",
                &(0..=7).map(|i| PASS + i as f64 * HOLD).collect::<Vec<_>>(),
            );
            assert_times(&h, &log, &go_name, &[PASS + 0.05]);
            assert_eq!(times(&log, copy.shell.start), [ticks(PASS + 0.05)]);
            assert!(times(&log, copy.shell.delete_timer).contains(&ticks(PASS + 0.55)));
            assert!(!active(
                &log,
                entry.copies[1].shell.start,
                ticks(PASS + 0.70)
            ));
            assert!(active(
                &log,
                entry.copies[1].shell.start,
                ticks(PASS + 0.90)
            ));
            assert_zone_times(&h, &log, entry.number, "CHANCE", &[PASS + 0.65]);
            assert_zone_times(&h, &log, entry.number, "FIRE ARM", &[PASS + 0.65]);
            assert_zone_times(&h, &log, entry.number, "FIRE", &[PASS + 0.70]);
            assert_zone_times(&h, &log, entry.number, "DENY WAIT", &[PASS + 1.70]);
            assert_zone_times(&h, &log, entry.number, "WAITING", &[PASS + 1.70]);
            assert_zone_times(
                &h,
                &log,
                entry.number,
                "COOLDOWN",
                &[PASS + 1.70 + COOLDOWN],
            );
            assert_zone_times(&h, &log, entry.number, "ARM", &[PASS + 1.70 + COOLDOWN]);
            assert!(times(&log, entry.copies[1].shell.start).is_empty());
            assert_eq!(starts(&log, &h).len(), 1);
        }
    }

    #[test]
    fn zone_clock_launch_after_fire_arm_leaves_a_way_back() {
        let mut plan = certain_plan(2, TriggerMode::Both);
        plan.pool[0].mode = TriggerMode::Clock;
        plan.pool[0].runs = 1;
        plan.budget.per_side[NATO] = 1;
        let h = harness(&plan, 60.0);
        let blocker = &h.copies.entries[0].copies[0];
        let clock_start = PASS + 0.75;
        let done = clock_start + 60.0;
        for config in configs() {
            let log = run(
                &h,
                config,
                0,
                done + COOLDOWN,
                &[
                    (2.0, entry_id(&h, 1, "GO")),
                    (PASS - 0.2, blocker.shell.delete_timer),
                    (clock_start, entry_id(&h, 2, "GO")),
                ],
            );
            assert_zone_times(&h, &log, 2, "CHANCE", &[PASS + 0.1]);
            assert_zone_times(&h, &log, 2, "FIRE ARM", &[PASS + 0.1]);
            assert_zone_times(&h, &log, 2, "FIRE", &[PASS + 0.15]);
            assert_zone_times(&h, &log, 2, "DENY WAIT", &[]);
            assert_zone_times(&h, &log, 2, "GRANTED", &[clock_start]);
            assert_zone_times(&h, &log, 2, "WAITING", &[done]);
            assert_zone_times(&h, &log, 2, "COOLDOWN", &[done + COOLDOWN]);
            assert_zone_times(&h, &log, 2, "ARM", &[done + COOLDOWN]);
            let copy = &h.copies.entries[1].copies[0];
            assert_eq!(times(&log, copy.shell.start), [ticks(clock_start)]);
            assert_eq!(times(&log, copy.shell.done_hub), [ticks(done)]);
            assert_eq!(starts(&log, &h).len(), 2);
        }
    }

    #[test]
    fn zone_runs_1_both_entry_used_by_the_clock_starts_no_copy() {
        let mut plan = certain_plan(1, TriggerMode::Both);
        plan.pool[0].runs = 1;
        let mut h = harness(&plan, 60.0);
        // There is no tick strictly between CHANCE and FIRE. A test relay
        // after CHANCE's real targets schedules GO after FIRE is started,
        // on CHANCE's tick. FIRE expires on the next tick, after the last
        // pointer closes, in all three delivery configurations. This keeps
        // the requested order without the accepted same-tick double request.
        let mut clock_request =
            template::timer("Test clock after CHANCE", 0.0, &mut h.next, 0.0, 0.0);
        clock_request.set_targets(vec![entry_id(&h, 1, "GO")]);
        let chance = zone_id(&h, 1, "CHANCE");
        by_id_mut(&mut h.root, chance)
            .unwrap()
            .append_target(clock_request.index.unwrap());
        h.root.children.push(clock_request);
        for config in configs() {
            for clock_start in [2.0, PASS + 0.1] {
                let pulses = if clock_start < PASS {
                    vec![(clock_start, entry_id(&h, 1, "GO"))]
                } else {
                    vec![]
                };
                let log = run(&h, config, 0, 1200.0, &pulses);
                let expected_chance = if clock_start > PASS {
                    vec![PASS + 0.1]
                } else {
                    vec![]
                };
                let expected_fire = if clock_start > PASS {
                    vec![PASS + 0.15]
                } else {
                    vec![]
                };
                assert_zone_times(&h, &log, 1, "CHANCE", &expected_chance);
                assert_zone_times(&h, &log, 1, "FIRE", &expected_fire);
                assert_zone_times(&h, &log, 1, "DENY WAIT", &[]);
                assert_zone_times(&h, &log, 1, "COOLDOWN", &[]);
                assert_zone_times(&h, &log, 1, "ARM", &[]);
                assert_times(&h, &log, "SN01 START", &[clock_start]);
                assert_times(&h, &log, "EN01 EXHAUSTED", &[clock_start]);
                assert_eq!(starts(&log, &h).len(), 1);
                for role in ["PASS IN", "MISS WAIT OUT", "COOLDOWN"] {
                    assert!(!active(&log, zone_id(&h, 1, role), ticks(1200.0)));
                }
            }
        }
    }

    #[test]
    fn zone_three_entries_on_one_area_fill_the_budget() {
        let plan = certain_plan(3, TriggerMode::Zone);
        let h = harness(&plan, 60.0);
        let order = clock::waterfall_order(plan.seed, AirSide::Nato, 3);
        for config in configs() {
            let log = run(&h, config, 0, PASS + COOLDOWN + 3.0, &[]);
            for (p, &i) in order.iter().enumerate() {
                let entry = &h.copies.entries[i];
                let fire = PASS + 0.15 + p as f64 * 0.8;
                assert_zone_times(&h, &log, entry.number, "CHANCE", &[fire - 0.05]);
                assert_zone_times(&h, &log, entry.number, "FIRE", &[fire]);
                if p < 2 {
                    assert_eq!(times(&log, entry.copies[0].shell.start), [ticks(fire)]);
                    assert_zone_times(&h, &log, entry.number, "DENY WAIT", &[]);
                    assert_zone_times(&h, &log, entry.number, "WAITING", &[fire + 60.0]);
                    assert_zone_times(&h, &log, entry.number, "COOLDOWN", &[]);
                    assert_zone_times(&h, &log, entry.number, "ARM", &[]);
                } else {
                    assert!(times(&log, entry.copies[0].shell.start).is_empty());
                    assert_zone_times(&h, &log, entry.number, "DENY WAIT", &[fire + 1.0]);
                    assert_zone_times(&h, &log, entry.number, "WAITING", &[fire + 1.0]);
                    assert_zone_times(&h, &log, entry.number, "COOLDOWN", &[fire + 1.0 + COOLDOWN]);
                    assert_zone_times(&h, &log, entry.number, "ARM", &[fire + 1.0 + COOLDOWN]);
                }
            }
            assert_eq!(starts(&log, &h).len(), 2);
            assert_eq!(
                h.logic
                    .flags
                    .iter()
                    .filter(|&&flag| active(&log, flag, ticks(PASS + 3.0)))
                    .count(),
                2
            );
        }
    }

    #[test]
    fn zone_granted_request_does_not_start_the_cooldown() {
        let h = harness(&certain_plan(1, TriggerMode::Zone), 60.0);
        for config in configs() {
            let log = run(&h, config, 0, PASS + 0.15 + 60.0 + COOLDOWN, &[]);
            assert_zone_times(&h, &log, 1, "CHANCE", &[PASS + 0.1]);
            assert_zone_times(&h, &log, 1, "FIRE", &[PASS + 0.15]);
            assert_zone_times(&h, &log, 1, "GRANTED", &[PASS + 0.15]);
            assert_zone_times(&h, &log, 1, "DENY WAIT", &[]);
            assert_zone_times(&h, &log, 1, "MISS WAIT OUT", &[]);
            assert_times(&h, &log, "SN01 START", &[PASS + 0.15]);
            assert_times(&h, &log, "SN01 DONE HUB", &[PASS + 60.15]);
            assert_zone_times(&h, &log, 1, "WAITING", &[PASS + 60.15]);
            // Walker pulses record timer expiry, so COOLDOWN is observed at
            // DONE + C; WAITING is the pulse that starts that timer.
            assert_zone_times(&h, &log, 1, "COOLDOWN", &[PASS + 60.15 + COOLDOWN]);
            assert_zone_times(&h, &log, 1, "ARM", &[PASS + 60.15 + COOLDOWN]);
            assert_eq!(starts(&log, &h).len(), 1);
        }
    }

    #[test]
    fn zone_no_request_after_runs_starts() {
        let h = harness(&certain_plan(1, TriggerMode::Zone), 60.0);
        for config in configs() {
            let log = run(&h, config, 0, 5000.0, &[]);
            let launches: Vec<_> = (0..3)
                // ARM (launch + 60 s + C) is 0.15 s after the eighth pass.
                .map(|i| PASS + i as f64 * 9.0 * HOLD + 0.15)
                .collect();
            let chances: Vec<_> = launches.iter().map(|t| t - 0.05).collect();
            assert_zone_times(&h, &log, 1, "CHANCE", &chances);
            assert_zone_times(&h, &log, 1, "FIRE", &launches);
            assert_zone_times(&h, &log, 1, "DENY WAIT", &[]);
            let rearms: Vec<_> = launches[..2].iter().map(|t| t + 60.0 + COOLDOWN).collect();
            assert_zone_times(&h, &log, 1, "COOLDOWN", &rearms);
            assert_zone_times(&h, &log, 1, "ARM", &rearms);
            for (copy, &t) in h.copies.entries[0].copies.iter().zip(&launches) {
                assert_eq!(times(&log, copy.shell.start), [ticks(t)]);
            }
            assert_eq!(starts(&log, &h).len(), 3);
            assert_times(&h, &log, "EN01 EXHAUSTED", &[launches[2]]);
            assert!(!active(&log, zone_id(&h, 1, "COOLDOWN"), ticks(5000.0)));
        }
    }

    #[test]
    fn zone_fire_then_later_chance_miss_reaches_cooldown() {
        let h = harness(&zone_plan(1, TriggerMode::Zone), 60.0);
        // SplitMix64 seed 14 rolls 38 then 94: a success, then a miss at 50%.
        for config in configs() {
            let second_pass = PASS + 9.0 * HOLD; // The eighth is 0.15 s before ARM.
            let log = run(&h, config, 14, second_pass + 1.0 + COOLDOWN, &[]);
            assert_zone_times(&h, &log, 1, "CHANCE", &[PASS + 0.1]);
            assert_zone_times(&h, &log, 1, "FIRE", &[PASS + 0.15]);
            assert_zone_times(&h, &log, 1, "DENY WAIT", &[]);
            assert_zone_times(&h, &log, 1, "MISS WAIT OUT", &[second_pass + 1.0]);
            assert_zone_times(
                &h,
                &log,
                1,
                "COOLDOWN",
                &[PASS + 60.15 + COOLDOWN, second_pass + 1.0 + COOLDOWN],
            );
            assert_zone_times(
                &h,
                &log,
                1,
                "ARM",
                &[PASS + 60.15 + COOLDOWN, second_pass + 1.0 + COOLDOWN],
            );
            assert_times(&h, &log, "SN01 START", &[PASS + 0.15]);
            assert_times(&h, &log, "SN01 DONE HUB", &[PASS + 60.15]);
            assert_eq!(starts(&log, &h).len(), 1);
        }
    }

    #[test]
    fn zone_clock_done_does_not_start_idle_zone_cooldown() {
        let mut plan = certain_plan(1, TriggerMode::Both);
        // Keep the full threshold active but let the clock copy finish before
        // the first PASS; WAITING is still in its initial off state.
        plan.threshold.dwell_s = Some(2000.0);
        let h = harness(&plan, 60.0);
        for config in configs() {
            let log = run(&h, config, 0, 1100.0, &[(2.0, entry_id(&h, 1, "GO"))]);
            assert_times(&h, &log, "SN01 START", &[2.0]);
            assert_times(&h, &log, "SN01 DONE HUB", &[62.0]);
            for role in ["CHANCE", "FIRE", "DENY WAIT", "WAITING", "COOLDOWN", "ARM"] {
                assert_zone_times(&h, &log, 1, role, &[]);
            }
            assert_times(&h, &log, "THN01 INNER", &[]);
            assert!(active(&log, zone_id(&h, 1, "CHANCE"), ticks(1100.0)));
            assert_eq!(starts(&log, &h).len(), 1);
        }
    }

    #[test]
    fn zone_second_pass_while_cooling_starts_nothing() {
        let h = harness(&certain_plan(1, TriggerMode::Zone), 60.0);
        for config in configs() {
            let log = run(&h, config, 0, 1000.0, &[]);
            let passes: Vec<_> = (0..8).map(|i| PASS + i as f64 * HOLD).collect();
            assert_times(&h, &log, "THN01 INNER", &passes);
            assert_zone_times(&h, &log, 1, "CHANCE", &[PASS + 0.1]);
            assert_zone_times(&h, &log, 1, "FIRE", &[PASS + 0.15]);
            assert_zone_times(&h, &log, 1, "DENY WAIT", &[]);
            assert_zone_times(&h, &log, 1, "MISS WAIT OUT", &[]);
            assert_zone_times(&h, &log, 1, "COOLDOWN", &[]);
            assert_zone_times(&h, &log, 1, "ARM", &[]);
            assert_times(&h, &log, "SN01 START", &[PASS + 0.15]);
            assert_zone_times(&h, &log, 1, "WAITING", &[PASS + 60.15]);
            assert_eq!(starts(&log, &h).len(), 1);
        }
        let h = harness(&zone_plan(1, TriggerMode::Zone), 60.0);
        for config in configs() {
            // SplitMix64 seed 1 first rolls 65: a miss at the default 50%.
            let log = run(&h, config, 1, 1000.0, &[]);
            let passes: Vec<_> = (0..8).map(|i| PASS + i as f64 * HOLD).collect();
            assert_times(&h, &log, "THN01 INNER", &passes);
            for role in ["CHANCE", "FIRE", "DENY WAIT", "WAITING"] {
                assert_zone_times(&h, &log, 1, role, &[]);
            }
            assert_zone_times(&h, &log, 1, "MISS WAIT OUT", &[PASS + 1.0]);
            assert_zone_times(&h, &log, 1, "DISARM", &[PASS + 1.0]);
            assert_zone_times(&h, &log, 1, "COOLDOWN", &[PASS + 1.0 + COOLDOWN]);
            assert_zone_times(&h, &log, 1, "ARM", &[PASS + 1.0 + COOLDOWN]);
            assert!(starts(&log, &h).is_empty());
        }
    }

    #[test]
    fn zone_rolls_again_at_the_first_pass_after_one_cooldown() {
        // Flight A2's fault: a won roll left MISS WAIT running, and switching it
        // off only paused it (flight 2K). ARM resumed it, and it shut the zone
        // for a second cooldown 0.9 s later.
        let mut plan = certain_plan(1, TriggerMode::Zone);
        plan.pool[0].runs = 2;
        plan.zone.cooldown_min = 14.0;
        let h = harness(&plan, 60.0);
        for config in configs() {
            let log = run(&h, config, 0, 1200.0, &[]);
            let rearm = PASS + 60.15 + 840.0;
            assert_zone_times(&h, &log, 1, "ARM", &[rearm]);
            let passes = times(&log, id(&h, "THN01 INNER"));
            let next = *passes.iter().find(|&&t| t > ticks(rearm)).unwrap();
            let chances = [ticks(PASS + 0.1), next + ticks(0.1)];
            assert_eq!(times(&log, zone_id(&h, 1, "CHANCE")), chances, "{config:?}");
            let launches = [ticks(PASS + 0.15), next + ticks(0.15)];
            for (copy, &t) in h.copies.entries[0].copies.iter().zip(&launches) {
                assert_eq!(times(&log, copy.shell.start), [t], "{config:?}");
            }
            assert_eq!(times(&log, zone_id(&h, 1, "COOLDOWN")), [ticks(rearm)]);
        }
    }

    #[test]
    fn zone_pass_in_the_last_second_of_a_cooldown_is_dropped() {
        // The threshold passes every HOLD while a plane stays. A pass shortly
        // before ARM must not run a miss timer that ARM then lets through:
        // that would be a miss nobody rolled, and a second cooldown.
        let mut plan = zone_plan(1, TriggerMode::Zone);
        plan.zone.cooldown_min = 2.0;
        let h = harness(&plan, 60.0);
        for config in configs() {
            // SplitMix64 seed 1 first rolls 65: a miss at the default 50%.
            let log = run(&h, config, 1, PASS + 200.0, &[]);
            let passes = times(&log, id(&h, "THN01 INNER"));
            assert_eq!(passes.len(), 2);
            assert!((ticks(PASS + 120.0)..ticks(PASS + 121.0)).contains(&passes[1]));
            assert_zone_times(&h, &log, 1, "CHANCE", &[]);
            assert_zone_times(&h, &log, 1, "DISARM", &[PASS + 1.0]);
            assert_zone_times(&h, &log, 1, "COOLDOWN", &[PASS + 121.0]);
            assert_zone_times(&h, &log, 1, "ARM", &[PASS + 121.0]);
            assert!(starts(&log, &h).is_empty());
        }
    }

    #[test]
    fn zone_chance_rate() {
        let h = harness(&zone_plan(1, TriggerMode::Zone), 60.0);
        let mut fired = 0;
        for seed in 0..200 {
            let log = run(&h, configs()[seed as usize % 3], seed, 63.0, &[]);
            assert_times(&h, &log, "THN01 INNER", &[PASS]);
            let success = !times(&log, zone_id(&h, 1, "CHANCE")).is_empty();
            if success {
                fired += 1;
                assert_zone_times(&h, &log, 1, "CHANCE", &[PASS + 0.1]);
                assert_zone_times(&h, &log, 1, "FIRE", &[PASS + 0.15]);
                assert_zone_times(&h, &log, 1, "MISS WAIT OUT", &[]);
                assert_times(&h, &log, "SN01 START", &[PASS + 0.15]);
            } else {
                assert_zone_times(&h, &log, 1, "CHANCE", &[]);
                assert_zone_times(&h, &log, 1, "FIRE", &[]);
                assert_zone_times(&h, &log, 1, "MISS WAIT OUT", &[PASS + 1.0]);
                assert!(starts(&log, &h).is_empty());
            }
            for role in ["DENY WAIT", "COOLDOWN", "ARM"] {
                assert_zone_times(&h, &log, 1, role, &[]);
            }
        }
        assert!(
            (80..=120).contains(&fired),
            "{fired}/200 independent armings fired"
        );
    }

    #[test]
    fn zone_fire_starts_one_copy() {
        let mut plan = certain_plan(1, TriggerMode::Zone);
        plan.pool[0].runs = 6;
        let h = harness(&plan, 60.0);
        for config in configs() {
            let log = run(&h, config, 0, 65.0, &[]);
            assert_zone_times(&h, &log, 1, "CHANCE", &[PASS + 0.1]);
            assert_zone_times(&h, &log, 1, "FIRE", &[PASS + 0.15]);
            for role in ["DENY WAIT", "COOLDOWN", "ARM"] {
                assert_zone_times(&h, &log, 1, role, &[]);
            }
            assert_times(&h, &log, "SN01 START", &[PASS + 0.15]);
            for copy in &h.copies.entries[0].copies[1..] {
                assert!(times(&log, copy.shell.start).is_empty());
            }
            assert_eq!(starts(&log, &h).len(), 1);
            assert!(!active(
                &log,
                h.copies.entries[0].copies[0].pointer,
                ticks(65.0)
            ));
            assert!(active(
                &log,
                h.copies.entries[0].copies[1].pointer,
                ticks(65.0)
            ));
        }
    }

    /// Test-only B′: the latest entry's START also clears every other LAST.
    /// The existing LAST SET and all production builders are left intact.
    fn add_b_prime(h: &mut Harness) {
        let before = mcus(&h.root);
        for entry in &h.copies.entries {
            let mut clear = template::mcu(
                "MCU_Deactivate",
                &format!("Test EN{:02} OTHER LAST OFF", entry.number),
                &mut h.next,
                0.0,
                0.0,
            );
            clear.set_targets(
                h.copies
                    .entries
                    .iter()
                    .filter(|other| other.number != entry.number)
                    .map(|other| entry_id(h, other.number, "LAST"))
                    .collect(),
            );
            for copy in &entry.copies {
                by_id_mut(&mut h.root, copy.shell.start)
                    .unwrap()
                    .append_target(clear.index.unwrap());
            }
            h.root.children.push(clear);
        }
        assert_eq!(mcus(&h.root) - before, h.copies.entries.len());
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Request {
        Clock,
        Zone,
    }

    struct ZonePulses {
        chance: Vec<Tick>,
        fire: Vec<Tick>,
        deny: Vec<Tick>,
        cooldown: Vec<Tick>,
        arm: Vec<Tick>,
        starts: Vec<Tick>,
        go: Vec<Tick>,
    }

    struct BusyCounts {
        seed: u64,
        launches: usize,
        launches_before_exhaustion: usize,
        first_exhaustion: Option<Tick>,
        unique_copies: usize,
        non_quiet_slots: usize,
        slots_lost: usize,
        repeats: usize,
        clock_launches: usize,
        zone_launches: usize,
        zones: Vec<ZonePulses>,
    }

    fn busy_counts(h: &Harness, log: &WalkerLog, seed: u64, budget: u8) -> BusyCounts {
        let start_entry: BTreeMap<_, _> = h
            .copies
            .entries
            .iter()
            .flat_map(|entry| {
                entry
                    .copies
                    .iter()
                    .map(move |copy| (copy.shell.start, entry.number))
            })
            .collect();
        let requests: BTreeMap<_, _> = h
            .copies
            .entries
            .iter()
            .flat_map(|entry| {
                [
                    (
                        entry_id(h, entry.number, "GO"),
                        (entry.number, Request::Clock),
                    ),
                    (
                        zone_id(h, entry.number, "FIRE"),
                        (entry.number, Request::Zone),
                    ),
                ]
            })
            .collect();
        let flags: BTreeSet<_> = h.logic.flags.iter().copied().collect();
        let flag_switches: BTreeMap<_, _> = nodes(&h.root)
            .into_iter()
            .filter(|node| matches!(node.block_type.as_str(), "MCU_Activate" | "MCU_Deactivate"))
            .map(|node| {
                (
                    node.index.unwrap(),
                    (
                        node.block_type == "MCU_Activate",
                        node.targets
                            .iter()
                            .chain(&node.objects)
                            .copied()
                            .filter(|flag| flags.contains(flag))
                            .collect::<Vec<_>>(),
                    ),
                )
            })
            .collect();
        // Lost slots include the in-flight attempt after SLOT CLOSE. The
        // prompt's repeat metric is restricted to the slot window itself.
        // Quiet ticks never enter the slots list.
        let slot_window = ticks(number(named(&h.root, "CLK N SLOT CLOSE"), "Time").unwrap());
        let slot_span =
            slot_window + ticks(number(named(&h.root, "CLK N ATTEMPT END"), "Time").unwrap());
        let slot_open = id(h, "CLK N SLOT OPEN");
        let mut slots = Vec::new();
        let mut flying = flags; // All timers start active; INIT OFF is in the log.
        let mut used = BTreeSet::new();
        let mut last_request = BTreeMap::new();
        let mut zone_since_open = BTreeSet::new();
        let mut zone_before_open = BTreeSet::new();
        let mut launches = Vec::new();
        let mut repeats = 0;
        let mut clock_launches = 0;
        let mut zone_launches = 0;
        for pulse in &log.pulses {
            // Busy-area runs use DepthFirst. Replay only recorded flag
            // switches so capacity is measured at SLOT OPEN itself, even if
            // another launch/deletion shares its tick. End-of-tick snapshots
            // would include later changes and could misclassify a lost slot.
            if let Some((on, targets)) = flag_switches.get(&pulse.index) {
                for &target in targets {
                    if *on {
                        flying.insert(target);
                    } else {
                        flying.remove(&target);
                    }
                }
            }
            if pulse.index == slot_open {
                let copies_left = h.copies.entries.iter().any(|entry| {
                    entry
                        .copies
                        .iter()
                        .any(|copy| !used.contains(&copy.shell.start))
                });
                slots.push((pulse.t, flying.len() < usize::from(budget) && copies_left));
                // A clock launch in this slot is a repeat if the zone started
                // that entry after the preceding SLOT OPEN (or mission begin
                // for slot 1), including a zone start inside the current slot.
                zone_before_open = std::mem::take(&mut zone_since_open);
            }
            if let Some(&(entry, source)) = requests.get(&pulse.index) {
                last_request.insert(entry, (pulse.t, source));
            }
            if let Some(&entry) = start_entry.get(&pulse.index) {
                launches.push(pulse.t);
                used.insert(pulse.index);
                if let Some(&(t, source)) = last_request.get(&entry)
                    && t == pulse.t
                {
                    match source {
                        Request::Clock
                            if slots
                                .last()
                                .is_some_and(|&(open, _)| pulse.t <= open + slot_span) =>
                        {
                            clock_launches += 1;
                            if pulse.t <= slots.last().unwrap().0 + slot_window
                                && (zone_before_open.contains(&entry)
                                    || zone_since_open.contains(&entry))
                            {
                                repeats += 1;
                            }
                        }
                        Request::Zone => {
                            zone_launches += 1;
                            zone_since_open.insert(entry);
                        }
                        _ => {}
                    }
                }
            }
        }
        let slots_lost = slots
            .iter()
            .filter(|&&(open, room)| {
                room && !launches
                    .iter()
                    .any(|&t| (open..=open + slot_span).contains(&t))
            })
            .count();
        let first_exhaustion = h
            .copies
            .entries
            .iter()
            .flat_map(|entry| times(log, entry.exhausted))
            .min();
        // Include the launch that consumes the last copy. If no entry runs
        // out within three hours, every observed launch is before exhaustion.
        let launches_before_exhaustion = launches
            .iter()
            .filter(|&&t| first_exhaustion.is_none_or(|end| t <= end))
            .count();
        BusyCounts {
            seed,
            launches: launches.len(),
            launches_before_exhaustion,
            first_exhaustion,
            unique_copies: used.len(),
            non_quiet_slots: slots.len(),
            slots_lost,
            repeats,
            clock_launches,
            zone_launches,
            zones: h
                .copies
                .entries
                .iter()
                .map(|entry| ZonePulses {
                    chance: times(log, zone_id(h, entry.number, "CHANCE")),
                    fire: times(log, zone_id(h, entry.number, "FIRE")),
                    deny: times(log, zone_id(h, entry.number, "DENY WAIT")),
                    cooldown: times(log, zone_id(h, entry.number, "COOLDOWN")),
                    arm: times(log, zone_id(h, entry.number, "ARM")),
                    starts: entry
                        .copies
                        .iter()
                        .flat_map(|copy| times(log, copy.shell.start))
                        .collect(),
                    go: times(log, entry_id(h, entry.number, "GO")),
                })
                .collect(),
        }
    }

    fn busy_pair(
        entries: usize,
        budget: u8,
        runs: u8,
        done_min: f64,
        seed: u64,
    ) -> (BusyCounts, BusyCounts) {
        let mut plan = zone_plan(entries, TriggerMode::Both);
        plan.seed = seed;
        plan.budget.per_side[NATO] = budget;
        plan.schedule[NATO] = Schedule::default();
        for entry in &mut plan.pool {
            entry.runs = runs;
        }
        let mut h = harness(&plan, done_min * 60.0);
        // Keep the seed and the delivery model identical within every pair.
        // Both scenarios use the default zone, schedule and 1 s..3 h loiter.
        let measure = |h: &Harness| {
            let log = run(h, configs()[0], seed, 3.0 * 60.0 * 60.0, &[]);
            busy_counts(h, &log, seed, budget)
        };
        let b = measure(&h);
        add_b_prime(&mut h);
        let b_prime = measure(&h);
        assert_eq!(b.seed, b_prime.seed);
        (b, b_prime)
    }

    #[test]
    fn zone_busy_area_check_counts() {
        // B′ clears every other LAST on the side, even on a different area.
        let mut plan = zone_plan(2, TriggerMode::Both);
        plan.pool[1].target = TargetRef::Objective {
            side: AirSide::Nato,
            xz: (150_000.0, 285_000.0),
        };
        let mut h = harness(&plan, 60.0);
        assert_ne!(h.copies.entries[0].area, h.copies.entries[1].area);
        add_b_prime(&mut h);
        for (entry, other) in [(1, 2), (2, 1)] {
            let clear = named(&h.root, &format!("Test EN{entry:02} OTHER LAST OFF"));
            assert_eq!(clear.targets, [entry_id(&h, other, "LAST")]);
            for copy in &h.copies.entries[entry - 1].copies {
                assert!(
                    by_id(&h.root, copy.shell.start)
                        .targets
                        .contains(&clear.index.unwrap())
                );
            }
        }
        for seed in 0..2 {
            let (b, b_prime) = busy_pair(2, 1, 3, 20.0, seed);
            for counts in [b, b_prime] {
                assert!(counts.launches >= 1);
                assert!((1..=counts.launches).contains(&counts.launches_before_exhaustion));
                // Independently derive exhaustion from each entry's STARTs,
                // rather than the EXHAUSTED pulses used by busy_counts.
                let first_exhaustion = counts
                    .zones
                    .iter()
                    .filter(|zone| zone.starts.len() == 3)
                    .filter_map(|zone| zone.starts.iter().copied().max())
                    .min();
                assert_eq!(counts.first_exhaustion, first_exhaustion);
                assert!(
                    counts
                        .first_exhaustion
                        .is_some_and(|t| t > 0 && t <= ticks(10_800.0))
                );
                assert_eq!(
                    counts.launches_before_exhaustion,
                    counts
                        .zones
                        .iter()
                        .flat_map(|zone| &zone.starts)
                        .filter(|&&t| first_exhaustion.is_none_or(|end| t <= end))
                        .count()
                );
                assert_eq!(
                    counts.launches, counts.unique_copies,
                    "no copy starts twice"
                );
                assert_eq!(
                    counts.launches,
                    counts.clock_launches + counts.zone_launches
                );
                assert!(counts.slots_lost <= counts.non_quiet_slots);
                assert!(counts.repeats <= counts.clock_launches);
                assert!(counts.non_quiet_slots <= 9);
                for zone in counts.zones {
                    assert_eq!(
                        zone.fire,
                        zone.chance
                            .iter()
                            .map(|t| t + ticks(0.05))
                            .collect::<Vec<_>>()
                    );
                    assert!(
                        zone.deny
                            .iter()
                            .all(|t| zone.fire.contains(&(t - ticks(1.0))))
                    );
                    assert_eq!(zone.cooldown, zone.arm);
                    assert!(
                        zone.starts
                            .iter()
                            .all(|t| zone.fire.contains(t) || zone.go.contains(t))
                    );
                }
            }
        }
    }

    #[test]
    #[ignore] // 1,600 paired three-hour missions; run explicitly for decision 0029.
    fn zone_busy_area_no_repeat_b_against_b_prime() {
        let wall = std::time::Instant::now();
        println!(
            "Means over 200 paired seeds per row; before exhaustion includes the exhausting launch."
        );
        println!(
            "First exhaustion: mean minutes among exhausted runs (count/200); no exhaustion uses all launches through 180 min."
        );
        println!(
            "entries | budget | runs | DONE min | launches B | launches B' | B'-B | lost B | lost B' | B' repeats | abs(diff) < 0.5 | before exhaustion B | before exhaustion B' | before B'-B | first exhaustion B min (n) | first exhaustion B' min (n) | abs(before diff) < 0.5 | both bars"
        );
        for (runs, done_min) in [(3, 20.0), (6, 10.0)] {
            for entries in [2, 4] {
                for budget in [1, 2] {
                    let mut sums = [0usize; 7];
                    let mut exhaustion = [(0u64, 0usize); 2];
                    for seed in 0..200 {
                        let (b, b_prime) = busy_pair(entries, budget, runs, done_min, seed);
                        for counts in [&b, &b_prime] {
                            assert_eq!(counts.launches, counts.unique_copies);
                            assert!(counts.launches_before_exhaustion <= counts.launches);
                            assert!(counts.slots_lost <= counts.non_quiet_slots);
                        }
                        for (sum, value) in sums.iter_mut().zip([
                            b.launches,
                            b_prime.launches,
                            b.slots_lost,
                            b_prime.slots_lost,
                            b_prime.repeats,
                            b.launches_before_exhaustion,
                            b_prime.launches_before_exhaustion,
                        ]) {
                            *sum += value;
                        }
                        for ((sum, count), first) in exhaustion
                            .iter_mut()
                            .zip([b.first_exhaustion, b_prime.first_exhaustion])
                        {
                            if let Some(first) = first {
                                *sum += first;
                                *count += 1;
                            }
                        }
                    }
                    let [
                        launches_b,
                        launches_prime,
                        lost_b,
                        lost_prime,
                        repeats,
                        before_b,
                        before_prime,
                    ] = sums.map(|sum| sum as f64 / 200.0);
                    let [exhaustion_b, exhaustion_prime] = exhaustion.map(|(sum, count)| {
                        if count == 0 {
                            "none (0/200)".to_owned()
                        } else {
                            format!(
                                "{:.3} ({count}/200)",
                                sum as f64 / count as f64 / ticks(60.0) as f64
                            )
                        }
                    });
                    let diff = launches_prime - launches_b;
                    let before_diff = before_prime - before_b;
                    let total_bar = diff.abs() < 0.5;
                    let before_bar = before_diff.abs() < 0.5;
                    // Report the bars, never assert them: crossing one asks
                    // the user to choose B or B′, not to change this test.
                    println!(
                        "{entries} | {budget} | {runs} | {done_min:.0} | {launches_b:.3} | {launches_prime:.3} | {diff:+.3} | {lost_b:.3} | {lost_prime:.3} | {repeats:.3} | {total_bar} | {before_b:.3} | {before_prime:.3} | {before_diff:+.3} | {exhaustion_b} | {exhaustion_prime} | {before_bar} | {}",
                        total_bar && before_bar
                    );
                }
            }
        }
        println!("Busy-area wall time: {:.2} s", wall.elapsed().as_secs_f64());
    }
}
