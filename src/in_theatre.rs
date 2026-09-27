#![allow(dead_code)] // P14: removed in step 9
//! Aircraft availability for P14, keyed by model_spec's plane ids.
//!
//! Dates/citations come from korea-1950-53-unit-reference.md sections 2.2, 3
//! and 9. Only the explicit entry dates in P14 section 5 are treated as sourced;
//! other rows retain the mandated, permissive war-span fallback. A dated sortie
//! or a unit's base move alone does not establish a model's first theatre date.

use crate::airtask::{AirSide, DateRange, Ymd};
use crate::model_spec;

pub const WAR_SPAN: DateRange = DateRange {
    from: (1950, 6, 25),
    to: (1953, 7, 27),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TheatreEntry {
    pub id: &'static str,
    pub valid: DateRange,
    pub side: AirSide,
    pub citation: &'static str,
    pub sourced: bool,
}

const fn unsourced(id: &'static str, side: AirSide) -> TheatreEntry {
    TheatreEntry {
        id,
        valid: WAR_SPAN,
        side,
        citation: "",
        sourced: false,
    }
}

pub const ROWS: &[TheatreEntry] = &[
    unsourced("b29", AirSide::Nato),
    unsourced("simpleb29", AirSide::Nato),
    unsourced("c47b", AirSide::Nato),
    unsourced("f51d", AirSide::Nato),
    unsourced("f80c10", AirSide::Nato),
    unsourced("f84e", AirSide::Nato),
    unsourced("f86a5", AirSide::Nato),
    unsourced("il10", AirSide::Dprk),
    TheatreEntry {
        id: "la11",
        valid: DateRange {
            from: (1953, 4, 15),
            to: WAR_SPAN.to,
        },
        side: AirSide::Dprk,
        citation: "First La-11 night-heckler raids from 15 Apr 1953 [S9 p.685], [S9 p.686]",
        sourced: true,
    },
    unsourced("li2t", AirSide::Dprk),
    TheatreEntry {
        id: "mig15bis",
        valid: DateRange {
            from: (1950, 11, 1),
            to: WAR_SPAN.to,
        },
        side: AirSide::Dprk,
        citation: "First MiG-15s on 1 Nov 1950 [S9 p.241]; stand-in for MiG-15; the bis variant appeared Nov 1951 [S9 p.434]",
        sourced: true,
    },
    unsourced("tu2", AirSide::Dprk),
    unsourced("yak9p", AirSide::Dprk),
];

// R4 P11: La-9 first appears on 1951-11-30 [S9 p.437], but model_spec has
// no La-9 id. Add its row when that model exists; do not substitute La-11.

/// Accept either a model id or its quoted/unquoted Script path.
pub fn for_model(model: &str) -> Option<&'static TheatreEntry> {
    let id = model_spec::script_id(model);
    ROWS.iter().find(|row| row.id == id)
}

/// Inclusive range check; an unknown model has no availability claim.
pub fn valid_on(model: &str, date: Ymd) -> bool {
    for_model(model).is_some_and(|row| row.valid.from <= date && date <= row.valid.to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_spec::ModelClass;
    use std::collections::BTreeSet;
    use std::path::Path;

    #[test]
    fn in_theatre_la11_invalid_in_1950() {
        assert!(!valid_on("la11", (1950, 8, 1)));
        assert!(!valid_on("la11", (1953, 4, 14)));
        assert!(valid_on("la11", (1953, 4, 15)));
        let row = for_model("la11").unwrap();
        assert!(row.sourced);
        assert!(row.citation.contains("[S9 p.686]"));
        assert_eq!(row.side, AirSide::Dprk);
    }

    #[test]
    fn in_theatre_mig15bis_from_1950_11_01() {
        assert!(!valid_on("mig15bis", (1950, 10, 31)));
        assert!(valid_on("mig15bis", (1950, 11, 1)));
        let row = for_model("mig15bis").unwrap();
        assert!(row.sourced);
        assert!(row.citation.contains("stand-in"));
        assert!(row.citation.contains("[S9 p.241]"));
        assert!(row.citation.contains("Nov 1951 [S9 p.434]"));
    }

    #[test]
    fn in_theatre_all_builtins_valid_on_1950_07_01() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("TemplateExamples/Historical1950");
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "Group"))
            .collect();
        files.sort();
        assert_eq!(files.len(), 6);
        let mut models = BTreeSet::new();
        for path in files {
            let root =
                crate::parser::parse_group_file(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let mut planes = 0;
            root.for_each(&mut |e| {
                if e.block_type == "Plane" {
                    planes += 1;
                    let script = e.property("Script").expect("plane Script");
                    assert!(
                        valid_on(script, (1950, 7, 1)),
                        "{}: {script}",
                        path.display()
                    );
                    models.insert(model_spec::script_id(script));
                }
            });
            assert!(planes > 0, "{} has no planes", path.display());
        }
        assert_eq!(
            models,
            ["f51d", "f80c10", "il10", "yak9p"].map(String::from).into()
        );
    }

    #[test]
    fn in_theatre_every_row_is_cited_or_unsourced() {
        // SPECS is private. Discover ids in its source, then use the real public
        // lookup/classification so no parallel, hard-coded plane list can drift.
        let plane_ids: BTreeSet<_> = include_str!("model_spec.rs")
            .split("spec(")
            .skip(1)
            .filter_map(|call| call.trim_start().strip_prefix('"'))
            .filter_map(|call| call.split('"').next())
            .filter(|id| {
                model_spec::spec_for(id).is_some_and(|s| {
                    matches!(
                        s.class,
                        ModelClass::Fighter
                            | ModelClass::FighterBomber
                            | ModelClass::Attack
                            | ModelClass::Bomber
                            | ModelClass::Transport
                    )
                })
            })
            .collect();
        assert!(plane_ids.len() >= 13);
        let row_ids: BTreeSet<_> = ROWS.iter().map(|row| row.id).collect();
        assert_eq!(row_ids.len(), ROWS.len(), "duplicate availability row");
        assert_eq!(
            row_ids, plane_ids,
            "every model_spec plane needs exactly one row"
        );
        for row in ROWS {
            assert!(!row.citation.is_empty() || !row.sourced, "{}", row.id);
            assert!(row.valid.from <= row.valid.to, "{}", row.id);
            if !row.sourced {
                assert_eq!(
                    row.valid, WAR_SPAN,
                    "{} must use the unsourced fallback",
                    row.id
                );
            }
            assert!(valid_on(row.id, row.valid.from));
            assert!(valid_on(row.id, row.valid.to));
            assert!(!valid_on(row.id, (1950, 6, 24)));
            assert!(!valid_on(row.id, (1953, 7, 28)));
        }
        assert_eq!(for_model("f80c10").unwrap().side, AirSide::Nato);
        assert!(for_model("not-a-model").is_none());
        assert!(!valid_on("not-a-model", (1950, 7, 1)));
    }
}
