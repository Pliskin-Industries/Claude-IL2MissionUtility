//! Acceptance flights after A0 (decision 0036), and the checks of every
//! frozen acceptance flight. Test code only: `acceptance_test_plan()` and
//! what Generate writes for users do not change.
//!
//! Flight A1: a NATO player sits parked on Kimpo (K14), inside the DPRK area
//! that both DPRK entries attack. Run sheet:
//! `handoff/sessions/2026-10-27-p14-acceptance-a1.md`.

use super::*;

/// K14's spawn point: the Airfield `K-14_Kimpo_AF` in the reference group the
/// A1 build adds with `--ref` (ground start, F-80C).
const K14: (f64, f64) = (105_934.0, 269_477.0);
const K14_REF: &str = "TemplateExamples/K14 AFB_mp.Group";
const K14_AIRFIELD: &str = "K-14_Kimpo_AF";
/// The Map's own fighter pack in the base map fixture: 18 Planes of 601.
const WAVE: &str = "NATO Fighters Wave 1 pack 1";
const WAVE_PLANES: usize = 18;
const A1_STEM: &str = "P14_Acceptance_A1";
/// Flight A2's frozen files (built by the Map tab; see the A2 test below).
const A2_STEM: &str = "P14_Acceptance_A2";
/// Flight A3's frozen files (built by the Map tab; see the A3 test below).
const A3_STEM: &str = "P14_Acceptance_A3";
/// The fallback NATO air start (`--spawns`): 20 km south of K14 at 500 m,
/// heading north (YOri 0 is north, +X), so a level pass crosses the area centre.
const AIR_START: (f64, f64) = (K14.0 - 20_000.0, K14.1);
const AIR_START_HEADING: f64 = 0.0;
const AIR_START_ALTITUDE: f64 = 500.0;

fn a1_spawns_json() -> String {
    let (x, z) = AIR_START;
    format!(
        "[{{\"name\":\"P14 A1 NATO air start\",\"country\":601,\"x\":{x},\"z\":{z},\
         \"heading_deg\":{AIR_START_HEADING},\"altitude_m\":{AIR_START_ALTITUDE},\
         \"script\":\"f80c10\"}}]\n"
    )
}

/// `acceptance_test_plan()` with its one DPRK objective on K14, both DPRK
/// entries (the Yak raid, Zone; the Il-10 Kimpo attack, Both) aimed at it, and
/// the Map's NATO Fighters Wave 1 moved from 601 (the NATO player country) to
/// 602, so only the player can start a DPRK cycle.
fn acceptance_a1_plan() -> (AirPlan, TestContext) {
    let (mut plan, mut input) = acceptance_test_plan();
    input.east = vec![K14];
    let mut moved = 0;
    for entry in &mut plan.pool {
        if let TargetRef::Objective {
            side: AirSide::Dprk,
            xz,
        } = &mut entry.target
        {
            *xz = K14;
            moved += 1;
        }
    }
    assert_eq!(moved, 2, "the acceptance plan's two DPRK entries");
    let wave = input
        .opts
        .fighter_packs
        .iter_mut()
        .find(|pack| pack.root.name() == Some(WAVE))
        .expect("the base map's NATO Fighters Wave 1");
    let mut planes = 0;
    wave.root.for_each_mut(&mut |node| {
        if node.block_type == "Plane" {
            assert_eq!(node.property("Country"), Some("601"));
            node.set_property("Country", "602");
            planes += 1;
        }
    });
    assert_eq!(planes, WAVE_PLANES);
    (plan, input)
}

fn acceptance_a1_mission() -> (frontlines::FrontPack, trace::TraceMap) {
    let (plan, input) = acceptance_a1_plan();
    traced_acceptance_mission(plan, input)
}

/// Writes the A1 Group, locale sidecars, `.trace.json` and `.spawns.json`.
fn write_a1_files(dir: &Path) -> (Vec<PathBuf>, trace::TraceMap) {
    let (mission, map) = acceptance_a1_mission();
    let mut written = write_acceptance_files(dir, A1_STEM, &mission, &map);
    let spawns = dir.join(format!("{A1_STEM}.spawns.json"));
    std::fs::write(&spawns, a1_spawns_json()).expect("write A1 spawn list");
    written.push(spawns);
    (written, map)
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn planes_of_country(root: &Il2Entity, country: &str) -> usize {
    let mut count = 0;
    root.for_each(&mut |node| {
        if node.block_type == "Plane" && node.property("Country") == Some(country) {
            count += 1;
        }
    });
    count
}

#[test]
fn a1_plan_parks_the_player_inside_the_dprk_area() {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(K14_REF))
        .expect("read the K14 reference group");
    let reference = parser::parse_group_file(&text).unwrap();
    let airfield = named(&reference, K14_AIRFIELD);
    assert_eq!(airfield.block_type, "Airfield");
    assert_eq!(airfield.pos_xz(), Some(K14));
    assert_eq!(airfield.property("Country"), Some("601"));

    let (plan, input) = acceptance_a1_plan();
    let ctx = input.context();
    let issues = super::super::validate(&plan, &ctx);
    assert_no_errors(&issues);
    // A0's only issue was D30 on the old DPRK area 1, which A1 no longer uses.
    assert!(issues.is_empty(), "{issues:?}");

    // K14 lies on DPRK's side of the 1950-09-15 front, so DPRK sorties
    // aimed at it spawn on their own side, as the preview check below shows.
    assert!(mapclip::point_north_of_front(&input.front, K14.0, K14.1));
    let dprk: Vec<_> = plan
        .pool
        .iter()
        .filter(|entry| {
            let sortie = plan.library.iter().find(|s| s.id == entry.sortie_id);
            sortie.unwrap().side == AirSide::Dprk
        })
        .collect();
    assert_eq!(
        dprk.iter().map(|entry| entry.mode).collect::<Vec<_>>(),
        [TriggerMode::Zone, TriggerMode::Both]
    );
    let inner = timing::derived_timing(&plan.threshold).inner_m;
    assert_eq!(inner, 6_000.0);
    let preview = super::super::preview(&plan, &ctx);
    let rings: Vec<_> = preview
        .rings
        .iter()
        .filter(|ring| ring.side == AirSide::Dprk && ring.inner)
        .collect();
    assert_eq!(rings.len(), 1, "one DPRK area");
    // The area is centred on the ground spawn, so the parked player is inside r_in.
    assert_eq!((rings[0].centre, rings[0].radius_m), (K14, inner));
    for spawn in &preview.spawns {
        assert_eq!(
            mapclip::point_north_of_front(&input.front, spawn.at.0, spawn.at.1),
            spawn.side == AirSide::Dprk,
            "{spawn:?}"
        );
        if spawn.side == AirSide::Dprk {
            assert_eq!(spawn.target, K14);
        }
    }

    // The fallback air start begins outside every area's R and flies north
    // straight over the DPRK area's centre.
    let outer = plan.threshold.radius_m;
    assert!((distance(AIR_START, K14) - 20_000.0).abs() < 1e-6);
    assert_eq!((AIR_START.1, AIR_START_HEADING), (K14.1, 0.0));
    assert!(AIR_START.0 < K14.0, "heading north reaches K14");
    for ring in &preview.rings {
        assert!(distance(AIR_START, ring.centre) > outer, "{ring:?}");
    }
}

#[test]
fn a1_mission_watches_k14_for_nato_players_only() {
    let (mission, map) = acceptance_a1_mission();
    let root = &mission.root;

    // The wave is on 602 in A1 (`acceptance_a1_plan` checks each Plane was
    // 601 before), and no aircraft of 601 is left in the file.
    let wave = named(root, WAVE);
    assert_eq!(wave.count_block_type("Plane"), WAVE_PLANES);
    assert_eq!(planes_of_country(wave, "602"), WAVE_PLANES);
    assert_eq!(planes_of_country(root, "601"), 0);

    // One DPRK area, on K14: its entry trigger counts planes of 601 only.
    let air = named(root, "Air Tasking");
    assert!(air.find_by_name("THD02 TRIGGER").is_none());
    let trigger = named(air, "THD01 TRIGGER");
    assert_eq!(trigger.block_type, "MCU_TR_ComplexTrigger");
    assert_eq!(trigger.pos_xz(), Some(K14));
    for (key, value) in [
        ("Radius", "6000"),
        ("Country", "601"),
        ("CheckPlanes", "1"),
        ("CheckVehicles", "0"),
    ] {
        assert_eq!(trigger.property(key), Some(value), "{key}");
    }
    let inner = named(air, "THD01 INNER");
    assert_eq!(inner.block_type, "MCU_CheckZone");
    assert_eq!(inner.pos_xz(), Some(K14));
    assert_eq!(inner.property("Zone"), Some("6000"));

    // The file parses back, links resolve, indexes are unique, every LC id
    // has text, and every trace entry names existing MCUs.
    let reparsed = parser::parse_group_file(&serialize::serialize_group(root)).unwrap();
    assert_links_resolve(&reparsed);
    assert_unique(&reparsed);
    let mut indexes = Vec::new();
    reparsed.collect_indexes(&mut indexes);
    let indexes: BTreeSet<_> = indexes.into_iter().collect();
    assert!(!map.entries.is_empty());
    for entry in &map.entries {
        assert!(indexes.contains(&entry.source_index));
        assert!(indexes.contains(&entry.breadcrumb_index));
    }
    let tables = acceptance_locales(&mission);
    reparsed.for_each(&mut |node| {
        for key in ["LCName", "LCDesc", "LCText"] {
            if let Some(id) = node.property(key) {
                let id: i32 = id.parse().unwrap();
                assert!(id == 0 || tables["eng"].get(id).is_some(), "locale {id}");
            }
        }
    });
}

/// Frozen acceptance flights: directory and file stem. Decision 0038 changed
/// the shell that A0 and A1 flew (DONE HUB OFF now quiets each used copy), so
/// their files stay as flown and are no longer regenerated (as 0018 to 0025
/// did for the probe flights). A flight flown after 0038 gets a byte check.
const FROZEN: [(&str, &str); 2] = [
    ("acceptance-a0", "P14_Acceptance_1"),
    ("acceptance-a1", A1_STEM),
];

#[test]
fn frozen_acceptance_files_parse_and_links_resolve() {
    let frozen = Path::new(env!("CARGO_MANIFEST_DIR")).join("handoff/p14-probes");
    for (directory, stem) in FROZEN {
        let root = crate::airtask::testkit::assert_frozen_files_resolve(&frozen.join(directory).join(stem));
        // As flown, before 0038: DONE HUB OFF closes only its hub.
        let mut offs = 0;
        root.for_each(&mut |node| {
            if node.name().is_some_and(|name| name.ends_with(" DONE HUB OFF")) {
                assert_eq!(node.targets.len(), 1, "{stem}: flown DONE HUB OFF");
                offs += 1;
            }
        });
        assert!(offs > 0, "{stem}: no sortie copy");
    }
    // A1's spawn list is still what the A1 writer produces.
    let spawns = frozen.join("acceptance-a1").join(format!("{A1_STEM}.spawns.json"));
    assert_eq!(std::fs::read_to_string(spawns).unwrap(), a1_spawns_json());
}

/// Flight A2: built by the Map tab after decision 0038 and flown on
/// 2026-10-29, so frozen as flown. Its writer and its build-facts check are UI
/// tests (`ui_tests::write_p14_acceptance_a2`,
/// `a2_map_build_watches_three_nato_areas_for_dprk_players`).
#[test]
fn frozen_a2_files_parse_and_links_resolve() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("handoff/p14-probes/acceptance-a2");
    let root = crate::airtask::testkit::assert_frozen_files_resolve(&dir.join(A2_STEM));
    assert_done_hub_offs(&root, 9, "A2");
    // One DPRK (501) Yak-9P air start, outside every area's R (twice the
    // entry trigger's radius), so the player is seen flying in.
    let spawns = std::fs::read_to_string(dir.join(format!("{A2_STEM}.spawns.json"))).unwrap();
    assert!(spawns.contains("\"country\":501,") && spawns.contains("\"script\":\"yak9p\""), "{spawns}");
    // The list is one flat JSON object (no serde_json here): read a number field.
    let number = |key: &str| -> f64 {
        let label = format!("\"{key}\":");
        let start = spawns.find(&label).unwrap_or_else(|| panic!("A2 spawn list: no {key}")) + label.len();
        let tail = &spawns[start..];
        let end = tail.find([',', '}']).unwrap_or_else(|| panic!("A2 spawn list: {key} unterminated"));
        tail[..end].trim().parse().unwrap_or_else(|_| panic!("A2 spawn list: {key} is not a number"))
    };
    assert_areas_watch_dprk_from_outside(&root, (number("x"), number("z")), 3, "A2");
}

/// After decision 0038, each copy's DONE HUB OFF also switches off the copy's
/// timers and zones. `copies` is the number of NATO copies in the flight.
fn assert_done_hub_offs(root: &Il2Entity, copies: usize, flight: &str) {
    let mut offs = 0;
    root.for_each(&mut |node| {
        if node.name().is_some_and(|name| name.ends_with(" DONE HUB OFF")) {
            assert!(node.targets.len() > 1, "{flight}: DONE HUB OFF closes only its hub");
            offs += 1;
        }
    });
    assert_eq!(offs, copies, "{flight}: NATO copies");
}

/// Every NATO area's entry trigger watches DPRK players (501), and the player
/// spawn is outside each area's R (twice the trigger's radius), so the player
/// is seen flying in. `areas` is the number of NATO areas in the flight.
fn assert_areas_watch_dprk_from_outside(root: &Il2Entity, spawn: (f64, f64), areas: usize, flight: &str) {
    let mut triggers = 0;
    root.for_each(&mut |node| {
        if node.block_type == "MCU_TR_ComplexTrigger" {
            triggers += 1;
            assert_eq!(node.property("Country"), Some("501"), "{flight}: the areas watch DPRK players");
            let radius: f64 = node.property("Radius").unwrap().parse().unwrap();
            assert!(distance(spawn, node.pos_xz().unwrap()) > 2.0 * radius, "{flight}: spawn inside R");
        }
    });
    assert_eq!(triggers, areas, "{flight}: NATO areas");
}

/// Flight A3: built by the Map tab after decisions 0042 and 0045 (run sheet
/// `handoff/sessions/2026-10-30-p14-acceptance-a3.md`). Its writer and its
/// byte and facts check are UI tests (`ui_tests::write_p14_acceptance_a3`,
/// `a3_map_build_matches_the_frozen_files_and_the_run_sheet`).
#[test]
fn frozen_a3_files_parse_and_links_resolve() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("handoff/p14-probes/acceptance-a3");
    let root = crate::airtask::testkit::assert_frozen_files_resolve(&dir.join(A3_STEM));
    assert_done_hub_offs(&root, 6, "A3");
    // The player spawn is the Map's own air start (no --spawns): one Airfield
    // on 501, the country the area watches, outside the area's R (twice the
    // entry trigger's radius).
    let mut airfields = Vec::new();
    root.for_each(&mut |node| {
        if node.block_type == "Airfield" {
            airfields.push((node.property("Country").map(str::to_string), node.pos_xz().expect("A3: Airfield without a position")));
        }
    });
    let [(Some(country), spawn)] = airfields.as_slice() else {
        panic!("A3: one air start, {airfields:?}");
    };
    assert_eq!(country, "501", "A3: the air start is on the DPRK player country");
    assert_areas_watch_dprk_from_outside(&root, *spawn, 1, "A3");
}

#[test]
#[ignore = "writes target/p14/P14_Acceptance_A1.Group, locale, trace and spawn sidecars"]
fn write_p14_acceptance_a1() {
    let (written, map) = write_a1_files(Path::new("target/p14"));
    report_written(&written, &map);
}

/// P14 step 11 item 4 (risk l): the Air Tasking tab's standalone file,
/// imported with the Mission Editor's File › Import From File into a mission
/// built from the base map (`build_standalone_l` in `ui_tests.rs`), then saved
/// by the editor. Run sheet: `handoff/sessions/2026-10-30-p14-standalone-l.md`.
const L_DIR: &str = "handoff/p14-probes/standalone-l";
const L_SAVED: &str = "P14_Standalone_L_20261030";

/// The editor renumbered both parts of the mission and kept every block,
/// property, position and link: the saved mission is the base map and the
/// standalone group, each under one index map, with no index used twice. The
/// standalone file's trace sidecar still matches the saved breadcrumbs, as
/// Replay matches them, by position.
#[test]
fn standalone_l_editor_import_renumbers_and_keeps_every_link() {
    use crate::airtask::testkit::{
        EDITOR_GROUP_NAME_CHARS, Renumbering, assert_indexes_unique_and_links_resolve, assert_same_up_to_indexes,
        read_locale_file,
    };
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(L_DIR);
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let saved = crate::harvest::parse_mission_body(&read(&format!("{L_SAVED}.Mission"))).unwrap();
    let saved_eng = read_locale_file(&dir.join(format!("{L_SAVED}.eng")));
    let group = |stem: &str| crate::parser::parse_group_file(&read(&format!("{stem}.Group"))).unwrap();
    let (base, air) = (group("P14_Standalone_L_Base"), group("P14_Standalone_L_Air"));

    // The body is two blocks: the base map, then the imported group.
    let top: Vec<_> = saved.children.iter().map(|c| (c.block_type.as_str(), c.name())).collect();
    assert_eq!(top, [("Group", Some("Korea Base Map 1951-04-22")), ("Group", Some("Air Tasking"))]);
    let (saved_base, saved_air) = (&saved.children[0], &saved.children[1]);
    let unique = assert_indexes_unique_and_links_resolve(&saved, L_SAVED);

    // Each part is its file, renumbered. The base map's texts moved with the
    // builder's language offset; the standalone file has none (every id is 0).
    let base_eng = read_locale_file(&dir.join("P14_Standalone_L_Base.eng"));
    let base_map = assert_same_up_to_indexes(
        &base,
        saved_base,
        Renumbering { locales: Some((&base_eng, &saved_eng)), editor_saved: true },
    );
    let no_texts = crate::locale::LocaleTable::default();
    let air_map =
        assert_same_up_to_indexes(&air, saved_air, Renumbering { locales: Some((&no_texts, &saved_eng)), editor_saved: true });
    assert_eq!(base_map.len() + air_map.len(), unique.len(), "a block was added or lost");
    // The two files share indexes (both start at 1), so the editor had to
    // renumber: the saved file's indexes are unique (above).
    assert!(base_map.keys().any(|k| air_map.contains_key(k)), "the two files do not share an index");
    assert_eq!(air_map.get(&1), Some(&2251), "the Air Tasking group, as the editor's tree shows it");
    // Only the six copies' group names were longer than the editor keeps.
    let mut cut = Vec::new();
    air.for_each(&mut |node| {
        if let Some(name) = node.name()
            && name.chars().count() > EDITOR_GROUP_NAME_CHARS
        {
            cut.push((node.block_type.clone(), name.chars().take(4).collect::<String>()));
        }
    });
    let copies: Vec<_> = (1..=6).map(|n| ("Group".to_string(), format!("SN0{n}"))).collect();
    assert_eq!(cut, copies, "names the editor cut");

    // The trace sidecar: each breadcrumb is at its position in the saved file,
    // and that position is unique among the saved objectives.
    let map = crate::trace::read_trace_sidecar(&dir.join("P14_Standalone_L_Air.trace.json")).unwrap();
    let mut objectives = Vec::new();
    saved.for_each(&mut |node| {
        if node.block_type == "MCU_TR_MissionObjective" {
            let at = node.pos_xz().unwrap_or_else(|| panic!("objective {:?} without a position", node.index));
            objectives.push((node.index.unwrap(), at));
        }
    });
    assert_eq!(map.entries.len(), objectives.len(), "every objective is a breadcrumb");
    for entry in &map.entries {
        let at: Vec<_> = objectives
            .iter()
            .filter(|(_, (x, z))| (x - entry.pos.0).abs() <= 1.0 && (z - entry.pos.1).abs() <= 1.0)
            .collect();
        assert_eq!(at.len(), 1, "{}: one objective at its position", entry.source_name);
        assert_eq!(Some(&at[0].0), air_map.get(&entry.breadcrumb_index), "{}", entry.source_name);
    }
}
