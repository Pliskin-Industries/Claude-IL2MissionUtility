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
    Fixture::new(
        "template.Group",
        template::generate_template(&template_options()).unwrap(),
        &[],
    )
}

/// The Template tab inputs behind `template.Group` (also driven through the UI
/// by `ui_tests::trace_checkbox_off_output_matches_baseline`).
pub(crate) fn template_options() -> TemplateOptions {
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
    TemplateOptions {
        name: "P14 F-80C baseline".into(),
        waypoint_count: template::used_waypoint_count(&seats),
        waypoint_speed: model_spec::suggested_waypoint_speed_kmh(["f80c10"]),
        waypoint_altitude: 3050.0,
        zone_coalition: template::ZoneCoalition::Eastern,
        seats,
        ..TemplateOptions::default()
    }
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
    // Decision 0016: Fighter Pack death events target each aircraft's Delay Delete.
    // This baseline's Heavy Wedge stays Safe (1), per decision 0019.
    // Pairs fly Loose except under Cover (decisions 0020/0022).
    // Upstream v0.7.0 merge (2026-10-18): template.Group writes the Korea editor id
    // for Heavy Wedge (23 -> 8); exclusive.Group gains each plan's 3-hour WINDOW.
    assert_eq!(fingerprint, 0x1171aa00b3b1051c);

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
            if file == "ui.rs" && path.parent().is_some_and(|p| p.ends_with("src")) {
                // Step 9T: the one call is the header "Trace build" hook,
                // which returns before instrumenting while the box is off.
                let calls: Vec<_> = source.match_indices("instrument(").collect();
                assert_eq!(calls.len(), 1, "ui.rs: one trace hook only");
                let hook = source.find("fn trace_generated(").expect("ui.rs trace hook");
                let body = &source[hook..calls[0].0];
                assert!(
                    body.contains("if !self.trace_build {") && body.contains("return None;"),
                    "ui.rs: instrument( must sit behind the Trace build flag"
                );
                continue;
            }
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

/// Reuse the unchanged fixture's placed packs and original map settings. A
/// second stamp allocates the same indexes; the tests below pin every byte.
pub(crate) fn air_export_options() -> FrontOptions {
    let fixture = base_map_fixture();
    let named = |name| fixture.root.find_by_name(name).unwrap().clone();
    FrontOptions {
        battles: false,
        buildups: false,
        defenses: false,
        attacks: false,
        naval: false,
        user_attacks: vec![((140_000.0, 220_000.0), (130_000.0, 240_000.0))],
        fighter_packs: vec![MapFighterPack { root: named("NATO Fighters Wave 1 pack 1") }],
        ship_packs: vec![MapShipPack { root: named("Eastern Shipping") }],
        ground_packs: vec![
            MapGroundPack { root: named("Eastern Ground") },
            MapGroundPack { root: named("Eastern DPRK ML20 Arty") },
        ],
        ..FrontOptions::default()
    }
}

/// Match the UI's AirContext front for the selected timeline mark.
pub(crate) fn air_export_front(opts: &FrontOptions) -> Vec<(f64, f64)> {
    let raw = opts.custom_front.clone().filter(|line| !line.is_empty()).unwrap_or_else(|| {
        let i = opts.timeline_idx.unwrap_or_else(|| frontlines::timeline_index(opts.year, opts.season));
        frontlines::preview_front_xz(i as f32)
    });
    let dense = frontlines::densify(&raw, 4000.0);
    let (composite, _) = crate::mapclip::apply_salients(dense.clone(), &opts.salients);
    if composite.len() >= 2 { composite } else { dense }
}

pub(crate) mod air_export_tests {
    use super::*;
    use crate::airtask::{self, AirContext, AirPlan, AirSide, PoolEntry, TargetRef};

    fn check_baseline(opts: &FrontOptions, label: &str) {
        let pack = frontlines::generate_front(opts).unwrap();
        let mut fixture = Fixture::new("base_map.Group", pack.root, &[SHIPS, ARMOR, ARMY]);
        for ext in LANG_EXTS {
            fixture.locales.entry((*ext).into()).or_default().overlay(pack.locale.clone());
        }
        let dir = repo(&format!("target/p14/step7-{label}"));
        assert_fixture_matches(&fixture.write(&dir));
    }

    fn context<'a>(opts: &'a FrontOptions, front: &'a [(f64, f64)]) -> AirContext<'a> {
        AirContext {
            east_objectives: &[], nato_objectives: &[], arrows: &opts.user_attacks,
            front, aabb: opts.aabb, date: (1950, 9, 15),
        }
    }

    fn arrow_plan(ctx: &AirContext<'_>) -> AirPlan {
        let mut plan = AirPlan { library: airtask::builtin_library(), seed: 7, ..AirPlan::default() };
        let (tail, tip) = ctx.arrows[0];
        let side = if crate::mapclip::point_north_of_front(ctx.front, tail.0, tail.1) { AirSide::Dprk } else { AirSide::Nato };
        let sortie = plan.library.iter().find(|sortie| sortie.side == side).unwrap();
        plan.pool.push(PoolEntry::new(sortie.id.clone(), TargetRef::Arrow { tail, tip }));
        plan
    }

    fn same_imported_units(a: &frontlines::ImportedBaseMap, b: &frontlines::ImportedBaseMap) {
        assert_eq!(a.armies.len(), b.armies.len());
        assert_eq!(a.fighters.len(), b.fighters.len());
        for (a, b) in a.armies.iter().zip(&b.armies) {
            assert_eq!((&a.name, a.eastern, &a.entity), (&b.name, b.eastern, &b.entity));
        }
        for (a, b) in a.fighters.iter().zip(&b.fighters) {
            assert_eq!((&a.root, a.eastern, a.wave, &a.spots), (&b.root, b.eastern, b.wave, &b.spots));
        }
    }

    #[test]
    fn generate_front_with_empty_air_packs_matches_baseline() {
        let mut opts = air_export_options();
        opts.air_packs = Vec::new();
        check_baseline(&opts, "empty-air");
    }

    #[test]
    fn generate_front_with_disabled_air_plan_matches_baseline() {
        let mut opts = air_export_options();
        let line = air_export_front(&opts);
        for (i, kind) in ["disabled", "orphaned", "out-of-date"].into_iter().enumerate() {
            let mut ctx = context(&opts, &line);
            let mut plan = arrow_plan(&ctx);
            match i {
                0 => plan.pool[0].enabled = false,
                1 => ctx.arrows = &[],
                _ => ctx.date = (1900, 1, 1),
            }
            let packs = airtask::build_air_packs(&plan, &ctx).unwrap();
            assert!(packs.is_empty(), "{kind}");
            opts.air_packs = packs;
            check_baseline(&opts, kind);
        }
    }

    #[test]
    fn inspect_export_keeps_armies_and_flags_air() {
        let mut opts = air_export_options();
        let line = air_export_front(&opts);
        let ctx = context(&opts, &line);
        opts.air_packs = airtask::build_air_packs(&arrow_plan(&ctx), &ctx).unwrap();
        let root = frontlines::generate_front(&opts).unwrap().root;
        let loaded = frontlines::inspect_base_map(&root).unwrap();
        let baseline = frontlines::inspect_base_map(&load("src/testdata/p14_baseline/base_map.Group")).unwrap();
        assert!(loaded.air_tasking_found);
        same_imported_units(&loaded, &baseline);
        // Recognition takes precedence even over the fighter-pack signature.
        let mut root = root;
        let air = root.find_by_name_mut("Air Tasking").unwrap();
        for name in ["NodeGates", "Group 1"] {
            let mut child = Il2Entity::new("Group");
            child.set_name(name);
            air.children.push(child);
        }
        same_imported_units(&frontlines::inspect_base_map(&root).unwrap(), &baseline);
    }

    #[test]
    fn inspect_baseline_has_no_air_tasking() {
        let baseline = frontlines::inspect_base_map(&load("src/testdata/p14_baseline/base_map.Group")).unwrap();
        let current = frontlines::inspect_base_map(&base_map_fixture().root).unwrap();
        assert!(!baseline.air_tasking_found);
        assert!(!current.air_tasking_found);
        same_imported_units(&current, &baseline);
    }

    #[test]
    fn inspect_keeps_a_user_group_whose_name_only_starts_with_air_tasking() {
        let mut root = load("src/testdata/p14_baseline/base_map.Group");
        let baseline = frontlines::inspect_base_map(&root).unwrap();
        let mut user = root.find_by_name("Eastern Ground").unwrap().clone();
        user.set_name("Air Tasking user units");
        root.children.push(user.clone());
        let loaded = frontlines::inspect_base_map(&root).unwrap();
        assert!(!loaded.air_tasking_found);
        assert_eq!(loaded.armies.len(), baseline.armies.len() + 1);
        assert!(loaded.armies.iter().any(|army| army.entity == user));
    }

    pub(crate) fn check_load_then_generate(
        plan: &AirPlan,
        mut opts: FrontOptions,
        ctx: &AirContext<'_>,
    ) {
        opts.air_packs = airtask::build_air_packs(plan, ctx).unwrap();
        let first = frontlines::generate_front(&opts).unwrap().root;
        let loaded = frontlines::inspect_base_map(&first).unwrap();
        let baseline = frontlines::inspect_base_map(&load("src/testdata/p14_baseline/base_map.Group")).unwrap();
        same_imported_units(&loaded, &baseline);
        assert!(loaded.air_tasking_found);
        opts.aabb = loaded.aabb.unwrap();
        opts.custom_front = Some(loaded.front.clone());
        opts.salients = loaded.salients;
        opts.user_attacks = loaded.attack_arrows;
        opts.fighter_packs = loaded.fighters.into_iter().map(|p| MapFighterPack { root: p.root }).collect();
        opts.ship_packs.clear();
        opts.ground_packs = loaded.armies.into_iter().map(|a| MapGroundPack { root: a.entity }).collect();
        opts.ref_groups = loaded.refs;
        let front = air_export_front(&opts);
        // Keep the current plan's objectives. The UI clears them on Load;
        // restoring or orphaning those UI targets belongs to step 9.
        let ctx = AirContext {
            east_objectives: ctx.east_objectives,
            nato_objectives: ctx.nato_objectives,
            arrows: &opts.user_attacks,
            front: &front,
            aabb: opts.aabb,
            date: ctx.date,
        };
        // Arrow tips survive the Load round trip, so the arrow entry keeps
        // its target and the plan stays clean.
        let issues = airtask::validate(plan, &ctx);
        assert!(issues.is_empty(), "{issues:?}");
        opts.air_packs = airtask::build_air_packs(plan, &ctx).unwrap();
        let regenerated = frontlines::generate_front(&opts).unwrap().root;
        let air: Vec<_> = regenerated.children.iter().filter(|g| g.name() == Some("Air Tasking")).collect();
        assert_eq!(air.len(), 1);
        for side in ["DPRK", "NATO"] {
            let sorties = format!("Air Tasking {side} Sorties");
            assert!(air[0].children.iter().any(|g| g.name() == Some(sorties.as_str())), "missing {sorties}");
        }
        let reloaded = frontlines::inspect_base_map(&regenerated).unwrap();
        assert_eq!(reloaded.armies.len(), baseline.armies.len());
        assert_eq!(reloaded.fighters.len(), baseline.fighters.len());
    }
}
