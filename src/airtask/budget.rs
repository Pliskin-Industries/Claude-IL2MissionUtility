//! Recount the live-copy flags after every launch and deletion (U20/U22).

use std::collections::BTreeMap;

use super::shell::{by_id_mut, group};
use super::{AirSide, BudgetImpl, CensusImpl, SideLogic, Strategies};
use crate::ast::Il2Entity;
use crate::template;

pub(crate) fn census_delay(census: CensusImpl, order: usize) -> f64 {
    match census {
        CensusImpl::Spaced => order as f64 / 20.0, // Flight 0: E7 confirmed; Spaced remains the census default.
        CensusImpl::SameTick => 0.0,
    }
}

pub(crate) fn recount_time(census: CensusImpl, copies: usize) -> f64 {
    match census {
        CensusImpl::Spaced => (copies as f64 + 3.0) / 20.0,
        CensusImpl::SameTick => 0.2,
    }
}

/// Returns one Budget group. Flag delays are assigned by assemble_side using
/// census_delay; links into the flags and lifecycle events are deferred.
pub(crate) fn build_side(
    side: AirSide,
    n: u8,
    logic: &mut SideLogic,
    strategies: &Strategies,
    next_id: &mut i32,
) -> Vec<Il2Entity> {
    let n = match strategies.budget {
        BudgetImpl::HeadCount => n, // Flight 0: E1/E2/E4 confirmed; HeadCount remains the budget default.
        BudgetImpl::SingleSlot => 1,
    };
    assert!((1..=6).contains(&n), "validated air budget must be 1..=6");
    let mut root = group("Budget", next_id);
    let mut ids = BTreeMap::new();
    for (name, kind, time) in [
        ("MARK", "MCU_Activate", 0.0),
        ("CHECK", "MCU_Timer", 0.0),
        ("SHUT", "MCU_Deactivate", 0.0),
        ("REOPEN ARM", "MCU_Activate", 0.0),
        ("ZERO", "MCU_ModifierSetVal", 0.0),
        ("CLEAR", "MCU_Timer", 0.02),
        ("CLEAR OFF", "MCU_Deactivate", 0.0),
        ("CENSUS", "MCU_Timer", 0.05),
        ("COUNT", "MCU_Counter", 0.0),
        ("STAY SHUT", "MCU_Deactivate", 0.0),
        (
            "END",
            "MCU_Timer",
            recount_time(strategies.census, logic.flags.len()),
        ),
        ("REOPEN OUT", "MCU_Timer", 0.0),
        ("OPEN", "MCU_Activate", 0.0),
        ("CHECK ON", "MCU_Activate", 0.0),
        ("AGAIN DELAY", "MCU_Timer", 0.05),
        ("AGAIN", "MCU_Timer", 0.0),
    ] {
        if n == 1 && matches!(name, "COUNT" | "ZERO") {
            continue;
        }
        let full_name = format!("BUD {} {name}", side.letter());
        let node = match kind {
            "MCU_Timer" => template::timer(&full_name, time, next_id, 0.0, 0.0),
            "MCU_Counter" => template::counter(&full_name, i32::from(n), 1, next_id, 0.0, 0.0),
            "MCU_ModifierSetVal" => template::modifier_set_val(&full_name, next_id, 0.0, 0.0),
            _ => template::mcu(kind, &full_name, next_id, 0.0, 0.0),
        };
        ids.insert(name, node.index.unwrap());
        root.children.push(node);
    }
    let mut shut = logic.gates.clone();
    shut.push(ids["CHECK"]);
    let mut check = vec![ids["SHUT"], ids["REOPEN ARM"]];
    if n > 1 {
        check.push(ids["ZERO"]);
    }
    check.extend([ids["CLEAR"], ids["CENSUS"], ids["END"]]);
    let mut flags = logic.flags.clone();
    flags.sort_unstable();
    for (from, to) in [
        ("MARK", vec![ids["AGAIN"]]),
        ("CHECK", check),
        ("SHUT", shut),
        ("REOPEN ARM", vec![ids["REOPEN OUT"]]),
        ("CLEAR", vec![ids["CLEAR OFF"]]),
        ("CLEAR OFF", vec![ids["AGAIN"]]),
        ("CENSUS", flags),
        ("STAY SHUT", vec![ids["REOPEN OUT"]]),
        (
            "END",
            vec![ids["REOPEN OUT"], ids["CHECK ON"], ids["AGAIN DELAY"]],
        ),
        ("REOPEN OUT", vec![ids["OPEN"]]),
        ("OPEN", logic.gates.clone()),
        ("CHECK ON", vec![ids["CHECK"]]),
        ("AGAIN DELAY", vec![ids["AGAIN"]]),
    ] {
        by_id_mut(&mut root, ids[from]).unwrap().set_targets(to);
    }
    if n > 1 {
        by_id_mut(&mut root, ids["ZERO"])
            .unwrap()
            .set_targets(vec![ids["COUNT"]]);
        by_id_mut(&mut root, ids["COUNT"])
            .unwrap()
            .set_targets(vec![ids["STAY SHUT"]]);
    }
    for &event in logic
        .budget_events
        .iter()
        .chain(std::iter::once(&ids["AGAIN"]))
    {
        logic
            .links
            .extend([(event, ids["MARK"]), (event, ids["CHECK"])]);
    }
    let destination = if n == 1 {
        ids["STAY SHUT"]
    } else {
        ids["COUNT"]
    };
    logic
        .links
        .extend(logic.flags.iter().map(|&flag| (flag, destination)));
    for &flag in &logic.flags {
        if !logic.init_off.contains(&flag) {
            logic.init_off.push(flag);
        }
    }
    logic.init_off.push(ids["AGAIN"]);
    vec![root]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::airtask::assemble::emit_side_logic;
    use crate::airtask::library::{by_id, nodes, number};
    use crate::airtask::shell::test_support::{active, configs, mcus, named, ticks, times};
    use crate::airtask::testkit::*;

    pub(crate) struct Harness {
        pub(crate) root: Il2Entity,
        pub(crate) logic: SideLogic,
        pub(crate) gates: Vec<i32>,
        pub(crate) flags: Vec<i32>,
        pub(crate) requests: Vec<i32>,
        pub(crate) done: Vec<i32>,
        pub(crate) deletions: Vec<i32>,
        pub(crate) next_id: i32,
    }

    pub(crate) fn harness(side: AirSide, n: u8, copies: usize, strategies: Strategies) -> Harness {
        let mut next_id = 1;
        let mut root = group("Budget harness", &mut next_id);
        let mut logic = SideLogic::default();
        let mut requests = Vec::new();
        let mut done_ids = Vec::new();
        let mut deletions = Vec::new();
        for q in 1..=copies {
            let prefix = format!("S{}{:02}", side.letter(), q);
            let mut start =
                template::timer(&format!("{prefix} START"), 0.0, &mut next_id, 0.0, 0.0);
            let flag = template::timer(
                &format!("{prefix} FLYING"),
                census_delay(strategies.census, q),
                &mut next_id,
                0.0,
                0.0,
            );
            let mut request =
                template::timer(&format!("{prefix} REQUEST"), 0.0, &mut next_id, 0.0, 0.0);
            let mut on = template::mcu(
                "MCU_Activate",
                &format!("{prefix} STARTED ON"),
                &mut next_id,
                0.0,
                0.0,
            );
            let mut off = template::mcu(
                "MCU_Deactivate",
                &format!("{prefix} STARTED OFF"),
                &mut next_id,
                0.0,
                0.0,
            );
            let mut done =
                template::timer(&format!("{prefix} DONE HUB"), 0.0, &mut next_id, 0.0, 0.0);
            let mut done_off = template::mcu(
                "MCU_Deactivate",
                &format!("{prefix} DONE HUB OFF"),
                &mut next_id,
                0.0,
                0.0,
            );
            let mut delay = template::timer(
                &format!("{prefix} RTB AND DELETE DELAY"),
                60.6,
                &mut next_id,
                0.0,
                0.0,
            );
            let mut deleted = template::timer(
                &format!("{prefix} DELETE TIMER"),
                0.0,
                &mut next_id,
                0.0,
                0.0,
            );
            let mut landed = template::mcu(
                "MCU_Deactivate",
                &format!("{prefix} LANDED"),
                &mut next_id,
                0.0,
                0.0,
            );
            request.set_targets(vec![start.index.unwrap()]);
            start.set_targets(vec![on.index.unwrap(), off.index.unwrap()]);
            on.set_targets(vec![flag.index.unwrap()]);
            off.set_targets(vec![request.index.unwrap()]);
            done.set_targets(vec![done_off.index.unwrap(), delay.index.unwrap()]);
            done_off.set_targets(vec![done.index.unwrap()]);
            delay.set_targets(vec![deleted.index.unwrap()]);
            deleted.set_targets(vec![landed.index.unwrap()]);
            landed.set_targets(vec![flag.index.unwrap()]);
            requests.push(request.index.unwrap());
            done_ids.push(done.index.unwrap());
            deletions.push(deleted.index.unwrap());
            logic.gates.push(start.index.unwrap());
            logic.flags.push(flag.index.unwrap());
            logic
                .budget_events
                .extend([start.index.unwrap(), deleted.index.unwrap()]);
            root.children.extend([
                request, start, flag, on, off, done, done_off, delay, deleted, landed,
            ]);
        }
        root.children
            .extend(build_side(side, n, &mut logic, &strategies, &mut next_id));
        let init =
            emit_side_logic(side, &logic, std::slice::from_mut(&mut root), &mut next_id).unwrap();
        root.children.extend(init);
        assert_links_resolve(&root);
        Harness {
            root,
            gates: logic.gates.clone(),
            flags: logic.flags.clone(),
            logic,
            requests,
            done: done_ids,
            deletions,
            next_id,
        }
    }

    fn run(h: &Harness, config: WalkerConfig, until: f64, events: &[(f64, i32)]) -> WalkerLog {
        walk(
            &h.root,
            config,
            &WalkerInput {
                until: ticks(until),
                pulses: events
                    .iter()
                    .map(|&(t, id)| (ticks(t), id.into()))
                    .collect(),
                ..WalkerInput::default()
            },
        )
    }
    fn budget_id(h: &Harness, suffix: &str) -> i32 {
        named(&h.root, &format!("BUD N {suffix}")).index.unwrap()
    }
    fn all_open(h: &Harness, log: &WalkerLog, t: f64) {
        assert!(
            h.gates.iter().all(|&id| active(log, id, ticks(t))),
            "gates should be open at {t}"
        );
    }

    #[test]
    fn budget_structure() {
        let h = harness(AirSide::Nato, 2, 3, Strategies::default());
        assert_eq!(mcus(named(&h.root, "Budget")), 16);
        assert_eq!(
            nodes(&h.root)
                .iter()
                .filter(|n| n.name().is_some_and(|s| s.starts_with("BUD ")))
                .count(),
            16
        );
        assert!(!nodes(&h.root).iter().any(|n| {
            n.name()
                .is_some_and(|s| s.ends_with(" EVENT") || s.ends_with(" GATE"))
        }));
        for event in h
            .logic
            .budget_events
            .iter()
            .copied()
            .chain([budget_id(&h, "AGAIN")])
        {
            for suffix in ["MARK", "CHECK"] {
                assert!(h.logic.links.contains(&(event, budget_id(&h, suffix))));
            }
        }
        assert_eq!(
            by_id(&h.root, budget_id(&h, "SHUT")).targets,
            [h.gates.clone(), vec![budget_id(&h, "CHECK")]].concat()
        );
        assert_eq!(by_id(&h.root, budget_id(&h, "OPEN")).targets, h.gates);
        assert_eq!(by_id(&h.root, budget_id(&h, "CENSUS")).targets, h.flags);
        for (&id, delay) in h.flags.iter().zip([0.05, 0.10, 0.15]) {
            assert_eq!(number(by_id(&h.root, id), "Time").unwrap(), delay);
            assert_eq!(by_id(&h.root, id).targets, [budget_id(&h, "COUNT")]);
            assert!(h.logic.init_off.contains(&id));
        }
        assert_eq!(
            by_id(&h.root, budget_id(&h, "COUNT")).property("Counter"),
            Some("2")
        );
        assert_eq!(
            by_id(&h.root, budget_id(&h, "COUNT")).property("Dropcount"),
            Some("1")
        );
        assert_eq!(
            number(by_id(&h.root, budget_id(&h, "END")), "Time").unwrap(),
            0.30
        );
        assert!(
            by_id(&h.root, budget_id(&h, "END"))
                .targets
                .contains(&budget_id(&h, "CHECK ON"))
        );
        assert_eq!(
            by_id(&h.root, budget_id(&h, "CHECK ON")).targets,
            [budget_id(&h, "CHECK")]
        );
        assert_eq!(
            number(by_id(&h.root, budget_id(&h, "AGAIN DELAY")), "Time").unwrap(),
            0.05
        );
        assert!(h.logic.init_off.contains(&budget_id(&h, "AGAIN")));
    }

    #[test]
    fn budget_mcu_count_does_not_depend_on_n() {
        for n in 2..=6 {
            let h = harness(AirSide::Nato, n, 3, Strategies::default());
            assert_eq!(mcus(named(&h.root, "Budget")), 16);
        }
    }

    #[test]
    fn budget_n1_has_no_counter() {
        let h = harness(AirSide::Nato, 1, 3, Strategies::default());
        assert_eq!(mcus(named(&h.root, "Budget")), 14);
        for name in ["BUD N COUNT", "BUD N ZERO"] {
            assert!(h.root.find_by_name(name).is_none());
        }
        for &flag in &h.flags {
            assert_eq!(by_id(&h.root, flag).targets, [budget_id(&h, "STAY SHUT")]);
        }
    }

    #[test]
    fn budget_single_slot_structure() {
        for side in [AirSide::Dprk, AirSide::Nato] {
            for n in 1..=6 {
                let h = harness(
                    side,
                    n,
                    3,
                    Strategies {
                        budget: BudgetImpl::SingleSlot,
                        ..Strategies::default()
                    },
                );
                assert_eq!(mcus(named(&h.root, "Budget")), 14);
                assert_eq!(h.root.count_block_type("MCU_Counter"), 0);
            }
        }
    }

    #[test]
    fn budget_census_same_tick_structure() {
        let h = harness(
            AirSide::Nato,
            2,
            3,
            Strategies {
                census: CensusImpl::SameTick,
                ..Strategies::default()
            },
        );
        for &flag in &h.flags {
            assert_eq!(number(by_id(&h.root, flag), "Time").unwrap(), 0.0);
        }
        assert_eq!(
            number(by_id(&h.root, budget_id(&h, "END")), "Time").unwrap(),
            0.2
        );
        for config in configs() {
            let log = run(
                &h,
                config,
                10.0,
                &[
                    (2.0, h.requests[0]),
                    (4.0, h.requests[1]),
                    (6.0, h.requests[2]),
                ],
            );
            assert_eq!(times(&log, h.gates[0]), [ticks(2.0)]);
            assert_eq!(times(&log, h.gates[1]), [ticks(4.0)]);
            assert!(times(&log, h.gates[2]).is_empty());
        }
    }

    #[test]
    fn budget_grants_2_refuses_3rd_then_grants_after_deletion() {
        let h = harness(AirSide::Nato, 2, 3, Strategies::default());
        for config in configs() {
            let log = run(
                &h,
                config,
                90.0,
                &[
                    (2.0, h.requests[0]),
                    (4.0, h.requests[1]),
                    (6.0, h.requests[2]),
                    (10.0, h.done[0]),
                    (40.0, h.requests[2]),
                    (75.6, h.requests[2]),
                ],
            );
            assert_eq!(times(&log, h.gates[0]), [ticks(2.0)]);
            assert_eq!(times(&log, h.gates[1]), [ticks(4.0)]);
            assert_eq!(times(&log, h.deletions[0]), [ticks(70.6)]);
            assert!(active(&log, h.flags[0], ticks(40.0)));
            assert_eq!(times(&log, h.gates[2]), [ticks(75.6)]);
        }
    }

    fn two_deletions(gap: f64) {
        let h = harness(AirSide::Nato, 2, 4, Strategies::default());
        for config in configs() {
            let log = run(
                &h,
                config,
                80.0,
                &[
                    (2.0, h.requests[0]),
                    (4.0, h.requests[1]),
                    (10.0, h.done[0]),
                    (10.0 + gap, h.done[1]),
                    (72.0, h.requests[2]),
                    (74.0, h.requests[3]),
                ],
            );
            assert_eq!(times(&log, h.deletions[0]), [ticks(70.6)]);
            assert_eq!(times(&log, h.deletions[1]), [ticks(70.6 + gap)]);
            all_open(&h, &log, 70.6 + recount_time(CensusImpl::Spaced, 4));
            assert_eq!(times(&log, h.gates[2]), [ticks(72.0)]);
            assert_eq!(times(&log, h.gates[3]), [ticks(74.0)]);
        }
    }

    #[test]
    fn budget_two_deletions_in_one_tick_reopen() {
        two_deletions(0.0);
    }
    #[test]
    fn budget_two_deletions_50_ms_apart_reopen() {
        two_deletions(0.05);
    }

    #[test]
    fn budget_deletion_during_recount_runs_a_second_recount() {
        let h = harness(AirSide::Nato, 2, 3, Strategies::default());
        for config in configs() {
            let log = run(
                &h,
                config,
                4.0,
                &[(2.0, h.requests[0]), (2.2, h.deletions[0])],
            );
            assert_eq!(
                times(&log, budget_id(&h, "CHECK")),
                [ticks(2.0), ticks(2.35)]
            );
            all_open(&h, &log, 4.0);
        }
    }

    #[test]
    fn budget_request_during_recount_is_dropped() {
        let h = harness(AirSide::Nato, 2, 3, Strategies::default());
        for config in configs() {
            let log = run(
                &h,
                config,
                4.0,
                &[(2.0, h.requests[0]), (2.1, h.requests[1])],
            );
            assert_eq!(times(&log, h.gates[0]), [ticks(2.0)]);
            assert!(times(&log, h.gates[1]).is_empty());
            all_open(&h, &log, 4.0);
        }
    }

    #[test]
    fn budget_never_locks_when_nothing_flies() {
        for n in 1..=3 {
            let h = harness(AirSide::Nato, n, 4, Strategies::default());
            for seed in 0..200_u64 {
                let mut rng = seed + 7;
                let mut random = || {
                    rng = rng
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    rng >> 32
                };
                // Every random gap is 0..1 s in hundredths, quantized upward
                // to the existing walker's 0.05 s grid. Ends schedule deletion
                // through the real 60.6 s stand-in delay; later ends are latched.
                let mut centiseconds = 200_u64;
                let mut pulses = Vec::new();
                for _ in 0..36 {
                    centiseconds += random() % 101;
                    let copy = random() as usize % h.gates.len();
                    let id = if random() % 3 == 0 {
                        h.done[copy]
                    } else {
                        h.requests[copy]
                    };
                    pulses.push((centiseconds.div_ceil(5), id.into()));
                }
                // End all remaining copies after the request stream. A copy
                // whose early scripted end already fired must not be launched
                // later: close its request at that end, matching a one-use copy.
                let mut root = h.root.clone();
                for (i, &id) in h.done.iter().enumerate() {
                    let off = named(&root, &format!("SN{:02} DONE HUB OFF", i + 1))
                        .index
                        .unwrap();
                    by_id_mut(&mut root, off)
                        .unwrap()
                        .append_target(h.requests[i]);
                    pulses.push((centiseconds.div_ceil(5) + 20, id.into()));
                }
                let until = centiseconds.div_ceil(5) + ticks(67.0);
                for mut config in configs() {
                    for delivery in [SameTickDelivery::DepthFirst, SameTickDelivery::Batched] {
                        config.same_tick_delivery = delivery;
                        let log = walk(
                            &root,
                            config,
                            &WalkerInput {
                                until,
                                seed,
                                pulses: pulses.clone(),
                                ..WalkerInput::default()
                            },
                        );
                        assert!(
                            h.flags.iter().all(|&id| !active(&log, id, until)),
                            "seed {seed}, {config:?}"
                        );
                        assert!(
                            h.gates.iter().all(|&id| active(&log, id, until)),
                            "seed {seed}, budget {n}, {config:?}"
                        );
                        let mut previous = 0;
                        for snapshot in log.snapshots.iter().filter(|s| s.t >= ticks(0.5)) {
                            let live = h.flags.iter().filter(|id| snapshot.active[id]).count();
                            let requests = log
                                .pulses
                                .iter()
                                .filter(|p| p.t == snapshot.t && h.requests.contains(&p.index))
                                .count();
                            let cap = if delivery == SameTickDelivery::Batched {
                                previous.max(usize::from(n) + requests.saturating_sub(1))
                            } else {
                                usize::from(n)
                            };
                            assert!(
                                live <= cap,
                                "seed {seed}, budget {n}, {config:?}, tick {}, live {live}, cap {cap}",
                                snapshot.t
                            );
                            previous = live;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn budget_requests_in_one_tick_overshoot_then_heal() {
        let h = harness(AirSide::Nato, 1, 3, Strategies::default());
        for mut config in configs() {
            for delivery in [SameTickDelivery::DepthFirst, SameTickDelivery::Batched] {
                config.same_tick_delivery = delivery;
                let log = run(
                    &h,
                    config,
                    40.0,
                    &[
                        (2.0, h.requests[0]),
                        (2.0, h.requests[1]),
                        (10.0, h.requests[2]),
                        (20.0, h.deletions[0]),
                        (22.0, h.requests[2]),
                        (30.0, h.deletions[1]),
                        (32.0, h.requests[2]),
                    ],
                );
                assert_eq!(times(&log, h.gates[0]), [ticks(2.0)]);
                if delivery == SameTickDelivery::Batched {
                    assert_eq!(times(&log, h.gates[1]), [ticks(2.0)]);
                    assert!(!active(&log, h.gates[2], ticks(21.0)));
                    assert_eq!(times(&log, h.gates[2]), [ticks(32.0)]);
                } else {
                    assert!(times(&log, h.gates[1]).is_empty());
                    assert_eq!(times(&log, h.gates[2]), [ticks(22.0)]);
                }
            }
        }
    }

    #[test]
    fn budget_same_copy_started_twice_in_one_tick() {
        use crate::airtask::library::tests::builtin;
        use crate::airtask::shell::test_support::fixture;
        let mut f = fixture(builtin(1), Strategies::default());
        let mut go = template::timer("EN01 GO", 0.0, &mut f.next_id, 0.0, 0.0);
        let mut fire = template::timer("ZN01 FIRE", 0.0, &mut f.next_id, 0.0, 0.0);
        go.set_targets(vec![f.pointer]);
        fire.set_targets(vec![f.pointer]);
        let input = WalkerInput {
            until: ticks(2200.0),
            pulses: vec![
                (ticks(2.0), go.index.unwrap().into()),
                (ticks(2.0), fire.index.unwrap().into()),
            ],
            kills: f
                .shape
                .planes
                .iter()
                .enumerate()
                .map(|(i, &id)| (ticks(10.0 + i as f64), id.into()))
                .collect(),
            ..WalkerInput::default()
        };
        f.root.children.extend([go, fire]);
        f.root.children.extend(build_side(
            AirSide::Nato,
            2,
            &mut f.logic,
            &Strategies::default(),
            &mut f.next_id,
        ));
        let init = emit_side_logic(
            AirSide::Nato,
            &f.logic,
            std::slice::from_mut(&mut f.root),
            &mut f.next_id,
        )
        .unwrap();
        f.root.children.extend(init);
        for mut config in configs() {
            config.same_tick_delivery = SameTickDelivery::Batched;
            let log = walk(&f.root, config, &input);
            assert_eq!(times(&log, f.ids.start), [ticks(2.0), ticks(2.0)]);
            assert_eq!(
                f.logic
                    .flags
                    .iter()
                    .filter(|&&id| active(&log, id, ticks(3.0)))
                    .count(),
                1
            );
            assert_eq!(times(&log, f.ids.done_hub), [ticks(13.0)]);
            assert_eq!(times(&log, f.ids.delete_timer), [ticks(73.6)]);
            assert!(active(&log, f.ids.start, ticks(2200.0)));
            assert!(!active(&log, f.ids.flying, ticks(2200.0)));
            let bring_ups = if config.timer_retrigger == TimerRetrigger::Twice {
                2
            } else {
                1
            };
            assert_eq!(times(&log, f.shape.bring_up_timer).len(), bring_ups);
        }
    }

    #[test]
    fn budget_single_slot_grants_1() {
        for side in [AirSide::Dprk, AirSide::Nato] {
            let h = harness(
                side,
                6,
                3,
                Strategies {
                    budget: BudgetImpl::SingleSlot,
                    ..Strategies::default()
                },
            );
            for config in configs() {
                let log = run(
                    &h,
                    config,
                    10.0,
                    &[
                        (2.0, h.requests[0]),
                        (4.0, h.requests[1]),
                        (6.0, h.deletions[0]),
                        (8.0, h.requests[1]),
                    ],
                );
                assert_eq!(times(&log, h.gates[0]), [ticks(2.0)]);
                assert_eq!(times(&log, h.gates[1]), [ticks(8.0)]);
            }
        }
    }
}
