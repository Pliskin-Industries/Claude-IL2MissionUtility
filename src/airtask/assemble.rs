//! Placement, one-use copy pointers and the final cross-builder wiring pass.

use std::collections::BTreeSet;

use super::budget::census_delay;
use super::library::{BUILTINS, inspect_sortie, nodes};
use super::place::{SortiePlacement, place_air_sortie};
use super::shell::{ShellIds, ShellSpec, by_id_mut, group, wrap_sortie};
use super::{AirContext, AirPlan, AirSide, SideLogic, SortieSource, TargetRef, TriggerMode};
use crate::ast::Il2Entity;
use crate::{duplicate, mapclip, mapfighters, parser, placement, template};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CopyIds {
    pub(crate) number: usize,
    pub(crate) run: u8,
    pub(crate) root: i32,
    pub(crate) pointer: i32,
    pub(crate) shell: ShellIds,
    pub(crate) spawn: (f64, f64),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntryCopies {
    pub(crate) pool_index: usize,
    pub(crate) number: usize,
    pub(crate) area: usize,
    pub(crate) mode: TriggerMode,
    pub(crate) exhausted: i32,
    pub(crate) copies: Vec<CopyIds>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SideCopies {
    /// Logic first, Sorties second; later builders append to trees[0].
    pub(crate) trees: Vec<Il2Entity>,
    pub(crate) entries: Vec<EntryCopies>,
    /// Stable, 1-based area numbers in order of first eligible pool entry.
    pub(crate) areas: Vec<(f64, f64)>,
    pub(crate) warnings: Vec<String>,
}

fn resolve(target: TargetRef, ctx: &AirContext<'_>) -> Option<(AirSide, (f64, f64), Option<f64>)> {
    let near = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).hypot(a.1 - b.1);
    match target {
        TargetRef::Objective { side, xz } => {
            let objectives = if side == AirSide::Dprk {
                ctx.east_objectives
            } else {
                ctx.nato_objectives
            };
            objectives
                .iter()
                .copied()
                .filter(|&p| near(p, xz) <= 50.0)
                .min_by(|&a, &b| near(a, xz).total_cmp(&near(b, xz)))
                .map(|p| (side, p, None))
        }
        TargetRef::Arrow { tip, .. } => ctx
            .arrows
            .iter()
            .copied()
            .filter(|&(_, p)| near(p, tip) <= 50.0)
            .min_by(|&(_, a), &(_, b)| near(a, tip).total_cmp(&near(b, tip)))
            .map(|(tail, tip)| {
                let side = if mapclip::point_north_of_front(ctx.front, tail.0, tail.1) {
                    AirSide::Dprk
                } else {
                    AirSide::Nato
                };
                (side, tip, Some(placement::heading_toward(tail, tip)))
            }),
    }
}

/// Stage an entire side so an invalid template cannot leave half a registry.
pub(crate) fn assemble_side(
    side: AirSide,
    plan: &AirPlan,
    ctx: &AirContext<'_>,
    logic: &mut SideLogic,
    next_id: &mut i32,
) -> Result<SideCopies, String> {
    let mut alloc = *next_id;
    let mut registry = logic.clone();
    let label = if side == AirSide::Dprk {
        "DPRK"
    } else {
        "NATO"
    };
    let mut result = SideCopies {
        trees: vec![
            group(&format!("Air Tasking {label} Logic"), &mut alloc),
            group(&format!("Air Tasking {label} Sorties"), &mut alloc),
        ],
        entries: Vec::new(),
        areas: Vec::new(),
        warnings: Vec::new(),
    };
    let mut number = 0;
    for (pool_index, entry) in plan
        .pool
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.enabled)
    {
        let Some(sortie) = plan
            .library
            .iter()
            .find(|sortie| sortie.id == entry.sortie_id)
        else {
            return Err(format!(
                "Air Tasking cannot find sortie {}",
                entry.sortie_id
            ));
        };
        if sortie.side != side || ctx.date < sortie.valid.from || ctx.date > sortie.valid.to {
            continue;
        }
        let Some((target_side, target, arrow_heading)) = resolve(entry.target, ctx) else {
            result.warnings.push(format!(
                "Entry {} has an orphaned target; skipped",
                pool_index + 1
            ));
            continue;
        };
        if target_side != side {
            return Err(format!(
                "Entry {} has a target on the wrong side",
                pool_index + 1
            ));
        }
        if !(1..=6).contains(&entry.runs) {
            return Err(format!("Entry {} needs 1..=6 runs", pool_index + 1));
        }
        let text = match &sortie.source {
            SortieSource::BuiltIn(file) => BUILTINS
                .iter()
                .find(|(name, _)| name == file)
                .map(|(_, text)| (*text).to_owned())
                .ok_or_else(|| format!("Unknown built-in {file}"))?,
            SortieSource::File(path) => {
                std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?
            }
        };
        let source = parser::parse_group_file(&text).map_err(|e| e.to_string())?;
        let shape = inspect_sortie(&source).map_err(|e| e.join("; "))?;
        if shape.side != side {
            return Err(format!(
                "{} changed coalition since it was loaded",
                sortie.name
            ));
        }
        let area = if let Some(index) = result.areas.iter().position(|&p| p == target) {
            index + 1
        } else {
            result.areas.push(target);
            result.areas.len()
        };
        let rtb = mapfighters::rtb_ao_point(side == AirSide::Dprk, target.0, target.1, ctx.aabb);
        let heading_deg = entry
            .heading_override
            .or(arrow_heading)
            .unwrap_or_else(|| placement::heading_toward(rtb, target));
        let spawn_dist_m = if entry.mode == TriggerMode::Clock {
            shape.wp_speed_kmh / 3.6 * plan.schedule[side.idx()].arrive_min * 60.0
        } else {
            plan.zone.spawn_dist_km * 1000.0
        };
        let placement = SortiePlacement {
            target,
            heading_deg,
            spawn_dist_m,
            rtb,
        };
        let entry_number = result.entries.len() + 1;
        let entry_prefix = format!("E{}{:02}", side.letter(), entry_number);
        let exhausted = template::mcu(
            "MCU_Deactivate",
            &format!("{entry_prefix} EXHAUSTED"),
            &mut alloc,
            target.0,
            target.1,
        );
        let exhausted_id = exhausted.index.unwrap();
        result.trees[0].children.push(exhausted);
        let mut copies = Vec::new();
        for run in 1..=entry.runs {
            number += 1;
            let prefix = format!("S{}{:02}", side.letter(), number);
            // Place before stamping so the placement-owned RTB ids are remapped too.
            let mut placed = source.clone();
            let facts = place_air_sortie(&mut placed, &placement)?;
            let (mut copy, _) = duplicate::duplicate_template(&placed, &mut alloc);
            let shape = inspect_sortie(&copy).map_err(|e| e.join("; "))?;
            copy.set_name(&format!("{prefix} {} #{run}", sortie.name));
            let spec = ShellSpec {
                attack_time_s: facts.attack_time_s,
                final_time_s: spawn_dist_m / (facts.wp_speed_kmh / 3.6)
                    + facts.attack_time_s
                    + (plan.stop.margin_min + plan.stop.extension_min) * 60.0,
                extension_s: plan.stop.extension_min * 60.0,
                target,
                census_delay_s: census_delay(plan.strategies.census, registry.flags.len() + 1),
                zone_out: plan.strategies.zone_out, // P14 G0: default pending flight 0
                losses: plan.strategies.losses,     // P14 G0: default pending flight 0
            };
            let shell = wrap_sortie(&mut copy, &prefix, &shape, &spec, &mut alloc)?;
            shell.register(area, &mut registry);
            let mut pointer = template::timer(
                &format!("{entry_prefix} COPY {run}"),
                0.0,
                &mut alloc,
                target.0,
                target.1,
            );
            pointer.set_targets(vec![shell.start]);
            let pointer_id = pointer.index.unwrap();
            registry.links.push((shell.started_off, pointer_id));
            if let Some(previous) = copies.last() {
                let previous: &CopyIds = previous;
                registry.links.push((previous.shell.started_on, pointer_id));
            }
            if run > 1 {
                registry.init_off.push(pointer_id);
            }
            if run == entry.runs {
                registry.links.push((shell.start, exhausted_id));
            }
            // Keep the per-copy pointer next to its shell, including for Zone-only entries.
            fn insert(root: &mut Il2Entity, start: i32, node: &mut Option<Il2Entity>) -> bool {
                if root.children.iter().any(|child| child.index == Some(start)) {
                    root.children.push(node.take().unwrap());
                    true
                } else {
                    root.children
                        .iter_mut()
                        .any(|child| insert(child, start, node))
                }
            }
            assert!(insert(&mut copy, shell.start, &mut Some(pointer)));
            copies.push(CopyIds {
                number,
                run,
                root: copy.index.unwrap(),
                pointer: pointer_id,
                shell,
                spawn: facts.spawn,
            });
            result.trees[1].children.push(copy);
            result.warnings.extend(facts.warnings);
        }
        result.entries.push(EntryCopies {
            pool_index,
            number: entry_number,
            area,
            mode: entry.mode,
            exhausted: exhausted_id,
            copies,
        });
    }
    *logic = registry;
    *next_id = alloc;
    Ok(result)
}

/// Apply all deferred links across the side's trees, then return the three
/// housekeeping MCUs for its Logic group. All references are checked first.
pub(crate) fn emit_side_logic(
    side: AirSide,
    logic: &SideLogic,
    trees: &mut [Il2Entity],
    next_id: &mut i32,
) -> Result<Vec<Il2Entity>, String> {
    let mut present = BTreeSet::new();
    for node in trees.iter().flat_map(nodes) {
        if let Some(id) = node.index
            && !present.insert(id)
        {
            return Err(format!("Air Tasking duplicate Index {id}"));
        }
    }
    for &(from, to) in &logic.links {
        if !present.contains(&from) {
            return Err(format!("Air Tasking link source {from} does not exist"));
        }
        if !present.contains(&to) {
            return Err(format!("Air Tasking link target {to} does not exist"));
        }
    }
    for id in logic.mission_begin.iter().chain(&logic.init_off) {
        if !present.contains(id) {
            return Err(format!(
                "Air Tasking initialization target {id} does not exist"
            ));
        }
    }
    for &(from, to) in &logic.links {
        trees
            .iter_mut()
            .find_map(|tree| by_id_mut(tree, from))
            .unwrap()
            .append_target(to);
    }
    *next_id = (*next_id).max(present.last().copied().unwrap_or(0) + 1);
    let mut begin = template::mcu(
        "MCU_TR_MissionBegin",
        "Translator Mission Begin",
        next_id,
        0.0,
        0.0,
    );
    begin.set_property("Enabled", "1");
    let mut init = template::timer(
        &format!("AT {} INIT", side.letter()),
        0.5,
        next_id,
        0.0,
        0.0,
    );
    let mut off = template::mcu(
        "MCU_Deactivate",
        &format!("AT {} INIT OFF", side.letter()),
        next_id,
        0.0,
        0.0,
    );
    off.set_targets(logic.init_off.clone());
    init.set_targets(vec![off.index.unwrap()]);
    begin.set_targets(logic.mission_begin.clone());
    begin.append_target(init.index.unwrap());
    Ok(vec![begin, init, off])
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::airtask::budget;
    use crate::airtask::library::by_id;
    use crate::airtask::shell::test_support::{active, configs, named, ticks, times};
    use crate::airtask::testkit::*;
    use crate::airtask::{AirBudget, PoolEntry, builtin_library};
    use crate::mapclip::WorldAabb;

    pub(crate) fn context() -> AirContext<'static> {
        AirContext {
            east_objectives: &[(180_000.0, 260_000.0)],
            nato_objectives: &[(150_000.0, 260_000.0), (155_000.0, 280_000.0)],
            arrows: &[],
            front: &[(165_000.0, 0.0), (165_000.0, 400_000.0)],
            aabb: WorldAabb {
                x_min: 50_000.0,
                x_max: 300_000.0,
                z_min: 100_000.0,
                z_max: 400_000.0,
            },
            date: (1950, 11, 15),
        }
    }

    pub(crate) fn test_plan() -> AirPlan {
        let mut plan = AirPlan {
            library: builtin_library(),
            budget: AirBudget { per_side: [6, 6] },
            seed: 7,
            ..AirPlan::default()
        };
        for (mode, target) in [
            (TriggerMode::Clock, (150_000.0, 260_000.0)),
            (TriggerMode::Zone, (150_000.0, 260_000.0)),
            (TriggerMode::Both, (155_000.0, 280_000.0)),
        ] {
            let mut entry = PoolEntry::new(
                plan.library[1].id.clone(),
                TargetRef::Objective {
                    side: AirSide::Nato,
                    xz: target,
                },
            );
            entry.mode = mode;
            entry.runs = 2;
            plan.pool.push(entry);
        }
        let mut dprk = PoolEntry::new(
            plan.library[5].id.clone(),
            TargetRef::Objective {
                side: AirSide::Dprk,
                xz: (180_000.0, 260_000.0),
            },
        );
        dprk.runs = 2;
        plan.pool.push(dprk);
        let mut disabled = plan.pool[0].clone();
        disabled.enabled = false;
        plan.pool.push(disabled);
        let mut orphan = plan.pool[0].clone();
        orphan.target = TargetRef::Objective {
            side: AirSide::Nato,
            xz: (0.0, 0.0),
        };
        plan.pool.push(orphan);
        plan
    }

    pub(crate) fn finish(
        side: AirSide,
        result: &mut SideCopies,
        logic: &SideLogic,
        next: &mut i32,
    ) -> Il2Entity {
        let init = emit_side_logic(side, logic, &mut result.trees, next).unwrap();
        result.trees[0].children.extend(init);
        let mut root = group("Air Tasking", next);
        root.children = result.trees.clone();
        assert_links_resolve(&root);
        root
    }

    #[test]
    fn assemble_copy_pointer_structure() {
        let mut logic = SideLogic::default();
        let mut next = 1;
        let mut copies = assemble_side(
            AirSide::Nato,
            &test_plan(),
            &context(),
            &mut logic,
            &mut next,
        )
        .unwrap();
        let root = finish(AirSide::Nato, &mut copies, &logic, &mut next);
        assert_eq!(copies.entries.len(), 3);
        for entry in &copies.entries {
            assert_eq!(entry.copies.len(), 2);
            for (r, copy) in entry.copies.iter().enumerate() {
                assert_eq!(
                    by_id(&root, copy.pointer).name(),
                    Some(format!("EN{:02} COPY {}", entry.number, r + 1).as_str())
                );
                assert_eq!(by_id(&root, copy.pointer).targets, [copy.shell.start]);
                assert_eq!(by_id(&root, copy.shell.started_off).targets, [copy.pointer]);
                if r + 1 < entry.copies.len() {
                    assert!(
                        by_id(&root, copy.shell.started_on)
                            .targets
                            .contains(&entry.copies[r + 1].pointer)
                    );
                } else {
                    assert!(
                        by_id(&root, copy.shell.start)
                            .targets
                            .contains(&entry.exhausted)
                    );
                }
                assert_eq!(logic.init_off.contains(&copy.pointer), r != 0);
            }
        }
        let starts: BTreeSet<_> = copies
            .entries
            .iter()
            .flat_map(|e| e.copies.iter().map(|c| c.shell.start))
            .collect();
        assert_eq!(starts.len(), 6);
    }

    #[test]
    fn assemble_emit_side_logic() {
        let mut logic = SideLogic::default();
        let mut next = 1;
        let mut copies = assemble_side(
            AirSide::Nato,
            &test_plan(),
            &context(),
            &mut logic,
            &mut next,
        )
        .unwrap();
        let test_timer = template::timer("Mission subscriber", 1.0, &mut next, 0.0, 0.0);
        logic.mission_begin.push(test_timer.index.unwrap());
        copies.trees[0].children.push(test_timer);
        let root = finish(AirSide::Nato, &mut copies, &logic, &mut next);
        let begin = named(&copies.trees[0], "Translator Mission Begin");
        assert_eq!(
            begin.targets,
            [
                logic.mission_begin.clone(),
                vec![named(&root, "AT N INIT").index.unwrap()]
            ]
            .concat()
        );
        assert_eq!(named(&root, "AT N INIT").property("Time"), Some("0.50"));
        assert_eq!(named(&root, "AT N INIT OFF").targets, logic.init_off);
        for entry in &copies.entries {
            for copy in &entry.copies {
                assert!(logic.hot_consumers[&entry.area].contains(&copy.shell.hot));
                assert!(logic.cold_consumers[&entry.area].contains(&copy.shell.cold));
                assert!(logic.gates.contains(&copy.shell.start));
                assert!(logic.flags.contains(&copy.shell.flying));
                assert!(logic.budget_events.contains(&copy.shell.start));
                assert!(logic.budget_events.contains(&copy.shell.delete_timer));
            }
        }
    }

    #[test]
    fn assemble_applies_links() {
        let mut logic = SideLogic::default();
        let mut next = 1;
        let mut copies = assemble_side(
            AirSide::Nato,
            &test_plan(),
            &context(),
            &mut logic,
            &mut next,
        )
        .unwrap();
        copies.trees[0].children.extend(budget::build_side(
            AirSide::Nato,
            2,
            &mut logic,
            &test_plan().strategies,
            &mut next,
        ));
        let root = finish(AirSide::Nato, &mut copies, &logic, &mut next);
        for &(from, to) in &logic.links {
            assert!(by_id(&root, from).targets.contains(&to));
        }
        let before = copies.trees.clone();
        let allocator = next;
        logic.links.push((999_999, logic.gates[0]));
        assert!(
            emit_side_logic(AirSide::Nato, &logic, &mut copies.trees, &mut next)
                .unwrap_err()
                .contains("source 999999")
        );
        assert_eq!(copies.trees, before);
        assert_eq!(next, allocator);
    }

    #[test]
    fn assemble_exhausted_is_one_deactivate_per_entry() {
        let mut logic = SideLogic::default();
        let mut next = 1;
        let mut copies = assemble_side(
            AirSide::Nato,
            &test_plan(),
            &context(),
            &mut logic,
            &mut next,
        )
        .unwrap();
        let both = copies
            .entries
            .iter()
            .find(|e| e.mode == TriggerMode::Both)
            .unwrap();
        let exhausted = both.exhausted;
        let entry_number = both.number;
        let mut expected = Vec::new();
        for (prefix, suffix) in [
            ("EN", "AVAILABLE"),
            ("EN", "RANDOM"),
            ("EN", "GO"),
            ("ZN", "REARM"),
            ("ZN", "ARMED"),
            ("ZN", "FIRE"),
        ] {
            let relay = template::timer(
                &format!("{prefix}{entry_number:02} {suffix}"),
                0.0,
                &mut next,
                0.0,
                0.0,
            );
            expected.push(relay.index.unwrap());
            logic.links.push((exhausted, relay.index.unwrap()));
            copies.trees[0].children.push(relay);
        }
        let root = finish(AirSide::Nato, &mut copies, &logic, &mut next);
        assert_eq!(by_id(&root, exhausted).targets, expected);
        for entry in &copies.entries {
            let node = by_id(&root, entry.exhausted);
            assert_eq!(node.block_type, "MCU_Deactivate");
            assert_eq!(
                nodes(&root)
                    .iter()
                    .filter(|n| n.name() == node.name())
                    .count(),
                1
            );
            assert!(
                by_id(&root, entry.copies.last().unwrap().shell.start)
                    .targets
                    .contains(&entry.exhausted)
            );
        }
    }

    #[test]
    fn assemble_side_numbers_copies_and_uses_spawn_rule() {
        let plan = test_plan();
        let mut next = 1;
        for side in [AirSide::Dprk, AirSide::Nato] {
            let mut logic = SideLogic::default();
            let mut copies = assemble_side(side, &plan, &context(), &mut logic, &mut next).unwrap();
            copies.trees[0].children.extend(budget::build_side(
                side,
                6,
                &mut logic,
                &plan.strategies,
                &mut next,
            ));
            let root = finish(side, &mut copies, &logic, &mut next);
            let flat: Vec<_> = copies.entries.iter().flat_map(|e| &e.copies).collect();
            assert_eq!(flat.len(), if side == AirSide::Nato { 6 } else { 2 });
            assert_eq!(
                flat.iter().map(|c| c.number).collect::<Vec<_>>(),
                (1..=flat.len()).collect::<Vec<_>>()
            );
            for entry in &copies.entries {
                let target = copies.areas[entry.area - 1];
                for copy in &entry.copies {
                    let distance = (copy.spawn.0 - target.0).hypot(copy.spawn.1 - target.1);
                    let speed = plan
                        .library
                        .iter()
                        .find(|s| s.id == plan.pool[entry.pool_index].sortie_id)
                        .unwrap()
                        .wp_speed_kmh;
                    let expected = if entry.mode == TriggerMode::Clock {
                        speed / 3.6 * 300.0
                    } else {
                        30_000.0
                    };
                    assert!((distance - expected).abs() <= 1.0);
                    assert!(
                        by_id(&root, copy.root)
                            .name()
                            .unwrap()
                            .starts_with(&format!("S{}{:02} ", side.letter(), copy.number))
                    );
                }
            }
            let script = WalkerInput {
                until: ticks(20.0),
                pulses: flat
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (ticks(2.0 + i as f64 * 2.0), c.pointer.into()))
                    .collect(),
                ..WalkerInput::default()
            };
            for config in configs() {
                let log = walk(&root, config, &script);
                for (i, copy) in flat.iter().enumerate() {
                    assert_eq!(times(&log, copy.shell.start), [ticks(2.0 + i as f64 * 2.0)]);
                    assert!(active(&log, copy.shell.flying, ticks(20.0)));
                    assert!(!active(&log, copy.pointer, ticks(20.0)));
                }
            }
        }
    }
}
