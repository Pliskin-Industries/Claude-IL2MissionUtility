//! Standalone Air Tasking output and the source files supplying its sidecars.

use std::path::PathBuf;

use super::{AirContext, AirPlan, SortieSource, build_air_packs};
use crate::{ast::Il2Entity, duplicate::duplicate_template, frontlines::MapAirPack};

const GROUP_NAME: &str = "Air Tasking";

/// One top-level group, with all generated packs and indexes starting at 1.
pub fn build_air_group(plan: &AirPlan, ctx: &AirContext) -> Result<Il2Entity, String> {
    Ok(air_group_from(build_air_packs(plan, ctx)?))
}

/// A lone pack already rooted at an "Air Tasking" group (step 7's proposed
/// decision 0035) becomes the root itself; any other list is wrapped in one.
fn air_group_from(mut packs: Vec<MapAirPack>) -> Il2Entity {
    let root = if packs.len() == 1
        && packs[0].root.block_type == "Group"
        && packs[0].root.name() == Some(GROUP_NAME)
    {
        packs.pop().unwrap().root
    } else {
        let mut root = Il2Entity::new("Group");
        root.set_name(GROUP_NAME);
        root.children = packs.into_iter().map(|pack| pack.root).collect();
        root.index = Some(root.max_index() + 1);
        root.set_property("Index", root.index.unwrap().to_string());
        root
    };
    duplicate_template(&root, &mut 1).0
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

    fn builtin_root(index: usize) -> Il2Entity {
        parser::parse_group_file(crate::airtask::library::BUILTINS[index].1).unwrap()
    }

    fn sorted_indexes(root: &Il2Entity) -> Vec<i32> {
        let mut indexes = Vec::new();
        root.for_each(&mut |node| {
            if let Some(id) = node.index {
                indexes.push(id);
            }
        });
        indexes.sort_unstable();
        indexes
    }

    #[test]
    fn lone_pack_rooted_at_air_tasking_is_not_wrapped_again() {
        let mut pack_root = Il2Entity::new("Group");
        pack_root.set_name("Air Tasking");
        pack_root.children = vec![builtin_root(0), builtin_root(1)];
        pack_root.index = Some(pack_root.max_index() + 1);
        pack_root.set_property("Index", pack_root.index.unwrap().to_string());
        let root = air_group_from(vec![MapAirPack { root: pack_root }]);
        assert_eq!(root.name(), Some("Air Tasking"));
        assert_eq!(root.children.len(), 2);
        assert!(root.children.iter().all(|child| child.name() != Some("Air Tasking")));
        let indexes = sorted_indexes(&root);
        assert_eq!(indexes, (1..=indexes.len() as i32).collect::<Vec<_>>());
        crate::airtask::testkit::assert_links_resolve(&root);
    }

    #[test]
    fn several_packs_are_wrapped_in_one_air_tasking_group() {
        let packs = vec![
            MapAirPack { root: builtin_root(0) },
            MapAirPack { root: builtin_root(4) },
        ];
        let root = air_group_from(packs);
        assert_eq!(root.name(), Some("Air Tasking"));
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.index, Some(1));
        let indexes = sorted_indexes(&root);
        assert_eq!(indexes, (1..=indexes.len() as i32).collect::<Vec<_>>());
        crate::airtask::testkit::assert_links_resolve(&root);
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
