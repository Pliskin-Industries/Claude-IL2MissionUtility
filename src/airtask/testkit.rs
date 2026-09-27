//! Air tasking test helpers for checking links in a mission tree.
//!
//! Dry-run walker placeholder: step 1b adds the walker here.

use std::collections::HashSet;

use crate::ast::Il2Entity;

pub(crate) fn assert_links_resolve(root: &Il2Entity) {
    let mut indexes = HashSet::new();
    root.for_each(&mut |node| {
        if let Some(index) = node.index {
            indexes.insert(index);
        }
    });
    root.for_each(&mut |node| {
        for (field, links) in [("Targets", &node.targets), ("Objects", &node.objects)] {
            for index in links {
                assert!(
                    indexes.contains(index),
                    "{} {:?} (Index {:?}) has dangling {field} index {index}",
                    node.block_type,
                    node.name().unwrap_or("<unnamed>"),
                    node.index,
                );
            }
        }
    });
}

fn builtin_template() -> Il2Entity {
    crate::parser::parse_group_file(include_str!(
        "../../TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group"
    ))
    .expect("the built-in HVAR template parses")
}

#[test]
fn assert_links_resolve_accepts_a_builtin_template() {
    assert_links_resolve(&builtin_template());
}

#[test]
#[should_panic(expected = "has dangling Targets index")]
fn assert_links_resolve_rejects_a_dangling_target() {
    let mut root = builtin_template();
    let missing_index = root.max_index() + 1;
    root.find_by_name_mut("Translator Mission Begin")
        .expect("the built-in HVAR template has a Mission Begin node")
        .append_target(missing_index);
    assert_links_resolve(&root);
}
