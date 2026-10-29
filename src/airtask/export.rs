//! One deterministic export, shared by readiness, totals and Map generation.

use std::collections::BTreeSet;

use super::assemble::{
    RUNS_RANGE, SideCopies, assemble_side, emit_side_logic, resolve_placement, side_tree_names,
};
use super::place::{SortiePlacement, spawn_point};
use super::shell::group;
use super::{
    AirCheck, AirContext, AirIssue, AirLine, AirPlan, AirPoint, AirPreview, AirRing, AirSide, AirTotals,
    BudgetImpl, IssueSeverity, SideLogic, SlotMark, SortieSource, TriggerMode, budget, clock,
    threshold, timing, zone,
};
use crate::ast::Il2Entity;
use crate::frontlines::{AIR_TASKING_GROUP, MapAirPack};
use crate::locale;

const SIDES: [AirSide; 2] = [AirSide::Dprk, AirSide::Nato];
const PREVIEW_WINDOW_S: f64 = 3.0 * 3600.0;

struct EligibleEntry {
    pool_index: usize,
    side: AirSide,
    placement: SortiePlacement,
}

fn issue(severity: IssueSeverity, text: String) -> AirIssue {
    AirIssue {
        severity,
        text,
        rail_only: false,
    }
}

fn blocking(issues: &[AirIssue]) -> Option<&AirIssue> {
    issues
        .iter()
        .find(|issue| issue.blocks_build())
}

/// Eligibility follows assembly: disabled, out-of-date and orphaned entries
/// contribute no copies. Invalid enabled entries still receive diagnostics.
fn entries(plan: &AirPlan, ctx: &AirContext<'_>) -> (Vec<EligibleEntry>, Vec<AirIssue>) {
    let mut eligible = Vec::new();
    let mut issues = Vec::new();
    for (pool_index, entry) in plan.pool.iter().enumerate().filter(|(_, e)| e.enabled) {
        let number = pool_index + 1;
        let Some(sortie) = plan.library.iter().find(|s| s.id == entry.sortie_id) else {
            issues.push(issue(
                IssueSeverity::Error,
                format!(
                    "Air Tasking Entry {number}: cannot find sortie {} in the library",
                    entry.sortie_id
                ),
            ));
            continue;
        };
        let label = format!("Air Tasking {} Entry {number}", sortie.side.label());
        if ctx.date < sortie.valid.from || ctx.date > sortie.valid.to {
            issues.push(issue(
                IssueSeverity::Warning,
                format!(
                    "{label}: sortie is out of date for {:04}-{:02}-{:02}; skipped",
                    ctx.date.0, ctx.date.1, ctx.date.2
                ),
            ));
            continue;
        }
        let Some((side, placement)) = resolve_placement(entry, plan, ctx, sortie.wp_speed_kmh)
        else {
            issues.push(issue(
                IssueSeverity::Warning,
                format!("{label}: orphaned target; skipped"),
            ));
            continue;
        };
        if side != sortie.side {
            issues.push(issue(
                IssueSeverity::Error,
                format!("{label}: target is on the wrong side ({})", side.label()),
            ));
            continue;
        }
        if !RUNS_RANGE.contains(&entry.runs) {
            issues.push(issue(
                IssueSeverity::Error,
                format!("{label}: runs must be 1 to 6"),
            ));
        }
        if matches!(entry.mode, TriggerMode::Clock | TriggerMode::Both)
            && !clock::WEIGHT_RANGE.contains(&entry.weight)
        {
            issues.push(issue(
                IssueSeverity::Error,
                format!("{label}: weight must be 1 to 10"),
            ));
        }
        if let Err(error) = spawn_point(&placement) {
            issues.push(issue(IssueSeverity::Error, format!("{label}: {error}")));
        }
        eligible.push(EligibleEntry {
            pool_index,
            side,
            placement,
        });
    }
    (eligible, issues)
}

fn areas(entries: &[EligibleEntry], side: AirSide) -> Vec<(f64, f64)> {
    let mut result = Vec::new();
    for entry in entries.iter().filter(|entry| entry.side == side) {
        if !result.contains(&entry.placement.target) {
            result.push(entry.placement.target);
        }
    }
    result
}

fn segment_distance(point: (f64, f64), from: (f64, f64), to: (f64, f64)) -> f64 {
    let length = (to.0 - from.0).hypot(to.1 - from.1);
    if length == 0.0 {
        return (point.0 - from.0).hypot(point.1 - from.1);
    }
    let unit = ((to.0 - from.0) / length, (to.1 - from.1) / length);
    let along = ((point.0 - from.0) * unit.0 + (point.1 - from.1) * unit.1).clamp(0.0, length);
    (point.0 - from.0 - along * unit.0).hypot(point.1 - from.1 - along * unit.1)
}

fn path_issues(plan: &AirPlan, entries: &[EligibleEntry]) -> Vec<AirIssue> {
    let radius = timing::derived_timing(&plan.threshold).inner_m;
    let mut issues = Vec::new();
    for side in SIDES {
        let other = side.other();
        let other_areas = areas(entries, other);
        for entry in entries.iter().filter(|entry| entry.side == side) {
            let p = &entry.placement;
            let Ok(spawn) = spawn_point(p) else { continue };
            for (index, &centre) in other_areas.iter().enumerate() {
                if segment_distance(centre, spawn, p.target)
                    .min(segment_distance(centre, p.target, p.rtb))
                    <= radius
                {
                    issues.push(issue(
                        if plan.strategies.ai_counts { IssueSeverity::Error } else { IssueSeverity::Warning },
                        format!(
                            "Air Tasking {} Entry {}: path passes within the inner radius ({radius:.0} m) of {} area {}",
                            side.label(), entry.pool_index + 1, other.label(), index + 1
                        ),
                    ));
                }
            }
        }
    }
    issues
}

fn locale_issues(plan: &AirPlan, eligible: &[EligibleEntry]) -> Vec<AirIssue> {
    let files: BTreeSet<_> = eligible
        .iter()
        .map(|entry| &plan.pool[entry.pool_index])
        .filter_map(|entry| plan.library.iter().find(|s| s.id == entry.sortie_id))
        .filter_map(|sortie| match &sortie.source {
            SortieSource::File(path) => Some(path.clone()),
            SortieSource::BuiltIn(_) => None,
        })
        .collect();
    let files: Vec<_> = files
        .into_iter()
        .map(|path| {
            let tables = locale::merge_template_sidecars(std::slice::from_ref(&path));
            (path, tables)
        })
        .collect();
    let mut issues = Vec::new();
    for (i, (first, tables)) in files.iter().enumerate() {
        for (second, other) in &files[i + 1..] {
            let mut conflicts = BTreeSet::new();
            for ext in locale::LANG_EXTS {
                let (Some(a), Some(b)) = (tables.get(*ext), other.get(*ext)) else {
                    continue;
                };
                for id in a.ids() {
                    if b.get(id).is_some_and(|text| Some(text) != a.get(id)) {
                        conflicts.insert(id);
                    }
                }
            }
            for id in conflicts {
                issues.push(issue(
                    IssueSeverity::Warning,
                    format!(
                        "Air Tasking templates {} and {} have different text for locale id {id}",
                        first.display(),
                        second.display()
                    ),
                ));
            }
        }
    }
    issues
}

struct SideTrees {
    logic: Il2Entity,
    sorties: Il2Entity,
}

fn side_error(side: AirSide, error: String) -> AirIssue {
    issue(
        IssueSeverity::Error,
        if error.starts_with("Air Tasking ") {
            error
        } else {
            format!("Air Tasking {}: {error}", side.label())
        },
    )
}

fn build_side(
    side: AirSide,
    plan: &AirPlan,
    mut copies: SideCopies,
    mut logic: SideLogic,
    next_id: &mut i32,
) -> Result<SideTrees, String> {
    let clock = clock::build_side(side, plan, &copies, &mut logic, next_id)?;
    copies.trees[0].children.extend(clock);
    let zone = zone::build_side(side, plan, &copies, &mut logic, next_id)?;
    copies.trees[0].children.extend(zone);
    copies.trees[0].children.extend(threshold::build_side(
        side,
        &copies.areas,
        &plan.threshold,
        &plan.strategies,
        &mut logic,
        next_id,
    )?);
    copies.trees[0].children.extend(budget::build_side(
        side,
        plan.budget.per_side[side.idx()],
        &mut logic,
        &plan.strategies,
        next_id,
    ));
    let init = emit_side_logic(side, &logic, &mut copies.trees, next_id)?;
    copies.trees[0].children.extend(init);
    let country = threshold::ai_country(side);
    copies.trees[1].for_each_mut(&mut |node| {
        if node.block_type == "Plane" {
            node.set_property("Country", country.to_string());
        }
    });
    let [logic, sorties] = copies.trees.try_into().map_err(|_| {
        format!(
            "Air Tasking {}: expected Logic and Sorties groups",
            side.label()
        )
    })?;
    Ok(SideTrees { logic, sorties })
}

/// The indexes of the Air Tasking nodes a trace build follows by name (the
/// sortie cycle's decision points: slot opens, GO, the zone and threshold
/// steps, and each sortie's START to DELETE DELAY). Sorted, no repeats.
pub(super) fn trace_role_indexes(root: &Il2Entity) -> Vec<i32> {
    let mut indexes = BTreeSet::new();
    root.for_each(&mut |group| {
        if group.block_type != "Group" || group.name() != Some(AIR_TASKING_GROUP) {
            return;
        }
        group.for_each(&mut |node| {
            let name = node.name().unwrap_or("");
            let role = name.split_once(' ').map(|(prefix, role)| {
                (prefix.trim_end_matches(|c: char| c.is_ascii_digit()), role)
            });
            let traced = matches!(
                role,
                Some(("CLK", "D SLOT OPEN" | "N SLOT OPEN"))
                    | Some(("ED" | "EN", "GO"))
                    | Some((
                        "ZD" | "ZN",
                        "CHANCE" | "MISS WAIT" | "FIRE" | "DENY WAIT" | "COOLDOWN" | "ARM"
                    ))
                    // INNER is PASS. PASS OFF cancels CLOSE DELAY on success;
                    // CLOSE still fires then, so only CLOSE DELAY means COLD.
                    | Some(("THD" | "THN", "INNER" | "CLOSE DELAY"))
                    | Some((
                        "SD" | "SN",
                        "START" | "ON STATION" | "EXTEND" | "COLD" | "LOSSES"
                            | "HARD STOP" | "DONE HUB" | "DELETE DELAY"
                    ))
                    | Some(("Zone", "Out"))
            );
            if traced {
                indexes.extend(node.index);
            }
        });
    });
    indexes.into_iter().collect()
}

#[cfg(test)]
thread_local! {
    static EVALUATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn evaluations() -> usize {
    EVALUATIONS.with(|n| n.get())
}

/// Each public operation evaluates once. Validation reuses the completed tree
/// for totals, and builders never call back into the public entry points.
fn evaluate(plan: &AirPlan, ctx: &AirContext<'_>) -> (Vec<MapAirPack>, Vec<AirIssue>) {
    #[cfg(test)]
    EVALUATIONS.with(|n| n.set(n.get() + 1));
    let (eligible, mut issues) = entries(plan, ctx);
    issues.extend(locale_issues(plan, &eligible));
    if eligible.is_empty() {
        issues.push(AirIssue {
            severity: IssueSeverity::Error,
            text: "Air Tasking has no enabled entry valid for the date with a current target"
                .into(),
            rail_only: true,
        });
        return (Vec::new(), issues);
    }
    for side in SIDES {
        if eligible.iter().any(|entry| entry.side == side)
            && plan.strategies.budget != BudgetImpl::SingleSlot
            && !budget::BUDGET_RANGE.contains(&plan.budget.per_side[side.idx()])
        {
            issues.push(issue(
                IssueSeverity::Error,
                format!("Air Tasking {} budget must be 1 to 6", side.label()),
            ));
        }
    }
    // An empty-area threshold call validates the spec without emitting anything.
    // Keep its diagnostics in the owning builder rather than copying its rules.
    if let Err(error) = threshold::build_side(
        AirSide::Dprk,
        &[],
        &plan.threshold,
        &plan.strategies,
        &mut SideLogic::default(),
        &mut 1,
    ) {
        issues.push(issue(IssueSeverity::Error, error));
    }
    issues.extend(path_issues(plan, &eligible));
    let mut next_id = 1;
    let mut assembled = Vec::new();
    for side in SIDES {
        if !eligible.iter().any(|entry| entry.side == side) {
            continue;
        }
        let mut logic = SideLogic::default();
        match assemble_side(side, plan, ctx, &mut logic, &mut next_id) {
            Ok(copies) => {
                issues.extend(clock::validate_schedule(
                    side,
                    &plan.schedule[side.idx()],
                    &copies.entries,
                ));
                issues.extend(zone::validate_zone(side, plan, &copies.entries));
                assembled.push((side, copies, logic));
            }
            Err(error) => issues.push(side_error(side, error)),
        }
    }
    if blocking(&issues).is_some() {
        return (Vec::new(), issues);
    }
    let mut logic_trees = Vec::new();
    let mut sortie_trees = Vec::new();
    for (side, copies, logic) in assembled {
        match build_side(side, plan, copies, logic, &mut next_id) {
            Ok(trees) => {
                logic_trees.push(trees.logic);
                sortie_trees.push(trees.sorties);
            }
            Err(error) => {
                issues.push(side_error(side, error));
                return (Vec::new(), issues);
            }
        }
    }
    let mut root = group(AIR_TASKING_GROUP, &mut next_id);
    root.children = logic_trees;
    root.children.extend(sortie_trees);
    let packs = vec![MapAirPack { root }];
    let totals = count_totals(&packs);
    for (name, counts, limit) in [
        ("aircraft", totals.aircraft, plan.limits.aircraft),
        ("MCUs", totals.mcus, plan.limits.mcus),
        ("check zones", totals.check_zones, plan.limits.check_zones),
    ] {
        let total: u64 = counts.into_iter().map(u64::from).sum();
        if total > u64::from(limit) {
            issues.push(issue(
                IssueSeverity::Warning,
                format!("Air Tasking total {name} ({total}) exceeds the limit ({limit})"),
            ));
        }
    }
    (packs, issues)
}

pub(super) fn build_air_packs(
    plan: &AirPlan,
    ctx: &AirContext<'_>,
) -> Result<Vec<MapAirPack>, String> {
    let (packs, issues) = evaluate(plan, ctx);
    match blocking(&issues) {
        Some(issue) => Err(issue.text.clone()),
        None => Ok(packs),
    }
}

#[cfg(test)] // `check` serves the UI
pub(super) fn validate(plan: &AirPlan, ctx: &AirContext<'_>) -> Vec<AirIssue> {
    evaluate(plan, ctx).1
}

fn count_totals(packs: &[MapAirPack]) -> AirTotals {
    let mut totals = AirTotals::default();
    for pack in packs {
        for side in SIDES {
            let i = side.idx();
            for (kind, name) in side_tree_names(side).into_iter().enumerate() {
                let Some(tree) = pack
                    .root
                    .children
                    .iter()
                    .find(|tree| tree.name() == Some(name.as_str()))
                else {
                    continue;
                };
                if kind == 1 {
                    totals.copies[i] += tree.children.len() as u32;
                }
                tree.for_each(&mut |node| {
                    totals.aircraft[i] += u32::from(node.block_type == "Plane");
                    totals.mcus[i] += u32::from(node.block_type.starts_with("MCU_"));
                    totals.check_zones[i] += u32::from(node.block_type == "MCU_CheckZone");
                });
            }
        }
    }
    totals
}

/// One evaluation for the UI: `validate`'s issues, `totals` (zero when the
/// plan cannot build, as `totals` gives) and the geometry-only `preview`.
pub(super) fn check(plan: &AirPlan, ctx: &AirContext<'_>) -> AirCheck {
    let (packs, issues) = evaluate(plan, ctx);
    let totals = if blocking(&issues).is_some() {
        AirTotals::default()
    } else {
        count_totals(&packs)
    };
    AirCheck {
        issues,
        totals,
        preview: preview(plan, ctx),
    }
}

#[cfg(test)] // `check` serves the UI
pub(super) fn totals(plan: &AirPlan, ctx: &AirContext<'_>) -> AirTotals {
    build_air_packs(plan, ctx)
        .map(|packs| count_totals(&packs))
        .unwrap_or_default()
}

pub(super) fn preview(plan: &AirPlan, ctx: &AirContext<'_>) -> AirPreview {
    let (entries, _) = entries(plan, ctx);
    let mut preview = AirPreview::default();
    for entry in &entries {
        let p = &entry.placement;
        let Ok(spawn) = spawn_point(p) else { continue };
        preview.lines.push(AirLine {
            side: entry.side,
            entry: entry.pool_index,
            from: spawn,
            to: p.target,
        });
        preview.spawns.push(AirPoint {
            side: entry.side,
            entry: entry.pool_index,
            at: spawn,
            target: p.target,
        });
        preview.rtbs.push(AirPoint {
            side: entry.side,
            entry: entry.pool_index,
            at: p.rtb,
            target: p.target,
        });
    }
    for side in SIDES {
        for centre in areas(&entries, side) {
            for (radius_m, inner) in [
                (plan.threshold.radius_m, false),
                (timing::derived_timing(&plan.threshold).inner_m, true),
            ] {
                preview.rings.push(AirRing {
                    side,
                    centre,
                    radius_m,
                    inner,
                });
            }
        }
        let schedule = &plan.schedule[side.idx()];
        if !schedule.enabled
            || !entries.iter().any(|entry| {
                entry.side == side
                    && matches!(
                        plan.pool[entry.pool_index].mode,
                        TriggerMode::Clock | TriggerMode::Both
                    )
            })
        {
            continue;
        }
        let first = schedule.first_slot_min * 60.0;
        let period = schedule.period_min * 60.0;
        let jitter_span_s =
            f64::from(schedule.jitter_steps.saturating_sub(1)) * schedule.jitter_step_min * 60.0;
        if !first.is_finite() || first < 0.0 || !period.is_finite() || period <= 0.0 {
            continue;
        }
        let mut k = 0;
        while first + f64::from(k) * period <= PREVIEW_WINDOW_S {
            preview.slot_times[side.idx()].push(SlotMark {
                t_s: first + f64::from(k) * period,
                jitter_span_s,
                quiet_pct: schedule.quiet_pct,
            });
            k += 1;
        }
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::super::library::{BUILTINS, by_id, inspect_sortie, nodes, number};
    use super::super::place::place_air_sortie;
    use super::super::shell::test_support::named;
    use super::super::testkit::{
        assert_links_resolve, walker_every_deactivated_out_has_a_rearm_or_is_single_use,
    };
    use super::super::{AirDrawCounts, PoolEntry, TargetRef, builtin_library, load_user_sortie};
    use super::*;
    use crate::ast::Il2Entity;
    use crate::baseline_tests::air_export_front;
    use crate::frontlines::{self, FrontOptions};
    use crate::{duplicate, mapclip, parser, serialize, trace};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    // Acceptance flights A1 onward, and the checks of every frozen
    // acceptance flight (src/airtask/export_acceptance_flights.rs).
    #[path = "../../export_acceptance_flights.rs"]
    mod acceptance_flights;

    struct TestContext {
        opts: FrontOptions,
        front: Vec<(f64, f64)>,
        east: Vec<(f64, f64)>,
        nato: Vec<(f64, f64)>,
        date: super::super::Ymd,
    }

    impl TestContext {
        fn context(&self) -> AirContext<'_> {
            AirContext {
                east_objectives: &self.east,
                nato_objectives: &self.nato,
                arrows: &self.opts.user_attacks,
                front: &self.front,
                aabb: self.opts.aabb,
                date: self.date,
            }
        }
    }

    fn raw_front(opts: &FrontOptions) -> Vec<(f64, f64)> {
        opts.custom_front.clone().unwrap_or_else(|| {
            let i = opts
                .timeline_idx
                .unwrap_or_else(|| frontlines::timeline_index(opts.year, opts.season));
            frontlines::front_xz(&frontlines::TIMELINE[i])
        })
    }

    fn export_test_plan() -> (AirPlan, TestContext) {
        let mut opts = crate::baseline_tests::air_export_options();
        let front = air_export_front(&opts);
        let original = opts.user_attacks[0];
        // The committed fixture's arrow is NATO. Use a separate DPRK arrow
        // in this test map/context; never change the fixture or its generator.
        assert!(!mapclip::point_north_of_front(
            &front,
            original.0.0,
            original.0.1
        ));
        opts.user_attacks = vec![((250_000.0, 260_000.0), (220_000.0, 260_000.0))];
        let input = TestContext {
            opts,
            front,
            east: vec![(220_000.0, 260_000.0)],
            nato: vec![(150_000.0, 300_000.0)],
            date: (1950, 9, 15),
        };
        let mut plan = AirPlan {
            library: builtin_library(),
            seed: 7,
            ..AirPlan::default()
        };
        plan.pool.push(PoolEntry::new(
            plan.library[1].id.clone(),
            TargetRef::Objective {
                side: AirSide::Nato,
                xz: input.nato[0],
            },
        ));
        let (tail, tip) = input.opts.user_attacks[0];
        let mut entry = PoolEntry::new(plan.library[5].id.clone(), TargetRef::Arrow { tail, tip });
        entry.mode = TriggerMode::Both;
        plan.pool.push(entry);
        (plan, input)
    }

    fn acceptance_test_plan() -> (AirPlan, TestContext) {
        let mut opts = crate::baseline_tests::air_export_options();
        let i = frontlines::TIMELINE
            .iter()
            .position(|mark| (mark.year, mark.month, mark.day) == (1950, 9, 15))
            .unwrap();
        opts.timeline_idx = Some(i);
        opts.year = frontlines::TIMELINE[i].year;
        opts.season = frontlines::TIMELINE[i].season;
        let input = TestContext {
            front: air_export_front(&opts),
            opts,
            east: vec![(45_000.0, 280_000.0), (185_000.0, 280_000.0)],
            nato: vec![(55_000.0, 270_000.0), (55_000.0, 300_000.0)],
            date: (1950, 9, 15),
        };
        let mut plan = AirPlan {
            library: builtin_library(),
            seed: 7,
            ..AirPlan::default()
        };
        for (sortie, area, mode) in [
            (0, 0, TriggerMode::Clock),
            (1, 1, TriggerMode::Clock),
            (2, 0, TriggerMode::Zone),
            (3, 1, TriggerMode::Both),
            (4, 0, TriggerMode::Zone),
            (5, 1, TriggerMode::Both),
        ] {
            let side = plan.library[sortie].side;
            let xz = if side == AirSide::Dprk {
                input.east[area]
            } else {
                input.nato[area]
            };
            let mut entry = PoolEntry::new(
                plan.library[sortie].id.clone(),
                TargetRef::Objective { side, xz },
            );
            entry.mode = mode;
            plan.pool.push(entry);
        }
        // AirAlert crosses DPRK area 1 to exercise D30 while its spawn stays
        // on NATO's side of the front; the other paths clear the opposing areas.
        plan.pool[0].heading_override = Some(320.0);
        (plan, input)
    }

    fn pack(plan: &AirPlan, ctx: &AirContext<'_>) -> Il2Entity {
        let mut packs = super::super::build_air_packs(plan, ctx).unwrap();
        assert_eq!(packs.len(), 1);
        packs.remove(0).root
    }

    fn assert_unique(root: &Il2Entity) {
        let mut indexes = BTreeSet::new();
        root.for_each(&mut |node| {
            if let Some(index) = node.index {
                assert!(indexes.insert(index), "duplicate index {index}");
            }
        });
    }

    fn assert_no_errors(issues: &[AirIssue]) {
        assert!(
            issues
                .iter()
                .all(|issue| issue.severity != IssueSeverity::Error),
            "{issues:?}"
        );
        assert!(issues.iter().all(|issue| !issue.rail_only));
    }

    fn d30(issues: &[AirIssue]) -> Vec<&AirIssue> {
        issues
            .iter()
            .filter(|issue| issue.text.contains("path passes within the inner radius"))
            .collect()
    }

    #[test]
    fn export_plan_has_air_tasking_groups() {
        let (plan, mut input) = export_test_plan();
        input.opts.air_packs = super::super::build_air_packs(&plan, &input.context()).unwrap();
        let root = frontlines::generate_front(&input.opts).unwrap().root;
        assert_eq!(
            root.children
                .iter()
                .filter(|g| g.name() == Some("Air Tasking"))
                .count(),
            1
        );
        let air = named(&root, "Air Tasking");
        assert_eq!(
            air.children
                .iter()
                .map(|g| g.name().unwrap())
                .collect::<Vec<_>>(),
            [
                "Air Tasking DPRK Logic",
                "Air Tasking NATO Logic",
                "Air Tasking DPRK Sorties",
                "Air Tasking NATO Sorties",
            ]
        );
        assert_links_resolve(&root);
        assert_unique(&root);
        walker_every_deactivated_out_has_a_rearm_or_is_single_use(air, &[]).unwrap();
        // An air pack is sufficient to satisfy Generate's enabled-layer check.
        let opts = FrontOptions {
            front: false,
            battles: false,
            buildups: false,
            defenses: false,
            attacks: false,
            naval: false,
            influence: false,
            air_packs: input.opts.air_packs,
            ..FrontOptions::default()
        };
        assert!(frontlines::generate_front(&opts).is_ok());
    }

    #[test]
    fn export_is_deterministic() {
        for factory in [export_test_plan, acceptance_test_plan] {
            let (plan, input) = factory();
            assert_eq!(
                serialize::serialize_group(&pack(&plan, &input.context())),
                serialize::serialize_group(&pack(&plan, &input.context()))
            );
        }
    }

    #[test]
    fn export_sets_the_ai_country() {
        let (plan, input) = acceptance_test_plan();
        let root = pack(&plan, &input.context());
        let mut next = 1;
        for (side, country, original) in
            [(AirSide::Dprk, "503", "503"), (AirSide::Nato, "602", "601")]
        {
            let sorties = named(&root, &format!("Air Tasking {} Sorties", side.label()));
            let planes: Vec<_> = nodes(sorties)
                .into_iter()
                .filter(|n| n.block_type == "Plane")
                .collect();
            assert!(!planes.is_empty());
            assert!(
                planes
                    .iter()
                    .all(|p| p.property("Country") == Some(country))
            );
            let copies = assemble_side(
                side,
                &plan,
                &input.context(),
                &mut SideLogic::default(),
                &mut next,
            )
            .unwrap();
            assert!(
                nodes(&copies.trees[1])
                    .iter()
                    .filter(|n| n.block_type == "Plane")
                    .all(|p| p.property("Country") == Some(original))
            );
        }
        for (i, entry) in plan.pool.iter().enumerate() {
            let mut source = parser::parse_group_file(BUILTINS[i].1).unwrap();
            let (_, placement) =
                resolve_placement(entry, &plan, &input.context(), plan.library[i].wp_speed_kmh)
                    .unwrap();
            let before: Vec<_> = nodes(&source)
                .iter()
                .filter(|n| n.block_type == "Plane")
                .map(|p| p.property("Country").unwrap().to_owned())
                .collect();
            place_air_sortie(&mut source, &placement).unwrap();
            assert_eq!(
                nodes(&source)
                    .iter()
                    .filter(|n| n.block_type == "Plane")
                    .map(|p| p.property("Country").unwrap().to_owned())
                    .collect::<Vec<_>>(),
                before
            );
        }
    }

    #[test]
    fn stamp_leaves_air_pack_formations_unchanged() {
        let (plan, mut input) = acceptance_test_plan();
        let air = pack(&plan, &input.context());
        input.opts.air_packs = vec![MapAirPack { root: air.clone() }];
        let root = frontlines::generate_front(&input.opts).unwrap().root;
        let stamped = named(&root, "Air Tasking");
        let mut next = stamped.index.unwrap();
        let (expected, _) = duplicate::duplicate_template(&air, &mut next);
        assert_eq!(stamped, &expected);
        assert!(
            nodes(stamped)
                .iter()
                .any(|n| n.block_type == "MCU_CMD_Formation")
        );
    }

    #[test]
    fn load_then_generate_has_one_air_tasking_group() {
        let (plan, input) = export_test_plan();
        crate::baseline_tests::air_export_tests::check_load_then_generate(
            &plan,
            input.opts.clone(),
            &input.context(),
        );
    }

    #[test]
    fn validate_clean_plan_has_no_issues() {
        let (plan, input) = export_test_plan();
        assert_eq!(super::super::validate(&plan, &input.context()), []);
    }

    #[test]
    fn validate_flags_orphaned_target() {
        let (mut plan, input) = export_test_plan();
        plan.pool[0].target = TargetRef::Objective {
            side: AirSide::Nato,
            xz: (0.0, 0.0),
        };
        let issues = super::super::validate(&plan, &input.context());
        assert_no_errors(&issues);
        assert_eq!(issues.len(), 1);
        assert!(
            issues[0].text.contains("NATO Entry 1") && issues[0].text.contains("orphaned target")
        );
        assert_eq!(super::super::totals(&plan, &input.context()).copies, [3, 0]);
    }

    struct TempTemplates(PathBuf);

    impl TempTemplates {
        fn new(label: &str) -> Self {
            static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "il2-p14-export-{label}-{}-{serial}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, name: &str, root: &Il2Entity) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, serialize::serialize_group(root)).unwrap();
            path
        }
    }

    impl Drop for TempTemplates {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn validate_flags_out_of_date_sortie() {
        let temp = TempTemplates::new("date");
        let mut la11 = parser::parse_group_file(BUILTINS[4].1).unwrap();
        la11.for_each_mut(&mut |node| {
            if node.block_type == "Plane" {
                for key in ["Script", "Model"] {
                    let value = node.property(key).unwrap().replace("yak9p", "la11");
                    node.set_property(key, value);
                }
            }
        });
        let path = temp.write("La11.Group", &la11);
        let sortie = load_user_sortie(&path).unwrap();
        assert_eq!(sortie.models, ["la11"]);
        let (mut plan, input) = export_test_plan();
        plan.pool[1].sortie_id = sortie.id.clone();
        plan.library.push(sortie);
        let issues = super::super::validate(&plan, &input.context());
        assert_no_errors(&issues);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].text.contains("DPRK Entry 2") && issues[0].text.contains("out of date"));
        assert_eq!(super::super::totals(&plan, &input.context()).copies, [0, 3]);
    }

    #[test]
    fn validate_flags_d30_path_conflict() {
        let (mut plan, input) = acceptance_test_plan();
        let issues = super::super::validate(&plan, &input.context());
        assert_no_errors(&issues);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(d30(&issues).len(), 1);
        assert!(issues[0].text.contains("NATO Entry 1") && issues[0].text.contains("DPRK area 1"));
        // At 8 km from area 1 the path is inside R but outside r_in: no D30.
        plan.pool[0].heading_override = Some(160.0);
        assert!(d30(&super::super::validate(&plan, &input.context())).is_empty());
    }

    #[test]
    fn validate_d30_is_an_error_when_ai_counts() {
        let (mut plan, input) = acceptance_test_plan();
        plan.strategies.ai_counts = true;
        let issues = super::super::validate(&plan, &input.context());
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(d30(&issues).len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Error);
        assert!(!issues[0].rail_only);
        assert_eq!(
            super::super::build_air_packs(&plan, &input.context()).unwrap_err(),
            issues[0].text
        );
    }

    #[test]
    fn validate_orphaned_only_pool_issue_is_rail_only() {
        let (plan, input) = export_test_plan();
        let ctx = AirContext {
            nato_objectives: &[],
            arrows: &[],
            ..input.context()
        };
        let issues = super::super::validate(&plan, &ctx);
        assert_eq!(
            issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Warning)
                .count(),
            2
        );
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].rail_only);
        assert_eq!(issues.iter().filter(|i| i.rail_only).count(), 1);
        assert!(
            super::super::build_air_packs(&plan, &ctx)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn validate_empty_pool_is_an_error() {
        let (mut plan, input) = export_test_plan();
        plan.pool.clear();
        let issues = super::super::validate(&plan, &input.context());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Error);
        assert!(issues[0].rail_only);
        assert!(
            super::super::build_air_packs(&plan, &input.context())
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            super::super::totals(&plan, &input.context()),
            AirTotals::default()
        );
    }

    #[test]
    fn validate_budget_zero_is_an_error() {
        let (mut plan, input) = export_test_plan();
        for side in SIDES {
            plan.budget.per_side = [2, 2];
            plan.budget.per_side[side.idx()] = 0;
            let issues = super::super::validate(&plan, &input.context());
            assert_eq!(issues.len(), 1);
            assert_eq!(issues[0].severity, IssueSeverity::Error);
            assert!(issues[0].text.contains(side.label()) && issues[0].text.contains("budget"));
            assert!(!issues[0].rail_only);
            assert_eq!(
                super::super::build_air_packs(&plan, &input.context()).unwrap_err(),
                issues[0].text
            );
        }
        plan.pool[0].enabled = false;
        // NATO has no entries; its zero budget does not affect DPRK's export.
        assert!(super::super::validate(&plan, &input.context()).is_empty());
    }

    #[test]
    fn validate_single_slot_ignores_budget_setting() {
        let (mut plan, input) = export_test_plan();
        plan.strategies.budget = BudgetImpl::SingleSlot;
        plan.budget.per_side = [1, 1];
        let expected = serialize::serialize_group(&pack(&plan, &input.context()));
        for budget in [0, 7, u8::MAX] {
            plan.budget.per_side = [budget; 2];
            assert!(super::super::validate(&plan, &input.context()).is_empty());
            assert_eq!(
                serialize::serialize_group(&pack(&plan, &input.context())),
                expected
            );
        }
    }

    #[test]
    fn validate_reports_schedule_error_with_budget_error() {
        let (base, input) = export_test_plan();
        for budget_side in SIDES {
            for clock_side in SIDES {
                let mut plan = base.clone();
                plan.budget.per_side[budget_side.idx()] = 0;
                plan.schedule[clock_side.idx()].period_min = 0.0;
                let issues = super::super::validate(&plan, &input.context());
                assert_eq!(issues.len(), 2, "{issues:?}");
                assert!(
                    issues.iter().all(|issue| {
                        issue.severity == IssueSeverity::Error && !issue.rail_only
                    })
                );
                for text in [
                    format!("{} budget", budget_side.label()),
                    format!("{} clock: period", clock_side.label()),
                ] {
                    assert!(
                        issues.iter().any(|issue| issue.text.contains(&text)),
                        "{issues:?}"
                    );
                }
                assert!(super::super::build_air_packs(&plan, &input.context()).is_err());
                assert_eq!(
                    super::super::totals(&plan, &input.context()),
                    AirTotals::default()
                );
            }
        }
        let mut plan = base;
        plan.budget.per_side = [0, 0];
        plan.zone.chance_pct = 0;
        let issues = super::super::validate(&plan, &input.context());
        assert_eq!(issues.len(), 3, "{issues:?}");
        assert!(
            issues
                .iter()
                .all(|issue| issue.severity == IssueSeverity::Error)
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.text.contains("DPRK zone: chance"))
        );
    }

    #[test]
    fn validate_ignores_invalid_values_on_skipped_entries() {
        let (base, input) = export_test_plan();
        let mut expected = base.clone();
        expected.pool[0].enabled = false;
        let expected = serialize::serialize_group(&pack(&expected, &input.context()));
        for skip in ["disabled", "out-of-date", "orphaned"] {
            for (runs, weight) in [(0, 0), (7, 11)] {
                let mut plan = base.clone();
                plan.pool[0].runs = runs;
                plan.pool[0].weight = weight;
                match skip {
                    "disabled" => plan.pool[0].enabled = false,
                    "out-of-date" => plan.library[1].valid.to = (1900, 1, 1),
                    _ => {
                        plan.pool[0].target = TargetRef::Objective {
                            side: AirSide::Nato,
                            xz: (0.0, 0.0),
                        }
                    }
                }
                let issues = super::super::validate(&plan, &input.context());
                assert_no_errors(&issues);
                assert_eq!(issues.len(), usize::from(skip != "disabled"));
                assert_eq!(
                    serialize::serialize_group(&pack(&plan, &input.context())),
                    expected
                );
            }
        }
    }

    #[test]
    fn validate_zone_entries_ignore_weight() {
        let (mut plan, input) = export_test_plan();
        plan.pool
            .iter_mut()
            .for_each(|entry| entry.mode = TriggerMode::Zone);
        let expected = serialize::serialize_group(&pack(&plan, &input.context()));
        for weight in [0, 11, u8::MAX] {
            plan.pool.iter_mut().for_each(|entry| entry.weight = weight);
            assert!(super::super::validate(&plan, &input.context()).is_empty());
            assert_eq!(
                serialize::serialize_group(&pack(&plan, &input.context())),
                expected
            );
        }
    }

    #[test]
    fn validate_warns_above_totals_limits() {
        let (mut plan, input) = export_test_plan();
        plan.pool = vec![plan.pool[0].clone(); 30];
        assert!(plan.pool.iter().all(|entry| entry.runs == 3));
        let counts = super::super::totals(&plan, &input.context());
        let issues = super::super::validate(&plan, &input.context());
        assert_no_errors(&issues);
        let totals: Vec<_> = issues
            .iter()
            .filter(|i| i.text.starts_with("Air Tasking total "))
            .collect();
        let mut exceeded = 0;
        for (name, total, limit) in [
            (
                "aircraft",
                counts.aircraft.iter().sum::<u32>(),
                plan.limits.aircraft,
            ),
            ("MCUs", counts.mcus.iter().sum::<u32>(), plan.limits.mcus),
            (
                "check zones",
                counts.check_zones.iter().sum::<u32>(),
                plan.limits.check_zones,
            ),
        ] {
            assert!(total > limit, "{name}: {total} <= {limit}");
            exceeded += 1;
            assert_eq!(
                totals
                    .iter()
                    .filter(|i| i.text
                        == format!(
                            "Air Tasking total {name} ({total}) exceeds the limit ({limit})"
                        ))
                    .count(),
                1
            );
        }
        assert_eq!(totals.len(), exceeded);
        // Equality is allowed; these are warnings only when the total exceeds it.
        plan.limits.aircraft = counts.aircraft.iter().sum();
        plan.limits.mcus = counts.mcus.iter().sum();
        plan.limits.check_zones = counts.check_zones.iter().sum();
        assert!(
            !super::super::validate(&plan, &input.context())
                .iter()
                .any(|i| i.text.starts_with("Air Tasking total "))
        );
    }

    #[test]
    fn validate_warns_on_locale_id_conflict() {
        let temp = TempTemplates::new("locale");
        let source = parser::parse_group_file(BUILTINS[1].1).unwrap();
        let paths = [
            temp.write("first.Group", &source),
            temp.write("second.Group", &source),
        ];
        let (mut plan, input) = export_test_plan();
        plan.pool.truncate(1);
        for (i, path) in paths.iter().enumerate() {
            let sortie = load_user_sortie(path).unwrap();
            let mut entry = plan.pool[0].clone();
            entry.sortie_id = sortie.id.clone();
            plan.library.push(sortie);
            plan.pool.push(entry);
            let table = locale::parse_locale(&format!("42:text {i}\n99:shared\n"));
            locale::write_sidecars(
                path,
                &HashMap::from([("eng".into(), table.clone()), ("rus".into(), table)]),
            )
            .unwrap();
        }
        // A repeated use and a second conflicting language do not duplicate D41.
        plan.pool.push(plan.pool[1].clone());
        let issues = super::super::validate(&plan, &input.context());
        assert_no_errors(&issues);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].text.contains("locale id 42"));
        for path in &paths {
            assert!(issues[0].text.contains(path.to_str().unwrap()));
        }
        plan.pool[2].enabled = false;
        assert!(super::super::validate(&plan, &input.context()).is_empty());
        plan.pool[2].enabled = true;
        for orphaned in [true, false] {
            let mut skipped = plan.clone();
            if orphaned {
                skipped.pool[2].target = TargetRef::Objective {
                    side: AirSide::Nato,
                    xz: (0.0, 0.0),
                };
            } else {
                skipped
                    .library
                    .iter_mut()
                    .find(|sortie| sortie.id == plan.pool[2].sortie_id)
                    .unwrap()
                    .valid
                    .to = (1900, 1, 1);
            }
            let issues = super::super::validate(&skipped, &input.context());
            assert_no_errors(&issues);
            assert_eq!(issues.len(), 1, "{issues:?}");
            assert!(issues[0].text.contains("skipped"));
            assert!(!issues[0].text.contains("locale id"));
        }
        for ext in ["eng", "rus"] {
            std::fs::remove_file(paths[1].with_extension(ext)).unwrap();
        }
        assert!(super::super::validate(&plan, &input.context()).is_empty());
    }

    #[test]
    fn validate_and_build_agree() {
        let (base, input) = export_test_plan();
        let mut plans = vec![base.clone(), AirPlan::default()];
        let mutations: &[fn(&mut AirPlan)] = &[
            |p| p.pool[0].sortie_id = "missing".into(),
            |p| {
                p.pool[0].target = TargetRef::Objective {
                    side: AirSide::Dprk,
                    xz: (220_000.0, 260_000.0),
                }
            },
            |p| p.pool[0].runs = 0,
            |p| p.pool[0].runs = 7,
            |p| p.pool[0].weight = 0,
            |p| p.pool[1].weight = 11,
            |p| p.budget.per_side = [0, 7],
            |p| p.strategies.budget = super::super::BudgetImpl::SingleSlot,
            |p| p.threshold.players_n = 0,
            |p| p.strategies.threshold = super::super::ThresholdImpl::Counted,
            |p| p.threshold.radius_m = f64::NAN,
            |p| p.threshold.inner_ratio = 1.1,
            |p| p.threshold.dwell_s = Some(0.0),
            |p| p.threshold.hold_s = Some(f64::INFINITY),
            |p| p.schedule[1].period_min = 0.0,
            |p| p.schedule[1].period_min = f64::NAN,
            |p| p.schedule[0].jitter_steps = 0,
            |p| p.schedule[1].quiet_pct = 101,
            |p| p.schedule[1].first_slot_min = f64::MAX,
            |p| p.schedule[1].arrive_min = f64::NAN,
            |p| p.zone.chance_pct = 0,
            |p| p.zone.cooldown_min = -1.0,
            |p| p.zone.spawn_dist_km = -1.0,
            |p| p.threshold.hold_s = Some(1.0),
            |p| p.pool[0].heading_override = Some(f64::INFINITY),
            |p| p.stop.extension_min = -1.0,
            |p| p.stop.margin_min = f64::NAN,
            |p| p.library[1].source = SortieSource::BuiltIn("missing.Group"),
            |p| p.library[1].source = SortieSource::File(PathBuf::from("missing-p14-sortie.Group")),
            |p| p.pool.iter_mut().for_each(|e| e.enabled = false),
        ];
        for mutate in mutations {
            let mut plan = base.clone();
            mutate(&mut plan);
            plans.push(plan);
        }
        for (i, plan) in plans.iter().enumerate() {
            let issues = super::super::validate(plan, &input.context());
            assert!(
                issues
                    .iter()
                    .all(|issue| issue.text.starts_with("Air Tasking "))
            );
            assert!(issues.iter().filter(|issue| issue.rail_only).count() <= 1);
            let built = super::super::build_air_packs(plan, &input.context());
            if let Some(error) = blocking(&issues) {
                assert_eq!(built.unwrap_err(), error.text, "case {i}");
                assert_eq!(
                    super::super::totals(plan, &input.context()),
                    AirTotals::default()
                );
            } else {
                assert!(built.is_ok(), "case {i}: {built:?}");
            }
            // Invalid schedule edits must also leave the geometry preview finite.
            let _ = super::super::preview(plan, &input.context());
        }
        // Missing or incompatible user files are reported by the actual builder.
        let temp = TempTemplates::new("broken");
        let path = temp.write("broken.Group", &Il2Entity::new("Group"));
        let mut plan = base;
        plan.library[1].source = SortieSource::File(path);
        let issues = super::super::validate(&plan, &input.context());
        assert!(blocking(&issues).is_some());
        assert_eq!(
            super::super::build_air_packs(&plan, &input.context()).unwrap_err(),
            blocking(&issues).unwrap().text
        );
    }

    fn recount(root: &Il2Entity) -> AirTotals {
        let mut result = AirTotals::default();
        for side in SIDES {
            let i = side.idx();
            let sorties = named(root, &format!("Air Tasking {} Sorties", side.label()));
            result.copies[i] = sorties
                .children
                .iter()
                .filter(|n| n.block_type == "Group")
                .count() as u32;
            let logic = named(root, &format!("Air Tasking {} Logic", side.label()));
            let flat: Vec<_> = nodes(sorties).into_iter().chain(nodes(logic)).collect();
            result.aircraft[i] = flat.iter().filter(|n| n.block_type == "Plane").count() as u32;
            result.mcus[i] = flat
                .iter()
                .filter(|n| n.block_type.starts_with("MCU_"))
                .count() as u32;
            result.check_zones[i] = flat
                .iter()
                .filter(|n| n.block_type == "MCU_CheckZone")
                .count() as u32;
        }
        result
    }

    /// The UI's one call: the same answers as validate, totals and preview,
    /// for one evaluation instead of three.
    #[test]
    fn check_matches_validate_totals_and_preview_in_one_evaluation() {
        for factory in [export_test_plan as fn() -> _, acceptance_test_plan] {
            let (plan, input) = factory();
            let ctx = input.context();
            let before = super::super::evaluations();
            let check = super::super::check(&plan, &ctx);
            assert_eq!(super::super::evaluations(), before + 1);
            assert_eq!(check.issues, super::super::validate(&plan, &ctx));
            assert_eq!(check.totals, super::super::totals(&plan, &ctx));
            assert_eq!(check.preview, super::super::preview(&plan, &ctx));
        }
        let (mut plan, input) = export_test_plan();
        plan.budget.per_side = [0, 0];
        let check = super::super::check(&plan, &input.context());
        assert!(check.issues.iter().any(|issue| issue.blocks_build()));
        assert_eq!(check.totals, AirTotals::default(), "a plan that cannot build counts nothing");
    }

    #[test]
    fn totals_match_the_generated_pack() {
        for (label, factory, expected) in [
            (
                "export",
                export_test_plan as fn() -> _,
                AirTotals {
                    copies: [3, 3],
                    aircraft: [24, 12],
                    mcus: [240, 199],
                    check_zones: [7, 7],
                },
            ),
            (
                "acceptance",
                acceptance_test_plan,
                AirTotals {
                    copies: [6, 12],
                    aircraft: [36, 48],
                    mcus: [418, 674],
                    check_zones: [14, 26],
                },
            ),
        ] {
            let (plan, input) = factory();
            let root = pack(&plan, &input.context());
            let counts =
                recount(&parser::parse_group_file(&serialize::serialize_group(&root)).unwrap());
            assert_eq!(super::super::totals(&plan, &input.context()), counts);
            assert_eq!(counts, expected, "{label} D36 totals");
            println!("{label}: {counts:?}");
        }
    }

    #[test]
    fn preview_counts_follow_the_plan() {
        for (factory, expected) in [
            (export_test_plan as fn() -> _, (2, 4)),
            (acceptance_test_plan, (6, 8)),
        ] {
            let (mut plan, input) = factory();
            let preview = super::super::preview(&plan, &input.context());
            assert_eq!(
                preview.counts(),
                AirDrawCounts {
                    lines: expected.0,
                    rings: expected.1,
                    spawns: expected.0,
                    rtbs: expected.0
                }
            );
            for side in SIDES {
                assert_eq!(
                    preview.slot_times[side.idx()],
                    (0..9)
                        .map(|k| SlotMark {
                            t_s: 600.0 + f64::from(k) * 1200.0,
                            jitter_span_s: 360.0,
                            quiet_pct: 20
                        })
                        .collect::<Vec<_>>()
                );
                let rings: Vec<_> = preview
                    .rings
                    .iter()
                    .filter(|ring| ring.side == side)
                    .collect();
                assert!(
                    rings
                        .chunks_exact(2)
                        .all(|pair| pair[0].centre == pair[1].centre
                            && pair[0].radius_m == 12000.0
                            && !pair[0].inner
                            && pair[1].radius_m == 6000.0
                            && pair[1].inner)
                );
            }
            plan.schedule[0].enabled = false;
            assert!(super::super::preview(&plan, &input.context()).slot_times[0].is_empty());
            plan.pool
                .iter_mut()
                .for_each(|entry| entry.mode = TriggerMode::Zone);
            assert!(
                super::super::preview(&plan, &input.context())
                    .slot_times
                    .iter()
                    .all(Vec::is_empty)
            );
            plan.pool.iter_mut().for_each(|entry| entry.enabled = false);
            assert_eq!(
                super::super::preview(&plan, &input.context()),
                AirPreview::default()
            );
        }
    }

    #[test]
    fn preview_spawn_matches_the_built_copy() {
        for factory in [export_test_plan, acceptance_test_plan] {
            let (plan, input) = factory();
            let root = pack(&plan, &input.context());
            let preview = super::super::preview(&plan, &input.context());
            let mut ordinal = [0; 2];
            for (i, entry) in plan.pool.iter().enumerate() {
                let sortie = plan
                    .library
                    .iter()
                    .find(|s| s.id == entry.sortie_id)
                    .unwrap();
                let source_text = BUILTINS
                    .iter()
                    .find(|(file, _)| sortie.source == SortieSource::BuiltIn(file))
                    .unwrap()
                    .1;
                let source = parser::parse_group_file(source_text).unwrap();
                let shape = inspect_sortie(&source).unwrap();
                let lead = by_id(&source, shape.plane_objects[&shape.navigation_lead]);
                let index = sortie.side.idx();
                let copies = named(
                    &root,
                    &format!("Air Tasking {} Sorties", sortie.side.label()),
                );
                let point = preview
                    .spawns
                    .iter()
                    .find(|point| point.entry == i)
                    .unwrap();
                for copy in
                    &copies.children[ordinal[index]..ordinal[index] + usize::from(entry.runs)]
                {
                    let placed = nodes(copy)
                        .into_iter()
                        .find(|n| n.block_type == "Plane" && n.name() == lead.name())
                        .unwrap();
                    let at = placed.pos_xz().unwrap();
                    assert!(
                        (at.0 - point.at.0).hypot(at.1 - point.at.1) <= 1.0,
                        "Entry {}: {at:?} != {:?}",
                        i + 1,
                        point.at
                    );
                }
                ordinal[index] += usize::from(entry.runs);
            }
        }
    }

    #[test]
    fn preview_and_assembly_use_the_library_speed() {
        let (mut plan, input) = export_test_plan();
        plan.library[1].wp_speed_kmh *= 0.5;
        let preview = super::super::preview(&plan, &input.context());
        let expected = preview
            .spawns
            .iter()
            .find(|point| point.entry == 0)
            .unwrap();
        let mut logic = SideLogic::default();
        let copies =
            assemble_side(AirSide::Nato, &plan, &input.context(), &mut logic, &mut 1).unwrap();
        let distance = (expected.at.0 - expected.target.0).hypot(expected.at.1 - expected.target.1);
        assert!((distance - plan.library[1].wp_speed_kmh / 3.6 * 300.0).abs() < 1e-6);
        for copy in &copies.entries[0].copies {
            assert_eq!(copy.spawn, expected.at);
        }
    }

    #[test]
    fn preview_keeps_marks_for_an_invalid_schedule() {
        let (mut plan, input) = export_test_plan();
        plan.schedule[AirSide::Nato.idx()].period_min = 1.0;
        let issues = super::super::validate(&plan, &input.context());
        assert!(
            issues
                .iter()
                .any(|issue| issue.severity == IssueSeverity::Error
                    && issue.text.contains("NATO clock: period"))
        );
        let preview = super::super::preview(&plan, &input.context());
        let marks = &preview.slot_times[AirSide::Nato.idx()];
        assert_eq!(marks.len(), 171);
        assert_eq!(marks.first().unwrap().t_s, 600.0);
        assert_eq!(marks.last().unwrap().t_s, 10800.0);
    }

    #[test]
    fn arrow_side_matches_generate_front() {
        for factory in [export_test_plan, acceptance_test_plan] {
            let (_, input) = factory();
            let extended = mapclip::extend_front_to_aabb_ex(
                &raw_front(&input.opts),
                input.opts.aabb,
                true,
                input.opts.custom_front.is_none(),
            );
            let dense_base = frontlines::densify(&extended, 4000.0);
            let original = crate::baseline_tests::air_export_options().user_attacks[0];
            for (i, &(tail, tip)) in input
                .opts
                .user_attacks
                .iter()
                .chain(std::iter::once(&original))
                .enumerate()
            {
                assert_eq!(
                    mapclip::point_north_of_front(&input.front, tail.0, tail.1),
                    mapclip::point_north_of_front(&dense_base, tail.0, tail.1),
                    "arrow {} {tail:?} -> {tip:?} disagrees between preview and Generate",
                    i + 1
                );
            }
        }
    }

    /// The P14 names a trace build adds on the Air Tasking and Map tabs:
    /// only nodes inside Air Tasking, and the roles the A1 build traced.
    #[test]
    fn trace_role_indexes_name_p14_roles_inside_air_tasking() {
        let (plan, mut input) = acceptance_test_plan();
        input.opts.air_packs = super::super::build_air_packs(&plan, &input.context()).unwrap();
        let mission = frontlines::generate_front(&input.opts).unwrap();
        let indexes = super::super::trace_role_indexes(&mission.root);
        let mut inside = BTreeSet::new();
        named(&mission.root, "Air Tasking").for_each(&mut |n| {
            inside.extend(n.index);
        });
        let mut names = BTreeSet::new();
        mission.root.for_each(&mut |n| {
            if n.index.is_some_and(|i| indexes.contains(&i)) {
                names.insert(n.name().unwrap_or("").to_string());
            }
        });
        assert!(indexes.iter().all(|i| inside.contains(i)), "only Air Tasking nodes");
        assert!(indexes.windows(2).all(|w| w[0] < w[1]), "sorted, no repeats");
        for name in ["CLK D SLOT OPEN", "SD01 START", "SD01 DONE HUB", "ZD01 FIRE", "THD01 INNER"] {
            assert!(names.contains(name), "{name} missing from {names:?}");
        }
        assert!(!names.iter().any(|n| n.ends_with("JITTER 1")), "jitter timers are not roles");
    }

    fn acceptance_mission() -> (frontlines::FrontPack, trace::TraceMap) {
        let (plan, input) = acceptance_test_plan();
        traced_acceptance_mission(plan, input)
    }

    /// The Map's Generate with the plan's Air Tasking packs, as a trace build.
    fn traced_acceptance_mission(
        plan: AirPlan,
        mut input: TestContext,
    ) -> (frontlines::FrontPack, trace::TraceMap) {
        assert!(!plan.debug_subtitles);
        input.opts.air_packs = super::super::build_air_packs(&plan, &input.context()).unwrap();
        let mut mission = frontlines::generate_front(&input.opts).unwrap();
        let decisions = trace::TraceSelect::decision_points();
        let mut indexes: BTreeSet<_> = super::super::trace_role_indexes(&mission.root)
            .into_iter()
            .collect();
        // Scope decision points to Air Tasking as well: existing Map packs
        // must not acquire breadcrumbs merely because they contain a counter.
        // (The UI's trace build traces decision points across the whole file.)
        named(&mission.root, "Air Tasking").for_each(&mut |node| {
            let decision = decisions.block_types.contains(&node.block_type)
                || (decisions.random_timers
                    && node.block_type == "MCU_Timer"
                    && number(node, "Random").is_ok_and(|pct| pct < 100.0));
            if decision {
                indexes.insert(node.index.expect("trace source has an index"));
            }
        });
        let selection = trace::TraceSelect {
            indexes: indexes.into_iter().collect(),
            ..trace::TraceSelect::default()
        };
        let mut next = mission.root.max_index() + 1;
        let mut map = trace::TraceMap::default();
        trace::instrument(
            &mut mission.root,
            &selection,
            &mut next,
            trace::TraceCarrier::default(),
            &mut map,
        );
        (mission, map)
    }

    fn acceptance_locales(mission: &frontlines::FrontPack) -> HashMap<String, locale::LocaleTable> {
        // The acceptance map retains the baseline's ship, ground and army
        // packs. Merge their text before overlaying this map's generated labels,
        // as Generate does; the six built-in air sorties need no sidecars.
        let sources: Vec<_> = [
            "DPRK Ships.Group",
            "DPRK Tank Platoon.Group",
            "DPRK ML20 Arty.Group",
        ]
        .into_iter()
        .map(|file| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("TemplateExamples/GroundUnits/DropIns")
                .join(file)
        })
        .collect();
        let mut tables = locale::merge_template_sidecars(&sources);
        for ext in locale::LANG_EXTS {
            tables
                .entry((*ext).into())
                .or_default()
                .overlay(mission.locale.clone());
        }
        tables
    }

    #[test]
    fn acceptance_mission_has_zone_entries_and_trace() {
        let (plan, input) = acceptance_test_plan();
        assert_eq!(plan.seed, 7);
        assert!(!plan.debug_subtitles);
        assert_eq!((input.east.len(), input.nato.len()), (2, 2));
        let terrain = crate::watermap::TerrainMap::builtin().unwrap();
        for &(x, z) in input.east.iter().chain(&input.nato) {
            assert!(
                terrain.is_land_xz(x, z),
                "acceptance objective {x}, {z} is not on land"
            );
        }
        assert!(
            plan.library
                .iter()
                .all(|sortie| { sortie.valid.from <= input.date && input.date <= sortie.valid.to })
        );
        let preview = super::super::preview(&plan, &input.context());
        assert_eq!(preview.spawns.len(), plan.pool.len());
        for spawn in &preview.spawns {
            assert_eq!(
                mapclip::point_north_of_front(&input.front, spawn.at.0, spawn.at.1),
                spawn.side == AirSide::Dprk,
                "{} Entry {} spawn {:?} is on the wrong side of the front",
                spawn.side.label(),
                spawn.entry + 1,
                spawn.at
            );
        }
        let air_alert = preview.lines.iter().find(|line| line.entry == 0).unwrap();
        assert!(
            segment_distance(input.east[0], air_alert.from, air_alert.to)
                <= timing::derived_timing(&plan.threshold).inner_m
        );
        let issues = super::super::validate(&plan, &input.context());
        assert_no_errors(&issues);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(d30(&issues).len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);

        let (mission, map) = acceptance_mission();
        let root = &mission.root;
        let air = named(root, "Air Tasking");
        assert_eq!(air.count_block_type("MCU_TR_Subtitle"), 0);
        let mut expected = BTreeSet::new();
        // Enumerate the pinned plan independently of the writer's name selector.
        for (side, clock_entries, zone_entries, copies) in [
            (AirSide::Dprk, vec![2], vec![1, 2], 6),
            (AirSide::Nato, vec![1, 2, 4], vec![3, 4], 12),
        ] {
            let letter = side.letter();
            let side_entries: Vec<_> = plan
                .pool
                .iter()
                .filter(|entry| {
                    plan.library
                        .iter()
                        .find(|sortie| sortie.id == entry.sortie_id)
                        .unwrap()
                        .side
                        == side
                })
                .collect();
            for mode in [TriggerMode::Zone, TriggerMode::Both] {
                assert!(
                    side_entries.iter().enumerate().any(|(i, entry)| {
                        entry.mode == mode && zone_entries.contains(&(i + 1))
                    }),
                    "{} is missing a {mode:?} zone cell",
                    side.label()
                );
            }
            let logic = named(air, &format!("Air Tasking {} Logic", side.label()));
            expected.insert(
                named(logic, &format!("CLK {letter} SLOT OPEN"))
                    .index
                    .unwrap(),
            );
            for entry in clock_entries {
                expected.insert(
                    named(logic, &format!("E{letter}{entry:02} GO"))
                        .index
                        .unwrap(),
                );
            }
            for entry in zone_entries {
                for role in [
                    "CHANCE",
                    "MISS WAIT",
                    "FIRE",
                    "DENY WAIT",
                    "COOLDOWN",
                    "ARM",
                ] {
                    expected.insert(
                        named(logic, &format!("Z{letter}{entry:02} {role}"))
                            .index
                            .unwrap(),
                    );
                }
            }
            for area in 1..=2 {
                for role in ["INNER", "CLOSE DELAY"] {
                    expected.insert(
                        named(logic, &format!("TH{letter}{area:02} {role}"))
                            .index
                            .unwrap(),
                    );
                }
            }
            let sorties = named(air, &format!("Air Tasking {} Sorties", side.label()));
            assert_eq!(sorties.children.len(), copies);
            for (i, copy) in sorties.children.iter().enumerate() {
                for role in [
                    "START",
                    "ON STATION",
                    "EXTEND",
                    "COLD",
                    "LOSSES",
                    "HARD STOP",
                    "DONE HUB",
                    "DELETE DELAY",
                ] {
                    expected.insert(
                        named(copy, &format!("S{letter}{:02} {role}", i + 1))
                            .index
                            .unwrap(),
                    );
                }
                expected.insert(named(copy, "Zone Out").index.unwrap());
            }
        }
        assert_eq!(expected.len(), 200, "every named breadcrumb, on every copy");
        air.for_each(&mut |node| {
            if matches!(
                node.block_type.as_str(),
                "MCU_CheckZone" | "MCU_Counter" | "MCU_TR_MissionBegin" | "MCU_Spawner"
            ) || (node.block_type == "MCU_Timer"
                && number(node, "Random").is_ok_and(|pct| pct < 100.0))
            {
                expected.insert(node.index.unwrap());
            }
        });
        let actual: BTreeSet<_> = map.entries.iter().map(|entry| entry.source_index).collect();
        assert_eq!(
            actual, expected,
            "only Air Tasking decision points and requested breadcrumbs"
        );
        assert_eq!(
            actual.len(),
            map.entries.len(),
            "each source is instrumented once"
        );
        for entry in &map.entries {
            let source = by_id(air, entry.source_index);
            assert_eq!(source.name(), Some(entry.source_name.as_str()));
            assert_eq!(source.block_type, entry.source_type);
            assert!(source.targets.contains(&entry.breadcrumb_index));
            assert_eq!(entry.carrier, trace::TraceCarrier::default());
            assert_eq!(entry.event_type, None);
            let breadcrumb = by_id(root, entry.breadcrumb_index);
            assert_eq!(breadcrumb.block_type, "MCU_TR_MissionObjective");
            assert_eq!(breadcrumb.property("Success"), Some("0"));
        }
        assert_links_resolve(root);
        assert_unique(root);
        assert_links_resolve(&parser::parse_group_file(&serialize::serialize_group(root)).unwrap());
        let tables = acceptance_locales(&mission);
        root.for_each(&mut |node| {
            for key in ["LCName", "LCDesc", "LCText"] {
                if let Some(id) = node.property(key) {
                    let id: i32 = id.parse().unwrap();
                    for ext in locale::LANG_EXTS {
                        assert!(
                            id == 0 || tables[*ext].get(id).is_some(),
                            "missing {ext} locale {id}"
                        );
                    }
                }
            }
        });
    }

    #[test]
    #[ignore = "writes target/p14/P14_Acceptance_1.Group, locale and trace sidecars"]
    fn write_p14_acceptance_mission() {
        let (mission, map) = acceptance_mission();
        let written =
            write_acceptance_files(Path::new("target/p14"), "P14_Acceptance_1", &mission, &map);
        report_written(&written, &map);
    }

    /// Writes `<dir>/<stem>.Group`, its locale sidecars and its `.trace.json`,
    /// and returns the paths in that order.
    fn write_acceptance_files(
        dir: &Path,
        stem: &str,
        mission: &frontlines::FrontPack,
        map: &trace::TraceMap,
    ) -> Vec<PathBuf> {
        std::fs::create_dir_all(dir).expect("create the acceptance output directory");
        let path = dir.join(format!("{stem}.Group"));
        std::fs::write(&path, serialize::serialize_group(&mission.root))
            .expect("write acceptance Group");
        let tables = acceptance_locales(mission);
        locale::write_sidecars(&path, &tables).expect("write acceptance locales");
        let sidecar = path.with_extension("trace.json");
        trace::write_trace_sidecar(&sidecar, map).unwrap_or_else(|error| {
            panic!("{error}; acceptance Group and locale files were still written")
        });
        assert_eq!(trace::read_trace_sidecar(&sidecar).unwrap(), *map);
        std::iter::once(path.clone())
            .chain(locale::LANG_EXTS.iter().map(|ext| path.with_extension(ext)))
            .chain(std::iter::once(sidecar))
            .collect()
    }

    fn report_written(written: &[PathBuf], map: &trace::TraceMap) {
        // Direct output, as in probe.rs, keeps the inventory visible even when
        // libtest captures passing-test stdout without --nocapture.
        use std::io::Write;
        let mut report = std::io::stdout().lock();
        for path in written {
            writeln!(
                report,
                "WROTE {} ({} bytes)",
                path.display(),
                std::fs::metadata(path).unwrap().len()
            )
            .unwrap();
        }
        writeln!(report, "TRACE {} Air Tasking sources", map.entries.len()).unwrap();
    }
}
