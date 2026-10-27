//! Acceptance flights after A0 (decision 0036), and the byte check of every
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

#[test]
fn acceptance_flights_regenerate_byte_for_byte() {
    // Flown and frozen acceptance files never change (decision 0036): the
    // current writers must still produce every frozen byte.
    let scratch = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/p14/acceptance-byte-check");
    let a0 = scratch.join("a0");
    let (mission, map) = acceptance_mission();
    write_acceptance_files(&a0, "P14_Acceptance_1", &mission, &map);
    let a1 = scratch.join("a1");
    write_a1_files(&a1);
    let frozen = Path::new(env!("CARGO_MANIFEST_DIR")).join("handoff/p14-probes");
    let mut compared = 0;
    for (directory, generated) in [("acceptance-a0", &a0), ("acceptance-a1", &a1)] {
        for entry in std::fs::read_dir(frozen.join(directory)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|extension| extension == "md") {
                continue;
            }
            let name = path.file_name().unwrap();
            let expected = std::fs::read(generated.join(name))
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert!(
                std::fs::read(&path).unwrap() == expected,
                "{} differs from the generated file",
                path.display()
            );
            compared += 1;
        }
    }
    // A0: Group, eng, trace.json. A1 adds spawns.json.
    assert_eq!(compared, 7);
}

#[test]
#[ignore = "writes target/p14/P14_Acceptance_A1.Group, locale, trace and spawn sidecars"]
fn write_p14_acceptance_a1() {
    let (written, map) = write_a1_files(Path::new("target/p14"));
    report_written(&written, &map);
}
