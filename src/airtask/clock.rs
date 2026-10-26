//! Mission clock: quiet/jitter latch and a weighted, re-armed waterfall (0029 B).

use std::collections::{BTreeMap, BTreeSet};

use super::assemble::{EntryCopies, SideCopies};
use super::shell::{by_id_mut, group};
use super::{
    AirIssue, AirPlan, AirSide, IssueSeverity, Schedule, SideLogic, TriggerMode,
    weighted_random_pct,
};
use crate::ast::Il2Entity;
use crate::template;

const SLOT_WINDOW_CAP_S: f64 = 300.0;

fn clock_entries(entries: &[EntryCopies]) -> impl Iterator<Item = &EntryCopies> {
    entries
        .iter()
        .filter(|entry| matches!(entry.mode, TriggerMode::Clock | TriggerMode::Both))
}

// Return A and the uncapped window; validation and construction share the formula.
fn slot_timing(entries: &[EntryCopies]) -> (f64, f64) {
    let eligible: Vec<_> = clock_entries(entries).collect();
    let areas: BTreeSet<_> = eligible.iter().map(|entry| entry.area).collect();
    let attempt = 0.5 * (eligible.len() as f64 + 1.0) + 0.5;
    (attempt, (areas.len() as f64 + 10.0) * attempt)
}

/// Schedule diagnostics use assembly's eligibility, including shared areas.
/// Step 7 calls this per side before building the mission.
pub(crate) fn validate_schedule(
    side: AirSide,
    schedule: &Schedule,
    entries: &[EntryCopies],
) -> Vec<AirIssue> {
    if !schedule.enabled || clock_entries(entries).next().is_none() {
        return Vec::new();
    }
    let error = |message: &str| AirIssue {
        severity: IssueSeverity::Error,
        text: format!("Air Tasking {} clock: {message}", side.label()),
        rail_only: false,
    };
    let mut issues = Vec::new();
    if schedule.jitter_steps == 0 {
        issues.push(error("jitter steps must be at least 1"));
    }
    if schedule.quiet_pct > 100 {
        issues.push(error("quiet chance must be 0 to 100 %"));
    }
    for (name, minutes) in [
        ("first slot time", schedule.first_slot_min),
        ("period", schedule.period_min),
    ] {
        if !minutes.is_finite() || minutes <= 0.0 || !(minutes * 60.0).is_finite() {
            issues.push(error(&format!(
                "{name} must be a positive number of minutes, finite in both minutes and seconds"
            )));
        }
    }
    if !schedule.jitter_step_min.is_finite()
        || schedule.jitter_step_min < 0.0
        || !(schedule.jitter_step_min * 60.0 * f64::from(schedule.jitter_steps)).is_finite()
    {
        issues.push(error(
            "jitter step must be a finite number of minutes, at least zero, with a finite total span",
        ));
    }
    if !issues.is_empty() {
        return issues;
    }
    let (attempt, window) = slot_timing(entries);
    let minimum = f64::from(schedule.jitter_steps - 1) * schedule.jitter_step_min * 60.0
        + window.min(SLOT_WINDOW_CAP_S)
        + attempt
        + 10.0;
    if schedule.period_min * 60.0 <= minimum {
        issues.push(error(&format!(
            "period must exceed {minimum:.1} s (jitter span + slot window + attempt + 10 s)"
        )));
    }
    if window > SLOT_WINDOW_CAP_S {
        issues.push(AirIssue {
            severity: IssueSeverity::Warning,
            text: format!(
                "Air Tasking {} clock: slot window is capped at {SLOT_WINDOW_CAP_S:.0} s (uncapped {window:.1} s); some areas may go untried",
                side.label()
            ),
            rail_only: false,
        });
    }
    issues
}

/// Fixed SplitMix64 Fisher-Yates shuffle, shared with step 6's zone stagger.
/// No platform-dependent hashing or global random state enters the ordering.
pub(crate) fn waterfall_order(seed: u64, side: AirSide, n: usize) -> Vec<usize> {
    let mut state = seed
        ^ match side {
            AirSide::Dprk => 0x243f6a8885a308d3,
            AirSide::Nato => 0x13198a2e03707344,
        };
    let mut order: Vec<_> = (0..n).collect();
    for i in (1..n).rev() {
        state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^= value >> 31;
        order.swap(i, (value % (i as u64 + 1)) as usize);
    }
    order
}

/// Return one Clock group for copies.trees[0]. Stage both allocation and wiring;
/// the clock owns no copy, threshold, or budget MCU.
pub(crate) fn build_side(
    side: AirSide,
    plan: &AirPlan,
    copies: &SideCopies,
    logic: &mut SideLogic,
    next_id: &mut i32,
) -> Result<Vec<Il2Entity>, String> {
    let schedule = &plan.schedule[side.idx()];
    let eligible: Vec<_> = clock_entries(&copies.entries).collect();
    if !schedule.enabled || eligible.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(issue) = validate_schedule(side, schedule, &copies.entries)
        .into_iter()
        .find(|issue| issue.severity == IssueSeverity::Error)
    {
        return Err(issue.text);
    }
    let error = |message: &str| format!("Air Tasking {} clock: {message}", side.label());
    let entries: Vec<_> = waterfall_order(plan.seed, side, eligible.len())
        .into_iter()
        .map(|i| eligible[i])
        .collect();
    let mut weights = Vec::new();
    for entry in &entries {
        let pool = plan
            .pool
            .get(entry.pool_index)
            .ok_or_else(|| error(&format!("entry {} has no pool entry", entry.number)))?;
        if !(1..=10).contains(&pool.weight) {
            return Err(error(&format!(
                "entry {} weight must be 1 to 10",
                entry.number
            )));
        }
        weights.push(u32::from(pool.weight));
    }

    let mut alloc = *next_id;
    let mut registry = logic.clone();
    let mut root = group("Clock", &mut alloc);
    let prefix = format!("CLK {}", side.letter());
    let (attempt, window) = slot_timing(&copies.entries);
    let mut ids = BTreeMap::new();
    for (role, kind, time) in [
        ("FIRST", "MCU_Timer", schedule.first_slot_min * 60.0),
        ("TICK", "MCU_Timer", 0.0),
        ("PERIOD", "MCU_Timer", schedule.period_min * 60.0),
        ("SLOT ON", "MCU_Activate", 0.0),
        ("SLOT OPEN", "MCU_Timer", 0.0),
        ("SLOT OPEN OFF", "MCU_Deactivate", 0.0),
        ("NO REPEAT", "MCU_Timer", 0.1),
        ("REROLL ON", "MCU_Activate", 0.0),
        ("SLOT CLOSE", "MCU_Timer", window.min(SLOT_WINDOW_CAP_S)),
        ("DRAW", "MCU_Timer", 0.3),
        ("REROLL OFF", "MCU_Deactivate", 0.0),
        ("LAST CLEAR", "MCU_Deactivate", 0.0),
        ("Randomizer:INPUT", "MCU_Timer", 0.0),
        ("ReOpen Outputs", "MCU_Activate", 0.0),
        ("ATTEMPT END", "MCU_Timer", attempt),
        ("REROLL", "MCU_Timer", 0.0),
        ("WIN", "MCU_Deactivate", 0.0),
    ] {
        let name = format!("{prefix} {role}");
        let node = if kind == "MCU_Timer" {
            template::timer(&name, time, &mut alloc, 0.0, 0.0)
        } else {
            template::mcu(kind, &name, &mut alloc, 0.0, 0.0)
        };
        ids.insert(role, node.index.unwrap());
        root.children.push(node);
    }
    let jitter_steps = usize::from(schedule.jitter_steps);
    let quiet_pct = u32::from(schedule.quiet_pct);
    let mut jitter_weights = vec![100 - quiet_pct; jitter_steps];
    jitter_weights.push(quiet_pct * jitter_steps as u32);
    let mut tick = vec![ids["PERIOD"], ids["SLOT ON"]];
    for (k, pct) in weighted_random_pct(&jitter_weights)
        .into_iter()
        .take(jitter_steps)
        .enumerate()
    {
        let mut node = template::timer_random(
            &format!("{prefix} JITTER {}", k + 1),
            0.1 + k as f64 * schedule.jitter_step_min * 60.0,
            pct,
            &mut alloc,
            0.0,
            0.0,
        );
        node.set_targets(vec![ids["SLOT OPEN"]]);
        tick.push(node.index.unwrap());
        root.children.push(node);
    }
    let areas: BTreeSet<_> = entries.iter().map(|entry| entry.area).collect();
    let mut area_out = BTreeMap::new();
    for area in areas {
        let node = template::mcu(
            "MCU_Deactivate",
            &format!("{prefix} AREA {area:02} OUT"),
            &mut alloc,
            0.0,
            0.0,
        );
        area_out.insert(area, node.index.unwrap());
        root.children.push(node);
    }
    let percentages = weighted_random_pct(&weights);
    let mut entry_ids = Vec::new();
    for (k, entry) in entries.iter().enumerate() {
        let mut cell = BTreeMap::new();
        for (role, kind) in [
            ("AVAILABLE", "MCU_Timer"),
            ("RANDOM ON", "MCU_Activate"),
            ("LAST", "MCU_Timer"),
            ("EXCLUDE", "MCU_Deactivate"),
            ("LAST SET", "MCU_Activate"),
            ("RANDOM", "MCU_Timer"),
            ("OUT", "MCU_Timer"),
            ("CLOSE", "MCU_Deactivate"),
            ("HOT", "MCU_Timer"),
            ("COLD", "MCU_Timer"),
            ("GO", "MCU_Timer"),
        ] {
            if role == "CLOSE" && k + 1 == entries.len() {
                continue;
            }
            let name = format!("E{}{:02} {role}", side.letter(), entry.number);
            let node = if role == "RANDOM" {
                template::timer_random(
                    &name,
                    0.5 * (k + 1) as f64,
                    percentages[k],
                    &mut alloc,
                    0.0,
                    0.0,
                )
            } else if kind == "MCU_Timer" {
                template::timer(&name, 0.0, &mut alloc, 0.0, 0.0)
            } else {
                template::mcu(kind, &name, &mut alloc, 0.0, 0.0)
            };
            cell.insert(role, node.index.unwrap());
            root.children.push(node);
        }
        registry
            .hot_consumers
            .entry(entry.area)
            .or_default()
            .push(cell["HOT"]);
        registry
            .cold_consumers
            .entry(entry.area)
            .or_default()
            .push(cell["COLD"]);
        registry.init_off.push(cell["LAST"]);
        registry.links.extend(
            entry
                .copies
                .iter()
                .map(|copy| (copy.shell.start, cell["LAST SET"])),
        );
        registry
            .links
            .extend(["AVAILABLE", "RANDOM", "GO"].map(|role| (entry.exhausted, cell[role])));
        entry_ids.push(cell);
    }
    let all = |role: &str| entry_ids.iter().map(|cell| cell[role]).collect::<Vec<_>>();
    let mut open = vec![ids["SLOT OPEN OFF"]];
    open.extend(all("AVAILABLE"));
    open.extend([
        ids["NO REPEAT"],
        ids["REROLL ON"],
        ids["SLOT CLOSE"],
        ids["DRAW"],
    ]);
    let mut input = vec![ids["ReOpen Outputs"]];
    input.extend(all("RANDOM"));
    for (from, to) in [
        ("FIRST", vec![ids["TICK"]]),
        ("TICK", tick),
        ("PERIOD", vec![ids["TICK"]]),
        ("SLOT ON", vec![ids["SLOT OPEN"]]),
        ("SLOT OPEN", open),
        ("SLOT OPEN OFF", vec![ids["SLOT OPEN"]]),
        ("NO REPEAT", all("LAST")),
        ("REROLL ON", vec![ids["REROLL"], ids["ATTEMPT END"]]),
        ("SLOT CLOSE", vec![ids["REROLL OFF"]]),
        ("REROLL OFF", vec![ids["REROLL"]]),
        (
            "DRAW",
            vec![
                ids["LAST CLEAR"],
                ids["Randomizer:INPUT"],
                ids["ATTEMPT END"],
            ],
        ),
        ("LAST CLEAR", all("LAST")),
        ("Randomizer:INPUT", input),
        ("ReOpen Outputs", all("OUT")),
        ("ATTEMPT END", vec![ids["REROLL"]]),
        ("REROLL", vec![ids["Randomizer:INPUT"], ids["ATTEMPT END"]]),
        ("WIN", vec![ids["ATTEMPT END"], ids["REROLL"]]),
    ] {
        by_id_mut(&mut root, ids[from]).unwrap().set_targets(to);
    }
    for (k, (entry, cell)) in entries.iter().zip(&entry_ids).enumerate() {
        let mut random = vec![cell["OUT"]];
        if let Some(&closer) = cell.get("CLOSE") {
            random.push(closer);
            by_id_mut(&mut root, closer).unwrap().set_targets(
                entry_ids[k + 1..]
                    .iter()
                    .map(|later| later["OUT"])
                    .collect(),
            );
        }
        let mut go = vec![ids["WIN"]];
        // START activates COPY r+1. Visit it before COPY r so depth-first
        // delivery cannot consume a newly activated pointer in this same tick.
        go.extend(entry.copies.iter().rev().map(|copy| copy.pointer));
        for (from, to) in [
            ("AVAILABLE", vec![cell["RANDOM ON"]]),
            ("RANDOM ON", vec![cell["RANDOM"]]),
            ("LAST", vec![cell["EXCLUDE"]]),
            ("EXCLUDE", vec![cell["RANDOM"]]),
            ("LAST SET", vec![cell["LAST"]]),
            ("RANDOM", random),
            ("OUT", vec![cell["HOT"], cell["COLD"]]),
            ("HOT", vec![cell["GO"]]),
            ("COLD", vec![area_out[&entry.area]]),
            ("GO", go),
        ] {
            by_id_mut(&mut root, cell[from]).unwrap().set_targets(to);
        }
    }
    for (area, id) in area_out {
        by_id_mut(&mut root, id).unwrap().set_targets(
            entries
                .iter()
                .zip(&entry_ids)
                .filter(|(entry, _)| entry.area == area)
                .map(|(_, cell)| cell["RANDOM"])
                .collect(),
        );
    }
    // J1 cancels ATTEMPT END directly; REROLL also rejects a late expiry if
    // that timer runs on. SLOT OPEN re-arms both before DRAW can reach either.
    registry.init_off.push(ids["REROLL"]);
    registry.mission_begin.push(ids["FIRST"]);
    *logic = registry;
    *next_id = alloc;
    Ok(vec![root])
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::airtask::assemble::{
        assemble_side,
        tests::{context, finish},
    };
    use crate::airtask::library::{by_id, nodes, number};
    use crate::airtask::shell::test_support::{active, mcus, named, ticks, times};
    use crate::airtask::testkit::*;
    use crate::airtask::{
        AirBudget, NATO, PoolEntry, TargetRef, budget, builtin_library, threshold, zone,
    };

    fn clock_test_plan() -> AirPlan {
        let mut plan = AirPlan {
            library: builtin_library(),
            budget: AirBudget { per_side: [6, 6] },
            seed: 7,
            ..AirPlan::default()
        };
        for xz in [
            (150_000.0, 200_000.0),
            (150_000.0, 225_000.0),
            (150_000.0, 250_000.0),
        ] {
            let mut entry = PoolEntry::new(
                plan.library[1].id.clone(),
                TargetRef::Objective {
                    side: AirSide::Nato,
                    xz,
                },
            );
            entry.runs = 6;
            plan.pool.push(entry);
        }
        plan
    }

    pub(crate) struct Harness {
        pub(crate) root: Il2Entity,
        pub(crate) copies: SideCopies,
        pub(crate) logic: SideLogic,
        pub(crate) next: i32,
    }

    pub(crate) fn assembled(plan: &AirPlan) -> (SideCopies, SideLogic, i32) {
        let targets: Vec<_> = plan
            .pool
            .iter()
            .filter_map(|entry| match entry.target {
                TargetRef::Objective {
                    side: AirSide::Nato,
                    xz,
                } if xz != (0.0, 0.0) => Some(xz),
                _ => None,
            })
            .collect();
        let mut ctx = context();
        ctx.nato_objectives = &targets;
        let mut logic = SideLogic::default();
        let mut next = 1;
        let copies = assemble_side(AirSide::Nato, plan, &ctx, &mut logic, &mut next).unwrap();
        (copies, logic, next)
    }

    pub(crate) fn harness(plan: &AirPlan, done_delay_s: f64) -> Harness {
        let (mut copies, mut logic, mut next) = assembled(plan);
        let clock = build_side(AirSide::Nato, plan, &copies, &mut logic, &mut next).unwrap();
        copies.trees[0].children.extend(clock);
        let zone = zone::build_side(AirSide::Nato, plan, &copies, &mut logic, &mut next).unwrap();
        copies.trees[0].children.extend(zone);
        copies.trees[0].children.extend(
            threshold::build_side(
                AirSide::Nato,
                &copies.areas,
                &plan.threshold,
                &plan.strategies,
                &mut logic,
                &mut next,
            )
            .unwrap(),
        );
        copies.trees[0].children.extend(budget::build_side(
            AirSide::Nato,
            plan.budget.per_side[NATO],
            &mut logic,
            &plan.strategies,
            &mut next,
        ));
        // The walker does not fly AI. End every launched copy after the delay,
        // through the real DONE HUB and its real RTB/deletion/budget chain.
        for copy in copies.entries.iter().flat_map(|entry| &entry.copies) {
            let mut done = template::timer(
                &format!("Test SN{:02} DONE", copy.number),
                done_delay_s,
                &mut next,
                0.0,
                0.0,
            );
            done.set_targets(vec![copy.shell.done_hub]);
            logic.links.push((copy.shell.start, done.index.unwrap()));
            copies.trees[0].children.push(done);
        }
        let root = finish(AirSide::Nato, &mut copies, &logic, &mut next);
        Harness {
            root,
            copies,
            logic,
            next,
        }
    }

    fn clock_id(h: &Harness, role: &str) -> i32 {
        named(&h.root, &format!("CLK N {role}")).index.unwrap()
    }

    pub(crate) fn entry_id(h: &Harness, entry: usize, role: &str) -> i32 {
        named(&h.root, &format!("EN{entry:02} {role}"))
            .index
            .unwrap()
    }

    fn clock_only(plan: &AirPlan) -> (Il2Entity, SideCopies, SideLogic) {
        let (copies, mut logic, mut next) = assembled(plan);
        let before = copies.clone();
        let clock = build_side(AirSide::Nato, plan, &copies, &mut logic, &mut next)
            .unwrap()
            .remove(0);
        assert_eq!(
            copies, before,
            "clock must not edit another builder's output"
        );
        (clock, copies, logic)
    }

    // Last-first pointers are safe with depth-first forward delivery. Batched
    // delivery models J3 in either target order (0029's required GO check).
    pub(crate) fn configs() -> [WalkerConfig; 3] {
        [
            (SameTickDelivery::DepthFirst, false),
            (SameTickDelivery::Batched, false),
            (SameTickDelivery::Batched, true),
        ]
        .map(|(same_tick_delivery, reverse_targets)| WalkerConfig {
            same_tick_delivery,
            reverse_targets,
            deactivate_running_timer: DeactivateRunningTimer::Cancels,
            ..WalkerConfig::default()
        })
    }

    pub(crate) fn loiter(h: &Harness, areas: &[usize]) -> Vec<ScriptedTrack> {
        areas
            .iter()
            .map(|&area| {
                let (x, z) = h.copies.areas[area - 1];
                ScriptedTrack {
                    country: threshold::DPRK_PLAYER_COUNTRY,
                    coalition: 1,
                    // Absent at mission load, enters at 1 s, then holds position
                    // inside r_in. The walker explicitly holds the last sample.
                    samples: vec![(ticks(1.0), x + 1000.0, z)],
                }
            })
            .collect()
    }

    fn run(
        h: &Harness,
        config: WalkerConfig,
        seed: u64,
        until: f64,
        hot: &[usize],
        pulses: &[(f64, i32)],
    ) -> WalkerLog {
        walk(
            &h.root,
            config,
            &WalkerInput {
                seed,
                until: ticks(until),
                tracks: loiter(h, hot),
                pulses: pulses
                    .iter()
                    .map(|&(t, id)| (ticks(t), id.into()))
                    .collect(),
                ..WalkerInput::default()
            },
        )
    }

    fn plan_with_entries(n: usize, areas: usize) -> AirPlan {
        let mut plan = clock_test_plan();
        let entry = plan.pool[0].clone();
        plan.pool = (0..n)
            .map(|i| {
                let mut entry = entry.clone();
                entry.target = TargetRef::Objective {
                    side: AirSide::Nato,
                    xz: (150_000.0, 150_000.0 + (i % areas) as f64 * 25_000.0),
                };
                entry
            })
            .collect();
        plan
    }

    fn fast_plan(n: usize, areas: usize) -> AirPlan {
        let mut plan = plan_with_entries(n, areas);
        // Engineered draws keep the real dwell and deletion/recount delays.
        plan.schedule[NATO].first_slot_min = 80.0 / 60.0;
        plan.schedule[NATO].period_min = 180.0 / 60.0;
        plan.schedule[NATO].jitter_steps = 1;
        plan.schedule[NATO].quiet_pct = 0;
        plan
    }

    pub(crate) fn starts<'a>(log: &'a WalkerLog, h: &Harness) -> Vec<&'a WalkerPulse> {
        log.pulses
            .iter()
            .filter(|pulse| h.logic.gates.contains(&pulse.index))
            .collect()
    }

    fn gos<'a>(log: &'a WalkerLog, h: &Harness) -> Vec<&'a WalkerPulse> {
        let ids: Vec<_> = clock_entries(&h.copies.entries)
            .map(|entry| entry_id(h, entry.number, "GO"))
            .collect();
        log.pulses
            .iter()
            .filter(|pulse| ids.contains(&pulse.index))
            .collect()
    }

    fn start_entry(h: &Harness, start: i32) -> usize {
        h.copies
            .entries
            .iter()
            .find(|entry| entry.copies.iter().any(|copy| copy.shell.start == start))
            .unwrap()
            .number
    }

    fn assert_slots(h: &Harness, log: &WalkerLog, expected: &[f64]) {
        assert_eq!(
            times(log, clock_id(h, "SLOT OPEN")),
            expected.iter().map(|&t| ticks(t)).collect::<Vec<_>>()
        );
    }

    // A quiet period has no passing jitter timer; otherwise the first pass opens it.
    fn default_slots(h: &Harness, log: &WalkerLog, count: usize) -> Vec<Option<Tick>> {
        let ticks_at: Vec<_> = (0..count)
            .map(|i| ticks(600.0 + 1200.0 * i as f64))
            .collect();
        assert_eq!(times(log, clock_id(h, "FIRST")), [ticks(600.0)]);
        assert_eq!(times(log, clock_id(h, "SLOT ON")), ticks_at);
        let jitter: Vec<_> = (1..=3)
            .map(|step| times(log, clock_id(h, &format!("JITTER {step}"))))
            .collect();
        for (step, passed) in jitter.iter().enumerate() {
            assert!(passed.iter().all(|time| {
                ticks_at
                    .iter()
                    .any(|tick| *time == tick + ticks(0.1 + 180.0 * step as f64))
            }));
        }
        let slots: Vec<_> = ticks_at
            .iter()
            .map(|tick| {
                jitter
                    .iter()
                    .flatten()
                    .copied()
                    .filter(|time| (*tick..tick + ticks(1200.0)).contains(time))
                    .min()
            })
            .collect();
        assert_eq!(
            times(log, clock_id(h, "SLOT OPEN")),
            slots.iter().flatten().copied().collect::<Vec<_>>()
        );
        slots
    }

    fn assert_wins_match_requests(h: &Harness, log: &WalkerLog) {
        assert_eq!(
            times(log, clock_id(h, "WIN")),
            gos(log, h).iter().map(|pulse| pulse.t).collect::<Vec<_>>()
        );
        for pulse in starts(log, h) {
            assert_eq!(
                pulse.name,
                format!(
                    "SN{:02} START",
                    h.copies
                        .entries
                        .iter()
                        .flat_map(|entry| &entry.copies)
                        .find(|copy| copy.shell.start == pulse.index)
                        .unwrap()
                        .number
                )
            );
        }
    }

    #[test]
    fn clock_structure_three_entries() {
        let plan = clock_test_plan();
        let (root, copies, _) = clock_only(&plan);
        assert_eq!(root.name(), Some("Clock"));
        let id = |name: &str| named(&root, name).index.unwrap();
        for (role, time) in [
            ("FIRST", 600.0),
            ("PERIOD", 1200.0),
            ("ATTEMPT END", 2.5),
            ("DRAW", 0.3),
            ("NO REPEAT", 0.1),
        ] {
            assert_eq!(
                number(named(&root, &format!("CLK N {role}")), "Time"),
                Ok(time)
            );
        }
        for role in ["FIRST", "PERIOD"] {
            assert_eq!(
                named(&root, &format!("CLK N {role}")).targets,
                [id("CLK N TICK")]
            );
        }
        assert_eq!(
            named(&root, "CLK N TICK").targets,
            ["PERIOD", "SLOT ON", "JITTER 1", "JITTER 2", "JITTER 3"]
                .map(|role| id(&format!("CLK N {role}")))
        );
        for (k, (time, pct)) in [(0.1, 27), (180.1, 36), (360.1, 57)]
            .into_iter()
            .enumerate()
        {
            let node = named(&root, &format!("CLK N JITTER {}", k + 1));
            assert_eq!(number(node, "Time"), Ok(time));
            assert_eq!(number(node, "Random"), Ok(f64::from(pct)));
            assert_eq!(node.targets, [id("CLK N SLOT OPEN")]);
        }
        for (role, kind) in [
            ("SLOT ON", "MCU_Activate"),
            ("SLOT OPEN OFF", "MCU_Deactivate"),
        ] {
            let node = named(&root, &format!("CLK N {role}"));
            assert_eq!(node.block_type, kind);
            assert_eq!(node.targets, [id("CLK N SLOT OPEN")]);
        }
        let order = waterfall_order(plan.seed, AirSide::Nato, 3);
        for (k, &i) in order.iter().enumerate() {
            let entry = &copies.entries[i];
            let node = named(&root, &format!("EN{:02} RANDOM", entry.number));
            assert_eq!(number(node, "Time"), Ok(0.5 * (k + 1) as f64));
            assert_eq!(number(node, "Random"), Ok([33.0, 50.0, 100.0][k]));
            let mut targets = vec![id(&format!("EN{:02} OUT", entry.number))];
            if k < 2 {
                let closer = named(&root, &format!("EN{:02} CLOSE", entry.number));
                targets.push(closer.index.unwrap());
                assert_eq!(closer.block_type, "MCU_Deactivate");
                assert_eq!(
                    closer.targets,
                    order[k + 1..]
                        .iter()
                        .map(|&i| id(&format!("EN{:02} OUT", copies.entries[i].number)))
                        .collect::<Vec<_>>()
                );
            }
            assert_eq!(node.targets, targets);
        }
        for node in &root.children {
            assert!(node.objects.is_empty());
            assert_eq!(node.pos_xz(), Some((0.0, 0.0)));
            for forbidden in [
                "QUIET ROLL",
                "Wait for Output",
                "CloseInput",
                "ATTEMPT ARM",
                "ATTEMPT END OUT",
                "DRAW DELAY",
            ] {
                assert!(!node.name().unwrap().contains(forbidden));
            }
        }
    }

    #[test]
    fn clock_entry_relays() {
        let h = harness(&clock_test_plan(), 60.0);
        for entry in &h.copies.entries {
            let id = |role| entry_id(&h, entry.number, role);
            for role in ["AVAILABLE", "LAST", "OUT", "HOT", "COLD", "GO"] {
                let node = by_id(&h.root, id(role));
                assert_eq!(node.block_type, "MCU_Timer");
                assert_eq!(number(node, "Time"), Ok(0.0));
                assert_eq!(number(node, "Random"), Ok(100.0));
            }
            for (role, kind, targets) in [
                ("AVAILABLE", "MCU_Timer", vec![id("RANDOM ON")]),
                ("RANDOM ON", "MCU_Activate", vec![id("RANDOM")]),
                ("LAST", "MCU_Timer", vec![id("EXCLUDE")]),
                ("EXCLUDE", "MCU_Deactivate", vec![id("RANDOM")]),
                ("LAST SET", "MCU_Activate", vec![id("LAST")]),
                ("OUT", "MCU_Timer", vec![id("HOT"), id("COLD")]),
                ("HOT", "MCU_Timer", vec![id("GO")]),
                (
                    "COLD",
                    "MCU_Timer",
                    vec![clock_id(&h, &format!("AREA {:02} OUT", entry.area))],
                ),
            ] {
                assert_eq!(by_id(&h.root, id(role)).block_type, kind);
                assert_eq!(by_id(&h.root, id(role)).targets, targets);
            }
            assert!(h.logic.hot_consumers[&entry.area].contains(&id("HOT")));
            assert!(h.logic.cold_consumers[&entry.area].contains(&id("COLD")));
            let mut go = vec![clock_id(&h, "WIN")];
            go.extend(entry.copies.iter().rev().map(|copy| copy.pointer));
            assert_eq!(by_id(&h.root, id("GO")).targets, go);
            assert!(
                h.root
                    .find_by_name(&format!("EN{:02} MISS", entry.number))
                    .is_none()
            );
        }
    }

    #[test]
    fn clock_mcu_counts() {
        let plan = clock_test_plan();
        let (root, _, _) = clock_only(&plan);
        let counts: Vec<_> = waterfall_order(plan.seed, AirSide::Nato, 3)
            .iter()
            .map(|i| {
                root.children
                    .iter()
                    .filter(|node| {
                        node.name()
                            .unwrap()
                            .starts_with(&format!("EN{:02} ", i + 1))
                    })
                    .count()
            })
            .collect();
        assert_eq!(counts, [11, 11, 10]);
        assert_eq!(
            root.children
                .iter()
                .filter(|node| node.name().unwrap().starts_with("CLK N "))
                .count(),
            23
        );
        assert_eq!(mcus(&root), 55);
    }

    #[test]
    fn clock_mcu_counts_follow_jitter_and_areas() {
        for (jitter, side_count, total) in [(1, 19, 51), (5, 23, 55)] {
            let mut plan = fast_plan(3, 1);
            plan.schedule[NATO].jitter_steps = jitter;
            plan.schedule[NATO].jitter_step_min = 0.1;
            let (root, _, _) = clock_only(&plan);
            assert_eq!(
                root.children
                    .iter()
                    .filter(|node| node.name().unwrap().starts_with("CLK N "))
                    .count(),
                side_count
            );
            assert_eq!(mcus(&root), total);
            assert_eq!(named(&root, "CLK N AREA 01 OUT").targets.len(), 3);
        }
    }

    #[test]
    fn clock_rearm_edges() {
        let h = harness(&clock_test_plan(), 60.0);
        let order = waterfall_order(7, AirSide::Nato, 3);
        let all = |role: &str| {
            order
                .iter()
                .map(|i| entry_id(&h, i + 1, role))
                .collect::<Vec<_>>()
        };
        for (role, kind, targets) in [
            (
                "DRAW",
                "MCU_Timer",
                vec![
                    clock_id(&h, "LAST CLEAR"),
                    clock_id(&h, "Randomizer:INPUT"),
                    clock_id(&h, "ATTEMPT END"),
                ],
            ),
            (
                "REROLL",
                "MCU_Timer",
                vec![
                    clock_id(&h, "Randomizer:INPUT"),
                    clock_id(&h, "ATTEMPT END"),
                ],
            ),
            (
                "WIN",
                "MCU_Deactivate",
                vec![clock_id(&h, "ATTEMPT END"), clock_id(&h, "REROLL")],
            ),
            (
                "REROLL ON",
                "MCU_Activate",
                vec![clock_id(&h, "REROLL"), clock_id(&h, "ATTEMPT END")],
            ),
            ("ATTEMPT END", "MCU_Timer", vec![clock_id(&h, "REROLL")]),
            ("SLOT CLOSE", "MCU_Timer", vec![clock_id(&h, "REROLL OFF")]),
            ("REROLL OFF", "MCU_Deactivate", vec![clock_id(&h, "REROLL")]),
            ("ReOpen Outputs", "MCU_Activate", all("OUT")),
            ("LAST CLEAR", "MCU_Deactivate", all("LAST")),
            ("NO REPEAT", "MCU_Timer", all("LAST")),
        ] {
            assert_eq!(by_id(&h.root, clock_id(&h, role)).block_type, kind);
            assert_eq!(by_id(&h.root, clock_id(&h, role)).targets, targets);
        }
        let mut input = vec![clock_id(&h, "ReOpen Outputs")];
        input.extend(all("RANDOM"));
        assert_eq!(
            by_id(&h.root, clock_id(&h, "Randomizer:INPUT")).targets,
            input
        );
        let mut open = vec![clock_id(&h, "SLOT OPEN OFF")];
        open.extend(all("AVAILABLE"));
        open.extend(
            ["NO REPEAT", "REROLL ON", "SLOT CLOSE", "DRAW"].map(|role| clock_id(&h, role)),
        );
        assert_eq!(by_id(&h.root, clock_id(&h, "SLOT OPEN")).targets, open);
        for entry in &h.copies.entries {
            let last_set = entry_id(&h, entry.number, "LAST SET");
            assert!(
                !by_id(&h.root, entry_id(&h, entry.number, "GO"))
                    .targets
                    .contains(&last_set)
            );
            for copy in &entry.copies {
                assert!(h.logic.links.contains(&(copy.shell.start, last_set)));
                assert!(by_id(&h.root, copy.shell.start).targets.contains(&last_set));
            }
        }
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
    }

    #[test]
    fn clock_last_copy_disables_available_random_and_go() {
        let h = harness(&clock_test_plan(), 60.0);
        for entry in &h.copies.entries {
            let targets =
                ["AVAILABLE", "RANDOM", "GO"].map(|role| entry_id(&h, entry.number, role));
            assert_eq!(by_id(&h.root, entry.exhausted).targets, targets);
            for target in targets {
                assert!(h.logic.links.contains(&(entry.exhausted, target)));
            }
        }
    }

    #[test]
    fn clock_jitter_odds_fold_the_quiet_roll() {
        for (quiet_pct, jitter_steps, expected) in [
            (0, 1, vec![100]),
            (0, 3, vec![33, 50, 100]),
            (20, 1, vec![80]),
            (20, 3, vec![27, 36, 57]),
            (50, 1, vec![50]),
            (50, 3, vec![17, 20, 25]),
            (100, 3, vec![0, 0, 0]),
        ] {
            let mut plan = clock_test_plan();
            plan.schedule[NATO].quiet_pct = quiet_pct;
            plan.schedule[NATO].jitter_steps = jitter_steps;
            let (root, _, _) = clock_only(&plan);
            let mut missed = 1.0;
            for (i, pct) in expected.into_iter().enumerate() {
                let node = named(&root, &format!("CLK N JITTER {}", i + 1));
                assert_eq!(number(node, "Random"), Ok(f64::from(pct)));
                missed *= 1.0 - f64::from(pct) / 100.0;
            }
            assert!((100.0 * missed - f64::from(quiet_pct)).abs() < 1.0);
        }
    }

    // Schedule/count tests need assembled metadata, not 864 placed sorties.
    fn metadata(n: usize, areas: usize) -> Vec<EntryCopies> {
        (0..n)
            .map(|i| EntryCopies {
                pool_index: i,
                number: i + 1,
                area: i % areas + 1,
                mode: TriggerMode::Clock,
                exhausted: -(i as i32 + 1),
                copies: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn clock_slot_window_formula() {
        for (n, areas, attempt, window) in [
            (3, 3, 2.5, 32.5),
            (3, 1, 2.5, 27.5),
            (16, 8, 9.0, 162.0),
            (144, 144, 73.0, 300.0),
        ] {
            let mut plan = clock_test_plan();
            plan.pool = vec![plan.pool[0].clone(); n];
            let copies = SideCopies {
                entries: metadata(n, areas),
                trees: Vec::new(),
                areas: Vec::new(),
                warnings: Vec::new(),
            };
            let root = build_side(
                AirSide::Nato,
                &plan,
                &copies,
                &mut SideLogic::default(),
                &mut 1,
            )
            .unwrap()
            .remove(0);
            assert_eq!(
                number(named(&root, "CLK N ATTEMPT END"), "Time"),
                Ok(attempt)
            );
            assert_eq!(number(named(&root, "CLK N SLOT CLOSE"), "Time"), Ok(window));
        }
    }

    #[test]
    fn clock_ignores_zone_only_and_ineligible_entries() {
        let mut plan = clock_test_plan();
        plan.pool[1].mode = TriggerMode::Zone;
        plan.pool[2].mode = TriggerMode::Both;
        let mut disabled = plan.pool[0].clone();
        disabled.enabled = false;
        plan.pool.push(disabled);
        let mut orphan = plan.pool[0].clone();
        orphan.target = TargetRef::Objective {
            side: AirSide::Nato,
            xz: (0.0, 0.0),
        };
        plan.pool.push(orphan);
        let mut expired = plan.library[1].clone();
        expired.id = "expired".into();
        expired.valid.to = (1950, 1, 1);
        let mut invalid = plan.pool[0].clone();
        invalid.sortie_id = expired.id.clone();
        plan.library.push(expired);
        plan.pool.push(invalid);
        let mut other_side = plan.pool[0].clone();
        other_side.sortie_id = plan.library[5].id.clone();
        other_side.target = TargetRef::Objective {
            side: AirSide::Dprk,
            xz: (180_000.0, 260_000.0),
        };
        plan.pool.push(other_side);
        let (root, copies, _) = clock_only(&plan);
        assert_eq!(
            copies
                .entries
                .iter()
                .map(|e| e.pool_index)
                .collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(
            nodes(&root)
                .iter()
                .filter(|node| node.name().is_some_and(|name| name.ends_with(" RANDOM")))
                .count(),
            2
        );
        assert!(root.find_by_name("EN01 RANDOM").is_some());
        assert!(root.find_by_name("EN03 RANDOM").is_some());
        assert!(root.find_by_name("EN02 RANDOM").is_none());
        for copies in [
            SideCopies {
                entries: Vec::new(),
                ..copies.clone()
            },
            SideCopies {
                entries: copies
                    .entries
                    .iter()
                    .cloned()
                    .map(|mut e| {
                        e.mode = TriggerMode::Zone;
                        e
                    })
                    .collect(),
                ..copies
            },
        ] {
            let mut logic = SideLogic {
                init_off: vec![999],
                ..SideLogic::default()
            };
            let before = logic.clone();
            let mut next = 123;
            assert!(
                build_side(AirSide::Nato, &plan, &copies, &mut logic, &mut next)
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(logic, before);
            assert_eq!(next, 123);
            assert!(
                validate_schedule(AirSide::Nato, &plan.schedule[NATO], &copies.entries).is_empty()
            );
        }
    }

    #[test]
    fn schedule_disabled_emits_no_clk() {
        let mut plan = clock_test_plan();
        let (copies, mut logic, mut next) = assembled(&plan);
        let before = (logic.clone(), next);
        plan.schedule[NATO].enabled = false;
        plan.schedule[NATO].jitter_steps = 0; // An unused schedule registers nothing.
        assert!(
            build_side(AirSide::Nato, &plan, &copies, &mut logic, &mut next)
                .unwrap()
                .is_empty()
        );
        assert_eq!((logic, next), before);
        assert!(validate_schedule(AirSide::Nato, &plan.schedule[NATO], &copies.entries).is_empty());
    }

    #[test]
    fn clock_waterfall_order_is_seeded_and_deterministic() {
        let order = waterfall_order(7, AirSide::Nato, 16);
        assert_eq!(order, waterfall_order(7, AirSide::Nato, 16));
        assert_ne!(order, waterfall_order(8, AirSide::Nato, 16));
        assert_ne!(order, waterfall_order(7, AirSide::Dprk, 16));
        for n in [0, 1, 2, 3, 16, 144] {
            for side in [AirSide::Nato, AirSide::Dprk] {
                let mut order = waterfall_order(u64::MAX, side, n);
                order.sort_unstable();
                assert_eq!(order, (0..n).collect::<Vec<_>>());
            }
        }
        let mut plan = clock_test_plan();
        plan.pool[0].weight = 1;
        plan.pool[1].weight = 3;
        plan.pool[2].weight = 10;
        let (root, _, _) = clock_only(&plan);
        let order = waterfall_order(plan.seed, AirSide::Nato, 3);
        let weights: Vec<_> = order
            .iter()
            .map(|&i| u32::from(plan.pool[i].weight))
            .collect();
        for (&i, pct) in order.iter().zip(weighted_random_pct(&weights)) {
            assert_eq!(
                number(named(&root, &format!("EN{:02} RANDOM", i + 1)), "Random"),
                Ok(f64::from(pct))
            );
        }
    }

    #[test]
    fn clock_refuses_invalid_input() {
        let plan = clock_test_plan();
        let (copies, original, next) = assembled(&plan);
        let mut cases = Vec::new();
        for weight in [0, 11, 255] {
            let mut p = plan.clone();
            p.pool[0].weight = weight;
            cases.push((p, "weight"));
        }
        let mut p = plan.clone();
        p.schedule[NATO].period_min = 1.0;
        cases.push((p, "period"));
        for (mut plan, message) in cases {
            for side in [AirSide::Nato, AirSide::Dprk] {
                plan.schedule[side.idx()] = plan.schedule[NATO].clone();
                let mut logic = original.clone();
                let mut alloc = next;
                let error = build_side(side, &plan, &copies, &mut logic, &mut alloc).unwrap_err();
                assert!(
                    error.starts_with(&format!("Air Tasking {} clock: ", side.label()))
                        && error.contains(message),
                    "{error}"
                );
                assert_eq!(logic, original);
                assert_eq!(alloc, next);
            }
        }
        let mut plan = plan;
        plan.schedule[NATO].jitter_step_min = 0.0;
        assert!(validate_schedule(AirSide::Nato, &plan.schedule[NATO], &copies.entries).is_empty());
        assert!(
            build_side(
                AirSide::Nato,
                &plan,
                &copies,
                &mut original.clone(),
                &mut next.clone()
            )
            .is_ok()
        );
    }

    #[test]
    fn clock_validate_schedule_reports_invalid_fields() {
        let plan = clock_test_plan();
        let (copies, original, next) = assembled(&plan);
        let mut cases = Vec::new();
        for value in [
            0.0,
            -1.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
        ] {
            let mut p = plan.clone();
            p.schedule[NATO].first_slot_min = value;
            cases.push((p, "first slot time must be a positive number of minutes, finite in both minutes and seconds"));
            let mut p = plan.clone();
            p.schedule[NATO].period_min = value;
            cases.push((
                p,
                "period must be a positive number of minutes, finite in both minutes and seconds",
            ));
        }
        for value in [
            -1.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
            f64::MAX / 120.0,
        ] {
            let mut p = plan.clone();
            p.schedule[NATO].jitter_step_min = value;
            cases.push((p, "jitter step must be a finite number of minutes, at least zero, with a finite total span"));
        }
        let mut p = plan.clone();
        p.schedule[NATO].jitter_steps = 0;
        cases.push((p, "jitter steps must be at least 1"));
        let mut p = plan.clone();
        p.schedule[NATO].quiet_pct = 101;
        cases.push((p, "quiet chance must be 0 to 100 %"));
        for (mut plan, message) in cases {
            for side in [AirSide::Nato, AirSide::Dprk] {
                plan.schedule[side.idx()] = plan.schedule[NATO].clone();
                let schedule = &plan.schedule[side.idx()];
                let issues = validate_schedule(side, schedule, &copies.entries);
                assert_eq!(issues.len(), 1, "{schedule:?}: {issues:?}");
                assert_eq!(issues[0].severity, IssueSeverity::Error);
                assert_eq!(
                    issues[0].text,
                    format!("Air Tasking {} clock: {message}", side.label())
                );
                assert!(!issues[0].rail_only);
                let mut logic = original.clone();
                let mut alloc = next;
                let error = build_side(side, &plan, &copies, &mut logic, &mut alloc).unwrap_err();
                assert_eq!(error, issues[0].text);
                assert_eq!(logic, original);
                assert_eq!(alloc, next);
                let disabled = Schedule {
                    enabled: false,
                    ..schedule.clone()
                };
                assert!(validate_schedule(side, &disabled, &copies.entries).is_empty());
                assert!(validate_schedule(side, schedule, &[]).is_empty());
                let mut zone_only = copies.entries.clone();
                for entry in &mut zone_only {
                    entry.mode = TriggerMode::Zone;
                }
                assert!(validate_schedule(side, schedule, &zone_only).is_empty());
            }
        }
        let invalid = Schedule {
            first_slot_min: f64::NAN,
            period_min: 0.0,
            jitter_steps: 0,
            jitter_step_min: f64::INFINITY,
            quiet_pct: 101,
            ..Schedule::default()
        };
        let issues = validate_schedule(AirSide::Nato, &invalid, &copies.entries);
        assert_eq!(
            issues.len(),
            5,
            "report all fields without comparing the period"
        );
        assert!(
            issues
                .iter()
                .all(|issue| issue.severity == IssueSeverity::Error)
        );
    }

    #[test]
    fn clock_init_off_and_mission_begin() {
        for side in [AirSide::Nato, AirSide::Dprk] {
            let plan = clock_test_plan();
            let (copies, mut logic, mut next) = assembled(&plan);
            let before = logic.clone();
            let root = build_side(side, &plan, &copies, &mut logic, &mut next)
                .unwrap()
                .remove(0);
            let id = |name: &str| named(&root, name).index.unwrap();
            let mut off: Vec<_> = waterfall_order(plan.seed, side, 3)
                .iter()
                .map(|i| id(&format!("E{}{:02} LAST", side.letter(), i + 1)))
                .collect();
            off.push(id(&format!("CLK {} REROLL", side.letter())));
            assert_eq!(logic.init_off[before.init_off.len()..], off);
            assert_eq!(
                logic.mission_begin[before.mission_begin.len()..],
                [id(&format!("CLK {} FIRST", side.letter()))]
            );
        }
        let h = harness(&clock_test_plan(), 60.0);
        // Seed 0 is immaterial before FIRST fires; this walk only checks startup state.
        let log = run(&h, configs()[0], 0, 1.0, &[], &[]);
        for entry in &h.copies.entries {
            assert!(!active(
                &log,
                entry_id(&h, entry.number, "LAST"),
                ticks(0.5)
            ));
            assert!(!active(&log, entry_id(&h, entry.number, "HOT"), ticks(0.5)));
            assert!(active(&log, entry_id(&h, entry.number, "COLD"), ticks(0.5)));
        }
        assert!(!active(&log, clock_id(&h, "REROLL"), ticks(0.5)));
        assert!(times(&log, clock_id(&h, "FIRST")).is_empty());
    }

    #[test]
    fn clock_validate_schedule_period_shorter_than_jitter_plus_window_plus_attempt_is_an_error() {
        let entries = metadata(144, 144);
        let mut schedule = Schedule {
            period_min: 671.0 / 60.0,
            ..Schedule::default()
        };
        let issues = validate_schedule(AirSide::Nato, &schedule, &entries);
        assert!(
            issues
                .iter()
                .any(|issue| issue.severity == IssueSeverity::Error
                    && issue.text.contains("743.0")
                    && issue.text.contains("NATO")
                    && !issue.rail_only)
        );
        schedule.period_min = 743.0 / 60.0;
        assert!(
            validate_schedule(AirSide::Nato, &schedule, &entries)
                .iter()
                .any(|issue| issue.severity == IssueSeverity::Error)
        );
        schedule.period_min = 743.1 / 60.0;
        assert!(
            !validate_schedule(AirSide::Nato, &schedule, &entries)
                .iter()
                .any(|issue| issue.severity == IssueSeverity::Error)
        );
    }

    #[test]
    fn clock_validate_schedule_slot_window_cap_warns() {
        let schedule = Schedule::default();
        let issues = validate_schedule(AirSide::Dprk, &schedule, &metadata(144, 144));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert!(issues[0].text.contains("DPRK") && issues[0].text.contains("300 s"));
        // A = 25, a = 2: exactly 300 s is not capped.
        assert!(validate_schedule(AirSide::Nato, &schedule, &metadata(48, 2)).is_empty());
        assert_eq!(
            validate_schedule(AirSide::Nato, &schedule, &metadata(49, 2))[0].severity,
            IssueSeverity::Warning
        );
    }

    #[test]
    fn clock_all_hot_one_start_per_slot_no_back_to_back() {
        let plan = plan_with_entries(4, 4);
        let h = harness(&plan, 60.0);
        for config in configs() {
            // Seed 2 exercises quiet periods and launches across the default jitter steps.
            let log = run(&h, config, 2, 14400.0, &[1, 2, 3, 4], &[]);
            let slots = default_slots(&h, &log, 12);
            assert_wins_match_requests(&h, &log);
            let launches = starts(&log, &h);
            let requests = gos(&log, &h);
            let quiet = slots.iter().filter(|slot| slot.is_none()).count();
            assert!(quiet > 0 && quiet < 12, "quiet {quiet}, {config:?}");
            assert_eq!(launches.len(), 12 - quiet, "{config:?}");
            assert_eq!(requests.len(), launches.len(), "{config:?}");
            for (period, slot) in slots.iter().enumerate() {
                let beginning = ticks(600.0 + 1200.0 * period as f64);
                let in_slot: Vec<_> = launches
                    .iter()
                    .filter(|pulse| (beginning..beginning + ticks(1200.0)).contains(&pulse.t))
                    .collect();
                assert_eq!(
                    in_slot.len(),
                    usize::from(slot.is_some()),
                    "period {period}, {config:?}"
                );
                if let Some(opened) = slot {
                    assert!(in_slot[0].t > *opened && in_slot[0].t < opened + ticks(45.0));
                }
            }
            for (launch, request) in launches.iter().zip(&requests) {
                assert_eq!(launch.t, request.t);
                assert_eq!(
                    request.name,
                    format!("EN{:02} GO", start_entry(&h, launch.index))
                );
            }
            for pair in launches.windows(2) {
                assert_ne!(
                    start_entry(&h, pair[0].index),
                    start_entry(&h, pair[1].index)
                );
            }
        }
    }

    #[test]
    fn clock_only_one_area_hot_alternates_with_quiet() {
        let h = harness(&clock_test_plan(), 60.0);
        let outputs: Vec<_> = h
            .copies
            .entries
            .iter()
            .map(|entry| entry_id(&h, entry.number, "OUT"))
            .collect();
        for config in configs() {
            // Seed 2 includes quiet periods and cold first attempts that must reroll to entry 2.
            let log = run(&h, config, 2, 12000.0, &[h.copies.entries[1].area], &[]);
            let slots = default_slots(&h, &log, 10);
            assert_wins_match_requests(&h, &log);
            let launches = starts(&log, &h);
            let requests = gos(&log, &h);
            let quiet = slots.iter().filter(|slot| slot.is_none()).count();
            assert!(quiet > 0 && quiet < 10, "quiet {quiet}, {config:?}");
            assert_eq!(launches.len(), (10 - quiet).div_ceil(2), "{config:?}");
            assert_eq!(requests.len(), launches.len());
            let mut non_quiet = 0;
            let mut rerolled_starts = 0;
            for (period, slot) in slots.iter().enumerate() {
                let beginning = ticks(600.0 + 1200.0 * period as f64);
                let in_slot: Vec<_> = launches
                    .iter()
                    .filter(|pulse| (beginning..beginning + ticks(1200.0)).contains(&pulse.t))
                    .collect();
                let slot_requests: Vec<_> = requests
                    .iter()
                    .filter(|pulse| (beginning..beginning + ticks(1200.0)).contains(&pulse.t))
                    .collect();
                let expected = usize::from(slot.is_some() && non_quiet % 2 == 0);
                assert_eq!(in_slot.len(), expected, "period {period}, {config:?}");
                assert_eq!(
                    slot_requests.len(),
                    expected,
                    "the non-quiet slot after a start must send no request: period {period}, {config:?}"
                );
                for pulse in in_slot {
                    let opened = slot.unwrap();
                    assert_eq!(start_entry(&h, pulse.index), 2);
                    assert_eq!(slot_requests[0].index, entry_id(&h, 2, "GO"));
                    assert_eq!(slot_requests[0].t, pulse.t);
                    assert!(pulse.t > opened && pulse.t < opened + ticks(32.5));
                    let first_winner = log.pulses.iter().find(|output| {
                        output.t > opened
                            && output.t < opened + ticks(0.3 + 2.5)
                            && outputs.contains(&output.index)
                    });
                    if first_winner.map(|output| output.index) != Some(entry_id(&h, 2, "OUT")) {
                        assert!(
                            times(&log, clock_id(&h, "REROLL"))
                                .iter()
                                .any(|time| opened < *time && *time < pulse.t),
                            "entry 2 must be reached by reroll: period {period}, {config:?}"
                        );
                        rerolled_starts += 1;
                    }
                }
                if slot.is_some() {
                    non_quiet += 1;
                }
            }
            assert!(
                rerolled_starts > 0,
                "the seed must exercise a rerolled start, {config:?}"
            );
        }
    }

    #[test]
    fn clock_no_area_hot_sends_no_request() {
        let h = harness(&clock_test_plan(), 60.0);
        for config in configs() {
            // Seed 2 opens the first period so every cold area is actually tried.
            let log = run(&h, config, 2, 1100.0, &[], &[]);
            let slot = default_slots(&h, &log, 1)[0].expect("seed must open the first slot");
            assert!(starts(&log, &h).is_empty());
            assert!(gos(&log, &h).is_empty());
            assert!(times(&log, clock_id(&h, "WIN")).is_empty());
            assert_eq!(
                times(&log, clock_id(&h, "SLOT CLOSE")),
                [slot + ticks(32.5)]
            );
            for entry in &h.copies.entries {
                assert_eq!(
                    times(&log, clock_id(&h, &format!("AREA {:02} OUT", entry.area))).len(),
                    1
                );
                assert!(times(&log, entry_id(&h, entry.number, "LAST SET")).is_empty());
            }
        }
    }

    #[test]
    fn clock_budget_full_slot_is_quiet_and_sets_no_last() {
        // Short slots keep the engineered first draw inside the blocker's real flight.
        let mut plan = fast_plan(3, 3);
        plan.budget.per_side[NATO] = 1;
        let order = waterfall_order(plan.seed, AirSide::Nato, 3);
        let winner = order[0] + 1;
        let blocker = order[2];
        let h = harness(&plan, 60.0);
        for config in configs() {
            let log = run(
                &h,
                config,
                367, // First waterfall position passes immediately in both slots/orders.
                300.0,
                &[h.copies.entries[winner - 1].area],
                &[(50.0, h.copies.entries[blocker].copies[0].pointer)],
            );
            assert_slots(&h, &log, &[80.1, 260.1]);
            assert_wins_match_requests(&h, &log);
            let requests = gos(&log, &h);
            assert_eq!(requests.len(), 2, "{config:?}");
            assert!(
                requests
                    .iter()
                    .all(|p| p.index == entry_id(&h, winner, "GO"))
            );
            let launches = starts(&log, &h);
            assert_eq!(launches.len(), 2);
            assert_eq!(
                (launches[0].index, launches[0].t),
                (h.copies.entries[blocker].copies[0].shell.start, ticks(50.0))
            );
            assert_eq!(start_entry(&h, launches[1].index), winner);
            assert!(launches[1].t > ticks(260.1));
            assert_eq!(
                times(&log, entry_id(&h, winner, "LAST SET")),
                [launches[1].t]
            );
            assert!(!active(&log, entry_id(&h, winner, "LAST"), ticks(120.0)));
            assert!(times(&log, clock_id(&h, "REROLL")).is_empty());
            assert_eq!(
                times(&log, clock_id(&h, "Randomizer:INPUT")),
                [ticks(80.4), ticks(260.4)]
            );
        }
    }

    #[test]
    fn clock_entry_never_wins_after_runs_starts() {
        let mut plan = plan_with_entries(1, 1);
        plan.pool[0].runs = 3;
        let h = harness(&plan, 60.0);
        for config in configs() {
            // Seed 2 opens enough default slots to exhaust three copies and try again afterwards.
            let log = run(&h, config, 2, 12000.0, &[1], &[]);
            let slots: Vec<_> = default_slots(&h, &log, 10).into_iter().flatten().collect();
            assert!(slots.len() >= 7, "{config:?}: {slots:?}");
            assert_wins_match_requests(&h, &log);
            assert_eq!(
                times(&log, entry_id(&h, 1, "GO")),
                [slots[0], slots[2], slots[4]].map(|slot| slot + ticks(0.8))
            );
            let launches = starts(&log, &h);
            assert_eq!(
                launches.iter().map(|p| p.index).collect::<Vec<_>>(),
                h.copies.entries[0]
                    .copies
                    .iter()
                    .map(|c| c.shell.start)
                    .collect::<Vec<_>>()
            );
            let exhausted = launches[2].t;
            assert_eq!(times(&log, h.copies.entries[0].exhausted), [exhausted]);
            for role in ["AVAILABLE", "RANDOM", "GO"] {
                assert!(!active(&log, entry_id(&h, 1, role), ticks(12000.0)));
                assert!(
                    times(&log, entry_id(&h, 1, role))
                        .iter()
                        .all(|&t| t <= exhausted)
                );
            }
        }
    }

    #[test]
    fn clock_early_miss_then_later_winner_one_request_per_slot() {
        // One short slot isolates the engineered cold first draw and its later hot reroll.
        let plan = fast_plan(3, 3);
        let order = waterfall_order(plan.seed, AirSide::Nato, 3);
        let h = harness(&plan, 60.0);
        let cold = order[0] + 1;
        let hot: Vec<_> = order[1..]
            .iter()
            .map(|&i| h.copies.entries[i].area)
            .collect();
        for config in configs() {
            // Seed 2 selects the cold first position, then a hot entry on the next attempt.
            let log = run(&h, config, 2, 120.0, &hot, &[]);
            assert_slots(&h, &log, &[80.1]);
            assert_wins_match_requests(&h, &log);
            assert_eq!(times(&log, entry_id(&h, cold, "COLD")), [ticks(80.9)]);
            assert_eq!(
                times(
                    &log,
                    clock_id(
                        &h,
                        &format!("AREA {:02} OUT", h.copies.entries[cold - 1].area)
                    )
                ),
                [ticks(80.9)]
            );
            let requests = gos(&log, &h);
            let launches = starts(&log, &h);
            assert_eq!(requests.len(), 1);
            assert_eq!(launches.len(), 1);
            assert_ne!(start_entry(&h, launches[0].index), cold);
            assert!(requests[0].t > ticks(82.9)); // The miss closed all later OUTs; a new attempt is needed.
            assert_eq!(requests[0].t, launches[0].t);
            assert_eq!(times(&log, clock_id(&h, "REROLL")), [ticks(82.9)]);
        }
    }

    #[test]
    fn clock_win_then_later_miss_rerolls() {
        // Short slots align the engineered third draw with the first area's transition to COLD.
        let mut plan = fast_plan(3, 3);
        let order = waterfall_order(plan.seed, AirSide::Nato, 3);
        // Make the first draw reproducible in both input orders, while leaving
        // actual per-trigger random rolls and the entire waterfall in place.
        for entry in &mut plan.pool {
            entry.weight = 1;
        }
        plan.pool[order[0]].weight = 9;
        let h = harness(&plan, 60.0);
        let first = order[0] + 1;
        let first_area = h.copies.entries[first - 1].area;
        let mut tracks = loiter(&h, &[1, 2, 3]);
        let (x, z) = h.copies.areas[first_area - 1];
        tracks[first_area - 1].samples = (1..=100)
            .map(|s| {
                (
                    ticks(s as f64),
                    if s < 100 { x + 1000.0 } else { x - 100_000.0 },
                    z,
                )
            })
            .collect();
        for config in configs() {
            let log = walk(
                &h.root,
                config,
                &WalkerInput {
                    // Seed 2 wins in position 1, then revisits that position after its area cools.
                    seed: 2,
                    until: ticks(480.0),
                    tracks: tracks.clone(),
                    ..WalkerInput::default()
                },
            );
            assert_slots(&h, &log, &[80.1, 260.1, 440.1]);
            assert_wins_match_requests(&h, &log);
            let launches = starts(&log, &h);
            assert_eq!(launches.len(), 3, "{config:?}");
            assert_eq!(start_entry(&h, launches[0].index), first);
            assert_ne!(start_entry(&h, launches[2].index), first);
            assert_eq!(times(&log, entry_id(&h, first, "COLD")), [ticks(440.9)]);
            assert_eq!(
                times(&log, clock_id(&h, &format!("AREA {first_area:02} OUT"))),
                [ticks(440.9)]
            );
            assert!(times(&log, clock_id(&h, "REROLL")).contains(&ticks(442.9)));
            assert!(launches[2].t > ticks(442.9));
            assert_eq!(gos(&log, &h).len(), 3);
        }
    }

    #[test]
    fn clock_sixteen_entries_one_passing_area_found() {
        let plan = plan_with_entries(16, 8);
        let order = waterfall_order(plan.seed, AirSide::Nato, 16);
        let h = harness(&plan, 60.0);
        let hot = h.copies.entries[order[15]].area;
        for config in configs() {
            // Seed 2 opens the first period and reaches the hot area after cold-area misses.
            let log = run(&h, config, 2, 1140.0, &[hot], &[]);
            let slot = default_slots(&h, &log, 1)[0].expect("seed must open the first slot");
            assert_wins_match_requests(&h, &log);
            let launches = starts(&log, &h);
            assert_eq!(launches.len(), 1, "{config:?}");
            assert_eq!(gos(&log, &h).len(), 1);
            let winner = start_entry(&h, launches[0].index);
            assert_eq!(h.copies.entries[winner - 1].area, hot);
            assert_eq!(times(&log, entry_id(&h, winner, "GO")), [launches[0].t]);
            assert!(launches[0].t < slot + ticks(162.0));
            assert!(!times(&log, clock_id(&h, "REROLL")).is_empty());
        }
    }

    fn statistical_plan() -> AirPlan {
        let mut plan = fast_plan(3, 3);
        plan.schedule[NATO].quiet_pct = 20;
        plan.schedule[NATO].jitter_steps = 3;
        plan.schedule[NATO].jitter_step_min = 0.05;
        plan
    }

    #[test]
    fn clock_quiet_rate_and_jitter_spread() {
        let h = harness(&statistical_plan(), 60.0);
        let mut quiet = 0;
        let mut jitter = [0; 3];
        // Exactly 200 independent missions, with distinct RNG seeds. Cycle
        // through delivery modes so both target orders enter the sample.
        for seed in 0..200 {
            let config = configs()[seed as usize % 3];
            let log = run(&h, config, seed, 125.0, &[1, 2, 3], &[]);
            let slots = times(&log, clock_id(&h, "SLOT OPEN"));
            assert!(slots.len() <= 1, "seed {seed}, {config:?}");
            assert_wins_match_requests(&h, &log);
            assert_eq!(starts(&log, &h).len(), slots.len());
            assert_eq!(gos(&log, &h).len(), slots.len());
            if slots.is_empty() {
                quiet += 1;
                assert!(times(&log, clock_id(&h, "WIN")).is_empty());
            } else {
                let k = [80.1, 83.1, 86.1]
                    .map(ticks)
                    .iter()
                    .position(|&t| t == slots[0])
                    .expect("slot must use a jitter step");
                jitter[k] += 1;
                let launch = starts(&log, &h)[0];
                assert!(launch.t > slots[0] && launch.t < slots[0] + ticks(3.0));
                assert_eq!(times(&log, clock_id(&h, "WIN")), [launch.t]);
            }
        }
        assert!(
            (26..=54).contains(&quiet),
            "quiet {quiet}/200; jitter {jitter:?}"
        );
        assert!(jitter.iter().all(|&count| count > 0), "{jitter:?}");
    }

    #[test]
    fn clock_both_entry_exhausted_by_zone_never_wins_a_slot() {
        let mut plan = clock_test_plan();
        let first = waterfall_order(plan.seed, AirSide::Nato, 3)[0];
        plan.pool[first].mode = TriggerMode::Both;
        plan.pool[first].runs = 3;
        let h = harness(&plan, 60.0);
        let used = &h.copies.entries[first];
        let pulses: Vec<_> = used
            .copies
            .iter()
            .zip([5.0, 15.0, 25.0])
            .map(|(copy, t)| (t, copy.pointer))
            .collect();
        for config in configs() {
            // Seed 2 opens a default slot after the zone has consumed the Both entry's copies.
            let log = run(&h, config, 2, 1100.0, &[1, 2, 3], &pulses);
            assert!(default_slots(&h, &log, 1)[0].is_some());
            assert_wins_match_requests(&h, &log);
            assert_eq!(times(&log, used.exhausted), [ticks(25.0)]);
            for (copy, t) in used.copies.iter().zip([5.0, 15.0, 25.0]) {
                assert_eq!(times(&log, copy.shell.start), [ticks(t)]);
            }
            assert!(times(&log, entry_id(&h, used.number, "GO")).is_empty());
            assert!(times(&log, entry_id(&h, used.number, "RANDOM")).is_empty());
            assert_eq!(gos(&log, &h).len(), 1);
            let launches = starts(&log, &h);
            assert_eq!(launches.len(), 4);
            assert_ne!(start_entry(&h, launches[3].index), used.number);
            assert_eq!(times(&log, clock_id(&h, "WIN")), [launches[3].t]);
        }
    }

    #[test]
    fn clock_nobody_won_quiet_rate_is_small() {
        let plan = statistical_plan();
        let order = waterfall_order(plan.seed, AirSide::Nato, 3);
        let h = harness(&plan, 60.0);
        let first = order[0] + 1;
        let last = order[2] + 1;
        let mut opened = 0;
        let mut launched = 0;
        for seed in 2000..2200 {
            let config = configs()[seed as usize % 3];
            let log = run(
                &h,
                config,
                seed,
                125.0,
                &[h.copies.entries[first - 1].area],
                &[(70.0, entry_id(&h, last, "LAST SET"))],
            );
            assert_wins_match_requests(&h, &log);
            let slots = times(&log, clock_id(&h, "SLOT OPEN"));
            let launches = starts(&log, &h);
            assert!(slots.len() <= 1);
            assert!(launches.len() <= slots.len());
            assert_eq!(gos(&log, &h).len(), launches.len());
            assert!(times(&log, entry_id(&h, last, "GO")).is_empty());
            opened += slots.len();
            launched += launches.len();
            for launch in launches {
                assert_eq!(start_entry(&h, launch.index), first);
                assert!(launch.t > slots[0] && launch.t < slots[0] + ticks(35.0));
                assert_eq!(times(&log, clock_id(&h, "WIN")), [launch.t]);
            }
        }
        assert!(opened >= 100, "only {opened} non-quiet slots");
        assert!(
            launched * 100 >= opened * 97,
            "{launched}/{opened} non-quiet slots launched"
        );
    }

    #[test]
    fn clock_attempt_in_flight_at_slot_close_can_still_win() {
        // A short first slot isolates a deliberately delayed win across the 300 s cap.
        // An uncapped window is an exact multiple of A, and its final outputs
        // precede close. Here n=24, a=14, A=13, window=min(300,312): the last
        // attempt begins at 379.4, just before close at 380.1, and can win late.
        let mut plan = fast_plan(24, 14);
        plan.schedule[NATO].period_min = 7.0;
        let order = waterfall_order(plan.seed, AirSide::Nato, 24);
        for (position, &i) in order.iter().enumerate() {
            let area = if position == 22 { 13 } else { position % 13 };
            plan.pool[i].target = TargetRef::Objective {
                side: AirSide::Nato,
                xz: (150_000.0, 150_000.0 + area as f64 * 15_000.0),
            };
        }
        let h = harness(&plan, 60.0);
        let hot = order[22] + 1;
        let last = order[23] + 1;
        assert_eq!(h.copies.areas.len(), 14);
        assert_eq!(
            number(by_id(&h.root, clock_id(&h, "SLOT CLOSE")), "Time"),
            Ok(300.0)
        );
        for config in configs() {
            // Seeds 1303/10399 delay the hot win to 390.9 s, after the slot closes at 380.1 s.
            let seed = if config.reverse_targets { 1303 } else { 10399 };
            let log = run(
                &h,
                config,
                seed,
                395.0,
                &[h.copies.entries[hot - 1].area],
                &[(70.0, entry_id(&h, last, "LAST SET"))],
            );
            assert_slots(&h, &log, &[80.1]);
            assert_wins_match_requests(&h, &log);
            assert_eq!(times(&log, clock_id(&h, "SLOT CLOSE")), [ticks(380.1)]);
            let requests = gos(&log, &h);
            let launches = starts(&log, &h);
            assert_eq!(requests.len(), 1, "{config:?}");
            assert_eq!(launches.len(), 1);
            assert_eq!(requests[0].index, entry_id(&h, hot, "GO"));
            assert_eq!(requests[0].t, ticks(390.9));
            assert_eq!(launches[0].t, requests[0].t);
            assert!(requests[0].t > ticks(380.1) && requests[0].t < ticks(393.1));
            let attempts = times(&log, clock_id(&h, "Randomizer:INPUT"));
            assert_eq!(attempts.last(), Some(&ticks(379.4)));
            assert!(
                times(&log, clock_id(&h, "REROLL"))
                    .iter()
                    .all(|&t| t < ticks(380.1))
            );
        }
    }

    #[test]
    fn clock_pending_random_of_an_exhausted_entry_does_not_end_the_slot() {
        let mut plan = clock_test_plan();
        let first = waterfall_order(plan.seed, AirSide::Nato, 3)[0];
        plan.pool[first].mode = TriggerMode::Both;
        plan.pool[first].runs = 1;
        let h = harness(&plan, 60.0);
        let used = &h.copies.entries[first];
        // This scenario scripts the zone start. Disarm its PASS timers before
        // they can launch early or consume the clock's seeded random sequence.
        let zone_disarm = named(&h.root, &format!("ZN{:02} DISARM", used.number))
            .index
            .unwrap();
        for config in configs() {
            for cancellation in [
                DeactivateRunningTimer::RunsOn,
                DeactivateRunningTimer::Cancels,
            ] {
                let config = WalkerConfig {
                    deactivate_running_timer: cancellation,
                    ..config
                };
                let log = run(
                    &h,
                    config,
                    548, // Passes all jitter rolls and position 1, leaving its timer pending at the zone start.
                    1100.0,
                    &[1, 2, 3],
                    &[(2.0, zone_disarm), (600.5, used.copies[0].pointer)],
                );
                assert_eq!(default_slots(&h, &log, 1), [Some(ticks(600.1))]);
                assert_wins_match_requests(&h, &log);
                assert_eq!(times(&log, used.copies[0].shell.start), [ticks(600.5)]);
                assert_eq!(times(&log, used.exhausted), [ticks(600.5)]);
                assert!(times(&log, entry_id(&h, used.number, "GO")).is_empty());
                let expected = if cancellation == DeactivateRunningTimer::RunsOn {
                    vec![ticks(600.9)]
                } else {
                    vec![]
                };
                assert_eq!(times(&log, entry_id(&h, used.number, "RANDOM")), expected);
                let launches = starts(&log, &h);
                assert_eq!(launches.len(), 2, "{config:?}");
                assert_ne!(start_entry(&h, launches[1].index), used.number);
                assert_eq!(gos(&log, &h).len(), 1);
                assert_eq!(times(&log, clock_id(&h, "WIN")), [launches[1].t]);
                assert!(launches[1].t > ticks(600.9));
                if cancellation == DeactivateRunningTimer::RunsOn {
                    assert!(times(&log, clock_id(&h, "REROLL")).contains(&ticks(602.9)));
                }
            }
        }
    }

    #[test]
    fn clock_zone_start_inside_the_first_300_ms_has_its_last_cleared() {
        // Short slots isolate a zone start inside DRAW's 300 ms delay and its next-slot eligibility.
        let mut plan = fast_plan(2, 2);
        let order = waterfall_order(plan.seed, AirSide::Nato, 2);
        let zone = order[1] + 1;
        let first = order[0] + 1;
        plan.pool[zone - 1].mode = TriggerMode::Both;
        let h = harness(&plan, 60.0);
        // The scripted start below owns the zone timing in this clock check.
        let zone_disarm = named(&h.root, &format!("ZN{zone:02} DISARM"))
            .index
            .unwrap();
        for config in configs() {
            let log = run(
                &h,
                config,
                14, // First position wins slot 1; it misses in slot 2, exposing the zone entry.
                300.0,
                &[1, 2],
                &[
                    (2.0, zone_disarm),
                    (80.25, h.copies.entries[zone - 1].copies[0].pointer),
                ],
            );
            assert_slots(&h, &log, &[80.1, 260.1]);
            assert_wins_match_requests(&h, &log);
            assert_eq!(
                times(&log, clock_id(&h, "LAST CLEAR")),
                [ticks(80.4), ticks(260.4)]
            );
            assert_eq!(
                times(&log, entry_id(&h, zone, "LAST SET")),
                [ticks(80.25), ticks(261.4)]
            );
            assert!(!active(&log, entry_id(&h, zone, "LAST"), ticks(81.0)));
            assert!(times(&log, entry_id(&h, zone, "LAST")).is_empty());
            assert_eq!(times(&log, entry_id(&h, first, "GO")), [ticks(80.9)]);
            assert_eq!(times(&log, entry_id(&h, zone, "GO")), [ticks(261.4)]);
            assert_eq!(
                times(&log, clock_id(&h, "WIN")),
                [ticks(80.9), ticks(261.4)]
            );
            let launches = starts(&log, &h);
            // The zone's START initiates a real budget recount through 81.0;
            // the 80.9 clock request is dropped. Its WIN still ends that slot.
            // With LAST cleared, the zone entry can start again in slot 2.
            assert_eq!(launches.len(), 2);
            assert_eq!(
                launches[0].index,
                h.copies.entries[zone - 1].copies[0].shell.start
            );
            assert_eq!(
                launches[1].index,
                h.copies.entries[zone - 1].copies[1].shell.start
            );
            assert_eq!([launches[0].t, launches[1].t], [ticks(80.25), ticks(261.4)]);
            assert_eq!(
                launches
                    .iter()
                    .map(|p| start_entry(&h, p.index))
                    .collect::<Vec<_>>(),
                [zone, zone]
            );
        }
    }

    #[test]
    fn clock_go_starts_one_copy_per_win() {
        let h = harness(&plan_with_entries(1, 1), 60.0);
        for config in configs() {
            // Seed 2 supplies several non-quiet slots to exercise advancing copy pointers.
            let log = run(&h, config, 2, 6000.0, &[1], &[]);
            let slots: Vec<_> = default_slots(&h, &log, 5).into_iter().flatten().collect();
            assert!(slots.len() >= 3, "{config:?}: {slots:?}");
            let wins: Vec<_> = slots
                .iter()
                .step_by(2)
                .map(|slot| slot + ticks(0.8))
                .collect();
            assert_wins_match_requests(&h, &log);
            assert_eq!(times(&log, entry_id(&h, 1, "GO")), wins);
            assert_eq!(times(&log, clock_id(&h, "WIN")), wins);
            for (i, copy) in h.copies.entries[0].copies.iter().enumerate() {
                let expected: Vec<_> = wins.get(i).copied().into_iter().collect();
                assert_eq!(
                    times(&log, copy.shell.start),
                    expected,
                    "copy {}, {config:?}",
                    copy.run
                );
                assert_eq!(times(&log, copy.pointer), expected);
            }
            assert_eq!(starts(&log, &h).len(), wins.len());
        }
    }

    #[test]
    fn clock_one_slot_per_period_when_two_jitter_timers_pass() {
        let h = harness(&plan_with_entries(1, 1), 60.0);
        for config in configs() {
            // Seed 106 passes all three jitter timers in period 1, stays quiet in period 3, then reopens.
            let log = run(&h, config, 106, 4800.0, &[1], &[]);
            let passed = (1..=3)
                .filter(|k| {
                    times(&log, clock_id(&h, &format!("JITTER {k}")))
                        .iter()
                        .any(|&t| t < ticks(1200.0))
                })
                .count();
            assert_eq!(
                passed, 3,
                "all three jitter rolls must pass in the first period, {config:?}"
            );
            let periods = default_slots(&h, &log, 4);
            let quiet = periods.iter().filter(|slot| slot.is_none()).count();
            assert_eq!(quiet, 1, "seed must exercise a quiet period, {config:?}");
            assert_eq!(periods[2], None);
            let slots: Vec<_> = periods.into_iter().flatten().collect();
            assert_eq!(slots.len(), 4 - quiet);
            assert_eq!(slots[0], ticks(600.1));
            assert_eq!(times(&log, clock_id(&h, "SLOT OPEN OFF")), slots);
            assert_wins_match_requests(&h, &log);
            assert_eq!(gos(&log, &h).len(), 2);
            let launches = starts(&log, &h);
            assert_eq!(launches.len(), 2);
            assert_eq!(launches[0].t, slots[0] + ticks(0.8));
            assert_eq!(launches[1].t, slots[2] + ticks(0.8));
        }
    }

    #[test]
    fn clock_late_attempt_end_after_win_does_not_reroll() {
        // One short slot isolates a late RunsOn expiry after its winner has shut REROLL.
        let h = harness(&fast_plan(1, 1), 60.0);
        for config in configs() {
            let config = WalkerConfig {
                deactivate_running_timer: DeactivateRunningTimer::RunsOn,
                ..config
            };
            // Seed 2 preserves the common fixture; this single-entry draw is certain to win.
            let log = run(&h, config, 2, 100.0, &[1], &[]);
            assert_slots(&h, &log, &[80.1]);
            assert_wins_match_requests(&h, &log);
            assert_eq!(times(&log, entry_id(&h, 1, "GO")), [ticks(80.9)]);
            assert_eq!(
                times(&log, h.copies.entries[0].copies[0].shell.start),
                [ticks(80.9)]
            );
            assert_eq!(starts(&log, &h).len(), 1);
            assert_eq!(times(&log, clock_id(&h, "WIN")), [ticks(80.9)]);
            assert_eq!(times(&log, clock_id(&h, "ATTEMPT END")), [ticks(81.9)]);
            assert!(times(&log, clock_id(&h, "REROLL")).is_empty());
            assert_eq!(times(&log, clock_id(&h, "Randomizer:INPUT")), [ticks(80.4)]);
        }
    }
}
