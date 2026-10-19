//! Target choices, resolution and pool edits shared by the rail tab and Map dock.

use super::{
    AirContext, AirPlan, AirRole, AirSide, AirSortie, PoolEntry, SortieSource, TargetRef, Ymd,
};
use crate::mapclip::point_north_of_front;

impl AirSide {
    pub const ALL: [AirSide; 2] = [AirSide::Dprk, AirSide::Nato];
}

impl AirSortie {
    pub fn valid_on(&self, date: Ymd) -> bool {
        self.valid.from <= date && date <= self.valid.to
    }

    pub fn is_escorted(&self) -> bool {
        self.lead_roles
            .iter()
            .flatten()
            .any(|role| *role == AirRole::Escort)
    }
}

impl AirPlan {
    pub fn sortie(&self, id: &str) -> Option<&AirSortie> {
        self.library.iter().find(|sortie| sortie.id == id)
    }
}

/// Counts include switched-off entries; unresolved sortie ids have no side.
pub fn pool_counts(plan: &AirPlan) -> [usize; 2] {
    let mut counts = [0; 2];
    for entry in &plan.pool {
        if let Some(sortie) = plan.sortie(&entry.sortie_id) {
            counts[sortie.side.idx()] += 1;
        }
    }
    counts
}

pub fn default_categories() -> Vec<String> {
    super::DEFAULT_CATEGORIES
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn same_category(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// Register a label without changing the spelling carried by a sortie.
fn ensure_category(plan: &mut AirPlan, name: &str) -> String {
    let name = name.trim();
    if !plan.categories.iter().any(|c| same_category(c, name)) {
        plan.categories.push(name.to_owned());
    }
    name.to_owned()
}

pub fn add_library_item(plan: &mut AirPlan, mut sortie: AirSortie) {
    let name = if sortie.category.trim().is_empty() {
        super::default_category(&sortie)
    } else {
        sortie.category.clone()
    };
    sortie.category = ensure_category(plan, &name);
    plan.library.push(sortie);
}

/// Set a nonempty category, adding its label if a caller supplies a new one.
pub fn set_category(plan: &mut AirPlan, sortie_index: usize, name: &str) {
    if sortie_index < plan.library.len() && !name.trim().is_empty() {
        plan.library[sortie_index].category = ensure_category(plan, name);
    }
}

fn checked_category(plan: &AirPlan, name: &str, except: Option<usize>) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a category name.".into());
    }
    if plan
        .categories
        .iter()
        .enumerate()
        .any(|(i, c)| Some(i) != except && same_category(c, name))
    {
        return Err(format!("A category named {name} already exists."));
    }
    Ok(name.to_owned())
}

pub fn add_category(plan: &mut AirPlan, name: &str) -> Result<(), String> {
    let name = checked_category(plan, name, None)?;
    plan.categories.push(name);
    Ok(())
}

pub fn rename_category(plan: &mut AirPlan, old: &str, new: &str) -> Result<(), String> {
    let index = plan
        .categories
        .iter()
        .position(|c| same_category(c, old.trim()))
        .ok_or("Select a category to rename.")?;
    let name = checked_category(plan, new, Some(index))?;
    let old = std::mem::replace(&mut plan.categories[index], name.clone());
    for sortie in &mut plan.library {
        if same_category(&sortie.category, &old) {
            sortie.category = name.clone();
        }
    }
    Ok(())
}

/// Removed labels fall back to file/order defaults, even if that restores a label.
pub fn remove_category(plan: &mut AirPlan, name: &str) -> usize {
    let Some(index) = plan
        .categories
        .iter()
        .position(|c| same_category(c, name.trim()))
    else {
        return 0;
    };
    let old = plan.categories.remove(index);
    let moved: Vec<_> = plan
        .library
        .iter()
        .enumerate()
        .filter(|(_, sortie)| same_category(&sortie.category, &old))
        .map(|(i, sortie)| (i, super::default_category(sortie)))
        .collect();
    for (i, default) in &moved {
        set_category(plan, *i, default);
    }
    moved.len()
}

pub fn category_counts(plan: &AirPlan) -> Vec<(String, usize)> {
    plan.categories
        .iter()
        .map(|category| {
            let count = plan
                .library
                .iter()
                .filter(|s| same_category(&s.category, category))
                .count();
            (category.clone(), count)
        })
        .collect()
}

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
    AirSide::ALL
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
    let Some(sortie) = plan.sortie(&entry.sortie_id) else {
        return EntryStatus::MissingSortie;
    };
    let Some(target) = resolve_target(&entry.target, ctx) else {
        return EntryStatus::Orphaned;
    };
    if target.side != sortie.side {
        EntryStatus::SideChanged
    } else if !sortie.valid_on(ctx.date) {
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
pub fn auto_fill(
    plan: &mut AirPlan,
    ctx: &AirContext,
    side: AirSide,
    category: Option<&str>,
) -> usize {
    let sorties: Vec<_> = plan
        .library
        .iter()
        .filter(|s| {
            s.side == side
                && s.valid_on(ctx.date)
                && category.is_none_or(|category| same_category(&s.category, category))
        })
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
        assert_eq!(auto_fill(&mut plan, &ctx, AirSide::Dprk, None), 2 * 3);
        assert_eq!(auto_fill(&mut plan, &ctx, AirSide::Nato, None), 4 * 2);
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
        assert_eq!(auto_fill(&mut plan, &ctx, AirSide::Dprk, None), 0);
        assert_eq!(auto_fill(&mut plan, &ctx, AirSide::Nato, None), 0);
        let empty = AirContext {
            east_objectives: &[],
            nato_objectives: &[],
            arrows: &[],
            ..ctx
        };
        assert_eq!(auto_fill(&mut plan, &empty, AirSide::Dprk, None), 0);
    }

    #[test]
    fn remove_library_item_refuses_builtins_without_changes() {
        let mut plan = AirPlan {
            library: super::super::builtin_library(),
            ..AirPlan::default()
        };
        auto_fill(&mut plan, &context(), AirSide::Nato, None);
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
        auto_fill(&mut plan, &context(), AirSide::Nato, None);
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

    #[test]
    fn sortie_dates_are_inclusive_and_escort_comes_from_any_lead() {
        let mut sortie = la11();
        assert!(!sortie.valid_on((1953, 4, 14)));
        assert!(sortie.valid_on((1953, 4, 15)));
        assert!(sortie.valid_on((1953, 7, 27)));
        assert!(!sortie.valid_on((1953, 7, 28)));
        assert!(!sortie.is_escorted());
        sortie.lead_roles.push(vec![AirRole::Escort]);
        assert!(sortie.is_escorted());
        sortie.lead_roles = vec![vec![AirRole::AirPatrol, AirRole::GroundAttack]];
        assert!(!sortie.is_escorted());
    }

    #[test]
    fn sortie_lookup_and_pool_counts_include_disabled_but_not_missing() {
        let ctx = context();
        let mut plan = AirPlan {
            library: super::super::builtin_library(),
            ..AirPlan::default()
        };
        assert_eq!(AirSide::ALL.map(AirSide::idx), [0, 1]);
        assert_eq!(plan.sortie(&plan.library[1].id), Some(&plan.library[1]));
        assert!(plan.sortie("missing").is_none());
        assert_eq!(pool_counts(&plan), [0, 0]);
        auto_fill(&mut plan, &ctx, AirSide::Dprk, None);
        auto_fill(&mut plan, &ctx, AirSide::Nato, None);
        plan.pool[0].enabled = false;
        plan.pool.push(PoolEntry::new(
            "missing".into(),
            target_choices(&ctx, AirSide::Dprk)[0].target,
        ));
        assert_eq!(pool_counts(&plan), [6, 8]);
    }

    #[test]
    fn default_categories_have_the_five_labels_in_order() {
        assert_eq!(
            default_categories(),
            [
                "Air alert",
                "Strike",
                "Close support",
                "Armed recon",
                "Interdiction"
            ]
        );
    }

    #[test]
    fn add_library_item_appends_and_registers_its_category_once() {
        let mut plan = AirPlan {
            categories: default_categories(),
            ..AirPlan::default()
        };
        let mut sortie = la11();
        sortie.category = "Night raids".into();
        add_library_item(&mut plan, sortie.clone());
        assert_eq!(plan.library, [sortie.clone()]);
        assert_eq!(
            plan.categories,
            [
                "Air alert",
                "Strike",
                "Close support",
                "Armed recon",
                "Interdiction",
                "Night raids"
            ]
        );
        sortie.category = " night RAIDS ".into();
        add_library_item(&mut plan, sortie);
        assert_eq!(plan.categories.len(), 6);
        assert_eq!(plan.library[1].category, "night RAIDS");
        let mut unlabeled = la11();
        unlabeled.category.clear();
        add_library_item(&mut plan, unlabeled);
        assert_eq!(plan.library[2].category, "Strike");
    }

    #[test]
    fn set_category_changes_only_the_selected_sortie_and_registers_new_labels() {
        let mut plan = AirPlan {
            library: super::super::builtin_library(),
            categories: default_categories(),
            ..AirPlan::default()
        };
        set_category(&mut plan, 1, " interdiction ");
        assert_eq!(plan.library[1].category, "interdiction");
        assert_eq!(plan.library[0].category, "Air alert");
        set_category(&mut plan, 1, "Night raids");
        assert_eq!(plan.categories.last().unwrap(), "Night raids");
        let before = plan.clone();
        set_category(&mut plan, 99, "Unused");
        set_category(&mut plan, 1, "  ");
        assert_eq!(plan, before);
    }

    #[test]
    fn add_category_trims_and_rejects_empty_or_case_duplicate_names() {
        let mut plan = AirPlan {
            categories: default_categories(),
            ..AirPlan::default()
        };
        assert_eq!(add_category(&mut plan, "  Night raids  "), Ok(()));
        assert_eq!(plan.categories.last().unwrap(), "Night raids");
        let before = plan.clone();
        assert_eq!(
            add_category(&mut plan, " \t ").unwrap_err(),
            "Enter a category name."
        );
        assert_eq!(
            add_category(&mut plan, " STRIKE ").unwrap_err(),
            "A category named STRIKE already exists."
        );
        assert_eq!(plan, before);
    }

    #[test]
    fn rename_category_keeps_list_order_and_moves_every_sortie() {
        let mut plan = AirPlan {
            library: super::super::builtin_library(),
            categories: default_categories(),
            ..AirPlan::default()
        };
        assert_eq!(rename_category(&mut plan, "Strike", "  Raids  "), Ok(()));
        assert_eq!(
            plan.categories,
            [
                "Air alert",
                "Raids",
                "Close support",
                "Armed recon",
                "Interdiction"
            ]
        );
        assert_eq!(
            plan.library
                .iter()
                .map(|s| s.category.as_str())
                .collect::<Vec<_>>(),
            [
                "Air alert",
                "Raids",
                "Close support",
                "Armed recon",
                "Raids",
                "Raids"
            ]
        );
        assert_eq!(rename_category(&mut plan, "Raids", "RAIDS"), Ok(()));
        assert_eq!(plan.library[5].category, "RAIDS");
        let before = plan.clone();
        assert_eq!(
            rename_category(&mut plan, "RAIDS", " ").unwrap_err(),
            "Enter a category name."
        );
        assert_eq!(
            rename_category(&mut plan, "RAIDS", "air ALERT").unwrap_err(),
            "A category named air ALERT already exists."
        );
        assert_eq!(
            rename_category(&mut plan, "missing", "New").unwrap_err(),
            "Select a category to rename."
        );
        assert_eq!(plan, before);
    }

    #[test]
    fn remove_category_restores_builtin_and_file_defaults_and_missing_labels() {
        let mut plan = AirPlan {
            categories: vec!["Custom".into()],
            ..AirPlan::default()
        };
        for mut sortie in super::super::builtin_library() {
            sortie.category = "Custom".into();
            add_library_item(&mut plan, sortie);
        }
        let mut file = la11();
        file.category = "Custom".into();
        add_library_item(&mut plan, file);
        assert_eq!(remove_category(&mut plan, "Custom"), 7);
        assert_eq!(
            plan.categories,
            ["Air alert", "Strike", "Close support", "Armed recon"]
        );
        assert_eq!(
            plan.library
                .iter()
                .map(|s| s.category.as_str())
                .collect::<Vec<_>>(),
            [
                "Air alert",
                "Strike",
                "Close support",
                "Armed recon",
                "Strike",
                "Strike",
                "Strike"
            ]
        );
        assert_eq!(remove_category(&mut plan, "Strike"), 4);
        assert_eq!(
            plan.categories,
            ["Air alert", "Close support", "Armed recon", "Strike"]
        );
        assert_eq!(remove_category(&mut plan, "missing"), 0);
        add_category(&mut plan, "Unused").unwrap();
        assert_eq!(remove_category(&mut plan, "Unused"), 0);
        assert!(!plan.categories.iter().any(|c| c == "Unused"));
    }

    #[test]
    fn category_counts_follow_list_order_and_include_unused_categories() {
        let plan = AirPlan {
            library: super::super::builtin_library(),
            categories: default_categories(),
            ..AirPlan::default()
        };
        assert_eq!(
            category_counts(&plan),
            vec![
                ("Air alert".into(), 1),
                ("Strike".into(), 3),
                ("Close support".into(), 1),
                ("Armed recon".into(), 1),
                ("Interdiction".into(), 0)
            ]
        );
    }

    #[test]
    fn auto_fill_filters_category_without_changing_existing_entries() {
        let ctx = context();
        let mut plan = AirPlan {
            library: super::super::builtin_library(),
            ..AirPlan::default()
        };
        assert_eq!(
            auto_fill(&mut plan, &ctx, AirSide::Dprk, Some("Air alert")),
            0
        );
        assert_eq!(
            auto_fill(&mut plan, &ctx, AirSide::Nato, Some("Air alert")),
            2
        );
        let alert = plan.pool.clone();
        assert_eq!(auto_fill(&mut plan, &ctx, AirSide::Dprk, Some("Strike")), 6);
        assert_eq!(auto_fill(&mut plan, &ctx, AirSide::Nato, Some("Strike")), 2);
        assert_eq!(&plan.pool[..2], &alert);
        assert!(
            plan.pool[2..]
                .iter()
                .all(|e| plan.sortie(&e.sortie_id).unwrap().category == "Strike")
        );
        assert_eq!(auto_fill(&mut plan, &ctx, AirSide::Nato, Some("Strike")), 0);
        assert_eq!(
            auto_fill(&mut plan, &ctx, AirSide::Nato, Some("Interdiction")),
            0
        );
    }
}
