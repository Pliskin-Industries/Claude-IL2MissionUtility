//! Standalone Air Tasking output and the source files supplying its sidecars.

use std::path::PathBuf;

use super::{AirContext, AirPlan, SortieSource, build_air_packs};
use crate::{ast::Il2Entity, duplicate::duplicate_template};

/// One top-level group, with all generated packs and indexes starting at 1.
pub fn build_air_group(plan: &AirPlan, ctx: &AirContext) -> Result<Il2Entity, String> {
    let packs = build_air_packs(plan, ctx)?;
    let mut root = Il2Entity::new("Group");
    root.set_name("Air Tasking");
    root.children = packs.into_iter().map(|pack| pack.root).collect();
    root.index = Some(root.max_index() + 1);
    root.set_property("Index", root.index.unwrap().to_string());
    Ok(duplicate_template(&root, &mut 1).0)
}

/// Each user template used by an enabled pool entry, once in library order.
pub fn sidecar_paths(plan: &AirPlan) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for sortie in &plan.library {
        if let SortieSource::File(path) = &sortie.source
            && plan
                .pool
                .iter()
                .any(|entry| entry.enabled && entry.sortie_id == sortie.id)
            && !paths.contains(path)
        {
            paths.push(path.clone());
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airtask::{AirSide, PoolEntry, TargetRef, builtin_library};
    use crate::{mapclip::WorldAabb, parser, serialize::serialize_group};

    #[test]
    fn standalone_has_one_named_group_and_indexes_from_one() {
        let ctx = AirContext {
            east_objectives: &[],
            nato_objectives: &[],
            arrows: &[],
            front: &[(1000.0, 0.0), (1000.0, 1000.0)],
            aabb: WorldAabb::full_map(),
            date: (1950, 7, 1),
        };
        let plan = AirPlan::default();
        let before = plan.clone();
        let root = build_air_group(&plan, &ctx).unwrap();
        assert_eq!(root.block_type, "Group");
        assert_eq!(root.name(), Some("Air Tasking"));
        assert_eq!(root.index, Some(1));
        assert_eq!(root.property("Index"), Some("1"));
        assert_eq!(
            root.children.len(),
            build_air_packs(&plan, &ctx).unwrap().len()
        );
        let mut indexes = Vec::new();
        root.for_each(&mut |node| {
            if let Some(id) = node.index {
                indexes.push(id);
            }
        });
        indexes.sort_unstable();
        assert_eq!(indexes, (1..=indexes.len() as i32).collect::<Vec<_>>());
        crate::airtask::testkit::assert_links_resolve(&root);
        assert_eq!(
            parser::parse_group_file(&serialize_group(&root)).unwrap(),
            root
        );
        assert_eq!(plan, before);
    }

    #[test]
    fn sidecars_only_include_enabled_user_templates_once() {
        let mut plan = AirPlan {
            library: builtin_library(),
            ..AirPlan::default()
        };
        for (id, file) in [
            ("used", "used.Group"),
            ("off", "off.Group"),
            ("unused", "unused.Group"),
            ("alias", "used.Group"),
        ] {
            let mut sortie = plan.library[0].clone();
            sortie.id = id.into();
            sortie.source = SortieSource::File(file.into());
            plan.library.push(sortie);
        }
        let target = TargetRef::Objective {
            side: AirSide::Nato,
            xz: (900.0, 100.0),
        };
        for id in [
            plan.library[0].id.as_str(),
            "used",
            "used",
            "off",
            "alias",
            "missing",
        ] {
            plan.pool.push(PoolEntry::new(id.into(), target));
        }
        plan.pool[3].enabled = false;
        assert_eq!(sidecar_paths(&plan), vec![PathBuf::from("used.Group")]);
        for entry in &mut plan.pool {
            entry.enabled = false;
        }
        assert!(sidecar_paths(&plan).is_empty());
    }
}
