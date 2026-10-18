//! Target choices, resolution and pool edits shared by the rail tab and Map dock.

use super::{AirContext, AirPlan, AirSide, PoolEntry, SortieSource, TargetRef};
use crate::mapclip::point_north_of_front;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Objective,
    Arrow,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetChoice {
    pub kind: TargetKind,
    pub number: usize,
    pub side: AirSide,
    pub target: TargetRef,
}

fn arrow_side(ctx: &AirContext, tail: (f64, f64)) -> AirSide {
    if point_north_of_front(ctx.front, tail.0, tail.1) {
        AirSide::Dprk
    } else {
        AirSide::Nato
    }
}

/// Objectives retain their side-local numbering; arrows retain their Map numbering.
pub fn target_choices(ctx: &AirContext, side: AirSide) -> Vec<TargetChoice> {
    let objectives = match side {
        AirSide::Dprk => ctx.east_objectives,
        AirSide::Nato => ctx.nato_objectives,
    };
    objectives
        .iter()
        .enumerate()
        .map(|(i, &xz)| TargetChoice {
            kind: TargetKind::Objective,
            number: i + 1,
            side,
            target: TargetRef::Objective { side, xz },
        })
        .chain(
            ctx.arrows
                .iter()
                .enumerate()
                .filter_map(|(i, &(tail, tip))| {
                    (arrow_side(ctx, tail) == side).then_some(TargetChoice {
                        kind: TargetKind::Arrow,
                        number: i + 1,
                        side,
                        target: TargetRef::Arrow { tail, tip },
                    })
                }),
        )
        .collect()
}

/// Resolve to the nearest current objective or arrow tip within 50 metres, inclusive.
pub fn resolve_target(target: &TargetRef, ctx: &AirContext) -> Option<TargetChoice> {
    let (kind, side, at) = match *target {
        TargetRef::Objective { side, xz } => (TargetKind::Objective, Some(side), xz),
        TargetRef::Arrow { tip, .. } => (TargetKind::Arrow, None, tip),
    };
    [AirSide::Dprk, AirSide::Nato]
        .into_iter()
        .filter(|s| side.is_none_or(|wanted| *s == wanted))
        .flat_map(|s| target_choices(ctx, s))
        .filter(|choice| choice.kind == kind)
        .filter_map(|choice| {
            let current = match choice.target {
                TargetRef::Objective { xz, .. } => xz,
                TargetRef::Arrow { tip, .. } => tip,
            };
            let distance_sq = (current.0 - at.0).powi(2) + (current.1 - at.1).powi(2);
            (distance_sq <= 50.0_f64.powi(2)).then_some((distance_sq, choice))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, choice)| choice)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryStatus {
    Ok,
    Orphaned,
    SideChanged,
    OutOfDate,
    MissingSortie,
}

/// Status is independent of the entry's on/off switch. Missing inputs take precedence.
pub fn entry_status(plan: &AirPlan, index: usize, ctx: &AirContext) -> EntryStatus {
    let Some(entry) = plan.pool.get(index) else {
        return EntryStatus::MissingSortie;
    };
    let Some(sortie) = plan.library.iter().find(|s| s.id == entry.sortie_id) else {
        return EntryStatus::MissingSortie;
    };
    let Some(target) = resolve_target(&entry.target, ctx) else {
        return EntryStatus::Orphaned;
    };
    if target.side != sortie.side {
        EntryStatus::SideChanged
    } else if ctx.date < sortie.valid.from || ctx.date > sortie.valid.to {
        EntryStatus::OutOfDate
    } else {
        EntryStatus::Ok
    }
}

/// The first entry opts the plan into Map output (D21).
pub fn add_entry(plan: &mut AirPlan, entry: PoolEntry) {
    if plan.pool.is_empty() {
        plan.include_in_base_map = true;
    }
    plan.pool.push(entry);
}

/// Add each date-valid sortie/target pair once, including existing disabled entries.
pub fn auto_fill(plan: &mut AirPlan, ctx: &AirContext, side: AirSide) -> usize {
    let sorties: Vec<_> = plan
        .library
        .iter()
        .filter(|s| s.side == side && s.valid.from <= ctx.date && ctx.date <= s.valid.to)
        .map(|s| s.id.clone())
        .collect();
    let choices = target_choices(ctx, side);
    let before = plan.pool.len();
    for id in sorties {
        for choice in &choices {
            let exists = plan.pool.iter().any(|entry| {
                entry.sortie_id == id && resolve_target(&entry.target, ctx) == Some(*choice)
            });
            if !exists {
                add_entry(plan, PoolEntry::new(id.clone(), choice.target));
            }
        }
    }
    plan.pool.len() - before
}

/// Removing a user template also removes every entry that refers to it.
pub fn remove_library_item(plan: &mut AirPlan, index: usize) -> Result<usize, String> {
    let sortie = plan
        .library
        .get(index)
        .ok_or("Select a library sortie to remove.")?;
    if matches!(sortie.source, SortieSource::BuiltIn(_)) {
        return Err("Built-in sorties stay in the library.".into());
    }
    let id = sortie.id.clone();
    let before = plan.pool.len();
    plan.pool.retain(|entry| entry.sortie_id != id);
    plan.library.remove(index);
    Ok(before - plan.pool.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airtask::{AirRole, AirSortie, TriggerMode};
    use crate::{in_theatre, mapclip::WorldAabb};

    fn context() -> AirContext<'static> {
        AirContext {
            east_objectives: &[(1100.0, 100.0), (1200.0, 200.0)],
            nato_objectives: &[(900.0, 100.0)],
            arrows: &[
                ((1100.0, 300.0), (800.0, 300.0)),
                ((900.0, 400.0), (1200.0, 400.0)),
            ],
            front: &[(1000.0, 0.0), (1000.0, 1000.0)],
            aabb: WorldAabb::full_map(),
            date: (1950, 7, 1),
        }
    }

    fn la11() -> AirSortie {
        AirSortie {
            id: "file:La11_Night_Intruder.Group".into(),
            name: "La11_Night_Intruder".into(),
            source: SortieSource::File("La11_Night_Intruder.Group".into()),
            side: AirSide::Dprk,
            models: vec!["la11".into()],
            planes: 2,
            lead_roles: vec![vec![AirRole::GroundAttack]],
            wp_speed_kmh: 500.0,
            wp_alt_m: 1500.0,
            attack_time_s: 600.0,
            valid: in_theatre::for_model("la11").unwrap().valid,
            category: "Strike".into(),
            warnings: vec![],
        }
    }

    #[test]
    fn target_choices_keep_objective_and_arrow_numbers_and_sides() {
        let ctx = context();
        let dprk = target_choices(&ctx, AirSide::Dprk);
        let nato = target_choices(&ctx, AirSide::Nato);
        assert_eq!(
            dprk.iter().map(|c| (c.kind, c.number)).collect::<Vec<_>>(),
            [
                (TargetKind::Objective, 1),
                (TargetKind::Objective, 2),
                (TargetKind::Arrow, 1)
            ]
        );
        assert_eq!(
            nato.iter().map(|c| (c.kind, c.number)).collect::<Vec<_>>(),
            [(TargetKind::Objective, 1), (TargetKind::Arrow, 2)]
        );
        assert!(dprk.iter().all(|c| c.side == AirSide::Dprk));
        assert!(nato.iter().all(|c| c.side == AirSide::Nato));
    }

    #[test]
    fn resolve_objective_uses_nearest_same_side_within_50_metres() {
        let mut ctx = context();
        ctx.east_objectives = &[(1100.0, 100.0), (1140.0, 100.0)];
        let target = |x| TargetRef::Objective {
            side: AirSide::Dprk,
            xz: (x, 100.0),
        };
        assert_eq!(resolve_target(&target(1130.0), &ctx).unwrap().number, 2);
        assert_eq!(resolve_target(&target(1050.0), &ctx).unwrap().number, 1);
        assert!(resolve_target(&target(1049.9), &ctx).is_none());
        assert!(
            resolve_target(&target(900.0), &ctx).is_none(),
            "NATO point is not a DPRK objective"
        );
    }

    #[test]
    fn resolve_arrow_uses_tip_within_50_metres_and_current_tail() {
        let ctx = context();
        let target = |x| TargetRef::Arrow {
            tail: (0.0, 0.0),
            tip: (x, 300.0),
        };
        let choice = resolve_target(&target(850.0), &ctx).unwrap();
        assert_eq!(choice.number, 1);
        assert_eq!(choice.side, AirSide::Dprk);
        assert_eq!(
            choice.target,
            TargetRef::Arrow {
                tail: (1100.0, 300.0),
                tip: (800.0, 300.0)
            }
        );
        assert!(resolve_target(&target(850.1), &ctx).is_none());
    }

    #[test]
    fn entry_status_checks_missing_orphan_side_then_date_even_when_disabled() {
        let mut ctx = context();
        let mut plan = AirPlan {
            library: vec![la11()],
            ..AirPlan::default()
        };
        add_entry(
            &mut plan,
            PoolEntry::new(plan_id(), target_choices(&ctx, AirSide::Dprk)[2].target),
        );
        assert_eq!(entry_status(&plan, 0, &ctx), EntryStatus::OutOfDate);
        plan.pool[0].enabled = false;
        assert_eq!(entry_status(&plan, 0, &ctx), EntryStatus::OutOfDate);
        ctx.front = &[(1200.0, 0.0), (1200.0, 1000.0)];
        assert_eq!(entry_status(&plan, 0, &ctx), EntryStatus::SideChanged);
        ctx.arrows = &[];
        assert_eq!(entry_status(&plan, 0, &ctx), EntryStatus::Orphaned);
        plan.library.clear();
        assert_eq!(entry_status(&plan, 0, &ctx), EntryStatus::MissingSortie);
    }

    fn plan_id() -> String {
        "file:La11_Night_Intruder.Group".into()
    }

    #[test]
    fn entry_status_date_range_is_inclusive() {
        let mut ctx = context();
        let mut plan = AirPlan {
            library: vec![la11()],
            ..AirPlan::default()
        };
        add_entry(
            &mut plan,
            PoolEntry::new(plan_id(), target_choices(&ctx, AirSide::Dprk)[0].target),
        );
        for (date, status) in [
            ((1953, 4, 14), EntryStatus::OutOfDate),
            ((1953, 4, 15), EntryStatus::Ok),
            ((1953, 7, 27), EntryStatus::Ok),
            ((1953, 7, 28), EntryStatus::OutOfDate),
        ] {
            ctx.date = date;
            assert_eq!(entry_status(&plan, 0, &ctx), status, "{date:?}");
        }
    }

    #[test]
    fn add_entry_ticks_include_only_when_pool_was_empty() {
        let mut plan = AirPlan::default();
        let entry = PoolEntry::new(
            plan_id(),
            target_choices(&context(), AirSide::Dprk)[0].target,
        );
        add_entry(&mut plan, entry.clone());
        assert!(plan.include_in_base_map);
        plan.include_in_base_map = false;
        add_entry(&mut plan, entry.clone());
        assert!(!plan.include_in_base_map);
        assert_eq!(plan.pool, [entry.clone(), entry]);
    }

    #[test]
    fn auto_fill_counts_both_sides_filters_dates_and_is_idempotent() {
        let ctx = context();
        let mut plan = AirPlan {
            library: super::super::builtin_library(),
            ..AirPlan::default()
        };
        plan.library.push(la11()); // Invalid on 1 Jul 1950: must not add its three targets.
        assert_eq!(auto_fill(&mut plan, &ctx, AirSide::Dprk), 2 * 3);
        assert_eq!(auto_fill(&mut plan, &ctx, AirSide::Nato), 4 * 2);
        assert!(plan.include_in_base_map);
        assert_eq!(plan.pool.len(), 14);
        assert!(
            plan.pool
                .iter()
                .all(|e| e.weight == 5 && e.mode == TriggerMode::Clock && e.runs == 3)
        );
        plan.pool[0].enabled = false;
        let TargetRef::Objective { xz, .. } = &mut plan.pool[0].target else {
            panic!()
        };
        xz.0 += 25.0; // Still resolves to objective 1: do not add it again.
        assert_eq!(auto_fill(&mut plan, &ctx, AirSide::Dprk), 0);
        assert_eq!(auto_fill(&mut plan, &ctx, AirSide::Nato), 0);
        let empty = AirContext {
            east_objectives: &[],
            nato_objectives: &[],
            arrows: &[],
            ..ctx
        };
        assert_eq!(auto_fill(&mut plan, &empty, AirSide::Dprk), 0);
    }

    #[test]
    fn remove_library_item_refuses_builtins_without_changes() {
        let mut plan = AirPlan {
            library: super::super::builtin_library(),
            ..AirPlan::default()
        };
        auto_fill(&mut plan, &context(), AirSide::Nato);
        let before = plan.clone();
        assert_eq!(
            remove_library_item(&mut plan, 0).unwrap_err(),
            "Built-in sorties stay in the library."
        );
        assert_eq!(plan, before);
        assert!(remove_library_item(&mut plan, 99).is_err());
    }

    #[test]
    fn remove_library_item_takes_only_its_entries_including_disabled() {
        let mut plan = AirPlan {
            library: super::super::builtin_library(),
            ..AirPlan::default()
        };
        auto_fill(&mut plan, &context(), AirSide::Nato);
        let keep = plan.pool.clone();
        plan.library.push(la11());
        for choice in target_choices(&context(), AirSide::Dprk) {
            add_entry(&mut plan, PoolEntry::new(plan_id(), choice.target));
        }
        plan.pool.last_mut().unwrap().enabled = false;
        assert_eq!(remove_library_item(&mut plan, 6), Ok(3));
        assert_eq!(plan.pool, keep);
        assert_eq!(plan.library.len(), 6);
    }
}
