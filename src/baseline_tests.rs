//! Byte-for-byte Generate baselines, recorded before the P14 helpers are built.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::Il2Entity;
use crate::frontlines::{FrontOptions, MapFighterPack, MapGroundPack, MapShipPack};
use crate::locale::{LANG_EXTS, LocaleTable, merge_template_sidecars, write_sidecars};
use crate::mapground::{GroundKind, GroundSpot};
use crate::recon::{ReconBuild, ReconInput};
use crate::template::{FlightRole, OrderKind, OrderSpec, TemplateOptions, TemplateSeat};
use crate::{
    aircraft, airfield, bombers, duplicate, flights, frontlines, mapfighters, mapshipping,
};
use crate::{model_spec, pack, parser, recon, serialize, template};

const FIGHTER: &str = "TemplateExamples/Eastern_Fighters_Random_3pack_V6.Group";
const ARMOR: &str = "TemplateExamples/GroundUnits/DropIns/DPRK Tank Platoon.Group";
const SHIPS: &str = "TemplateExamples/GroundUnits/DropIns/DPRK Ships.Group";
const ARMY: &str = "TemplateExamples/GroundUnits/DropIns/DPRK ML20 Arty.Group";
const AIRFIELD: &str = "TemplateExamples/Seoul AFB no helper.Group";
const EXCLUSIVE: &str = "TemplateExamples/Exclusive_Activation_6plan.Group";
const PLACEMENT_SEED: u64 = 14_001;

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn load(path: &str) -> Il2Entity {
    let text = std::fs::read_to_string(repo(path)).unwrap();
    parser::parse_group_file(&text).unwrap_or_else(|err| panic!("{path}: {err}"))
}

struct Fixture {
    name: &'static str,
    root: Il2Entity,
    locales: HashMap<String, LocaleTable>,
}

impl Fixture {
    fn new(name: &'static str, root: Il2Entity, sources: &[&str]) -> Self {
        Self {
            name,
            root,
            locales: merge_template_sidecars(&sources.iter().map(|p| repo(p)).collect::<Vec<_>>()),
        }
    }

    fn write(&self, dir: &Path) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(self.name);
        // Remove only this fixture's previous outputs, so a missing sidecar cannot
        // be hidden by a file left by an earlier test run.
        for ext in std::iter::once("Group").chain(LANG_EXTS.iter().copied()) {
            match std::fs::remove_file(path.with_extension(ext)) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => panic!("{}: {err}", path.display()),
            }
        }
        std::fs::write(&path, serialize::serialize_group(&self.root)).unwrap();
        write_sidecars(&path, &self.locales).unwrap();
        path
    }
}

fn template_fixture() -> Fixture {
    let unit = template::builtin_plane_catalog()
        .into_iter()
        .find(|u| model_spec::script_id(&u.script) == "f80c10")
        .unwrap();
    let mut seats = Vec::new();
    for i in 0..4 {
        let mut seat = TemplateSeat::new(unit.clone());
        seat.country = 601;
        seat.skill = 2;
        seat.altitude = 3050.0;
        seat.role = if i == 0 {
            FlightRole::Lead
        } else {
            FlightRole::Follows(0)
        };
        seat.number_in_formation = i;
        if i == 0 {
            seat.formation_count = 4;
            seat.orders = [
                OrderKind::Formation,
                OrderKind::GotoWaypoint,
                OrderKind::AttackArea,
                OrderKind::MissionComplete,
            ]
            .into_iter()
            .map(|kind| OrderSpec {
                kind,
                waypoint: 1,
                ..OrderSpec::default()
            })
            .collect();
        }
        seats.push(seat);
    }
    let opts = TemplateOptions {
        name: "P14 F-80C baseline".into(),
        waypoint_count: template::used_waypoint_count(&seats),
        waypoint_speed: model_spec::suggested_waypoint_speed_kmh(["f80c10"]),
        waypoint_altitude: 3050.0,
        zone_coalition: template::ZoneCoalition::Eastern,
        seats,
        ..TemplateOptions::default()
    };
    Fixture::new(
        "template.Group",
        template::generate_template(&opts).unwrap(),
        &[],
    )
}

fn recon_input(path: &str, copies: usize) -> ReconInput {
    let root = load(path);
    let info = recon::inspect_unit(&root).unwrap();
    assert!(!info.suggested_triggers.is_empty(), "{path}: no Zone IN");
    ReconInput {
        label: info.name,
        root,
        trigger_zone_ids: info.suggested_triggers,
        copies,
    }
}

// Rebuild ui.rs::configured_fighter_root through the public preparation function.
fn configured_fighter_root() -> Il2Entity {
    let mut root = load(FIGHTER);
    let cfg = flights::FlightConfig {
        flight_count: 3,
        max_in_flight: 4,
        type_ids: vec!["f80c10".into(), "f51d".into()],
        type_skills: vec![3, 2],
        country: 601,
        cooldown: 180.0,
        reinforcement: 300.0,
        delete_orders: 60.0,
        altitude_min: 1000.0,
        altitude_max: 5500.0,
    };
    flights::configure_aircraft(&mut root, &cfg).unwrap();
    root
}

fn base_map_fixture() -> Fixture {
    let mut opts = FrontOptions {
        battles: false,
        buildups: false,
        defenses: false,
        attacks: false,
        naval: false,
        user_attacks: vec![((140_000.0, 220_000.0), (130_000.0, 240_000.0))],
        ..FrontOptions::default()
    };
    let front = frontlines::snapshot_front_xz(opts.year, opts.season);

    // ui.rs::build_map_fighter_packs, with fixed preview positions.
    let positions = [(110_000.0, 280_000.0), (130_000.0, 300_000.0)];
    let mut fighters = pack::generate_pack_at(
        &configured_fighter_root(),
        &positions,
        "NATO Fighters Wave 1 pack 1",
    )
    .unwrap();
    let rtbs: Vec<_> = positions
        .iter()
        .map(|&(x, z)| mapfighters::rtb_ao_point(false, x, z, opts.aabb))
        .collect();
    pack::park_rtbs(&mut fighters, &rtbs);
    duplicate::apply_overrides(&mut fighters, "", 601);
    opts.fighter_packs.push(MapFighterPack { root: fighters });

    // ui.rs::build_map_ship_packs. Preview yaw uses a fixed seed, never the clock.
    let mut layout = mapshipping::MapShipLayout {
        eastern: true,
        spots: [(170_000.0, 150_000.0), (190_000.0, 160_000.0)]
            .into_iter()
            .map(|(x, z)| mapshipping::ShipSpot {
                x,
                z,
                in_ao: true,
                heading_deg: 0.0,
            })
            .collect(),
    };
    layout.randomize_headings(PLACEMENT_SEED);
    let mut ships = recon::generate_recon_ex(
        &[recon_input(SHIPS, 2)],
        ReconBuild {
            keep_positions: true,
            start_delay_s: mapshipping::START_DELAY_S,
            group_delay_s: mapshipping::GROUP_DELAY_S,
            ..ReconBuild::default()
        },
    )
    .unwrap();
    let positions: Vec<_> = layout.spots.iter().map(|s| (s.x, s.z)).collect();
    let headings: Vec<_> = layout.spots.iter().map(|s| s.heading_deg).collect();
    recon::park_recon_copies_headed(&mut ships, &positions, &headings);
    duplicate::apply_overrides(&mut ships, "", 502);
    ships.set_name("Eastern Shipping"); // Existing Map output spelling.
    opts.ship_packs.push(MapShipPack { root: ships });

    // ui.rs::build_one_ground_pack, with explicit preview spots and objectives.
    let spots = [
        GroundSpot::at(
            180_000.0,
            240_000.0,
            true,
            180.0,
            GroundKind::Armor,
            Some((175_000.0, 240_000.0)),
        ),
        GroundSpot::at(
            190_000.0,
            250_000.0,
            true,
            135.0,
            GroundKind::Armor,
            Some((185_000.0, 255_000.0)),
        ),
    ];
    let mut ground = recon::generate_recon_ex(
        &[recon_input(ARMOR, 2)],
        ReconBuild {
            keep_positions: true,
            start_delay_s: crate::mapground::START_DELAY_S,
            group_delay_s: crate::mapground::GROUP_DELAY_S,
            ..ReconBuild::default()
        },
    )
    .unwrap();
    recon::park_recon_copies_spots(&mut ground, &spots);
    recon::snap_placed_attack_areas(&mut ground, &spots, &front, true);
    duplicate::apply_overrides(&mut ground, "", 502);
    ground.set_name("Eastern Ground");
    opts.ground_packs.push(MapGroundPack { root: ground });

    // ui.rs::build_loaded_army_packs: armies enter FrontOptions as ground/ship packs.
    let mut army = load(ARMY);
    let copies = recon::inspect_army_copies(&army);
    let spots = [GroundSpot::at(
        185_000.0,
        230_000.0,
        true,
        180.0,
        GroundKind::Artillery,
        Some((175_000.0, 230_000.0)),
    )];
    recon::park_army_mixed(&mut army, &copies, &[], &spots);
    recon::snap_army_placed_attack_areas(&mut army, &copies, &spots, &front, true);
    duplicate::apply_overrides(&mut army, "", 502);
    army.set_name("Eastern DPRK ML20 Arty");
    opts.ground_packs.push(MapGroundPack { root: army });

    let pack = frontlines::generate_front(&opts).unwrap();
    let mut fixture = Fixture::new("base_map.Group", pack.root, &[SHIPS, ARMOR, ARMY]);
    for ext in LANG_EXTS {
        fixture
            .locales
            .entry((*ext).into())
            .or_default()
            .overlay(pack.locale.clone());
    }
    fixture
}

fn fixtures() -> Vec<Fixture> {
    let recon = recon::generate_recon_ex(
        &[recon_input(ARMOR, 3), recon_input(SHIPS, 2)],
        ReconBuild {
            activate_percent: 50,
            start_delay_s: 5.0,
            ..ReconBuild::default()
        },
    )
    .unwrap();
    let mut fighter = pack::generate_pack(&configured_fighter_root(), 3).unwrap();
    duplicate::apply_overrides(&mut fighter, "", 601);
    fighter.set_name(&aircraft::linked_fighter_pack_name(601, 3));

    let plans: Vec<_> = bombers::extract_exclusive_plans(&load(EXCLUSIVE))
        .unwrap()
        .into_iter()
        .map(|root| {
            let info = bombers::inspect_plan(&root).unwrap();
            bombers::BomberInput {
                label: info.name,
                source_key: repo(EXCLUSIVE).to_string_lossy().into_owned(),
                root,
                trigger_zone_ids: info.suggested_triggers,
                completion_timer_id: info.suggested_completion.unwrap(),
            }
        })
        .collect();
    assert_eq!(plans.len(), 6);
    let exclusive = bombers::link_bomber_plans_with(&plans, false).unwrap();

    let mut airfield = load(AIRFIELD);
    let report =
        airfield::clean_airfield(&mut airfield, airfield::WESTERN_PLANE_COALITIONS).unwrap();
    assert!(report.stripped > 0 && report.unlinked_checkzones > 0);
    if airfield.block_type == "Group"
        && matches!(airfield.name(), Some("Group" | "Airfield") | None)
    {
        airfield.set_name(repo(AIRFIELD).file_stem().unwrap().to_str().unwrap());
    }
    vec![
        template_fixture(),
        Fixture::new("recon.Group", recon, &[ARMOR, SHIPS]),
        Fixture::new("fighter.Group", fighter, &[]), // Fighter Pack writes no sidecars.
        Fixture::new("exclusive.Group", exclusive, &[EXCLUSIVE]),
        Fixture::new("airfield_mp.Group", airfield, &[AIRFIELD]),
        base_map_fixture(),
    ]
}

pub(crate) fn baseline_dir() -> PathBuf {
    repo("src/testdata/p14_baseline")
}

/// Also checks sidecar presence in both directions. Offsets are zero-based;
/// a missing file or a strict prefix differs at the first missing byte.
pub(crate) fn assert_fixture_matches(actual: &Path) {
    let expected = baseline_dir().join(actual.file_name().unwrap());
    for ext in std::iter::once("Group").chain(LANG_EXTS.iter().copied()) {
        let actual = actual.with_extension(ext);
        let expected = expected.with_extension(ext);
        if ext != "Group" && !actual.exists() && !expected.exists() {
            continue;
        }
        let name = expected.file_name().unwrap().to_string_lossy();
        let read = |path: &Path| {
            std::fs::read(path).unwrap_or_else(|err| {
                panic!(
                    "{name}: first differing byte offset 0 ({}: {err})",
                    path.display()
                )
            })
        };
        let (a, b) = (read(&actual), read(&expected));
        if let Some(offset) = a
            .iter()
            .zip(&b)
            .position(|(a, b)| a != b)
            .or_else(|| (a.len() != b.len()).then_some(a.len().min(b.len())))
        {
            panic!(
                "{name}: first differing byte offset {offset} (actual {} bytes, baseline {} bytes)",
                a.len(),
                b.len()
            );
        }
    }
}

#[test]
#[ignore = "records P14 baselines; run only before Part B or with explicit approval"]
fn write_p14_baseline() {
    for fixture in fixtures() {
        fixture.write(&baseline_dir());
    }
}

#[test]
fn existing_outputs_match_baseline() {
    let dir = std::env::temp_dir().join(format!("il2_p14_baselines_{}", std::process::id()));
    for fixture in fixtures() {
        assert_fixture_matches(&fixture.write(&dir));
    }
}

#[test]
fn trace_off_changes_nothing() {
    let dir = std::env::temp_dir().join(format!("il2_p14_trace_off_{}", std::process::id()));
    for fixture in fixtures() {
        assert_fixture_matches(&fixture.write(&dir));
    }

    // Pin all 37 committed files, including the UI fixture exercised separately
    // by ui_tests::map_generate_without_air_matches_baseline in the full suite.
    let mut paths: Vec<_> = std::fs::read_dir(baseline_dir())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 37);
    let mut fingerprint = 0xcbf29ce484222325u64;
    for path in paths {
        let name = path.file_name().unwrap().to_str().unwrap();
        for byte in name.bytes().chain(std::fs::read(&path).unwrap()) {
            fingerprint = (fingerprint ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
    }
    assert_eq!(fingerprint, 0xac124eef53f1139f);

    fn check_source(dir: &Path) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                check_source(&path);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let file = path.file_name().unwrap().to_str().unwrap();
            if matches!(file, "trace.rs" | "probe.rs") || file.ends_with("_tests.rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            let tests = source.find("#[cfg(test)]\nmod tests").or_else(|| {
                source.find("#[cfg(test)]\r\nmod tests")
            });
            for (offset, _) in source.match_indices("instrument(") {
                assert!(
                    tests.is_some_and(|start| offset > start),
                    "unconditional instrumentation in {} at byte {offset}",
                    path.display()
                );
            }
        }
    }
    check_source(&repo("src"));
}
