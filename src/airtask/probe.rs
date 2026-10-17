//! Test-only generators for the P14 in-game probes (steps 0b/0e/0f/0g/0h/0i/0j/0k).
//! Flights 0, 1A, 1G, 1N and 1D were flown; their results are recorded in
//! `handoff/P14-probe-results.md`. Committed copies of the flown files are in
//! `handoff/p14-probes/`. Flight 1B was retired by decision 0010.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::sync::OnceLock;

use super::AiRtbDecision;
use crate::ast::Il2Entity;
use crate::locale::{P14_PROBE_LC_START, p14_probe_locale};
use crate::parser::parse_group_file;
use crate::serialize::serialize_group;
use crate::template::{
    BringUp, FlightRole, PlaneStart, TemplateOptions, TemplateSeat, UnitKind,
    apply_plane_start, attach_event, builtin_plane_catalog, bundled_catalog,
    checkzone, counter, generate_template, mcu, modifier_set_val, read_event, timer, timer_random,
};
use crate::trace::{self, ObjectiveStyle, TraceCarrier, TraceMap, TraceSelect, TraceSource};

const HVAR: &str =
    include_str!("../testdata/p14_probe_inputs/historical1950_pre0019/1950_US_F80_HVAR_Strike_4ship.Group");
const W_INPUT_DIR: &str = "src/testdata/p14_probe_inputs/flight1l";
const X_INPUT_DIR: &str = "src/testdata/p14_probe_inputs/flight1m";
const P_INPUT_DIR: &str = "src/testdata/p14_probe_inputs/flight1p";
const P_PACK_FILE: &str = "FighterPack_F80C_4ship.Group";
const P_FLIGHT: &str = "1P";
const P_PACK_AIRCRAFT: u32 = 4;
const P_PACK_SKILL: i32 = 2;
const P_PACK_DEATH_COUNT_RESET: &str = "1";
const P_PACK_RANDOM_PERCENT: &str = "100";
const U_INPUT_DIR: &str = "src/testdata/p14_probe_inputs/upstream_a31ad0a";
const U_NATO_STRIKE_FILE: &str = "Upstream_Strike_NATO_F80C_4ship.Group";
const U_DPRK_STRIKE_FILE: &str = "Upstream_Strike_DPRK_Il10_4ship.Group";
const U_CAP_FILE: &str = "Upstream_CAP_NATO_F86A_4ship.Group";
const COMPLEX_TRIGGER: &str =
    include_str!("../../TemplateExamples/Reference/ComplexTrigger_reference.Group");
const DAMAGE: &str = include_str!("../../TemplateExamples/Reference/CommandDamage_reference.Group");
const ATTACK_TARGET: &str =
    include_str!("../../TemplateExamples/CorrectedFlightfromTemplate.Group");
const BEHAVIOUR: &str = include_str!("../../TemplateExamples/DPRK_Scramble.Group");
const TWO_B_ENEMY: &str =
    include_str!("../testdata/p14_probe_inputs/historical1950_pre0019/1950_DPRK_Yak_AirfieldRaid_2plus2.Group");
// Assets and loadout from the first Plane in
// TemplateExamples/CorrectedFlightfromTemplate.Group (a MiG-15bis).
const MIG15BIS_PLANE_KEYS: [&str; 8] = [
    "Script", "Model", "Skin", "BotSkin", "PayloadId", "ModMask", "GunLoad", "GunBelt",
];
pub(crate) const K14_ORIGIN: (f64, f64) = (105_934.0, 269_477.0);

const COUNTRY_USA: i32 = 601; // NATO players and the built-in HVAR template.
const COUNTRY_NATO_AI: i32 = 602;
const COUNTRY_USSR: i32 = 501; // DPRK-side players.
const COUNTRY_DPRK: i32 = 503; // DPRK-side AI.
const COALITION_NATO: &str = "[2]";
const COALITION_DPRK: &str = "[1]";
const EVENT_OBJECT_ENTERED: i32 = 58;
const EVENT_OBJECT_LEFT: i32 = 60;
// template.rs EntityEvent names (the app's names, not assumed game semantics).
const EVENT_PILOT_KILLED: i32 = 0;
const EVENT_PILOT_WOUNDED: i32 = 1;
const EVENT_PLANE_CRASHED: i32 = 2;
const EVENT_PLANE_CRITICAL_DAMAGE: i32 = 3;
const EVENT_PLANE_DESTROYED: i32 = 4;
const EVENT_PLANE_LANDED: i32 = 5;
const EVENT_PLANE_TOOK_OFF: i32 = 6;
const EVENT_PLANE_BINGO_FUEL: i32 = 7;
const EVENT_PLANE_BINGO_MAIN_MG: i32 = 8;
const EVENT_PLANE_BINGO_BOMBS: i32 = 9;
const EVENT_PLANE_BINGO_TURRETS: i32 = 10;
const EVENT_PLANE_GUNNERS_KILLED: i32 = 11;
const EVENT_DAMAGED: i32 = 12;
const EVENT_KILLED: i32 = 13;
const AI_LEVEL_ACE: &str = "4";

// Node names written by template::generate_template, also retained in HVAR.
const TEMPLATE_SPAWN_UNITS: &str = "SPAWN UNITS";
const TEMPLATE_MISSION_BEGIN: &str = "MISSION BEGIN";
const TEMPLATE_MISSION_END: &str = "MISSION END";
const TEMPLATE_TRIGGER_DELETE: &str = "Trigger Delete";
const TEMPLATE_COOLDOWN: &str = "COOLDOWN";
const TEMPLATE_DEATH_COUNT: &str = "DeathCount";
const TEMPLATE_SPAWN_COUNT: &str = "SpawnCount";
const TEMPLATE_TRANSLATOR_MISSION_BEGIN: &str = "Translator Mission Begin";
const TEMPLATE_ZONE_IN: &str = "Zone IN";
const TEMPLATE_DELAYED_END_ORDERS: &str = "DELAYED END ORDERS";
const TEMPLATE_LOGIC: &str = "Logic";
const TEMPLATE_FORCE_COMPLETE_HIGH: &str = "Force Complete - High";
const STRIKE_HOME_TIMER: &str = "Goto WP 2";
const CAP_HOME_TIMER: &str = "Goto WP 5";
const HOME_ONCE_GATE: &str = "Go Home Once";
const PACK_RANDOMIZER_INPUT: &str = "Randomizer:INPUT";
const PACK_CLOSE_INPUT: &str = "CloseInput";
const PACK_WAIT_FOR_OUTPUT: &str = "Wait for Output 1000ms";
const PACK_RANDOM_FLIGHT: &str = "Random 1:100% 500ms";
const PACK_RANDOMIZER_OUTPUT: &str = "Out 1";
const PACK_SPAWN: &str = "Spawn 1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flight {
    Zero,
    OneA,
    OneN,
    OneD,
    OneG,
    OneH,
    OneJ,
    OneK,
    OneL,
    OneM,
    OneP,
    Two,
    TwoB,
    TwoC,
    TwoD,
    TwoE,
    TwoF,
    Load,
}

impl Flight {
    const ALL: [Self; 18] = [
        Self::Zero, Self::OneA, Self::OneN, Self::OneD, Self::OneG, Self::OneH, Self::OneJ,
        Self::OneK, Self::OneL, Self::OneM, Self::OneP, Self::Two, Self::TwoB, Self::TwoC, Self::TwoD, Self::TwoE, Self::TwoF, Self::Load,
    ];

    fn stem(self) -> &'static str {
        match self {
            Self::Zero => "P14_Probe_0",
            Self::OneA => "P14_Probe_1A",
            Self::OneN => "P14_Probe_1N",
            Self::OneD => "P14_Probe_1D",
            Self::OneG => "P14_Probe_1G",
            Self::OneH => "P14_Probe_1H",
            Self::OneJ => "P14_Probe_1J",
            Self::OneK => "P14_Probe_1K",
            Self::OneL => "P14_Probe_1L",
            Self::OneM => "P14_Probe_1M",
            Self::OneP => "P14_Probe_1P",
            Self::Two => "P14_Probe_2",
            Self::TwoB => "P14_Probe_2B",
            Self::TwoC => "P14_Probe_2C",
            Self::TwoD => "P14_Probe_2D",
            Self::TwoE => "P14_Probe_2E",
            Self::TwoF => "P14_Probe_2F",
            Self::Load => "P14_Load_Probe",
        }
    }

    fn sheet(self) -> &'static [Cell] {
        match self {
            Self::Zero => RUN_SHEET_0,
            Self::OneA => RUN_SHEET_1A,
            Self::OneN => RUN_SHEET_1N,
            Self::OneD => RUN_SHEET_1D,
            Self::OneG => RUN_SHEET_1G,
            Self::OneH => RUN_SHEET_1H,
            Self::OneJ => RUN_SHEET_1J,
            Self::OneK => RUN_SHEET_1K,
            Self::OneL => RUN_SHEET_1L,
            Self::OneM => RUN_SHEET_1M,
            Self::OneP => RUN_SHEET_1P,
            Self::Two => RUN_SHEET_2,
            Self::TwoB => RUN_SHEET_2B,
            Self::TwoC => RUN_SHEET_2C,
            Self::TwoD => RUN_SHEET_2D,
            Self::TwoE => RUN_SHEET_2E,
            Self::TwoF => RUN_SHEET_2F,
            Self::Load => &[],
        }
    }

    fn is_frozen(self) -> bool {
        // 0018 changed flight 2's shell. 0019 changes placement in 1M's Y1
        // and every sortie in 2B/2C/2D, even with their original Dense inputs.
        matches!(self, Self::OneM | Self::Two | Self::TwoB | Self::TwoC | Self::TwoD)
    }

    fn regenerates_flown_files(self) -> bool {
        match self {
            Self::TwoF | Self::Load => false, // Unflown; no committed artifacts to compare.
            _ => !self.is_frozen(),
        }
    }

    fn writes_schedule(self) -> bool {
        matches!(self, Self::TwoB | Self::TwoC | Self::TwoD | Self::TwoE | Self::TwoF)
    }

    fn empty_target_sorties(self) -> &'static [EmptyTargetSortie] {
        match self {
            Self::TwoC => TWO_C_SORTIES,
            Self::TwoD => TWO_D_SORTIES,
            Self::TwoE => TWO_E_SORTIES,
            Self::TwoF => TWO_F_SORTIES,
            _ => panic!("{}: no empty-target sortie rows", self.stem()),
        }
    }

    fn writes_load_summary(self) -> bool {
        matches!(self, Self::Load)
    }

    fn spawns(self) -> &'static [Spawn] {
        match self {
            Self::OneN => &[Spawn {
                name: "1N NATO",
                country: COUNTRY_USA,
                km: (-10.0, 30.0),
                heading_deg: 0.0,
                altitude_m: 3050.0,
                script: "f80c10",
            }],
            Self::OneD => &[Spawn {
                name: "1D USSR",
                country: COUNTRY_USSR,
                km: (40.0, -30.0),
                heading_deg: 0.0,
                altitude_m: 3050.0,
                script: "mig15bis",
            }],
            Self::TwoC => &[Spawn {
                name: "2C NATO",
                country: COUNTRY_USA,
                km: (130.0, -90.0),
                heading_deg: 0.0,
                altitude_m: 3050.0,
                script: "f80c10",
            }],
            Self::Zero | Self::OneA | Self::OneG | Self::OneH | Self::OneJ | Self::OneK
                | Self::OneL | Self::OneM | Self::OneP | Self::Two | Self::TwoB | Self::TwoD | Self::TwoE | Self::TwoF | Self::Load => &[],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Spawn {
    name: &'static str,
    country: i32,
    km: (f64, f64),
    heading_deg: f64,
    altitude_m: f64,
    script: &'static str,
}

fn spawn_json(flight: Flight) -> String {
    let entries = flight.spawns().iter().map(|spawn| {
        // Hand-written JSON is safe only for ASCII names/script ids without
        // quotes, backslashes or control characters, which would need escaping.
        for text in [spawn.name, spawn.script] {
            assert!(
                text.is_ascii() && !text.contains(['"', '\\'])
                    && !text.bytes().any(|byte| byte.is_ascii_control())
            );
        }
        let (x, z) = world(spawn.km);
        format!(
            "{{\"name\":\"{}\",\"country\":{},\"x\":{x},\"z\":{z},\"heading_deg\":{},\"altitude_m\":{},\"script\":\"{}\"}}",
            spawn.name, spawn.country, spawn.heading_deg, spawn.altitude_m, spawn.script
        )
    }).collect::<Vec<_>>();
    format!("[{}]\n", entries.join(","))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Cell {
    pub key: &'static str,
    pub centre_km: (f64, f64),
    pub arm_s: f64,
    pub close_s: f64,
}

pub(crate) const RUN_SHEET_0: &[Cell] = &[
    Cell {
        key: "T-a1",
        centre_km: (0.0, 60.0),
        arm_s: 5.0,
        close_s: 360.0,
    },
    Cell {
        key: "T-e",
        centre_km: (0.0, 5.0),
        arm_s: 60.0,
        close_s: 150.0,
    },
    Cell {
        key: "T-f",
        centre_km: (0.0, 5.0),
        arm_s: 150.0,
        close_s: 540.0,
    },
    Cell {
        key: "T-t",
        centre_km: (0.0, 5.0),
        arm_s: 270.0,
        close_s: 330.0,
    },
    Cell {
        key: "T-j",
        centre_km: (0.0, 5.0),
        arm_s: 540.0,
        close_s: 600.0,
    },
    Cell {
        key: "T-z",
        centre_km: (-40.0, -40.0),
        arm_s: 600.0,
        close_s: 630.0,
    },
];
pub(crate) const RUN_SHEET_1A: &[Cell] = &[
    Cell {
        key: "T-c",
        centre_km: (40.0, -20.0),
        arm_s: 600.0,
        close_s: 840.0,
    },
    Cell {
        key: "T-n",
        centre_km: (42.0, 30.0),
        arm_s: 840.0,
        close_s: 1560.0,
    },
];
pub(crate) const RUN_SHEET_1N: &[Cell] = &[
    Cell {
        key: "T-n2",
        centre_km: (42.0, 30.0),
        arm_s: 30.0,
        close_s: 1200.0,
    },
    Cell {
        key: "Y-N",
        centre_km: (100.0, 30.0),
        arm_s: 10.0,
        close_s: 1200.0,
    },
];
pub(crate) const RUN_SHEET_1D: &[Cell] = &[Cell {
    key: "Y-D",
    centre_km: (140.0, -30.0),
    arm_s: 10.0,
    close_s: 1200.0,
}];
pub(crate) const RUN_SHEET_1G: &[Cell] = &[Cell {
    key: "T-g2",
    centre_km: (130.0, 50.0),
    arm_s: 60.0,
    close_s: 900.0,
}];
pub(crate) const RUN_SHEET_1H: &[Cell] = &[Cell {
    key: "T-d",
    centre_km: (130.0, 20.0),
    arm_s: D_START_S,
    close_s: D_CLOSE_S,
}];
pub(crate) const RUN_SHEET_1J: &[Cell] = &[Cell {
    key: "T-d2",
    centre_km: (130.0, 20.0),
    arm_s: D_START_S,
    close_s: D_CLOSE_S,
}];
pub(crate) const RUN_SHEET_1K: &[Cell] = &[
    Cell {
        key: "T-d3",
        centre_km: (130.0, 20.0),
        arm_s: D_START_S,
        close_s: K_CLOSE_S,
    },
    Cell {
        key: "T-d4",
        centre_km: (60.0, 20.0),
        arm_s: D_START_S,
        close_s: K_CLOSE_S,
    },
];

pub(crate) const RUN_SHEET_1L: &[Cell] = &[
    Cell {
        key: "T-r",
        centre_km: (130.0, 20.0),
        arm_s: L_START_S,
        close_s: L_CLOSE_S,
    },
    Cell {
        key: "T-v",
        centre_km: (130.0, 52.0),
        arm_s: L_START_S,
        close_s: L_CLOSE_S,
    },
    Cell {
        key: "T-w",
        centre_km: (90.0, 20.0),
        arm_s: L_START_S,
        close_s: L_CLOSE_S,
    },
];

pub(crate) const RUN_SHEET_1M: &[Cell] = &[
    Cell {
        key: "T-u",
        centre_km: (130.0, 20.0),
        arm_s: M_START_S,
        close_s: M_CLOSE_S,
    },
    Cell {
        key: "T-x",
        centre_km: (10.0, 60.0),
        arm_s: M_START_S,
        close_s: M_CLOSE_S,
    },
    Cell {
        key: "T-y",
        centre_km: (50.0, 20.0),
        arm_s: M_START_S,
        close_s: M_CLOSE_S,
    },
    // The whole flight 0 cell, including its 09:00/10:00 window.
    Cell {
        key: "T-j",
        centre_km: (0.0, 5.0),
        arm_s: 540.0,
        close_s: 600.0,
    },
];

pub(crate) const RUN_SHEET_1P: &[Cell] = &[
    Cell {
        key: "T-h",
        centre_km: (50.0, 20.0),
        arm_s: P_START_S,
        close_s: P_CLOSE_S,
    },
    Cell {
        key: "T-f",
        centre_km: (90.0, 20.0),
        arm_s: P_START_S,
        close_s: P_CLOSE_S,
    },
    Cell {
        key: "T-b",
        centre_km: (10.0, 20.0),
        arm_s: P_START_S,
        close_s: P_CLOSE_S,
    },
];

const TWO_CLOSE_S: f64 = 2760.0;
const TWO_HOT_S: f64 = 600.0;
const TWO_TRANSIT_S: f64 = 300.0;
const TWO_MARGIN_S: f64 = 600.0;
const TWO_EXTENSION_S: f64 = 600.0;
const LOSS_ALTITUDE_M: f64 = 150.0;
const LOSS_CRUISE_KM: (f64, f64) = (-25.0, 60.0);
const SORTIE_PROBE_BUDGET: u8 = 1;
const SORTIE_PROBE_COPY_ORDER: usize = 1;
const SORTIE_PROBE_AREA: usize = 1;
const LOAD_PROBE_BUDGET: u8 = 2;

pub(crate) const RUN_SHEET_2: &[Cell] = &[
    Cell { key: "T-s1", centre_km: (50.0, 20.0), arm_s: 60.0, close_s: TWO_CLOSE_S },
    Cell { key: "T-s2", centre_km: (50.0, 60.0), arm_s: 90.0, close_s: TWO_CLOSE_S },
    Cell { key: "T-k", centre_km: (0.0, 5.0), arm_s: 180.0, close_s: 220.0 },
    Cell { key: "T-k1", centre_km: (0.0, 5.0), arm_s: 240.0, close_s: 280.0 },
    // Land in combined_terrain.bin; nearest airfield K-49 is 21.577 km away
    // (TemplateExamples/K49 DPRK_AB.Group; FullScene AIRFIELDS checked too).
    Cell { key: "T-l", centre_km: (-25.0, 50.0), arm_s: 60.0, close_s: TWO_CLOSE_S },
];

struct SortieProbe {
    cell: &'static str,
    file: &'static str,
    prefix: &'static str,
}
const TWO_SORTIES: &[SortieProbe] = &[
    SortieProbe { cell: "T-s1", file: "1950_US_F80_HVAR_Strike_4ship.Group", prefix: "SN01" },
    SortieProbe { cell: "T-s2", file: "1950_US_F80_AirAlert_4ship.Group", prefix: "SN02" },
];

const TWO_B_CLOSE_S: f64 = 1800.0;
// Measured from START in flight 2, not the 300 s placement distance.
const TWO_B_HVAR_TRANSIT_S: f64 = 286.0;
const TWO_B_AIRALERT_TRANSIT_S: f64 = 279.0;
const TWO_B_ENEMY_ALTITUDE_M: f64 = 8000.0;
const TWO_B_BEHAVIOUR_DELAY_S: f64 = 0.1;
const TWO_B_ORDERS_DELAY_S: f64 = 0.5;
const TWO_B_ENEMY_AIRCRAFT: usize = 2;
const TWO_B_ENEMY_ROTATION_DEG: f64 = 22.5;
const TWO_B_ORBIT_PRIORITY: &str = "2";
const TWO_B_TRACK_COALITION: i32 = 1;
const TWO_B_TRACK_OUTSIDE_OFFSET_M: f64 = 40_000.0;
// A regular octagon, radius 2 km, centred 20 km east of the target.
// Its 45-degree turns and 18 km inner waypoint leave room for AI tracking.
const TWO_B_ORBIT_OFFSETS_KM: &[(f64, f64)] = &[
    (0.0, 18.0), (std::f64::consts::SQRT_2, 20.0 - std::f64::consts::SQRT_2),
    (2.0, 20.0), (std::f64::consts::SQRT_2, 20.0 + std::f64::consts::SQRT_2),
    (0.0, 22.0), (-std::f64::consts::SQRT_2, 20.0 + std::f64::consts::SQRT_2),
    (-2.0, 20.0), (-std::f64::consts::SQRT_2, 20.0 - std::f64::consts::SQRT_2),
];

pub(crate) const RUN_SHEET_2B: &[Cell] = &[
    Cell { key: "T-s1", centre_km: (140.0, -90.0), arm_s: 60.0, close_s: TWO_B_CLOSE_S },
    Cell { key: "T-s2", centre_km: (140.0, 0.0), arm_s: 90.0, close_s: TWO_B_CLOSE_S },
    Cell { key: "T-s3", centre_km: (230.0, -90.0), arm_s: 120.0, close_s: TWO_B_CLOSE_S },
    Cell { key: "T-s4", centre_km: (230.0, 0.0), arm_s: 150.0, close_s: TWO_B_CLOSE_S },
];

struct EnemyWindow {
    activate_s: f64,
    delete_s: f64,
}
const TWO_B_PRESENT_ENEMY: EnemyWindow = EnemyWindow {
    activate_s: 30.0,
    delete_s: RUN_SHEET_2B[2].arm_s + TWO_B_HVAR_TRANSIT_S + 180.0,
};
const TWO_B_LATE_ENEMY: EnemyWindow = EnemyWindow {
    activate_s: RUN_SHEET_2B[3].arm_s + TWO_B_HVAR_TRANSIT_S + 120.0,
    delete_s: RUN_SHEET_2B[3].arm_s + TWO_B_HVAR_TRANSIT_S + 300.0,
};
struct SortieReflight {
    sortie: SortieProbe,
    measured_transit_s: f64,
    hot_at_s: Option<f64>,
    enemy: Option<EnemyWindow>,
}
const TWO_B_SORTIES: &[SortieReflight] = &[
    SortieReflight {
        sortie: SortieProbe { cell: "T-s1", file: "1950_US_F80_HVAR_Strike_4ship.Group", prefix: "SN01" },
        measured_transit_s: TWO_B_HVAR_TRANSIT_S, hot_at_s: Some(TWO_HOT_S), enemy: None,
    },
    SortieReflight {
        sortie: SortieProbe { cell: "T-s2", file: "1950_US_F80_AirAlert_4ship.Group", prefix: "SN02" },
        measured_transit_s: TWO_B_AIRALERT_TRANSIT_S, hot_at_s: None, enemy: None,
    },
    SortieReflight {
        sortie: SortieProbe { cell: "T-s3", file: "1950_US_F80_HVAR_Strike_4ship.Group", prefix: "SN03" },
        measured_transit_s: TWO_B_HVAR_TRANSIT_S, hot_at_s: None,
        enemy: Some(TWO_B_PRESENT_ENEMY),
    },
    SortieReflight {
        sortie: SortieProbe { cell: "T-s4", file: "1950_US_F80_HVAR_Strike_4ship.Group", prefix: "SN04" },
        measured_transit_s: TWO_B_HVAR_TRANSIT_S, hot_at_s: None,
        enemy: Some(TWO_B_LATE_ENEMY),
    },
];

const TWO_C_CLOSE_S: f64 = 1800.0;
const TWO_C_LEAVE_S: f64 = 900.0;
const TWO_C_LEAVE_CUE_S: f64 = 870.0;
const TWO_C_GROUND_TIME_S: f64 = 120.0;
const TWO_C_AIR_TIME_S: f64 = 600.0;
const TWO_C_AIR_RADIUS_M: f64 = 5000.0;

// Land checked with TerrainMap::builtin (combined_terrain.bin), as for T-l.
// Decision 0003: this checkout has no height grid; the mask has no elevations.
pub(crate) const RUN_SHEET_2C: &[Cell] = &[
    Cell { key: "C0a", centre_km: (140.0, -90.0), arm_s: 60.0, close_s: TWO_C_CLOSE_S },
    Cell { key: "C0b", centre_km: (185.0, 45.0), arm_s: 65.0, close_s: TWO_C_CLOSE_S },
    Cell { key: "C0c", centre_km: (230.0, 0.0), arm_s: 70.0, close_s: TWO_C_CLOSE_S },
    Cell { key: "C1a", centre_km: (140.0, -45.0), arm_s: 75.0, close_s: TWO_C_CLOSE_S },
    Cell { key: "C1b", centre_km: (185.0, -90.0), arm_s: 80.0, close_s: TWO_C_CLOSE_S },
    Cell { key: "C1c", centre_km: (230.0, 45.0), arm_s: 85.0, close_s: TWO_C_CLOSE_S },
    Cell { key: "C2a", centre_km: (140.0, 0.0), arm_s: 90.0, close_s: TWO_C_CLOSE_S },
    Cell { key: "C2b", centre_km: (185.0, -45.0), arm_s: 95.0, close_s: TWO_C_CLOSE_S },
    Cell { key: "C2c", centre_km: (230.0, -90.0), arm_s: 100.0, close_s: TWO_C_CLOSE_S },
    Cell { key: "C3a", centre_km: (140.0, 45.0), arm_s: 105.0, close_s: TWO_C_CLOSE_S },
    Cell { key: "C3b", centre_km: (185.0, 0.0), arm_s: 110.0, close_s: TWO_C_CLOSE_S },
    Cell { key: "C3c", centre_km: (230.0, -45.0), arm_s: 115.0, close_s: TWO_C_CLOSE_S },
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum EmptyTargetVariant {
    C0,
    C1,
    C2,
    C3,
    Formation(ProbeFormation),
    BuiltIn { file: &'static str, shape: super::library::TestedShape, attack_time_s: u32 },
    BuiltInFormation {
        file: &'static str,
        shape: super::library::TestedShape,
        attack_time_s: u32,
        measured_transit_s: f64,
        formation: ProbeFormation,
    },
}

impl EmptyTargetVariant {
    fn file(self) -> &'static str {
        match self {
            Self::C0 | Self::C1 | Self::C2 | Self::Formation(_) => "1950_US_F80_HVAR_Strike_4ship.Group",
            Self::C3 => "1950_US_F80_AirAlert_4ship.Group",
            Self::BuiltIn { file, .. } | Self::BuiltInFormation { file, .. } => file,
        }
    }

    fn measured_transit_s(self) -> f64 {
        match self {
            Self::C0 | Self::C1 | Self::C2 | Self::Formation(_) => TWO_B_HVAR_TRANSIT_S,
            Self::C3 => TWO_B_AIRALERT_TRANSIT_S,
            Self::BuiltIn { file, .. } => panic!("{file}: transit has not been measured"),
            Self::BuiltInFormation { measured_transit_s, .. } => measured_transit_s,
        }
    }
}

struct EmptyTargetSortie {
    cell: &'static str,
    prefix: &'static str,
    variant: EmptyTargetVariant,
}

impl EmptyTargetSortie {
    fn sortie(&self) -> SortieProbe {
        SortieProbe { cell: self.cell, prefix: self.prefix, file: self.variant.file() }
    }

    fn predicted_transit_s(&self) -> f64 {
        match self.variant {
            // Flight 2E uses the nominal placement distance, not an F-80 measurement.
            EmptyTargetVariant::BuiltIn { .. } => TWO_TRANSIT_S,
            _ => self.variant.measured_transit_s(),
        }
    }
}

const TWO_C_SORTIES: &[EmptyTargetSortie] = &[
    EmptyTargetSortie { cell: "C0a", prefix: "SN01", variant: EmptyTargetVariant::C0 },
    EmptyTargetSortie { cell: "C0b", prefix: "SN02", variant: EmptyTargetVariant::C0 },
    EmptyTargetSortie { cell: "C0c", prefix: "SN03", variant: EmptyTargetVariant::C0 },
    EmptyTargetSortie { cell: "C1a", prefix: "SN04", variant: EmptyTargetVariant::C1 },
    EmptyTargetSortie { cell: "C1b", prefix: "SN05", variant: EmptyTargetVariant::C1 },
    EmptyTargetSortie { cell: "C1c", prefix: "SN06", variant: EmptyTargetVariant::C1 },
    EmptyTargetSortie { cell: "C2a", prefix: "SN07", variant: EmptyTargetVariant::C2 },
    EmptyTargetSortie { cell: "C2b", prefix: "SN08", variant: EmptyTargetVariant::C2 },
    EmptyTargetSortie { cell: "C2c", prefix: "SN09", variant: EmptyTargetVariant::C2 },
    EmptyTargetSortie { cell: "C3a", prefix: "SN10", variant: EmptyTargetVariant::C3 },
    EmptyTargetSortie { cell: "C3b", prefix: "SN11", variant: EmptyTargetVariant::C3 },
    EmptyTargetSortie { cell: "C3c", prefix: "SN12", variant: EmptyTargetVariant::C3 },
];

const TWO_D_CLOSE_S: f64 = 1800.0;
const TWO_D_LEAVE_S: f64 = 900.0;
const TWO_D_LEAVE_CUE_S: f64 = 870.0;
const TWO_D_ATTACK_TIME_S: f64 = 600.0;

// Flight 2C targets C1b, C2c, C2b, C1c: similar measured settled heights.
pub(crate) const RUN_SHEET_2D: &[Cell] = &[
    Cell { key: "D0", centre_km: (185.0, -90.0), arm_s: 60.0, close_s: TWO_D_CLOSE_S },
    Cell { key: "D1", centre_km: (230.0, -90.0), arm_s: 120.0, close_s: TWO_D_CLOSE_S },
    Cell { key: "D2", centre_km: (185.0, -45.0), arm_s: 180.0, close_s: TWO_D_CLOSE_S },
    Cell { key: "DP", centre_km: (230.0, 45.0), arm_s: 240.0, close_s: TWO_D_CLOSE_S },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProbeFormation {
    formation_type: i32,
    formation_density: i32,
    formation_name: &'static str,
    editor_label: &'static str,
}

const TWO_D_SORTIES: &[EmptyTargetSortie] = &[
    EmptyTargetSortie { cell: "D0", prefix: "SN01", variant: EmptyTargetVariant::Formation(ProbeFormation {
        formation_type: 20, formation_density: 0, formation_name: "Wedge", editor_label: "Dense",
    }) },
    EmptyTargetSortie { cell: "D1", prefix: "SN02", variant: EmptyTargetVariant::Formation(ProbeFormation {
        formation_type: 20, formation_density: 1, formation_name: "Wedge", editor_label: "Safe",
    }) },
    EmptyTargetSortie { cell: "D2", prefix: "SN03", variant: EmptyTargetVariant::Formation(ProbeFormation {
        formation_type: 20, formation_density: 2, formation_name: "Wedge", editor_label: "Loose",
    }) },
    EmptyTargetSortie { cell: "DP", prefix: "SN04", variant: EmptyTargetVariant::Formation(ProbeFormation {
        formation_type: 19, formation_density: 0, formation_name: "Pairs", editor_label: "Dense",
    }) },
];

const TWO_E_CLOSE_S: f64 = 1800.0;
const TWO_E_LEAVE_S: f64 = 900.0;
const TWO_E_LEAVE_CUE_S: f64 = 870.0;

// Flight 2C targets C2c, C0c, C0b and C0a, unchanged. Opposing paths stay 90 km apart.
pub(crate) const RUN_SHEET_2E: &[Cell] = &[
    Cell { key: "EA", centre_km: (230.0, -90.0), arm_s: 60.0, close_s: TWO_E_CLOSE_S },
    Cell { key: "EY", centre_km: (230.0, 0.0), arm_s: 120.0, close_s: TWO_E_CLOSE_S },
    Cell { key: "EI", centre_km: (185.0, 45.0), arm_s: 180.0, close_s: TWO_E_CLOSE_S },
    Cell { key: "EC", centre_km: (140.0, -90.0), arm_s: 240.0, close_s: TWO_E_CLOSE_S },
];

const TWO_E_SORTIES: &[EmptyTargetSortie] = &[
    EmptyTargetSortie { cell: "EA", prefix: "SN01", variant: EmptyTargetVariant::BuiltIn {
        file: "1950_US_F51_ArmedRecon_2plus2.Group", shape: super::library::TestedShape::S3, attack_time_s: 1200,
    } },
    EmptyTargetSortie { cell: "EY", prefix: "SN02", variant: EmptyTargetVariant::BuiltIn {
        file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", shape: super::library::TestedShape::S4, attack_time_s: 300,
    } },
    EmptyTargetSortie { cell: "EI", prefix: "SN03", variant: EmptyTargetVariant::BuiltIn {
        file: "1950_DPRK_Il10_KimpoAttack_2x4.Group", shape: super::library::TestedShape::S4, attack_time_s: 600,
    } },
    EmptyTargetSortie { cell: "EC", prefix: "SN04", variant: EmptyTargetVariant::BuiltIn {
        file: "1950_US_F51_CloseSupport_4ship.Group", shape: super::library::TestedShape::S1, attack_time_s: 900,
    } },
];

const TWO_F_CLOSE_S: f64 = 2700.0;
const TWO_F_LEAVE_S: f64 = 2310.0;
const TWO_F_LEAVE_CUE_S: f64 = 2280.0;
// Flight 2E EY: WP 1 at 384.1 s for START 120 s, all three runs within 0.1 s.
const TWO_F_YAK_TRANSIT_S: f64 = 264.1;

struct ProbeTarget {
    name: &'static str,
    centre_km: (f64, f64),
}
// Unchanged flight 2C C0c (also 2E EY) and C2a. Alternate these same-line paths.
const TWO_F_TARGET_A: ProbeTarget = ProbeTarget { name: "A", centre_km: (230.0, 0.0) };
const TWO_F_TARGET_B: ProbeTarget = ProbeTarget { name: "B", centre_km: (140.0, 0.0) };

pub(crate) const RUN_SHEET_2F: &[Cell] = &[
    Cell { key: "PD", centre_km: TWO_F_TARGET_A.centre_km, arm_s: 60.0, close_s: TWO_F_CLOSE_S },
    Cell { key: "PS", centre_km: TWO_F_TARGET_B.centre_km, arm_s: 390.0, close_s: TWO_F_CLOSE_S },
    Cell { key: "PL", centre_km: TWO_F_TARGET_A.centre_km, arm_s: 720.0, close_s: TWO_F_CLOSE_S },
    Cell { key: "WD", centre_km: TWO_F_TARGET_B.centre_km, arm_s: 1050.0, close_s: TWO_F_CLOSE_S },
    Cell { key: "WS", centre_km: TWO_F_TARGET_A.centre_km, arm_s: 1380.0, close_s: TWO_F_CLOSE_S },
    Cell { key: "WL", centre_km: TWO_F_TARGET_B.centre_km, arm_s: 1710.0, close_s: TWO_F_CLOSE_S },
];

const TWO_F_SORTIES: &[EmptyTargetSortie] = &[
    EmptyTargetSortie { cell: "PD", prefix: "SN01", variant: EmptyTargetVariant::BuiltInFormation {
        file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", shape: super::library::TestedShape::S4, attack_time_s: 300,
        measured_transit_s: TWO_F_YAK_TRANSIT_S,
        formation: ProbeFormation { formation_type: 19, formation_density: 0, formation_name: "Pairs", editor_label: "Dense" },
    } },
    EmptyTargetSortie { cell: "PS", prefix: "SN02", variant: EmptyTargetVariant::BuiltInFormation {
        file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", shape: super::library::TestedShape::S4, attack_time_s: 300,
        measured_transit_s: TWO_F_YAK_TRANSIT_S,
        formation: ProbeFormation { formation_type: 19, formation_density: 1, formation_name: "Pairs", editor_label: "Safe" },
    } },
    EmptyTargetSortie { cell: "PL", prefix: "SN03", variant: EmptyTargetVariant::BuiltInFormation {
        file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", shape: super::library::TestedShape::S4, attack_time_s: 300,
        measured_transit_s: TWO_F_YAK_TRANSIT_S,
        formation: ProbeFormation { formation_type: 19, formation_density: 2, formation_name: "Pairs", editor_label: "Loose" },
    } },
    EmptyTargetSortie { cell: "WD", prefix: "SN04", variant: EmptyTargetVariant::BuiltInFormation {
        file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", shape: super::library::TestedShape::S4, attack_time_s: 300,
        measured_transit_s: TWO_F_YAK_TRANSIT_S,
        formation: ProbeFormation { formation_type: 20, formation_density: 0, formation_name: "Wedge", editor_label: "Dense" },
    } },
    EmptyTargetSortie { cell: "WS", prefix: "SN05", variant: EmptyTargetVariant::BuiltInFormation {
        file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", shape: super::library::TestedShape::S4, attack_time_s: 300,
        measured_transit_s: TWO_F_YAK_TRANSIT_S,
        formation: ProbeFormation { formation_type: 20, formation_density: 1, formation_name: "Wedge", editor_label: "Safe" },
    } },
    EmptyTargetSortie { cell: "WL", prefix: "SN06", variant: EmptyTargetVariant::BuiltInFormation {
        file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", shape: super::library::TestedShape::S4, attack_time_s: 300,
        measured_transit_s: TWO_F_YAK_TRANSIT_S,
        formation: ProbeFormation { formation_type: 20, formation_density: 2, formation_name: "Wedge", editor_label: "Loose" },
    } },
];

struct BudgetRequest {
    copy: usize,
    offset_s: f64,
}
struct BudgetDeletion {
    offset_s: f64,
    copies: &'static [usize],
}
struct BudgetProbe {
    cell: &'static str,
    limit: u8,
    copies: usize,
    requests: &'static [BudgetRequest],
    deletions: &'static [BudgetDeletion],
}
const TWO_BUDGETS: &[BudgetProbe] = &[
    BudgetProbe {
        cell: "T-k", limit: 2, copies: 7,
        requests: &[
            BudgetRequest { copy: 1, offset_s: 0.0 },
            BudgetRequest { copy: 2, offset_s: 2.0 },
            BudgetRequest { copy: 3, offset_s: 4.0 },
            BudgetRequest { copy: 4, offset_s: 6.0 },
            BudgetRequest { copy: 5, offset_s: 32.0 },
            BudgetRequest { copy: 6, offset_s: 34.0 },
            BudgetRequest { copy: 7, offset_s: 36.0 },
        ],
        deletions: &[BudgetDeletion { offset_s: 30.0, copies: &[1, 2] }],
    },
    BudgetProbe {
        cell: "T-k1", limit: 1, copies: 5,
        requests: &[
            BudgetRequest { copy: 1, offset_s: 0.0 },
            BudgetRequest { copy: 2, offset_s: 0.0 },
            BudgetRequest { copy: 3, offset_s: 5.0 },
            BudgetRequest { copy: 4, offset_s: 22.0 },
            BudgetRequest { copy: 5, offset_s: 32.0 },
        ],
        deletions: &[
            BudgetDeletion { offset_s: 20.0, copies: &[1] },
            BudgetDeletion { offset_s: 30.0, copies: &[2, 3, 4] },
        ],
    },
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Cue {
    pub flight: Flight,
    pub time_s: f64,
    pub text: &'static str,
}

pub(crate) const CUES: &[Cue] = &[
    Cue {
        flight: Flight::Zero,
        time_s: 0.0,
        text: "Flight 0: stay within 20 km of K14 until 11:00",
    },
    Cue {
        flight: Flight::Zero,
        time_s: 60.0,
        text: "T-e start",
    },
    Cue {
        flight: Flight::Zero,
        time_s: 150.0,
        text: "T-f F1 start",
    },
    Cue {
        flight: Flight::Zero,
        time_s: 270.0,
        text: "T-t start: note any objective message or map marker in the next 60 s",
    },
    Cue {
        flight: Flight::Zero,
        time_s: 330.0,
        text: "T-f F2 start",
    },
    Cue {
        flight: Flight::Zero,
        time_s: 540.0,
        text: "T-j start",
    },
    Cue {
        flight: Flight::Zero,
        time_s: 600.0,
        text: "T-z start",
    },
    Cue {
        flight: Flight::Zero,
        time_s: 660.0,
        text: "Flight 0 done. End the mission normally",
    },
    Cue {
        flight: Flight::OneA,
        time_s: 0.0,
        text: "T-c: take off, fly to the T-c C icon, orbit within 5 km at 3050 m by 10:00",
    },
    Cue {
        flight: Flight::OneA,
        time_s: 720.0,
        text: "T-c: fly east into C4 now",
    },
    Cue {
        flight: Flight::OneA,
        time_s: 840.0,
        text: "T-c done",
    },
    Cue {
        flight: Flight::OneA,
        time_s: 840.0,
        text: "T-n: fly to T-n START, then north through START and END, level at 3050 m, F-80C cruise setting",
    },
    Cue {
        flight: Flight::OneA,
        time_s: 1560.0,
        text: "Flight 1A done. End the mission normally",
    },
    Cue {
        flight: Flight::OneN,
        time_s: 0.0,
        text: "1N: join NATO at the 1N NATO spawn at 04:00 or later. Level flight on at once, hold north at 3050 m, F-80C cruise setting",
    },
    Cue {
        flight: Flight::OneN,
        time_s: 1200.0,
        text: "Flight 1N done. End the mission normally",
    },
    Cue {
        flight: Flight::OneD,
        time_s: 0.0,
        text: "1D: join the 1D USSR spawn at 04:00 or later. Level flight on at once, hold north at 3050 m",
    },
    Cue {
        flight: Flight::OneD,
        time_s: 1200.0,
        text: "Flight 1D done. End the mission normally",
    },
    Cue {
        flight: Flight::OneG,
        time_s: 0.0,
        text: "1G: no flying needed. Stay on the ground at K14 until 15:30",
    },
    Cue {
        flight: Flight::OneG,
        time_s: 930.0,
        text: "Flight 1G done. End the mission normally",
    },
    Cue {
        flight: Flight::OneH,
        time_s: 0.0,
        text: "1H: no flying needed. Stay on the ground at K14 until 05:30",
    },
    Cue {
        flight: Flight::OneH,
        time_s: D_DONE_S,
        text: "Flight 1H done. End the mission normally",
    },
    Cue {
        flight: Flight::OneJ,
        time_s: 0.0,
        text: "1J: no flying needed. Stay on the ground at K14 until 05:30",
    },
    Cue {
        flight: Flight::OneJ,
        time_s: D_DONE_S,
        text: "Flight 1J done. End the mission normally",
    },
    Cue {
        flight: Flight::OneK,
        time_s: 0.0,
        text: "1K: no flying needed. Stay on the ground at K14 until 16:00",
    },
    Cue {
        flight: Flight::OneK,
        time_s: K_DONE_S,
        text: "Flight 1K done. End the mission normally",
    },
    Cue {
        flight: Flight::OneL,
        time_s: 0.0,
        text: "1L: no flying needed. Stay on the ground at K14 until 24:00",
    },
    Cue {
        flight: Flight::OneL,
        time_s: L_DONE_S,
        text: "Flight 1L done. End the mission normally",
    },
    Cue {
        flight: Flight::OneM,
        time_s: 0.0,
        text: "1M: no flying needed. Stay on the ground at K14 until 14:00",
    },
    Cue {
        flight: Flight::OneM,
        time_s: M_DONE_S,
        text: "Flight 1M done. End the mission normally",
    },
    Cue {
        flight: Flight::OneP,
        time_s: 0.0,
        text: "1P: no flying needed. Stay on the ground at K14 until 14:00",
    },
    Cue {
        flight: Flight::OneP,
        time_s: P_DONE_S,
        text: "Flight 1P done. End the mission normally",
    },
    Cue {
        flight: Flight::Two, time_s: 0.0,
        text: "Flight 2: stay on the ground at K14 until 46:00. Do not damage the AI",
    },
    Cue { flight: Flight::Two, time_s: 180.0, text: "T-k: seven requests, budget 2" },
    Cue { flight: Flight::Two, time_s: 240.0, text: "T-k1: simultaneous requests, budget 1" },
    Cue { flight: Flight::Two, time_s: TWO_HOT_S, text: "T-s1 and T-s2 areas now HOT" },
    Cue { flight: Flight::Two, time_s: TWO_CLOSE_S, text: "Flight 2 closed. End the mission normally" },
    Cue { flight: Flight::TwoB, time_s: 0.0, text: "Flight 2B: stay at K14 until 30:00. Do not approach the four AI cells" },
    Cue { flight: Flight::TwoB, time_s: TWO_B_PRESENT_ENEMY.activate_s, text: "T-s3 enemy pair activated on its high altitude orbit" },
    Cue { flight: Flight::TwoB, time_s: RUN_SHEET_2B[0].arm_s, text: "T-s1 HVAR starts. Empty target, HOT at 10:00, expect one extension" },
    Cue { flight: Flight::TwoB, time_s: RUN_SHEET_2B[1].arm_s, text: "T-s2 AirAlert starts. Empty target stays COLD, expect 1080 s on station" },
    Cue { flight: Flight::TwoB, time_s: RUN_SHEET_2B[2].arm_s, text: "T-s3 HVAR starts. Enemy already inside, delete at 09:46" },
    Cue { flight: Flight::TwoB, time_s: RUN_SHEET_2B[3].arm_s, text: "T-s4 HVAR starts. Enemy appears at 09:16, delete at 12:16" },
    Cue { flight: Flight::TwoB, time_s: TWO_B_LATE_ENEMY.activate_s, text: "T-s4 enemy pair activated. Expect Zone IN now" },
    Cue { flight: Flight::TwoB, time_s: TWO_B_PRESENT_ENEMY.delete_s, text: "T-s3 enemy pair deleted. Expect Zone Out then DONE HUB" },
    Cue { flight: Flight::TwoB, time_s: TWO_HOT_S, text: "T-s1 area now HOT. T-s2, T-s3 and T-s4 stay COLD" },
    Cue { flight: Flight::TwoB, time_s: TWO_B_LATE_ENEMY.delete_s, text: "T-s4 enemy pair deleted. Expect Zone Out then DONE HUB" },
    Cue { flight: Flight::TwoB, time_s: TWO_B_CLOSE_S, text: "Flight 2B closed. Every aircraft has a Delete. End the mission normally" },
    Cue { flight: Flight::TwoC, time_s: 0.0, text: "Flight 2C: spectate if you can. Camera on C0a, then LCtrl+R to record. Leave the server at 15:00" },
    Cue { flight: Flight::TwoC, time_s: TWO_C_LEAVE_CUE_S, text: "Flight 2C: leave the server now (15:00)" },
    Cue { flight: Flight::TwoC, time_s: TWO_C_CLOSE_S, text: "Flight 2C closed. Every aircraft has a Delete. End the mission normally" },
    Cue { flight: Flight::TwoD, time_s: 0.0, text: "Flight 2D: spectate, then LCtrl+R to record. Note the first time the server overload message shows. Leave the server at 15:00" },
    Cue { flight: Flight::TwoD, time_s: RUN_SHEET_2D[0].arm_s, text: "Flight 2D: D0 starts (4 aircraft flying)" },
    Cue { flight: Flight::TwoD, time_s: RUN_SHEET_2D[1].arm_s, text: "Flight 2D: D1 starts (8 aircraft flying)" },
    Cue { flight: Flight::TwoD, time_s: RUN_SHEET_2D[2].arm_s, text: "Flight 2D: D2 starts (12 aircraft flying)" },
    Cue { flight: Flight::TwoD, time_s: RUN_SHEET_2D[3].arm_s, text: "Flight 2D: DP starts (16 aircraft flying)" },
    Cue { flight: Flight::TwoD, time_s: TWO_D_LEAVE_CUE_S, text: "Flight 2D: leave the server now (15:00)" },
    Cue { flight: Flight::TwoD, time_s: TWO_D_CLOSE_S, text: "Flight 2D closed. Every aircraft has a Delete. End the mission normally" },
    Cue { flight: Flight::TwoE, time_s: 0.0, text: "Flight 2E: spectate, then LCtrl+R to record. Note the first time the server overload message shows. Leave the server at 15:00" },
    Cue { flight: Flight::TwoE, time_s: RUN_SHEET_2E[0].arm_s, text: "Flight 2E: EA starts (4 aircraft flying)" },
    Cue { flight: Flight::TwoE, time_s: RUN_SHEET_2E[1].arm_s, text: "Flight 2E: EY starts (8 aircraft flying)" },
    Cue { flight: Flight::TwoE, time_s: RUN_SHEET_2E[2].arm_s, text: "Flight 2E: EI starts (16 aircraft flying)" },
    Cue { flight: Flight::TwoE, time_s: RUN_SHEET_2E[3].arm_s, text: "Flight 2E: EC starts (20 aircraft flying)" },
    Cue { flight: Flight::TwoE, time_s: TWO_E_LEAVE_CUE_S, text: "Flight 2E: leave the server now (15:00)" },
    Cue { flight: Flight::TwoE, time_s: TWO_E_CLOSE_S, text: "Flight 2E closed. Every aircraft has a Delete. End the mission normally" },
    Cue { flight: Flight::TwoF, time_s: 0.0, text: "Flight 2F: spectate, then LCtrl+R to record. Check the menu now and then for the server overload message. Leave the server at 38:30" },
    Cue { flight: Flight::TwoF, time_s: RUN_SHEET_2F[0].arm_s, text: "Flight 2F: PD starts (Pairs, Dense)" },
    Cue { flight: Flight::TwoF, time_s: RUN_SHEET_2F[1].arm_s, text: "Flight 2F: PS starts (Pairs, Safe)" },
    Cue { flight: Flight::TwoF, time_s: RUN_SHEET_2F[2].arm_s, text: "Flight 2F: PL starts (Pairs, Loose)" },
    Cue { flight: Flight::TwoF, time_s: RUN_SHEET_2F[3].arm_s, text: "Flight 2F: WD starts (Wedge, Dense)" },
    Cue { flight: Flight::TwoF, time_s: RUN_SHEET_2F[4].arm_s, text: "Flight 2F: WS starts (Wedge, Safe)" },
    Cue { flight: Flight::TwoF, time_s: RUN_SHEET_2F[5].arm_s, text: "Flight 2F: WL starts (Wedge, Loose)" },
    Cue { flight: Flight::TwoF, time_s: TWO_F_LEAVE_CUE_S, text: "Flight 2F: leave the server at 38:30" },
    Cue { flight: Flight::TwoF, time_s: TWO_F_CLOSE_S, text: "Flight 2F closed. Every aircraft has a Delete. End the mission normally" },
];

const A1_START_KM: (f64, f64) = (-30.0, 60.0);
const A1_EXIT_KM: (f64, f64) = (30.0, 60.0);
const A1_AI_START_S: f64 = 10.0;
const AI_VS_PLAYER_RADIUS_M: f64 = 12_000.0;
const C4_KM: (f64, f64) = (40.0, -8.0);
const C_ZONE_RADIUS_M: f64 = 5000.0;
const C_ACTIVATE_S: f64 = 660.0;
const C_PULSE_AFTER_ACTIVATE_S: f64 = 0.1;
const C2_RECHECK_OFF_S: f64 = 670.0;
const C2_RECHECK_ON_S: f64 = 675.0;
const C4_WAITING_ON_S: f64 = 690.0;
const C4_WAITING_OFF_S: f64 = 705.0;
const C5_REPEAT_PULSE_S: f64 = 630.0;
const N_START_KM: (f64, f64) = (30.0, 30.0);
const N_END_KM: (f64, f64) = (54.0, 30.0);
const START_SOUTH_KM: f64 = 10.0;
const HUNTER_BEHIND_START_KM: f64 = 5.0;
const HUNTER_FIRST_ATTACK_S: f64 = 90.0;
const HUNTER_ATTACK_STEP_S: f64 = 90.0;
const G_START_S: f64 = 60.0; // 01:00.
const G_DAMAGE_S: f64 = 240.0; // 04:00.
const G_REMOVE_S: f64 = 600.0; // 10:00.
const G_HUNTER_ATTACK_COUNT: usize = 4; // The first four orders of the shared rule.
const D_START_S: f64 = 60.0;
const D_KILL_S: f64 = 120.0;
const D_REMOVE_S: f64 = 240.0;
const D_DETECT_S: f64 = 270.0;
const D_DONE_S: f64 = 330.0;
const D_CLOSE_S: f64 = 480.0;
const J_PAIR_HALF_SPACING_KM: f64 = 1.0;
const K_CHANGE_S: f64 = D_KILL_S; // T-d3 changes share 1H/1J's 02:00 step.
const K_FIGHT_REMOVE_S: f64 = 900.0;
const K_FIGHT_DETECT_S: f64 = 930.0;
const K_DONE_S: f64 = 960.0;
const K_CLOSE_S: f64 = 1200.0;
// K9 and K10: The 14:30 order is off the 90 s step: the build prompt asked for it
// and the flight was flown with it (handoff/P14-air-tasking.md, step 0h).
const K_HUNTER_EXTRA_ATTACK_S: f64 = 870.0;
const L_START_S: f64 = 60.0;
const L_DAMAGE_S: f64 = 120.0;
const R_REMOVE_S: f64 = 300.0;
const R_DETECT_S: f64 = 330.0;
const W_CHANGE_S: f64 = 180.0;
const W_RESTART_S: f64 = 240.0;
const W_DETECT_S: f64 = 360.0;
const W_COOLDOWN_S: f64 = 60.0;
const W_ALTITUDE_M: f32 = 3050.0;
const L_DONE_S: f64 = 1440.0;
const L_CLOSE_S: f64 = 1560.0;
const M_START_S: f64 = 60.0;
const M_DONE_S: f64 = 840.0;
const M_CLOSE_S: f64 = 960.0;
const P_START_S: f64 = 60.0;
const P_KILL_S: f64 = 180.0;
const P_DONE_S: f64 = 840.0;
const P_CLOSE_S: f64 = 960.0;
const X_CHANGE_S: f64 = 180.0;
const X_RESTART_S: f64 = 240.0;
const X_DETECT_S: f64 = 360.0;
const X_COOLDOWN_MIN: f32 = 1.0;
const X_ALTITUDE_M: f32 = 3050.0;
const Y_DAMAGE_S: f64 = 120.0;
const Y_REMOVE_S: f64 = 300.0;
const Y_DETECT_S: f64 = 330.0;
const Y_HEADING_DEG: f64 = 0.0;
const Y_SPAWN_DISTANCE_M: f64 = 10_000.0;
const FOUR_SHIP_SUFFIXES: [char; 4] = ['a', 'b', 'c', 'd'];
const V_LOW_FUEL: &str = "0.03";
const V_EVENT_TYPES: [i32; 14] = [
    EVENT_PILOT_KILLED,
    EVENT_PILOT_WOUNDED,
    EVENT_PLANE_CRASHED,
    EVENT_PLANE_CRITICAL_DAMAGE,
    EVENT_PLANE_DESTROYED,
    EVENT_PLANE_LANDED,
    EVENT_PLANE_TOOK_OFF,
    EVENT_PLANE_BINGO_FUEL,
    EVENT_PLANE_BINGO_MAIN_MG,
    EVENT_PLANE_BINGO_BOMBS,
    EVENT_PLANE_BINGO_TURRETS,
    EVENT_PLANE_GUNNERS_KILLED,
    EVENT_DAMAGED,
    EVENT_KILLED,
];
const G_EVENT_TYPES: [i32; 5] = [
    EVENT_PILOT_KILLED,
    EVENT_PLANE_CRASHED,
    EVENT_PLANE_DESTROYED,
    EVENT_PLANE_LANDED,
    EVENT_KILLED,
];
const B_EVENT_TYPES: [i32; 5] = [
    EVENT_PLANE_BINGO_FUEL, EVENT_PLANE_BINGO_MAIN_MG, EVENT_PLANE_BINGO_BOMBS,
    EVENT_KILLED, EVENT_PLANE_LANDED,
];
const E_SIX_PULSES: [f64; 6] = [60.0, 62.0, 64.0, 66.0, 68.0, 70.0];
const E3_PULSES: [f64; 3] = [60.0, 62.0, 66.0];
const E_PARTIAL_PULSES: [f64; 3] = [60.0, 62.0, 64.0];
const E9_PULSES: [f64; 3] = [120.0, 120.05, 120.10];
const E_PULSE_PERIOD_S: f64 = 2.0;
const E3_DEACTIVATE_S: f64 = 63.0;
const E3_ACTIVATE_S: f64 = 65.0;
const E4_RESET_S: f64 = 63.0;
const E5_ACTIVATE_S: f64 = 63.0;
const E6_RESET_S: f64 = 65.0;
const E7_SAME_TICK_S: f64 = 100.0;
const E8_ONCE_GATE_S: f64 = 110.0;
const F_PULSE_COUNT: usize = 40;
const F1_PULSE_PERIOD_S: f64 = 3.0;
const F2_START_S: f64 = 330.0;
const F2_PULSE_PERIOD_S: f64 = 5.0;
const F2_RANDOM_DELAY_STEP_S: f64 = 0.5;
const T1_SINGLE_PULSE_S: f64 = 275.0;
const T2_PULSES: [f64; 5] = [277.0, 279.0, 281.0, 283.0, 285.0];
const T3_SAME_TICK_S: f64 = 287.0;
const T5_PULSES: [f64; 2] = [315.0, 320.0];
const T7_SAME_TICK_S: f64 = 325.0;

struct CounterProbeRow {
    name: &'static str,
    threshold: i32,
    dropcount: i32,
    repeat_readout: bool,
}

const E_COUNTERS: [CounterProbeRow; 9] = [
    CounterProbeRow {
        name: "E1",
        threshold: 3,
        dropcount: 0,
        repeat_readout: true,
    },
    CounterProbeRow {
        name: "E2",
        threshold: 3,
        dropcount: 1,
        repeat_readout: true,
    },
    CounterProbeRow {
        name: "E3",
        threshold: 3,
        dropcount: 0,
        repeat_readout: false,
    },
    CounterProbeRow {
        name: "E4",
        threshold: 3,
        dropcount: 0,
        repeat_readout: false,
    },
    CounterProbeRow {
        name: "E5",
        threshold: 3,
        dropcount: 0,
        repeat_readout: false,
    },
    CounterProbeRow {
        name: "E6",
        threshold: 3,
        dropcount: 0,
        repeat_readout: true,
    },
    CounterProbeRow {
        name: "E7",
        threshold: 3,
        dropcount: 1,
        repeat_readout: false,
    },
    CounterProbeRow {
        name: "E8",
        threshold: 1,
        dropcount: 0,
        repeat_readout: true,
    },
    CounterProbeRow {
        name: "E9",
        threshold: 3,
        dropcount: 1,
        repeat_readout: false,
    },
];

enum EventProbeAction {
    ShotDownByHunter,
    ProbeDelete,
    TemplateCleanup,
    DamageCommand,
}

struct EventProbeRow {
    name: &'static str,
    centre_km: Option<(f64, f64)>, // None uses the current cell's centre.
    action: EventProbeAction,
}

const G_FLIGHTS: &[EventProbeRow] = &[
    EventProbeRow {
        name: "G1",
        centre_km: Some((130.0, 80.0)),
        action: EventProbeAction::ShotDownByHunter,
    },
    EventProbeRow {
        name: "G2",
        centre_km: None,
        action: EventProbeAction::ProbeDelete,
    },
    EventProbeRow {
        name: "G3",
        centre_km: Some((130.0, 53.0)),
        action: EventProbeAction::TemplateCleanup,
    },
    EventProbeRow {
        name: "G4",
        centre_km: Some((130.0, 110.0)),
        action: EventProbeAction::DamageCommand,
    },
];

#[derive(Clone, Copy)]
enum Seat {
    Lead,
    Wingman,
}

impl Seat {
    fn index(self) -> usize {
        match self {
            Self::Lead => 0,
            Self::Wingman => 1,
        }
    }
}

enum DeleteRemoval {
    Nothing,
    DeleteBothEntities,
    DeleteEachEntity,
    DeleteLeadEntity,
    TemplateCleanup,
    DeletePlaneIndexes,
    DeactivateOnly,
}

struct DeleteProbeRow {
    name: &'static str,
    centre_km: (f64, f64),
    killed: Option<Seat>,
    removal: DeleteRemoval,
    detector: bool,
}

// Step 0f: killed at 02:00, removed at 04:00, detector at 04:30.
const D_FLIGHTS: &[DeleteProbeRow] = &[
    DeleteProbeRow {
        name: "D0",
        centre_km: (130.0, 20.0),
        killed: None,
        removal: DeleteRemoval::Nothing,
        detector: false,
    },
    DeleteProbeRow {
        name: "D1",
        centre_km: (130.0, 28.0),
        killed: None,
        removal: DeleteRemoval::DeleteBothEntities,
        detector: true,
    },
    DeleteProbeRow {
        name: "D2",
        centre_km: (130.0, 36.0),
        killed: Some(Seat::Wingman),
        removal: DeleteRemoval::DeleteBothEntities,
        detector: true,
    },
    DeleteProbeRow {
        name: "D3",
        centre_km: (130.0, 44.0),
        killed: Some(Seat::Lead),
        removal: DeleteRemoval::DeleteBothEntities,
        detector: true,
    },
    DeleteProbeRow {
        name: "D4",
        centre_km: (130.0, 52.0),
        killed: Some(Seat::Wingman),
        removal: DeleteRemoval::DeleteEachEntity,
        detector: true,
    },
    DeleteProbeRow {
        name: "D5",
        centre_km: (130.0, 60.0),
        killed: Some(Seat::Wingman),
        removal: DeleteRemoval::DeleteLeadEntity,
        detector: true,
    },
    DeleteProbeRow {
        name: "D6",
        centre_km: (130.0, 68.0),
        killed: Some(Seat::Wingman),
        removal: DeleteRemoval::TemplateCleanup,
        detector: true,
    },
    DeleteProbeRow {
        name: "D7",
        centre_km: (130.0, 76.0),
        killed: None,
        removal: DeleteRemoval::Nothing,
        detector: true,
    },
    DeleteProbeRow {
        name: "D8",
        centre_km: (114.0, 20.0),
        killed: None,
        removal: DeleteRemoval::DeletePlaneIndexes,
        detector: true,
    },
    DeleteProbeRow {
        name: "D9",
        centre_km: (114.0, 28.0),
        killed: None,
        removal: DeleteRemoval::DeactivateOnly,
        detector: true,
    },
];

#[derive(Clone, Copy)]
enum PairAircraft {
    X,
    Y,
}

impl PairAircraft {
    const BOTH: [Self; 2] = [Self::X, Self::Y];

    fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
        }
    }

    fn suffix(self) -> char {
        match self {
            Self::X => 'x',
            Self::Y => 'y',
        }
    }

    fn east_offset_km(self) -> f64 {
        match self {
            Self::X => -J_PAIR_HALF_SPACING_KM,
            Self::Y => J_PAIR_HALF_SPACING_KM,
        }
    }
}

// Each aircraft leads its own flight, so this cell has no lead/wingman removal.
enum DeletePairRemoval {
    Nothing,
    DeleteBothEntities,
    DeleteEachEntity,
    DeleteXEntity,
}

struct DeletePairRow {
    name: &'static str,
    centre_km: (f64, f64),
    killed: Option<PairAircraft>,
    removal: DeletePairRemoval,
    detector: bool,
}

// Step 0g: two independent 1-ship flights per pair, timed as step 0f.
const J_PAIRS: &[DeletePairRow] = &[
    DeletePairRow {
        name: "J0",
        centre_km: (130.0, 20.0),
        killed: None,
        removal: DeletePairRemoval::Nothing,
        detector: false,
    },
    DeletePairRow {
        name: "J1",
        centre_km: (130.0, 28.0),
        killed: None,
        removal: DeletePairRemoval::DeleteBothEntities,
        detector: true,
    },
    DeletePairRow {
        name: "J2",
        centre_km: (130.0, 36.0),
        killed: Some(PairAircraft::Y),
        removal: DeletePairRemoval::DeleteBothEntities,
        detector: true,
    },
    DeletePairRow {
        name: "J3",
        centre_km: (130.0, 44.0),
        killed: Some(PairAircraft::X),
        removal: DeletePairRemoval::DeleteBothEntities,
        detector: true,
    },
    DeletePairRow {
        name: "J4",
        centre_km: (130.0, 52.0),
        killed: Some(PairAircraft::Y),
        removal: DeletePairRemoval::DeleteEachEntity,
        detector: true,
    },
    DeletePairRow {
        name: "J5",
        centre_km: (130.0, 60.0),
        killed: Some(PairAircraft::Y),
        removal: DeletePairRemoval::DeleteXEntity,
        detector: true,
    },
    DeletePairRow {
        name: "J6",
        centre_km: (130.0, 68.0),
        killed: Some(PairAircraft::Y),
        removal: DeletePairRemoval::Nothing,
        detector: true,
    },
];

#[derive(Clone, Copy)]
enum DamageLevel {
    Level0,
    Level1,
    Complete,
}

impl DamageLevel {
    fn property(self) -> &'static str {
        match self {
            Self::Level0 => "0",
            Self::Level1 => "1",
            Self::Complete => "2",
        }
    }
}

enum FormationChange {
    Nothing,
    DeleteWingmanEntity,
    DamageLead(DamageLevel),
}

#[derive(Clone, Copy)]
enum FormationRemoval {
    Nothing,
    DeleteLeadEntity,
    DeleteWingmanEntity,
    DeleteBothEntities,
    DeleteEachEntity,
    DeactivateBothEntities,
}

struct FormationDeleteRow {
    name: &'static str,
    centre_km: (f64, f64),
    change: FormationChange,
    removal: FormationRemoval,
    detector: bool,
}

// Step 0h, T-d3: change at 02:00, removal at 04:00, detector at 04:30.
const K_FORMATIONS: &[FormationDeleteRow] = &[
    FormationDeleteRow {
        name: "K0",
        centre_km: (130.0, 20.0),
        change: FormationChange::Nothing,
        removal: FormationRemoval::Nothing,
        detector: false,
    },
    FormationDeleteRow {
        name: "K1",
        centre_km: (130.0, 28.0),
        change: FormationChange::Nothing,
        removal: FormationRemoval::DeleteLeadEntity,
        detector: true,
    },
    FormationDeleteRow {
        name: "K2",
        centre_km: (130.0, 36.0),
        change: FormationChange::Nothing,
        removal: FormationRemoval::DeleteWingmanEntity,
        detector: true,
    },
    FormationDeleteRow {
        name: "K3",
        centre_km: (130.0, 44.0),
        change: FormationChange::DeleteWingmanEntity,
        removal: FormationRemoval::DeleteBothEntities,
        detector: true,
    },
    FormationDeleteRow {
        name: "K4",
        centre_km: (130.0, 52.0),
        change: FormationChange::DamageLead(DamageLevel::Level1),
        removal: FormationRemoval::DeleteBothEntities,
        detector: true,
    },
    FormationDeleteRow {
        name: "K5",
        centre_km: (130.0, 60.0),
        change: FormationChange::DamageLead(DamageLevel::Level0),
        removal: FormationRemoval::DeleteBothEntities,
        detector: true,
    },
    FormationDeleteRow {
        name: "K6",
        centre_km: (130.0, 68.0),
        change: FormationChange::DamageLead(DamageLevel::Level1),
        removal: FormationRemoval::Nothing,
        detector: false,
    },
    FormationDeleteRow {
        name: "K7",
        centre_km: (130.0, 76.0),
        change: FormationChange::DamageLead(DamageLevel::Level0),
        removal: FormationRemoval::Nothing,
        detector: false,
    },
];

struct FightDeleteRow {
    name: &'static str,
    centre_km: (f64, f64),
    delete_s: f64,
    removal: FormationRemoval,
    detector_s: f64,
}

// Step 0h, T-d4: hunter orders follow the shared rule, strictly before Delete,
// plus K_HUNTER_EXTRA_ATTACK_S for K9 and K10.
const K_FIGHTS: &[FightDeleteRow] = &[
    FightDeleteRow {
        name: "K8",
        centre_km: (60.0, 20.0),
        delete_s: D_REMOVE_S,
        removal: FormationRemoval::DeleteBothEntities,
        detector_s: D_DETECT_S,
    },
    FightDeleteRow {
        name: "K9",
        centre_km: (60.0, 60.0),
        delete_s: K_FIGHT_REMOVE_S,
        removal: FormationRemoval::DeleteBothEntities,
        detector_s: K_FIGHT_DETECT_S,
    },
    FightDeleteRow {
        name: "K10",
        centre_km: (60.0, 100.0),
        delete_s: K_FIGHT_REMOVE_S,
        removal: FormationRemoval::DeleteEachEntity,
        detector_s: K_FIGHT_DETECT_S,
    },
];

struct RtbProbeRow {
    name: &'static str,
    centre_km: (f64, f64),
    ai_rtb_decision: AiRtbDecision,
    damage: DamageLevel,
    removal: FormationRemoval,
    detector: bool,
}

// Step 0i, T-r: damage the lead at 02:00, remove at 05:00, detect at 05:30.
const R_FORMATIONS: &[RtbProbeRow] = &[
    RtbProbeRow {
        name: "R0",
        centre_km: (130.0, 20.0),
        ai_rtb_decision: AiRtbDecision::Enabled,
        damage: DamageLevel::Level1,
        removal: FormationRemoval::DeleteBothEntities,
        detector: true,
    },
    RtbProbeRow {
        name: "R1",
        centre_km: (130.0, 28.0),
        ai_rtb_decision: AiRtbDecision::Disabled,
        damage: DamageLevel::Level1,
        removal: FormationRemoval::DeleteBothEntities,
        detector: true,
    },
    RtbProbeRow {
        name: "R2",
        centre_km: (130.0, 36.0),
        ai_rtb_decision: AiRtbDecision::Disabled,
        damage: DamageLevel::Level1,
        removal: FormationRemoval::Nothing,
        detector: false,
    },
    RtbProbeRow {
        name: "R3",
        centre_km: (130.0, 44.0),
        ai_rtb_decision: AiRtbDecision::Enabled,
        damage: DamageLevel::Level1,
        removal: FormationRemoval::DeactivateBothEntities,
        detector: false,
    },
];

enum AllEventsAction {
    Damage(DamageLevel),
    LowFuel,
}

struct AllEventsRow {
    name: &'static str,
    centre_km: (f64, f64),
    action: AllEventsAction,
}

// Step 0i, T-v: each independent aircraft reports every event type 0 to 13.
const V_AIRCRAFT: &[AllEventsRow] = &[
    AllEventsRow {
        name: "V1",
        centre_km: (130.0, 52.0),
        action: AllEventsAction::Damage(DamageLevel::Complete),
    },
    AllEventsRow {
        name: "V2",
        centre_km: (130.0, 60.0),
        action: AllEventsAction::LowFuel,
    },
    AllEventsRow {
        name: "V3",
        centre_km: (130.0, 68.0),
        action: AllEventsAction::Damage(DamageLevel::Level1),
    },
];

enum TemplateChange {
    Cooldown60Seconds,
    DeathEventKilled,
    DeleteListsEntities,
}

enum TemplateSchedule {
    KillAndDetect,
    CleanupAndRestart,
}

struct TemplateProbeRow {
    name: &'static str,
    centre_km: (f64, f64),
    bring_up: BringUp,
    allow_multiple_spawns: bool,
    changes_after_generation: &'static [TemplateChange],
    schedule: TemplateSchedule,
}

// Step 0i, T-w: frozen generate_template output first; event/Delete/cooldown
// edits are applied only after loading, preserving every generated node and other link.
// W1 leaves its generated OnPlaneDestroyed event untouched.
const W_GROUPS: &[TemplateProbeRow] = &[
    TemplateProbeRow {
        name: "W1",
        centre_km: (90.0, 20.0),
        bring_up: BringUp::Spawn,
        allow_multiple_spawns: true,
        changes_after_generation: &[TemplateChange::Cooldown60Seconds],
        schedule: TemplateSchedule::KillAndDetect,
    },
    TemplateProbeRow {
        name: "W2",
        centre_km: (90.0, 28.0),
        bring_up: BringUp::Spawn,
        allow_multiple_spawns: true,
        changes_after_generation: &[
            TemplateChange::Cooldown60Seconds,
            TemplateChange::DeathEventKilled,
        ],
        schedule: TemplateSchedule::KillAndDetect,
    },
    TemplateProbeRow {
        name: "W3",
        centre_km: (90.0, 36.0),
        bring_up: BringUp::Spawn,
        allow_multiple_spawns: true,
        changes_after_generation: &[
            TemplateChange::Cooldown60Seconds,
            TemplateChange::DeathEventKilled,
        ],
        schedule: TemplateSchedule::CleanupAndRestart,
    },
    TemplateProbeRow {
        name: "W4",
        centre_km: (90.0, 44.0),
        bring_up: BringUp::Spawn,
        allow_multiple_spawns: true,
        changes_after_generation: &[
            TemplateChange::Cooldown60Seconds,
            TemplateChange::DeathEventKilled,
            TemplateChange::DeleteListsEntities,
        ],
        schedule: TemplateSchedule::CleanupAndRestart,
    },
    TemplateProbeRow {
        name: "W5",
        centre_km: (90.0, 52.0),
        bring_up: BringUp::Activate,
        allow_multiple_spawns: false,
        changes_after_generation: &[],
        schedule: TemplateSchedule::CleanupAndRestart,
    },
    TemplateProbeRow {
        name: "W6",
        centre_km: (90.0, 60.0),
        bring_up: BringUp::Activate,
        allow_multiple_spawns: false,
        changes_after_generation: &[TemplateChange::DeleteListsEntities],
        schedule: TemplateSchedule::CleanupAndRestart,
    },
];

enum UpstreamOrders {
    Strike,
    Cap,
}

impl UpstreamOrders {
    fn home_timer(&self) -> &'static str {
        match self {
            Self::Strike => STRIKE_HOME_TIMER,
            Self::Cap => CAP_HOME_TIMER,
        }
    }

    fn abort_events(&self) -> [i32; 4] {
        match self {
            Self::Strike => [EVENT_PLANE_BINGO_BOMBS, EVENT_PLANE_CRITICAL_DAMAGE,
                EVENT_PILOT_WOUNDED, EVENT_PLANE_BINGO_FUEL],
            Self::Cap => [EVENT_PLANE_BINGO_MAIN_MG, EVENT_PLANE_BINGO_FUEL,
                EVENT_PLANE_CRITICAL_DAMAGE, EVENT_PILOT_WOUNDED],
        }
    }

    fn event_types(&self) -> Vec<i32> {
        self.abort_events().into_iter().chain([EVENT_KILLED, EVENT_PLANE_LANDED]).collect()
    }

    fn timers(&self) -> &'static [&'static str] {
        match self {
            Self::Strike => &["Formation 1", "Goto WP 1", "AttackArea 1", "OnAreaAttacked 1",
                STRIKE_HOME_TIMER, "Mission Complete 1", TEMPLATE_MISSION_END,
                TEMPLATE_DELAYED_END_ORDERS],
            Self::Cap => &["Goto WP 1", "Goto WP 2", "Goto WP 3", "Return to WP 1", CAP_HOME_TIMER],
        }
    }
}

enum UpstreamChange {
    None,
    RemoveEvent7AbortHooks,
    OnceOnlyAbortGate,
    RemoveHomeForceComplete,
    RemoveEvent7AbortHooksAndGate,
}

impl UpstreamChange {
    fn removes_event7_hooks(&self) -> bool {
        matches!(self, Self::RemoveEvent7AbortHooks | Self::RemoveEvent7AbortHooksAndGate)
    }

    fn gates_aborts(&self) -> bool {
        matches!(self, Self::OnceOnlyAbortGate | Self::RemoveEvent7AbortHooksAndGate)
    }

    fn removes_home_force_complete(&self) -> bool {
        matches!(self, Self::RemoveHomeForceComplete)
    }
}

struct UpstreamProbeRow {
    name: &'static str,
    centre_km: (f64, f64),
    source_file: &'static str,
    orders: UpstreamOrders,
    change: UpstreamChange,
}

const U_GROUPS: &[UpstreamProbeRow] = &[
    UpstreamProbeRow {
        name: "U1",
        centre_km: (130.0, 20.0),
        source_file: U_NATO_STRIKE_FILE,
        orders: UpstreamOrders::Strike,
        change: UpstreamChange::None,
    },
    UpstreamProbeRow {
        name: "U2",
        centre_km: (50.0, 100.0),
        source_file: U_DPRK_STRIKE_FILE,
        orders: UpstreamOrders::Strike,
        change: UpstreamChange::None,
    },
    UpstreamProbeRow {
        name: "U3",
        centre_km: (130.0, 60.0),
        source_file: U_NATO_STRIKE_FILE,
        orders: UpstreamOrders::Strike,
        change: UpstreamChange::RemoveEvent7AbortHooks,
    },
    UpstreamProbeRow {
        name: "U4",
        centre_km: (90.0, 20.0),
        source_file: U_CAP_FILE,
        orders: UpstreamOrders::Cap,
        change: UpstreamChange::None,
    },
    UpstreamProbeRow {
        name: "U5",
        centre_km: (90.0, 60.0),
        source_file: U_CAP_FILE,
        orders: UpstreamOrders::Cap,
        change: UpstreamChange::RemoveEvent7AbortHooks,
    },
];

const H_GROUPS: &[UpstreamProbeRow] = &[
    UpstreamProbeRow {
        name: "H1",
        centre_km: (50.0, 20.0),
        source_file: U_DPRK_STRIKE_FILE,
        orders: UpstreamOrders::Strike,
        change: UpstreamChange::None,
    },
    UpstreamProbeRow {
        name: "H2",
        centre_km: (50.0, 60.0),
        source_file: U_DPRK_STRIKE_FILE,
        orders: UpstreamOrders::Strike,
        change: UpstreamChange::OnceOnlyAbortGate,
    },
    UpstreamProbeRow {
        name: "H3",
        centre_km: (50.0, 100.0),
        source_file: U_DPRK_STRIKE_FILE,
        orders: UpstreamOrders::Strike,
        change: UpstreamChange::RemoveHomeForceComplete,
    },
    UpstreamProbeRow {
        name: "H4",
        centre_km: (130.0, 20.0),
        source_file: U_NATO_STRIKE_FILE,
        orders: UpstreamOrders::Strike,
        change: UpstreamChange::RemoveEvent7AbortHooksAndGate,
    },
];

enum FighterCountChange {
    None,
    RemoveEvent7Hooks,
    RemoveEvent7HooksAndCountDeathsDirectly,
}

impl FighterCountChange {
    fn removes_event7_hooks(&self) -> bool {
        matches!(self, Self::RemoveEvent7Hooks | Self::RemoveEvent7HooksAndCountDeathsDirectly)
    }

    fn counts_deaths_directly(&self) -> bool {
        matches!(self, Self::RemoveEvent7HooksAndCountDeathsDirectly)
    }
}

struct FighterCountRow {
    name: &'static str,
    centre_km: (f64, f64),
    change: FighterCountChange,
}

// One frozen four-aircraft flight, three copies. All seats are Independent:
// each row's 03:00 timer fires four separate entity-linked Damage commands.
const P_FIGHTERS: &[FighterCountRow] = &[
    FighterCountRow {
        name: "F1",
        centre_km: (90.0, 20.0),
        change: FighterCountChange::None,
    },
    FighterCountRow {
        name: "F2",
        centre_km: (90.0, 60.0),
        change: FighterCountChange::RemoveEvent7Hooks,
    },
    FighterCountRow {
        name: "F3",
        centre_km: (90.0, 100.0),
        change: FighterCountChange::RemoveEvent7HooksAndCountDeathsDirectly,
    },
];

struct BingoAircraftRow {
    name: &'static str,
    centre_km: (f64, f64),
    script: &'static str,
}

// Template Builder's bundled catalog, selected by script, excluding simpleb29.
const B_AIRCRAFT: &[BingoAircraftRow] = &[
    BingoAircraftRow { name: "B1", centre_km: (10.0, 20.0), script: "f80c10" },
    BingoAircraftRow { name: "B2", centre_km: (10.0, 28.0), script: "f84e" },
    BingoAircraftRow { name: "B3", centre_km: (10.0, 36.0), script: "f86a5" },
    BingoAircraftRow { name: "B4", centre_km: (10.0, 44.0), script: "f51d" },
    BingoAircraftRow { name: "B5", centre_km: (10.0, 52.0), script: "b29" },
    BingoAircraftRow { name: "B6", centre_km: (10.0, 60.0), script: "c47b" },
    BingoAircraftRow { name: "B7", centre_km: (10.0, 68.0), script: "mig15bis" },
    BingoAircraftRow { name: "B8", centre_km: (10.0, 76.0), script: "la11" },
    BingoAircraftRow { name: "B9", centre_km: (10.0, 84.0), script: "yak9p" },
    BingoAircraftRow { name: "B10", centre_km: (10.0, 92.0), script: "il10" },
    BingoAircraftRow { name: "B11", centre_km: (10.0, 100.0), script: "tu2" },
    BingoAircraftRow { name: "B12", centre_km: (10.0, 108.0), script: "li2t" },
];

struct GeneratedProbeRow {
    name: &'static str,
    centre_km: (f64, f64),
    bring_up: BringUp,
    allow_multiple_spawns: bool,
    schedule: TemplateSchedule,
}

const X_GROUPS: &[GeneratedProbeRow] = &[
    GeneratedProbeRow {
        name: "X1",
        centre_km: (10.0, 60.0),
        bring_up: BringUp::Spawn,
        allow_multiple_spawns: true,
        schedule: TemplateSchedule::KillAndDetect,
    },
    GeneratedProbeRow {
        name: "X2",
        centre_km: (10.0, 68.0),
        bring_up: BringUp::Spawn,
        allow_multiple_spawns: true,
        schedule: TemplateSchedule::CleanupAndRestart,
    },
    GeneratedProbeRow {
        name: "X3",
        centre_km: (10.0, 76.0),
        bring_up: BringUp::Activate,
        allow_multiple_spawns: false,
        schedule: TemplateSchedule::CleanupAndRestart,
    },
];

enum RtbFlightBuilder {
    PlaceAirSortie,
    ProbeFlight,
}

struct LargerRtbRow {
    name: &'static str,
    centre_km: (f64, f64),
    builder: RtbFlightBuilder,
    aircraft: &'static [&'static str],
    damage: DamageLevel,
}

const Y_FLIGHTS: &[LargerRtbRow] = &[
    LargerRtbRow {
        name: "Y1",
        centre_km: (50.0, 20.0),
        builder: RtbFlightBuilder::PlaceAirSortie,
        aircraft: &["Y1a", "Y1b", "Y1c", "Y1d"],
        damage: DamageLevel::Level1,
    },
    LargerRtbRow {
        name: "Y2",
        centre_km: (50.0, 36.0),
        builder: RtbFlightBuilder::ProbeFlight,
        aircraft: &["Y2a", "Y2b", "Y2c", "Y2d"],
        damage: DamageLevel::Level1,
    },
    LargerRtbRow {
        name: "Y3",
        centre_km: (50.0, 52.0),
        builder: RtbFlightBuilder::ProbeFlight,
        aircraft: &["Y3"],
        damage: DamageLevel::Level1,
    },
];

fn world(km: (f64, f64)) -> (f64, f64) {
    (K14_ORIGIN.0 + km.0 * 1000.0, K14_ORIGIN.1 + km.1 * 1000.0)
}

fn group(name: &str, next_id: &mut i32) -> Il2Entity {
    let mut node = Il2Entity::new("Group");
    node.index = Some(*next_id);
    node.set_property("Index", next_id.to_string());
    *next_id += 1;
    node.set_name(name);
    node.set_property("Desc", "\"\"");
    node
}

fn by_id(root: &Il2Entity, id: i32) -> &Il2Entity {
    fn find(root: &Il2Entity, id: i32) -> Option<&Il2Entity> {
        if root.index == Some(id) {
            Some(root)
        } else {
            root.children.iter().find_map(|c| find(c, id))
        }
    }
    find(root, id).unwrap_or_else(|| panic!("missing probe Index {id}"))
}

fn by_id_mut(root: &mut Il2Entity, id: i32) -> &mut Il2Entity {
    fn find(root: &mut Il2Entity, id: i32) -> Option<&mut Il2Entity> {
        if root.index == Some(id) {
            Some(root)
        } else {
            root.children.iter_mut().find_map(|c| find(c, id))
        }
    }
    find(root, id).unwrap_or_else(|| panic!("missing probe Index {id}"))
}

fn named<'a>(root: &'a Il2Entity, name: &str) -> &'a Il2Entity {
    root.find_by_name(name).unwrap_or_else(|| {
        panic!("probe group {}: missing node {name}", root.name().unwrap_or("<unnamed>"))
    })
}

fn id(root: &Il2Entity, name: &str) -> i32 {
    named(root, name).index.unwrap_or_else(|| {
        panic!("probe group {}: node {name} missing Index", root.name().unwrap_or("<unnamed>"))
    })
}

fn block_ids(root: &Il2Entity, block: &str) -> Vec<i32> {
    let mut ids = Vec::new();
    root.for_each(&mut |node| {
        if node.block_type == block {
            ids.push(node.index.unwrap());
        }
    });
    ids
}

fn single_aircraft(flight: &Il2Entity, label: &str) -> (i32, i32) {
    let planes = block_ids(flight, "Plane");
    let entities = block_ids(flight, "MCU_TR_Entity");
    let ([plane], [entity]) = (planes.as_slice(), entities.as_slice()) else {
        panic!("{label}: expected exactly one Plane and one MCU_TR_Entity");
    };
    (*plane, *entity)
}

/// Parse each MCU reference once, retaining its original ordered properties.
fn reference_block(block: &str) -> &'static Il2Entity {
    static COMPLEX_TRIGGER_NODE: OnceLock<Il2Entity> = OnceLock::new();
    static DAMAGE_NODE: OnceLock<Il2Entity> = OnceLock::new();
    static ATTACK_TARGET_NODE: OnceLock<Il2Entity> = OnceLock::new();
    static BEHAVIOUR_NODE: OnceLock<Il2Entity> = OnceLock::new();
    let (src, cached, expected_count) = match block {
        "MCU_TR_ComplexTrigger" => (COMPLEX_TRIGGER, &COMPLEX_TRIGGER_NODE, 1),
        "MCU_CMD_Damage" => (DAMAGE, &DAMAGE_NODE, 1),
        "MCU_CMD_AttackTarget" => (ATTACK_TARGET, &ATTACK_TARGET_NODE, 1),
        // DPRK_Scramble has one identical command per formation.
        "MCU_CMD_Behaviour" => (BEHAVIOUR, &BEHAVIOUR_NODE, 2),
        _ => panic!("unknown probe reference {block}"),
    };
    cached.get_or_init(|| {
        let reference = crate::parser::parse_il2_document(src).expect("probe MCU reference parses");
        let ids = block_ids(&reference, block);
        assert_eq!(ids.len(), expected_count, "probe reference {block}: expected {expected_count} copies");
        let first = by_id(&reference, ids[0]);
        for &other in &ids[1..] {
            for (key, value) in &first.properties {
                // Identity, links and editor placement may differ per copy.
                if matches!(key.as_str(), "Index" | "Name" | "Desc" | "Targets" | "Objects"
                    | "XPos" | "YPos" | "ZPos" | "XOri" | "YOri" | "ZOri") {
                    continue;
                }
                assert_eq!(by_id(&reference, other).property(key), Some(value.as_str()),
                    "probe reference {block}: copies disagree on {key}");
            }
        }
        first.clone()
    })
}

fn wingman_letter(index: usize) -> char {
    if index == 0 { 'a' } else { 'b' }
}

fn hunter_name_and_start(name: &str, target_start_km: (f64, f64)) -> (String, (f64, f64)) {
    (
        format!("{name} HUNTER"),
        (target_start_km.0 - HUNTER_BEHIND_START_KM, target_start_km.1),
    )
}

fn hunter_attack_times_before(delete_s: f64) -> impl Iterator<Item = f64> {
    std::iter::successors(Some(HUNTER_FIRST_ATTACK_S), |time_s| {
        Some(time_s + HUNTER_ATTACK_STEP_S)
    })
    .take_while(move |&time_s| time_s < delete_s)
}

/// Remove nodes and their links, including event/report links if a future HVAR
/// fixture gains any. Use the AST/nom parser, never text surgery on a .Group.
fn remove_ids(root: &mut Il2Entity, removed: &HashSet<i32>) {
    root.children.retain(|child| {
        !child.index.is_some_and(|i| removed.contains(&i))
            && !["TarId", "CmdId"].iter().any(|key| {
                child
                    .property(key)
                    .and_then(|v| v.parse().ok())
                    .is_some_and(|i| removed.contains(&i))
            })
    });
    if root.property("Targets").is_some() {
        root.set_targets(
            root.targets
                .iter()
                .copied()
                .filter(|i| !removed.contains(i))
                .collect(),
        );
    }
    if root.property("Objects").is_some() {
        root.set_objects(
            root.objects
                .iter()
                .copied()
                .filter(|i| !removed.contains(i))
                .collect(),
        );
    }
    for child in &mut root.children {
        remove_ids(child, removed);
    }
}

/// 0b steps 1-5. The caller does step 6: an absolute start timer into MISSION
/// BEGIN, and a cell-close Delete targeting the retained aircraft entities.
pub(crate) fn probe_flight(
    src: &str,
    keep_planes: usize,
    start: (f64, f64),
    exit: (f64, f64),
    next_id: &mut i32,
) -> Il2Entity {
    let mut copy = parse_group_file(src).expect("the probe's HVAR source parses");
    let planes = block_ids(&copy, "Plane");
    assert!((1..=planes.len()).contains(&keep_planes));
    let lead_xz = by_id(&copy, planes[0]).pos_xz().unwrap();
    let mut removed = HashSet::new();
    for &plane in &planes[keep_planes..] {
        removed.insert(plane);
        removed.insert(
            by_id(&copy, plane)
                .property("LinkTrId")
                .unwrap()
                .parse()
                .unwrap(),
        );
    }
    remove_ids(&mut copy, &removed);
    let (mut copy, _) = crate::duplicate::duplicate_template(&copy, next_id);
    crate::placement::move_anchor_to(&mut copy, lead_xz, start);
    crate::mapnet::park_path_waypoints(&mut copy, &[exit]);
    crate::weapon_range::snap_ground_attack_areas(&mut copy, exit.0, exit.1);
    cut_probe_flight_logic(&mut copy);
    copy
}

fn cut_probe_flight_logic(copy: &mut Il2Entity) {
    crate::recon::silence_clone_starts(copy);
    let cut = [
        "ENABLE / PULSE IN",
        "Zone IN",
        "Self Deactivate",
        "Zone In ReActivate",
        TEMPLATE_COOLDOWN,
        "Mission Complete 1",
    ];
    let mut removed = HashSet::new();
    copy.for_each(&mut |node| {
        if node.name().is_some_and(|name| cut.contains(&name)) {
            removed.insert(node.index.unwrap());
        }
    });
    assert_eq!(
        removed.len(),
        7,
        "HVAR start/end wiring changed; review 0b's cuts"
    );
    remove_ids(copy, &removed);
}

struct Probe {
    root: Il2Entity,
    next_id: i32,
    next_lc: i32,
    begin: i32,
    init_off: i32,
    cell: Option<Cell>,
    arm: i32,
    close_off: i32,
    sources: Vec<TraceSource>,
    hand_sources: HashSet<i32>,
    map: TraceMap,
}

impl Probe {
    fn new(flight: Flight) -> Self {
        let mut next_id = 1;
        let mut root = group(flight.stem(), &mut next_id);
        let mut begin = mcu(
            "MCU_TR_MissionBegin",
            TEMPLATE_TRANSLATOR_MISSION_BEGIN,
            &mut next_id,
            K14_ORIGIN.0,
            K14_ORIGIN.1,
        );
        begin.set_property("Enabled", "1");
        let mut init = timer("PROBE INIT", 0.5, &mut next_id, K14_ORIGIN.0, K14_ORIGIN.1);
        let off = mcu(
            "MCU_Deactivate",
            "PROBE INIT OFF",
            &mut next_id,
            K14_ORIGIN.0,
            K14_ORIGIN.1,
        );
        let begin_id = begin.index.unwrap();
        let init_off = off.index.unwrap();
        init.append_target(init_off);
        begin.append_target(init.index.unwrap());
        root.children.extend([begin, init, off]);
        Self {
            root,
            next_id,
            next_lc: P14_PROBE_LC_START,
            begin: begin_id,
            init_off,
            cell: None,
            arm: 0,
            close_off: 0,
            sources: vec![],
            hand_sources: HashSet::new(),
            map: TraceMap::default(),
        }
    }

    fn position(&self) -> (f64, f64) {
        self.cell
            .map(|cell| world(cell.centre_km))
            .unwrap_or(K14_ORIGIN)
    }

    fn locale_id(&mut self) -> i32 {
        let id = self.next_lc;
        self.next_lc += 1;
        id
    }

    fn add(&mut self, node: Il2Entity) -> i32 {
        let id = node.index.unwrap();
        let parent = if let Some(cell) = self.cell {
            self.root.find_by_name_mut(cell.key).unwrap()
        } else {
            &mut self.root
        };
        parent.children.push(node);
        id
    }

    fn reference_mcu(&mut self, block: &str, name: &str) -> Il2Entity {
        let mut node = reference_block(block).clone();
        node.index = Some(self.next_id);
        node.set_property("Index", self.next_id.to_string());
        self.next_id += 1;
        node.set_name(name);
        let (x, z) = self.position();
        node.set_property("XPos", format!("{x:.3}"));
        node.set_property("ZPos", format!("{z:.3}"));
        node.set_targets(vec![]);
        node.set_objects(vec![]);
        node.children.clear();
        node
    }

    fn link(&mut self, from: i32, to: i32) {
        by_id_mut(&mut self.root, from).append_target(to);
    }

    fn relay(&mut self, name: &str, delay_s: f64) -> i32 {
        let (x, z) = self.position();
        let node = timer(name, delay_s, &mut self.next_id, x, z);
        self.add(node)
    }

    fn at(&mut self, name: &str, time_s: f64, targets: &[i32]) -> i32 {
        let node = self.relay(name, time_s);
        by_id_mut(&mut self.root, node).set_targets(targets.to_vec());
        self.link(self.begin, node);
        node
    }

    fn control(&mut self, block: &str, name: &str, targets: &[i32]) -> i32 {
        let (x, z) = self.position();
        let mut node = mcu(block, name, &mut self.next_id, x, z);
        node.set_targets(targets.to_vec());
        self.add(node)
    }

    /// Create an object-linked Delete; the caller supplies its trigger and timing.
    fn delete(&mut self, name: &str, objects: &[i32]) -> i32 {
        let delete = self.control("MCU_Delete", name, &[]);
        by_id_mut(&mut self.root, delete).set_objects(objects.to_vec());
        delete
    }

    fn switch_at(&mut self, block: &str, name: &str, time_s: f64, targets: &[i32]) -> i32 {
        let action = self.control(block, name, targets);
        self.at(&format!("{name} AT"), time_s, &[action]);
        action
    }

    fn start_cell(&mut self, cell: Cell) {
        self.cell = None;
        let mut node = group(cell.key, &mut self.next_id);
        let (x, z) = world(cell.centre_km);
        node.set_property("XPos", format!("{x:.3}"));
        node.set_property("ZPos", format!("{z:.3}"));
        self.add(node);
        self.cell = Some(cell);
        self.arm = self.at(&format!("{} ARM", cell.key), cell.arm_s, &[]);
        self.close_off = self.control("MCU_Deactivate", &format!("{} CLOSE OFF", cell.key), &[]);
        self.at(
            &format!("{} CLOSE", cell.key),
            cell.close_s,
            &[self.close_off],
        );
    }

    fn close_node(&mut self, node: i32) {
        self.link(self.close_off, node);
    }

    fn zone(
        &mut self,
        name: &str,
        radius: f64,
        closer: bool,
        coalitions: &str,
        km: (f64, f64),
    ) -> i32 {
        let (x, z) = world(km);
        let node = checkzone(name, radius, closer, coalitions, &mut self.next_id, x, z);
        let id = self.add(node);
        self.close_node(id);
        id
    }

    fn count(&mut self, name: &str, threshold: i32, drop: i32) -> i32 {
        let (x, z) = self.position();
        let node = counter(name, threshold, drop, &mut self.next_id, x, z);
        self.add(node)
    }

    fn subtitle(&mut self, text: &str, duration: i32) -> i32 {
        let (x, z) = self.position();
        let mut node = mcu("MCU_TR_Subtitle", text, &mut self.next_id, x, z);
        node.set_property("Enabled", "1");
        node.set_property("Coalitions", "[0,1,2]");
        let mut info = Il2Entity::new("SubtitleInfo");
        for (key, value) in [
            ("Duration", duration),
            ("FontSize", 20),
            ("HAlign", 1),
            ("VAlign", 2),
            ("RColor", 0),
            ("GColor", 100),
            ("BColor", 0),
            ("LCText", self.locale_id()),
        ] {
            info.set_property(key, value.to_string());
        }
        node.children.push(info);
        self.add(node)
    }

    fn source(&self, index: i32, event_type: Option<i32>, expected_s: Option<f64>) -> TraceSource {
        fn path(root: &Il2Entity, index: i32, groups: &mut Vec<String>) -> bool {
            let is_group = root.block_type == "Group";
            if is_group {
                groups.push(root.name().unwrap().to_string());
            }
            if root.index == Some(index) || root.children.iter().any(|c| path(c, index, groups)) {
                return true;
            }
            if is_group {
                groups.pop();
            }
            false
        }
        let node = by_id(&self.root, index);
        let mut group_path = vec![];
        assert!(path(&self.root, index, &mut group_path));
        TraceSource {
            index,
            name: node.name().unwrap().into(),
            block_type: node.block_type.clone(),
            event_type,
            group_path,
            expected_s,
        }
    }

    fn observe(&mut self, source: i32, text: &str) {
        let subtitle = self.subtitle(text, 1);
        self.link(source, subtitle);
        self.sources.push(self.source(source, None, None));
    }

    fn readout(&mut self, source: i32, text: &str, threshold: i32) -> i32 {
        let count = self.count(&format!("READ {text}"), threshold, 0);
        self.link(source, count);
        self.observe(count, text);
        count
    }

    fn icon(&mut self, text: &str, km: (f64, f64), coalitions: &str) {
        let (x, z) = world(km);
        let mut node = mcu("MCU_Icon", text, &mut self.next_id, x, z);
        // Shipped named icons keep Name, but use LCDesc instead of Desc.
        node.properties.retain(|(key, _)| key != "Desc");
        for (key, value) in [
            ("Enabled", 1),
            ("LCName", self.locale_id()),
            ("LCDesc", self.locale_id()),
            // The existing frontlines::BATTLE stand-alone point marker.
            ("IconId", 501),
            ("RColor", 0),
            ("GColor", 100),
            ("BColor", 0),
            ("LineType", 0),
        ] {
            node.set_property(key, value.to_string());
        }
        node.set_property("Coalitions", coalitions);
        self.add(node);
    }

    fn cues(&mut self, flight: Flight) {
        self.cell = None;
        for (i, cue) in CUES
            .iter()
            .enumerate()
            .filter(|(_, cue)| cue.flight == flight)
        {
            let sub = self.subtitle(cue.text, 20);
            let source = self.at(&format!("CUE {:02}", i + 1), cue.time_s, &[sub]);
            self.sources
                .push(self.source(source, None, Some(cue.time_s)));
        }
    }

    fn pulses(&mut self, name: &str, times: &[f64], target: i32) {
        for (i, &time) in times.iter().enumerate() {
            self.at(&format!("{name} PULSE {}", i + 1), time, &[target]);
        }
    }

    /// Fixed-time stop avoids using the counter under test to bound its inputs.
    fn pulse_loop(
        &mut self,
        name: &str,
        start: f64,
        period: f64,
        count: usize,
        targets: &[i32],
    ) -> i32 {
        let tick = self.relay(&format!("{name} TICK"), 0.0);
        let delay = self.relay(&format!("{name} PERIOD"), period);
        by_id_mut(&mut self.root, tick).set_targets(targets.to_vec());
        self.link(tick, delay);
        self.link(delay, tick);
        self.at(&format!("{name} START"), start, &[tick]);
        self.switch_at(
            "MCU_Deactivate",
            &format!("{name} STOP"),
            start + (count - 1) as f64 * period + 1.0,
            &[tick],
        );
        self.close_node(tick);
        tick
    }

    /// Start a copied AI flight; the caller supplies any cell-close Deletes.
    fn ai_without_close_delete(
        &mut self,
        name: &str,
        planes: usize,
        start_km: (f64, f64),
        exit_km: (f64, f64),
        start_s: f64,
    ) -> i32 {
        let mut flight = probe_flight(
            HVAR,
            planes,
            world(start_km),
            world(exit_km),
            &mut self.next_id,
        );
        flight.set_name(name);
        self.start_group_at(flight, TEMPLATE_MISSION_BEGIN, start_s)
    }

    /// Add a group, pulse its named bring-up node, and close its zones with the cell.
    fn start_group_at(&mut self, group: Il2Entity, bring_up_name: &str, start_s: f64) -> i32 {
        let name = group.name().expect("probe startup group missing Name").to_string();
        let bring_up = id(&group, bring_up_name);
        let zones = block_ids(&group, "MCU_CheckZone");
        let group_id = self.add(group);
        self.at(&format!("{name} START"), start_s, &[bring_up]);
        for zone in zones {
            self.close_node(zone);
        }
        group_id
    }

    /// Start a copied AI flight with one cell-close Delete listing all aircraft entities.
    fn ai_with_close_delete(
        &mut self,
        name: &str,
        planes: usize,
        start_km: (f64, f64),
        exit_km: (f64, f64),
        start_s: f64,
    ) -> i32 {
        let flight_id = self.ai_without_close_delete(name, planes, start_km, exit_km, start_s);
        let entities = block_ids(by_id(&self.root, flight_id), "MCU_TR_Entity");
        let delete = self.delete(&format!("{name} CLOSE DELETE"), &entities);
        let close = id(&self.root, &format!("{} CLOSE", self.cell.unwrap().key));
        self.link(close, delete);
        flight_id
    }

    /// Set every plane's country in a copied flight. The built-in HVAR
    /// template's planes are all USA; fail if that source assumption changes.
    fn set_flight_plane_country(&mut self, flight_id: i32, country: i32) {
        let flight = by_id_mut(&mut self.root, flight_id);
        let plane_count = flight.count_block_type("Plane");
        let template_country = COUNTRY_USA.to_string();
        let mut changed = 0;
        flight.for_each_mut(&mut |node| {
            if node.block_type == "Plane" {
                assert_eq!(node.property("Country"), Some(template_country.as_str()));
                node.set_property("Country", country.to_string());
                changed += 1;
            }
        });
        assert_eq!(changed, plane_count, "set every copied flight plane's Country");
    }

    fn ai_delete_at(&mut self, name: &str, time_s: f64) {
        let delete = id(&self.root, &format!("{name} CLOSE DELETE"));
        self.at(&format!("{name} DELETE AT"), time_s, &[delete]);
    }

    fn damage_at(&mut self, name: &str, time_s: f64, entity: i32) {
        self.damage_level_at(name, time_s, entity, DamageLevel::Complete);
    }

    fn damage_level_at(&mut self, name: &str, time_s: f64, entity: i32, level: DamageLevel) {
        let damage = self.damage_command(name, entity, level);
        self.at(&format!("{name} AT"), time_s, &[damage]);
    }

    fn damage_command(&mut self, name: &str, entity: i32, level: DamageLevel) -> i32 {
        let mut damage = self.reference_mcu("MCU_CMD_Damage", name);
        damage.set_property("Damage", level.property());
        damage.set_objects(vec![entity]);
        self.add(damage)
    }

    /// Keep G1's country, skill, Attack shape and object order for every hunter.
    fn configure_hunter_and_attacks(
        &mut self,
        hunter: i32,
        target: i32,
        times_s: impl IntoIterator<Item = f64>,
    ) {
        self.set_flight_plane_country(hunter, COUNTRY_DPRK);
        by_id_mut(&mut self.root, hunter).for_each_mut(&mut |node| {
            if node.block_type == "Plane" {
                // Template Builder writes seat.skill.clamp(0, 4).
                node.set_property("AILevel", AI_LEVEL_ACE);
            }
        });
        let flight = by_id(&self.root, hunter);
        let name = flight
            .name()
            .unwrap_or_else(|| panic!("hunter flight Index {hunter}: missing Name"))
            .to_string();
        let entities = block_ids(flight, "MCU_TR_Entity");
        let lead = *entities
            .first()
            .unwrap_or_else(|| panic!("{name}: missing lead MCU_TR_Entity"));
        let mut attack = self.reference_mcu("MCU_CMD_AttackTarget", &format!("{name} ATTACK"));
        attack.set_objects(vec![lead]);
        attack.set_targets(vec![target]);
        let attack = self.add(attack);
        // Reissue while retaining the copied template's waypoint/AttackArea orders.
        for (i, time_s) in times_s.into_iter().enumerate() {
            self.at(&format!("{name} ATTACK AT {}", i + 1), time_s, &[attack]);
        }
    }

    fn set_mig15bis(&mut self, flight_id: i32) {
        // The same saved file as AttackTarget also supplies a MiG Plane.
        // Copy its assets and loadout; the HVAR payload belongs to the F-80C.
        static MIG15BIS_PLANE: OnceLock<Il2Entity> = OnceLock::new();
        let plane = MIG15BIS_PLANE.get_or_init(|| {
            let reference = parse_group_file(ATTACK_TARGET)
                .expect("TemplateExamples/CorrectedFlightfromTemplate.Group: MiG reference must parse");
            let planes = block_ids(&reference, "Plane");
            let plane = planes.first().expect(
                "TemplateExamples/CorrectedFlightfromTemplate.Group: missing first Plane (MiG-15bis)",
            );
            by_id(&reference, *plane).clone()
        });
        by_id_mut(&mut self.root, flight_id).for_each_mut(&mut |node| {
            if node.block_type == "Plane" {
                for key in MIG15BIS_PLANE_KEYS {
                    let value = plane.property(key).unwrap_or_else(|| {
                        panic!(
                            "TemplateExamples/CorrectedFlightfromTemplate.Group: MiG-15bis Plane missing {key}",
                        )
                    });
                    node.set_property(key, value);
                }
            }
        });
    }

    fn watch_aircraft(&mut self, label: &str, plane: i32, entity: i32, close: i32) {
        self.watch_aircraft_events(label, plane, entity, close, &[EVENT_KILLED]);
    }

    fn watch_aircraft_events(
        &mut self,
        label: &str,
        plane: i32,
        entity: i32,
        close: i32,
        event_types: &[i32],
    ) {
        by_id_mut(&mut self.root, plane).set_name(label);
        by_id_mut(&mut self.root, entity).set_name(&format!("{label} entity"));
        self.listen_aircraft_events(label, entity, close, event_types);
    }

    /// Add observations and an individual close Delete without renaming the source entity.
    fn listen_aircraft_events(&mut self, label: &str, entity: i32, close: i32, event_types: &[i32]) {
        for &event_type in event_types {
            let subtitle = self.subtitle(&format!("{label} ev{event_type}"), 1);
            attach_event(by_id_mut(&mut self.root, entity), event_type, subtitle);
            self.sources.push(self.source(entity, Some(event_type), None));
        }

        // The close must not depend on a Delete handling a mixed live/dead list.
        let delete = self.delete(&format!("{label} CLOSE DELETE"), &[entity]);
        self.link(close, delete);
    }

    fn watch_formation(&mut self, name: &str, flight_id: i32) -> Vec<i32> {
        let flight = by_id(&self.root, flight_id);
        let planes = block_ids(flight, "Plane");
        let entities = block_ids(flight, "MCU_TR_Entity");
        assert_eq!(planes.len(), 2, "{name}: expected exactly two Plane blocks");
        assert_eq!(
            entities.len(), 2, "{name}: expected exactly two MCU_TR_Entity blocks",
        );
        let cell = self.cell
            .unwrap_or_else(|| panic!("{name}: missing formation cell"));
        let close = id(&self.root, &format!("{} CLOSE", cell.key));
        for (i, &entity) in entities.iter().enumerate() {
            let label = format!("{name}{}", wingman_letter(i));
            self.watch_aircraft(&label, planes[i], entity, close);
        }
        entities
    }

    fn detect_formation_at(&mut self, name: &str, entities: &[i32], time_s: f64) {
        for (i, &entity) in entities.iter().enumerate() {
            self.detect_aircraft(&format!("{name}{}", wingman_letter(i)), entity, time_s);
        }
    }

    fn remove_formation_at(
        &mut self,
        name: &str,
        entities: &[i32],
        time_s: f64,
        removal: FormationRemoval,
    ) {
        match removal {
            FormationRemoval::Nothing => {}
            FormationRemoval::DeleteLeadEntity => {
                self.delete_entities_at(name, &[entities[Seat::Lead.index()]], time_s);
            }
            FormationRemoval::DeleteWingmanEntity => {
                self.delete_entities_at(name, &[entities[Seat::Wingman.index()]], time_s);
            }
            FormationRemoval::DeleteBothEntities => self.delete_entities_at(name, entities, time_s),
            FormationRemoval::DeleteEachEntity => {
                self.delete_each_entity_at(name, entities, time_s, wingman_letter);
            }
            FormationRemoval::DeactivateBothEntities => {
                self.deactivate_entities_at(name, entities, time_s);
            }
        }
    }

    fn detect_aircraft(&mut self, label: &str, entity: i32, time_s: f64) {
        self.damage_at(&format!("{label} DETECTOR"), time_s, entity);
    }

    fn kill_aircraft(&mut self, label: &str, entity: i32) {
        self.damage_at(&format!("{label} DAMAGE"), D_KILL_S, entity);
    }

    fn remove_at(&mut self, name: &str, deletes: &[i32], time_s: f64) {
        self.at(&format!("{name} DELETE AT"), time_s, deletes);
    }

    fn delete_entities_at(&mut self, name: &str, entities: &[i32], time_s: f64) {
        let delete = self.delete(&format!("{name} DELETE"), entities);
        self.remove_at(name, &[delete], time_s);
    }

    fn deactivate_entities_at(&mut self, name: &str, entities: &[i32], time_s: f64) {
        let deactivate = self.control("MCU_Deactivate", &format!("{name} DEACTIVATE"), &[]);
        by_id_mut(&mut self.root, deactivate).set_objects(entities.to_vec());
        self.at(&format!("{name} DEACTIVATE AT"), time_s, &[deactivate]);
    }

    fn delete_each_entity_at(
        &mut self,
        name: &str,
        entities: &[i32],
        time_s: f64,
        suffix: impl Fn(usize) -> char,
    ) {
        let deletes: Vec<_> = entities.iter().enumerate().map(|(i, &entity)| {
            let label = format!("{name}{}", suffix(i));
            self.delete(&format!("{label} DELETE"), &[entity])
        }).collect();
        self.remove_at(name, &deletes, time_s);
    }

    fn hand_breadcrumb(
        &mut self,
        source: i32,
        inputs: &[i32],
        carrier: TraceCarrier,
        expected_s: Option<f64>,
    ) {
        let source = self.source(source, None, expected_s);
        self.hand_sources.insert(source.index);
        self.hand_sources.extend(inputs);
        let before = self.map.entries.len();
        let nodes = trace::breadcrumb(carrier, &mut self.map, &mut self.next_id, source);
        // breadcrumb registers its own entry. Re-register through push, as the
        // hand-built probe contract requires, without retaining a duplicate.
        let entries = self.map.entries.split_off(before);
        for entry in entries {
            for &input in inputs {
                self.link(input, entry.breadcrumb_index);
            }
            self.map.push(entry);
        }
        for node in nodes {
            self.add(node);
        }
    }

    fn selected_entities_share_event_types(&self, selection: &TraceSelect) -> bool {
        selection.indexes.iter()
            .filter(|&&index| by_id(&self.root, index).block_type == "MCU_TR_Entity")
            .all(|&index| {
                self.sources.iter().filter(|source| source.index == index)
                    .filter_map(|source| source.event_type).collect::<BTreeSet<_>>()
                    .into_iter().collect::<Vec<_>>() == selection.event_types
            })
    }

    fn traced(mut self, carrier: TraceCarrier) -> (Il2Entity, TraceMap) {
        let indexes = self
            .sources
            .iter()
            .filter(|source| !self.hand_sources.contains(&source.index))
            .map(|source| source.index)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let event_types = self
            .sources
            .iter()
            .filter_map(|source| source.event_type)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let selection = TraceSelect {
            indexes,
            event_types,
            ..TraceSelect::default()
        };
        // TraceSelect applies every event type to every selected entity. Keep
        // instrument's order and bytes for the flown probes with uniform sets;
        // use per-source breadcrumbs when T-v needs 0..13 but T-r/T-w need only
        // ev13 (W1's generated ev4 -> DeathCount is not an observed source).
        if self.selected_entities_share_event_types(&selection) {
            trace::instrument(
                &mut self.root,
                &selection,
                &mut self.next_id,
                carrier,
                &mut self.map,
            );
        } else {
            // Use the public breadcrumb builder per requested source, in tree order.
            let mut sources = Vec::new();
            self.root.for_each(&mut |node| {
                sources.extend(self.sources.iter().filter(|source| {
                    node.index == Some(source.index)
                        && !self.hand_sources.contains(&source.index)
                }).cloned());
            });
            for source in sources {
                let node = source.index;
                let event_type = source.event_type;
                let blocks = trace::breadcrumb(carrier, &mut self.map, &mut self.next_id, source);
                let breadcrumb = self.map.entries.last()
                    .expect("trace::breadcrumb must register a trace map entry")
                    .breadcrumb_index;
                if let Some(event_type) = event_type {
                    attach_event(by_id_mut(&mut self.root, node), event_type, breadcrumb);
                } else {
                    self.link(node, breadcrumb);
                }
                self.root.children.extend(blocks);
            }
            trace::rebuild_edges(&self.root, &mut self.map);
        }
        // Pass the explicit cue TraceSource metadata on to its instrumented
        // entry. The cue timer is a direct Mission Begin child, also allowing
        // 0T to infer this time itself. No extra breadcrumb is created here.
        for source in self.sources.into_iter().filter(|s| s.expected_s.is_some()) {
            set_expected_time(&mut self.map, source);
        }
        (self.root, self.map)
    }
}

fn set_expected_time(map: &mut TraceMap, source: TraceSource) {
    for entry in &mut map.entries {
        if entry.source_index == source.index && entry.event_type == source.event_type {
            entry.expected_s = source.expected_s;
        }
    }
}

fn build_a1(p: &mut Probe) {
    let centre = p.cell.unwrap().centre_km;
    for (name, coalition) in [("ZA1", COALITION_NATO), ("ZA2", COALITION_DPRK)] {
        let zone = p.zone(name, AI_VS_PLAYER_RADIUS_M, true, coalition, centre);
        p.link(p.arm, zone);
        p.observe(zone, &format!("{name} fired"));
    }
    p.ai_with_close_delete("T-a1 AI", 4, A1_START_KM, A1_EXIT_KM, A1_AI_START_S);
    p.icon("T-a1", centre, COALITION_NATO);
}

fn build_e(p: &mut Probe) {
    let [
        dropcount_once,
        dropcount_reset,
        deactivate_activate,
        reset_partial,
        inactive_pulses,
        reset_fired,
        same_tick_count,
        once_gate,
        census_spacing,
    ] = E_COUNTERS.map(|row| {
        let count = p.count(row.name, row.threshold, row.dropcount);
        p.observe(count, &format!("{} fired", row.name));
        if row.repeat_readout {
            p.readout(count, &format!("{} fired 2+", row.name), 2);
        }
        count
    });
    p.pulse_loop(
        "E1 E2",
        p.cell.unwrap().arm_s,
        E_PULSE_PERIOD_S,
        E_SIX_PULSES.len(),
        &[dropcount_once, dropcount_reset],
    );
    p.pulses("E3", &E3_PULSES, deactivate_activate);
    p.switch_at("MCU_Deactivate", "E3 OFF", E3_DEACTIVATE_S, &[deactivate_activate]);
    p.switch_at("MCU_Activate", "E3 ON", E3_ACTIVATE_S, &[deactivate_activate]);
    p.pulses("E4", &E_PARTIAL_PULSES, reset_partial);
    p.pulses("E5", &E_PARTIAL_PULSES, inactive_pulses);
    p.link(p.init_off, inactive_pulses);
    p.switch_at("MCU_Activate", "E5 ON", E5_ACTIVATE_S, &[inactive_pulses]);
    p.pulses("E6", &E_SIX_PULSES, reset_fired);
    for (name, at, count) in [
        ("E4 RESET", E4_RESET_S, reset_partial),
        ("E6 RESET", E6_RESET_S, reset_fired),
    ] {
        let (x, z) = p.position();
        let mut node = modifier_set_val(name, &mut p.next_id, x, z);
        node.append_target(count);
        let reset = p.add(node);
        p.at(&format!("{name} AT"), at, &[reset]);
    }
    for (name, at, size, count) in [
        ("E7", E7_SAME_TICK_S, 3, same_tick_count),
        ("E8", E8_ONCE_GATE_S, 2, once_gate),
    ] {
        let relays: Vec<_> = (1..=size)
            .map(|n| {
                let relay = p.relay(&format!("{name} INPUT {n}"), 0.0);
                p.link(relay, count);
                relay
            })
            .collect();
        p.at(&format!("{name} SAME TICK"), at, &relays);
    }
    p.pulses("E9", &E9_PULSES, census_spacing);
}

fn build_f(p: &mut Probe) {
    let (x, z) = p.position();
    let f1 = timer_random("F1", 0.0, 50, &mut p.next_id, x, z);
    let f1 = p.add(f1);
    p.observe(f1, "F1 hit");
    p.readout(f1, "F1 reached 12", 12);
    p.readout(f1, "F1 reached 29", 29);
    p.pulse_loop("F1", p.cell.unwrap().arm_s, F1_PULSE_PERIOD_S, F_PULSE_COUNT, &[f1]);

    let waterfall_chances_pct = [25, 33, 50, 100];
    let outs: Vec<_> = (1..=waterfall_chances_pct.len())
        .map(|k| {
            let out = p.relay(&format!("F2 OUT {k}"), 0.0);
            p.observe(out, &format!("F2 out {k}"));
            out
        })
        .collect();
    let open = p.control("MCU_Activate", "F2 OPEN OUTPUTS", &outs);
    let mut inputs = vec![open];
    // Equal-weight waterfall, using the shipped randomizer's 0.5*k delays.
    // Close future output relays, not running random timers (risk J1).
    for (k, pct) in waterfall_chances_pct.into_iter().enumerate() {
        let node = timer_random(
            &format!("F2 RANDOM {}", k + 1),
            F2_RANDOM_DELAY_STEP_S * (k + 1) as f64,
            pct,
            &mut p.next_id,
            x,
            z,
        );
        let random = p.add(node);
        p.link(random, outs[k]);
        let later_outputs = &outs[k + 1..];
        if !later_outputs.is_empty() {
            let close = p.control(
                "MCU_Deactivate",
                &format!("F2 CLOSE AFTER {}", k + 1),
                later_outputs,
            );
            p.link(random, close);
        }
        inputs.push(random);
    }
    p.pulse_loop("F2", F2_START_S, F2_PULSE_PERIOD_S, F_PULSE_COUNT, &inputs);
    for out in outs {
        p.close_node(out);
    }
}

fn build_tt(p: &mut Probe) {
    // Flight 0, 2026-09-28: Success 0 logs without ending the round. T1/T2/
    // T3/T7 test single, repeated and same-tick firings of the default style.
    let t1 = p.at("T-t T1", T1_SINGLE_PULSE_S, &[]);
    p.observe(t1, "T-t T1 fired");
    p.hand_breadcrumb(t1, &[t1], TraceCarrier::default(), Some(T1_SINGLE_PULSE_S));

    let t2 = p.relay("T-t T2", 0.0);
    p.observe(t2, "T-t T2 fired");
    p.pulses("T-t T2", &T2_PULSES, t2);
    p.hand_breadcrumb(t2, &[t2], TraceCarrier::default(), None);

    let mut t3 = vec![];
    for suffix in ['a', 'b'] {
        let relay = p.relay(&format!("T-t T3{suffix}"), 0.0);
        p.observe(relay, &format!("T-t T3{suffix} fired"));
        p.hand_breadcrumb(relay, &[relay], TraceCarrier::default(), Some(T3_SAME_TICK_S));
        t3.push(relay);
    }
    p.at("T-t T3 SAME TICK", T3_SAME_TICK_S, &t3);

    // Information only: spawned-and-deleted vehicles were silent in real_2.
    let t5 = p.relay("T-t T5", 0.0);
    p.observe(t5, "T-t T5 fired");
    p.pulses("T-t T5", &T5_PULSES, t5);
    p.hand_breadcrumb(t5, &[t5], TraceCarrier::Spawn, None);

    let t7 = p.at("T-t T7 SAME TICK", T7_SAME_TICK_S, &[]);
    let subtitle = p.subtitle("T-t T7 fired", 1);
    let mut inputs = vec![];
    for n in 1..=2 {
        let relay = p.relay(&format!("T-t T7 INPUT {n}"), 0.0);
        p.link(t7, relay);
        p.link(relay, subtitle);
        inputs.push(relay);
    }
    // Exactly one carrier, with two direct inputs (no intervening latch).
    p.hand_breadcrumb(t7, &inputs, TraceCarrier::default(), Some(T7_SAME_TICK_S));
    for source in [t1, t2, t5, t7] {
        p.close_node(source);
    }
}

fn build_j(p: &mut Probe) {
    let arm_s = p.cell.unwrap().arm_s;
    let running_timer_s = 10.0;
    let deactivate_running_s = arm_s + 5.0;
    let activate_pulsed_relay_s = arm_s + 1.0;
    let retrigger_running_s = arm_s + 5.0;
    let j1 = p.relay("J1", running_timer_s);
    p.link(p.arm, j1);
    p.switch_at("MCU_Deactivate", "J1 OFF", deactivate_running_s, &[j1]);
    p.observe(j1, "J1 out");
    let j2 = p.relay("J2", 0.0);
    p.link(p.init_off, j2);
    p.link(p.arm, j2);
    p.switch_at("MCU_Activate", "J2 ON", activate_pulsed_relay_s, &[j2]);
    p.observe(j2, "J2 out");
    let j3 = p.relay("J3", 0.0);
    p.link(p.init_off, j3);
    let j3_on = p.control("MCU_Activate", "J3 ON", &[j3]);
    p.link(p.arm, j3_on);
    p.link(p.arm, j3);
    p.observe(j3, "J3 out");
    let j4 = p.relay("J4", 0.0);
    let off = p.control("MCU_Deactivate", "J4 LATCH OFF", &[j4]);
    p.link(j4, off);
    p.observe(j4, "J4 out");
    p.readout(j4, "J4 out 2+", 2);
    let mut inputs = vec![];
    for n in 1..=2 {
        let input = p.relay(&format!("J4 INPUT {n}"), 0.0);
        p.link(input, j4);
        inputs.push(input);
    }
    p.at("J4 SAME TICK", arm_s, &inputs);
    let j5 = p.relay("J5", running_timer_s);
    p.link(p.arm, j5);
    p.at("J5 RETRIGGER", retrigger_running_s, &[j5]);
    p.observe(j5, "J5 out");
    p.readout(j5, "J5 out 2+", 2);
    for relay in [j1, j2, j3, j4, j5] {
        p.close_node(relay);
    }
}

/// Gate a second-use observation by the clock, independently of repeat firings.
/// Flight 1A's C5 fired once while occupied; flight 1N showed a pulsed Closer 1
/// zone fires again for each new entry into an empty zone.
fn again(p: &mut Probe, zone: i32, name: &str, off_s: f64, on_s: f64, pulse_s: f64) {
    let out = p.relay(&format!("{name} AGAIN OUT"), 0.0);
    p.link(p.init_off, out);
    p.link(zone, out);
    p.observe(out, &format!("{name} fired again"));
    p.switch_at(
        "MCU_Deactivate",
        &format!("{name} RECHECK OFF"),
        off_s,
        &[zone],
    );
    let on = p.switch_at(
        "MCU_Activate",
        &format!("{name} RECHECK ON"),
        on_s,
        &[zone, out],
    );
    if on_s == pulse_s {
        let tick = id(&p.root, &format!("{name} RECHECK ON AT"));
        // Activate and pulse are siblings, as required by the Z1 experiment.
        assert!(by_id(&p.root, tick).targets.contains(&on));
        p.link(tick, zone);
    } else {
        p.at(&format!("{name} RECHECK PULSE"), pulse_s, &[zone]);
    }
    p.close_node(out);
}

fn build_z(p: &mut Probe) {
    let cell = p.cell.unwrap();
    let empty_zone_radius_m = 5000.0;
    let recheck_off_s = cell.arm_s + 10.0;
    let recheck_on_s = cell.arm_s + 15.0;
    let z1 = p.zone("Z1", empty_zone_radius_m, false, COALITION_DPRK, cell.centre_km);
    p.link(p.init_off, z1);
    let on = p.control("MCU_Activate", "Z1 ON", &[z1]);
    p.link(p.arm, on);
    p.link(p.arm, z1);
    p.observe(z1, "Z1 fired");
    again(p, z1, "Z1", recheck_off_s, recheck_on_s, recheck_on_s);
    p.icon("T-z", cell.centre_km, COALITION_NATO);
}

fn build_c(p: &mut Probe) {
    let centre = p.cell.unwrap().centre_km;
    let mut observed_zone = |name: &str, km| {
        let zone = p.zone(name, C_ZONE_RADIUS_M, true, COALITION_NATO, km);
        p.observe(zone, &format!("{name} fired"));
        zone
    };
    let sentinel = observed_zone("C0", centre);
    let activate_only = observed_zone("C1", centre);
    let activate_and_pulse = observed_zone("C2", centre);
    let pulse_while_deactivated = observed_zone("C3", centre);
    let cancel_waiting = observed_zone("C4", C4_KM);
    let repeat_firings = observed_zone("C5", centre);
    for zone in [activate_only, activate_and_pulse, pulse_while_deactivated, cancel_waiting] {
        p.link(p.init_off, zone);
    }
    p.at("C0 SENTINEL", C_ACTIVATE_S, &[sentinel]);
    p.switch_at("MCU_Activate", "C1 ON", C_ACTIVATE_S, &[activate_only]);
    p.switch_at("MCU_Activate", "C2 ON", C_ACTIVATE_S, &[activate_and_pulse]);
    p.at("C2 PULSE", C_ACTIVATE_S + C_PULSE_AFTER_ACTIVATE_S, &[activate_and_pulse]);
    p.at("C3 INACTIVE PULSE", C_ACTIVATE_S, &[pulse_while_deactivated]);
    again(
        p,
        activate_and_pulse,
        "C2",
        C2_RECHECK_OFF_S,
        C2_RECHECK_ON_S,
        C2_RECHECK_ON_S + C_PULSE_AFTER_ACTIVATE_S,
    );
    p.switch_at("MCU_Activate", "C4 ON", C4_WAITING_ON_S, &[cancel_waiting]);
    p.at("C4 PULSE", C4_WAITING_ON_S + C_PULSE_AFTER_ACTIVATE_S, &[cancel_waiting]);
    p.switch_at("MCU_Deactivate", "C4 OFF", C4_WAITING_OFF_S, &[cancel_waiting]);
    p.at("C5 PULSE", C5_REPEAT_PULSE_S, &[repeat_firings]);
    p.readout(repeat_firings, "C5 fired 2+", 2);
    p.readout(repeat_firings, "C5 fired 10+", 10);
    p.icon("T-c C", centre, COALITION_NATO);
    p.icon("T-c C4", C4_KM, COALITION_NATO);
}

enum NVariant {
    Tn,
    Tn2,
}

fn build_n(p: &mut Probe, variant: NVariant) {
    let radius_m = match variant {
        NVariant::Tn => 2000.0,
        NVariant::Tn2 => 5000.0,
    };
    for (name, km) in [("T-n START", N_START_KM), ("T-n END", N_END_KM)] {
        let zone = p.zone(&format!("{name} ZONE"), radius_m, true, COALITION_NATO, km);
        let off = p.control("MCU_Deactivate", &format!("{name} OFF"), &[zone]);
        p.link(zone, off);
        p.link(p.arm, zone);
        p.observe(zone, name);
        p.icon(&format!("{name} ICON"), km, COALITION_NATO);
        // Editor and map label use the exact run-sheet name; the zone has a suffix.
        let icon = p.root.find_by_name_mut(&format!("{name} ICON")).unwrap();
        icon.set_name(name);
    }
    if matches!(variant, NVariant::Tn) {
        let za4 = p.zone("ZA4", AI_VS_PLAYER_RADIUS_M, true, COALITION_NATO, p.cell.unwrap().centre_km);
        p.link(p.arm, za4);
        p.observe(za4, "ZA4 fired");
    }
}

enum Side {
    Nato,
    Dprk,
}

fn build_y(p: &mut Probe, side: Side) {
    let (prefix, player_country, ai_country, coalition) = match side {
        Side::Nato => ("YN", COUNTRY_USA, COUNTRY_NATO_AI, COALITION_NATO),
        Side::Dprk => ("YD", COUNTRY_USSR, COUNTRY_DPRK, COALITION_DPRK),
    };
    let centre = p.cell.unwrap().centre_km;
    let zone_radius_m = 10_000.0;
    let ai_country_zone_pulse_s = p.cell.unwrap().arm_s;
    let player_country_zone_pulse_s = 325.0;
    let pilot_zone_pulse_s = 640.0;
    let (ai_country_start_s, ai_country_delete_s) = (20.0, 320.0);
    let (player_country_start_s, player_country_delete_s) = (330.0, 630.0);
    let ai_start_km = (centre.0 - 30.0, centre.1);
    let ai_exit_km = (centre.0 + 30.0, centre.1);
    for (suffix, filtered) in [("CT", true), ("CT0", false)] {
        let name = format!("{prefix}-{suffix}");
        let mut node = p.reference_mcu("MCU_TR_ComplexTrigger", &name);
        // These country-only and unfiltered shapes were not saved from the
        // editor. Both load in MissionResaver and worked in flights 1N and 1D
        // on 2026-10-12; see handoff/P14-probe-results.md.
        node.properties.retain(|(key, _)| {
            !matches!(key.as_str(), "ObjectScript" | "ObjectName") && (filtered || key != "Country")
        });
        if filtered {
            node.set_property("Country", player_country.to_string());
        }
        let trigger = p.add(node);
        // Live from Mission Begin: Activate on Complex Triggers is unverified.
        p.close_node(trigger);
        for (event_type, verb, suffix) in [
            (EVENT_OBJECT_ENTERED, "entered", "ENTERED"),
            (EVENT_OBJECT_LEFT, "left", "LEFT"),
        ] {
            let text = format!("{name} {verb}");
            let relay = p.relay(&format!("{name} {suffix}"), 0.0);
            attach_event(by_id_mut(&mut p.root, trigger), event_type, relay);
            // The relay fans out to observations and the instrumented breadcrumb.
            p.observe(relay, &text);
            if event_type == EVENT_OBJECT_ENTERED {
                for threshold in [2, 3] {
                    p.readout(relay, &format!("{text} {threshold}+"), threshold);
                }
            }
        }
    }
    for (n, time_s) in [
        (1, ai_country_zone_pulse_s),
        (2, player_country_zone_pulse_s),
        (3, pilot_zone_pulse_s),
    ] {
        let name = format!("{prefix}-Z{n}");
        let zone = p.zone(&name, zone_radius_m, true, coalition, centre);
        p.at(&format!("{name} PULSE"), time_s, &[zone]);
        p.observe(zone, &format!("{name} fired"));
    }
    for (suffix, flight_country, start_s, delete_s) in [
        ("A", ai_country, ai_country_start_s, ai_country_delete_s),
        ("B", player_country, player_country_start_s, player_country_delete_s),
    ] {
        let name = format!("{prefix}-{suffix}");
        let flight = p.ai_with_close_delete(&name, 2, ai_start_km, ai_exit_km, start_s);
        p.set_flight_plane_country(flight, flight_country);
        p.ai_delete_at(&name, delete_s);
    }
    p.icon(p.cell.unwrap().key, centre, coalition);
}

fn build_g(p: &mut Probe) {
    let centre = p.cell.unwrap().centre_km;
    let mut hunter_target = None;
    for row in G_FLIGHTS {
        let name = row.name;
        let at = row.centre_km.unwrap_or(centre);
        let flight_id =
            p.ai_with_close_delete(name, 2, (at.0 - START_SOUTH_KM, at.1), at, G_START_S);
        let entities = block_ids(by_id(&p.root, flight_id), "MCU_TR_Entity");
        for (i, &entity) in entities.iter().enumerate() {
            let label = format!("{name}{}", wingman_letter(i));
            by_id_mut(&mut p.root, entity).set_name(&format!("{label} entity"));
            for event_type in G_EVENT_TYPES {
                let text = format!("{label} ev{event_type}");
                let subtitle = p.subtitle(&text, 1);
                attach_event(by_id_mut(&mut p.root, entity), event_type, subtitle);
                p.sources.push(p.source(entity, Some(event_type), None));
                if event_type == EVENT_PLANE_DESTROYED {
                    let readout = p.count(&format!("READ {text} 2+"), 2, 0);
                    attach_event(by_id_mut(&mut p.root, entity), event_type, readout);
                    p.observe(readout, &format!("{text} 2+"));
                }
            }
        }
        match row.action {
            EventProbeAction::ShotDownByHunter => {
                hunter_target = Some((name, at, entities[Seat::Lead.index()]));
            }
            EventProbeAction::ProbeDelete => {
                p.delete_entities_at(name, &entities, G_REMOVE_S);
            }
            EventProbeAction::TemplateCleanup => {
                let flight = by_id(&p.root, flight_id);
                let deactivate = id(flight, "Deactivate Units");
                let delay = id(flight, "DELETE DELAY");
                // Preserve the HVAR Delete's Plane-object links and its 0.5 s
                // DELETE DELAY; the probe-owned Deletes use entity links as specified.
                p.at(
                    &format!("{name} DEACTIVATE AT"),
                    G_REMOVE_S,
                    &[deactivate, delay],
                );
            }
            EventProbeAction::DamageCommand => {
                for (i, entity) in entities.into_iter().enumerate() {
                    let name = format!("{name}{} DAMAGE", wingman_letter(i));
                    p.damage_at(&name, G_DAMAGE_S, entity);
                }
            }
        }
    }
    // Create the hunter after G1-G4 to preserve generated object order.
    let (name, at, target) = hunter_target.expect("G flights include a hunter target");
    let (hunter_name, hunter_start_km) =
        hunter_name_and_start(name, (at.0 - START_SOUTH_KM, at.1));
    let hunter = p.ai_with_close_delete(&hunter_name, 2, hunter_start_km, at, G_START_S);
    p.configure_hunter_and_attacks(
        hunter,
        target,
        hunter_attack_times_before(G_REMOVE_S).take(G_HUNTER_ATTACK_COUNT),
    );
    for row in G_FLIGHTS {
        // The spec slipped: icons show to the DPRK side; kept because the flown file must regenerate.
        p.icon(
            &format!("T-g {}", row.name),
            row.centre_km.unwrap_or(centre),
            COALITION_DPRK,
        );
    }
}

fn build_d(p: &mut Probe) {
    let cell = p.cell.unwrap();
    let close = id(&p.root, &format!("{} CLOSE", cell.key));
    for row in D_FLIGHTS {
        let name = row.name;
        let centre = row.centre_km;
        let flight_id = p.ai_without_close_delete(
            name,
            2,
            (centre.0 - START_SOUTH_KM, centre.1),
            centre,
            cell.arm_s,
        );
        let flight = by_id(&p.root, flight_id);
        let planes = block_ids(flight, "Plane");
        let entities = block_ids(flight, "MCU_TR_Entity");
        for (i, &entity) in entities.iter().enumerate() {
            let label = format!("{name}{}", wingman_letter(i));
            p.watch_aircraft(&label, planes[i], entity, close);
            if row.detector {
                p.detect_aircraft(&label, entity, D_DETECT_S);
            }
        }
        if let Some(seat) = row.killed {
            let label = format!("{name}{}", wingman_letter(seat.index()));
            p.kill_aircraft(&label, entities[seat.index()]);
        }
        match row.removal {
            DeleteRemoval::Nothing => {}
            DeleteRemoval::DeleteBothEntities => p.delete_entities_at(name, &entities, D_REMOVE_S),
            DeleteRemoval::DeleteEachEntity => {
                p.delete_each_entity_at(name, &entities, D_REMOVE_S, wingman_letter);
            }
            DeleteRemoval::DeleteLeadEntity => {
                p.delete_entities_at(name, &[entities[Seat::Lead.index()]], D_REMOVE_S);
            }
            DeleteRemoval::TemplateCleanup => {
                let flight = by_id(&p.root, flight_id);
                let deactivate = id(flight, "Deactivate Units");
                let delay = id(flight, "DELETE DELAY");
                // As G3: keep the template's Plane-object Delete and 0.5 s delay.
                p.at(&format!("{name} DEACTIVATE AT"), D_REMOVE_S, &[deactivate, delay]);
            }
            DeleteRemoval::DeletePlaneIndexes => {
                // Deliberately match generate_template's Trigger Delete Objects
                // (object_ids): Plane indexes, unlike the editor-saved examples'
                // entity indexes. Keep the lead before the wingman.
                let delete = p.delete(&format!("{name} DELETE"), &planes);
                p.remove_at(name, &[delete], D_REMOVE_S);
            }
            DeleteRemoval::DeactivateOnly => {
                p.deactivate_entities_at(name, &entities, D_REMOVE_S);
            }
        }
        p.icon(&format!("{} {name}", cell.key), centre, COALITION_NATO);
    }
}

fn build_d2(p: &mut Probe) {
    let cell = p.cell.unwrap();
    let close = id(&p.root, &format!("{} CLOSE", cell.key));
    for row in J_PAIRS {
        let name = row.name;
        let entities = PairAircraft::BOTH.map(|aircraft| {
            let label = format!("{name}{}", aircraft.suffix());
            let centre = (row.centre_km.0, row.centre_km.1 + aircraft.east_offset_km());
            let flight_id = p.ai_without_close_delete(
                &label,
                1,
                (centre.0 - START_SOUTH_KM, centre.1),
                centre,
                cell.arm_s,
            );
            let (plane, entity) = single_aircraft(by_id(&p.root, flight_id), &label);
            p.watch_aircraft(&label, plane, entity, close);
            if row.detector {
                p.detect_aircraft(&label, entity, D_DETECT_S);
            }
            entity
        });
        if let Some(aircraft) = row.killed {
            let label = format!("{name}{}", aircraft.suffix());
            p.kill_aircraft(&label, entities[aircraft.index()]);
        }
        match row.removal {
            DeletePairRemoval::Nothing => {}
            DeletePairRemoval::DeleteBothEntities => p.delete_entities_at(name, &entities, D_REMOVE_S),
            DeletePairRemoval::DeleteEachEntity => {
                p.delete_each_entity_at(name, &entities, D_REMOVE_S, |i| PairAircraft::BOTH[i].suffix());
            }
            DeletePairRemoval::DeleteXEntity => {
                p.delete_entities_at(name, &[entities[PairAircraft::X.index()]], D_REMOVE_S);
            }
        }
        // The spec slipped: icons show to the DPRK side; kept because the flown file must regenerate.
        p.icon(&format!("{} {name}", cell.key), row.centre_km, COALITION_DPRK);
    }
}

fn build_d3(p: &mut Probe) {
    let cell = p.cell.expect("flight 1K: missing T-d3 cell");
    for row in K_FORMATIONS {
        let name = row.name;
        let centre = row.centre_km;
        let flight = p.ai_without_close_delete(
            name, 2, (centre.0 - START_SOUTH_KM, centre.1), centre, cell.arm_s,
        );
        let entities = p.watch_formation(name, flight);
        match row.change {
            FormationChange::Nothing => {}
            FormationChange::DeleteWingmanEntity => {
                let label = format!("{name}{}", wingman_letter(Seat::Wingman.index()));
                p.delete_entities_at(&label, &[entities[Seat::Wingman.index()]], K_CHANGE_S);
            }
            FormationChange::DamageLead(level) => {
                let label = format!("{name}{}", wingman_letter(Seat::Lead.index()));
                p.damage_level_at(&format!("{label} DAMAGE"), K_CHANGE_S, entities[Seat::Lead.index()], level);
            }
        }
        p.remove_formation_at(name, &entities, D_REMOVE_S, row.removal);
        if row.detector {
            p.detect_formation_at(name, &entities, D_DETECT_S);
        }
        p.icon(&format!("{} {name}", cell.key), centre, COALITION_NATO);
    }
}

fn build_d4(p: &mut Probe) {
    let cell = p.cell.expect("flight 1K: missing T-d4 cell");
    for row in K_FIGHTS {
        let name = row.name;
        let centre = row.centre_km;
        let start = (centre.0 - START_SOUTH_KM, centre.1);
        let flight = p.ai_without_close_delete(name, 2, start, centre, cell.arm_s);
        let entities = p.watch_formation(name, flight);
        p.remove_formation_at(name, &entities, row.delete_s, row.removal);
        p.detect_formation_at(name, &entities, row.detector_s);

        let (hunter_name, hunter_start) = hunter_name_and_start(name, start);
        let hunter = p.ai_without_close_delete(&hunter_name, 2, hunter_start, centre, cell.arm_s);
        p.set_mig15bis(hunter);
        let extra_order = std::iter::once(K_HUNTER_EXTRA_ATTACK_S)
            .filter(|&time_s| time_s < row.delete_s);
        let attack_times = hunter_attack_times_before(row.delete_s).chain(extra_order);
        p.configure_hunter_and_attacks(hunter, entities[Seat::Lead.index()], attack_times);
        p.watch_formation(&format!("{name}H"), hunter);
        p.icon(&format!("{} {name}", cell.key), centre, COALITION_NATO);
    }
}

fn build_r(p: &mut Probe) {
    let cell = p.cell.expect("flight 1L: missing T-r cell");
    for row in R_FORMATIONS {
        let name = row.name;
        let centre = row.centre_km;
        let flight = p.ai_without_close_delete(
            name, 2, (centre.0 - START_SOUTH_KM, centre.1), centre, cell.arm_s,
        );
        let mut changed = 0;
        by_id_mut(&mut p.root, flight).for_each_mut(&mut |node| {
            if node.block_type == "Plane" {
                assert_eq!(node.property("AiRTBDecision"), Some(AiRtbDecision::Enabled.property()),
                    "flight 1L {name}: HVAR AiRTBDecision changed");
                node.set_property("AiRTBDecision", row.ai_rtb_decision.property());
                changed += 1;
            }
        });
        assert_eq!(changed, 2, "flight 1L {name}: set AiRTBDecision on exactly two planes");
        let entities = p.watch_formation(name, flight);
        let label = format!("{name}{}", wingman_letter(Seat::Lead.index()));
        p.damage_level_at(
            &format!("{label} DAMAGE"), L_DAMAGE_S, entities[Seat::Lead.index()], row.damage,
        );
        p.remove_formation_at(name, &entities, R_REMOVE_S, row.removal);
        if row.detector {
            p.detect_formation_at(name, &entities, R_DETECT_S);
        }
        p.icon(&format!("{} {name}", cell.key), centre, COALITION_NATO);
    }
}

fn build_v(p: &mut Probe) {
    let cell = p.cell.expect("flight 1L: missing T-v cell");
    let close = id(&p.root, &format!("{} CLOSE", cell.key));
    for row in V_AIRCRAFT {
        let name = row.name;
        let centre = row.centre_km;
        // The table leaves the start point open: use the same 10 km south
        // approach as T-r and the preceding single-aircraft probe.
        let flight = p.ai_without_close_delete(
            name, 1, (centre.0 - START_SOUTH_KM, centre.1), centre, cell.arm_s,
        );
        let (plane, entity) = single_aircraft(by_id(&p.root, flight), name);
        p.watch_aircraft_events(name, plane, entity, close, &V_EVENT_TYPES);
        match row.action {
            AllEventsAction::Damage(level) => {
                p.damage_level_at(&format!("{name} DAMAGE"), L_DAMAGE_S, entity, level);
            }
            AllEventsAction::LowFuel => {
                let plane = by_id_mut(&mut p.root, plane);
                assert_eq!(plane.property("Fuel"), Some("1"),
                    "flight 1L {name}: HVAR Fuel changed");
                plane.set_property("Fuel", V_LOW_FUEL);
            }
        }
        p.icon(&format!("{} {name}", cell.key), centre, COALITION_NATO);
    }
}

fn generated_w_group(row: &TemplateProbeRow, next_id: &mut i32) -> Il2Entity {
    let name = row.name;
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(W_INPUT_DIR)
        .join(format!("{name}.Group"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("flight 1L {name}: cannot read frozen input {}: {error}", path.display()));
    let generated = parse_group_file(&source)
        .unwrap_or_else(|error| panic!("flight 1L {name}: cannot parse frozen input {}: {error}", path.display()));
    let anchor = named(&generated, "Zone IN").pos_xz()
        .unwrap_or_else(|| panic!("flight 1L {name}: Zone IN missing position"));
    let (mut copy, _) = crate::duplicate::duplicate_template(&generated, next_id);
    crate::placement::move_anchor_to(&mut copy, anchor, world(row.centre_km));
    crate::recon::silence_clone_starts(&mut copy);
    apply_w_changes_after_generation(&mut copy, row);
    copy
}

fn apply_w_changes_after_generation(group: &mut Il2Entity, row: &TemplateProbeRow) {
    let name = row.name;
    let entities = block_ids(group, "MCU_TR_Entity");
    assert_eq!(entities.len(), 2, "flight 1L {name}: expected two unit entities");
    for change in row.changes_after_generation {
        match change {
            TemplateChange::Cooldown60Seconds => {
                let cooldown = id(group, TEMPLATE_COOLDOWN);
                by_id_mut(group, cooldown).set_property("Time", W_COOLDOWN_S.to_string());
            }
            TemplateChange::DeathEventKilled => {
                let death = id(group, TEMPLATE_DEATH_COUNT).to_string();
                let generated_event = EVENT_PLANE_DESTROYED.to_string();
                for &entity in &entities {
                    let mut changed = 0;
                    by_id_mut(group, entity).for_each_mut(&mut |event| {
                        if event.block_type == "OnEvent" && event.property("TarId") == Some(death.as_str()) {
                            assert_eq!(event.property("Type"), Some(generated_event.as_str()),
                                "flight 1L {name}: generated DeathCount event changed");
                            event.set_property("Type", EVENT_KILLED.to_string());
                            changed += 1;
                        }
                    });
                    assert_eq!(changed, 1, "flight 1L {name}: unit {entity} missing one DeathCount event");
                }
            }
            TemplateChange::DeleteListsEntities => {
                let delete = id(group, TEMPLATE_TRIGGER_DELETE);
                by_id_mut(group, delete).set_objects(entities.clone());
            }
        }
    }
}

fn build_w(p: &mut Probe) {
    let cell = p.cell.expect("flight 1L: missing T-w cell");
    for row in W_GROUPS {
        let name = row.name;
        let group = generated_w_group(row, &mut p.next_id);
        let bring_up_name = match row.bring_up {
            BringUp::Spawn => TEMPLATE_SPAWN_UNITS,
            BringUp::Activate => TEMPLATE_MISSION_BEGIN,
        };
        let mission_end = id(&group, TEMPLATE_MISSION_END);
        assert_eq!(group.count_block_type("MCU_CheckZone"), 2,
            "flight 1L {name}: expected two generated check zones");
        let group_id = p.start_group_at(group, bring_up_name, cell.arm_s);
        let entities = p.watch_formation(name, group_id);
        let outputs: &[&str] = match row.bring_up {
            BringUp::Spawn => &[TEMPLATE_DEATH_COUNT, TEMPLATE_COOLDOWN, TEMPLATE_SPAWN_COUNT],
            BringUp::Activate => {
                for absent in [TEMPLATE_DEATH_COUNT, TEMPLATE_SPAWN_COUNT] {
                    assert!(by_id(&p.root, group_id).find_by_name(absent).is_none(),
                        "flight 1L {name}: Activate group unexpectedly contains {absent}");
                }
                &[TEMPLATE_COOLDOWN]
            }
        };
        for output in outputs {
            let source = id(by_id(&p.root, group_id), output);
            p.observe(source, &format!("{name} {output} fired"));
        }
        match row.schedule {
            TemplateSchedule::KillAndDetect => {
                for (i, &entity) in entities.iter().enumerate() {
                    p.damage_at(&format!("{name}{} DAMAGE", wingman_letter(i)), W_CHANGE_S, entity);
                }
            }
            TemplateSchedule::CleanupAndRestart => {
                let bring_up = id(by_id(&p.root, group_id), bring_up_name);
                p.at(&format!("{name} END"), W_CHANGE_S, &[mission_end]);
                p.at(&format!("{name} RESTART"), W_RESTART_S, &[bring_up]);
            }
        }
        p.detect_formation_at(name, &entities, W_DETECT_S);
        p.icon(&format!("{} {name}", cell.key), row.centre_km, COALITION_NATO);
    }
}

fn m_node_id(root: &Il2Entity, group_name: &str, node_name: &str) -> i32 {
    probe_node_id(root, "1M", group_name, node_name)
}

fn probe_node_id(root: &Il2Entity, flight: &str, group_name: &str, node_name: &str) -> i32 {
    root.find_by_name(node_name)
        .unwrap_or_else(|| panic!("flight {flight} {group_name}: missing node {node_name}"))
        .index.unwrap_or_else(|| panic!("flight {flight} {group_name}: node {node_name} missing Index"))
}

fn frozen_m_group(
    name: &str,
    directory: &str,
    file: &str,
    centre_km: (f64, f64),
    next_id: &mut i32,
) -> Il2Entity {
    frozen_probe_group("1M", name, directory, file, centre_km, next_id)
}

fn frozen_probe_group(
    flight: &str,
    name: &str,
    directory: &str,
    file: &str,
    centre_km: (f64, f64),
    next_id: &mut i32,
) -> Il2Entity {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(directory).join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("flight {flight} {name}: cannot read {}: {error}", path.display()));
    let source = parse_group_file(&text)
        .unwrap_or_else(|error| panic!("flight {flight} {name}: cannot parse {}: {error}", path.display()));
    let zone = probe_node_id(&source, flight, name, TEMPLATE_ZONE_IN);
    let anchor = by_id(&source, zone).pos_xz()
        .unwrap_or_else(|| panic!("flight {flight} {name}: Zone IN missing position"));
    probe_node_id(&source, flight, name, TEMPLATE_TRANSLATOR_MISSION_BEGIN);
    let (mut copy, _) = crate::duplicate::duplicate_template(&source, next_id);
    crate::placement::move_anchor_to(&mut copy, anchor, world(centre_km));
    crate::recon::silence_clone_starts(&mut copy);
    copy
}

fn build_u(p: &mut Probe) {
    build_upstream_rows(p, "1M", U_GROUPS);
}

fn remove_probe_event_hook(copy: &mut Il2Entity, entity: i32, hook: (i32, i32), context: &str) {
    let mut removed = 0;
    by_id_mut(copy, entity).for_each_mut(&mut |node| {
        if node.block_type == "OnEvents" {
            node.children.retain(|event| {
                let remove = read_event(event) == Some(hook);
                removed += usize::from(remove);
                !remove
            });
        }
    });
    assert_eq!(removed, 1, "{context}: entity {entity} missing one event {} hook to {}", hook.0, hook.1);
}

fn retarget_probe_event_hook(
    copy: &mut Il2Entity, entity: i32, hook: (i32, i32), target: i32, context: &str,
) {
    let mut changed = 0;
    by_id_mut(copy, entity).for_each_mut(&mut |event| {
        if read_event(event) == Some(hook) {
            event.set_property("TarId", target.to_string());
            changed += 1;
        }
    });
    assert_eq!(changed, 1, "{context}: entity {entity} missing one event {} hook to {}", hook.0, hook.1);
}

fn apply_upstream_change(
    flight: &str, row: &UpstreamProbeRow, copy: &mut Il2Entity, entities: &[i32], next_id: &mut i32,
) {
    let name = row.name;
    let context = format!("flight {flight} {name}");
    let home = probe_node_id(copy, flight, name, row.orders.home_timer());
    if row.change.removes_event7_hooks() {
        for &entity in entities {
            remove_probe_event_hook(copy, entity, (EVENT_PLANE_BINGO_FUEL, home), &context);
        }
    }
    if row.change.gates_aborts() {
        let (x, z) = by_id(copy, home).pos_xz()
            .unwrap_or_else(|| panic!("{context}: {} missing position", row.orders.home_timer()));
        // The same existing counter builder as Probe::count; keep its shape.
        let mut gate = counter(HOME_ONCE_GATE, 1, 0, next_id, x, z);
        gate.set_targets(vec![home]);
        let gate_id = gate.index.expect("probe counter missing Index");
        let aborts: Vec<_> = row.orders.abort_events().into_iter()
            .filter(|&event| event != EVENT_PLANE_BINGO_FUEL || !row.change.removes_event7_hooks())
            .collect();
        for &entity in entities {
            for &event in &aborts {
                retarget_probe_event_hook(copy, entity, (event, home), gate_id, &context);
            }
        }
        let logic = probe_node_id(copy, flight, name, TEMPLATE_LOGIC);
        by_id_mut(copy, logic).children.push(gate);
    }
    if row.change.removes_home_force_complete() {
        let force = probe_node_id(copy, flight, name, TEMPLATE_FORCE_COMPLETE_HIGH);
        let home = by_id_mut(copy, home);
        assert_eq!(home.targets.iter().filter(|&&target| target == force).count(), 1,
            "{context}: {} missing one Force Complete target", row.orders.home_timer());
        home.set_targets(home.targets.iter().copied().filter(|&target| target != force).collect());
    }
}

fn build_upstream_rows(p: &mut Probe, flight: &str, rows: &[UpstreamProbeRow]) {
    let cell = p.cell.unwrap_or_else(|| panic!("flight {flight}: missing upstream probe cell"));
    let close = id(&p.root, &format!("{} CLOSE", cell.key));
    for row in rows {
        let name = row.name;
        // The named wrapper is probe-owned; even the source Group Name stays intact.
        let mut wrapper = group(name, &mut p.next_id);
        let mut copy = frozen_probe_group(flight, name, U_INPUT_DIR, row.source_file, row.centre_km, &mut p.next_id);
        let planes = block_ids(&copy, "Plane");
        let entities = block_ids(&copy, "MCU_TR_Entity");
        assert_eq!(planes.len(), 4, "flight {flight} {name}: expected four Plane blocks");
        assert_eq!(entities.len(), 4, "flight {flight} {name}: expected four plane entities");
        apply_upstream_change(flight, row, &mut copy, &entities, &mut p.next_id);
        probe_node_id(&copy, flight, name, TEMPLATE_MISSION_BEGIN);
        let outputs: Vec<_> = row.orders.timers().iter().map(|&timer_name| {
            let index = probe_node_id(&copy, flight, name, timer_name);
            assert_eq!(by_id(&copy, index).block_type, "MCU_Timer",
                "flight {flight} {name}: {timer_name} must be a timer");
            (timer_name, index)
        }).collect();
        wrapper.children.push(copy);
        p.start_group_at(wrapper, TEMPLATE_MISSION_BEGIN, cell.arm_s);
        for ((plane, entity), suffix) in planes.into_iter().zip(entities).zip(FOUR_SHIP_SUFFIXES) {
            let label = format!("{name}{suffix}");
            by_id_mut(&mut p.root, plane).set_name(&label);
            p.listen_aircraft_events(&label, entity, close, &row.orders.event_types());
        }
        for (timer_name, source) in outputs {
            p.observe(source, &format!("{name} {timer_name} fired"));
        }
        p.icon(&format!("{} {name}", cell.key), row.centre_km, COALITION_NATO);
    }
}

fn current_x_group(row: &GeneratedProbeRow) -> Il2Entity {
    let name = row.name;
    let unit = builtin_plane_catalog().into_iter()
        .find(|unit| crate::model_spec::script_id(&unit.script) == "f80c10")
        .expect("flight 1M T-x: built-in catalog missing F-80C f80c10");
    let seats = (0..2).map(|_| {
        let mut seat = TemplateSeat::new(unit.clone());
        seat.role = FlightRole::Independent;
        seat.country = COUNTRY_USA;
        seat.altitude = X_ALTITUDE_M;
        seat
    }).collect();
    generate_template(&TemplateOptions {
        name: name.into(),
        seats,
        bring_up: row.bring_up,
        allow_multiple_spawns: row.allow_multiple_spawns,
        spawn_cooldown_min: X_COOLDOWN_MIN,
        ..TemplateOptions::default()
    }).unwrap_or_else(|error| panic!("flight 1M {name}: generate_template failed: {error}"))
}

fn build_x(p: &mut Probe) {
    let cell = p.cell.expect("flight 1M: missing T-x cell");
    let close = id(&p.root, &format!("{} CLOSE", cell.key));
    for row in X_GROUPS {
        let name = row.name;
        let copy = frozen_m_group(name, X_INPUT_DIR, &format!("{name}.Group"), row.centre_km, &mut p.next_id);
        let bring_up_name = match row.bring_up {
            BringUp::Spawn => TEMPLATE_SPAWN_UNITS,
            BringUp::Activate => TEMPLATE_MISSION_BEGIN,
        };
        let bring_up = m_node_id(&copy, name, bring_up_name);
        let mission_end = m_node_id(&copy, name, TEMPLATE_MISSION_END);
        let planes = block_ids(&copy, "Plane");
        let entities = block_ids(&copy, "MCU_TR_Entity");
        assert_eq!(planes.len(), 2, "flight 1M {name}: expected two Plane blocks");
        assert_eq!(entities.len(), 2, "flight 1M {name}: expected two unit entities");
        let group_id = p.start_group_at(copy, bring_up_name, cell.arm_s);
        for (i, (&plane, &entity)) in planes.iter().zip(&entities).enumerate() {
            let label = format!("{name}{}", wingman_letter(i));
            by_id_mut(&mut p.root, plane).set_name(&label);
            p.listen_aircraft_events(&label, entity, close, &[EVENT_KILLED]);
            if matches!(row.schedule, TemplateSchedule::KillAndDetect) {
                p.damage_at(&format!("{label} DAMAGE"), X_CHANGE_S, entity);
            }
            p.detect_aircraft(&label, entity, X_DETECT_S);
        }
        let outputs: &[&str] = match row.bring_up {
            BringUp::Spawn => &[TEMPLATE_DEATH_COUNT, TEMPLATE_COOLDOWN, TEMPLATE_SPAWN_COUNT],
            BringUp::Activate => &[TEMPLATE_COOLDOWN],
        };
        for &output in outputs {
            let source = m_node_id(by_id(&p.root, group_id), name, output);
            p.observe(source, &format!("{name} {output} fired"));
        }
        if matches!(row.schedule, TemplateSchedule::CleanupAndRestart) {
            p.at(&format!("{name} END"), X_CHANGE_S, &[mission_end]);
            p.at(&format!("{name} RESTART"), X_RESTART_S, &[bring_up]);
        }
        p.icon(&format!("{} {name}", cell.key), row.centre_km, COALITION_NATO);
    }
}

fn placed_y_flight(row: &LargerRtbRow, next_id: &mut i32) -> Il2Entity {
    let name = row.name;
    let mut source = parse_group_file(HVAR)
        .unwrap_or_else(|error| panic!("flight 1M {name}: HVAR cannot parse: {error}"));
    let start = world((row.centre_km.0 - START_SOUTH_KM, row.centre_km.1));
    // RTB has no table coordinate: use the departure point. The common probe
    // cuts leave this exit chain dormant until explicitly asked to end.
    super::place::place_air_sortie(&mut source, &super::place::SortiePlacement {
        target: world(row.centre_km),
        heading_deg: Y_HEADING_DEG,
        spawn_dist_m: Y_SPAWN_DISTANCE_M,
        rtb: start,
    }).unwrap_or_else(|error| panic!("flight 1M {name}: place_air_sortie failed: {error}"));
    let (mut copy, _) = crate::duplicate::duplicate_template(&source, next_id);
    cut_probe_flight_logic(&mut copy);
    copy
}

fn build_larger_rtb(p: &mut Probe) {
    let cell = p.cell.expect("flight 1M: missing T-y cell");
    let close = id(&p.root, &format!("{} CLOSE", cell.key));
    for row in Y_FLIGHTS {
        let name = row.name;
        let mut copy = match row.builder {
            RtbFlightBuilder::PlaceAirSortie => placed_y_flight(row, &mut p.next_id),
            RtbFlightBuilder::ProbeFlight => probe_flight(HVAR, row.aircraft.len(),
                world((row.centre_km.0 - START_SOUTH_KM, row.centre_km.1)),
                world(row.centre_km), &mut p.next_id),
        };
        copy.set_name(name);
        let planes = block_ids(&copy, "Plane");
        let entities = block_ids(&copy, "MCU_TR_Entity");
        assert_eq!(planes.len(), row.aircraft.len(), "flight 1M {name}: missing Plane blocks");
        assert_eq!(entities.len(), row.aircraft.len(), "flight 1M {name}: missing plane entities");
        m_node_id(&copy, name, TEMPLATE_MISSION_BEGIN);
        p.start_group_at(copy, TEMPLATE_MISSION_BEGIN, cell.arm_s);
        for ((&plane, &entity), &label) in planes.iter().zip(&entities).zip(row.aircraft) {
            p.watch_aircraft(label, plane, entity, close);
            p.detect_aircraft(label, entity, Y_DETECT_S);
        }
        p.damage_level_at(&format!("{} DAMAGE", row.aircraft[Seat::Lead.index()]),
            Y_DAMAGE_S, entities[Seat::Lead.index()], row.damage);
        p.delete_entities_at(name, &entities, Y_REMOVE_S);
        p.icon(&format!("{} {name}", cell.key), row.centre_km, COALITION_NATO);
    }
}

fn current_p_pack() -> Il2Entity {
    let mut source = crate::pack::builtin_template()
        .unwrap_or_else(|error| panic!("flight {P_FLIGHT} T-f: cannot load Fighter Pack template: {error}"));
    crate::flights::configure_aircraft(&mut source, &crate::flights::FlightConfig {
        flight_count: 1,
        max_in_flight: P_PACK_AIRCRAFT,
        type_ids: vec!["f80c10".into()],
        type_skills: vec![P_PACK_SKILL],
        country: COUNTRY_USA,
        ..crate::flights::FlightConfig::default()
    }).unwrap_or_else(|error| panic!("flight {P_FLIGHT} T-f: configure_aircraft failed: {error}"));
    // Complete the app's normal generation path: rebuild NodeGates for one
    // pack group, retaining the generated orders and per-aircraft cleanup.
    let pack = crate::pack::generate_pack(&source, 1)
        .unwrap_or_else(|error| panic!("flight {P_FLIGHT} T-f: generate_pack failed: {error}"));
    validate_p_pack(&pack, "frozen T-f input");
    pack
}

fn validate_p_pack(pack: &Il2Entity, name: &str) {
    let defaults = crate::flights::FlightConfig::default();
    let node = |node_name: &str| probe_node_id(pack, P_FLIGHT, name, node_name);
    let death = node(TEMPLATE_DEATH_COUNT);
    let cooldown = node(TEMPLATE_COOLDOWN);
    let input = node(PACK_RANDOMIZER_INPUT);
    let spawn = node(PACK_SPAWN);
    let planes = block_ids(pack, "Plane");
    let entities = block_ids(pack, "MCU_TR_Entity");
    assert_eq!(planes.len(), P_PACK_AIRCRAFT as usize,
        "flight {P_FLIGHT} {name}: expected {P_PACK_AIRCRAFT} Plane blocks");
    assert_eq!(entities.len(), P_PACK_AIRCRAFT as usize,
        "flight {P_FLIGHT} {name}: expected {P_PACK_AIRCRAFT} aircraft entities");
    assert_eq!(by_id(pack, death).property("Counter"), Some(entities.len().to_string().as_str()),
        "flight {P_FLIGHT} {name}: DeathCount must count every aircraft entity");
    assert_eq!(by_id(pack, death).property("Dropcount"), Some(P_PACK_DEATH_COUNT_RESET), "flight {P_FLIGHT} {name}: DeathCount reset");
    assert_eq!(by_id(pack, death).targets, [cooldown], "flight {P_FLIGHT} {name}: DeathCount starts COOLDOWN");
    assert_eq!(by_id(pack, cooldown).property("Time"), Some(defaults.cooldown.to_string().as_str()),
        "flight {P_FLIGHT} {name}: COOLDOWN time");
    assert_eq!(by_id(pack, cooldown).targets, [input], "flight {P_FLIGHT} {name}: replacement must not need a zone/player");
    assert_eq!(by_id(pack, input).targets,
        [node(PACK_CLOSE_INPUT), node(PACK_WAIT_FOR_OUTPUT), node(PACK_RANDOM_FLIGHT)],
        "flight {P_FLIGHT} {name}: replacement randomizer changed");
    assert_eq!(by_id(pack, node(PACK_RANDOM_FLIGHT)).property("Random"), Some(P_PACK_RANDOM_PERCENT),
        "flight {P_FLIGHT} {name}: single flight must always be selected");
    assert_eq!(by_id(pack, node(PACK_RANDOM_FLIGHT)).targets, [node(PACK_RANDOMIZER_OUTPUT)],
        "flight {P_FLIGHT} {name}: missing randomizer output");
    assert_eq!(by_id(pack, node(PACK_RANDOMIZER_OUTPUT)).targets, [spawn], "flight {P_FLIGHT} {name}: missing replacement Spawn 1");
    assert_eq!(by_id(pack, spawn).objects, entities, "flight {P_FLIGHT} {name}: Spawn 1 aircraft");
    assert_eq!(by_id(pack, node(TEMPLATE_SPAWN_UNITS)).targets, [input], "flight {P_FLIGHT} {name}: bring-up");
    for (seat, &entity) in entities.iter().enumerate() {
        let n = seat + 1;
        let delay = node(&format!("Delay Delete {n}"));
        let done = node(&format!("Mission Complete {n}"));
        let cleanup = node(&format!("CLEANUP {n}"));
        assert!(by_id(pack, entity).targets.is_empty(), "flight {P_FLIGHT} {name}: aircraft {n} must be Independent");
        let events = probe_events(by_id(pack, entity));
        assert_eq!(events.iter().filter(|&&(event, _)| event == EVENT_KILLED).copied().collect::<Vec<_>>(),
            [(EVENT_KILLED, delay)], "flight {P_FLIGHT} {name}: aircraft {n} missing 0016 death hook");
        assert_eq!(events.iter().filter(|&&(event, _)| event == EVENT_PLANE_BINGO_FUEL).copied().collect::<Vec<_>>(),
            [(EVENT_PLANE_BINGO_FUEL, done)], "flight {P_FLIGHT} {name}: aircraft {n} missing event 7 hook");
        assert_eq!(by_id(pack, done).targets, [cleanup], "flight {P_FLIGHT} {name}: aircraft {n} Mission Complete");
        assert_eq!(by_id(pack, cleanup).targets,
            [node(&format!("Force Complete {n}")), node(&format!("RTB DELAY {n}")), delay],
            "flight {P_FLIGHT} {name}: aircraft {n} CLEANUP");
        assert_eq!(by_id(pack, delay).property("Time"), Some(defaults.delete_orders.to_string().as_str()),
            "flight {P_FLIGHT} {name}: aircraft {n} clean-up delay");
        assert_eq!(by_id(pack, delay).targets, [node(&format!("Deactivate {n}")), death],
            "flight {P_FLIGHT} {name}: aircraft {n} clean-up count");
    }
}

fn current_b_aircraft(row: &BingoAircraftRow) -> Il2Entity {
    let name = row.name;
    let unit = bundled_catalog().into_iter()
        .find(|unit| unit.kind == UnitKind::Plane && crate::model_spec::script_id(&unit.script) == row.script)
        .unwrap_or_else(|| panic!("flight {P_FLIGHT} T-b {name}: bundled catalog missing plane script {}", row.script));
    let mut seat = TemplateSeat::new(unit);
    seat.role = FlightRole::Independent;
    apply_plane_start(&mut seat, PlaneStart::Air);
    generate_template(&TemplateOptions {
        name: name.into(),
        seats: vec![seat],
        bring_up: BringUp::Activate,
        ..TemplateOptions::default()
    }).unwrap_or_else(|error| panic!("flight {P_FLIGHT} T-b {name} ({}): generate_template failed: {error}", row.script))
}

fn build_fighter_count(p: &mut Probe) {
    let cell = p.cell.unwrap_or_else(|| panic!("flight {P_FLIGHT}: missing T-f cell"));
    let close = id(&p.root, &format!("{} CLOSE", cell.key));
    for row in P_FIGHTERS {
        let name = row.name;
        let context = format!("flight {P_FLIGHT} T-f {name}");
        let mut wrapper = group(name, &mut p.next_id);
        let mut copy = frozen_probe_group(P_FLIGHT, name, P_INPUT_DIR, P_PACK_FILE, row.centre_km, &mut p.next_id);
        validate_p_pack(&copy, name);
        let planes = block_ids(&copy, "Plane");
        let entities = block_ids(&copy, "MCU_TR_Entity");
        let death = probe_node_id(&copy, P_FLIGHT, name, TEMPLATE_DEATH_COUNT);
        for (seat, &entity) in entities.iter().enumerate() {
            let n = seat + 1;
            if row.change.removes_event7_hooks() {
                let done = probe_node_id(&copy, P_FLIGHT, name, &format!("Mission Complete {n}"));
                remove_probe_event_hook(&mut copy, entity, (EVENT_PLANE_BINGO_FUEL, done), &context);
            }
            if row.change.counts_deaths_directly() {
                let delay = probe_node_id(&copy, P_FLIGHT, name, &format!("Delay Delete {n}"));
                retarget_probe_event_hook(&mut copy, entity, (EVENT_KILLED, delay), death, &context);
            }
        }
        wrapper.children.push(copy);
        let group_id = p.start_group_at(wrapper, TEMPLATE_SPAWN_UNITS, cell.arm_s);
        let mut damage_commands = Vec::new();
        for ((plane, entity), suffix) in planes.into_iter().zip(entities.iter().copied()).zip(FOUR_SHIP_SUFFIXES) {
            let label = format!("{name}{suffix}");
            by_id_mut(&mut p.root, plane).set_name(&label);
            p.listen_aircraft_events(&label, entity, close, &[EVENT_KILLED]);
            damage_commands.push(p.damage_command(&format!("{label} DAMAGE"), entity, DamageLevel::Complete));
        }
        // One timer per row, all four Independent aircraft killed on that tick.
        p.at(&format!("{name} DAMAGE AT"), P_KILL_S, &damage_commands);
        for output in [TEMPLATE_DEATH_COUNT, TEMPLATE_COOLDOWN] {
            let source = probe_node_id(by_id(&p.root, group_id), P_FLIGHT, name, output);
            p.observe(source, &format!("{name} {output} fired"));
        }
        // Keep observations after the shared timer to preserve node and locale IDs.
        for n in 1..=entities.len() {
            let output = format!("Delay Delete {n}");
            let source = probe_node_id(by_id(&p.root, group_id), P_FLIGHT, name, &output);
            p.observe(source, &format!("{name} {output} fired"));
        }
        p.icon(&format!("{} {name}", cell.key), row.centre_km, COALITION_NATO);
    }
}

fn build_bingo_aircraft(p: &mut Probe) {
    let cell = p.cell.unwrap_or_else(|| panic!("flight {P_FLIGHT}: missing T-b cell"));
    let close = id(&p.root, &format!("{} CLOSE", cell.key));
    for row in B_AIRCRAFT {
        let name = row.name;
        let copy = frozen_probe_group(P_FLIGHT, name, P_INPUT_DIR, &format!("{name}.Group"), row.centre_km, &mut p.next_id);
        let (plane, entity) = single_aircraft(&copy, &format!("flight {P_FLIGHT} T-b {name}"));
        probe_node_id(&copy, P_FLIGHT, name, TEMPLATE_MISSION_BEGIN);
        p.start_group_at(copy, TEMPLATE_MISSION_BEGIN, cell.arm_s);
        by_id_mut(&mut p.root, plane).set_name(name);
        p.listen_aircraft_events(name, entity, close, &B_EVENT_TYPES);
        p.icon(&format!("{} {name}", cell.key), row.centre_km, COALITION_NATO);
    }
}

fn build_sortie_probe(p: &mut Probe) {
    let cell = p.cell.expect("flight 2 sortie cell");
    let row = TWO_SORTIES.iter().find(|row| row.cell == cell.key)
        .unwrap_or_else(|| panic!("flight 2 {}: missing sortie row", cell.key));
    let (shape, shell) = add_sortie_probe(p, row);
    p.close_node(shape.zone_in);
    p.close_node(shape.zone_out);
    p.switch_at("MCU_Activate", &format!("{} AREA HOT ON", cell.key), TWO_HOT_S, &[shell.hot]);
    p.switch_at("MCU_Deactivate", &format!("{} AREA COLD OFF", cell.key), TWO_HOT_S, &[shell.cold]);
    let extend = id(&p.root, &format!("{} EXTEND", row.prefix));
    for (source, suffix) in [
        (shell.start, "START"), (shell.done_hub, "DONE HUB"), (extend, "EXTEND"),
        // Delete is a command sink; observe its issuing timer at the same instant.
        (shell.delete_timer, "DELETE"),
    ] {
        p.observe(source, &format!("{} {suffix}", cell.key));
    }
    let close = id(&p.root, &format!("{} CLOSE", cell.key));
    for (seat, &entity) in shape.planes.iter().enumerate() {
        let delete = p.delete(&format!("{} aircraft {} CLOSE DELETE", cell.key, seat + 1), &[entity]);
        p.link(close, delete);
    }
    p.icon(cell.key, cell.centre_km, COALITION_NATO);
}

fn sortie_probe_source(row: &SortieProbe) -> Il2Entity {
    let context = format!("{} ({})", row.cell, row.file);
    let text = super::library::BUILTINS.iter().find(|(file, _)| *file == row.file)
        .unwrap_or_else(|| panic!("{context}: missing built-in sortie")).1;
    parse_group_file(text).unwrap_or_else(|e| panic!("{context}: built-in sortie failed to parse: {e}"))
}

fn add_sortie_probe(p: &mut Probe, row: &SortieProbe) -> (super::library::SortieShape, super::shell::ShellIds) {
    add_sortie_probe_source(p, row, sortie_probe_source(row))
}

fn add_sortie_probe_source(
    p: &mut Probe,
    row: &SortieProbe,
    mut copy: Il2Entity,
) -> (super::library::SortieShape, super::shell::ShellIds) {
    use super::library::inspect_sortie;
    use super::place::{SortiePlacement, place_air_sortie};
    use super::shell::{ShellSpec, wrap_sortie};
    use super::{SideLogic, Strategies};

    let context = format!("{} {} ({})", p.root.name().unwrap_or("<unnamed flight>"), row.cell, row.file);
    let cell = p.cell.unwrap_or_else(|| panic!("{context}: missing sortie cell"));
    let facts = inspect_sortie(&copy).unwrap_or_else(|e| panic!("{context}: source shape: {e:?}"));
    let target = world(cell.centre_km);
    place_air_sortie(&mut copy, &SortiePlacement {
        target,
        heading_deg: 0.0,
        spawn_dist_m: facts.wp_speed_kmh / 3.6 * TWO_TRANSIT_S,
        rtb: K14_ORIGIN,
    }).unwrap_or_else(|e| panic!("{context}: placement: {e}"));
    let (mut copy, _) = crate::duplicate::duplicate_template(&copy, &mut p.next_id);
    copy.set_name(&format!("{} sortie", cell.key));
    let shape = inspect_sortie(&copy).unwrap_or_else(|e| panic!("{context}: placed shape: {e:?}"));
    let strategies = Strategies::default();
    let shell = wrap_sortie(&mut copy, row.prefix, &shape, &ShellSpec {
        attack_time_s: shape.attack_time_s,
        final_time_s: TWO_TRANSIT_S + shape.attack_time_s + TWO_MARGIN_S + TWO_EXTENSION_S,
        extension_s: TWO_EXTENSION_S,
        target,
        census_delay_s: super::budget::census_delay(strategies.census, SORTIE_PROBE_COPY_ORDER),
        zone_out: strategies.zone_out,
        losses: strategies.losses,
    }, &mut p.next_id).unwrap_or_else(|e| panic!("{context}: shell: {e}"));
    let mut logic = SideLogic::default();
    shell.register(SORTIE_PROBE_AREA, &mut logic);
    copy.children.extend(super::budget::build_side(
        facts.side, SORTIE_PROBE_BUDGET, &mut logic, &strategies, &mut p.next_id,
    ));
    let init = super::assemble::emit_side_logic(
        facts.side, &logic, std::slice::from_mut(&mut copy), &mut p.next_id,
    ).unwrap_or_else(|e| panic!("{context}: side logic: {e}"));
    copy.children.extend(init);
    p.add(copy);
    p.link(p.arm, shell.start);
    (shape, shell)
}

fn build_sortie_reflight(p: &mut Probe) {
    let cell = p.cell.expect("flight 2B sortie cell");
    let row = TWO_B_SORTIES.iter().find(|row| row.sortie.cell == cell.key)
        .unwrap_or_else(|| panic!("flight 2B {}: missing sortie row", cell.key));
    let (shape, shell) = add_sortie_probe(p, &row.sortie);
    observe_sortie_probe(p, row.sortie.prefix, &shape);
    if let Some(time_s) = row.hot_at_s {
        p.switch_at("MCU_Activate", &format!("{} AREA HOT ON", cell.key), time_s, &[shell.hot]);
        p.switch_at("MCU_Deactivate", &format!("{} AREA COLD OFF", cell.key), time_s, &[shell.cold]);
    }
    if let Some(enemy) = &row.enemy {
        build_reflight_enemy(p, enemy);
    }
    close_sortie_probe(p, shape.side);
}

fn observe_sortie_probe(p: &mut Probe, prefix: &str, shape: &super::library::SortieShape) {
    let cell = p.cell.expect("sortie observation cell");
    for suffix in ["START", "LOSSES", "COLD", "EXTEND", "HARD STOP", "ON STATION", "DONE HUB", "DELETE DELAY"] {
        let source = id(&p.root, &format!("{prefix} {suffix}"));
        p.observe(source, &format!("{} {suffix}", cell.key));
        p.close_node(source);
    }
    for (source, suffix) in [
        (shape.zone_in, "Zone IN"), (shape.zone_out, "Zone Out"),
        (shape.path_waypoints[&shape.navigation_lead], "WP 1"),
    ] {
        p.observe(source, &format!("{} {suffix}", cell.key));
        p.close_node(source);
    }
    // S2 has two chained attack timers; inspect_sortie identifies only
    // the final timer of each lead. Trace every timer issuing an AttackArea.
    let copy = named(&p.root, &format!("{} sortie", cell.key));
    let attacks: BTreeSet<_> = block_ids(copy, "MCU_CMD_AttackArea").into_iter().collect();
    let timers: Vec<_> = block_ids(copy, "MCU_Timer").into_iter().filter(|&index| {
        by_id(copy, index).targets.iter().any(|target| attacks.contains(target))
    }).collect();
    for source in timers {
        let name = by_id(&p.root, source).name().unwrap_or_else(|| {
            panic!("{}: attack timer {source} missing Name", cell.key)
        }).to_owned();
        p.observe(source, &format!("{} {name}", cell.key));
        p.close_node(source);
    }
}

fn close_sortie_probe(p: &mut Probe, side: super::AirSide) {
    let cell = p.cell.expect("sortie close cell");
    let close = id(&p.root, &format!("{} CLOSE", cell.key));
    // Include inactive and already-deleted aircraft: one entity per command,
    // covering every Plane still written in this cell, enemies included.
    for (seat, entity) in block_ids(named(&p.root, cell.key), "MCU_TR_Entity").into_iter().enumerate() {
        let delete = p.delete(&format!("{} aircraft {} CLOSE DELETE", cell.key, seat + 1), &[entity]);
        p.link(close, delete);
    }
    p.sources.push(p.source(close, None, Some(cell.close_s)));
    let coalition = match side {
        super::AirSide::Nato => COALITION_NATO,
        super::AirSide::Dprk => COALITION_DPRK,
    };
    p.icon(cell.key, cell.centre_km, coalition);
}

fn empty_target_source(row: &EmptyTargetSortie) -> Il2Entity {
    let mut copy = sortie_probe_source(&row.sortie());
    match row.variant {
        EmptyTargetVariant::C0 | EmptyTargetVariant::C3 | EmptyTargetVariant::BuiltIn { .. }
            | EmptyTargetVariant::BuiltInFormation { .. } => {}
        EmptyTargetVariant::Formation(formation) => set_probe_formation(&mut copy, row.cell, formation),
        EmptyTargetVariant::C1 => {
            copy.for_each_mut(&mut |node| {
                if node.block_type == "Plane" {
                    node.set_property("PayloadId", "2");
                }
            });
        }
        EmptyTargetVariant::C2 => {
            let first_timer = id(&copy, "AttackArea 1");
            let ground = id(&copy, "AttackArea");
            let complete = id(&copy, "Mission Complete 1");
            let wp = id(&copy, "WP 1");
            let second_timer = copy.max_index() + 1;
            let air = second_timer + 1;
            let mut delayed_air = by_id(&copy, first_timer).clone();
            delayed_air.index = Some(second_timer);
            delayed_air.set_property("Index", second_timer.to_string());
            delayed_air.set_name("AttackArea 2");
            delayed_air.set_property("Time", TWO_C_GROUND_TIME_S.to_string());
            delayed_air.set_targets(vec![air, complete]);
            let mut patrol = by_id(&copy, ground).clone();
            patrol.index = Some(air);
            patrol.set_property("Index", air.to_string());
            patrol.set_name("AttackArea air");
            for (key, value) in [("AttackAir", "1"), ("AttackGround", "0"), ("AttackGTargets", "0"), ("Priority", "1")] {
                patrol.set_property(key, value);
            }
            patrol.set_property("AttackArea", TWO_C_AIR_RADIUS_M.to_string());
            patrol.set_property("Time", TWO_C_AIR_TIME_S.to_string());
            by_id_mut(&mut copy, ground).set_property("Time", TWO_C_GROUND_TIME_S.to_string());
            by_id_mut(&mut copy, first_timer).set_targets(vec![ground, second_timer]);
            by_id_mut(&mut copy, wp).set_targets(vec![first_timer, second_timer]);
            let logic = id(&copy, "Logic");
            let orders = id(&copy, "Orders");
            by_id_mut(&mut copy, logic).children.push(delayed_air);
            by_id_mut(&mut copy, orders).children.push(patrol);
        }
    }
    // Fail here, before placement, with inspect_sortie's exact refusal text.
    let shape = super::library::inspect_sortie(&copy).unwrap_or_else(|errors| {
        panic!("empty-target probe {} {:?}: {}", row.cell, row.variant, errors.join("\n"))
    });
    if let EmptyTargetVariant::Formation(_) = row.variant {
        assert_eq!(shape.kind, super::library::TestedShape::S1, "flight 2D {}: source shape", row.cell);
        assert_eq!(shape.attack_time_s, TWO_D_ATTACK_TIME_S, "flight 2D {}: attack time", row.cell);
    }
    if let EmptyTargetVariant::BuiltIn { file, shape: expected_shape, attack_time_s } = row.variant {
        assert_eq!(shape.kind, expected_shape, "flight 2E {} ({file}): source shape", row.cell);
        assert_eq!(shape.attack_time_s, f64::from(attack_time_s), "flight 2E {} ({file}): attack time", row.cell);
    }
    if let EmptyTargetVariant::BuiltInFormation { file, shape: expected_shape, attack_time_s, .. } = row.variant {
        assert_eq!(shape.kind, expected_shape, "flight 2F {} ({file}): source shape", row.cell);
        assert_eq!(shape.attack_time_s, f64::from(attack_time_s), "flight 2F {} ({file}): attack time", row.cell);
    }
    copy
}

fn set_probe_formation(copy: &mut Il2Entity, cell: &str, formation: ProbeFormation) {
    assert_eq!(copy.count_block_type("MCU_CMD_Formation"), 1,
        "flight 2D {cell}: expected exactly one MCU_CMD_Formation");
    let shape = super::library::inspect_sortie(copy).unwrap_or_else(|errors| {
        panic!("flight 2D {cell}: {}", errors.join("\n"))
    });
    let formations = block_ids(copy, "MCU_CMD_Formation");
    let command = by_id_mut(copy, formations[0]);
    assert_eq!(command.objects, [shape.navigation_lead], "flight 2D {cell}: Formation Objects must be the navigation lead");
    command.set_property("FormationType", formation.formation_type.to_string());
    command.set_property("FormationDensity", formation.formation_density.to_string());
}

fn set_placed_probe_formation(
    copy: &mut Il2Entity,
    cell: &str,
    shape: &super::library::SortieShape,
    formation: ProbeFormation,
) {
    let formations = block_ids(copy, "MCU_CMD_Formation");
    assert!(!formations.is_empty(), "flight 2F {cell}: placed copy missing MCU_CMD_Formation");
    for &index in &formations {
        let command = by_id(copy, index);
        assert!(matches!(command.objects.as_slice(), [lead] if shape.leads.contains(lead)),
            "flight 2F {cell}: Formation {index} Objects {:?} must name one inspected flight lead", command.objects);
    }
    for index in formations {
        let command = by_id_mut(copy, index);
        command.set_property("FormationType", formation.formation_type.to_string());
        command.set_property("FormationDensity", formation.formation_density.to_string());
    }
}

fn build_empty_target_probe(p: &mut Probe, flight: Flight) {
    let cell = p.cell.unwrap_or_else(|| panic!("{}: missing sortie cell", flight.stem()));
    let row = flight.empty_target_sorties().iter().find(|row| row.cell == cell.key)
        .unwrap_or_else(|| panic!("{} {}: missing variant row", flight.stem(), cell.key));
    let (shape, _) = add_sortie_probe_source(p, &row.sortie(), empty_target_source(row));
    if let EmptyTargetVariant::BuiltInFormation { formation, .. } = row.variant {
        // Placement resets air FormationDensity to Safe. Change only the two
        // formation properties on the placed, wrapped copy, retaining its ids.
        let copy = p.root.find_by_name_mut(&format!("{} sortie", cell.key))
            .unwrap_or_else(|| panic!("{} {}: missing placed sortie", flight.stem(), cell.key));
        set_placed_probe_formation(copy, cell.key, &shape, formation);
    }
    observe_sortie_probe(p, row.prefix, &shape);
    close_sortie_probe(p, shape.side);
}

fn build_reflight_enemy(p: &mut Probe, window: &EnemyWindow) {
    let cell = p.cell.expect("flight 2B enemy cell");
    let name = format!("{} enemy", cell.key);
    let context = format!("flight 2B {name}");
    let source = parse_group_file(TWO_B_ENEMY)
        .unwrap_or_else(|e| panic!("{context}: built-in Yak source failed to parse: {e}"));
    let shape = super::library::inspect_sortie(&source)
        .unwrap_or_else(|e| panic!("{context}: Yak template shape {e:?}"));
    let pair = shape.planes.get(..TWO_B_ENEMY_AIRCRAFT)
        .unwrap_or_else(|| panic!("{context}: missing {TWO_B_ENEMY_AIRCRAFT}-aircraft pair"));
    assert_eq!(pair[0], shape.navigation_lead, "{context}: first pair must have the navigation lead");
    let mut flight = source.clone();
    flight.set_name(&name);
    flight.children = pair.iter().flat_map(|&entity| [
        by_id(&source, shape.plane_objects[&entity]).clone(), by_id(&source, entity).clone(),
    ]).collect();
    let start_offset = TWO_B_ORBIT_OFFSETS_KM[0];
    let start = world((cell.centre_km.0 + start_offset.0, cell.centre_km.1 + start_offset.1));
    let pivot = by_id(&flight, shape.plane_objects[&pair[0]]).pos_xz()
        .unwrap_or_else(|| panic!("{context}: navigation lead Plane missing XPos/ZPos"));
    crate::placement::rotate_tree(&mut flight, pivot, TWO_B_ENEMY_ROTATION_DEG);
    crate::placement::move_anchor_to(&mut flight, pivot, start);
    flight.for_each_mut(&mut |node| {
        if matches!(node.block_type.as_str(), "Plane" | "MCU_TR_Entity") {
            node.set_property("YPos", format!("{TWO_B_ENEMY_ALTITUDE_M:.3}"));
        }
        if node.block_type == "Plane" {
            node.set_property("Country", COUNTRY_DPRK.to_string());
            node.set_property("StartType", PlaneStart::Air.as_i32().to_string());
            node.set_property("AiRTBDecision", AiRtbDecision::Disabled.property());
            // Also set the shipped Plane property: Filter's meaning is not
            // documented in the repository, so do not depend on it alone.
            node.set_property("Engageable", "0");
        }
    });
    let (flight, _) = crate::duplicate::duplicate_template(&flight, &mut p.next_id);
    let entities = block_ids(&flight, "MCU_TR_Entity");
    let lead = *entities.first()
        .unwrap_or_else(|| panic!("{context}: missing lead MCU_TR_Entity after duplication"));
    p.add(flight);
    let activate = p.control("MCU_Activate", &format!("{name} ACTIVATE"), &[]);
    by_id_mut(&mut p.root, activate).set_objects(entities.clone());
    let mut behaviour = p.reference_mcu("MCU_CMD_Behaviour", &format!("{name} NON TARGETABLE"));
    // Shipped Filter = 8 is retained. Neither template.rs nor the repository's
    // docs/reference files say what it selects; its effect remains unverified.
    behaviour.set_property("Engageable", "0");
    behaviour.set_objects(entities.clone());
    let behaviour = p.add(behaviour);
    let behaviour_delay = p.relay(&format!("{name} BEHAVIOUR DELAY"), TWO_B_BEHAVIOUR_DELAY_S);
    p.link(behaviour_delay, behaviour);
    let formation_id = *block_ids(&source, "MCU_CMD_Formation").first()
        .unwrap_or_else(|| panic!("{context}: Yak template missing MCU_CMD_Formation"));
    let mut formation = crate::recon::clone_named(
        by_id(&source, formation_id),
        &mut p.next_id, &format!("{name} FORMATION"),
    );
    formation.set_objects(vec![lead]);
    formation.set_targets(vec![]);
    let formation = p.add(formation);
    let wp_source = by_id(&source, shape.path_waypoints[&shape.navigation_lead]);
    let waypoints: Vec<_> = TWO_B_ORBIT_OFFSETS_KM.iter().enumerate().map(|(i, offset)| {
        let mut wp = crate::recon::clone_named(wp_source, &mut p.next_id, &format!("{name} ORBIT {}", i + 1));
        let point = world((cell.centre_km.0 + offset.0, cell.centre_km.1 + offset.1));
        wp.set_property("XPos", format!("{:.3}", point.0));
        wp.set_property("ZPos", format!("{:.3}", point.1));
        wp.set_property("YPos", format!("{TWO_B_ENEMY_ALTITUDE_M:.3}"));
        wp.set_property("Priority", TWO_B_ORBIT_PRIORITY);
        wp.set_objects(vec![lead]);
        wp.set_targets(vec![]);
        p.add(wp)
    }).collect();
    for (i, &wp) in waypoints.iter().enumerate() {
        p.link(wp, waypoints[(i + 1) % waypoints.len()]);
        p.close_node(wp);
    }
    let orders = p.relay(&format!("{name} ORBIT DELAY"), TWO_B_ORDERS_DELAY_S);
    by_id_mut(&mut p.root, orders).set_targets(vec![formation, waypoints[1]]);
    let start = p.at(&format!("{name} ACTIVATE AT"), window.activate_s, &[activate, behaviour_delay, orders]);
    p.observe(start, &format!("{} enemy ACTIVATE", cell.key));
    let deletes: Vec<_> = entities.iter().enumerate().map(|(i, &entity)| {
        p.delete(&format!("{name} {} DELETE", i + 1), &[entity])
    }).collect();
    let delete = p.at(&format!("{name} DELETE AT"), window.delete_s, &deletes);
    p.observe(delete, &format!("{} enemy DELETE", cell.key));
    for source in [behaviour_delay, orders] {
        p.sources.push(p.source(source, None, None));
    }
}

fn build_budget_probe(p: &mut Probe) {
    let cell = p.cell.expect("flight 2 budget cell");
    let row = TWO_BUDGETS.iter().find(|row| row.cell == cell.key)
        .unwrap_or_else(|| panic!("flight 2 {}: missing budget row", cell.key));
    let h = super::budget::tests::harness(
        super::AirSide::Nato, row.limit, row.copies, super::Strategies::default(),
    );
    let (mut tree, map) = crate::duplicate::duplicate_template(&h.root, &mut p.next_id);
    tree.set_name(&format!("{} stand-ins", cell.key));
    let (x, z) = p.position();
    tree.for_each_mut(&mut |node| {
        if node.block_type.starts_with("MCU_") {
            if let Some(name) = node.name() {
                node.set_name(&format!("{} {name}", cell.key));
            }
            node.set_property("XPos", format!("{x:.3}"));
            node.set_property("ZPos", format!("{z:.3}"));
        }
    });
    p.add(tree);
    for request in row.requests {
        let source = p.at(
            &format!("{} REQUEST {} AT", cell.key, request.copy),
            cell.arm_s + request.offset_s,
            &[map[&h.requests[request.copy - 1]]],
        );
        p.observe(source, &format!("{} request {}", cell.key, request.copy));
    }
    for (step, deletion) in row.deletions.iter().enumerate() {
        let targets: Vec<_> = deletion.copies.iter().map(|&copy| map[&h.deletions[copy - 1]]).collect();
        p.at(&format!("{} DELETIONS {} AT", cell.key, step + 1), cell.arm_s + deletion.offset_s, &targets);
    }
    for (seat, (&gate, &deletion)) in h.gates.iter().zip(&h.deletions).enumerate() {
        p.observe(map[&gate], &format!("{} copy {} START", cell.key, seat + 1));
        p.observe(map[&deletion], &format!("{} copy {} deleted", cell.key, seat + 1));
        p.close_node(map[&gate]);
    }
    for &flag in &h.flags {
        p.sources.push(p.source(map[&flag], None, None));
        p.close_node(map[&flag]);
    }
    for suffix in ["CHECK", "CENSUS", "COUNT", "END", "REOPEN OUT", "AGAIN"] {
        if let Some(index) = p.root.find_by_name(&format!("{} BUD N {suffix}", cell.key)).and_then(|node| node.index) {
            p.sources.push(p.source(index, None, None));
        }
    }
}

fn build_no_damage_loss(p: &mut Probe) {
    let cell = p.cell.expect("flight 2 no-damage loss cell");
    let start = world(cell.centre_km);
    let exit = world(LOSS_CRUISE_KM);
    assert!(crate::watermap::TerrainMap::builtin().expect("Korea terrain mask").is_land_xz(start.0, start.1),
        "T-l1: fuel-zero start must be over land");
    // This repository has a WMAP land/water mask, not a measured ridge height.
    // No T-l2 is generated until a point with a sourced elevation is available.
    let mut flight = probe_flight(HVAR, 1, start, exit, &mut p.next_id);
    let mut wp = crate::recon::clone_named(
        by_id(&flight, block_ids(&flight, "MCU_Waypoint")[0]), &mut p.next_id, "T-l1 CRUISE",
    );
    wp.set_targets(vec![]);
    let mut plane = by_id(&flight, block_ids(&flight, "Plane")[0]).clone();
    let entity = by_id(&flight, block_ids(&flight, "MCU_TR_Entity")[0]).clone();
    let entity_id = entity.index.expect("T-l1 entity Index");
    let plane_id = plane.index.expect("T-l1 Plane Index");
    let cruise = crate::model_spec::spec_for(plane.property("Script").unwrap())
        .and_then(|spec| spec.cruise_kmh).expect("T-l1 F-80 cruise speed");
    plane.set_property("YPos", format!("{LOSS_ALTITUDE_M:.3}"));
    plane.set_property("Fuel", "0");
    plane.set_property("AiRTBDecision", AiRtbDecision::Disabled.property());
    let heading = crate::placement::heading_toward(start, exit);
    plane.set_property("YOri", format!("{heading:.3}"));
    flight.set_name("T-l1 flight");
    flight.children = vec![plane, entity];
    p.add(flight);
    let activate = p.control("MCU_Activate", "T-l1 ACTIVATE", &[]);
    by_id_mut(&mut p.root, activate).set_objects(vec![entity_id]);
    wp.set_property("XPos", format!("{:.3}", exit.0));
    wp.set_property("ZPos", format!("{:.3}", exit.1));
    wp.set_property("YPos", format!("{LOSS_ALTITUDE_M:.3}"));
    wp.set_property("Speed", format!("{cruise:.0}"));
    wp.set_objects(vec![entity_id]);
    let wp = p.add(wp);
    let orders = p.relay("T-l1 CRUISE DELAY", 0.5);
    p.link(orders, wp);
    p.link(p.arm, activate);
    p.link(p.arm, orders);
    let losses = p.count("T-l1 LOSSES", 1, 0);
    attach_event(by_id_mut(&mut p.root, entity_id), EVENT_KILLED, losses);
    p.observe(losses, "T-l1 ev13 LOSSES");
    let close = id(&p.root, "T-l CLOSE");
    p.watch_aircraft_events("T-l1", plane_id, entity_id, close,
        &[EVENT_PLANE_CRASHED, EVENT_PILOT_KILLED, EVENT_PLANE_LANDED]);
    p.icon("T-l1 fuel zero", cell.centre_km, COALITION_NATO);
}

fn build(flight: Flight) -> Probe {
    if flight == Flight::Load {
        return build_load_probe();
    }
    let mut p = Probe::new(flight);
    for &cell in flight.sheet() {
        p.start_cell(cell);
        match (flight, cell.key) {
            (Flight::TwoB, _) => build_sortie_reflight(&mut p),
            (Flight::TwoC | Flight::TwoD | Flight::TwoE | Flight::TwoF, _) => build_empty_target_probe(&mut p, flight),
            (Flight::Two, "T-s1" | "T-s2") => build_sortie_probe(&mut p),
            (Flight::Two, "T-k" | "T-k1") => build_budget_probe(&mut p),
            (Flight::Two, "T-l") => build_no_damage_loss(&mut p),
            (_, "T-a1") => build_a1(&mut p),
            (_, "T-e") => build_e(&mut p),
            (Flight::OneP, "T-f") => build_fighter_count(&mut p),
            (Flight::Zero, "T-f") => build_f(&mut p),
            (_, "T-t") => build_tt(&mut p),
            (_, "T-j") => build_j(&mut p),
            (_, "T-z") => build_z(&mut p),
            (_, "T-c") => build_c(&mut p),
            (_, "T-n") => build_n(&mut p, NVariant::Tn),
            (_, "T-n2") => build_n(&mut p, NVariant::Tn2),
            (_, "Y-N") => build_y(&mut p, Side::Nato),
            (_, "Y-D") => build_y(&mut p, Side::Dprk),
            (_, "T-g2") => build_g(&mut p),
            (_, "T-d") => build_d(&mut p),
            (_, "T-d2") => build_d2(&mut p),
            (_, "T-d3") => build_d3(&mut p),
            (_, "T-d4") => build_d4(&mut p),
            (_, "T-r") => build_r(&mut p),
            (_, "T-v") => build_v(&mut p),
            (_, "T-w") => build_w(&mut p),
            (_, "T-u") => build_u(&mut p),
            (_, "T-x") => build_x(&mut p),
            (_, "T-y") => build_larger_rtb(&mut p),
            (_, "T-h") => build_upstream_rows(&mut p, P_FLIGHT, H_GROUPS),
            (_, "T-b") => build_bingo_aircraft(&mut p),
            _ => panic!("{}: unknown probe cell {}", flight.stem(), cell.key),
        }
    }
    p.cues(flight);
    p
}

pub(crate) fn generate_probe_0() -> Il2Entity {
    build(Flight::Zero).root
}
pub(crate) fn generate_probe_1a() -> Il2Entity {
    build(Flight::OneA).root
}
pub(crate) fn generate_probe_1n() -> Il2Entity {
    build(Flight::OneN).root
}
pub(crate) fn generate_probe_1d() -> Il2Entity {
    build(Flight::OneD).root
}
pub(crate) fn generate_probe_1g() -> Il2Entity {
    build(Flight::OneG).root
}
pub(crate) fn generate_probe_1h() -> Il2Entity {
    build(Flight::OneH).root
}
pub(crate) fn generate_probe_1j() -> Il2Entity {
    build(Flight::OneJ).root
}
pub(crate) fn generate_probe_1k() -> Il2Entity {
    build(Flight::OneK).root
}
pub(crate) fn generate_probe_1l() -> Il2Entity {
    build(Flight::OneL).root
}
pub(crate) fn generate_probe_1m() -> Il2Entity {
    build(Flight::OneM).root
}
pub(crate) fn generate_probe_1p() -> Il2Entity {
    build(Flight::OneP).root
}

pub(crate) fn generate_probe_2() -> Il2Entity {
    build(Flight::Two).root
}

pub(crate) fn generate_probe_2b() -> Il2Entity {
    build(Flight::TwoB).root
}

pub(crate) fn generate_probe_2c() -> Il2Entity {
    build(Flight::TwoC).root
}

pub(crate) fn generate_probe_2d() -> Il2Entity {
    build(Flight::TwoD).root
}

pub(crate) fn generate_probe_2e() -> Il2Entity {
    build(Flight::TwoE).root
}

pub(crate) fn generate_probe_2f() -> Il2Entity {
    build(Flight::TwoF).root
}

fn reflight_walk_input(root: &Il2Entity) -> super::testkit::WalkerInput {
    use super::shell::test_support::ticks;
    use super::testkit::{ScriptedTrack, WalkerInput};
    let mut input = WalkerInput { until: ticks(TWO_B_CLOSE_S), ..WalkerInput::default() };
    for (cell, row) in RUN_SHEET_2B.iter().zip(TWO_B_SORTIES) {
        let copy = named(root, &format!("{} sortie", cell.key));
        input.arrivals.push((ticks(cell.arm_s + row.measured_transit_s), id(copy, "WP 1").into()));
        if let Some(enemy) = &row.enemy {
            let target = world(cell.centre_km);
            input.tracks.push(ScriptedTrack {
                coalition: TWO_B_TRACK_COALITION,
                // Deletion is modeled by moving the last sample outside the
                // zone; the walker does not simulate aircraft existence.
                samples: (enemy.activate_s as u32..=enemy.delete_s as u32).map(|second| {
                    let point = if f64::from(second) == enemy.delete_s {
                        (target.0, target.1 + TWO_B_TRACK_OUTSIDE_OFFSET_M)
                    } else {
                        let offset = TWO_B_ORBIT_OFFSETS_KM[0];
                        (target.0 + offset.0 * 1000.0, target.1 + offset.1 * 1000.0)
                    };
                    (ticks(f64::from(second)), point.0, point.1)
                }).collect(),
            });
        }
    }
    input
}

fn empty_target_walk_input(root: &Il2Entity, flight: Flight) -> super::testkit::WalkerInput {
    use super::shell::test_support::ticks;
    let mut input = super::testkit::WalkerInput {
        until: flight.sheet().iter().map(|cell| ticks(cell.close_s)).max()
            .unwrap_or_else(|| panic!("{}: missing cells", flight.stem())),
        ..super::testkit::WalkerInput::default()
    };
    for (cell, row) in flight.sheet().iter().zip(flight.empty_target_sorties()) {
        let copy = named(root, &format!("{} sortie", cell.key));
        input.arrivals.push((ticks(cell.arm_s + row.predicted_transit_s()), id(copy, "WP 1").into()));
    }
    input
}

fn append_walk_schedule(report: &mut String, root: &Il2Entity, input: &super::testkit::WalkerInput) {
    use std::fmt::Write as _;
    use super::testkit::{DeactivateRunningTimer, TICKS_PER_SECOND, WalkerConfig, walk};
    let log = walk(root, WalkerConfig {
        deactivate_running_timer: DeactivateRunningTimer::Cancels, ..WalkerConfig::default()
    }, input);
    for pulse in log.pulses {
        writeln!(report, "| {:.2} | {} | {} | {} |", pulse.t as f64 / TICKS_PER_SECOND as f64, pulse.index,
            by_id(root, pulse.index).block_type, pulse.name).unwrap();
    }
}

fn empty_target_schedule(root: &Il2Entity) -> String {
    use std::fmt::Write as _;
    use super::shell::test_support::times;
    use super::testkit::{DeactivateRunningTimer, TICKS_PER_SECOND, WalkerConfig, walk};
    let input = empty_target_walk_input(root, Flight::TwoC);
    let log = walk(root, WalkerConfig {
        deactivate_running_timer: DeactivateRunningTimer::Cancels, ..WalkerConfig::default()
    }, &input);
    let mut report = format!("# Flight 2C run sheet\n\n\
Spectate if possible, camera on C0a, then LCtrl+R to record. Leave the server at 15:00 ({TWO_C_LEAVE_S} s mission time). The reminder is at {TWO_C_LEAVE_CUE_S} s. Confirm Leave Server promptly so the end-of-mission list captures live aircraft heights and rocket counts before the first predicted COLD at 946.5 s. Do not wait for the {TWO_C_CLOSE_S} s safety close.\n\n\
C0: HVAR as shipped. C1: HVAR with only PayloadId changed to 2 on all four Planes. C2: HVAR ground order for 120 s, then air patrol for 600 s (fix A-prime). C3: AirAlert as shipped. All twelve stand-in areas stay COLD. No enemy aircraft or HOT switches.\n\n\
WP 1 predictions use measured flight 2 transits from START: HVAR {TWO_B_HVAR_TRANSIT_S} s, AirAlert {TWO_B_AIRALERT_TRANSIT_S} s. Placement retains flight 2B's {TWO_TRANSIT_S} s distance. Times below are predictions with Restart, normal target order and timer cancellation. Aircraft dynamics, losses and the attack-order override are not simulated.\n\n\
Every target passes TerrainMap::builtin().is_land_xz using assets/combined_terrain.bin, the same check as flight 2 T-l. No grid point moved. Nearest other target is 45 km for every cell. Ground height is unavailable at every target: this checkout has no ground-height grid (decision 0003), and the terrain mask supplies land/water flags only. AttackArea YPos is not a ground-height measurement.\n\n\
| Cell | Variant | Prefix | START s | Predicted WP 1 s | ON STATION start s | ON STATION end s | Target X m | Target Z m | Ground height m | Nearest target km |\n\
|---|---|---|---:|---:|---:|---:|---:|---:|---|---:|\n");
    let terrain = crate::watermap::TerrainMap::builtin().expect("flight 2C terrain mask");
    for (cell, row) in RUN_SHEET_2C.iter().zip(TWO_C_SORTIES) {
        let copy = named(root, &format!("{} sortie", cell.key));
        let starts = times(&log, id(copy, &format!("{} ON STATION IN", row.prefix)));
        let ends = times(&log, id(copy, &format!("{} ON STATION", row.prefix)));
        assert_eq!(starts.len(), 1, "flight 2C {}: ON STATION start count", cell.key);
        assert_eq!(ends.len(), 1, "flight 2C {}: ON STATION end count", cell.key);
        let target = world(cell.centre_km);
        assert!(terrain.is_land_xz(target.0, target.1), "flight 2C {}: target over water", cell.key);
        let nearest = RUN_SHEET_2C.iter().filter(|other| other.key != cell.key).map(|other| {
            (cell.centre_km.0 - other.centre_km.0).hypot(cell.centre_km.1 - other.centre_km.1)
        }).fold(f64::INFINITY, f64::min);
        writeln!(report, "| {} | {:?} | {} | {} | {} | {:.1} | {:.1} | {:.0} | {:.0} | unavailable | {nearest:.1} |",
            cell.key, row.variant, row.prefix, cell.arm_s, cell.arm_s + row.variant.measured_transit_s(),
            starts[0] as f64 / TICKS_PER_SECOND as f64, ends[0] as f64 / TICKS_PER_SECOND as f64,
            target.0, target.1).unwrap();
    }
    report.push_str("\nC0/C1 ON STATION starts at WP 1 + 0.5 s. C2 starts at WP 1 + 120.5 s under Restart, or +120.0 s under Ignore/Twice, in both target orders. C3 starts at WP 1 + 1.0 s under Restart with normal target order, or +0.5 s with reversed targets. Ignore/Twice give C3 +0.5 s in both orders. With reversed targets C3's second 0.5 s timer expires before the first timer can restart it. Under Twice the second attack timer outputs twice, but the shell's ON STATION IN latch starts the hold once. Holds last 600 s for C0/C1/C2 and 1080 s for C3. At 900 s every copy has at least 393.5 s since its ON STATION start and no scheduled DONE HUB has fired. Losses of all four aircraft could end a copy earlier.\n\n");
    report.push_str("## C2 wiring in the plain Group\n\nSource before placement: timer 37 -> [38,41], new timer 41 -> [42,39], WP 1 (40) -> [37,41]. Mission Complete 1 is 39. inspect_sortie accepts S2, navigation attack timer 41, attack_time_s 600. Placement adds RTB (S7), then the unchanged shell removes Mission Complete 1 and adds ON STATION IN and Zone IN links. Subtitles and traced breadcrumbs add observation links.\n\n| Cell | Index | Type | Name | Time s | AttackAir/Ground/GTargets | Radius m | Priority | Targets | Objects |\n|---|---:|---|---|---:|---|---:|---:|---|---|\n");
    for row in TWO_C_SORTIES.iter().filter(|row| row.variant == EmptyTargetVariant::C2) {
        let copy = named(root, &format!("{} sortie", row.cell));
        for name in ["AttackArea 1", "AttackArea 2", "AttackArea", "AttackArea air", "WP 1"] {
            let node = named(copy, name);
            writeln!(report, "| {} | {} | {} | {name} | {} | {}/{}/{} | {} | {} | {:?} | {:?} |",
                row.cell, node.index.unwrap(), node.block_type, node.property("Time").unwrap_or("-"),
                node.property("AttackAir").unwrap_or("-"), node.property("AttackGround").unwrap_or("-"),
                node.property("AttackGTargets").unwrap_or("-"), node.property("AttackArea").unwrap_or("-"),
                node.property("Priority").unwrap_or("-"), node.targets, node.objects).unwrap();
        }
    }
    report.push_str("\n## Optional recording spawn\n\nUse only if a spectating host cannot record Tacview. 2C NATO: USA country 601, NATO coalition 2, F-80C f80c10, air start at 3050 m, km (130,-90), world (235934,179477), heading 0 degrees toward C0a at (245934,179477). The flown flight 2B run 3 header (src/testdata/missionlog/real_22, first file) maps country 601 to coalition 2. Both source templates and every retained Zone IN watch PlaneCoalitions [1], so a NATO player cannot trip it.\n\n");
    report.push_str("## Predicted pulse and command schedule\n\nRestart, normal target order, timer cancellation, scripted WP 1 arrivals, no enemy tracks. Timer entries are expiry/output times; ON STATION IN marks the start of the hold. Each cell closes at 1800 s with one Delete per written aircraft entity. The final cue is a safety net, not the planned snapshot time.\n\n| Time s | Index | Type | Name |\n|---:|---:|---|---|\n");
    append_walk_schedule(&mut report, root, &input);
    report
}

fn formation_schedule(root: &Il2Entity) -> String {
    use std::fmt::Write as _;
    use super::shell::test_support::times;
    use super::testkit::{DeactivateRunningTimer, TICKS_PER_SECOND, WalkerConfig, walk};
    let input = empty_target_walk_input(root, Flight::TwoD);
    let log = walk(root, WalkerConfig {
        deactivate_running_timer: DeactivateRunningTimer::Cancels, ..WalkerConfig::default()
    }, &input);
    let mut report = format!("# Flight 2D run sheet\n\n\
Spectate, then LCtrl+R to record Tacview. Note the first mission time the server overload message shows and the latest START cue (4, 8, 12, then 16 aircraft). These are scheduled counts; collisions can reduce the live count. Mark any overloaded run. Leave the server at 15:00 ({TWO_D_LEAVE_S} s mission time); the reminder is at {TWO_D_LEAVE_CUE_S} s. Confirm Leave Server promptly so the end-of-mission list retains every live aircraft. Do not wait for the {TWO_D_CLOSE_S} s safety close.\n\n\
One HVAR four-ship per cell, 16 NATO AI aircraft in all. D0 is the as-shipped control. Only FormationType and FormationDensity on the source Formation command vary. All four stand-in areas stay COLD, with no HOT switch or enemy aircraft. No pilot spawn rows: both .spawns.json files contain []. Spectator recording works (2026-10-15 loader/log facts).\n\n\
WP 1 predictions use flight 2B's measured HVAR transit constant, {TWO_B_HVAR_TRANSIT_S} s from START. Placement retains the {TWO_TRANSIT_S} s distance. The walker uses Restart, normal target order and timer cancellation. ON STATION starts at WP 1 + 0.5 s and lasts {TWO_D_ATTACK_TIME_S} s. At 900 s the last cell has held for 373.5 s; no scheduled DONE HUB has fired. Aircraft dynamics and collisions are not simulated; losing all four aircraft can end a sortie earlier.\n\n\
Targets are exactly flight 2C's C1b, C2c, C2b and C1c, whose measured settled heights were similar. The built-in terrain mask checks land only; this checkout has no ground-height grid (decision 0003). Every target is at least 45 km from the others.\n\n\
| Cell | Formation / density | Prefix | START s | Predicted WP 1 s | ON STATION start s | ON STATION end s | Target X m | Target Z m | Nearest target km |\n\
|---|---|---|---:|---:|---:|---:|---:|---:|---:|\n");
    let terrain = crate::watermap::TerrainMap::builtin().expect("flight 2D terrain mask");
    for (cell, row) in RUN_SHEET_2D.iter().zip(TWO_D_SORTIES) {
        let EmptyTargetVariant::Formation(formation) = row.variant else {
            panic!("flight 2D {}: missing formation settings", cell.key);
        };
        let copy = named(root, &format!("{} sortie", cell.key));
        let starts = times(&log, id(copy, &format!("{} ON STATION IN", row.prefix)));
        let ends = times(&log, id(copy, &format!("{} ON STATION", row.prefix)));
        assert_eq!(starts.len(), 1, "flight 2D {}: ON STATION start count", cell.key);
        assert_eq!(ends.len(), 1, "flight 2D {}: ON STATION end count", cell.key);
        let target = world(cell.centre_km);
        assert!(terrain.is_land_xz(target.0, target.1), "flight 2D {}: target over water", cell.key);
        let nearest = RUN_SHEET_2D.iter().filter(|other| other.key != cell.key).map(|other| {
            (cell.centre_km.0 - other.centre_km.0).hypot(cell.centre_km.1 - other.centre_km.1)
        }).fold(f64::INFINITY, f64::min);
        writeln!(report, "| {} | {} {} / {} {} | {} | {} | {} | {:.1} | {:.1} | {:.0} | {:.0} | {nearest:.1} |",
            cell.key, formation.formation_type, formation.formation_name, formation.formation_density,
            formation.editor_label, row.prefix, cell.arm_s, cell.arm_s + row.variant.measured_transit_s(),
            starts[0] as f64 / TICKS_PER_SECOND as f64, ends[0] as f64 / TICKS_PER_SECOND as f64,
            target.0, target.1).unwrap();
    }
    report.push_str("\n## Formation commands as written\n\nPlain and traced Groups keep these same indices and links. Source command 35 has Objects [7] and is targeted by Formation 1 (timer 34).\n\n| Cell | Index | FormationType | FormationDensity | Objects | Targeted by (index, name) |\n|---|---:|---:|---:|---|---|\n");
    append_probe_formations(&mut report, root, TWO_D_SORTIES);
    report.push_str("\nFly three runs with Tacview. Per cell and run compare losses, nearest-wingman distance (median and fifth percentile) in transit, descent and on station, and seconds under 20 m. Density labels follow the editor list order; the spacing in metres remains to be measured. If overload appears before DP starts, the run identifies a load limit but cannot isolate the formation effect; reduce cells for a subsequent probe.\n\n## Predicted pulse and command schedule\n\nRestart, normal target order, timer cancellation, scripted WP 1 arrivals, no enemy tracks. Timer rows show expiry/output; ON STATION IN marks the hold start. Each cell closes at 1800 s with one Delete per written aircraft entity and a final cue.\n\n| Time s | Index | Type | Name |\n|---:|---:|---|---|\n");
    append_walk_schedule(&mut report, root, &input);
    report
}

fn append_probe_formations(report: &mut String, root: &Il2Entity, rows: &[EmptyTargetSortie]) {
    use std::fmt::Write as _;
    for row in rows {
        let copy = named(root, &format!("{} sortie", row.cell));
        for index in block_ids(copy, "MCU_CMD_Formation") {
            let command = by_id(copy, index);
            let mut incoming = Vec::new();
            root.for_each(&mut |node| {
                if node.targets.contains(&index) {
                    incoming.push(format!("{} {}", node.index.unwrap(), node.name().unwrap_or("<unnamed>")));
                }
            });
            writeln!(report, "| {} | {index} | {} | {} | {:?} | {} |", row.cell,
                command.property("FormationType").unwrap(), command.property("FormationDensity").unwrap(),
                command.objects, incoming.join(", ")).unwrap();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SortieProbePath {
    spawn: (f64, f64),
    target: (f64, f64),
}

impl SortieProbePath {
    fn separation_m(self, other: Self) -> f64 {
        // Heading-zero segments: their closest X coordinates either overlap
        // or are the two facing ends. Z is constant along each whole segment.
        let gap = (self.spawn.0.max(other.spawn.0) - self.target.0.min(other.target.0)).max(0.0);
        gap.hypot(self.spawn.1 - other.spawn.1)
    }
}

fn sortie_probe_path(copy: &Il2Entity) -> SortieProbePath {
    let context = copy.name().unwrap_or("<unnamed sortie>");
    let wp = named(copy, "WP 1");
    let lead = *wp.objects.first().unwrap_or_else(|| panic!("{context}: WP 1 missing navigation lead"));
    let plane = by_id(copy, number(by_id(copy, lead), "MisObjID") as i32);
    assert_eq!(plane.block_type, "Plane", "{context}: navigation lead Plane");
    let path = SortieProbePath {
        spawn: plane.pos_xz().unwrap_or_else(|| panic!("{context}: lead Plane missing XPos/ZPos")),
        target: wp.pos_xz().unwrap_or_else(|| panic!("{context}: WP 1 missing XPos/ZPos")),
    };
    assert_eq!(path.spawn.1, path.target.1, "{context}: path must have heading zero");
    assert!(path.spawn.0 <= path.target.0, "{context}: path must run toward positive X");
    path
}

fn empty_target_pulse_s(
    log: &super::testkit::WalkerLog,
    copy: &Il2Entity,
    row: &EmptyTargetSortie,
    suffix: &str,
) -> f64 {
    let pulses = super::shell::test_support::times(log, id(copy, &format!("{} {suffix}", row.prefix)));
    assert_eq!(pulses.len(), 1, "{}: {suffix} pulse count", row.cell);
    pulses[0] as f64 / super::testkit::TICKS_PER_SECOND as f64
}

fn builtin_sortie_schedule(root: &Il2Entity) -> String {
    use std::fmt::Write as _;
    use super::testkit::{DeactivateRunningTimer, TimerRetrigger, WalkerConfig, walk};
    let input = empty_target_walk_input(root, Flight::TwoE);
    let log = walk(root, WalkerConfig {
        timer_retrigger: TimerRetrigger::Restart,
        deactivate_running_timer: DeactivateRunningTimer::Cancels, ..WalkerConfig::default()
    }, &input);
    let mut report = format!("# Flight 2E run sheet\n\n\
Spectate, then LCtrl+R to record Tacview. Note the first mission time the server overload message shows against the START cues: 4, 8, 16, then 20 aircraft. EC at 240 s takes the scheduled count from 16 to 20. Losses can reduce the live count. Mark any overloaded run; its losses alone are not formation evidence. Leave the server at 15:00 ({TWO_E_LEAVE_S} s mission time); the reminder is at {TWO_E_LEAVE_CUE_S} s. Confirm Leave Server promptly. Do not wait for the {TWO_E_CLOSE_S} s safety close.\n\n\
Four built-ins as shipped after decision 0019, parsed without probe edits, then placed and wrapped as in the app. Every Formation command flies at density 1 (Safe). EA and EY use Pairs (19); EI and EC use Wedge (20). All four stand-in areas stay COLD, with no HOT switch or enemy aircraft at the targets. No pilot spawn rows: both .spawns.json files contain [].\n\n\
WP 1 predictions are nominal, not measured: START + {TWO_TRANSIT_S} s, from the placement distance (WP 1 speed times 300 s). No F-80 measured transit is reused. Restart, normal target order and timer cancellation give ON STATION at WP 1 + 0.5 s. Aircraft dynamics and collisions are not simulated.\n\n\
Targets are exactly flight 2C C2c, C0c, C0b and C0a. The terrain mask checks land only; ground heights along the paths are unavailable (decision 0003). Distances below use the placed navigation lead Plane and WP 1, measuring the closest points of the complete heading-zero segments. Every pair is at least 45 km apart; every NATO path is at least 90 km from each DPRK path.\n\n\
| Cell | Built-in file | Side | Shape | Prefix | START s | Predicted WP 1 s | ON STATION start s | ON STATION end s | DONE HUB s | DELETE DELAY s | Target X m | Target Z m | Spawn X m | Spawn Z m | Closest path | Distance km |\n\
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|\n");
    let paths: Vec<_> = TWO_E_SORTIES.iter().map(|row| {
        (row.cell, sortie_probe_path(named(root, &format!("{} sortie", row.cell))))
    }).collect();
    let terrain = crate::watermap::TerrainMap::builtin().expect("flight 2E terrain mask");
    for (cell, row) in RUN_SHEET_2E.iter().zip(TWO_E_SORTIES) {
        let copy = named(root, &format!("{} sortie", cell.key));
        let source = empty_target_source(row);
        let shape = super::library::inspect_sortie(&source).unwrap_or_else(|errors| {
            panic!("flight 2E {}: {}", cell.key, errors.join("\n"))
        });
        let side = match shape.side { super::AirSide::Nato => "NATO", super::AirSide::Dprk => "DPRK" };
        let pulse_s = |suffix| empty_target_pulse_s(&log, copy, row, suffix);
        let path = sortie_probe_path(copy);
        assert!(terrain.is_land_xz(path.target.0, path.target.1), "flight 2E {}: target over water", cell.key);
        let (nearest_cell, nearest_m) = paths.iter().filter(|(key, _)| *key != cell.key)
            .map(|(key, other)| (*key, path.separation_m(*other)))
            .min_by(|a, b| a.1.total_cmp(&b.1)).expect("flight 2E: other paths");
        writeln!(report, "| {} | {} | {side} | {:?} | {} | {} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.0} | {:.0} | {:.3} | {:.0} | {nearest_cell} | {:.3} |",
            cell.key, row.sortie().file, shape.kind, row.prefix, cell.arm_s, cell.arm_s + row.predicted_transit_s(),
            pulse_s("ON STATION IN"), pulse_s("ON STATION"), pulse_s("DONE HUB"), pulse_s("DELETE DELAY"),
            path.target.0, path.target.1, path.spawn.0, path.spawn.1, nearest_m / 1000.0).unwrap();
    }
    report.push_str("\nAt 900 s EA, EI and EC have held for 539.5, 419.5 and 359.5 s; their DONE HUB has not fired. EY keeps its shipped 300 s attack: ON STATION 420.5 to 720.5 s, DONE HUB at 720.5 s, DELETE DELAY at 781.1 s. Its observation window is transit, descent and five minutes on station. Losing all aircraft can end a cell earlier.\n\nEA is S3: one attack lead and a waypoint-free Cover escort, so only one attack timer exists. EY and EI are S4: both leads share WP 1, and both attack timers are observed. Every cell also observes START, ON STATION, COLD, EXTEND, HARD STOP, LOSSES, Zone IN, Zone Out, DONE HUB and DELETE DELAY.\n\n## Formation commands as written\n\nPlain and traced Groups retain these indices, Objects and incoming links.\n\n| Cell | Index | FormationType | FormationDensity | Objects | Targeted by (index, name) |\n|---|---:|---:|---:|---|---|\n");
    append_probe_formations(&mut report, root, TWO_E_SORTIES);
    report.push_str("\nFly three runs with Tacview. Measure same-flight spacing and other-flight spacing separately, by aircraft type and Red/Blue flight names. Record every loss, attacker, height above ground and nearest aircraft 0.5 s earlier. Check ammunition on every live aircraft at the 15:00 snapshot. Record the first overload time against the count cues; a split by side remains a possible later probe.\n\n## Predicted pulse and command schedule\n\nRestart, normal target order, timer cancellation, scripted nominal WP 1 arrivals, no enemy tracks. Timer rows show expiry/output; ON STATION IN marks the hold start. Each cell closes at 1800 s with one Delete per aircraft entity: EA 4, EY 4, EI 8, EC 4, plus the final cue.\n\n| Time s | Index | Type | Name |\n|---:|---:|---|---|\n");
    append_walk_schedule(&mut report, root, &input);
    report
}

fn builtin_formation_schedule(root: &Il2Entity) -> String {
    use std::fmt::Write as _;
    use super::shell::test_support::ticks;
    use super::testkit::{DeactivateRunningTimer, TICKS_PER_SECOND, TimerRetrigger, WalkerConfig, walk};
    let input = empty_target_walk_input(root, Flight::TwoF);
    let log = walk(root, WalkerConfig {
        timer_retrigger: TimerRetrigger::Restart,
        deactivate_running_timer: DeactivateRunningTimer::Cancels, ..WalkerConfig::default()
    }, &input);
    let mut report = format!("# Flight 2F run sheet\n\n\
Spectate, then LCtrl+R to record Tacview. Check the menu now and then for the server overload message. Leave the server at 38:30 ({TWO_F_LEAVE_S} s mission time); the reminder is at {TWO_F_LEAVE_CUE_S} s. Confirm Leave Server promptly. Safety close is at {TWO_F_CLOSE_S} s.\n\n\
Six copies of 1950_DPRK_Yak_AirfieldRaid_2plus2.Group: S4, four DPRK aircraft each, 24 written in all. PS is the as-shipped Pairs/Safe control. The source passed to placement is untouched; only FormationType and FormationDensity on both placed Formation commands change. All six stand-in areas stay COLD, with no HOT switch or enemy aircraft. No pilot spawn rows: both .spawns.json files contain [].\n\n\
Predicted WP 1 uses flight 2E's measured Yak transit: START + {TWO_F_YAK_TRANSIT_S} s (EY arrived at 384.1 s for START 120 s, three runs within 0.1 s). Placement retains the nominal {TWO_TRANSIT_S} s distance at 500 km/h. The shipped 300 s attack is unchanged. Restart, normal target order and timer cancellation give the timings below; ON STATION IN marks the hold start. Both S4 leads' attack timers and their shared WP 1 are observed, as in flight 2E.\n\n\
A is flight 2C C0c / 2E EY; B is flight 2C C2a. Both targets are over land. Coordinates and path separation use each placed navigation lead Plane and WP 1. The terrain mask provides no ground heights. Each target is reused only after the previous cell's predicted DELETE DELAY.\n\n\
| Cell | FormationType / name | FormationDensity / name | Prefix | Target | START s | Predicted WP 1 s | ON STATION start s | ON STATION end s | DONE HUB s | DELETE DELAY s | Target X m | Target Z m | Spawn X m | Spawn Z m |\n\
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    let terrain = crate::watermap::TerrainMap::builtin().expect("flight 2F terrain mask");
    let mut windows = Vec::new();
    let mut boundaries = BTreeSet::from([ticks(0.0), ticks(TWO_F_CLOSE_S)]);
    for (cell, row) in RUN_SHEET_2F.iter().zip(TWO_F_SORTIES) {
        let EmptyTargetVariant::BuiltInFormation { formation, .. } = row.variant else {
            panic!("flight 2F {}: missing placed formation settings", cell.key);
        };
        let target = [TWO_F_TARGET_A, TWO_F_TARGET_B].into_iter()
            .find(|target| target.centre_km == cell.centre_km)
            .unwrap_or_else(|| panic!("flight 2F {}: missing target label", cell.key));
        let copy = named(root, &format!("{} sortie", cell.key));
        let path = sortie_probe_path(copy);
        assert!(terrain.is_land_xz(path.target.0, path.target.1), "flight 2F {}: target over water", cell.key);
        let pulse_s = |suffix| empty_target_pulse_s(&log, copy, row, suffix);
        writeln!(report, "| {} | {} {} | {} {} | {} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.0} | {:.0} | {:.3} | {:.0} |",
            cell.key, formation.formation_type, formation.formation_name, formation.formation_density,
            formation.editor_label, row.prefix, target.name, pulse_s("START"), cell.arm_s + row.predicted_transit_s(),
            pulse_s("ON STATION IN"), pulse_s("ON STATION"), pulse_s("DONE HUB"), pulse_s("DELETE DELAY"),
            path.target.0, path.target.1, path.spawn.0, path.spawn.1).unwrap();
        let start = ticks(pulse_s("START"));
        let delete = ticks(pulse_s("DELETE DELAY"));
        windows.push((cell.key, start, delete, copy.count_block_type("Plane")));
        boundaries.extend([start, delete]);
    }
    let a = sortie_probe_path(named(root, "PD sortie"));
    let b = sortie_probe_path(named(root, "PS sortie"));
    writeln!(report, "\nA/B path separation: {:.3} km, end to end on z = 0 relative to K14.\n\n## Predicted aircraft alive\n\nIntervals include START and exclude DELETE DELAY. Counts assume the measured transit repeats and no losses; aircraft dynamics and collisions are not simulated.\n\n| From s | Until s | Cells | Aircraft alive |\n|---:|---:|---|---:|", a.separation_m(b) / 1000.0).unwrap();
    let boundaries: Vec<_> = boundaries.into_iter().collect();
    let mut maximum = 0;
    for interval in boundaries.windows(2) {
        let live: Vec<_> = windows.iter().filter(|row| row.1 <= interval[0] && interval[0] < row.2).collect();
        let aircraft: usize = live.iter().map(|row| row.3).sum();
        maximum = maximum.max(aircraft);
        let cells = live.iter().map(|row| row.0).collect::<Vec<_>>().join(", ");
        writeln!(report, "| {:.1} | {:.1} | {} | {aircraft} |",
            interval[0] as f64 / TICKS_PER_SECOND as f64, interval[1] as f64 / TICKS_PER_SECOND as f64,
            if cells.is_empty() { "none" } else { &cells }).unwrap();
    }
    writeln!(report, "\nMost aircraft alive at once: {maximum}. Every predicted attack ends before {TWO_F_LEAVE_S} s.\n\n## Cues\n\n| Time s | Subtitle |\n|---:|---|").unwrap();
    for cue in CUES.iter().filter(|cue| cue.flight == Flight::TwoF) {
        writeln!(report, "| {} | {} |", cue.time_s, cue.text).unwrap();
    }
    report.push_str("\n## Formation commands as written\n\nPlain and traced Groups retain these indices, Objects and incoming links. Source commands 35 and 41 name flight leads 7 and 11.\n\n| Cell | Index | FormationType | FormationDensity | Objects | Targeted by (index, name) |\n|---|---:|---:|---:|---|---|\n");
    append_probe_formations(&mut report, root, TWO_F_SORTIES);
    report.push_str("\n## Predicted pulse and command schedule\n\nRestart, normal target order, timer cancellation, scripted WP 1 arrivals from 2E's measured transit, no enemy tracks. Timer rows show expiry/output. The unchanged shell leaves HARD STOP running after completion: PD, PS and PL emit it at 1860, 2190 and 2520 s, but the latched DONE HUB does not fire again. Each cell closes at 2700 s with one Delete per aircraft entity and a final cue.\n\n| Time s | Index | Type | Name |\n|---:|---:|---|---|\n");
    append_walk_schedule(&mut report, root, &input);
    report
}

fn write_empty_target_schedule(root: &Il2Entity, flight: Flight) {
    let report = match flight {
        Flight::TwoC => empty_target_schedule(root),
        Flight::TwoD => formation_schedule(root),
        Flight::TwoE => builtin_sortie_schedule(root),
        Flight::TwoF => builtin_formation_schedule(root),
        _ => panic!("{}: no empty-target schedule", flight.stem()),
    };
    let path = Path::new("target/p14").join(format!("{}_schedule.md", flight.stem()));
    std::fs::write(&path, report).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    use std::io::Write as _;
    writeln!(std::io::stdout().lock(), "WROTE {}", path.display()).unwrap();
}

fn write_reflight_schedule(root: &Il2Entity) {
    use std::fmt::Write as _;
    let mut report = format!("# Flight 2B run sheet\n\nStay at K14 until {TWO_B_CLOSE_S} s. No player or combat is needed.\n\n\
WP 1 predictions use flight 2's measured START-to-WP transits: HVAR {TWO_B_HVAR_TRANSIT_S} s, AirAlert {TWO_B_AIRALERT_TRANSIT_S} s. Placement still uses the same {TWO_TRANSIT_S} s distance as flight 2. These are predictions, not measured flight 2B arrivals.\n\n\
| Cell | START s | Predicted WP 1 s | On station s | Area | Enemy activate/delete s | Expected DONE source |\n\
|---|---:|---:|---:|---|---|---|\n");
    for (cell, row) in RUN_SHEET_2B.iter().zip(TWO_B_SORTIES) {
        let copy = named(root, &format!("{} sortie", cell.key));
        let station = number(named(copy, &format!("{} ON STATION", row.sortie.prefix)), "Time");
        let enemy = row.enemy.as_ref().map_or_else(|| "none".into(), |enemy| format!("{} / {}", enemy.activate_s, enemy.delete_s));
        let area = row.hot_at_s.map_or_else(|| "COLD".into(), |time| format!("HOT at {time} s"));
        let done = if row.enemy.is_some() { "Zone Out".into() }
            else if row.hot_at_s.is_some() { format!("EXTEND, once after {TWO_EXTENSION_S} s") }
            else { "COLD".into() };
        writeln!(report, "| {} | {} | {} | {station} | {area} | {enemy} | {done} |",
            cell.key, cell.arm_s, cell.arm_s + row.measured_transit_s).unwrap();
    }
    report.push_str("\nHVAR ON STATION starts at WP 1 + 0.5 s. AirAlert retains both attack timers: its ON STATION latch starts at WP 1 + 0.5 to 1.0 s across the three re-trigger modes (Restart below gives +1.0 s). Each Delete follows DONE by 60.6 s. Zone reaction time in the game is measured by the breadcrumbs.\n\n");
    report.push_str("## Positions (X, Z in metres)\n\n");
    for cell in RUN_SHEET_2B {
        let target = world(cell.centre_km);
        writeln!(report, "- {} target ({:.3}, {:.3}), {:.3} km from K14.", cell.key, target.0, target.1,
            (target.0 - K14_ORIGIN.0).hypot(target.1 - K14_ORIGIN.1) / 1000.0).unwrap();
        let cell_root = named(root, cell.key);
        if let Some(enemy) = cell_root.find_by_name(&format!("{} enemy", cell.key)) {
            let context = format!("flight 2B {} schedule", cell.key);
            let copy = named(cell_root, &format!("{} sortie", cell.key));
            let attack = *block_ids(copy, "MCU_CMD_AttackArea").first()
                .unwrap_or_else(|| panic!("{context}: sortie missing MCU_CMD_AttackArea"));
            let attack_point = by_id(copy, attack).pos_xz()
                .unwrap_or_else(|| panic!("{context}: AttackArea {attack} missing XPos/ZPos"));
            for plane in block_ids(enemy, "Plane") {
                let node = by_id(enemy, plane);
                let point = node.pos_xz()
                    .unwrap_or_else(|| panic!("{context}: enemy Plane {plane} missing XPos/ZPos"));
                writeln!(report, "  - Enemy Plane {plane}: ({:.3}, {:.3}), altitude {TWO_B_ENEMY_ALTITUDE_M} m, {:.3} km from target.", point.0, point.1,
                    (point.0 - target.0).hypot(point.1 - target.1) / 1000.0).unwrap();
            }
            let offsets = TWO_B_ORBIT_OFFSETS_KM;
            for (i, offset) in offsets.iter().enumerate() {
                let seat = i + 1;
                let wp = named(cell_root, &format!("{} enemy ORBIT {seat}", cell.key));
                let point = wp.pos_xz()
                    .unwrap_or_else(|| panic!("{context}: enemy ORBIT {seat} missing XPos/ZPos"));
                let previous = offsets[(i + offsets.len() - 1) % offsets.len()];
                let next = offsets[(i + 1) % offsets.len()];
                let incoming = (offset.1 - previous.1).atan2(offset.0 - previous.0);
                let outgoing = (next.1 - offset.1).atan2(next.0 - offset.0);
                let turn = (outgoing - incoming).to_degrees().rem_euclid(360.0);
                writeln!(report, "  - Orbit {seat}: ({:.3}, {:.3}), altitude {TWO_B_ENEMY_ALTITUDE_M} m, {:.3} km from attack point, {:.3} km from target, {:.1} degree turn.",
                    point.0, point.1,
                    (point.0 - attack_point.0).hypot(point.1 - attack_point.1) / 1000.0,
                    (point.0 - target.0).hypot(point.1 - target.1) / 1000.0, turn.min(360.0 - turn)).unwrap();
            }
        }
    }
    for (i, a) in RUN_SHEET_2B.iter().enumerate() {
        for b in &RUN_SHEET_2B[i + 1..] {
            writeln!(report, "- {} to {}: {:.3} km.", a.key, b.key,
                (a.centre_km.0 - b.centre_km.0).hypot(a.centre_km.1 - b.centre_km.1)).unwrap();
        }
    }
    let (inner_km, outer_km) = TWO_B_ORBIT_OFFSETS_KM.iter().map(|(x, z)| x.hypot(*z))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(inner, outer), radius| (inner.min(radius), outer.max(radius)));
    let orbit_loop = (2..=TWO_B_ORBIT_OFFSETS_KM.len()).chain([1, 2])
        .map(|seat| seat.to_string()).collect::<Vec<_>>().join(" -> ");
    writeln!(report, "\n## Enemy orders\n\nEach Yak-9P pair has country {COUNTRY_DPRK}, AiRTBDecision {}, air start at {TWO_B_ENEMY_ALTITUDE_M} m and Plane Engageable 0. Activation issues Behaviour after {TWO_B_BEHAVIOUR_DELAY_S} s and formation plus ORBIT 2 after {TWO_B_ORDERS_DELAY_S} s. The waypoint loop is {orbit_loop}, at the shipped 500 km/h, Priority {TWO_B_ORBIT_PRIORITY}. Waypoint arrivals depend on flight dynamics and have no absolute scheduled time. The route is {inner_km:.1} to {outer_km:.1} km from the attack point; actual tracking is a flight check.\n\nThe Behaviour block is copied from DPRK_Scramble.Group. Filter 8 is retained but its selection is undocumented and unverified; Engageable is set to 0 on the command and on the aircraft.\n",
        AiRtbDecision::Disabled.property()).unwrap();
    for cell in RUN_SHEET_2B {
        if let Some(node) = root.find_by_name(&format!("{} enemy NON TARGETABLE", cell.key)) {
            writeln!(report, "```text\n{}\n```", serialize_group(node)).unwrap();
        }
    }
    writeln!(report, "\n## Predicted pulse and command schedule\n\nWalker schedule in time order, including initialization, budget, subtitles and close commands. Uses Restart, normal target order and timer cancellation, with scripted sortie arrivals and enemy presence. Timer entries are expiry/output times; waypoints are listed both when ordered and when scripted arrivals pulse them. Aircraft dynamics and repeated enemy orbit arrivals are not modeled. Later COLD or HARD STOP inputs find a closed DONE HUB. All cells close at {TWO_B_CLOSE_S} s with one Delete per written aircraft entity and the final cue.\n\n| Time s | Index | Type | Name |\n|---:|---:|---|---|").unwrap();
    append_walk_schedule(&mut report, root, &reflight_walk_input(root));
    let path = Path::new("target/p14").join(format!("{}_schedule.md", Flight::TwoB.stem()));
    std::fs::write(&path, report).expect("write flight 2B run sheet and schedule");
    use std::io::Write as _;
    writeln!(std::io::stdout().lock(), "WROTE {}", path.display()).unwrap();
}

fn serialize_probe(root: &Il2Entity) -> String {
    let mut exported = root.clone();
    exported.for_each_mut(&mut |node| {
        if node.block_type == "Group" {
            // Cell centres are run-sheet metadata. Shipped Group wrappers have
            // no position keys; each child already has its absolute position.
            node.properties
                .retain(|(key, _)| !matches!(key.as_str(), "XPos" | "ZPos"));
        }
    });
    serialize_group(&exported)
}

fn write_probe(flight: Flight, carrier: TraceCarrier) {
    // Decisions 0018/0019: these builders test current code; their flown
    // artifacts must remain the evidence of the old shell/placement.
    assert!(!flight.is_frozen(), "decisions 0018/0019: {} is frozen; use a new flight name", flight.stem());
    let dir = Path::new("target/p14");
    std::fs::create_dir_all(dir).expect("create target/p14");
    let probe = build(flight);
    let plain = dir.join(format!("{}.Group", flight.stem()));
    let traced = dir.join(format!("{}_traced.Group", flight.stem()));
    let plain_root = probe.root.clone();
    let (root, map) = probe.traced(carrier);
    for (path, tree) in [(&plain, &plain_root), (&traced, &root)] {
        std::fs::write(path, serialize_probe(tree)).expect("write probe");
        let locale = p14_probe_locale(tree);
        let english =
            crate::locale::encode_locale_utf16le(&crate::locale::serialize_locale(&locale));
        std::fs::write(path.with_extension("eng"), english).expect("write probe English locale");
        std::fs::write(path.with_extension("spawns.json"), spawn_json(flight))
            .expect("write probe spawn list");
    }
    let sidecar = traced.with_extension("trace.json");
    let written_sidecar = if let Err(error) = trace::write_trace_sidecar(&sidecar, &map) {
        // libtest captures println/eprintln on passing tests. Write directly so
        // the required --ignored command reports sidecar errors without --nocapture.
        use std::io::Write;
        writeln!(
            std::io::stderr().lock(),
            "{}: {error}; Group and .eng files were still written",
            sidecar.display()
        )
        .expect("report trace sidecar error");
        None
    } else {
        Some(sidecar)
    };
    for path in [
        &plain,
        &traced,
        &plain.with_extension("eng"),
        &traced.with_extension("eng"),
    ] {
        assert!(path.is_file(), "missing {}", path.display());
    }
    // Make the ignored writer's output a reviewable index/time inventory even
    // without --nocapture. Hold stdout once so concurrent writers do not mix rows.
    use std::fmt::Write as _;
    let mut report = String::new();
    for cell in flight.sheet() {
        named(&root, cell.key).for_each(&mut |node| {
            if node.block_type.starts_with("MCU_") && node.block_type != "MCU_TR_Subtitle" {
                writeln!(
                    report,
                    "{} | {} | {} | {} | {} | Time={}",
                    flight.stem(),
                    cell.key,
                    node.name().unwrap_or(""),
                    node.index.unwrap(),
                    node.block_type,
                    node.property("Time").unwrap_or("-")
                )
                .unwrap();
            }
        });
    }
    for path in [
        plain.clone(),
        traced.clone(),
        plain.with_extension("eng"),
        traced.with_extension("eng"),
        plain.with_extension("spawns.json"),
        traced.with_extension("spawns.json"),
    ].into_iter().chain(written_sidecar) {
        writeln!(
            report,
            "WROTE {} ({} bytes)",
            path.display(),
            std::fs::metadata(&path).unwrap().len()
        )
        .unwrap();
    }
    std::io::Write::write_all(&mut std::io::stdout().lock(), report.as_bytes()).unwrap();
    if flight.writes_schedule() {
        match flight {
            Flight::TwoB => write_reflight_schedule(&plain_root),
            Flight::TwoC | Flight::TwoD | Flight::TwoE | Flight::TwoF => write_empty_target_schedule(&plain_root, flight),
            _ => unreachable!("{}: no schedule writer", flight.stem()),
        }
    }
    if flight.writes_load_summary() {
        let mut report = String::new();
        for side in ["DPRK", "NATO"] {
            let groups: Vec<_> = ["Logic", "Sorties"].into_iter()
                .map(|suffix| named(&plain_root, &format!("Air Tasking {side} {suffix}"))).collect();
            let mcus: usize = groups.iter().map(|group| super::shell::test_support::mcus(group)).sum();
            let zones: usize = groups.iter().map(|group| group.count_block_type("MCU_CheckZone")).sum();
            let sources = map.entries.iter().filter(|entry| entry.group_path.iter()
                .any(|part| part == &format!("Air Tasking {side} Sorties"))).count();
            writeln!(report, "LOAD {side}: {mcus} MCUs, {zones} check zones, {sources} side trace objectives").unwrap();
        }
        std::io::Write::write_all(&mut std::io::stdout().lock(), report.as_bytes()).unwrap();
    }
}

#[test]
#[ignore = "freezes flight 1L inputs once; never regenerate"]
fn freeze_flight1l_w_inputs() {
    use std::io::Write;

    // Do not re-run after generate_template changes: that would replace the
    // flown flight's frozen inputs with different bytes. Refuse existing files.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(W_INPUT_DIR);
    for row in W_GROUPS {
        let path = dir.join(format!("{}.Group", row.name));
        assert!(!path.exists(), "refusing to overwrite frozen flight 1L input {}", path.display());
    }
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("cannot create flight 1L input directory {}: {error}", dir.display()));
    for row in W_GROUPS {
        let name = row.name;
        let unit = builtin_plane_catalog().into_iter()
            .find(|unit| crate::model_spec::script_id(&unit.script) == "f80c10")
            .expect("flight 1L T-w: built-in catalog missing F-80C f80c10");
        let seats = (0..2).map(|_| {
            let mut seat = TemplateSeat::new(unit.clone());
            seat.role = FlightRole::Independent;
            seat.country = COUNTRY_USA;
            seat.altitude = W_ALTITUDE_M;
            // TemplateSeat::new supplies empty orders/events. Keep its other
            // aircraft defaults and TemplateOptions' placement defaults.
            seat
        }).collect();
        let options = TemplateOptions {
            name: name.into(),
            seats,
            bring_up: row.bring_up,
            allow_multiple_spawns: row.allow_multiple_spawns,
            ..TemplateOptions::default()
        };
        let generated = generate_template(&options)
            .unwrap_or_else(|error| panic!("flight 1L {name}: generate_template failed: {error}"));
        let path = dir.join(format!("{name}.Group"));
        let mut file = std::fs::File::create_new(&path)
            .unwrap_or_else(|error| panic!("cannot create frozen flight 1L input {} (never overwrite): {error}", path.display()));
        file.write_all(serialize_group(&generated).as_bytes())
            .unwrap_or_else(|error| panic!("cannot write frozen flight 1L input {}: {error}", path.display()));
    }
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_0() {
    write_probe(Flight::Zero, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1a() {
    write_probe(Flight::OneA, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1n() {
    write_probe(Flight::OneN, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1d() {
    write_probe(Flight::OneD, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1g() {
    write_probe(Flight::OneG, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1h() {
    write_probe(Flight::OneH, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1j() {
    write_probe(Flight::OneJ, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1k() {
    write_probe(Flight::OneK, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1l() {
    write_probe(Flight::OneL, TraceCarrier::default());
}

#[test]
#[ignore = "freezes flight 1M inputs once; never overwrite"]
fn freeze_flight1m_x_inputs() {
    use std::io::Write;

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(X_INPUT_DIR);
    for row in X_GROUPS {
        let path = dir.join(format!("{}.Group", row.name));
        assert!(!path.exists(), "flight 1M {}: refusing to overwrite {}", row.name, path.display());
    }
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("flight 1M: cannot create {}: {error}", dir.display()));
    for row in X_GROUPS {
        let path = dir.join(format!("{}.Group", row.name));
        let mut file = std::fs::File::create_new(&path)
            .unwrap_or_else(|error| panic!("flight 1M {}: cannot create {}: {error}", row.name, path.display()));
        file.write_all(serialize_group(&current_x_group(row)).as_bytes())
            .unwrap_or_else(|error| panic!("flight 1M {}: cannot write {}: {error}", row.name, path.display()));
    }
}

#[test]
fn flight1m_frozen_inputs_match_current_generator() {
    for row in X_GROUPS {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(X_INPUT_DIR).join(format!("{}.Group", row.name));
        let frozen = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("flight 1M {}: cannot read {}: {error}", row.name, path.display()));
        assert_eq!(serialize_group(&current_x_group(row)).as_bytes(), frozen,
            "flight 1M {}: current generate_template differs from frozen input", row.name);
    }
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1m() {
    write_probe(Flight::OneM, TraceCarrier::default());
}

#[test]
#[ignore = "freezes flight 1P inputs once; never overwrite"]
fn freeze_flight1p_inputs() {
    use std::io::Write;

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(P_INPUT_DIR);
    let files: Vec<_> = std::iter::once(P_PACK_FILE.to_string())
        .chain(B_AIRCRAFT.iter().map(|row| format!("{}.Group", row.name))).collect();
    // Preflight the whole set before generating or writing any input.
    for file in &files {
        let path = dir.join(file);
        assert!(!path.exists(), "flight {P_FLIGHT} {file}: refusing to overwrite {}", path.display());
    }
    let inputs: Vec<_> = std::iter::once(current_p_pack())
        .chain(B_AIRCRAFT.iter().map(current_b_aircraft)).collect();
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("flight {P_FLIGHT}: cannot create {}: {error}", dir.display()));
    for (file, generated) in files.iter().zip(inputs) {
        let path = dir.join(file);
        let mut output = std::fs::File::create_new(&path)
            .unwrap_or_else(|error| panic!("flight {P_FLIGHT} {file}: cannot create {}: {error}", path.display()));
        output.write_all(serialize_group(&generated).as_bytes())
            .unwrap_or_else(|error| panic!("flight {P_FLIGHT} {file}: cannot write {}: {error}", path.display()));
    }
}

#[test]
fn flight1p_frozen_inputs_match_current_generators() {
    let inputs = std::iter::once((P_PACK_FILE.to_string(), current_p_pack()))
        .chain(B_AIRCRAFT.iter().map(|row| (format!("{}.Group", row.name), current_b_aircraft(row))));
    for (file, generated) in inputs {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(P_INPUT_DIR).join(&file);
        let frozen = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("flight {P_FLIGHT} {file}: cannot read {}: {error}", path.display()));
        assert_eq!(serialize_group(&generated).as_bytes(), frozen,
            "flight {P_FLIGHT} {file}: current generator differs from frozen input");
    }
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1p() {
    write_probe(Flight::OneP, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 flight 2B, its run sheet, and load files"]
fn write_p14_probe_2b() {
    // Flight 0 and decision 0007: default Objective carrier, C0/S0.
    write_probe(Flight::TwoB, TraceCarrier::default());
    write_load_probe();
}

#[test]
#[ignore = "writes target/p14 flight 2C and its run sheet and schedule"]
fn write_p14_probe_2c() {
    write_probe(Flight::TwoC, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 flight 2D and its run sheet and schedule"]
fn write_p14_probe_2d() {
    write_probe(Flight::TwoD, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 flight 2E and its run sheet and schedule"]
fn write_p14_probe_2e() {
    write_probe(Flight::TwoE, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 flight 2F and its run sheet and schedule"]
fn write_p14_probe_2f() {
    write_probe(Flight::TwoF, TraceCarrier::default());
}

fn build_load_probe() -> Probe {
    use super::assemble::{assemble_side, emit_side_logic};
    use super::library::builtin_library;
    use super::{AirBudget, AirContext, AirPlan, AirSide, PoolEntry, SideLogic, TargetRef, TriggerMode};

    struct LoadEntry {
        file: &'static str,
        area: usize,
        mode: TriggerMode,
    }
    const ENTRIES: &[LoadEntry] = &[
        LoadEntry { file: "1950_US_F80_AirAlert_4ship.Group", area: 0, mode: TriggerMode::Clock },
        LoadEntry { file: "1950_US_F80_HVAR_Strike_4ship.Group", area: 1, mode: TriggerMode::Clock },
        LoadEntry { file: "1950_US_F51_CloseSupport_4ship.Group", area: 0, mode: TriggerMode::Zone },
        LoadEntry { file: "1950_US_F51_ArmedRecon_2plus2.Group", area: 1, mode: TriggerMode::Both },
        LoadEntry { file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", area: 0, mode: TriggerMode::Zone },
        LoadEntry { file: "1950_DPRK_Il10_KimpoAttack_2x4.Group", area: 1, mode: TriggerMode::Both },
        // Two DPRK entries repeated, with six runs each, reach 24 copies per side.
        LoadEntry { file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", area: 0, mode: TriggerMode::Zone },
        LoadEntry { file: "1950_DPRK_Il10_KimpoAttack_2x4.Group", area: 1, mode: TriggerMode::Both },
    ];
    const RUNS: u8 = 6;
    let nato = [(150_000.0, 250_000.0), (155_000.0, 280_000.0)];
    let dprk = [(180_000.0, 250_000.0), (185_000.0, 280_000.0)];
    let ctx = AirContext {
        nato_objectives: &nato,
        east_objectives: &dprk,
        ..super::assemble::tests::context()
    };
    let mut plan = AirPlan {
        library: builtin_library(), budget: AirBudget::default(), seed: 7, ..AirPlan::default()
    };
    for row in ENTRIES {
        let sortie = plan.library.iter().find(|sortie| sortie.id == format!("builtin:{}", row.file))
            .unwrap_or_else(|| panic!("load probe: missing {}", row.file));
        let target = match sortie.side { AirSide::Nato => nato[row.area], AirSide::Dprk => dprk[row.area] };
        let mut entry = PoolEntry::new(sortie.id.clone(), TargetRef::Objective { side: sortie.side, xz: target });
        entry.mode = row.mode;
        entry.runs = RUNS;
        plan.pool.push(entry);
    }
    let mut p = Probe::new(Flight::Load);
    for side in [AirSide::Dprk, AirSide::Nato] {
        let mut logic = SideLogic::default();
        let mut copies = assemble_side(side, &plan, &ctx, &mut logic, &mut p.next_id)
            .unwrap_or_else(|e| panic!("load probe {side:?}: {e}"));
        assert_eq!(logic.gates.len(), 24, "load probe {side:?}");
        copies.trees[0].children.extend(super::budget::build_side(
            side, LOAD_PROBE_BUDGET, &mut logic, &plan.strategies, &mut p.next_id,
        ));
        let init = emit_side_logic(side, &logic, &mut copies.trees, &mut p.next_id)
            .unwrap_or_else(|e| panic!("load probe {side:?}: {e}"));
        copies.trees[0].children.extend(init);
        p.root.children.extend(copies.trees);
        for entry in copies.entries {
            for copy in entry.copies {
                for index in [copy.shell.start, copy.shell.done_hub, copy.shell.delete_timer] {
                    p.sources.push(p.source(index, None, None));
                }
            }
        }
    }
    let cue = p.at("LOAD PROBE READY", 1.0, &[]);
    p.observe(cue, "P14 load probe: 24 copies per side, idle. Record load time and server tick delay");
    p
}

#[test]
#[ignore = "writes target/p14 load file with 24 copies per side"]
fn write_p14_load_probe() {
    write_load_probe();
}

fn write_load_probe() {
    // Both ignored entry points may run together under write_p14_.
    static WRITTEN: OnceLock<()> = OnceLock::new();
    WRITTEN.get_or_init(|| write_probe(Flight::Load, TraceCarrier::default()));
}

#[test]
fn flight2_frozen_files_parse_and_links_resolve() {
    assert_frozen_files_parse_and_links_resolve(Flight::Two);
}

#[test]
fn placement_frozen_files_parse_and_links_resolve() {
    for flight in [Flight::OneM, Flight::TwoB, Flight::TwoC, Flight::TwoD] {
        assert!(flight.is_frozen(), "{}: placement changed under 0019", flight.stem());
        assert_frozen_files_parse_and_links_resolve(flight);
    }
}

fn assert_frozen_files_parse_and_links_resolve(flight: Flight) {
    // Each frozen flight has a traced Group, English, spawn list and map.
    let stem = format!("{}_traced", flight.stem());
    let directory = format!("flight{}", flight.stem().strip_prefix("P14_Probe_").unwrap().to_lowercase());
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("handoff/p14-probes").join(directory).join(&stem);
    let root = parse_group_file(&std::fs::read_to_string(path.with_extension("Group")).unwrap()).unwrap();
    super::testkit::assert_links_resolve(&root);
    let mut indexes = Vec::new();
    root.collect_indexes(&mut indexes);
    let unique: BTreeSet<_> = indexes.iter().copied().collect();
    assert_eq!(unique.len(), indexes.len(), "{stem}: duplicate Index");
    let english = std::fs::read(path.with_extension("eng")).unwrap();
    let locale = crate::locale::parse_locale(&crate::locale::decode_locale_bytes(&english).unwrap());
    root.for_each(&mut |node| {
        for key in ["LinkTrId", "MisObjID", "TarId", "CmdId"] {
            if let Some(value) = node.property(key) {
                assert!(unique.contains(&value.parse::<i32>().unwrap()), "{stem}: unresolved {key}={value}");
            }
        }
        for key in ["LCName", "LCDesc", "LCText"] {
            if let Some(value) = node.property(key) {
                let value = value.parse::<i32>().unwrap();
                assert!(value == 0 || locale.get(value).is_some(), "{stem}: missing locale {value}");
            }
        }
    });
    assert_eq!(root.count_block_type("Plane"), build(flight).root.count_block_type("Plane"), "{stem}: aircraft count");
    if flight == Flight::Two {
        assert_eq!(root.count_block_type("Plane"), 9);
    }
    let map = trace::read_trace_sidecar(&path.with_extension("trace.json")).unwrap();
    assert!(!map.entries.is_empty());
    for entry in &map.entries {
        assert!(unique.contains(&entry.source_index), "{stem}: missing trace source {}", entry.source_index);
        assert!(unique.contains(&entry.breadcrumb_index), "{stem}: missing breadcrumb {}", entry.breadcrumb_index);
    }
    for edge in &map.edges {
        assert!(unique.contains(&edge.from) && unique.contains(&edge.to), "{stem}: unresolved trace edge");
    }
    assert_eq!(std::fs::read_to_string(path.with_extension("spawns.json")).unwrap(), spawn_json(flight), "{stem}: spawns");
}

#[test]
fn flown_probes_regenerate_byte_for_byte_except_flight2() {
    use std::collections::BTreeMap;
    let scratch = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/p14/step3c-byte-check");
    std::fs::create_dir_all(&scratch).unwrap();
    let mut compared = 0;
    for flight in Flight::ALL {
        // 0018 freezes flight 2's shell; 0019 freezes placement in 1M,
        // 2B, 2C and 2D. Load is unflown. All other flown files regenerate.
        if !flight.regenerates_flown_files() {
            continue;
        }
        let p = build(flight);
        let plain = p.root.clone();
        let (traced, map) = p.traced(TraceCarrier::default());
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("handoff/p14-probes").join(format!("flight{}",
            flight.stem().strip_prefix("P14_Probe_").unwrap().to_lowercase()));
        let mut generated = BTreeMap::new();
        for (suffix, root) in [("", plain), ("_traced", traced)] {
            let stem = format!("{}{suffix}", flight.stem());
            generated.insert(format!("{stem}.Group"), serialize_probe(&root).into_bytes());
            let english = crate::locale::encode_locale_utf16le(&crate::locale::serialize_locale(&p14_probe_locale(&root)));
            generated.insert(format!("{stem}.eng"), english);
            generated.insert(format!("{stem}.spawns.json"), spawn_json(flight).into_bytes());
        }
        let sidecar_name = format!("{}_traced.trace.json", flight.stem());
        let sidecar = scratch.join(&sidecar_name);
        trace::write_trace_sidecar(&sidecar, &map).unwrap();
        generated.insert(sidecar_name, std::fs::read(sidecar).unwrap());
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|extension| extension == "md") {
                continue;
            }
            let name = path.file_name().unwrap().to_str().unwrap();
            let expected = generated.get(name).unwrap_or_else(|| panic!("{}: no generated artifact", path.display()));
            assert_eq!(&std::fs::read(&path).unwrap(), expected, "{}", path.display());
            compared += 1;
        }
    }
    // 56 minus 1M, 2B, 2C and 2D (four files each); flight 2 was already
    // excluded by 0018. The remaining flights use unchanged or frozen inputs.
    // Flight 2E (flown 2026-10-16) adds its four files.
    assert_eq!(compared, 44);
}

#[test]
fn flight2c_cells_match_grid_and_variants() {
    use EmptyTargetVariant::{C0, C1, C2, C3};
    let root = generate_probe_2c();
    assert_eq!(root.name(), Some("P14_Probe_2C"));
    let expected = [
        ("C0a", C0, "SN01", 60.0, (140.0, -90.0), (245_934.0, 179_477.0)),
        ("C0b", C0, "SN02", 65.0, (185.0, 45.0), (290_934.0, 314_477.0)),
        ("C0c", C0, "SN03", 70.0, (230.0, 0.0), (335_934.0, 269_477.0)),
        ("C1a", C1, "SN04", 75.0, (140.0, -45.0), (245_934.0, 224_477.0)),
        ("C1b", C1, "SN05", 80.0, (185.0, -90.0), (290_934.0, 179_477.0)),
        ("C1c", C1, "SN06", 85.0, (230.0, 45.0), (335_934.0, 314_477.0)),
        ("C2a", C2, "SN07", 90.0, (140.0, 0.0), (245_934.0, 269_477.0)),
        ("C2b", C2, "SN08", 95.0, (185.0, -45.0), (290_934.0, 224_477.0)),
        ("C2c", C2, "SN09", 100.0, (230.0, -90.0), (335_934.0, 179_477.0)),
        ("C3a", C3, "SN10", 105.0, (140.0, 45.0), (245_934.0, 314_477.0)),
        ("C3b", C3, "SN11", 110.0, (185.0, 0.0), (290_934.0, 269_477.0)),
        ("C3c", C3, "SN12", 115.0, (230.0, -45.0), (335_934.0, 224_477.0)),
    ];
    assert_eq!(RUN_SHEET_2C.len(), 12);
    assert_eq!(TWO_C_SORTIES.len(), 12);
    let terrain = crate::watermap::TerrainMap::builtin().unwrap();
    for ((cell, row), &(key, variant, prefix, start, km, target)) in RUN_SHEET_2C.iter().zip(TWO_C_SORTIES).zip(&expected) {
        assert_eq!(*cell, Cell { key, centre_km: km, arm_s: start, close_s: 1800.0 });
        assert_eq!((row.cell, row.variant, row.prefix), (key, variant, prefix));
        assert_eq!(world(km), target, "{key}: target");
        assert!(terrain.is_land_xz(target.0, target.1), "{key}: target over water");
        let copy = named(&root, &format!("{key} sortie"));
        assert_eq!(copy.count_block_type("Plane"), 4, "{key}: one four-ship");
        assert_eq!(named(copy, "WP 1").pos_xz(), Some(target));
        for attack in block_ids(copy, "MCU_CMD_AttackArea") {
            assert_eq!(by_id(copy, attack).pos_xz(), Some(target), "{key}: attack point");
        }
        assert_at(&root, &format!("{key} ARM"), start, &[id(copy, &format!("{prefix} START"))]);
        let nearest = expected.iter().filter(|other| other.0 != key).map(|other| {
            let distance = (target.0 - other.5.0).hypot(target.1 - other.5.1);
            assert!(distance >= 40_000.0, "{key} to {}: {distance} m", other.0);
            distance
        }).fold(f64::INFINITY, f64::min);
        assert_eq!(nearest, 45_000.0, "{key}: nearest target");
    }
    for variant in [C0, C1, C2, C3] {
        let cells: Vec<_> = expected.iter().filter(|row| row.1 == variant).collect();
        assert_eq!(cells.iter().map(|row| row.4.0 as i32).collect::<BTreeSet<_>>(), BTreeSet::from([140, 185, 230]));
        assert_eq!(cells.iter().map(|row| row.4.1 as i32).collect::<BTreeSet<_>>().len(), 3);
    }
}

#[test]
fn flight2c_sources_match_payloads_orders_and_shapes() {
    use EmptyTargetVariant::{C0, C1, C2, C3};
    use super::library::{TestedShape::{S1, S2}, inspect_sortie};
    let root = generate_probe_2c();
    for (variant, file, payload, kind, station, navigation_timer, attacks) in [
        (C0, "1950_US_F80_HVAR_Strike_4ship.Group", "4", S1, 600.0, 37,
            vec![(0.0, 0.0, 1.0, 2000.0, 600.0)]),
        (C1, "1950_US_F80_HVAR_Strike_4ship.Group", "2", S1, 600.0, 37,
            vec![(0.0, 0.0, 1.0, 2000.0, 600.0)]),
        (C2, "1950_US_F80_HVAR_Strike_4ship.Group", "4", S2, 600.0, 41,
            vec![(0.0, 0.0, 1.0, 2000.0, 120.0), (1.0, 0.0, 0.0, 5000.0, 600.0)]),
        (C3, "1950_US_F80_AirAlert_4ship.Group", "2", S2, 1080.0, 39,
            vec![(1.0, 0.0, 0.0, 5000.0, 1080.0), (0.0, 0.0, 1.0, 1500.0, 120.0)]),
    ] {
        for row in TWO_C_SORTIES.iter().filter(|row| row.variant == variant) {
            assert_eq!(row.sortie().file, file, "{}: source", row.cell);
            let source = empty_target_source(row);
            let shape = inspect_sortie(&source).unwrap_or_else(|errors| panic!("{}: {errors:?}", row.cell));
            assert_eq!(shape.kind, kind, "{}: source shape", row.cell);
            assert_eq!(shape.navigation_lead, 7);
            assert_eq!(shape.navigation_attack_timer, navigation_timer);
            assert_eq!(shape.attack_timers[&7], navigation_timer);
            assert_eq!(shape.attack_time_s, station);
            let copy = named(&root, &format!("{} sortie", row.cell));
            assert_eq!(number(named(copy, &format!("{} ON STATION", row.prefix)), "Time"), station);
            for tree in [&source, copy] {
                let planes = block_ids(tree, "Plane");
                assert_eq!(planes.len(), 4, "{}: aircraft count", row.cell);
                let lead = number(by_id(tree, planes[0]), "LinkTrId") as i32;
                for plane in planes {
                    assert_eq!(by_id(tree, plane).property("PayloadId"), Some(payload), "{}: Plane {plane}", row.cell);
                }
                let commands = block_ids(tree, "MCU_CMD_AttackArea");
                assert_eq!(commands.len(), attacks.len(), "{}: attack count", row.cell);
                for (&index, &(air, ground, targets, radius, duration)) in commands.iter().zip(&attacks) {
                    let command = by_id(tree, index);
                    for (field, value) in [("AttackAir", air), ("AttackGround", ground), ("AttackGTargets", targets),
                        ("AttackArea", radius), ("Time", duration), ("Priority", 1.0)] {
                        assert_eq!(number(command, field), value, "{}: AttackArea {index} {field}", row.cell);
                    }
                    assert_eq!(command.objects, [lead]);
                    assert!(command.targets.is_empty());
                }
            }
            let mut restored = source.clone();
            match variant {
                C0 | C3 => assert_eq!(restored, sortie_probe_source(&row.sortie())),
                C1 => {
                    restored.for_each_mut(&mut |node| {
                        if node.block_type == "Plane" { node.set_property("PayloadId", "4"); }
                    });
                    assert_eq!(restored, sortie_probe_source(&row.sortie()), "{}: C1 changes only four payloads", row.cell);
                }
                C2 => {} // Its complete source delta is pinned by the wiring test.
                EmptyTargetVariant::Formation(_) | EmptyTargetVariant::BuiltIn { .. }
                    | EmptyTargetVariant::BuiltInFormation { .. } => unreachable!("flight 2C variants only"),
            }
        }
    }
}

#[test]
fn flight2c_c2_wiring_changes_only_the_two_orders() {
    for cell in ["C2a", "C2b", "C2c"] {
        let row = TWO_C_SORTIES.iter().find(|row| row.cell == cell).unwrap();
        let source = empty_target_source(row);
        let original = sortie_probe_source(&row.sortie());
        assert_eq!(id(&source, "AttackArea 1"), 37);
        assert_eq!(id(&source, "AttackArea"), 38);
        assert_eq!(id(&source, "Mission Complete 1"), 39);
        assert_eq!(id(&source, "WP 1"), 40);
        assert_eq!(id(&source, "AttackArea 2"), 41);
        assert_eq!(id(&source, "AttackArea air"), 42);
        assert_eq!(by_id(&source, 37).targets, [38, 41]);
        assert_eq!(by_id(&source, 41).targets, [42, 39]);
        assert_eq!(by_id(&source, 40).targets, [37, 41]);
        assert_eq!(by_id(&source, 39).targets, [30]);
        assert_eq!(number(by_id(&source, 37), "Time"), 0.5);
        assert_eq!(number(by_id(&source, 41), "Time"), 120.0);
        assert_eq!(number(by_id(&source, 38), "Time"), 120.0);
        assert_eq!(number(by_id(&source, 42), "Time"), 600.0);
        for index in [37, 41] {
            assert_eq!(by_id(&source, index).property("Random"), Some("100"));
            assert!(by_id(&source, index).objects.is_empty());
        }
        for index in [38, 42] { assert_eq!(by_id(&source, index).objects, [7]); }
        let mut restored_timer = by_id(&source, 41).clone();
        restored_timer.index = Some(37);
        restored_timer.set_property("Index", "37");
        restored_timer.set_name("AttackArea 1");
        restored_timer.set_property("Time", "0.50");
        restored_timer.set_targets(vec![38, 39]);
        assert_eq!(&restored_timer, by_id(&original, 37), "{cell}: otherwise as timer 37");
        let mut restored = source.clone();
        by_id_mut(&mut restored, 38).set_property("Time", "600");
        by_id_mut(&mut restored, 37).set_targets(vec![38, 39]);
        by_id_mut(&mut restored, 40).set_targets(vec![37]);
        remove_ids(&mut restored, &HashSet::from([41, 42]));
        assert_eq!(restored, original, "{cell}: no other source edits");
    }
}

#[test]
fn flight2c_shells_breadcrumbs_and_close_match_plan() {
    let root = generate_probe_2c();
    assert_eq!(root.count_block_type("Plane"), 48);
    assert_eq!(root.count_block_type("MCU_TR_Entity"), 48);
    assert_eq!(root.count_block_type("MCU_CheckZone"), 24);
    assert_eq!(root.count_block_type("MCU_Spawner"), 0);
    assert_eq!(root.count_block_type("MCU_CMD_Damage"), 0);
    let (traced, map) = build(Flight::TwoC).traced(TraceCarrier::default());
    super::testkit::assert_links_resolve(&traced);
    for (cell, prefix, attack_count) in [
        ("C0a", "SN01", 1), ("C0b", "SN02", 1), ("C0c", "SN03", 1),
        ("C1a", "SN04", 1), ("C1b", "SN05", 1), ("C1c", "SN06", 1),
        ("C2a", "SN07", 2), ("C2b", "SN08", 2), ("C2c", "SN09", 2),
        ("C3a", "SN10", 2), ("C3b", "SN11", 2), ("C3c", "SN12", 2),
    ] {
        let copy = named(&root, &format!("{cell} sortie"));
        let hot = id(copy, &format!("{prefix} HOT"));
        root.for_each(&mut |node| {
            if node.block_type == "MCU_Activate" {
                assert!(!node.targets.contains(&hot), "{cell}: no HOT switch");
            }
        });
        for plane in block_ids(copy, "Plane") {
            assert_eq!(by_id(copy, plane).property("Country"), Some("601"), "{cell}: NATO only");
            assert_eq!(by_id(copy, plane).property("AiRTBDecision"), Some("0"));
        }
        let attacks: BTreeSet<_> = block_ids(copy, "MCU_CMD_AttackArea").into_iter().collect();
        let timers: Vec<_> = block_ids(copy, "MCU_Timer").into_iter().filter(|&index| {
            by_id(copy, index).targets.iter().any(|target| attacks.contains(target))
        }).collect();
        assert_eq!(timers.len(), attack_count, "{cell}: every issuing timer");
        let mut sources = timers;
        for name in ["Zone IN", "Zone Out", "WP 1"] { sources.push(id(copy, name)); }
        for suffix in ["START", "ON STATION", "COLD", "EXTEND", "HARD STOP", "LOSSES", "DONE HUB", "DELETE DELAY"] {
            sources.push(id(copy, &format!("{prefix} {suffix}")));
        }
        for source in sources {
            assert_eq!(map.entries.iter().filter(|entry| entry.source_index == source).count(), 1,
                "{cell}: breadcrumb for {:?}", by_id(copy, source).name());
            assert!(named(&root, &format!("{cell} CLOSE OFF")).targets.contains(&source));
        }
        let cell_root = named(&root, cell);
        let close = named(cell_root, &format!("{cell} CLOSE"));
        assert_eq!(number(close, "Time"), 1800.0);
        let deletes: Vec<_> = close.targets.iter().map(|&index| by_id(&root, index))
            .filter(|node| node.block_type == "MCU_Delete").collect();
        assert_eq!(deletes.len(), 4);
        for entity in block_ids(cell_root, "MCU_TR_Entity") {
            assert_eq!(deletes.iter().filter(|node| node.objects == [entity]).count(), 1, "{cell}: close entity {entity}");
        }
    }
}

#[test]
fn flight2c_walks_station_windows_and_snapshot() {
    use super::shell::test_support::{active, configs, ticks, times};
    use super::testkit::{DeactivateRunningTimer, TimerRetrigger, walk};
    let root = generate_probe_2c();
    for mut config in configs() {
        config.deactivate_running_timer = DeactivateRunningTimer::Cancels;
        let log = walk(&root, config, &empty_target_walk_input(&root, Flight::TwoC));
        for (cell, prefix, start, wp, restart_station, reverse_restart_station, other_station, duration) in [
            ("C0a", "SN01", 60.0, 346.0, 346.5, 346.5, 346.5, 600.0),
            ("C0b", "SN02", 65.0, 351.0, 351.5, 351.5, 351.5, 600.0),
            ("C0c", "SN03", 70.0, 356.0, 356.5, 356.5, 356.5, 600.0),
            ("C1a", "SN04", 75.0, 361.0, 361.5, 361.5, 361.5, 600.0),
            ("C1b", "SN05", 80.0, 366.0, 366.5, 366.5, 366.5, 600.0),
            ("C1c", "SN06", 85.0, 371.0, 371.5, 371.5, 371.5, 600.0),
            ("C2a", "SN07", 90.0, 376.0, 496.5, 496.5, 496.0, 600.0),
            ("C2b", "SN08", 95.0, 381.0, 501.5, 501.5, 501.0, 600.0),
            ("C2c", "SN09", 100.0, 386.0, 506.5, 506.5, 506.0, 600.0),
            ("C3a", "SN10", 105.0, 384.0, 385.0, 384.5, 384.5, 1080.0),
            ("C3b", "SN11", 110.0, 389.0, 390.0, 389.5, 389.5, 1080.0),
            ("C3c", "SN12", 115.0, 394.0, 395.0, 394.5, 394.5, 1080.0),
        ] {
            let copy = named(&root, &format!("{cell} sortie"));
            let station = match (config.timer_retrigger, config.reverse_targets) {
                (TimerRetrigger::Restart, false) => restart_station,
                (TimerRetrigger::Restart, true) => reverse_restart_station,
                (TimerRetrigger::Ignore | TimerRetrigger::Twice, _) => other_station,
            };
            assert_eq!(times(&log, id(copy, &format!("{prefix} START"))), [ticks(start)], "{cell} {config:?}");
            assert_eq!(times(&log, id(copy, "WP 1")), [ticks(start + 2.0), ticks(wp)], "{cell}: order and arrival");
            assert_eq!(times(&log, id(copy, &format!("{prefix} ON STATION IN"))), [ticks(station)], "{cell} {config:?}");
            assert_eq!(times(&log, id(copy, &format!("{prefix} ON STATION"))), [ticks(station + duration)], "{cell} {config:?}");
            assert!(900.0 - wp >= 300.0 && 900.0 - station >= 300.0, "{cell}: snapshot observation window");
            let done = times(&log, id(copy, &format!("{prefix} DONE HUB")));
            assert!(done.iter().all(|&time| time > ticks(900.0)), "{cell}: DONE before snapshot");
            assert_eq!(done, [ticks(station + duration)], "{cell}: COLD ends once");
            assert_eq!(times(&log, id(copy, &format!("{prefix} COLD"))), done);
            assert!(active(&log, id(copy, &format!("{prefix} COLD")), ticks(900.0)));
            assert!(!active(&log, id(copy, &format!("{prefix} HOT")), ticks(900.0)));
            for name in ["Zone IN", "Zone Out"] { assert!(times(&log, id(copy, name)).is_empty(), "{cell}: {name}"); }
            assert!(times(&log, id(copy, &format!("{prefix} EXTEND"))).is_empty());
        }
        assert_eq!(times(&log, id(&root, "SN01 COLD")), [ticks(946.5)]);
        for index in block_ids(&root, "MCU_Delete") {
            assert!(times(&log, index).iter().all(|&time| time > ticks(900.0)), "Delete {index} before snapshot");
        }
        for cell in ["C0a", "C0b", "C0c", "C1a", "C1b", "C1c", "C2a", "C2b", "C2c", "C3a", "C3b", "C3c"] {
            for seat in 1..=4 {
                assert_eq!(times(&log, id(&root, &format!("{cell} aircraft {seat} CLOSE DELETE"))), [ticks(1800.0)]);
            }
        }
    }
}

#[test]
fn flight2c_spawn_is_nato_and_cannot_trip_zones() {
    use super::shell::test_support::{ticks, times};
    use super::testkit::{ScriptedTrack, WalkerConfig, walk};
    assert_eq!(Flight::TwoC.spawns(), &[Spawn {
        name: "2C NATO", country: 601, km: (130.0, -90.0), heading_deg: 0.0, altitude_m: 3050.0, script: "f80c10",
    }]);
    assert_eq!(spawn_json(Flight::TwoC), "[{\"name\":\"2C NATO\",\"country\":601,\"x\":235934,\"z\":179477,\"heading_deg\":0,\"altitude_m\":3050,\"script\":\"f80c10\"}]\n");
    assert_eq!(crate::placement::heading_toward((235_934.0, 179_477.0), (245_934.0, 179_477.0)), 0.0);
    // This helper chooses the opposing check-zone filter, not membership.
    assert_eq!(crate::aircraft::plane_coalitions_for_country(601), "[1]");
    let flown_log = include_str!("../testdata/missionlog/real_22/missionReport(2026-10-15_02-43-19)[0].txt");
    let countries = flown_log.split_whitespace().find_map(|field| field.strip_prefix("CNTRS:"))
        .expect("flight 2B run 3 country-to-coalition header");
    assert!(countries.split(',').any(|country| country == "601:2"));
    let root = generate_probe_2c();
    let mut input = empty_target_walk_input(&root, Flight::TwoC);
    input.until = ticks(900.0);
    // A NATO track at every target is a stronger filter check than a host at C0a.
    for cell in ["C0a", "C0b", "C0c", "C1a", "C1b", "C1c", "C2a", "C2b", "C2c", "C3a", "C3b", "C3c"] {
        let zone = named(named(&root, &format!("{cell} sortie")), "Zone IN");
        assert_eq!(zone.property("PlaneCoalitions"), Some("[1]"), "{cell}: opposing coalition only");
        assert_eq!(zone.property("Closer"), Some("1"));
        assert_eq!(number(zone, "Zone"), 35_000.0);
        let (x, z) = zone.pos_xz().unwrap();
        input.tracks.push(ScriptedTrack { coalition: 2, samples: vec![(ticks(0.0), x, z)] });
    }
    let log = walk(&root, WalkerConfig::default(), &input);
    for cell in ["C0a", "C0b", "C0c", "C1a", "C1b", "C1c", "C2a", "C2b", "C2c", "C3a", "C3b", "C3c"] {
        let copy = named(&root, &format!("{cell} sortie"));
        assert!(times(&log, id(copy, "Zone IN")).is_empty(), "{cell}: NATO must not trip Zone IN");
        assert!(times(&log, id(copy, "Zone Out")).is_empty(), "{cell}: Zone Out stays unarmed");
    }
}

#[test]
fn flight2c_run_sheet_and_cues_match_plan() {
    use super::shell::test_support::{ticks, times};
    use super::testkit::{WalkerConfig, walk};
    let root = generate_probe_2c();
    let cues: Vec<_> = CUES.iter().filter(|cue| cue.flight == Flight::TwoC).map(|cue| (cue.time_s, cue.text)).collect();
    assert_eq!(cues, [
        (0.0, "Flight 2C: spectate if you can. Camera on C0a, then LCtrl+R to record. Leave the server at 15:00"),
        (870.0, "Flight 2C: leave the server now (15:00)"),
        (1800.0, "Flight 2C closed. Every aircraft has a Delete. End the mission normally"),
    ]);
    let log = walk(&root, WalkerConfig::default(), &empty_target_walk_input(&root, Flight::TwoC));
    for (time, text) in cues { assert_eq!(times(&log, id(&root, text)), [ticks(time)]); }
    let report = empty_target_schedule(&root);
    for line in [
        "| C0a | C0 | SN01 | 60 | 346 | 346.5 | 946.5 | 245934 | 179477 | unavailable | 45.0 |",
        "| C2c | C2 | SN09 | 100 | 386 | 506.5 | 1106.5 | 335934 | 179477 | unavailable | 45.0 |",
        "| C3c | C3 | SN12 | 115 | 394 | 395.0 | 1475.0 | 335934 | 224477 | unavailable | 45.0 |",
        "No grid point moved.", "this checkout has no ground-height grid (decision 0003)",
        "WP 1 + 120.5 s under Restart, or +120.0 s under Ignore/Twice",
        "Leave the server at 15:00 (900 s mission time)", "PlaneCoalitions [1]",
    ] { assert!(report.contains(line), "flight 2C run sheet missing {line}"); }
}

#[test]
fn flight2d_cells_match_table_and_land_separation() {
    let root = generate_probe_2d();
    assert_eq!(root.name(), Some("P14_Probe_2D"));
    let expected = [
        ("D0", "SN01", 60.0, (185.0, -90.0), (290_934.0, 179_477.0), 20, 0, "Wedge", "Dense", 45_000.0),
        ("D1", "SN02", 120.0, (230.0, -90.0), (335_934.0, 179_477.0), 20, 1, "Wedge", "Safe", 45_000.0),
        ("D2", "SN03", 180.0, (185.0, -45.0), (290_934.0, 224_477.0), 20, 2, "Wedge", "Loose", 45_000.0),
        ("DP", "SN04", 240.0, (230.0, 45.0), (335_934.0, 314_477.0), 19, 0, "Pairs", "Dense", 45_000.0_f64.hypot(90_000.0)),
    ];
    assert_eq!(RUN_SHEET_2D.len(), 4);
    assert_eq!(TWO_D_SORTIES.len(), 4);
    let terrain = crate::watermap::TerrainMap::builtin().unwrap();
    for ((cell, row), &(key, prefix, start, km, target, formation_type, formation_density, formation_name, editor_label, nearest))
        in RUN_SHEET_2D.iter().zip(TWO_D_SORTIES).zip(&expected)
    {
        assert_eq!(*cell, Cell { key, centre_km: km, arm_s: start, close_s: 1800.0 });
        assert_eq!((row.cell, row.prefix), (key, prefix));
        assert_eq!(row.variant, EmptyTargetVariant::Formation(ProbeFormation {
            formation_type, formation_density, formation_name, editor_label,
        }));
        assert_eq!(world(km), target, "{key}: target");
        assert!(terrain.is_land_xz(target.0, target.1), "{key}: target over water");
        let copy = named(&root, &format!("{key} sortie"));
        assert_eq!(copy.count_block_type("Plane"), 4, "{key}: one four-ship");
        assert_eq!(named(copy, "WP 1").pos_xz(), Some(target), "{key}: waypoint");
        assert_eq!(named(copy, "AttackArea").pos_xz(), Some(target), "{key}: attack point");
        assert_at(&root, &format!("{key} ARM"), start, &[id(copy, &format!("{prefix} START"))]);
        let actual_nearest = expected.iter().filter(|other| other.0 != key).map(|other| {
            let distance = (target.0 - other.4.0).hypot(target.1 - other.4.1);
            assert!(distance >= 45_000.0, "{key} to {}: {distance} m", other.0);
            distance
        }).fold(f64::INFINITY, f64::min);
        assert_eq!(actual_nearest, nearest, "{key}: nearest target");
    }
}

#[test]
fn flight2d_sources_and_placed_copies_change_only_formation() {
    use super::library::{TestedShape::S1, inspect_sortie};
    // Canonicalize the whole cell's indices first, so even observation links
    // leaving its sortie are remapped. In the sortie normalize only the cell
    // name/prefix and X/Z positions (placement and the shell move these).
    // Keep Y, orientation, aircraft offsets in the source comparison, every
    // other property, every link, child order, and the complete shell wiring.
    fn normalized_copy(root: &Il2Entity, cell: &str, prefix: &str) -> Il2Entity {
        let canonical = crate::duplicate::duplicate_template(named(root, cell), &mut 1).0;
        let mut copy = named(&canonical, &format!("{cell} sortie")).clone();
        copy.for_each_mut(&mut |node| {
            if let Some(name) = node.name() {
                node.set_name(&name.replace(cell, "D0").replace(prefix, "SN01"));
            }
            node.properties.retain(|(key, _)| !matches!(key.as_str(), "XPos" | "ZPos"));
        });
        copy
    }
    let root = generate_probe_2d();
    let control = normalized_copy(&root, "D0", "SN01");
    let original = sortie_probe_source(&TWO_D_SORTIES[0].sortie());
    assert_eq!(block_ids(&original, "MCU_CMD_Formation"), [35]);
    assert_eq!(by_id(&original, 35).objects, [7]);
    assert_eq!(by_id(&original, 35).property("FormationType"), Some("20"));
    assert_eq!(by_id(&original, 35).property("FormationDensity"), Some("1"));
    for (cell, prefix, formation_type, formation_density) in [
        ("D0", "SN01", "20", "0"), ("D1", "SN02", "20", "1"),
        ("D2", "SN03", "20", "2"), ("DP", "SN04", "19", "0"),
    ] {
        let row = TWO_D_SORTIES.iter().find(|row| row.cell == cell).unwrap();
        assert_eq!(row.sortie().file, "1950_US_F80_HVAR_Strike_4ship.Group");
        let mut source = empty_target_source(row);
        let shape = inspect_sortie(&source).unwrap_or_else(|errors| panic!("{cell}: {}", errors.join("\n")));
        assert_eq!(shape.kind, S1, "{cell}: source shape");
        assert_eq!(shape.attack_time_s, 600.0, "{cell}: attack duration");
        assert_eq!(shape.navigation_lead, 7, "{cell}: source navigation lead");
        assert_eq!(shape.navigation_attack_timer, 37, "{cell}: attack timer");
        assert_eq!(block_ids(&source, "MCU_CMD_Formation"), [35]);
        let copy = named(&root, &format!("{cell} sortie"));
        assert_eq!(number(named(copy, &format!("{prefix} ON STATION")), "Time"), 600.0);
        // The experiment's source variants remain explicit; current placement
        // enforces Safe on all of them. The flown variants are checked frozen.
        for (tree, density) in [(&source, formation_density), (copy, "1")] {
            assert_eq!(tree.count_block_type("MCU_CMD_Formation"), 1, "{cell}: Formation count");
            let formation = named(tree, "Formation");
            assert_eq!(formation.property("FormationType"), Some(formation_type), "{cell}");
            assert_eq!(formation.property("FormationDensity"), Some(density), "{cell}");
            assert_eq!(formation.objects, named(tree, "WP 1").objects, "{cell}: navigation lead");
            let mut incoming = Vec::new();
            tree.for_each(&mut |node| {
                if node.targets.contains(&formation.index.unwrap()) { incoming.push(node.index.unwrap()); }
            });
            assert_eq!(incoming, [id(tree, "Formation 1")], "{cell}: issuing timer");
        }
        by_id_mut(&mut source, 35).set_property("FormationType", "20");
        by_id_mut(&mut source, 35).set_property("FormationDensity", "1");
        assert_eq!(source, original, "{cell}: only two source properties may change");
        let mut normalized = normalized_copy(&root, cell, prefix);
        let formation = id(&normalized, "Formation");
        by_id_mut(&mut normalized, formation).set_property("FormationType", "20");
        by_id_mut(&mut normalized, formation).set_property("FormationDensity", "1");
        assert_eq!(normalized, control, "{cell}: only two placed properties may change");
    }
}

#[test]
fn flight2d_rejects_missing_duplicate_or_wrong_lead_formation() {
    let original = parse_group_file(HVAR).unwrap();
    let mut missing = original.clone();
    remove_ids(&mut missing, &HashSet::from([35]));
    let mut duplicate = original.clone();
    let mut extra = by_id(&duplicate, 35).clone();
    extra.index = Some(41);
    extra.set_property("Index", "41");
    duplicate.children.push(extra);
    let mut wrong_lead = original.clone();
    by_id_mut(&mut wrong_lead, 35).set_objects(vec![9]);
    for cell in ["D0", "D1", "D2", "DP"] {
        for (part, source) in [("missing", &missing), ("duplicate", &duplicate), ("wrong lead", &wrong_lead)] {
            let mut copy = source.clone();
            let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                set_probe_formation(&mut copy, cell, ProbeFormation {
                    formation_type: 20, formation_density: 2, formation_name: "Wedge", editor_label: "Loose",
                });
            })).expect_err("malformed Formation must fail");
            let message = error.downcast_ref::<String>().map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied()).expect("text panic");
            assert!(message.contains(cell), "{part}: failure must name {cell}: {message}");
            assert_eq!(&copy, source, "{cell} {part}: refuse before mutation");
        }
    }
}

#[test]
fn flight2d_shells_breadcrumbs_close_and_no_spawn() {
    use super::shell::test_support::{active, ticks};
    use super::testkit::{WalkerConfig, WalkerInput, walk};
    let root = generate_probe_2d();
    assert_eq!(root.count_block_type("Plane"), 16);
    assert_eq!(root.count_block_type("MCU_TR_Entity"), 16);
    assert_eq!(root.count_block_type("MCU_CheckZone"), 8);
    assert_eq!(root.count_block_type("MCU_Spawner"), 0);
    assert_eq!(root.count_block_type("Airfield"), 0);
    assert_eq!(root.count_block_type("MCU_CMD_Damage"), 0);
    assert!(Flight::TwoD.spawns().is_empty());
    assert_eq!(spawn_json(Flight::TwoD), "[]\n");
    let (traced, map) = build(Flight::TwoD).traced(TraceCarrier::default());
    super::testkit::assert_links_resolve(&traced);
    let initialized = walk(&root, WalkerConfig::default(), &WalkerInput {
        until: ticks(1.0), ..WalkerInput::default()
    });
    for (cell, prefix) in [("D0", "SN01"), ("D1", "SN02"), ("D2", "SN03"), ("DP", "SN04")] {
        let copy = named(&root, &format!("{cell} sortie"));
        let hot = id(copy, &format!("{prefix} HOT"));
        root.for_each(&mut |node| {
            if node.block_type == "MCU_Activate" {
                assert!(!node.targets.contains(&hot), "{cell}: no HOT switch");
            }
        });
        assert!(active(&initialized, id(copy, &format!("{prefix} COLD")), ticks(1.0)), "{cell}: starts COLD");
        assert!(!active(&initialized, hot, ticks(1.0)), "{cell}: starts without HOT");
        for plane in block_ids(copy, "Plane") {
            assert_eq!(by_id(copy, plane).property("Country"), Some("601"), "{cell}: NATO only");
            assert_eq!(by_id(copy, plane).property("AiRTBDecision"), Some("0"));
            assert_eq!(by_id(copy, plane).property("PayloadId"), Some("4"));
        }
        assert_eq!(named(copy, "Zone IN").property("PlaneCoalitions"), Some("[1]"));
        let mut sources = vec![id(copy, "WP 1"), id(copy, "AttackArea 1"), id(copy, "Zone IN"), id(copy, "Zone Out")];
        for suffix in ["START", "ON STATION", "COLD", "EXTEND", "HARD STOP", "LOSSES", "DONE HUB", "DELETE DELAY"] {
            sources.push(id(copy, &format!("{prefix} {suffix}")));
        }
        for source in sources {
            assert_eq!(map.entries.iter().filter(|entry| entry.source_index == source).count(), 1,
                "{cell}: breadcrumb for {:?}", by_id(copy, source).name());
            assert!(named(&root, &format!("{cell} CLOSE OFF")).targets.contains(&source));
        }
        let cell_root = named(&root, cell);
        let close = named(cell_root, &format!("{cell} CLOSE"));
        assert_eq!(number(close, "Time"), 1800.0);
        let deletes: Vec<_> = close.targets.iter().map(|&index| by_id(&root, index))
            .filter(|node| node.block_type == "MCU_Delete").collect();
        assert_eq!(deletes.len(), 4, "{cell}: safety Deletes");
        for entity in block_ids(cell_root, "MCU_TR_Entity") {
            assert_eq!(deletes.iter().filter(|node| node.objects == [entity]).count(), 1, "{cell}: Delete entity {entity}");
        }
    }
}

#[test]
fn flight2d_walks_station_windows_and_snapshot() {
    use super::shell::test_support::{active, ticks, times};
    use super::testkit::{DeactivateRunningTimer, TimerRetrigger, WalkerConfig, walk};
    let root = generate_probe_2d();
    let input = empty_target_walk_input(&root, Flight::TwoD);
    assert_eq!(input.until, ticks(1800.0));
    assert_eq!(input.arrivals, [
        (ticks(346.0), id(named(&root, "D0 sortie"), "WP 1").into()),
        (ticks(406.0), id(named(&root, "D1 sortie"), "WP 1").into()),
        (ticks(466.0), id(named(&root, "D2 sortie"), "WP 1").into()),
        (ticks(526.0), id(named(&root, "DP sortie"), "WP 1").into()),
    ]);
    assert!(input.tracks.is_empty());
    assert!(input.kills.is_empty());
    let log = walk(&root, WalkerConfig {
        timer_retrigger: TimerRetrigger::Restart,
        deactivate_running_timer: DeactivateRunningTimer::Cancels, ..WalkerConfig::default()
    }, &input);
    for (cell, prefix, start, wp, station, end) in [
        ("D0", "SN01", 60.0, 346.0, 346.5, 946.5),
        ("D1", "SN02", 120.0, 406.0, 406.5, 1006.5),
        ("D2", "SN03", 180.0, 466.0, 466.5, 1066.5),
        ("DP", "SN04", 240.0, 526.0, 526.5, 1126.5),
    ] {
        let copy = named(&root, &format!("{cell} sortie"));
        assert_eq!(times(&log, id(copy, &format!("{prefix} START"))), [ticks(start)], "{cell}");
        assert_eq!(times(&log, id(copy, "WP 1")), [ticks(start + 2.0), ticks(wp)], "{cell}: order and arrival");
        assert_eq!(times(&log, id(copy, "AttackArea 1")), [ticks(station)], "{cell}: attack order");
        assert_eq!(times(&log, id(copy, &format!("{prefix} ON STATION IN"))), [ticks(station)], "{cell}: hold starts once");
        assert_eq!(times(&log, id(copy, &format!("{prefix} ON STATION"))), [ticks(end)], "{cell}: hold ends once");
        assert_eq!(end - station, 600.0, "{cell}: hold duration");
        assert!(900.0 - wp >= 300.0 && 900.0 - station >= 300.0, "{cell}: snapshot window");
        let done = times(&log, id(copy, &format!("{prefix} DONE HUB")));
        assert_eq!(done, [ticks(end)], "{cell}: COLD ends once");
        assert!(done.iter().all(|&time| time > ticks(900.0)), "{cell}: DONE before snapshot");
        assert_eq!(times(&log, id(copy, &format!("{prefix} COLD"))), done);
        assert!(active(&log, id(copy, &format!("{prefix} COLD")), ticks(900.0)));
        assert!(!active(&log, id(copy, &format!("{prefix} HOT")), ticks(900.0)));
        for name in ["Zone IN", "Zone Out"] { assert!(times(&log, id(copy, name)).is_empty(), "{cell}: {name}"); }
        assert!(times(&log, id(copy, &format!("{prefix} EXTEND"))).is_empty());
        for seat in 1..=4 {
            assert_eq!(times(&log, id(&root, &format!("{cell} aircraft {seat} CLOSE DELETE"))), [ticks(1800.0)]);
        }
    }
    for index in block_ids(&root, "MCU_Delete") {
        assert!(times(&log, index).iter().all(|&time| time > ticks(900.0)), "Delete {index} before snapshot");
    }
}

#[test]
fn flight2d_run_sheet_and_cues_match_plan() {
    use super::shell::test_support::{ticks, times};
    use super::testkit::{WalkerConfig, walk};
    let root = generate_probe_2d();
    let cues: Vec<_> = CUES.iter().filter(|cue| cue.flight == Flight::TwoD).map(|cue| (cue.time_s, cue.text)).collect();
    assert_eq!(cues, [
        (0.0, "Flight 2D: spectate, then LCtrl+R to record. Note the first time the server overload message shows. Leave the server at 15:00"),
        (60.0, "Flight 2D: D0 starts (4 aircraft flying)"),
        (120.0, "Flight 2D: D1 starts (8 aircraft flying)"),
        (180.0, "Flight 2D: D2 starts (12 aircraft flying)"),
        (240.0, "Flight 2D: DP starts (16 aircraft flying)"),
        (870.0, "Flight 2D: leave the server now (15:00)"),
        (1800.0, "Flight 2D closed. Every aircraft has a Delete. End the mission normally"),
    ]);
    let log = walk(&root, WalkerConfig::default(), &empty_target_walk_input(&root, Flight::TwoD));
    for (time, text) in cues { assert_eq!(times(&log, id(&root, text)), [ticks(time)]); }
    let report = formation_schedule(&root);
    for line in [
        "| D0 | 20 Wedge / 0 Dense | SN01 | 60 | 346 | 346.5 | 946.5 | 290934 | 179477 | 45.0 |",
        "| D1 | 20 Wedge / 1 Safe | SN02 | 120 | 406 | 406.5 | 1006.5 | 335934 | 179477 | 45.0 |",
        "| D2 | 20 Wedge / 2 Loose | SN03 | 180 | 466 | 466.5 | 1066.5 | 290934 | 224477 | 45.0 |",
        "| DP | 19 Pairs / 0 Dense | SN04 | 240 | 526 | 526.5 | 1126.5 | 335934 | 314477 | 100.6 |",
        "Leave the server at 15:00 (900 s mission time)", "both .spawns.json files contain []",
        "D0 is the as-shipped control", "no HOT switch or enemy aircraft", "Formation commands as written",
        "373.5 s", "Predicted pulse and command schedule",
    ] { assert!(report.contains(line), "flight 2D run sheet missing {line}"); }
}

#[test]
fn flight2e_cells_match_table_and_land_path_separation() {
    use super::AirSide::{Dprk, Nato};
    use super::library::{TestedShape::{S1, S3, S4}, inspect_sortie};
    let root = generate_probe_2e();
    assert_eq!(root.name(), Some("P14_Probe_2E"));
    assert!(!Flight::TwoE.is_frozen());
    assert!(Flight::TwoE.regenerates_flown_files());
    assert!(Flight::TwoE.writes_schedule());
    assert_eq!(RUN_SHEET_2E.len(), 4);
    assert_eq!(TWO_E_SORTIES.len(), 4);
    let terrain = crate::watermap::TerrainMap::builtin().unwrap();
    let expected = [
        ("EA", "SN01", "1950_US_F51_ArmedRecon_2plus2.Group", Nato, S3, 1200, 4, 60.0, (230.0, -90.0), "C2c", (335_934.0, 179_477.0), (292_600.667, 179_477.0)),
        ("EY", "SN02", "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", Dprk, S4, 300, 4, 120.0, (230.0, 0.0), "C0c", (335_934.0, 269_477.0), (294_267.333, 269_477.0)),
        ("EI", "SN03", "1950_DPRK_Il10_KimpoAttack_2x4.Group", Dprk, S4, 600, 8, 180.0, (185.0, 45.0), "C0b", (290_934.0, 314_477.0), (258_434.0, 314_477.0)),
        ("EC", "SN04", "1950_US_F51_CloseSupport_4ship.Group", Nato, S1, 900, 4, 240.0, (140.0, -90.0), "C0a", (245_934.0, 179_477.0), (202_600.667, 179_477.0)),
    ];
    let mut paths = Vec::new();
    for ((cell, row), &(key, prefix, file, side, kind, attack_time_s, aircraft, start, km, prior, target, spawn))
        in RUN_SHEET_2E.iter().zip(TWO_E_SORTIES).zip(&expected)
    {
        assert_eq!(*cell, Cell { key, centre_km: km, arm_s: start, close_s: 1800.0 });
        assert_eq!((row.cell, row.prefix, row.sortie().file), (key, prefix, file));
        assert_eq!(row.variant, EmptyTargetVariant::BuiltIn { file, shape: kind, attack_time_s });
        assert_eq!(row.predicted_transit_s(), 300.0, "{key}: nominal transit");
        assert_eq!(RUN_SHEET_2C.iter().find(|cell| cell.key == prior).unwrap().centre_km, km, "{key}: unchanged 2C target");
        let source = empty_target_source(row);
        let shape = inspect_sortie(&source).unwrap_or_else(|errors| panic!("{key}: {}", errors.join("\n")));
        assert_eq!(shape.side, side, "{key}: source side");
        let copy = named(&root, &format!("{key} sortie"));
        assert_eq!(copy.count_block_type("Plane"), aircraft, "{key}: aircraft count");
        let path = sortie_probe_path(copy);
        assert_eq!(path, SortieProbePath { spawn, target }, "{key}: placed Plane to WP 1");
        assert_eq!(world(km), target, "{key}: target coordinates");
        assert!(terrain.is_land_xz(path.target.0, path.target.1), "{key}: target over water");
        for attack in block_ids(copy, "MCU_CMD_AttackArea") {
            assert_eq!(by_id(copy, attack).pos_xz(), Some(target), "{key}: attack point");
        }
        assert_at(&root, &format!("{key} ARM"), start, &[id(copy, &format!("{prefix} START"))]);
        paths.push((key, side, path));
    }
    for (i, &(key, side, path)) in paths.iter().enumerate() {
        for &(other_key, other_side, other) in &paths[i + 1..] {
            let distance = path.separation_m(other);
            assert!(distance >= 45_000.0, "{key} to {other_key}: closest path distance {distance} m");
            if side != other_side {
                assert!(distance >= 90_000.0, "{key} to {other_key}: opposing paths {distance} m");
            }
        }
    }
    // Same-line EA/EC are separated end to end. EY/EI have a 3.333 km X gap
    // as well as their 45 km Z separation; target-to-target distance is wrong.
    for (key, closest, metres) in [
        ("EA", "EC", 46_666.667), ("EY", "EI", 45_123.288),
        ("EI", "EY", 45_123.288), ("EC", "EA", 46_666.667),
    ] {
        let path = paths.iter().find(|row| row.0 == key).unwrap().2;
        let nearest = paths.iter().filter(|row| row.0 != key)
            .map(|row| (row.0, path.separation_m(row.2)))
            .min_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
        assert_eq!(nearest.0, closest, "{key}: closest path");
        assert!((nearest.1 - metres).abs() < 0.001, "{key}: closest distance {} m", nearest.1);
    }
}

#[test]
fn flight2e_sources_and_placed_copies_match_builtins() {
    use super::library::{TestedShape::{S1, S3, S4}, inspect_sortie};
    let root = generate_probe_2e();
    for (cell, prefix, file, kind, attack_time_s, country, types, speed) in [
        ("EA", "SN01", "1950_US_F51_ArmedRecon_2plus2.Group", S3, 1200.0, "601", vec!["19", "19"], 520.0),
        ("EY", "SN02", "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", S4, 300.0, "503", vec!["19", "19"], 500.0),
        ("EI", "SN03", "1950_DPRK_Il10_KimpoAttack_2x4.Group", S4, 600.0, "503", vec!["20", "20"], 390.0),
        ("EC", "SN04", "1950_US_F51_CloseSupport_4ship.Group", S1, 900.0, "601", vec!["20"], 520.0),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("TemplateExamples/Historical1950").join(file);
        let original = parse_group_file(&std::fs::read_to_string(path).unwrap()).unwrap();
        let row = TWO_E_SORTIES.iter().find(|row| row.cell == cell).unwrap();
        let source = empty_target_source(row);
        assert_eq!(source, original, "{cell}: the entire source passed to placement must equal the parsed file");
        let shape = inspect_sortie(&source).unwrap_or_else(|errors| panic!("{cell}: {}", errors.join("\n")));
        assert_eq!(shape.kind, kind, "{cell}: inspected source shape");
        assert_eq!(shape.attack_time_s, attack_time_s, "{cell}: inspected attack duration");
        assert_eq!(shape.wp_speed_kmh, speed, "{cell}: placement speed");
        let copy = named(&root, &format!("{cell} sortie"));
        assert_eq!(number(named(copy, &format!("{prefix} ON STATION")), "Time"), attack_time_s, "{cell}");
        let path = sortie_probe_path(copy);
        assert!((path.target.0 - path.spawn.0 - speed / 3.6 * 300.0).abs() < 0.001, "{cell}: placement distance");
        for tree in [&source, copy] {
            let formations = block_ids(tree, "MCU_CMD_Formation");
            assert_eq!(formations.len(), types.len(), "{cell}: every Formation");
            for (&index, &formation_type) in formations.iter().zip(&types) {
                let command = by_id(tree, index);
                assert_eq!(command.property("FormationType"), Some(formation_type), "{cell}: Formation {index}");
                assert_eq!(command.property("FormationDensity"), Some("1"), "{cell}: Formation {index}");
            }
            for plane in block_ids(tree, "Plane") {
                assert_eq!(by_id(tree, plane).property("Country"), Some(country), "{cell}: Plane {plane}");
            }
        }
        for (source_id, placed_id) in block_ids(&source, "MCU_CMD_Formation").into_iter().zip(block_ids(copy, "MCU_CMD_Formation")) {
            let lead_names = |tree: &Il2Entity, index| by_id(tree, index).objects.iter().map(|&lead| {
                by_id(tree, number(by_id(tree, lead), "MisObjID") as i32).name().unwrap().to_owned()
            }).collect::<Vec<_>>();
            assert_eq!(lead_names(&source, source_id), lead_names(copy, placed_id), "{cell}: Formation lead preserved");
        }
    }
}

#[test]
fn flight2e_side_logic_and_icons_match_sorties() {
    let root = generate_probe_2e();
    for (cell, prefix, side, opposing, coalition, enemy_coalition) in [
        ("EA", "SN01", "N", "D", "[2]", "[1]"), ("EY", "SN02", "D", "N", "[1]", "[2]"),
        ("EI", "SN03", "D", "N", "[1]", "[2]"), ("EC", "SN04", "N", "D", "[2]", "[1]"),
    ] {
        let copy = named(&root, &format!("{cell} sortie"));
        let budget = named(copy, "Budget");
        assert_eq!(budget.children.len(), 14, "{cell}: one-copy budget");
        for node in &budget.children {
            assert!(node.name().unwrap().starts_with(&format!("BUD {side} ")), "{cell}: budget side");
        }
        let init = named(copy, &format!("AT {side} INIT"));
        let off = named(copy, &format!("AT {side} INIT OFF"));
        assert_eq!(init.targets, [off.index.unwrap()], "{cell}: side initialization");
        assert!(off.targets.contains(&id(copy, &format!("{prefix} HOT"))), "{cell}: side turns HOT off");
        assert!(copy.find_by_name(&format!("AT {opposing} INIT")).is_none(), "{cell}: wrong-side init");
        for suffix in ["START", "DELETE DELAY"] {
            let event = named(copy, &format!("{prefix} {suffix}"));
            for target in ["MARK", "CHECK"] {
                assert!(event.targets.contains(&id(budget, &format!("BUD {side} {target}"))), "{cell}: budget event {suffix}");
            }
        }
        for name in ["Zone IN", "Zone Out"] {
            assert_eq!(named(copy, name).property("PlaneCoalitions"), Some(enemy_coalition), "{cell}: {name}");
        }
        let icons = block_ids(named(&root, cell), "MCU_Icon");
        assert_eq!(icons.len(), 1, "{cell}: one cell icon");
        let icon = by_id(&root, icons[0]);
        assert_eq!(icon.name(), Some(cell));
        assert_eq!(icon.property("Coalitions"), Some(coalition), "{cell}: icon side");
    }
}

#[test]
fn flight2e_shells_breadcrumbs_close_and_no_spawn() {
    use super::shell::test_support::{active, ticks};
    use super::testkit::{WalkerConfig, WalkerInput, walk};
    let root = generate_probe_2e();
    assert_eq!(root.count_block_type("Plane"), 20);
    assert_eq!(root.count_block_type("MCU_TR_Entity"), 20);
    assert_eq!(root.count_block_type("MCU_CheckZone"), 8);
    for block in ["MCU_Spawner", "Airfield", "MCU_CMD_Damage"] {
        assert_eq!(root.count_block_type(block), 0, "flight 2E: unexpected {block}");
    }
    assert!(Flight::TwoE.spawns().is_empty());
    assert_eq!(spawn_json(Flight::TwoE), "[]\n");
    let (traced, map) = build(Flight::TwoE).traced(TraceCarrier::default());
    super::testkit::assert_links_resolve(&traced);
    let initialized = walk(&root, WalkerConfig::default(), &WalkerInput {
        until: ticks(1.0), ..WalkerInput::default()
    });
    // S3's second lead Covers and has no attack timer. Both S4 leads attack.
    for (cell, prefix, aircraft, attack_names, path_leads) in [
        ("EA", "SN01", 4, vec!["AttackArea 1"], 1),
        ("EY", "SN02", 4, vec!["AttackArea 1", "AttackArea 3"], 2),
        ("EI", "SN03", 8, vec!["AttackArea 1", "AttackArea 5"], 2),
        ("EC", "SN04", 4, vec!["AttackArea 1"], 1),
    ] {
        let copy = named(&root, &format!("{cell} sortie"));
        assert_eq!(copy.count_block_type("MCU_TR_Entity"), aircraft, "{cell}: no added enemy aircraft");
        let hot = id(copy, &format!("{prefix} HOT"));
        root.for_each(&mut |node| {
            if node.block_type == "MCU_Activate" {
                assert!(!node.targets.contains(&hot), "{cell}: no HOT switch");
            }
        });
        assert!(active(&initialized, id(copy, &format!("{prefix} COLD")), ticks(1.0)), "{cell}: starts COLD");
        assert!(!active(&initialized, hot, ticks(1.0)), "{cell}: starts without HOT");
        let wp = named(copy, "WP 1");
        assert_eq!(wp.objects.len(), path_leads, "{cell}: shared WP 1 leads");
        let attacks = block_ids(copy, "MCU_CMD_AttackArea");
        assert_eq!(attacks.len(), attack_names.len(), "{cell}: attack commands");
        let timers: Vec<_> = block_ids(copy, "MCU_Timer").into_iter().filter(|&index| {
            by_id(copy, index).targets.iter().any(|target| attacks.contains(target))
        }).collect();
        assert_eq!(timers.iter().map(|&index| by_id(copy, index).name().unwrap()).collect::<Vec<_>>(), attack_names, "{cell}: every lead attack timer");
        let mut attack_leads = BTreeSet::new();
        for &timer in &timers {
            assert!(wp.targets.contains(&timer), "{cell}: WP 1 issues timer {timer}");
            for &attack in &attacks {
                if by_id(copy, timer).targets.contains(&attack) {
                    attack_leads.extend(&by_id(copy, attack).objects);
                }
            }
        }
        assert_eq!(attack_leads, wp.objects.iter().collect::<BTreeSet<_>>(), "{cell}: both S4 leads covered");
        let mut sources = vec![id(copy, "WP 1"), id(copy, "Zone IN"), id(copy, "Zone Out")];
        sources.extend(timers);
        for suffix in ["START", "ON STATION", "COLD", "EXTEND", "HARD STOP", "LOSSES", "DONE HUB", "DELETE DELAY"] {
            sources.push(id(copy, &format!("{prefix} {suffix}")));
        }
        for source in sources {
            assert_eq!(map.entries.iter().filter(|entry| entry.source_index == source).count(), 1,
                "{cell}: breadcrumb for {:?}", by_id(copy, source).name());
            assert!(named(&root, &format!("{cell} CLOSE OFF")).targets.contains(&source), "{cell}: close observation {source}");
        }
        let close = named(&root, &format!("{cell} CLOSE"));
        assert_eq!(number(close, "Time"), 1800.0, "{cell}: safety close");
        let deletes: Vec<_> = close.targets.iter().map(|&index| by_id(&root, index))
            .filter(|node| node.block_type == "MCU_Delete").collect();
        assert_eq!(deletes.len(), aircraft, "{cell}: one safety Delete per aircraft");
        for entity in block_ids(copy, "MCU_TR_Entity") {
            assert_eq!(deletes.iter().filter(|node| node.objects == [entity]).count(), 1, "{cell}: Delete entity {entity}");
        }
        for plane in block_ids(copy, "Plane") {
            assert_eq!(by_id(copy, plane).property("AiRTBDecision"), Some("0"), "{cell}: placement keeps control");
        }
    }
}

#[test]
fn flight2e_walks_station_windows_and_snapshot() {
    use super::shell::test_support::{active, ticks, times};
    use super::testkit::{DeactivateRunningTimer, TimerRetrigger, WalkerConfig, walk};
    let root = generate_probe_2e();
    let input = empty_target_walk_input(&root, Flight::TwoE);
    assert_eq!(input.until, ticks(1800.0));
    assert_eq!(input.arrivals, [
        (ticks(360.0), id(named(&root, "EA sortie"), "WP 1").into()),
        (ticks(420.0), id(named(&root, "EY sortie"), "WP 1").into()),
        (ticks(480.0), id(named(&root, "EI sortie"), "WP 1").into()),
        (ticks(540.0), id(named(&root, "EC sortie"), "WP 1").into()),
    ]);
    assert!(input.tracks.is_empty());
    assert!(input.kills.is_empty());
    let log = walk(&root, WalkerConfig {
        timer_retrigger: TimerRetrigger::Restart,
        deactivate_running_timer: DeactivateRunningTimer::Cancels, ..WalkerConfig::default()
    }, &input);
    for (cell, prefix, start, wp, station, end, delete, aircraft, attack_names) in [
        ("EA", "SN01", 60.0, 360.0, 360.5, 1560.5, 1621.1, 4, vec!["AttackArea 1"]),
        // EY keeps its shipped five-minute attack; DONE and DELETE precede 900 s.
        ("EY", "SN02", 120.0, 420.0, 420.5, 720.5, 781.1, 4, vec!["AttackArea 1", "AttackArea 3"]),
        ("EI", "SN03", 180.0, 480.0, 480.5, 1080.5, 1141.1, 8, vec!["AttackArea 1", "AttackArea 5"]),
        ("EC", "SN04", 240.0, 540.0, 540.5, 1440.5, 1501.1, 4, vec!["AttackArea 1"]),
    ] {
        let copy = named(&root, &format!("{cell} sortie"));
        assert_eq!(times(&log, id(copy, &format!("{prefix} START"))), [ticks(start)], "{cell}: START once");
        let waypoint_times = times(&log, id(copy, "WP 1"));
        assert_eq!(waypoint_times.iter().filter(|&&time| time == ticks(wp)).count(), 1, "{cell}: one arrival");
        assert!(waypoint_times.iter().all(|&time| time == ticks(start + 2.0) || time == ticks(wp)), "{cell}: WP 1 orders and arrival");
        for attack in attack_names {
            assert_eq!(times(&log, id(copy, attack)), [ticks(station)], "{cell}: {attack}");
        }
        assert_eq!(times(&log, id(copy, &format!("{prefix} ON STATION IN"))), [ticks(station)], "{cell}: hold starts once");
        assert_eq!(times(&log, id(copy, &format!("{prefix} ON STATION"))), [ticks(end)], "{cell}: hold ends once");
        for suffix in ["COLD", "DONE HUB"] {
            assert_eq!(times(&log, id(copy, &format!("{prefix} {suffix}"))), [ticks(end)], "{cell}: {suffix}");
        }
        for suffix in ["DELETE DELAY", "DELETE"] {
            assert_eq!(times(&log, id(copy, &format!("{prefix} {suffix}"))), [ticks(delete)], "{cell}: {suffix}");
        }
        assert!(!active(&log, id(copy, &format!("{prefix} HOT")), ticks(900.0)), "{cell}: no HOT at snapshot");
        for name in ["Zone IN", "Zone Out"] {
            assert!(times(&log, id(copy, name)).is_empty(), "{cell}: no enemy zone event {name}");
        }
        assert!(times(&log, id(copy, &format!("{prefix} EXTEND"))).is_empty(), "{cell}: no extension");
        for seat in 1..=aircraft {
            assert_eq!(times(&log, id(&root, &format!("{cell} aircraft {seat} CLOSE DELETE"))), [ticks(1800.0)], "{cell}: safety Delete {seat}");
        }
    }
    for (cell, prefix, held) in [("EA", "SN01", 539.5), ("EI", "SN03", 419.5), ("EC", "SN04", 359.5)] {
        let copy = named(&root, &format!("{cell} sortie"));
        let station = times(&log, id(copy, &format!("{prefix} ON STATION IN")))[0];
        assert_eq!(ticks(900.0) - station, ticks(held), "{cell}: snapshot hold");
        assert!(ticks(900.0) - station >= ticks(300.0), "{cell}: at least five minutes on station");
        assert!(times(&log, id(copy, &format!("{prefix} DONE HUB"))).iter().all(|&time| time > ticks(900.0)), "{cell}: DONE before snapshot");
        for index in block_ids(named(&root, cell), "MCU_Delete") {
            assert!(times(&log, index).iter().all(|&time| time > ticks(900.0)), "{cell}: Delete {index} before snapshot");
        }
    }
}

#[test]
fn flight2e_run_sheet_and_cues_match_plan() {
    use super::shell::test_support::{ticks, times};
    use super::testkit::{WalkerConfig, walk};
    let root = generate_probe_2e();
    let cues: Vec<_> = CUES.iter().filter(|cue| cue.flight == Flight::TwoE).map(|cue| (cue.time_s, cue.text)).collect();
    assert_eq!(cues, [
        (0.0, "Flight 2E: spectate, then LCtrl+R to record. Note the first time the server overload message shows. Leave the server at 15:00"),
        (60.0, "Flight 2E: EA starts (4 aircraft flying)"),
        (120.0, "Flight 2E: EY starts (8 aircraft flying)"),
        (180.0, "Flight 2E: EI starts (16 aircraft flying)"),
        (240.0, "Flight 2E: EC starts (20 aircraft flying)"),
        (870.0, "Flight 2E: leave the server now (15:00)"),
        (1800.0, "Flight 2E closed. Every aircraft has a Delete. End the mission normally"),
    ]);
    let log = walk(&root, WalkerConfig::default(), &empty_target_walk_input(&root, Flight::TwoE));
    for (time, text) in cues { assert_eq!(times(&log, id(&root, text)), [ticks(time)]); }
    let report = builtin_sortie_schedule(&root);
    for line in [
        "| EA | 1950_US_F51_ArmedRecon_2plus2.Group | NATO | S3 | SN01 | 60 | 360 | 360.5 | 1560.5 | 1560.5 | 1621.1 | 335934 | 179477 | 292600.667 | 179477 | EC | 46.667 |",
        "| EY | 1950_DPRK_Yak_AirfieldRaid_2plus2.Group | DPRK | S4 | SN02 | 120 | 420 | 420.5 | 720.5 | 720.5 | 781.1 | 335934 | 269477 | 294267.333 | 269477 | EI | 45.123 |",
        "| EI | 1950_DPRK_Il10_KimpoAttack_2x4.Group | DPRK | S4 | SN03 | 180 | 480 | 480.5 | 1080.5 | 1080.5 | 1141.1 | 290934 | 314477 | 258434.000 | 314477 | EY | 45.123 |",
        "| EC | 1950_US_F51_CloseSupport_4ship.Group | NATO | S1 | SN04 | 240 | 540 | 540.5 | 1440.5 | 1440.5 | 1501.1 | 245934 | 179477 | 202600.667 | 179477 | EA | 46.667 |",
        "Leave the server at 15:00 (900 s mission time)", "nominal, not measured: START + 300 s",
        "both .spawns.json files contain []", "no HOT switch or enemy aircraft at the targets",
        "EY keeps its shipped 300 s attack", "DONE HUB at 720.5 s, DELETE DELAY at 781.1 s",
        "Formation commands as written", "both attack timers are observed", "Predicted pulse and command schedule",
    ] { assert!(report.contains(line), "flight 2E run sheet missing {line}"); }
}

#[test]
fn flight2f_cells_match_table_and_land_path_separation() {
    use super::library::TestedShape::S4;
    let root = generate_probe_2f();
    assert_eq!(root.name(), Some("P14_Probe_2F"));
    assert!(Flight::ALL.contains(&Flight::TwoF));
    assert!(!Flight::TwoF.is_frozen());
    assert!(!Flight::TwoF.regenerates_flown_files(), "2F is unflown");
    assert!(Flight::TwoF.writes_schedule());
    assert_eq!(RUN_SHEET_2F.len(), 6);
    assert_eq!(TWO_F_SORTIES.len(), 6);
    assert_eq!((TWO_F_TARGET_A.name, TWO_F_TARGET_A.centre_km), ("A", (230.0, 0.0)));
    assert_eq!((TWO_F_TARGET_B.name, TWO_F_TARGET_B.centre_km), ("B", (140.0, 0.0)));
    let terrain = crate::watermap::TerrainMap::builtin().unwrap();
    let mut paths = Vec::new();
    for ((cell, row), (key, prefix, formation_type, density, name, label, start, km, prior, target, spawn))
        in RUN_SHEET_2F.iter().zip(TWO_F_SORTIES).zip([
            ("PD", "SN01", 19, 0, "Pairs", "Dense", 60.0, (230.0, 0.0), "C0c", (335_934.0, 269_477.0), (294_267.333, 269_477.0)),
            ("PS", "SN02", 19, 1, "Pairs", "Safe", 390.0, (140.0, 0.0), "C2a", (245_934.0, 269_477.0), (204_267.333, 269_477.0)),
            ("PL", "SN03", 19, 2, "Pairs", "Loose", 720.0, (230.0, 0.0), "C0c", (335_934.0, 269_477.0), (294_267.333, 269_477.0)),
            ("WD", "SN04", 20, 0, "Wedge", "Dense", 1050.0, (140.0, 0.0), "C2a", (245_934.0, 269_477.0), (204_267.333, 269_477.0)),
            ("WS", "SN05", 20, 1, "Wedge", "Safe", 1380.0, (230.0, 0.0), "C0c", (335_934.0, 269_477.0), (294_267.333, 269_477.0)),
            ("WL", "SN06", 20, 2, "Wedge", "Loose", 1710.0, (140.0, 0.0), "C2a", (245_934.0, 269_477.0), (204_267.333, 269_477.0)),
        ])
    {
        assert_eq!(*cell, Cell { key, centre_km: km, arm_s: start, close_s: 2700.0 });
        assert_eq!((row.cell, row.prefix), (key, prefix));
        assert_eq!(row.variant, EmptyTargetVariant::BuiltInFormation {
            file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group", shape: S4, attack_time_s: 300, measured_transit_s: 264.1,
            formation: ProbeFormation { formation_type, formation_density: density, formation_name: name, editor_label: label },
        });
        assert_eq!(row.predicted_transit_s(), 264.1, "{key}: measured Yak transit");
        assert_eq!(RUN_SHEET_2C.iter().find(|cell| cell.key == prior).unwrap().centre_km, km, "{key}: unchanged 2C target");
        let copy = named(&root, &format!("{key} sortie"));
        assert_eq!(copy.count_block_type("Plane"), 4, "{key}: one Yak raid");
        let path = sortie_probe_path(copy);
        assert_eq!(path, SortieProbePath { spawn, target }, "{key}: placed lead Plane to WP 1");
        assert_eq!(world(km), target, "{key}: target coordinates");
        assert!(terrain.is_land_xz(path.target.0, path.target.1), "{key}: target over water");
        for attack in block_ids(copy, "MCU_CMD_AttackArea") {
            assert_eq!(by_id(copy, attack).pos_xz(), Some(target), "{key}: attack point");
        }
        assert_at(&root, &format!("{key} ARM"), start, &[id(copy, &format!("{prefix} START"))]);
        paths.push((key, path));
    }
    assert_eq!(RUN_SHEET_2E.iter().find(|cell| cell.key == "EY").unwrap().centre_km, (230.0, 0.0));
    for (i, &(cell, path)) in paths.iter().enumerate() {
        for &(other_cell, other) in &paths[i + 1..] {
            if path.target == other.target {
                assert_eq!(path, other, "{cell}/{other_cell}: same reused path");
            } else {
                let separation = path.separation_m(other);
                assert!(separation >= 45_000.0, "{cell}/{other_cell}: path separation {separation} m");
                assert!((separation - 48_333.333).abs() < 0.001, "{cell}/{other_cell}: end-to-end distance");
            }
        }
    }
}

#[test]
fn flight2f_sources_and_placed_formations_match_yak() {
    use super::library::{TestedShape::S4, inspect_sortie};
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("TemplateExamples/Historical1950/1950_DPRK_Yak_AirfieldRaid_2plus2.Group");
    let original = parse_group_file(&std::fs::read_to_string(path).unwrap()).unwrap();
    let root = generate_probe_2f();
    for (cell, prefix, formation_type, density) in [
        ("PD", "SN01", "19", "0"), ("PS", "SN02", "19", "1"), ("PL", "SN03", "19", "2"),
        ("WD", "SN04", "20", "0"), ("WS", "SN05", "20", "1"), ("WL", "SN06", "20", "2"),
    ] {
        let row = TWO_F_SORTIES.iter().find(|row| row.cell == cell).unwrap();
        let source = empty_target_source(row);
        assert_eq!(source, original, "{cell}: entire source passed to placement equals the parsed file");
        let shape = inspect_sortie(&source).unwrap_or_else(|errors| panic!("{cell}: {}", errors.join("\n")));
        assert_eq!(shape.kind, S4, "{cell}: inspected source shape");
        assert_eq!(shape.side, super::AirSide::Dprk, "{cell}: source side");
        assert_eq!(shape.attack_time_s, 300.0, "{cell}: shipped attack time");
        assert_eq!(shape.wp_speed_kmh, 500.0, "{cell}: placement speed");
        assert_eq!(shape.leads, [7, 11], "{cell}: S4 leads");
        let copy = named(&root, &format!("{cell} sortie"));
        assert_eq!(number(named(copy, &format!("{prefix} ON STATION")), "Time"), 300.0, "{cell}");
        let formations = block_ids(copy, "MCU_CMD_Formation");
        assert_eq!(formations.len(), 2, "{cell}: both Formation commands");
        for (placed_id, (source_id, source_lead, lead_name)) in formations.into_iter().zip([
            (35, 7, "Red 11"), (41, 11, "Blue 21"),
        ]) {
            let source_command = by_id(&source, source_id);
            assert_eq!(source_command.objects, [source_lead], "{cell}: source Objects");
            assert_eq!(source_command.property("FormationType"), Some("19"));
            assert_eq!(source_command.property("FormationDensity"), Some("1"));
            let command = by_id(copy, placed_id);
            assert_eq!(command.property("FormationType"), Some(formation_type), "{cell}: Formation {placed_id}");
            assert_eq!(command.property("FormationDensity"), Some(density), "{cell}: Formation {placed_id}");
            let plane = named(copy, lead_name);
            let lead = number(plane, "LinkTrId") as i32;
            assert_eq!(command.objects, [lead], "{cell}: original flight lead retained");
            assert_eq!(number(plane, "NumberInFormation"), 0.0, "{cell}: lead Plane");
            assert!(named(copy, "WP 1").objects.contains(&lead), "{cell}: shared waypoint lead");
        }
        for tree in [&source, copy] {
            for plane in block_ids(tree, "Plane") {
                assert_eq!(by_id(tree, plane).property("Country"), Some("503"), "{cell}: Plane {plane}");
            }
            for attack in block_ids(tree, "MCU_CMD_AttackArea") {
                assert_eq!(number(by_id(tree, attack), "Time"), 300.0, "{cell}: attack {attack}");
            }
        }
    }
}

#[test]
fn flight2f_only_formation_properties_differ_from_shipped_placement() {
    let root = generate_probe_2f();
    let mut shipped = Probe::new(Flight::TwoF);
    for (cell, prefix, km, start) in [
        ("PD", "SN01", (230.0, 0.0), 60.0), ("PS", "SN02", (140.0, 0.0), 390.0),
        ("PL", "SN03", (230.0, 0.0), 720.0), ("WD", "SN04", (140.0, 0.0), 1050.0),
        ("WS", "SN05", (230.0, 0.0), 1380.0), ("WL", "SN06", (140.0, 0.0), 1710.0),
    ] {
        shipped.start_cell(Cell { key: cell, centre_km: km, arm_s: start, close_s: 2700.0 });
        let row = SortieProbe { cell, prefix, file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group" };
        // Flight 2E's as-shipped path, including observation ids between cells.
        let (shape, _) = add_sortie_probe(&mut shipped, &row);
        observe_sortie_probe(&mut shipped, prefix, &shape);
        close_sortie_probe(&mut shipped, shape.side);
        let actual = named(&root, &format!("{cell} sortie"));
        let expected = named(&shipped.root, &format!("{cell} sortie"));
        if cell == "PS" {
            assert_eq!(actual, expected, "PS: no AST difference from as shipped");
            assert_eq!(serialize_probe(actual), serialize_probe(expected), "PS: byte-identical placement");
        }
        let mut restored = actual.clone();
        for index in block_ids(&restored, "MCU_CMD_Formation") {
            let command = by_id_mut(&mut restored, index);
            command.set_property("FormationType", "19");
            command.set_property("FormationDensity", "1");
        }
        assert_eq!(&restored, expected, "{cell}: only FormationType/FormationDensity may differ");
        assert_eq!(serialize_probe(&restored), serialize_probe(expected), "{cell}: all other bytes preserved");
    }
}

#[test]
fn flight2f_placed_formations_reject_missing_or_nonlead_objects() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let mut p = Probe::new(Flight::TwoF);
    p.start_cell(Cell { key: "PD", centre_km: (230.0, 0.0), arm_s: 60.0, close_s: 2700.0 });
    let (shape, _) = add_sortie_probe(&mut p, &SortieProbe {
        cell: "PD", prefix: "SN01", file: "1950_DPRK_Yak_AirfieldRaid_2plus2.Group",
    });
    let placed = named(&p.root, "PD sortie");
    let command = block_ids(placed, "MCU_CMD_Formation")[1];
    let mut missing = placed.clone();
    missing.for_each_mut(&mut |node| node.children.retain(|child| child.block_type != "MCU_CMD_Formation"));
    let mut cases = vec![(missing, "missing MCU_CMD_Formation")];
    for objects in [vec![], vec![shape.planes[1]], shape.leads.clone()] {
        let mut broken = placed.clone();
        by_id_mut(&mut broken, command).set_objects(objects);
        cases.push((broken, "must name one inspected flight lead"));
    }
    for (mut copy, expected) in cases {
        let before = copy.clone();
        let error = catch_unwind(AssertUnwindSafe(|| set_placed_probe_formation(&mut copy, "PD", &shape, ProbeFormation {
            formation_type: 19, formation_density: 0, formation_name: "Pairs", editor_label: "Dense",
        }))).expect_err("invalid placed formations must fail");
        let message = error.downcast_ref::<String>().map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied()).unwrap();
        assert!(message.contains("flight 2F PD") && message.contains(expected), "{message}");
        assert_eq!(copy, before, "PD: validate every Formation before changing any");
    }
}

#[test]
fn flight2f_side_logic_and_icons_are_dprk() {
    let root = generate_probe_2f();
    for (cell, prefix) in [
        ("PD", "SN01"), ("PS", "SN02"), ("PL", "SN03"),
        ("WD", "SN04"), ("WS", "SN05"), ("WL", "SN06"),
    ] {
        let copy = named(&root, &format!("{cell} sortie"));
        let budget = named(copy, "Budget");
        assert_eq!(budget.children.len(), 14, "{cell}: own one-copy budget");
        for node in &budget.children {
            assert!(node.name().unwrap().starts_with("BUD D "), "{cell}: DPRK budget");
        }
        let init = named(copy, "AT D INIT");
        let off = named(copy, "AT D INIT OFF");
        assert_eq!(init.targets, [off.index.unwrap()], "{cell}: DPRK initialization");
        assert!(off.targets.contains(&id(copy, &format!("{prefix} HOT"))), "{cell}: HOT off");
        assert!(copy.find_by_name("AT N INIT").is_none(), "{cell}: no NATO initialization");
        for suffix in ["START", "DELETE DELAY"] {
            for target in ["MARK", "CHECK"] {
                assert!(named(copy, &format!("{prefix} {suffix}")).targets.contains(&id(budget, &format!("BUD D {target}"))),
                    "{cell}: DPRK budget event {suffix}");
            }
        }
        for name in ["Zone IN", "Zone Out"] {
            assert_eq!(named(copy, name).property("PlaneCoalitions"), Some("[2]"), "{cell}: enemy coalition");
        }
        let icons = block_ids(named(&root, cell), "MCU_Icon");
        assert_eq!(icons.len(), 1, "{cell}: one icon");
        let icon = by_id(&root, icons[0]);
        assert_eq!(icon.name(), Some(cell));
        assert_eq!(icon.property("Coalitions"), Some("[1]"), "{cell}: DPRK icon");
    }
}

#[test]
fn flight2f_shells_breadcrumbs_close_and_no_spawn() {
    use super::shell::test_support::{active, ticks};
    use super::testkit::{WalkerConfig, WalkerInput, walk};
    let root = generate_probe_2f();
    assert_eq!(root.count_block_type("Plane"), 24);
    assert_eq!(root.count_block_type("MCU_TR_Entity"), 24);
    assert_eq!(root.count_block_type("MCU_CheckZone"), 12);
    for block in ["MCU_Spawner", "Airfield", "MCU_CMD_Damage"] {
        assert_eq!(root.count_block_type(block), 0, "flight 2F: unexpected {block}");
    }
    assert!(Flight::TwoF.spawns().is_empty());
    assert_eq!(spawn_json(Flight::TwoF), "[]\n");
    let (traced, map) = build(Flight::TwoF).traced(TraceCarrier::default());
    super::testkit::assert_links_resolve(&traced);
    let initialized = walk(&root, WalkerConfig::default(), &WalkerInput {
        until: ticks(1.0), ..WalkerInput::default()
    });
    for (cell, prefix) in [
        ("PD", "SN01"), ("PS", "SN02"), ("PL", "SN03"),
        ("WD", "SN04"), ("WS", "SN05"), ("WL", "SN06"),
    ] {
        let copy = named(&root, &format!("{cell} sortie"));
        assert_eq!(copy.count_block_type("MCU_TR_Entity"), 4, "{cell}: no enemy aircraft");
        let hot = id(copy, &format!("{prefix} HOT"));
        root.for_each(&mut |node| {
            if node.block_type == "MCU_Activate" {
                assert!(!node.targets.contains(&hot), "{cell}: no HOT switch");
            }
        });
        assert!(active(&initialized, id(copy, &format!("{prefix} COLD")), ticks(1.0)), "{cell}: COLD");
        assert!(!active(&initialized, hot, ticks(1.0)), "{cell}: HOT off");
        let wp = named(copy, "WP 1");
        assert_eq!(wp.objects.len(), 2, "{cell}: shared S4 waypoint");
        let attacks = block_ids(copy, "MCU_CMD_AttackArea");
        assert_eq!(attacks.len(), 2, "{cell}: two attack leads");
        let timers: Vec<_> = block_ids(copy, "MCU_Timer").into_iter().filter(|&index| {
            by_id(copy, index).targets.iter().any(|target| attacks.contains(target))
        }).collect();
        assert_eq!(timers.iter().map(|&index| by_id(copy, index).name().unwrap()).collect::<Vec<_>>(),
            ["AttackArea 1", "AttackArea 3"], "{cell}: both attack timers");
        let mut attack_leads = BTreeSet::new();
        for &timer in &timers {
            assert!(wp.targets.contains(&timer), "{cell}: shared WP 1 targets timer {timer}");
            for &attack in &attacks {
                if by_id(copy, timer).targets.contains(&attack) {
                    attack_leads.extend(&by_id(copy, attack).objects);
                }
            }
        }
        assert_eq!(attack_leads, wp.objects.iter().collect::<BTreeSet<_>>(), "{cell}: both S4 leads");
        let mut sources = vec![id(copy, "WP 1"), id(copy, "Zone IN"), id(copy, "Zone Out")];
        sources.extend(timers);
        for suffix in ["START", "ON STATION", "COLD", "EXTEND", "HARD STOP", "LOSSES", "DONE HUB", "DELETE DELAY"] {
            sources.push(id(copy, &format!("{prefix} {suffix}")));
        }
        for source in sources {
            assert_eq!(map.entries.iter().filter(|entry| entry.source_index == source).count(), 1,
                "{cell}: breadcrumb for {:?}", by_id(copy, source).name());
            assert!(named(&root, &format!("{cell} CLOSE OFF")).targets.contains(&source), "{cell}: close observation {source}");
        }
        let close = named(&root, &format!("{cell} CLOSE"));
        assert_eq!(number(close, "Time"), 2700.0, "{cell}: safety close");
        let deletes: Vec<_> = close.targets.iter().map(|&index| by_id(&root, index))
            .filter(|node| node.block_type == "MCU_Delete").collect();
        assert_eq!(deletes.len(), 4, "{cell}: one safety Delete per aircraft");
        for entity in block_ids(copy, "MCU_TR_Entity") {
            assert_eq!(deletes.iter().filter(|node| node.objects == [entity]).count(), 1, "{cell}: Delete entity {entity}");
        }
        for plane in block_ids(copy, "Plane") {
            assert_eq!(by_id(copy, plane).property("AiRTBDecision"), Some("0"), "{cell}: placement keeps control");
        }
        for index in block_ids(copy, "MCU_CMD_Formation") {
            assert_eq!(by_id(copy, index), by_id(&traced, index), "{cell}: traced Formation unchanged");
        }
    }
}

#[test]
fn flight2f_walks_station_windows_target_reuse_and_low_load() {
    use super::shell::test_support::{active, ticks, times};
    use super::testkit::{DeactivateRunningTimer, TimerRetrigger, WalkerConfig, walk};
    let root = generate_probe_2f();
    let input = empty_target_walk_input(&root, Flight::TwoF);
    assert_eq!(input.until, ticks(2700.0));
    assert_eq!(input.arrivals, [
        (ticks(324.1), id(named(&root, "PD sortie"), "WP 1").into()),
        (ticks(654.1), id(named(&root, "PS sortie"), "WP 1").into()),
        (ticks(984.1), id(named(&root, "PL sortie"), "WP 1").into()),
        (ticks(1314.1), id(named(&root, "WD sortie"), "WP 1").into()),
        (ticks(1644.1), id(named(&root, "WS sortie"), "WP 1").into()),
        (ticks(1974.1), id(named(&root, "WL sortie"), "WP 1").into()),
    ]);
    assert!(input.tracks.is_empty());
    assert!(input.kills.is_empty());
    assert_eq!(TWO_F_LEAVE_S, 2310.0);
    let log = walk(&root, WalkerConfig {
        timer_retrigger: TimerRetrigger::Restart,
        deactivate_running_timer: DeactivateRunningTimer::Cancels, ..WalkerConfig::default()
    }, &input);
    let mut events = Vec::new();
    for (cell, prefix, start, wp, station, end, delete) in [
        ("PD", "SN01", 60.0, 324.1, 324.6, 624.6, 685.2),
        ("PS", "SN02", 390.0, 654.1, 654.6, 954.6, 1015.2),
        ("PL", "SN03", 720.0, 984.1, 984.6, 1284.6, 1345.2),
        ("WD", "SN04", 1050.0, 1314.1, 1314.6, 1614.6, 1675.2),
        ("WS", "SN05", 1380.0, 1644.1, 1644.6, 1944.6, 2005.2),
        ("WL", "SN06", 1710.0, 1974.1, 1974.6, 2274.6, 2335.2),
    ] {
        let copy = named(&root, &format!("{cell} sortie"));
        assert_eq!(times(&log, id(copy, &format!("{prefix} START"))), [ticks(start)], "{cell}: START once");
        let waypoint_times = times(&log, id(copy, "WP 1"));
        assert_eq!(waypoint_times.iter().filter(|&&time| time == ticks(wp)).count(), 1, "{cell}: one arrival");
        assert!(waypoint_times.iter().all(|&time| time == ticks(start + 2.0) || time == ticks(wp)), "{cell}: WP 1 orders and arrival");
        for attack in ["AttackArea 1", "AttackArea 3"] {
            assert_eq!(times(&log, id(copy, attack)), [ticks(station)], "{cell}: {attack}");
        }
        assert_eq!(times(&log, id(copy, &format!("{prefix} ON STATION IN"))), [ticks(station)], "{cell}: hold starts once");
        assert_eq!(times(&log, id(copy, &format!("{prefix} ON STATION"))), [ticks(end)], "{cell}: hold ends once");
        assert_eq!(ticks(end) - ticks(station), ticks(300.0), "{cell}: unchanged attack duration");
        assert!(end < 2310.0, "{cell}: attack must end before host leaves");
        for suffix in ["COLD", "DONE HUB"] {
            assert_eq!(times(&log, id(copy, &format!("{prefix} {suffix}"))), [ticks(end)], "{cell}: {suffix}");
        }
        for suffix in ["DELETE DELAY", "DELETE"] {
            assert_eq!(times(&log, id(copy, &format!("{prefix} {suffix}"))), [ticks(delete)], "{cell}: {suffix}");
        }
        assert!(!active(&log, id(copy, &format!("{prefix} HOT")), ticks(2310.0)), "{cell}: no HOT at leave");
        for name in ["Zone IN", "Zone Out"] {
            assert!(times(&log, id(copy, name)).is_empty(), "{cell}: no enemy zone event {name}");
        }
        for suffix in ["EXTEND", "LOSSES"] {
            assert!(times(&log, id(copy, &format!("{prefix} {suffix}"))).is_empty(), "{cell}: no {suffix}");
        }
        for seat in 1..=4 {
            assert_eq!(times(&log, id(&root, &format!("{cell} aircraft {seat} CLOSE DELETE"))), [ticks(2700.0)], "{cell}: safety Delete {seat}");
        }
        events.push((times(&log, id(copy, &format!("{prefix} START")))[0], 4));
        events.push((times(&log, id(copy, &format!("{prefix} DELETE DELAY")))[0], -4));
    }
    // The shipped shell does not cancel HARD STOP at DONE. The DONE latch
    // prevents a second completion; close cancels the later three timers.
    for (cell, prefix, hard_stop) in [
        ("PD", "SN01", vec![1860.0]), ("PS", "SN02", vec![2190.0]), ("PL", "SN03", vec![2520.0]),
        ("WD", "SN04", vec![]), ("WS", "SN05", vec![]), ("WL", "SN06", vec![]),
    ] {
        let copy = named(&root, &format!("{cell} sortie"));
        assert_eq!(times(&log, id(copy, &format!("{prefix} HARD STOP"))),
            hard_stop.into_iter().map(ticks).collect::<Vec<_>>(), "{cell}: unchanged shell HARD STOP");
    }
    for (previous, prefix, delete, next, start) in [
        ("PD", "SN01", 685.2, "PL", 720.0), ("PS", "SN02", 1015.2, "WD", 1050.0),
        ("PL", "SN03", 1345.2, "WS", 1380.0), ("WD", "SN04", 1675.2, "WL", 1710.0),
    ] {
        let previous = named(&root, &format!("{previous} sortie"));
        let next_copy = named(&root, &format!("{next} sortie"));
        assert_eq!(sortie_probe_path(previous), sortie_probe_path(next_copy), "{next}: reused path");
        assert_eq!(times(&log, id(previous, &format!("{prefix} DELETE DELAY"))), [ticks(delete)]);
        let starts = times(&log, id(&root, &format!("{next} ARM")));
        assert_eq!(starts, [ticks(start)]);
        assert!(ticks(delete) < starts[0], "{next}: previous cell deleted before target reuse");
    }
    events.sort_unstable();
    let mut alive = 0;
    let mut maximum = 0;
    for (time, delta) in events {
        alive += delta;
        maximum = maximum.max(alive);
        assert!((0..=8).contains(&alive), "{time}: {alive} aircraft alive");
    }
    assert_eq!(maximum, 8, "at most two four-aircraft cells at once");
    assert_eq!(alive, 0, "all cells deleted");
}

#[test]
fn flight2f_run_sheet_and_cues_match_plan() {
    use super::shell::test_support::{ticks, times};
    use super::testkit::{WalkerConfig, walk};
    let root = generate_probe_2f();
    let cues: Vec<_> = CUES.iter().filter(|cue| cue.flight == Flight::TwoF).map(|cue| (cue.time_s, cue.text)).collect();
    assert_eq!(cues, [
        (0.0, "Flight 2F: spectate, then LCtrl+R to record. Check the menu now and then for the server overload message. Leave the server at 38:30"),
        (60.0, "Flight 2F: PD starts (Pairs, Dense)"),
        (390.0, "Flight 2F: PS starts (Pairs, Safe)"),
        (720.0, "Flight 2F: PL starts (Pairs, Loose)"),
        (1050.0, "Flight 2F: WD starts (Wedge, Dense)"),
        (1380.0, "Flight 2F: WS starts (Wedge, Safe)"),
        (1710.0, "Flight 2F: WL starts (Wedge, Loose)"),
        (2280.0, "Flight 2F: leave the server at 38:30"),
        (2700.0, "Flight 2F closed. Every aircraft has a Delete. End the mission normally"),
    ]);
    let log = walk(&root, WalkerConfig::default(), &empty_target_walk_input(&root, Flight::TwoF));
    let report = builtin_formation_schedule(&root);
    for (time, text) in cues {
        assert_eq!(times(&log, id(&root, text)), [ticks(time)]);
        assert!(report.contains(&format!("| {time} | {text} |")), "schedule cue {text}");
    }
    for line in [
        "| PD | 19 Pairs | 0 Dense | SN01 | A | 60.0 | 324.1 | 324.6 | 624.6 | 624.6 | 685.2 | 335934 | 269477 | 294267.333 | 269477 |",
        "| PS | 19 Pairs | 1 Safe | SN02 | B | 390.0 | 654.1 | 654.6 | 954.6 | 954.6 | 1015.2 | 245934 | 269477 | 204267.333 | 269477 |",
        "| PL | 19 Pairs | 2 Loose | SN03 | A | 720.0 | 984.1 | 984.6 | 1284.6 | 1284.6 | 1345.2 | 335934 | 269477 | 294267.333 | 269477 |",
        "| WD | 20 Wedge | 0 Dense | SN04 | B | 1050.0 | 1314.1 | 1314.6 | 1614.6 | 1614.6 | 1675.2 | 245934 | 269477 | 204267.333 | 269477 |",
        "| WS | 20 Wedge | 1 Safe | SN05 | A | 1380.0 | 1644.1 | 1644.6 | 1944.6 | 1944.6 | 2005.2 | 335934 | 269477 | 294267.333 | 269477 |",
        "| WL | 20 Wedge | 2 Loose | SN06 | B | 1710.0 | 1974.1 | 1974.6 | 2274.6 | 2274.6 | 2335.2 | 245934 | 269477 | 204267.333 | 269477 |",
        "| 0.0 | 60.0 | none | 0 |", "| 60.0 | 390.0 | PD | 4 |", "| 390.0 | 685.2 | PD, PS | 8 |",
        "| 685.2 | 720.0 | PS | 4 |", "| 720.0 | 1015.2 | PS, PL | 8 |", "| 1015.2 | 1050.0 | PL | 4 |",
        "| 1050.0 | 1345.2 | PL, WD | 8 |", "| 1345.2 | 1380.0 | WD | 4 |", "| 1380.0 | 1675.2 | WD, WS | 8 |",
        "| 1675.2 | 1710.0 | WS | 4 |", "| 1710.0 | 2005.2 | WS, WL | 8 |", "| 2005.2 | 2335.2 | WL | 4 |",
        "| 2335.2 | 2700.0 | none | 0 |", "Most aircraft alive at once: 8",
        "Leave the server at 38:30 (2310 s mission time)", "flight 2E's measured Yak transit: START + 264.1 s",
        "both .spawns.json files contain []", "no HOT switch or enemy aircraft", "Both S4 leads' attack timers",
        "A/B path separation: 48.333 km", "Formation commands as written", "Predicted pulse and command schedule",
    ] { assert!(report.contains(line), "flight 2F run sheet missing {line}"); }
}

#[test]
fn flight2b_sorties_and_breadcrumbs_match_the_plan() {
    let p = build(Flight::TwoB);
    assert_eq!(p.root.name(), Some("P14_Probe_2B"));
    assert_eq!(p.root.count_block_type("Plane"), 20);
    assert_eq!(p.root.count_block_type("MCU_CheckZone"), 8);
    assert_eq!(p.root.count_block_type("MCU_Spawner"), 0);
    assert_eq!(p.root.count_block_type("MCU_CMD_Damage"), 0);
    let terrain = crate::watermap::TerrainMap::builtin().unwrap();
    let targets = [(245_934.0, 179_477.0), (245_934.0, 269_477.0), (335_934.0, 179_477.0), (335_934.0, 269_477.0)];
    let (traced, map) = build(Flight::TwoB).traced(TraceCarrier::default());
    super::testkit::assert_links_resolve(&traced);
    for (cell, prefix, start_s, station_s, target) in [
        ("T-s1", "SN01", 60.0, 600.0, targets[0]),
        ("T-s2", "SN02", 90.0, 1080.0, targets[1]),
        ("T-s3", "SN03", 120.0, 600.0, targets[2]),
        ("T-s4", "SN04", 150.0, 600.0, targets[3]),
    ] {
        let copy = named(&p.root, &format!("{cell} sortie"));
        assert_at(&p.root, &format!("{cell} ARM"), start_s, &[id(copy, &format!("{prefix} START"))]);
        assert_eq!(number(named(copy, &format!("{prefix} ON STATION")), "Time"), station_s);
        assert_eq!(number(named(copy, &format!("{prefix} EXTEND")), "Time"), 600.0);
        assert_eq!(copy.count_block_type("MCU_CheckZone"), 2);
        assert!(terrain.is_land_xz(target.0, target.1), "{cell}: target over water");
        assert!((target.0 - 105_934.0).hypot(target.1 - 269_477.0) >= 80_000.0);
        let zone_in = named(copy, "Zone IN");
        let zone_out = named(copy, "Zone Out");
        for (zone, closer) in [(zone_in, "1"), (zone_out, "0")] {
            assert_eq!(zone.pos_xz(), Some(target));
            assert_eq!(number(zone, "Zone"), 35_000.0);
            assert_eq!(zone.property("Closer"), Some(closer));
            assert_eq!(zone.property("PlaneCoalitions"), Some("[1]"));
        }
        assert!(named(copy, "WP 1").targets.contains(&zone_in.index.unwrap()));
        assert!(!named(copy, "WP 1").targets.contains(&id(copy, "PULSE OUT")));
        let arm_targets: Vec<_> = zone_in.targets.iter().copied()
            .filter(|&target| by_id(&p.root, target).block_type != "MCU_TR_Subtitle").collect();
        assert_eq!(arm_targets, [id(copy, "Zone Out ReActivate"), id(copy, "PULSE OUT")]);
        let attacks: BTreeSet<_> = block_ids(copy, "MCU_CMD_AttackArea").into_iter().collect();
        assert_eq!(attacks.len(), if cell == "T-s2" { 2 } else { 1 });
        let mut sources = vec![zone_in.index.unwrap(), zone_out.index.unwrap(), id(copy, "WP 1")];
        sources.extend(block_ids(copy, "MCU_Timer").into_iter().filter(|&index|
            by_id(copy, index).targets.iter().any(|target| attacks.contains(target))));
        for suffix in ["START", "LOSSES", "COLD", "EXTEND", "HARD STOP", "ON STATION", "DONE HUB", "DELETE DELAY"] {
            sources.push(id(copy, &format!("{prefix} {suffix}")));
        }
        for source in sources {
            assert_eq!(map.entries.iter().filter(|entry| entry.source_index == source).count(), 1,
                "flight 2B {cell}: missing or duplicate breadcrumb for {}", by_id(&p.root, source).name()
                    .unwrap_or_else(|| panic!("flight 2B {cell}: trace source {source} missing Name")));
        }
        let cell_root = named(&p.root, cell);
        let close = named(cell_root, &format!("{cell} CLOSE"));
        assert_eq!(number(close, "Time"), 1800.0);
        let deletes: Vec<_> = close.targets.iter().map(|&target| by_id(&p.root, target))
            .filter(|node| node.block_type == "MCU_Delete").collect();
        let entities = block_ids(cell_root, "MCU_TR_Entity");
        assert_eq!(deletes.len(), entities.len());
        for entity in entities {
            assert_eq!(deletes.iter().filter(|delete| delete.objects == [entity]).count(), 1, "{cell}: entity {entity}");
        }
    }
    for (i, a) in targets.iter().enumerate() {
        for b in &targets[i + 1..] {
            assert!((a.0 - b.0).hypot(a.1 - b.1) >= 80_000.0);
        }
    }
    assert_switch(&p.root, "T-s1 AREA HOT ON", "MCU_Activate", 600.0, &[id(&p.root, "SN01 HOT")]);
    assert_switch(&p.root, "T-s1 AREA COLD OFF", "MCU_Deactivate", 600.0, &[id(&p.root, "SN01 COLD")]);
    for cell in ["T-s2", "T-s3", "T-s4"] {
        assert!(p.root.find_by_name(&format!("{cell} AREA HOT ON")).is_none());
    }
}

#[test]
fn flight2b_enemies_are_isolated_and_scheduled() {
    let root = generate_probe_2b();
    let (_, map) = build(Flight::TwoB).traced(TraceCarrier::default());
    for (cell, target, activate_s, delete_s) in [
        ("T-s3", (335_934.0, 179_477.0), 30.0, 586.0),
        ("T-s4", (335_934.0, 269_477.0), 556.0, 736.0),
    ] {
        let name = format!("{cell} enemy");
        let flight = named(&root, &name);
        assert_eq!(flight.count_block_type("Plane"), 2);
        assert_eq!(flight.count_block_type("MCU_CMD_AttackArea"), 0);
        let entities = block_ids(flight, "MCU_TR_Entity");
        let sortie = named(&root, &format!("{cell} sortie"));
        let attacks: Vec<_> = block_ids(sortie, "MCU_CMD_AttackArea").into_iter().map(|index| {
            by_id(sortie, index).pos_xz()
                .unwrap_or_else(|| panic!("flight 2B {cell}: AttackArea {index} missing XPos/ZPos"))
        }).collect();
        assert_eq!(attacks, [target], "flight 2B {cell}: attack point must be on target");
        for plane in block_ids(flight, "Plane") {
            let node = by_id(flight, plane);
            assert_eq!(crate::model_spec::script_id(node.property("Script").unwrap()), "yak9p");
            for (key, value) in [("Country", "503"), ("AiRTBDecision", "0"), ("StartType", "0"), ("Engageable", "0")] {
                assert_eq!(node.property(key), Some(value), "{name}: {key}");
            }
            assert_eq!(number(node, "YPos"), 8000.0);
            let position = node.pos_xz()
                .unwrap_or_else(|| panic!("flight 2B {cell}: enemy Plane {plane} missing XPos/ZPos"));
            assert!((17_000.0..30_000.0).contains(&(position.0 - target.0).hypot(position.1 - target.1)));
            assert_eq!(number(node, "YOri"), 22.5);
        }
        assert_eq!(by_id(flight, entities[1]).targets, [entities[0]]);
        assert_eq!(named(&root, &format!("{name} ACTIVATE")).objects, entities);
        let behaviour = named(&root, &format!("{name} NON TARGETABLE"));
        assert_eq!(behaviour.block_type, "MCU_CMD_Behaviour");
        assert_eq!(behaviour.objects, entities);
        let reference = reference_block("MCU_CMD_Behaviour");
        for key in ["Filter", "Vulnerable", "LimitAmmo", "AILevel", "Country", "FloatParam"] {
            assert_eq!(behaviour.property(key), reference.property(key), "{name}: shipped {key}");
        }
        assert_eq!(behaviour.property("Filter"), Some("8"));
        assert_eq!(behaviour.property("Engageable"), Some("0"));
        let behaviour_delay = named(&root, &format!("{name} BEHAVIOUR DELAY"));
        assert_eq!(number(behaviour_delay, "Time"), 0.1);
        assert_eq!(behaviour_delay.targets, [behaviour.index.unwrap()]);
        let orders = named(&root, &format!("{name} ORBIT DELAY"));
        assert_eq!(number(orders, "Time"), 0.5);
        assert_eq!(orders.targets, [id(&root, &format!("{name} FORMATION")), id(&root, &format!("{name} ORBIT 2"))]);
        let offsets = [
            (0.0, 18_000.0), (1414.214, 18_585.786), (2000.0, 20_000.0), (1414.214, 21_414.214),
            (0.0, 22_000.0), (-1414.214, 21_414.214), (-2000.0, 20_000.0), (-1414.214, 18_585.786),
        ];
        let points: Vec<_> = (1..=8).map(|seat| {
            named(&root, &format!("{name} ORBIT {seat}")).pos_xz()
                .unwrap_or_else(|| panic!("flight 2B {cell}: enemy ORBIT {seat} missing XPos/ZPos"))
        }).collect();
        for (i, offset) in offsets.into_iter().enumerate() {
            let wp = named(&root, &format!("{name} ORBIT {}", i + 1));
            let point = points[i];
            assert!((point.0 - target.0 - offset.0).abs() < 0.001);
            assert!((point.1 - target.1 - offset.1).abs() < 0.001);
            assert!((point.0 - target.0).hypot(point.1 - target.1) < 30_000.0,
                "flight 2B {cell}: ORBIT {} outside target margin", i + 1);
            let previous = points[(i + points.len() - 1) % points.len()];
            let next = points[(i + 1) % points.len()];
            let incoming = (point.0 - previous.0, point.1 - previous.1);
            let outgoing = (next.0 - point.0, next.1 - point.1);
            let turn = ((incoming.0 * outgoing.0 + incoming.1 * outgoing.1)
                / (incoming.0.hypot(incoming.1) * outgoing.0.hypot(outgoing.1))).clamp(-1.0, 1.0).acos().to_degrees();
            assert!(turn < 90.0 && (turn - 45.0).abs() < 0.001,
                "flight 2B {cell}: ORBIT {} turn is {turn} degrees", i + 1);
            for attack in &attacks {
                assert!((point.0 - attack.0).hypot(point.1 - attack.1) >= 17_000.0,
                    "flight 2B {cell}: ORBIT {} inside attack margin", i + 1);
                // Pin the whole straight leg, not only its two endpoints.
                let along = (((attack.0 - point.0) * outgoing.0 + (attack.1 - point.1) * outgoing.1)
                    / (outgoing.0 * outgoing.0 + outgoing.1 * outgoing.1)).clamp(0.0, 1.0);
                let nearest = (point.0 + along * outgoing.0, point.1 + along * outgoing.1);
                assert!((nearest.0 - attack.0).hypot(nearest.1 - attack.1) >= 18_000.0,
                    "flight 2B {cell}: ORBIT {} leg inside attack margin", i + 1);
            }
            assert_eq!(number(wp, "YPos"), 8000.0);
            assert_eq!(number(wp, "Speed"), 500.0);
            assert_eq!(wp.property("Priority"), Some("2"));
            assert_eq!(wp.objects, [entities[0]]);
            assert_eq!(wp.targets, [id(&root, &format!("{name} ORBIT {}", (i + 1) % 8 + 1))]);
        }
        let start = named(&root, &format!("{name} ACTIVATE AT"));
        assert_eq!(number(start, "Time"), activate_s);
        for target in [id(&root, &format!("{name} ACTIVATE")), behaviour_delay.index.unwrap(), orders.index.unwrap()] {
            assert!(start.targets.contains(&target));
        }
        let delete = named(&root, &format!("{name} DELETE AT"));
        assert_eq!(number(delete, "Time"), delete_s);
        let commands: Vec<_> = delete.targets.iter().map(|&index| by_id(&root, index))
            .filter(|node| node.block_type == "MCU_Delete").collect();
        assert_eq!(commands.len(), 2);
        for entity in entities {
            assert_eq!(commands.iter().filter(|node| node.objects == [entity]).count(), 1);
        }
        for source in [start, delete] {
            assert_eq!(map.entries.iter().filter(|entry| Some(entry.source_index) == source.index).count(), 1);
        }
    }
}

#[test]
fn flight2b_walks_all_four_outcomes() {
    use super::shell::test_support::{configs, ticks, times};
    let root = generate_probe_2b();
    for config in configs() {
        let log = super::testkit::walk(&root, config, &reflight_walk_input(&root));
        for (prefix, done_s) in [("SN01", 1546.5), ("SN03", 586.0), ("SN04", 736.0)] {
            assert_eq!(times(&log, id(&root, &format!("{prefix} DONE HUB"))), [ticks(done_s)]);
        }
        let cold = times(&log, id(&root, "SN02 COLD"));
        assert_eq!(cold.len(), 1);
        assert!([ticks(1449.5), ticks(1450.0)].contains(&cold[0]));
        assert_eq!(times(&log, id(&root, "SN02 DONE HUB")), cold);
        assert_eq!(times(&log, id(&root, "SN01 EXTEND")), [ticks(1546.5)]);
        for (cell, seen_s, gone_s) in [("T-s3", 406.0, 586.0), ("T-s4", 556.0, 736.0)] {
            let copy = named(&root, &format!("{cell} sortie"));
            assert_eq!(times(&log, id(copy, "Zone IN")), [ticks(seen_s)]);
            assert_eq!(times(&log, id(copy, "Zone Out")), [ticks(gone_s)]);
        }
        for cell in ["T-s1", "T-s2"] {
            let copy = named(&root, &format!("{cell} sortie"));
            assert!(times(&log, id(copy, "Zone IN")).is_empty());
            assert!(times(&log, id(copy, "Zone Out")).is_empty());
        }
    }
}

#[test]
fn flight2_sorties_use_the_shell_lifecycle() {
    let root = generate_probe_2();
    assert_eq!(root.count_block_type("Plane"), 9);
    assert_eq!(root.count_block_type("MCU_Spawner"), 0);
    assert_eq!(root.count_block_type("MCU_CMD_Damage"), 0);
    for (cell, prefix, start_s, attack_s, final_s) in [
        ("T-s1", "SN01", 60.0, 600.0, 2100.0),
        ("T-s2", "SN02", 90.0, 1080.0, 2580.0),
    ] {
        let copy = named(&root, &format!("{cell} sortie"));
        let entities = block_ids(copy, "MCU_TR_Entity");
        assert_eq!(entities.len(), 4);
        let losses = id(copy, &format!("{prefix} LOSSES"));
        assert_counter(copy, &format!("{prefix} LOSSES"), 4, 0);
        for entity in &entities {
            assert_eq!(probe_events(by_id(copy, *entity)), [(13, losses)]);
        }
        for plane in block_ids(copy, "Plane") {
            assert_eq!(by_id(copy, plane).property("AiRTBDecision"), Some("0"));
        }
        let start = id(copy, &format!("{prefix} START"));
        assert_at(&root, &format!("{cell} ARM"), start_s, &[start]);
        assert_eq!(number(named(copy, &format!("{prefix} ON STATION")), "Time"), attack_s);
        assert_eq!(number(named(copy, &format!("{prefix} HARD STOP")), "Time"), final_s);
        let delete = named(copy, &format!("{prefix} DELETE"));
        assert_eq!(delete.objects, entities);
        assert_eq!(copy.count_block_type("MCU_Delete"), 1);
        assert!(copy.find_by_name("Trigger Delete").is_none());
        assert_eq!(number(named(copy, &format!("{prefix} DELETE DELAY")), "Time"), 0.5);
        assert!(named(copy, &format!("{prefix} DELETE DELAY")).targets.contains(&id(copy, "BUD N MARK")));
        assert!(named(copy, &format!("{prefix} DELETE DELAY")).targets.contains(&id(copy, "BUD N CHECK")));
        for suffix in ["START", "DONE HUB", "EXTEND"] {
            assert_observation(&root, id(copy, &format!("{prefix} {suffix}")), &format!("{cell} {suffix}"));
        }
        assert_observation(&root, id(copy, &format!("{prefix} DELETE DELAY")), &format!("{cell} DELETE"));
        assert_switch(&root, &format!("{cell} AREA HOT ON"), "MCU_Activate", 600.0, &[id(copy, &format!("{prefix} HOT"))]);
        assert_switch(&root, &format!("{cell} AREA COLD OFF"), "MCU_Deactivate", 600.0, &[id(copy, &format!("{prefix} COLD"))]);
        let close = named(&root, &format!("{cell} CLOSE"));
        for (seat, entity) in entities.iter().enumerate() {
            let cleanup = named(&root, &format!("{cell} aircraft {} CLOSE DELETE", seat + 1));
            assert_eq!(cleanup.objects, [*entity]);
            assert!(close.targets.contains(&cleanup.index.unwrap()));
        }
    }
}

#[test]
fn flight2_budget_requests_follow_the_plan() {
    use super::shell::test_support::{ticks, times};
    use super::testkit::{SameTickDelivery, WalkerConfig, WalkerInput, walk};
    let root = generate_probe_2();
    for (cell, requests, deletions) in [
        ("T-k", vec![180.0, 182.0, 184.0, 186.0, 212.0, 214.0, 216.0], vec![(210.0, vec![1, 2])]),
        ("T-k1", vec![240.0, 240.0, 245.0, 262.0, 272.0], vec![(260.0, vec![1]), (270.0, vec![2, 3, 4])]),
    ] {
        for (seat, time) in requests.into_iter().enumerate() {
            let timer = named(&root, &format!("{cell} REQUEST {} AT", seat + 1));
            assert_eq!(number(timer, "Time"), time);
            assert_eq!(timer.targets[0], id(&root, &format!("{cell} SN{:02} REQUEST", seat + 1)));
        }
        for (step, (time, copies)) in deletions.into_iter().enumerate() {
            assert_at(&root, &format!("{cell} DELETIONS {} AT", step + 1), time,
                &copies.into_iter().map(|copy| id(&root, &format!("{cell} SN{copy:02} DELETE TIMER"))).collect::<Vec<_>>());
        }
    }
    for delivery in [SameTickDelivery::DepthFirst, SameTickDelivery::Batched] {
        let log = walk(&root, WalkerConfig { same_tick_delivery: delivery, ..WalkerConfig::default() },
            &WalkerInput { until: ticks(280.0), ..WalkerInput::default() });
        for (seat, expected) in [vec![180.0], vec![182.0], vec![], vec![], vec![212.0], vec![214.0], vec![]].into_iter().enumerate() {
            assert_eq!(times(&log, id(&root, &format!("T-k SN{:02} START", seat + 1))),
                expected.into_iter().map(ticks).collect::<Vec<_>>());
        }
        assert_eq!(times(&log, id(&root, "T-k1 SN01 START")), [ticks(240.0)]);
        assert_eq!(times(&log, id(&root, "T-k1 SN02 START")).len(), usize::from(delivery == SameTickDelivery::Batched));
        assert!(times(&log, id(&root, "T-k1 SN03 START")).is_empty());
        assert_eq!(times(&log, id(&root, "T-k1 SN04 START")).len(), usize::from(delivery == SameTickDelivery::DepthFirst));
        assert_eq!(times(&log, id(&root, "T-k1 SN05 START")), [ticks(272.0)]);
    }
}

#[test]
fn flight2_no_damage_loss_uses_a_once_counter() {
    use super::shell::test_support::{ticks, times};
    use super::testkit::{WalkerConfig, WalkerInput, walk};
    let p = build(Flight::Two);
    let plane = named(&p.root, "T-l1");
    assert_eq!(plane.pos_xz(), Some((80_934.0, 319_477.0)));
    assert_eq!(number(plane, "YOri"), 90.0);
    assert_eq!(number(plane, "YPos"), 150.0);
    assert_eq!(number(plane, "Fuel"), 0.0);
    assert_eq!(plane.property("StartType"), Some("0"));
    assert_eq!(plane.property("AiRTBDecision"), Some("0"));
    assert_eq!(number(named(&p.root, "T-l1 CRUISE"), "Speed"), 732.0);
    assert_eq!(named(&p.root, "T-l1 CRUISE").pos_xz(), Some((80_934.0, 329_477.0)));
    assert_counter(&p.root, "T-l1 LOSSES", 1, 0);
    let entity = named(&p.root, "T-l1 entity");
    assert!(entity.targets.is_empty());
    let events = probe_events(entity);
    assert_eq!(events.iter().map(|event| event.0).collect::<Vec<_>>(), [13, 2, 0, 5]);
    let losses = id(&p.root, "T-l1 LOSSES");
    assert_eq!(events[0].1, losses);
    assert_eq!(named(&p.root, "T-l1 CLOSE DELETE").objects, [entity.index.unwrap()]);
    assert!(p.root.find_by_name("T-l2").is_none());
    assert_eq!(p.root.count_block_type("MCU_CMD_Damage"), 0);
    let log = walk(&p.root, WalkerConfig::default(), &WalkerInput {
        until: ticks(61.0), kills: vec![(ticks(60.75), entity.index.unwrap().into())], ..WalkerInput::default()
    });
    assert_eq!(times(&log, losses), [ticks(60.75)]);
    let (traced, map) = p.traced(TraceCarrier::default());
    assert!(map.entries.iter().any(|entry| entry.source_index == losses));
    super::testkit::assert_links_resolve(&traced);
}

#[test]
fn flight2_loss_starts_over_land_away_from_airfields() {
    let root = generate_probe_2();
    let plane = named(&root, "T-l1");
    let start = plane.pos_xz().unwrap();
    let exit = named(&root, "T-l1 CRUISE").pos_xz().unwrap();
    let terrain = crate::watermap::TerrainMap::builtin().unwrap();
    assert!(terrain.is_land_xz(start.0, start.1));
    assert!(terrain.is_land_xz(exit.0, exit.1));
    let catalog = crate::mapload::load_catalog().unwrap();
    let mut airfields: Vec<_> = catalog.airfields()
        .map(|field| (field.name.clone(), (field.x, field.z))).collect();
    assert!(!airfields.is_empty());
    // The K-49 Airfield block is slightly nearer than its FullScene marker.
    let k49 = crate::parser::parse_il2_document(include_str!("../../TemplateExamples/K49 DPRK_AB.Group")).unwrap();
    k49.for_each(&mut |node| {
        if node.block_type == "Airfield" {
            airfields.push((node.name().unwrap().to_owned(), node.pos_xz().unwrap()));
        }
    });
    airfields.push(("K14 probe origin".into(), (105_934.0, 269_477.0)));
    let distance = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).hypot(a.1 - b.1);
    let leg = (exit.0 - start.0, exit.1 - start.1);
    for (name, field) in &airfields {
        let along = (((field.0 - start.0) * leg.0 + (field.1 - start.1) * leg.1)
            / (leg.0 * leg.0 + leg.1 * leg.1)).clamp(0.0, 1.0);
        let nearest = (start.0 + along * leg.0, start.1 + along * leg.1);
        assert!(distance(nearest, *field) >= 15_000.0, "T-l1 cruise leg approaches {name}");
    }
    let nearest_start = airfields.iter().map(|(_, field)| distance(start, *field)).fold(f64::INFINITY, f64::min);
    let nearest_exit = airfields.iter().map(|(_, field)| distance(exit, *field)).fold(f64::INFINITY, f64::min);
    assert!(nearest_exit > nearest_start, "T-l1 must cruise away from nearby airfields");
    assert!(distance(exit, (105_934.0, 269_477.0)) > distance(start, (105_934.0, 269_477.0)));
    for other in block_ids(&root, "Plane").into_iter().filter(|index| Some(*index) != plane.index) {
        assert!(distance(start, by_id(&root, other).pos_xz().unwrap()) >= 15_000.0,
            "T-l1 starts near flight 2 aircraft {other}");
    }
}

#[test]
fn flight2_load_has_24_idle_copies_per_side() {
    use super::shell::test_support::ticks;
    use super::testkit::{WalkerConfig, WalkerInput, walk};
    let p = build(Flight::Load);
    assert_eq!(p.root.name(), Some("P14_Load_Probe"));
    assert_eq!(Flight::Load.stem(), "P14_Load_Probe");
    assert_eq!(p.root.count_block_type("Plane"), 240);
    for side in ["N", "D"] {
        for copy in 1..=24 {
            assert!(p.root.find_by_name(&format!("S{side}{copy:02} START")).is_some());
        }
        assert!(p.root.find_by_name(&format!("S{side}25 START")).is_none());
    }
    let log = walk(&p.root, WalkerConfig::default(), &WalkerInput { until: ticks(10.0), ..WalkerInput::default() });
    assert!(!log.pulses.iter().any(|pulse| pulse.name.ends_with(" START")));
    let (traced, _) = p.traced(TraceCarrier::default());
    super::testkit::assert_links_resolve(&traced);
    let mut indexes = Vec::new();
    traced.collect_indexes(&mut indexes);
    assert_eq!(indexes.iter().collect::<BTreeSet<_>>().len(), indexes.len());
}

fn number(node: &Il2Entity, property: &str) -> f64 {
    node.property(property)
        .unwrap_or_else(|| panic!("{:?} lacks {property}", node.name()))
        .parse()
        .unwrap()
}

fn assert_near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
}

fn assert_at(root: &Il2Entity, name: &str, time_s: f64, targets: &[i32]) {
    let node = named(root, name);
    assert_eq!(node.block_type, "MCU_Timer", "{name}");
    assert_near(number(node, "Time"), time_s);
    assert_eq!(node.property("Random"), Some("100"));
    assert!(
        named(root, "Translator Mission Begin")
            .targets
            .contains(&node.index.unwrap()),
        "{name} is not directly clocked"
    );
    assert_eq!(node.targets, targets, "{name} targets");
}

fn assert_switch(root: &Il2Entity, name: &str, block: &str, at: f64, targets: &[i32]) {
    let node = named(root, name);
    assert_eq!(node.block_type, block, "{name}");
    assert_eq!(node.targets, targets, "{name}");
    assert!(node.objects.is_empty());
    assert_at(root, &format!("{name} AT"), at, &[node.index.unwrap()]);
}

fn assert_counter(root: &Il2Entity, name: &str, count: i32, drop: i32) {
    let node = named(root, name);
    assert_eq!(node.block_type, "MCU_Counter");
    assert_eq!(number(node, "Counter"), f64::from(count), "{name}");
    assert_eq!(number(node, "Dropcount"), f64::from(drop), "{name}");
}

fn assert_observation(root: &Il2Entity, source: i32, text: &str) {
    let subtitle = named(root, text);
    assert_eq!(subtitle.block_type, "MCU_TR_Subtitle");
    assert!(
        by_id(root, source)
            .targets
            .contains(&subtitle.index.unwrap()),
        "{text} has no per-event link"
    );
}

fn assert_readout(root: &Il2Entity, source: i32, text: &str, threshold: i32) {
    let name = format!("READ {text}");
    assert_counter(root, &name, threshold, 0);
    assert!(
        by_id(root, source).targets.contains(&id(root, &name)),
        "{text} input"
    );
    assert_eq!(named(root, &name).targets, [id(root, text)]);
}

fn assert_zone(
    root: &Il2Entity,
    name: &str,
    radius: f64,
    closer: bool,
    coalition: &str,
    km: (f64, f64),
) {
    let node = named(root, name);
    assert_eq!(node.block_type, "MCU_CheckZone");
    assert_eq!(number(node, "Zone"), radius);
    assert_eq!(number(node, "Closer"), f64::from(closer));
    assert_eq!(node.property("Cylinder"), Some("1"));
    assert_eq!(node.property("PlaneCoalitions"), Some(coalition));
    let (x, z) = node.pos_xz().unwrap();
    let expected = world(km);
    assert_near(x, expected.0);
    assert_near(z, expected.1);
}

fn assert_loop(
    root: &Il2Entity,
    name: &str,
    start: f64,
    period: f64,
    count: usize,
    outputs: &[i32],
) {
    let tick = named(root, &format!("{name} TICK"));
    let delay = named(root, &format!("{name} PERIOD"));
    assert_eq!(number(tick, "Time"), 0.0);
    assert_eq!(tick.property("Random"), Some("100"));
    assert_near(number(delay, "Time"), period);
    assert_eq!(delay.targets, [tick.index.unwrap()]);
    let mut expected = outputs.to_vec();
    expected.push(delay.index.unwrap());
    assert_eq!(tick.targets, expected);
    assert_at(
        root,
        &format!("{name} START"),
        start,
        &[tick.index.unwrap()],
    );
    let stop_s = number(named(root, &format!("{name} STOP AT")), "Time");
    assert_switch(
        root,
        &format!("{name} STOP"),
        "MCU_Deactivate",
        stop_s,
        &[tick.index.unwrap()],
    );
    let scheduled: Vec<_> = (0..100)
        .map(|n| start + f64::from(n) * period)
        .take_while(|&t| t < stop_s)
        .collect();
    assert_eq!(scheduled.len(), count, "{name} pulse count");
    assert_near(
        *scheduled.last().unwrap(),
        start + (count - 1) as f64 * period,
    );
    assert!(stop_s < start + count as f64 * period);
}

fn assert_pulses(root: &Il2Entity, name: &str, times: &[f64], target: i32) {
    for (n, &time) in times.iter().enumerate() {
        assert_at(root, &format!("{name} PULSE {}", n + 1), time, &[target]);
    }
}

fn assert_fan(root: &Il2Entity, source_name: &str, time: f64, count: usize, target: i32) {
    let source = named(root, source_name);
    assert_at(root, source_name, time, &source.targets);
    assert_eq!(source.targets.len(), count);
    let distinct: HashSet<_> = source.targets.iter().collect();
    assert_eq!(distinct.len(), count);
    for &input in &source.targets {
        let input = by_id(root, input);
        assert_eq!(input.block_type, "MCU_Timer");
        assert_eq!(number(input, "Time"), 0.0);
        assert_eq!(input.property("Random"), Some("100"));
        assert!(input.targets.contains(&target));
    }
}

fn assert_flight(flight: &Il2Entity, count: usize, start: (f64, f64), exit: (f64, f64)) {
    assert_flight_country(flight, count, start, exit, COUNTRY_USA);
}

fn assert_flight_country(
    flight: &Il2Entity,
    count: usize,
    start: (f64, f64),
    exit: (f64, f64),
    country: i32,
) {
    let original = parse_group_file(HVAR).unwrap();
    let names: Vec<_> = block_ids(&original, "Plane")
        .into_iter()
        .take(count)
        .map(|plane| by_id(&original, plane).name().unwrap())
        .collect();
    assert_named_flight(flight, &original, &names, start, exit, country);
}

fn assert_named_flight(
    flight: &Il2Entity,
    original: &Il2Entity,
    names: &[&str],
    start: (f64, f64),
    exit: (f64, f64),
    country: i32,
) {
    let country = country.to_string();
    let planes = block_ids(flight, "Plane");
    let entities = block_ids(flight, "MCU_TR_Entity");
    assert_eq!(planes.len(), names.len());
    assert_eq!(entities.len(), names.len());
    let original_planes = block_ids(original, "Plane");
    for (i, &plane_id) in planes.iter().enumerate() {
        let plane = by_id(flight, plane_id);
        let source = by_id(original, original_planes[i]);
        assert_eq!(plane.name(), Some(names[i]), "plane names in file order");
        let source_pos = source.pos_xz().unwrap();
        let original_lead = by_id(original, original_planes[0]).pos_xz().unwrap();
        let position = plane.pos_xz().unwrap();
        assert_near(position.0, start.0 + source_pos.0 - original_lead.0);
        assert_near(position.1, start.1 + source_pos.1 - original_lead.1);
        assert_eq!(plane.property("Country"), Some(country.as_str()));
        assert_eq!(plane.property("Script"), source.property("Script"));
        assert_eq!(number(plane, "YPos"), 3050.0);
        assert_eq!(number(plane, "YOri"), 0.0);
        assert_eq!(number(plane, "LinkTrId"), f64::from(entities[i]));
        let entity = by_id(flight, entities[i]);
        assert_eq!(number(entity, "MisObjID"), f64::from(plane_id));
        assert_eq!(entity.property("Enabled"), Some("0"));
        assert_eq!(
            entity.targets,
            if i == 0 { vec![] } else { vec![entities[0]] }
        );
    }
    assert_eq!(named(flight, "WP 1").pos_xz(), Some(exit));
    assert_eq!(named(flight, "AttackArea").pos_xz(), Some(exit));
    assert_eq!(named(flight, "WP 1").property("Speed"), Some("660"));
    assert_eq!(named(flight, "AttackArea").property("Time"), Some("600"));
    for removed in [
        "ENABLE / PULSE IN",
        "Zone IN",
        "Self Deactivate",
        "Zone In ReActivate",
        "COOLDOWN",
        "Mission Complete 1",
    ] {
        assert!(
            flight.find_by_name(removed).is_none(),
            "uncut HVAR MCU {removed}"
        );
    }
    let begin = named(flight, "Translator Mission Begin");
    assert!(begin.targets.is_empty());
    assert_eq!(begin.property("Enabled"), Some("0"));
    flight.for_each(&mut |node| {
        if let Some(value) = node.property("Country") {
            assert_eq!(value, country.as_str());
        }
    });
}

#[test]
fn probe_one_plane_flight_has_no_removed_aircraft_links() {
    let mut next_id = 10_000;
    let flight = probe_flight(HVAR, 1, world((120.0, 19.0)), world((130.0, 19.0)), &mut next_id);
    assert_flight(&flight, 1, world((120.0, 19.0)), world((130.0, 19.0)));
    let plane = block_ids(&flight, "Plane")[0];
    let entity = block_ids(&flight, "MCU_TR_Entity")[0];
    assert_eq!(by_id(&flight, plane).property("NumberInFormation"), Some("0"));
    assert!(by_id(&flight, entity).targets.is_empty());
    assert!(by_id(&flight, entity).objects.is_empty());
    assert_eq!(named(&flight, "Formation").property("FlightSize"), Some("1"));
    for name in ["Activate Units", "Deactivate Units", "Force Complete - High", "Formation", "AttackArea", "WP 1"] {
        assert_eq!(named(&flight, name).objects, [entity], "{name}");
    }
    assert_eq!(named(&flight, "Trigger Delete").objects, [plane]);
    let parsed = parse_group_file(&serialize_probe(&flight)).unwrap();
    assert_eq!(parsed, flight);
    super::testkit::assert_links_resolve(&parsed);
    let mut indexes = Vec::new();
    parsed.collect_indexes(&mut indexes);
    let unique: HashSet<_> = indexes.iter().copied().collect();
    assert_eq!(unique.len(), indexes.len());
    assert!(unique.iter().all(|&index| index >= 10_000));
    parsed.for_each(&mut |node| {
        for key in ["LinkTrId", "MisObjID", "TarId", "CmdId"] {
            if let Some(value) = node.property(key) {
                let target: i32 = value.parse().unwrap();
                assert!(unique.contains(&target), "{:?}: dangling {key} {target}", node.name());
            }
        }
    });
}

#[test]
fn probe_files_parse_and_links_resolve() {
    use super::testkit::assert_links_resolve;
    for (flight, root) in [
        (Flight::Zero, generate_probe_0()),
        (Flight::OneA, generate_probe_1a()),
        (Flight::OneN, generate_probe_1n()),
        (Flight::OneD, generate_probe_1d()),
        (Flight::OneG, generate_probe_1g()),
        (Flight::OneH, generate_probe_1h()),
        (Flight::OneJ, generate_probe_1j()),
        (Flight::OneK, generate_probe_1k()),
        (Flight::OneL, generate_probe_1l()),
        (Flight::OneM, generate_probe_1m()),
        (Flight::OneP, generate_probe_1p()),
        (Flight::Two, generate_probe_2()),
        (Flight::TwoB, generate_probe_2b()),
        (Flight::TwoC, generate_probe_2c()),
        (Flight::TwoD, generate_probe_2d()),
        (Flight::TwoE, generate_probe_2e()),
        (Flight::TwoF, generate_probe_2f()),
        (Flight::Load, build(Flight::Load).root),
    ] {
        let (traced, _) = build(flight).traced(TraceCarrier::default());
        for root in [root, traced] {
            let parsed = parse_group_file(&serialize_group(&root)).expect("probe round trip");
            assert_eq!(root, parsed);
            assert_links_resolve(&parsed);
            let mut indexes = Vec::new();
            parsed.collect_indexes(&mut indexes);
            let unique: HashSet<_> = indexes.iter().copied().collect();
            assert_eq!(unique.len(), indexes.len(), "duplicate probe Index");
            // The shared helper covers Targets/Objects. Also validate the
            // entity and event pointers introduced by flight trimming/T-g.
            parsed.for_each(&mut |node| {
                for key in ["LinkTrId", "MisObjID", "TarId", "CmdId"] {
                    if let Some(value) = node.property(key) {
                        let target: i32 = value.parse().unwrap();
                        assert!(
                            unique.contains(&target),
                            "{:?}: dangling {key} {target}",
                            node.name()
                        );
                    }
                }
            });
        }
    }
    // All instrumented sources correspond to observations, including entity
    // event types and the absolute cue times; hand-built T-t sources stay out.
    for flight in Flight::ALL {
        let p = build(flight);
        let subtitles: HashSet<_> = block_ids(&p.root, "MCU_TR_Subtitle").into_iter().collect();
        p.root.for_each(&mut |node| {
            if node.targets.iter().any(|target| subtitles.contains(target)) {
                let index = node.index.unwrap();
                assert!(
                    p.hand_sources.contains(&index)
                        || p.sources
                            .iter()
                            .any(|s| s.index == index && s.event_type.is_none()),
                    "missing subtitle trace source {:?}",
                    node.name()
                );
            }
            if node.block_type == "MCU_TR_Entity" {
                node.for_each(&mut |event| {
                    if event.block_type == "OnEvent" {
                        let target = number(event, "TarId") as i32;
                        if subtitles.contains(&target) {
                            let event_type = number(event, "Type") as i32;
                            assert!(p.sources.iter().any(|s| s.index == node.index.unwrap()
                                && s.event_type == Some(event_type)));
                        }
                    }
                });
            } else if node.block_type == "MCU_TR_ComplexTrigger" {
                assert!(node.targets.is_empty());
                node.for_each(&mut |event| {
                    if event.block_type == "OnEvent" {
                        let target = number(event, "TarId") as i32;
                        let relay = by_id(&p.root, target);
                        assert_eq!(relay.block_type, "MCU_Timer");
                        assert_eq!(number(relay, "Time"), 0.0);
                        assert!(
                            p.sources
                                .iter()
                                .any(|s| s.index == target && s.event_type.is_none())
                        );
                    }
                });
            }
        });
        for cue in CUES.iter().filter(|c| c.flight == flight) {
            let subtitle = id(&p.root, cue.text);
            assert!(p.sources.iter().any(|s| s.expected_s == Some(cue.time_s)
                && by_id(&p.root, s.index).targets.contains(&subtitle)));
        }
    }
}

/// IL-2's mission loader (editor and MissionResaver) rejects the whole file
/// when a quoted value holds `;`, `=`, `{`, `}`, `'` or a non-ASCII character
/// such as `≥`; the editor shows an empty error box. Each was checked with
/// MissionResaver on 2026-09-28 (`:`, `,`, `+`, `-`, brackets pass).
#[test]
fn probe_strings_are_loader_safe() {
    for flight in Flight::ALL {
        let probe = build(flight);
        let plain = probe.root.clone();
        let (traced, _) = probe.traced(TraceCarrier::default());
        for root in [plain, traced] {
            let text = serialize_probe(&root);
            for line in text.lines() {
                let Some((_, quoted)) = line.split_once(" = \"") else {
                    continue;
                };
                // The text between the quotes; `";` closes the line.
                let value = quoted.rsplit_once('"').map_or(quoted, |(v, _)| v);
                assert!(
                    value.is_ascii() && !value.contains(['=', ';', '{', '}', '\'']),
                    "{}: loader-unsafe string: {}",
                    flight.stem(),
                    line.trim()
                );
            }
        }
    }
}

#[test]
fn probe_blocks_match_shipped_shapes() {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn group_files(dir: &Path, paths: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                group_files(&path, paths);
            } else if path.extension().is_some_and(|ext| ext == "Group") {
                paths.push(path);
            }
        }
    }

    fn keys(node: &Il2Entity) -> Vec<String> {
        node.properties.iter().map(|(key, _)| key.clone()).collect()
    }

    let references: Vec<_> = [
        "MCU_TR_ComplexTrigger",
        "MCU_CMD_Damage",
        "MCU_CMD_AttackTarget",
        "MCU_CMD_Behaviour",
    ]
    .into_iter()
    .map(|block| (block, reference_block(block)))
    .collect();
    let generated_unit = builtin_plane_catalog().into_iter()
        .find(|unit| crate::model_spec::script_id(&unit.script) == "f80c10")
        .expect("flight 1L shape reference: missing F-80C");
    let generated_reference = generate_template(&TemplateOptions {
        seats: vec![TemplateSeat::new(generated_unit)],
        ..TemplateOptions::default()
    }).expect("flight 1L shape reference: generate_template failed");
    let generated_plane = by_id(&generated_reference, block_ids(&generated_reference, "Plane")[0]);

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    group_files(&manifest.join("TemplateExamples"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no shipped .Group files found");
    let mut shipped = BTreeMap::<_, BTreeMap<_, _>>::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).unwrap();
        let file = path.strip_prefix(manifest).unwrap().display().to_string();
        let mut rest = text.trim_start_matches('\u{feff}').trim_start();
        // Parse actual top-level blocks, without parse_il2_document's synthetic Group.
        while !rest.is_empty() {
            let (tail, root) =
                crate::parser::parse_entity(rest).unwrap_or_else(|error| panic!("{file}: {error}"));
            root.for_each(&mut |node| {
                shipped
                    .entry(node.block_type.clone())
                    .or_default()
                    .entry(keys(node))
                    .or_insert_with(|| file.clone());
            });
            rest = tail.trim_start();
        }
    }

    let mut matched = BTreeMap::<_, BTreeSet<_>>::new();
    let mut missing = BTreeSet::new();
    let mut mismatches = BTreeMap::new();
    for flight in Flight::ALL {
        let probe = build(flight);
        let plain = probe.root.clone();
        let (traced, _) = probe.traced(TraceCarrier::default());
        for (suffix, root) in [("", plain), ("_traced", traced)] {
            let file = format!("{}{suffix}", flight.stem());
            // Check the serialized Group and UTF-16 .eng that the writers emit.
            let english = crate::locale::encode_locale_utf16le(&crate::locale::serialize_locale(
                &p14_probe_locale(&root),
            ));
            let root = parse_group_file(&serialize_probe(&root)).unwrap();
            super::testkit::assert_links_resolve(&root);
            let english = crate::locale::decode_locale_bytes(&english).unwrap();
            let locale = crate::locale::parse_locale(&english);
            let table_ids: BTreeSet<i32> = english
                .lines()
                .map(|line| line.split_once(':').unwrap().0.parse().unwrap())
                .collect();
            let mut used_ids = BTreeSet::new();
            // Flight 0's T-f contains no Plane blocks.
            let generated_planes: Vec<_> = ["T-w", "T-x", "T-f"].into_iter()
                .filter_map(|name| root.find_by_name(name))
                .flat_map(|cell| block_ids(cell, "Plane")).collect();
            root.for_each(&mut |node| {
                let ordered_keys = keys(node);
                if let Some((_, reference)) = references
                    .iter()
                    .find(|(block, _)| *block == node.block_type)
                {
                    let mut expected = keys(reference);
                    if node.block_type == "MCU_TR_ComplexTrigger" {
                        expected.retain(|key| {
                            !matches!(key.as_str(), "ObjectScript" | "ObjectName")
                                && (key != "Country" || node.property("Country").is_some())
                        });
                        assert_eq!(
                            node.properties
                                .iter()
                                .filter(|(key, _)| key == "Enabled")
                                .map(|(_, value)| value.as_str())
                                .collect::<Vec<_>>(),
                            ["1", "1"]
                        );
                        assert!(node.targets.is_empty());
                        assert_eq!(node.property("Targets"), Some("[]"));
                        assert_eq!(node.children.len(), 1);
                        assert_eq!(node.children[0].block_type, "OnEvents");
                    }
                    assert_eq!(ordered_keys, expected, "{file}: {:?}", node.name());
                } else if node.index.is_some_and(|index| generated_planes.contains(&index)) {
                    // These Plane blocks are generate_template's output, kept in
                    // the app's key order because flight 1L tests what the app writes.
                    // probe_blocks_match_shipped_shapes still checks that order
                    // and the shipped keys (moving Name only); probe_cells_match_the_table
                    // checks their properties and links via assert_app_logic_probe.
                    assert_eq!(ordered_keys, keys(generated_plane), "{file}: {:?}", node.name());
                    let mut shipped_order = ordered_keys.clone();
                    assert_eq!(&shipped_order[..3], ["Index", "LinkTrId", "Name"]);
                    let name_key = shipped_order.remove(2);
                    shipped_order.insert(0, name_key);
                    assert!(shipped["Plane"].contains_key(&shipped_order),
                        "{file}: generated Plane differs from shipped shape beyond Name order");
                } else if let Some(shapes) = shipped.get(&node.block_type) {
                    if let Some(example) = shapes.get(&ordered_keys) {
                        matched
                            .entry(node.block_type.clone())
                            .or_default()
                            .insert(example.clone());
                    } else {
                        mismatches
                            .entry((node.block_type.clone(), ordered_keys))
                            .or_insert_with(|| {
                                format!("{file}.Group: Index {:?}, {:?}", node.index, node.name())
                            });
                    }
                } else {
                    missing.insert(node.block_type.clone());
                }
                for (key, value) in &node.properties {
                    if matches!(key.as_str(), "LCName" | "LCDesc" | "LCText") {
                        let id: i32 = value.parse().unwrap();
                        assert!((0..10_000).contains(&id), "{file}: {key} = {id}");
                        if id != 0 {
                            assert!(locale.get(id).is_some(), "{file}.eng lacks {key} = {id}");
                            used_ids.insert(id);
                        }
                    }
                }
            });
            assert_eq!(
                table_ids, used_ids,
                "{file}.eng must contain exactly the used ids"
            );
            assert_eq!(
                used_ids,
                (P14_PROBE_LC_START..P14_PROBE_LC_START + table_ids.len() as i32).collect(),
                "{file}: locale ids must be contiguous from {P14_PROBE_LC_START}"
            );
            println!(
                "{file}: locale ids {}..={} ({} strings)",
                used_ids.first().unwrap(),
                used_ids.last().unwrap(),
                used_ids.len()
            );
        }
    }
    for (block, files) in matched {
        println!(
            "{block}: {}",
            files.into_iter().collect::<Vec<_>>().join(", ")
        );
    }
    println!("Block types with no shipped example: {missing:?}");
    let failures = mismatches
        .iter()
        .map(|((block, ordered_keys), file)| {
            format!(
                "{file}\n{block}: {ordered_keys:?}\nShipped shapes: {:?}",
                shipped[block].keys().collect::<Vec<_>>()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        mismatches.is_empty(),
        "unmatched probe block shapes:\n{failures}"
    );
}

#[test]
fn probe_subtitles_are_unique() {
    for flight in Flight::ALL {
        let root = build(flight).root;
        let locale = p14_probe_locale(&root);
        let mut seen = HashSet::new();
        root.for_each(&mut |node| {
            if node.block_type == "MCU_TR_Subtitle" {
                assert_eq!(node.property("Enabled"), Some("1"));
                assert_eq!(node.property("Coalitions"), Some("[0,1,2]"));
                let infos: Vec<_> = node
                    .children
                    .iter()
                    .filter(|c| c.block_type == "SubtitleInfo")
                    .collect();
                assert_eq!(infos.len(), 1);
                let info = infos[0];
                let lc = number(info, "LCText") as i32;
                assert!(seen.insert(lc), "duplicate subtitle LCText {lc}");
                assert_eq!(locale.get(lc), node.name());
                let cue = CUES
                    .iter()
                    .any(|cue| cue.flight == flight && Some(cue.text) == node.name());
                assert_eq!(number(info, "Duration"), if cue { 20.0 } else { 1.0 });
            } else if node.block_type == "MCU_Icon" {
                assert_eq!(locale.get(number(node, "LCName") as i32), node.name());
                assert_eq!(locale.get(number(node, "LCDesc") as i32), Some(""));
            }
        });
        assert!(!seen.is_empty());
        assert!(!locale.contains_text("T-t T4: named objective"));
        assert_eq!(
            locale.contains_text(
                "T-t start: note any objective message or map marker in the next 60 s"
            ),
            flight == Flight::Zero
        );
        for sub in 1..=5 {
            assert!(!locale.contains_text(&format!("T-o O{sub} fired")));
        }
        let bytes = crate::locale::encode_locale_utf16le(&crate::locale::serialize_locale(&locale));
        assert_eq!(&bytes[..2], &[0xff, 0xfe]);
        assert_eq!(
            crate::locale::parse_locale(&crate::locale::decode_locale_bytes(&bytes).unwrap()),
            locale
        );
    }
}

#[test]
fn losses_impl_defaults_to_type13() {
    assert_eq!(super::LossesImpl::default(), super::LossesImpl::Type13);
}

fn assert_watched_aircraft(
    root: &Il2Entity,
    flight: &Il2Entity,
    label: &str,
    event_type: i32,
) -> i32 {
    let entity = named(flight, &format!("{label} entity"));
    let entity_id = entity.index.unwrap();
    let plane = by_id(flight, number(entity, "MisObjID") as i32);
    assert_eq!(plane.block_type, "Plane");
    assert_eq!(plane.name(), Some(label));
    assert_eq!(number(plane, "LinkTrId"), f64::from(entity_id));
    assert_eq!(entity.children.len(), 1);
    let events = &entity.children[0];
    assert_eq!(events.block_type, "OnEvents");
    assert_eq!(events.children.len(), 1);
    let event = &events.children[0];
    assert_eq!(event.block_type, "OnEvent");
    assert_eq!(number(event, "Type"), f64::from(event_type));
    let subtitle = named(root, &format!("{label} ev{event_type}"));
    assert_eq!(subtitle.block_type, "MCU_TR_Subtitle");
    assert_eq!(number(event, "TarId"), f64::from(subtitle.index.unwrap()));

    let delete = named(root, &format!("{label} CLOSE DELETE"));
    assert_eq!(delete.block_type, "MCU_Delete");
    assert_eq!(delete.objects, [entity_id]);
    assert!(delete.targets.is_empty());
    delete.index.unwrap()
}

fn assert_damage_commands(root: &Il2Entity, rows: &[(&[&str], &str, f64)]) -> BTreeSet<i32> {
    let mut damage_commands = BTreeSet::new();
    for &(labels, suffix, time_s) in rows {
        for label in labels {
            let name = format!("{label} {suffix}");
            let damage = named(root, &name);
            let damage_id = damage.index.unwrap();
            assert_eq!(damage.block_type, "MCU_CMD_Damage");
            assert_eq!(damage.property("Damage"), Some("2"));
            assert_eq!(damage.property("Type"), Some("1"));
            assert_eq!(damage.objects, [id(root, &format!("{label} entity"))]);
            assert!(damage.targets.is_empty());
            let timer_name = format!("{name} AT");
            assert_at(root, &timer_name, time_s, &[damage_id]);
            let mut inputs = Vec::new();
            root.for_each(&mut |node| {
                if node.targets.contains(&damage_id) {
                    inputs.push(node.index.unwrap());
                }
            });
            assert_eq!(inputs, [id(root, &timer_name)], "{name} fires only at {time_s}");
            assert!(damage_commands.insert(damage_id));
        }
    }
    assert_eq!(
        block_ids(root, "MCU_CMD_Damage").into_iter().collect::<BTreeSet<_>>(),
        damage_commands
    );
    damage_commands
}

fn assert_delete_probe(root: &Il2Entity) {
    use super::testkit::{TICKS_PER_SECOND, WalkerConfig, WalkerInput, walk};

    let original = parse_group_file(HVAR).unwrap();
    assert_eq!(root.count_block_type("Plane"), 20);
    assert_eq!(root.count_block_type("MCU_TR_Entity"), 20);
    assert_eq!(root.count_block_type("OnEvent"), 20);
    assert_eq!(root.count_block_type("MCU_Delete"), 37); // 10 template + 20 close + 7 removals.
    assert_eq!(root.count_block_type("MCU_Icon"), 10);
    let mut close_targets = vec![id(root, "T-d CLOSE OFF")];
    for (name, labels, start, centre) in [
        ("D0", ["D0a", "D0b"], (120.0, 20.0), (130.0, 20.0)),
        ("D1", ["D1a", "D1b"], (120.0, 28.0), (130.0, 28.0)),
        ("D2", ["D2a", "D2b"], (120.0, 36.0), (130.0, 36.0)),
        ("D3", ["D3a", "D3b"], (120.0, 44.0), (130.0, 44.0)),
        ("D4", ["D4a", "D4b"], (120.0, 52.0), (130.0, 52.0)),
        ("D5", ["D5a", "D5b"], (120.0, 60.0), (130.0, 60.0)),
        ("D6", ["D6a", "D6b"], (120.0, 68.0), (130.0, 68.0)),
        ("D7", ["D7a", "D7b"], (120.0, 76.0), (130.0, 76.0)),
        ("D8", ["D8a", "D8b"], (104.0, 20.0), (114.0, 20.0)),
        ("D9", ["D9a", "D9b"], (104.0, 28.0), (114.0, 28.0)),
    ] {
        let flight = named(root, name);
        assert_named_flight(flight, &original, &labels, world(start), world(centre), 601);
        assert_at(root, &format!("{name} START"), 60.0, &[id(flight, "MISSION BEGIN")]);
        for label in labels {
            close_targets.push(assert_watched_aircraft(root, flight, label, 13));
        }
    }
    assert_at(root, "T-d CLOSE", 480.0, &close_targets);

    // Literal aircraft lists pin both the killed seat and the untouched controls.
    let damage_commands = assert_damage_commands(root, &[
        (&["D2b", "D3a", "D4b", "D5b", "D6b"][..], "DAMAGE", 120.0),
        (
            &[
                "D1a", "D1b", "D2a", "D2b", "D3a", "D3b", "D4a", "D4b",
                "D5a", "D5b", "D6a", "D6b", "D7a", "D7b", "D8a", "D8b",
                "D9a", "D9b",
            ][..],
            "DETECTOR",
            270.0,
        ),
    ]);
    assert_eq!(damage_commands.len(), 23);
    assert_eq!(
        damage_commands.iter()
            .filter(|&&index| by_id(root, index).name().unwrap().ends_with(" DETECTOR"))
            .count(),
        18
    );
    let mut probe_deletes: BTreeSet<_> = close_targets[1..].iter().copied().collect();
    for (name, labels) in [
        ("D1", &["D1a", "D1b"][..]),
        ("D2", &["D2a", "D2b"][..]),
        ("D3", &["D3a", "D3b"][..]),
        ("D5", &["D5a"][..]),
    ] {
        let delete = named(root, &format!("{name} DELETE"));
        let delete_id = delete.index.unwrap();
        assert_eq!(delete.block_type, "MCU_Delete");
        assert_eq!(
            delete.objects,
            labels.iter().map(|label| id(root, &format!("{label} entity"))).collect::<Vec<_>>(),
            "{name}: Delete Objects order"
        );
        assert!(delete.targets.is_empty());
        assert_at(root, &format!("{name} DELETE AT"), 240.0, &[delete_id]);
        assert!(probe_deletes.insert(delete_id));
    }
    let mut d4_deletes = Vec::new();
    for label in ["D4a", "D4b"] {
        let delete = named(root, &format!("{label} DELETE"));
        assert_eq!(delete.block_type, "MCU_Delete");
        assert_eq!(delete.objects, [id(root, &format!("{label} entity"))]);
        assert!(delete.targets.is_empty());
        d4_deletes.push(delete.index.unwrap());
        assert!(probe_deletes.insert(delete.index.unwrap()));
    }
    assert_at(root, "D4 DELETE AT", 240.0, &d4_deletes);

    let d8_delete = named(root, "D8 DELETE");
    assert_eq!(d8_delete.block_type, "MCU_Delete");
    assert_eq!(d8_delete.objects, [id(root, "D8a"), id(root, "D8b")]);
    assert!(d8_delete.targets.is_empty());
    assert_at(root, "D8 DELETE AT", 240.0, &[d8_delete.index.unwrap()]);
    assert!(probe_deletes.insert(d8_delete.index.unwrap()));
    assert_eq!(
        named(root, "T-d").children.iter()
            .filter(|node| node.block_type == "MCU_Delete")
            .map(|node| node.index.unwrap())
            .collect::<BTreeSet<_>>(),
        probe_deletes,
        "only the seven removal Deletes and twenty individual close Deletes"
    );

    let d9 = named(root, "D9");
    let d9_deactivate = named(root, "D9 DEACTIVATE");
    assert_eq!(d9_deactivate.block_type, "MCU_Deactivate");
    assert_eq!(d9_deactivate.objects, [id(d9, "D9a entity"), id(d9, "D9b entity")]);
    assert!(d9_deactivate.targets.is_empty());
    assert_ne!(d9_deactivate.index, named(d9, "Deactivate Units").index);
    assert!(named(root, "T-d").children.contains(d9_deactivate));
    assert_at(root, "D9 DEACTIVATE AT", 240.0, &[d9_deactivate.index.unwrap()]);
    assert!(root.find_by_name("D9 DELETE AT").is_none());

    // Walk the scheduled pulses, including the copied template's timers: no
    // Delete of either D9 Plane or entity may fire before the individual closes.
    let d9_objects = [id(d9, "D9a"), id(d9, "D9b"), id(d9, "D9a entity"), id(d9, "D9b entity")];
    let log = walk(root, WalkerConfig::default(), &WalkerInput {
        until: 480 * TICKS_PER_SECOND,
        ..WalkerInput::default()
    });
    let d9_deletes: Vec<_> = log.pulses.iter().filter_map(|pulse| {
        let node = by_id(root, pulse.index);
        (node.block_type == "MCU_Delete" && node.objects.iter().any(|id| d9_objects.contains(id)))
            .then_some((pulse.t, pulse.index))
    }).collect();
    assert_eq!(d9_deletes, [
        (480 * TICKS_PER_SECOND, id(root, "D9a CLOSE DELETE")),
        (480 * TICKS_PER_SECOND, id(root, "D9b CLOSE DELETE")),
    ]);

    let d6 = named(root, "D6");
    let deactivate = named(d6, "Deactivate Units");
    let delay = named(d6, "DELETE DELAY");
    let delete = named(d6, "Trigger Delete");
    assert_at(
        root,
        "D6 DEACTIVATE AT",
        240.0,
        &[deactivate.index.unwrap(), delay.index.unwrap()],
    );
    assert_eq!(deactivate.block_type, "MCU_Deactivate");
    assert_eq!(deactivate.objects, [id(d6, "D6a entity"), id(d6, "D6b entity")]);
    assert!(deactivate.targets.is_empty());
    assert_eq!(delay.block_type, "MCU_Timer");
    assert_eq!(number(delay, "Time"), 0.5);
    assert_eq!(delay.property("Random"), Some("100"));
    assert_eq!(delay.targets, [delete.index.unwrap()]);
    assert_eq!(delete.block_type, "MCU_Delete");
    assert_eq!(delete.objects, [id(d6, "D6a"), id(d6, "D6b")]);
    assert!(delete.targets.is_empty());
    for name in ["D0", "D7"] {
        assert!(root.find_by_name(&format!("{name} DELETE AT")).is_none());
        assert!(root.find_by_name(&format!("{name} DEACTIVATE AT")).is_none());
    }
}

fn assert_single_delete_probe(root: &Il2Entity) {
    use super::testkit::{TICKS_PER_SECOND, WalkerConfig, WalkerInput, walk};

    let original = parse_group_file(HVAR).unwrap();
    let cell = named(root, "T-d2");
    assert_eq!(cell.children.iter().filter(|node| node.block_type == "Group").count(), 14);
    assert_eq!(root.count_block_type("Plane"), 14);
    assert_eq!(root.count_block_type("MCU_TR_Entity"), 14);
    assert_eq!(root.count_block_type("OnEvent"), 14);
    assert_eq!(root.count_block_type("MCU_Delete"), 34); // 14 template + 14 close + 6 removals.
    assert_eq!(root.count_block_type("MCU_Icon"), 7);
    let mut close_targets = vec![id(root, "T-d2 CLOSE OFF")];
    let mut expected_deletes = Vec::new();
    for (label, start, centre) in [
        ("J0x", (120.0, 19.0), (130.0, 19.0)),
        ("J0y", (120.0, 21.0), (130.0, 21.0)),
        ("J1x", (120.0, 27.0), (130.0, 27.0)),
        ("J1y", (120.0, 29.0), (130.0, 29.0)),
        ("J2x", (120.0, 35.0), (130.0, 35.0)),
        ("J2y", (120.0, 37.0), (130.0, 37.0)),
        ("J3x", (120.0, 43.0), (130.0, 43.0)),
        ("J3y", (120.0, 45.0), (130.0, 45.0)),
        ("J4x", (120.0, 51.0), (130.0, 51.0)),
        ("J4y", (120.0, 53.0), (130.0, 53.0)),
        ("J5x", (120.0, 59.0), (130.0, 59.0)),
        ("J5y", (120.0, 61.0), (130.0, 61.0)),
        ("J6x", (120.0, 67.0), (130.0, 67.0)),
        ("J6y", (120.0, 69.0), (130.0, 69.0)),
    ] {
        let flight = named(root, label);
        assert_eq!(flight.block_type, "Group");
        assert!(cell.children.contains(flight), "{label}: separate flight");
        assert_named_flight(flight, &original, &[label], world(start), world(centre), 601);
        assert_at(root, &format!("{label} START"), 60.0, &[id(flight, "MISSION BEGIN")]);
        let plane = by_id(flight, block_ids(flight, "Plane")[0]);
        let entity = named(flight, &format!("{label} entity"));
        let entity_id = entity.index.unwrap();
        assert_eq!(plane.property("NumberInFormation"), Some("0"));
        assert_eq!(plane.property("DeleteAfterDeath"), Some("1"));
        assert_eq!(plane.property("Script"), Some("\"LuaScripts\\WorldObjects\\Planes\\f80c10.txt\""));
        assert!(entity.targets.is_empty(), "{label}: no formation links");
        // A command in one copied flight must never address another flight.
        super::testkit::assert_links_resolve(flight);
        assert_eq!(named(flight, "Formation").objects, [entity_id]);
        assert_eq!(named(flight, "Activate Units").objects, [entity_id]);
        let delete = assert_watched_aircraft(root, flight, label, 13);
        close_targets.push(delete);
        expected_deletes.push((480 * TICKS_PER_SECOND, delete));
    }
    assert_at(root, "T-d2 CLOSE", 480.0, &close_targets);

    let damage_commands = assert_damage_commands(root, &[
        (&["J2y", "J3x", "J4y", "J5y", "J6y"][..], "DAMAGE", 120.0),
        (
            &[
                "J1x", "J1y", "J2x", "J2y", "J3x", "J3y",
                "J4x", "J4y", "J5x", "J5y", "J6x", "J6y",
            ][..],
            "DETECTOR",
            270.0,
        ),
    ]);
    assert_eq!(damage_commands.len(), 17);
    assert_eq!(
        damage_commands.iter()
            .filter(|&&index| by_id(root, index).name().unwrap().ends_with(" DETECTOR"))
            .count(),
        12
    );
    let mut probe_deletes: BTreeSet<_> = close_targets[1..].iter().copied().collect();
    for (name, labels) in [
        ("J1", &["J1x", "J1y"][..]),
        ("J2", &["J2x", "J2y"][..]),
        ("J3", &["J3x", "J3y"][..]),
        ("J5", &["J5x"][..]),
    ] {
        let delete = named(root, &format!("{name} DELETE"));
        let delete_id = delete.index.unwrap();
        assert_eq!(delete.block_type, "MCU_Delete");
        assert_eq!(
            delete.objects,
            labels.iter().map(|label| id(root, &format!("{label} entity"))).collect::<Vec<_>>(),
            "{name}: Delete Objects order"
        );
        assert!(delete.targets.is_empty());
        assert_at(root, &format!("{name} DELETE AT"), 240.0, &[delete_id]);
        assert!(probe_deletes.insert(delete_id));
        expected_deletes.push((240 * TICKS_PER_SECOND, delete_id));
    }
    let mut j4_deletes = Vec::new();
    for label in ["J4x", "J4y"] {
        let delete = named(root, &format!("{label} DELETE"));
        assert_eq!(delete.block_type, "MCU_Delete");
        assert_eq!(delete.objects, [id(root, &format!("{label} entity"))]);
        assert!(delete.targets.is_empty());
        j4_deletes.push(delete.index.unwrap());
        assert!(probe_deletes.insert(delete.index.unwrap()));
        expected_deletes.push((240 * TICKS_PER_SECOND, delete.index.unwrap()));
    }
    assert_at(root, "J4 DELETE AT", 240.0, &j4_deletes);
    assert_eq!(
        cell.children.iter()
            .filter(|node| node.block_type == "MCU_Delete")
            .map(|node| node.index.unwrap())
            .collect::<BTreeSet<_>>(),
        probe_deletes,
        "only the six removal Deletes and fourteen individual close Deletes"
    );
    for name in ["J0", "J6"] {
        assert!(root.find_by_name(&format!("{name} DELETE AT")).is_none());
    }
    assert_eq!(
        cell.children.iter()
            .filter(|node| node.block_type == "MCU_Deactivate")
            .map(|node| node.index.unwrap())
            .collect::<Vec<_>>(),
        [id(root, "T-d2 CLOSE OFF")]
    );

    // Include copied template timers: none may remove a control early or add
    // another Delete to the experiment's scheduled removals and closes.
    let log = walk(root, WalkerConfig::default(), &WalkerInput {
        until: 480 * TICKS_PER_SECOND,
        ..WalkerInput::default()
    });
    let mut actual_deletes: Vec<_> = log.pulses.iter().filter_map(|pulse| {
        (by_id(root, pulse.index).block_type == "MCU_Delete").then_some((pulse.t, pulse.index))
    }).collect();
    actual_deletes.sort_unstable();
    expected_deletes.sort_unstable();
    assert_eq!(actual_deletes, expected_deletes);
}

fn assert_formation_delete_probe(root: &Il2Entity) {
    use super::testkit::{TICKS_PER_SECOND, WalkerConfig, WalkerInput, walk};

    let original = parse_group_file(HVAR).unwrap();
    let mut hunter_original = original.clone();
    hunter_original.for_each_mut(&mut |node| {
        if node.block_type == "Plane" {
            node.set_property("Script", "\"LuaScripts\\WorldObjects\\Planes\\mig15bis.txt\"");
        }
    });
    assert_eq!(root.count_block_type("Plane"), 28);
    assert_eq!(root.count_block_type("MCU_TR_Entity"), 28);
    assert_eq!(root.count_block_type("OnEvent"), 28);
    assert_eq!(root.count_block_type("MCU_Delete"), 52); // 14 template + 28 close + 10 removals.
    assert_eq!(root.count_block_type("MCU_CMD_Damage"), 20);
    assert_eq!(root.count_block_type("MCU_CMD_AttackTarget"), 3);
    assert_eq!(root.count_block_type("MCU_Icon"), 11);
    let mut expected_commands = Vec::new();
    let mut close_d3 = vec![id(root, "T-d3 CLOSE OFF")];
    let mut close_d4 = vec![id(root, "T-d4 CLOSE OFF")];
    // Literal rows pin all eleven formations, the three hunter formations,
    // their geometry, names, countries and retained formation links.
    for (cell, name, labels, start, centre, country) in [
        ("T-d3", "K0", ["K0a", "K0b"], (120.0, 20.0), (130.0, 20.0), 601),
        ("T-d3", "K1", ["K1a", "K1b"], (120.0, 28.0), (130.0, 28.0), 601),
        ("T-d3", "K2", ["K2a", "K2b"], (120.0, 36.0), (130.0, 36.0), 601),
        ("T-d3", "K3", ["K3a", "K3b"], (120.0, 44.0), (130.0, 44.0), 601),
        ("T-d3", "K4", ["K4a", "K4b"], (120.0, 52.0), (130.0, 52.0), 601),
        ("T-d3", "K5", ["K5a", "K5b"], (120.0, 60.0), (130.0, 60.0), 601),
        ("T-d3", "K6", ["K6a", "K6b"], (120.0, 68.0), (130.0, 68.0), 601),
        ("T-d3", "K7", ["K7a", "K7b"], (120.0, 76.0), (130.0, 76.0), 601),
        ("T-d4", "K8", ["K8a", "K8b"], (50.0, 20.0), (60.0, 20.0), 601),
        ("T-d4", "K8 HUNTER", ["K8Ha", "K8Hb"], (45.0, 20.0), (60.0, 20.0), 503),
        ("T-d4", "K9", ["K9a", "K9b"], (50.0, 60.0), (60.0, 60.0), 601),
        ("T-d4", "K9 HUNTER", ["K9Ha", "K9Hb"], (45.0, 60.0), (60.0, 60.0), 503),
        ("T-d4", "K10", ["K10a", "K10b"], (50.0, 100.0), (60.0, 100.0), 601),
        ("T-d4", "K10 HUNTER", ["K10Ha", "K10Hb"], (45.0, 100.0), (60.0, 100.0), 503),
    ] {
        let flight = named(root, name);
        assert_eq!(flight.block_type, "Group");
        assert!(named(root, cell).children.contains(flight));
        let source = if name.ends_with(" HUNTER") { &hunter_original } else { &original };
        assert_named_flight(flight, source, &labels, world(start), world(centre), country);
        assert_at(root, &format!("{name} START"), 60.0, &[id(flight, "MISSION BEGIN")]);
        let entities = labels.map(|label| id(flight, &format!("{label} entity")));
        assert_eq!(named(flight, "Formation").objects, [entities[0]]);
        assert_eq!(named(flight, "Activate Units").objects, entities);
        assert_eq!(named(flight, "Formation").property("FlightSize"), Some("1"));
        for (i, label) in labels.into_iter().enumerate() {
            let plane = named(flight, label);
            assert_eq!(number(plane, "NumberInFormation"), i as f64);
            assert_eq!(plane.property("DeleteAfterDeath"), Some("1"));
            let delete = assert_watched_aircraft(root, flight, label, 13);
            match cell {
                "T-d3" => close_d3.push(delete),
                "T-d4" => close_d4.push(delete),
                _ => unreachable!(),
            }
            expected_commands.push((1200 * TICKS_PER_SECOND, delete));
        }
    }
    for (cell, formation_count, targets) in [("T-d3", 8, close_d3), ("T-d4", 6, close_d4)] {
        let group = named(root, cell);
        assert_eq!(group.children.iter().filter(|node| node.block_type == "Group").count(), formation_count);
        assert_at(root, &format!("{cell} CLOSE"), 1200.0, &targets);
        assert_eq!(
            group.children.iter().filter(|node| node.block_type == "MCU_Deactivate")
                .map(|node| node.index.unwrap()).collect::<Vec<_>>(),
            [id(root, &format!("{cell} CLOSE OFF"))]
        );
    }

    let mut damage_commands = BTreeSet::new();
    for (labels, suffix, time_s, level) in [
        (&["K4a", "K6a"][..], "DAMAGE", 120.0, "1"),
        (&["K5a", "K7a"][..], "DAMAGE", 120.0, "0"),
        (
            &["K1a", "K1b", "K2a", "K2b", "K3a", "K3b", "K4a", "K4b", "K5a", "K5b", "K8a", "K8b"][..],
            "DETECTOR", 270.0, "2",
        ),
        (&["K9a", "K9b", "K10a", "K10b"][..], "DETECTOR", 930.0, "2"),
    ] {
        for label in labels {
            let name = format!("{label} {suffix}");
            let command = named(root, &name);
            let index = command.index.unwrap();
            assert_eq!(command.block_type, "MCU_CMD_Damage");
            assert_eq!(command.property("Damage"), Some(level));
            assert_eq!(command.property("Type"), Some("1"));
            assert_eq!(command.objects, [id(root, &format!("{label} entity"))]);
            assert!(command.targets.is_empty());
            assert_at(root, &format!("{name} AT"), time_s, &[index]);
            assert!(damage_commands.insert(index));
            expected_commands.push((time_s as u64 * TICKS_PER_SECOND, index));
        }
    }
    assert_eq!(block_ids(root, "MCU_CMD_Damage").into_iter().collect::<BTreeSet<_>>(), damage_commands);

    // Every removal row, including K3's earlier wingman Delete and K10's
    // two independent Deletes with one clock input, uses entity Objects.
    for (timer, time_s, commands) in [
        ("K3b DELETE AT", 120.0, vec![("K3b DELETE", vec!["K3b"])]),
        ("K1 DELETE AT", 240.0, vec![("K1 DELETE", vec!["K1a"])]),
        ("K2 DELETE AT", 240.0, vec![("K2 DELETE", vec!["K2b"])]),
        ("K3 DELETE AT", 240.0, vec![("K3 DELETE", vec!["K3a", "K3b"])]),
        ("K4 DELETE AT", 240.0, vec![("K4 DELETE", vec!["K4a", "K4b"])]),
        ("K5 DELETE AT", 240.0, vec![("K5 DELETE", vec!["K5a", "K5b"])]),
        ("K8 DELETE AT", 240.0, vec![("K8 DELETE", vec!["K8a", "K8b"])]),
        ("K9 DELETE AT", 900.0, vec![("K9 DELETE", vec!["K9a", "K9b"])]),
        ("K10 DELETE AT", 900.0, vec![("K10a DELETE", vec!["K10a"]), ("K10b DELETE", vec!["K10b"])]),
    ] {
        let mut targets = Vec::new();
        for (name, labels) in commands {
            let command = named(root, name);
            let index = command.index.unwrap();
            assert_eq!(command.block_type, "MCU_Delete");
            assert_eq!(command.objects, labels.iter().map(|label| id(root, &format!("{label} entity"))).collect::<Vec<_>>());
            assert!(command.targets.is_empty());
            targets.push(index);
            expected_commands.push((time_s as u64 * TICKS_PER_SECOND, index));
        }
        assert_at(root, timer, time_s, &targets);
    }
    for name in ["K0", "K6", "K7"] {
        assert!(root.find_by_name(&format!("{name} DELETE AT")).is_none());
    }

    for (name, lead, target, times_s) in [
        ("K8 HUNTER", "K8Ha", "K8a", &[90.0, 180.0][..]),
        ("K9 HUNTER", "K9Ha", "K9a", &[90.0, 180.0, 270.0, 360.0, 450.0, 540.0, 630.0, 720.0, 810.0, 870.0][..]),
        ("K10 HUNTER", "K10Ha", "K10a", &[90.0, 180.0, 270.0, 360.0, 450.0, 540.0, 630.0, 720.0, 810.0, 870.0][..]),
    ] {
        let flight = named(root, name);
        for plane in block_ids(flight, "Plane") {
            let plane = by_id(flight, plane);
            for (key, expected) in [
                ("Country", "503"), ("AILevel", "4"),
                ("Script", "\"LuaScripts\\WorldObjects\\Planes\\mig15bis.txt\""),
                ("Model", "\"graphics\\planes\\mig15bis\\mig15bis.mgm\""),
                ("Skin", "\"\""), ("BotSkin", "\"\""), ("PayloadId", "0"),
                ("ModMask", "1"), ("GunLoad", "[]"), ("GunBelt", "[]"),
            ] {
                assert_eq!(plane.property(key), Some(expected), "{name}: {key}");
            }
        }
        assert_eq!(named(flight, "WP 1").targets, [id(flight, "AttackArea 1")]);
        assert_eq!(number(named(flight, "AttackArea 1"), "Time"), 0.5);
        assert_eq!(named(flight, "AttackArea 1").targets, [id(flight, "AttackArea")]);
        assert_eq!(named(flight, "AttackArea").property("AttackGTargets"), Some("1"));
        let attack = named(root, &format!("{name} ATTACK"));
        let index = attack.index.unwrap();
        assert_eq!(attack.block_type, "MCU_CMD_AttackTarget");
        assert_eq!(attack.property("AttackGroup"), Some("1"));
        assert_eq!(attack.property("Priority"), Some("1"));
        assert_eq!(attack.objects, [id(root, &format!("{lead} entity"))]);
        assert_eq!(attack.targets, [id(root, &format!("{target} entity"))]);
        let mut timers = Vec::new();
        for (i, &time_s) in times_s.iter().enumerate() {
            let timer = format!("{name} ATTACK AT {}", i + 1);
            assert_at(root, &timer, time_s, &[index]);
            timers.push(id(root, &timer));
            expected_commands.push((time_s as u64 * TICKS_PER_SECOND, index));
        }
        assert_eq!(
            block_ids(root, "MCU_Timer").into_iter()
                .filter(|&timer| by_id(root, timer).targets.contains(&index)).collect::<Vec<_>>(),
            timers
        );
    }

    // Include copied template timers. No other command may damage/delete a
    // control or hunter, repeat an experiment, or attack past a pair's Delete.
    let log = walk(root, WalkerConfig::default(), &WalkerInput {
        until: 1200 * TICKS_PER_SECOND,
        ..WalkerInput::default()
    });
    let mut actual_commands: Vec<_> = log.pulses.iter().filter_map(|pulse| {
        matches!(by_id(root, pulse.index).block_type.as_str(),
            "MCU_Delete" | "MCU_CMD_Damage" | "MCU_CMD_AttackTarget")
            .then_some((pulse.t, pulse.index))
    }).collect();
    actual_commands.sort_unstable();
    expected_commands.sort_unstable();
    assert_eq!(actual_commands, expected_commands);
}

fn assert_app_logic_probe(root: &Il2Entity) {
    use super::testkit::{TICKS_PER_SECOND, WalkerConfig, WalkerInput, walk};

    let original = parse_group_file(HVAR).unwrap();
    assert_eq!(root.count_block_type("Plane"), 23);
    assert_eq!(root.count_block_type("MCU_TR_Entity"), 23);
    assert_eq!(root.count_block_type("OnEvent"), 8 + 42 + 12 + 8,
        "8 R ev13 listeners + 42 V listeners + 12 W ev13 listeners + 8 W1-W4 DeathCount events");
    assert_eq!(root.count_block_type("MCU_Delete"), 13 + 23 + 2,
        "13 template Deletes (4 R + 3 V + 6 W) + 23 cell-close Deletes + 2 R removals");
    assert_eq!(root.count_block_type("MCU_CMD_Damage"), 26);
    assert_eq!(root.count_block_type("MCU_CMD_AttackTarget"), 0);
    assert_eq!(root.count_block_type("MCU_Icon"), 13);
    let mut expected_commands = Vec::new();
    let mut close_r = vec![id(root, "T-r CLOSE OFF")];
    let mut close_v = vec![id(root, "T-v CLOSE OFF")];
    let mut close_w = vec![id(root, "T-w CLOSE OFF")];
    for (name, labels, centre, decision) in [
        ("R0", ["R0a", "R0b"], (130.0, 20.0), "1"),
        ("R1", ["R1a", "R1b"], (130.0, 28.0), "0"),
        ("R2", ["R2a", "R2b"], (130.0, 36.0), "0"),
        ("R3", ["R3a", "R3b"], (130.0, 44.0), "1"),
    ] {
        let flight = named(root, name);
        assert!(named(root, "T-r").children.contains(flight));
        assert_named_flight(flight, &original, &labels,
            world((120.0, centre.1)), world(centre), 601);
        assert_at(root, &format!("{name} START"), 60.0, &[id(flight, "MISSION BEGIN")]);
        for label in labels {
            let plane = named(flight, label);
            assert_eq!(plane.property("AiRTBDecision"), Some(decision));
            assert_eq!(number(plane, "Fuel"), 1.0);
            assert_eq!(plane.property("DeleteAfterLand"), Some("1"));
            close_r.push(assert_watched_aircraft(root, flight, label, 13));
        }
    }
    for (label, centre, fuel) in [
        ("V1", (130.0, 52.0), 1.0),
        ("V2", (130.0, 60.0), 0.03),
        ("V3", (130.0, 68.0), 1.0),
    ] {
        let flight = named(root, label);
        assert!(named(root, "T-v").children.contains(flight));
        assert_named_flight(flight, &original, &[label],
            world((120.0, centre.1)), world(centre), 601);
        assert_at(root, &format!("{label} START"), 60.0, &[id(flight, "MISSION BEGIN")]);
        let plane = by_id(flight, block_ids(flight, "Plane")[0]);
        assert_eq!(plane.property("AiRTBDecision"), Some("1"));
        assert_near(number(plane, "Fuel"), fuel);
        assert_eq!(plane.property("DeleteAfterLand"), Some("1"));
        let entity = named(flight, &format!("{label} entity"));
        assert!(entity.targets.is_empty(), "{label}: independent flight");
        assert_eq!(entity.children.len(), 1);
        let events = &entity.children[0];
        assert_eq!(events.block_type, "OnEvents");
        assert_eq!(events.children.len(), 14);
        for (event, event_type) in events.children.iter().zip([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]) {
            assert_eq!(event.block_type, "OnEvent");
            assert_eq!(number(event, "Type"), f64::from(event_type));
            assert_eq!(number(event, "TarId"), f64::from(id(root, &format!("{label} ev{event_type}"))));
        }
        close_v.push(id(root, &format!("{label} CLOSE DELETE")));
    }

    // Each expected row is independent of the generator's three const tables.
    for (name, labels, centre, starter, death_event, delete_block, cooldown, restart_s) in [
        ("W1", ["W1a", "W1b"], (90.0, 20.0), "SPAWN UNITS", Some(4), "Plane", 60.0, None),
        ("W2", ["W2a", "W2b"], (90.0, 28.0), "SPAWN UNITS", Some(13), "Plane", 60.0, None),
        ("W3", ["W3a", "W3b"], (90.0, 36.0), "SPAWN UNITS", Some(13), "Plane", 60.0, Some(240.0)),
        ("W4", ["W4a", "W4b"], (90.0, 44.0), "SPAWN UNITS", Some(13), "MCU_TR_Entity", 60.0, Some(240.0)),
        ("W5", ["W5a", "W5b"], (90.0, 52.0), "MISSION BEGIN", None, "Plane", 0.0, Some(240.0)),
        ("W6", ["W6a", "W6b"], (90.0, 60.0), "MISSION BEGIN", None, "MCU_TR_Entity", 0.0, Some(240.0)),
    ] {
        let group = named(root, name);
        assert!(named(root, "T-w").children.contains(group));
        assert_eq!(group.children.iter().map(|node| node.name().unwrap()).collect::<Vec<_>>(),
            ["Logic", "Units", "Waypoints"]);
        assert_eq!(group.count_block_type("Plane"), 2);
        assert_eq!(group.count_block_type("MCU_TR_Entity"), 2);
        assert_eq!(group.count_block_type("MCU_CheckZone"), 2);
        assert_eq!(group.count_block_type("MCU_Waypoint"), 0, "{name}: no orders means no waypoints");
        assert!(named(group, "Waypoints").children.is_empty());
        let logic = named(group, "Logic");
        let mut node_names = vec![
            "Translator Mission Begin", "ENABLE / PULSE IN", "PULSE OUT", "Zone IN", "Zone Out",
            "Self Deactivate", "Zone Out ReActivate", "Self Deactivate", "Zone In ReActivate",
            "COOLDOWN", "Force Complete - High", "Deactivate Units", "Trigger Delete",
            "MISSION END", "MISSION END ORDERS", "DELAYED END ORDERS", "DELETE DELAY", starter,
            "AFTER BRING UP",
        ];
        if death_event.is_some() {
            node_names.extend(["SpawnCount", "Trigger Spawner", "DeathCount", "DeathCount ReActivate",
                "DeathCount Deactivate", "Modifier Set Value", "Reset Counter"]);
        } else {
            node_names.push("Activate Units");
        }
        assert_eq!(logic.children.iter().map(|node| node.name().unwrap()).collect::<Vec<_>>(), node_names,
            "{name}: all generated nodes retained in order");
        let begin = named(group, "Translator Mission Begin");
        assert_eq!(begin.property("Enabled"), Some("0"));
        assert!(begin.targets.is_empty());
        let entities = labels.map(|label| id(group, &format!("{label} entity")));
        let planes = labels.map(|label| id(group, label));
        for ((label, entity_id), offset) in labels.into_iter().zip(entities).zip([(0.0, 0.0), (-52.5, 150.0)]) {
            let plane = named(group, label);
            let entity = by_id(group, entity_id);
            assert_eq!(plane.block_type, "Plane");
            assert_eq!(plane.property("Script"), Some("\"LuaScripts\\WorldObjects\\Planes\\f80c10.txt\""));
            assert_eq!(plane.property("Country"), Some("601"));
            assert_eq!(plane.property("AILevel"), Some("2"));
            assert_eq!(plane.property("AiRTBDecision"), Some("1"));
            assert_eq!(number(plane, "Fuel"), 1.0);
            assert_eq!(plane.property("NumberInFormation"), Some("0"));
            assert_eq!(plane.property("StartType"), Some("0"));
            assert_eq!(plane.property("DeleteAfterDeath"), Some("1"));
            assert_eq!(plane.property("DeleteAfterLand"), Some("1"));
            assert_near(number(plane, "YPos"), 3050.0);
            assert_eq!(number(plane, "YOri"), 0.0);
            let (x, z) = world(centre);
            assert_eq!(plane.pos_xz(), Some((x + offset.0, z + offset.1)));
            assert_eq!(entity.pos_xz(), plane.pos_xz());
            assert_near(number(entity, "YPos"), 3050.2);
            assert_eq!(number(plane, "LinkTrId"), f64::from(entity_id));
            assert_eq!(number(entity, "MisObjID"), f64::from(plane.index.unwrap()));
            assert_eq!(entity.property("Enabled"), Some("0"));
            assert!(entity.targets.is_empty(), "{name}: no Follows link");
            assert!(entity.objects.is_empty());
            assert_eq!(entity.children.len(), 1);
            let events = &entity.children[0];
            assert_eq!(events.block_type, "OnEvents");
            let mut expected_events = Vec::new();
            if let Some(event_type) = death_event {
                expected_events.push((event_type, id(group, "DeathCount")));
            }
            expected_events.push((13, id(root, &format!("{label} ev13"))));
            assert_eq!(events.children.iter().map(|event| {
                assert_eq!(event.block_type, "OnEvent");
                (number(event, "Type") as i32, number(event, "TarId") as i32)
            }).collect::<Vec<_>>(), expected_events, "{label}: original event plus extra listener");
            close_w.push(id(root, &format!("{label} CLOSE DELETE")));
        }
        for (zone, radius, closer) in [("Zone IN", 16_000.0, true), ("Zone Out", 35_000.0, false)] {
            assert_zone(group, zone, radius, closer, "[2]", centre);
        }
        assert_at(root, &format!("{name} START"), 60.0, &[id(group, starter)]);
        if let Some(time_s) = restart_s {
            assert_at(root, &format!("{name} RESTART"), time_s, &[id(group, starter)]);
            assert_at(root, &format!("{name} END"), 180.0, &[id(group, "MISSION END")]);
            expected_commands.push((182.6, id(group, "Trigger Delete")));
        } else {
            assert!(root.find_by_name(&format!("{name} END")).is_none());
            assert!(root.find_by_name(&format!("{name} RESTART")).is_none());
        }
        let delete = named(group, "Trigger Delete");
        assert_eq!(delete.block_type, "MCU_Delete");
        assert!(delete.targets.is_empty());
        assert_eq!(delete.objects, if delete_block == "Plane" { planes } else { entities });
        for &object in &delete.objects {
            assert_eq!(by_id(group, object).block_type, delete_block);
        }
        for (node, block) in [("Force Complete - High", "MCU_CMD_ForceComplete"), ("Deactivate Units", "MCU_Deactivate")] {
            assert_eq!(named(group, node).block_type, block);
            assert_eq!(named(group, node).objects, entities);
            assert!(named(group, node).targets.is_empty());
        }
        assert_eq!(named(group, "Force Complete - High").property("Priority"), Some("2"));
        for (node, time_s, targets) in [
            ("ENABLE / PULSE IN", 0.1, vec!["Zone IN"]),
            ("PULSE OUT", 0.1, vec!["Zone Out"]),
            ("MISSION END", 0.1, vec!["MISSION END ORDERS", "DELAYED END ORDERS"]),
            ("MISSION END ORDERS", 0.05, vec!["Force Complete - High"]),
            ("DELAYED END ORDERS", 2.0, vec!["Deactivate Units", "DELETE DELAY"]),
            ("DELETE DELAY", 0.5, vec!["Trigger Delete"]),
            ("AFTER BRING UP", 0.5, vec![]),
        ] {
            let node = named(group, node);
            assert_eq!(node.block_type, "MCU_Timer");
            assert_near(number(node, "Time"), time_s);
            assert_eq!(node.property("Random"), Some("100"));
            assert_eq!(node.targets, targets.iter().map(|target| id(group, target)).collect::<Vec<_>>());
        }
        let cooldown_node = named(group, "COOLDOWN");
        assert_eq!(cooldown_node.block_type, "MCU_Timer");
        assert_near(number(cooldown_node, "Time"), cooldown);
        assert_observation(root, cooldown_node.index.unwrap(), &format!("{name} COOLDOWN fired"));
        let starter_node = named(group, starter);
        assert_eq!(starter_node.block_type, "MCU_Timer");
        assert_near(number(starter_node, "Time"), 0.5);
        let self_off: Vec<_> = logic.children.iter().filter(|node| node.name() == Some("Self Deactivate")).collect();
        assert_eq!(self_off.len(), 2);
        for (node, target) in self_off.iter().zip(["Zone IN", "Zone Out"]) {
            assert_eq!(node.block_type, "MCU_Deactivate");
            assert_eq!(node.targets, [id(group, target)]);
        }
        for (node, target) in [("Zone Out ReActivate", "Zone Out"), ("Zone In ReActivate", "Zone IN")] {
            assert_eq!(named(group, node).block_type, "MCU_Activate");
            assert_eq!(named(group, node).targets, [id(group, target)]);
        }
        let mut in_targets = vec![self_off[0].index.unwrap(), id(group, "Zone Out ReActivate"),
            id(group, "PULSE OUT"), id(group, starter)];
        if death_event.is_some() {
            assert_eq!(group.count_block_type("MCU_Spawner"), 1);
            assert!(group.find_by_name("Activate Units").is_none());
            let spawner = named(group, "Trigger Spawner");
            assert_eq!(spawner.block_type, "MCU_Spawner");
            assert_eq!(spawner.property("SpawnAtMe"), Some("0"));
            assert_eq!(spawner.objects, entities);
            assert!(spawner.targets.is_empty());
            assert_counter(group, "DeathCount", 2, 1);
            assert_counter(group, "SpawnCount", 1, 1);
            assert_eq!(starter_node.targets, [id(group, "SpawnCount"), id(group, "AFTER BRING UP")]);
            for (node, successor) in [("DeathCount", "COOLDOWN"), ("COOLDOWN", "Trigger Spawner"), ("SpawnCount", "Trigger Spawner")] {
                let text = format!("{name} {node} fired");
                assert_observation(root, id(group, node), &text);
                assert_eq!(named(group, node).targets, [id(group, successor), id(root, &text)]);
            }
            for (node, block) in [("DeathCount ReActivate", "MCU_Activate"), ("DeathCount Deactivate", "MCU_Deactivate"),
                ("Modifier Set Value", "MCU_ModifierSetVal")] {
                assert_eq!(named(group, node).block_type, block);
                assert_eq!(named(group, node).targets, [id(group, "DeathCount")]);
            }
            assert_eq!(named(group, "Reset Counter").targets, [id(group, "Modifier Set Value")]);
            assert_near(number(named(group, "Reset Counter"), "Time"), 0.5);
            for key in ["ParamIndex", "Data0", "Data1", "Data2", "Data3"] {
                assert_eq!(named(group, "Modifier Set Value").property(key), Some("0"));
            }
            in_targets.push(id(group, "DeathCount ReActivate"));
            assert_eq!(named(group, "Zone Out").targets, [id(group, "DeathCount Deactivate"),
                self_off[1].index.unwrap(), id(group, "MISSION END"), id(group, "Zone In ReActivate"),
                id(group, "ENABLE / PULSE IN"), id(group, "Reset Counter")]);
        } else {
            assert_eq!(group.count_block_type("MCU_Spawner"), 0);
            assert_eq!(group.count_block_type("MCU_Counter"), 0);
            assert_eq!(named(group, "Activate Units").block_type, "MCU_Activate");
            assert_eq!(named(group, "Activate Units").objects, entities);
            assert!(named(group, "Activate Units").targets.is_empty());
            assert_eq!(starter_node.targets, [id(group, "Activate Units"), id(group, "AFTER BRING UP")]);
            assert_eq!(cooldown_node.targets, [id(group, "Zone In ReActivate"), id(group, "ENABLE / PULSE IN"),
                id(root, &format!("{name} COOLDOWN fired"))]);
            assert_eq!(named(group, "Zone Out").targets,
                [self_off[1].index.unwrap(), id(group, "COOLDOWN"), id(group, "MISSION END")]);
        }
        assert_eq!(named(group, "Zone IN").targets, in_targets);
        // The isolated walker requires all links to stay in its root. Remove
        // only the external subtitle listeners before checking for autonomous
        // output; every generated node and internal link remains.
        let mut detached = group.clone();
        remove_ids(&mut detached, &block_ids(root, "MCU_TR_Subtitle").into_iter().collect());
        let idle = walk(&detached, WalkerConfig::default(), &WalkerInput {
            until: 1560 * TICKS_PER_SECOND,
            ..WalkerInput::default()
        });
        // Mission Begin is recorded even with Enabled=0 by the walker. Its
        // empty Targets, not the flag alone, prevent every downstream pulse.
        assert_eq!(idle.pulses.iter().map(|pulse| (pulse.t, pulse.index)).collect::<Vec<_>>(),
            [(0, begin.index.unwrap())], "{name}: no autonomous downstream logic");
    }
    for (cell, labels, targets) in [
        ("T-r", &["R0a", "R0b", "R1a", "R1b", "R2a", "R2b", "R3a", "R3b"][..], close_r),
        ("T-v", &["V1", "V2", "V3"][..], close_v),
        ("T-w", &["W1a", "W1b", "W2a", "W2b", "W3a", "W3b", "W4a", "W4b", "W5a", "W5b", "W6a", "W6b"][..], close_w),
    ] {
        assert_at(root, &format!("{cell} CLOSE"), 1560.0, &targets);
        for label in labels {
            let delete = named(root, &format!("{label} CLOSE DELETE"));
            assert_eq!(delete.block_type, "MCU_Delete");
            assert_eq!(delete.objects, [id(root, &format!("{label} entity"))]);
            assert!(delete.targets.is_empty());
            expected_commands.push((1560.0, delete.index.unwrap()));
        }
    }
    for (labels, suffix, time_s, level) in [
        (&["R0a", "R1a", "R2a", "R3a", "V3"][..], "DAMAGE", 120.0, "1"),
        (&["V1"][..], "DAMAGE", 120.0, "2"),
        (&["W1a", "W1b", "W2a", "W2b"][..], "DAMAGE", 180.0, "2"),
        (&["R0a", "R0b", "R1a", "R1b"][..], "DETECTOR", 330.0, "2"),
        (&["W1a", "W1b", "W2a", "W2b", "W3a", "W3b", "W4a", "W4b", "W5a", "W5b", "W6a", "W6b"][..], "DETECTOR", 360.0, "2"),
    ] {
        for label in labels {
            let name = format!("{label} {suffix}");
            let command = named(root, &name);
            assert_eq!(command.block_type, "MCU_CMD_Damage");
            assert_eq!(command.property("Damage"), Some(level));
            assert_eq!(command.property("Type"), Some("1"));
            assert_eq!(command.objects, [id(root, &format!("{label} entity"))]);
            assert!(command.targets.is_empty());
            assert_at(root, &format!("{name} AT"), time_s, &[command.index.unwrap()]);
            expected_commands.push((time_s, command.index.unwrap()));
        }
    }
    for (name, block, labels) in [
        ("R0 DELETE", "MCU_Delete", ["R0a", "R0b"]),
        ("R1 DELETE", "MCU_Delete", ["R1a", "R1b"]),
        ("R3 DEACTIVATE", "MCU_Deactivate", ["R3a", "R3b"]),
    ] {
        let command = named(root, name);
        assert_eq!(command.block_type, block);
        assert!(named(root, "T-r").children.contains(command), "probe-owned command");
        assert_eq!(command.objects, labels.map(|label| id(root, &format!("{label} entity"))));
        assert!(command.targets.is_empty());
        assert_at(root, &format!("{name} AT"), 300.0, &[command.index.unwrap()]);
        expected_commands.push((300.0, command.index.unwrap()));
    }
    for absent in ["R2 DELETE", "R2 DEACTIVATE", "R3 DELETE", "R2a DETECTOR", "R2b DETECTOR",
        "R3a DETECTOR", "R3b DETECTOR", "V2 DAMAGE"] {
        assert!(root.find_by_name(absent).is_none(), "unexpected command {absent}");
    }
    // Walk all clocked cleanup, including the generated 0.1 + 2 + 0.5 s chain.
    // This checks wiring only: the walker does not simulate spawned aircraft.
    let log = walk(root, WalkerConfig::default(), &WalkerInput {
        until: 1560 * TICKS_PER_SECOND,
        ..WalkerInput::default()
    });
    let mut actual_commands: Vec<_> = log.pulses.iter().filter_map(|pulse| {
        let node = by_id(root, pulse.index);
        (matches!(node.block_type.as_str(), "MCU_Delete" | "MCU_CMD_Damage")
            || node.name() == Some("R3 DEACTIVATE")).then_some((pulse.t, pulse.index))
    }).collect();
    let mut expected_commands: Vec<_> = expected_commands.into_iter()
        .map(|(time_s, index)| ((time_s * TICKS_PER_SECOND as f64).round() as u64, index)).collect();
    actual_commands.sort_unstable();
    expected_commands.sort_unstable();
    assert_eq!(actual_commands, expected_commands);
    let w = named(root, "T-w");
    for zone in block_ids(w, "MCU_CheckZone") {
        assert!(log.pulses.iter().all(|pulse| pulse.index != zone), "T-w never pulses check zones");
    }
    for (name, node, times_s) in [
        ("W1", "Trigger Spawner", &[60.5][..]),
        ("W2", "Trigger Spawner", &[60.5][..]),
        ("W3", "Trigger Spawner", &[60.5, 240.5][..]),
        ("W4", "Trigger Spawner", &[60.5, 240.5][..]),
        ("W5", "Activate Units", &[60.5, 240.5][..]),
        ("W6", "Activate Units", &[60.5, 240.5][..]),
    ] {
        let index = id(named(root, name), node);
        assert_eq!(log.pulses.iter().filter(|pulse| pulse.index == index).map(|pulse| pulse.t).collect::<Vec<_>>(),
            times_s.iter().map(|time_s| (time_s * TICKS_PER_SECOND as f64).round() as u64).collect::<Vec<_>>(),
            "{name}: only the table's clocked bring-up pulses without aircraft events");
    }
}

fn assert_m_icon(root: &Il2Entity, name: &str, centre: (f64, f64)) {
    let icon = named(root, name);
    assert_eq!(icon.block_type, "MCU_Icon");
    assert_eq!(icon.pos_xz(), Some(world(centre)));
    assert_eq!(icon.property("Coalitions"), Some("[2]"));
    assert_eq!(icon.property("Enabled"), Some("1"));
    assert_eq!(icon.property("IconId"), Some("501"));
}

fn m_plane_named<'a>(root: &'a Il2Entity, label: &str) -> &'a Il2Entity {
    let planes: Vec<_> = block_ids(root, "Plane").into_iter().map(|index| by_id(root, index))
        .filter(|plane| plane.name() == Some(label)).collect();
    assert_eq!(planes.len(), 1, "flight 1M {label}: expected one named Plane");
    planes[0]
}

fn assert_m_listener(flight: &str, root: &Il2Entity, entity: i32, label: &str, event_type: i32) -> i32 {
    let subtitle = named(root, &format!("{label} ev{event_type}"));
    assert_eq!(subtitle.block_type, "MCU_TR_Subtitle");
    let target = subtitle.index.unwrap();
    let mut matching = 0;
    by_id(root, entity).for_each(&mut |event| {
        if event.block_type == "OnEvent" && number(event, "Type") == f64::from(event_type)
            && number(event, "TarId") == f64::from(target) {
            matching += 1;
        }
    });
    assert_eq!(matching, 1, "flight {flight} {label}: one ev{event_type} listener");
    target
}

fn assert_m_close(root: &Il2Entity, label: &str, entity: i32) -> i32 {
    let delete = named(root, &format!("{label} CLOSE DELETE"));
    assert_eq!(delete.block_type, "MCU_Delete");
    assert_eq!(delete.objects, [entity]);
    assert!(delete.targets.is_empty());
    delete.index.unwrap()
}

/// Undo only the permitted placement, indexing, Plane names, startup cut and
/// enumerated listeners. All remaining properties, children and links must equal
/// the source, including entity names and the ordered event/report blocks.
fn restore_m_frozen_copy(
    copy: &Il2Entity,
    source: &Il2Entity,
    centre: (f64, f64),
    listeners: &[i32],
) -> Il2Entity {
    use std::collections::HashMap;

    let mut restored = copy.clone();
    let mut source_ids = Vec::new();
    let mut copy_ids = Vec::new();
    source.collect_indexes(&mut source_ids);
    copy.collect_indexes(&mut copy_ids);
    assert_eq!(copy_ids.len(), source_ids.len(), "no source nodes added or removed");
    let inverse: HashMap<_, _> = copy_ids.into_iter().zip(source_ids).collect();
    remove_ids(&mut restored, &listeners.iter().copied().collect());
    crate::duplicate::reconnect_pointers(&mut restored, &inverse);
    let source_anchor = named(source, "Zone IN").pos_xz().unwrap();
    let at = world(centre);
    restored.for_each_mut(&mut |node| {
        let Some(index) = node.index else { return; };
        let old_index = inverse[&index];
        node.index = Some(old_index);
        node.set_property("Index", old_index.to_string());
        let original = by_id(source, old_index);
        for (key, delta) in [("XPos", at.0 - source_anchor.0), ("ZPos", at.1 - source_anchor.1)] {
            if let Some(value) = original.property(key) {
                assert_near(number(node, key), value.parse::<f64>().unwrap() + delta);
                node.set_property(key, value);
            }
        }
        if node.block_type == "Plane" {
            node.set_name(original.name().unwrap());
        }
        if node.block_type == "MCU_TR_Entity"
            && original.children.iter().all(|child| child.block_type != "OnEvents") {
            node.children.retain(|child| !(child.block_type == "OnEvents" && child.children.is_empty()));
        }
        if node.block_type == "MCU_TR_MissionBegin" {
            assert_eq!(node.property("Enabled"), Some("0"));
            assert!(node.targets.is_empty());
            node.set_property("Enabled", original.property("Enabled").unwrap());
            node.set_targets(original.targets.clone());
        }
    });
    restored
}

fn assert_no_autonomous_m_output(root: &Il2Entity, copy: &Il2Entity, listeners: &[i32]) {
    use super::testkit::{TICKS_PER_SECOND, WalkerConfig, WalkerInput, walk};

    let mut detached = copy.clone();
    remove_ids(&mut detached, &listeners.iter().copied().collect());
    let idle = walk(&detached, WalkerConfig::default(), &WalkerInput {
        until: 960 * TICKS_PER_SECOND,
        ..WalkerInput::default()
    });
    assert_eq!(idle.pulses.iter().map(|pulse| (pulse.t, pulse.index)).collect::<Vec<_>>(),
        [(0, id(copy, "Translator Mission Begin"))], "only the disconnected Mission Begin runs");
    // No probe clock pulses either zone; the cell's CLOSE OFF uses deactivation.
    for zone in block_ids(copy, "MCU_CheckZone") {
        root.for_each(&mut |node| {
            if node.targets.contains(&zone) && !["MCU_Activate", "MCU_Deactivate"].contains(&node.block_type.as_str()) {
                assert!(["ENABLE / PULSE IN", "PULSE OUT"].contains(&node.name().unwrap()),
                    "unexpected zone pulse from {:?}", node.name());
            }
        });
    }
}

fn assert_upstream_probe(root: &Il2Entity) {
    let cell = named(root, "T-u");
    assert_eq!(cell.count_block_type("Plane"), 20);
    assert_eq!(cell.count_block_type("MCU_TR_Entity"), 20);
    assert_eq!(cell.count_block_type("MCU_Icon"), 5);
    assert_eq!(cell.count_block_type("MCU_Delete"), 25);
    assert_eq!(cell.count_block_type("OnEvent"), 192); // 80 original - 8 + 120 listeners.
    let strike_timers = ["Formation 1", "Goto WP 1", "AttackArea 1", "OnAreaAttacked 1",
        "Goto WP 2", "Mission Complete 1", "MISSION END", "DELAYED END ORDERS"];
    let cap_timers = ["Goto WP 1", "Goto WP 2", "Goto WP 3", "Return to WP 1", "Goto WP 5"];
    let mut normalized = std::collections::BTreeMap::new();
    let mut close = vec![id(root, "T-u CLOSE OFF")];
    for (name, centre, file, labels, aborts, home, timers, removed_hooks) in [
        ("U1", (130.0, 20.0), "Upstream_Strike_NATO_F80C_4ship.Group", ["U1a", "U1b", "U1c", "U1d"],
            [9, 3, 1, 7], "Goto WP 2", &strike_timers[..], 0),
        ("U2", (50.0, 100.0), "Upstream_Strike_DPRK_Il10_4ship.Group", ["U2a", "U2b", "U2c", "U2d"],
            [9, 3, 1, 7], "Goto WP 2", &strike_timers[..], 0),
        ("U3", (130.0, 60.0), "Upstream_Strike_NATO_F80C_4ship.Group", ["U3a", "U3b", "U3c", "U3d"],
            [9, 3, 1, 7], "Goto WP 2", &strike_timers[..], 4),
        ("U4", (90.0, 20.0), "Upstream_CAP_NATO_F86A_4ship.Group", ["U4a", "U4b", "U4c", "U4d"],
            [8, 7, 3, 1], "Goto WP 5", &cap_timers[..], 0),
        ("U5", (90.0, 60.0), "Upstream_CAP_NATO_F86A_4ship.Group", ["U5a", "U5b", "U5c", "U5d"],
            [8, 7, 3, 1], "Goto WP 5", &cap_timers[..], 4),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/testdata/p14_probe_inputs/upstream_a31ad0a").join(file);
        let source = parse_group_file(&std::fs::read_to_string(path).unwrap()).unwrap();
        let wrapper = named(cell, name);
        assert_eq!(wrapper.children.len(), 1, "{name}: one intact source group inside wrapper");
        let copy = &wrapper.children[0];
        assert_eq!(copy.name(), Some("Unit Template"));
        assert_eq!(named(copy, "Zone IN").pos_xz(), Some(world(centre)));
        assert_at(root, &format!("{name} START"), 60.0, &[id(copy, "MISSION BEGIN")]);
        assert_m_icon(root, &format!("T-u {name}"), centre);
        let planes = block_ids(copy, "Plane");
        let entities = block_ids(copy, "MCU_TR_Entity");
        assert_eq!(planes.len(), 4);
        assert_eq!(entities.len(), 4);
        let mut listeners = Vec::new();
        for ((plane, entity), label) in planes.iter().zip(&entities).zip(labels) {
            assert_eq!(by_id(copy, *plane).name(), Some(label));
            assert_eq!(by_id(copy, *entity).name(), Some("Plane entity"));
            assert_eq!(number(by_id(copy, *plane), "LinkTrId"), f64::from(*entity));
            let mut expected: Vec<_> = aborts.iter().copied()
                .filter(|&event| removed_hooks == 0 || event != 7)
                .map(|event| (event, id(copy, home))).collect();
            for event_type in aborts.into_iter().chain([13, 5]) {
                let subtitle = assert_m_listener("1M", root, *entity, label, event_type);
                expected.push((event_type, subtitle));
                listeners.push(subtitle);
            }
            let events = by_id(copy, *entity).children.iter().find(|child| child.block_type == "OnEvents").unwrap();
            assert_eq!(events.children.iter().map(|event|
                (number(event, "Type") as i32, number(event, "TarId") as i32)).collect::<Vec<_>>(), expected);
            close.push(assert_m_close(root, label, *entity));
        }
        for &timer_name in timers {
            let timer = named(copy, timer_name);
            assert_eq!(timer.block_type, "MCU_Timer");
            let text = format!("{name} {timer_name} fired");
            assert_observation(root, timer.index.unwrap(), &text);
            listeners.push(id(root, &text));
        }
        assert_eq!(listeners.len(), 24 + timers.len());
        assert_eq!(named(copy, "Trigger Delete").objects, planes);
        assert_eq!(named(copy, "Deactivate Units").objects, entities);
        assert_no_autonomous_m_output(root, copy, &listeners);
        let restored = restore_m_frozen_copy(copy, &source, centre, &listeners);
        let mut expected = source;
        let mut removed = 0;
        if removed_hooks != 0 {
            let home_id = id(&expected, home).to_string();
            expected.for_each_mut(&mut |node| {
                node.children.retain(|child| {
                    let remove = child.block_type == "OnEvent" && child.property("Type") == Some("7")
                        && child.property("TarId") == Some(home_id.as_str());
                    removed += usize::from(remove);
                    !remove
                });
            });
        }
        assert_eq!(removed, removed_hooks, "{name}: exactly the listed hooks removed");
        assert_eq!(restored, expected, "flight 1M {name}: equal to source apart from listed changes");
        normalized.insert(name, restored);
    }
    assert_at(root, "T-u CLOSE", 960.0, &close);
    // Direct control comparisons, independent of the per-file checks above.
    for (control, original) in [("U3", "U1"), ("U5", "U4")] {
        let mut expected = normalized[original].clone();
        let before = expected.count_block_type("OnEvent");
        expected.for_each_mut(&mut |node| node.children.retain(|child|
            !(child.block_type == "OnEvent" && child.property("Type") == Some("7"))));
        assert_eq!(before - expected.count_block_type("OnEvent"), 4);
        assert_eq!(normalized[control], expected, "{control} differs from {original} by exactly four hooks");
    }
}

fn assert_generated_fix_probe(root: &Il2Entity) {
    use super::testkit::{TICKS_PER_SECOND, WalkerConfig, WalkerInput, walk};

    let cell = named(root, "T-x");
    assert_eq!(cell.count_block_type("Plane"), 6);
    assert_eq!(cell.count_block_type("MCU_Icon"), 3);
    assert_eq!(cell.count_block_type("MCU_Delete"), 8);
    assert_eq!(cell.count_block_type("OnEvent"), 10);
    let mut close = vec![id(root, "T-x CLOSE OFF")];
    for (name, centre, labels, starter, outputs, cooldown) in [
        ("X1", (10.0, 60.0), ["X1a", "X1b"], "SPAWN UNITS", &["DeathCount", "COOLDOWN", "SpawnCount"][..], 60.0),
        ("X2", (10.0, 68.0), ["X2a", "X2b"], "SPAWN UNITS", &["DeathCount", "COOLDOWN", "SpawnCount"][..], 60.0),
        ("X3", (10.0, 76.0), ["X3a", "X3b"], "MISSION BEGIN", &["COOLDOWN"][..], 0.0),
    ] {
        let copy = named(cell, name);
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/testdata/p14_probe_inputs/flight1m").join(format!("{name}.Group"));
        let source = parse_group_file(&std::fs::read_to_string(path).unwrap()).unwrap();
        let entities = block_ids(copy, "MCU_TR_Entity");
        assert_eq!(entities.len(), 2);
        assert_eq!(copy.count_block_type("MCU_CheckZone"), 2);
        assert_eq!(copy.count_block_type("MCU_Waypoint"), 0);
        assert_at(root, &format!("{name} START"), 60.0, &[id(copy, starter)]);
        assert_m_icon(root, &format!("T-x {name}"), centre);
        assert_eq!(number(named(copy, "COOLDOWN"), "Time"), cooldown);
        let mut listeners = Vec::new();
        for ((plane, &entity), label) in block_ids(copy, "Plane").into_iter().zip(&entities).zip(labels) {
            let plane = by_id(copy, plane);
            assert_eq!(plane.name(), Some(label));
            assert_eq!(plane.property("AiRTBDecision"), Some("1"));
            assert_eq!(plane.property("Fuel"), Some("1"));
            assert_eq!(plane.property("Country"), Some("601"));
            assert_eq!(plane.property("NumberInFormation"), Some("0"));
            assert_eq!(number(plane, "YPos"), 3050.0);
            assert!(by_id(copy, entity).targets.is_empty());
            listeners.push(assert_m_listener("1M", root, entity, label, 13));
            close.push(assert_m_close(root, label, entity));
        }
        for &output in outputs {
            let text = format!("{name} {output} fired");
            assert_observation(root, id(copy, output), &text);
            listeners.push(id(root, &text));
        }
        assert_no_autonomous_m_output(root, copy, &listeners);
        assert_eq!(restore_m_frozen_copy(copy, &source, centre, &listeners), source,
            "flight 1M {name}: no edits to the generated logic");
    }
    assert_at(root, "T-x CLOSE", 960.0, &close);
    for name in ["X1", "X2"] {
        let copy = named(cell, name);
        let entities = block_ids(copy, "MCU_TR_Entity");
        assert_counter(copy, "DeathCount", 2, 1);
        assert_counter(copy, "SpawnCount", 1, 1);
        assert_eq!(named(copy, "Trigger Delete").objects, entities);
        assert_eq!(named(copy, "Trigger Spawner").objects, entities);
        assert_eq!(named(copy, "DELAYED END ORDERS").targets,
            [id(copy, "Deactivate Units"), id(copy, "DELETE DELAY")]);
        assert_eq!(named(copy, "DELETE DELAY").targets, [id(copy, "Trigger Delete")]);
        for &entity in &entities {
            let events = &by_id(copy, entity).children[0];
            assert_eq!(events.children.len(), 2);
            assert_eq!(events.children[0].property("Type"), Some("13"));
            assert_eq!(number(&events.children[0], "TarId"), f64::from(id(copy, "DeathCount")));
        }
        for (output, successor) in [("DeathCount", "COOLDOWN"), ("COOLDOWN", "Trigger Spawner"), ("SpawnCount", "Trigger Spawner")] {
            assert_eq!(named(copy, output).targets,
                [id(copy, successor), id(root, &format!("{name} {output} fired"))]);
        }
    }
    let x3 = named(cell, "X3");
    for absent in ["Trigger Delete", "DELETE DELAY", "DeathCount", "SpawnCount", "Trigger Spawner"] {
        assert!(x3.find_by_name(absent).is_none(), "X3 unexpectedly contains {absent}");
    }
    assert_eq!(named(x3, "DELAYED END ORDERS").targets, [id(x3, "Deactivate Units")]);
    assert_eq!(named(x3, "Activate Units").objects, block_ids(x3, "MCU_TR_Entity"));
    for (name, starter) in [("X2", "SPAWN UNITS"), ("X3", "MISSION BEGIN")] {
        let copy = named(cell, name);
        assert_at(root, &format!("{name} END"), 180.0, &[id(copy, "MISSION END")]);
        assert_at(root, &format!("{name} RESTART"), 240.0, &[id(copy, starter)]);
    }
    assert!(root.find_by_name("X1 END").is_none());
    assert!(root.find_by_name("X1 RESTART").is_none());
    let log = walk(root, WalkerConfig::default(), &WalkerInput {
        until: 960 * TICKS_PER_SECOND,
        ..WalkerInput::default()
    });
    for (group, node, seconds) in [
        ("X1", "Trigger Spawner", &[60.5][..]),
        ("X2", "Trigger Spawner", &[60.5, 240.5][..]),
        ("X2", "Trigger Delete", &[182.6][..]),
        ("X3", "Activate Units", &[60.5, 240.5][..]),
    ] {
        let node = id(named(cell, group), node);
        assert_eq!(log.pulses.iter().filter(|pulse| pulse.index == node).map(|pulse| pulse.t).collect::<Vec<_>>(),
            seconds.iter().map(|s| (s * TICKS_PER_SECOND as f64).round() as u64).collect::<Vec<_>>());
    }
    for zone in block_ids(cell, "MCU_CheckZone") {
        assert!(log.pulses.iter().all(|pulse| pulse.index != zone));
    }
}

fn assert_larger_rtb_probe(root: &Il2Entity) {
    let source = parse_group_file(HVAR).unwrap();
    let cell = named(root, "T-y");
    assert_eq!(cell.count_block_type("Plane"), 9);
    assert_eq!(cell.count_block_type("MCU_Icon"), 3);
    assert_eq!(cell.count_block_type("MCU_Delete"), 15);
    let mut close = vec![id(root, "T-y CLOSE OFF")];
    for (name, labels, start, centre, decision) in [
        ("Y1", &["Y1a", "Y1b", "Y1c", "Y1d"][..], (40.0, 20.0), (50.0, 20.0), "0"),
        ("Y2", &["Y2a", "Y2b", "Y2c", "Y2d"][..], (40.0, 36.0), (50.0, 36.0), "1"),
        ("Y3", &["Y3"][..], (40.0, 52.0), (50.0, 52.0), "1"),
    ] {
        let copy = named(cell, name);
        assert_named_flight(copy, &source, labels, world(start), world(centre), 601);
        assert_at(root, &format!("{name} START"), 60.0, &[id(copy, "MISSION BEGIN")]);
        assert_m_icon(root, &format!("T-y {name}"), centre);
        for &label in labels {
            let plane = m_plane_named(copy, label);
            assert_eq!(plane.property("AiRTBDecision"), Some(decision));
            assert_eq!(plane.property("Fuel"), Some("1"));
            close.push(assert_watched_aircraft(root, copy, label, 13));
        }
        let delete = named(root, &format!("{name} DELETE"));
        assert_eq!(delete.block_type, "MCU_Delete");
        assert_eq!(delete.objects, labels.iter().map(|label| id(copy, &format!("{label} entity"))).collect::<Vec<_>>());
        assert!(delete.targets.is_empty());
        assert_at(root, &format!("{name} DELETE AT"), 300.0, &[delete.index.unwrap()]);
    }
    assert_at(root, "T-y CLOSE", 960.0, &close);

    // Establish the provenance of the zero: HVAR has ones; place_air_sortie
    // writes all four zeros before the probe applies any startup cuts/listeners.
    let mut placed = source.clone();
    for plane in block_ids(&placed, "Plane") {
        assert_eq!(by_id(&placed, plane).property("AiRTBDecision"), Some("1"));
    }
    super::place::place_air_sortie(
        &mut placed,
        &super::place::SortiePlacement {
            target: world((50.0, 20.0)),
            heading_deg: 0.0,
            spawn_dist_m: 10_000.0,
            rtb: world((40.0, 20.0)),
        },
    )
    .unwrap();
    let y1 = named(cell, "Y1");
    assert_eq!(named(&placed, "Trigger Delete").objects, [7, 9, 11, 13]);
    assert_eq!(
        named(y1, "Trigger Delete").objects,
        [
            id(y1, "Y1a entity"),
            id(y1, "Y1b entity"),
            id(y1, "Y1c entity"),
            id(y1, "Y1d entity"),
        ]
    );
    for (produced, actual) in block_ids(&placed, "Plane")
        .into_iter()
        .zip(block_ids(y1, "Plane"))
    {
        assert_eq!(
            by_id(&placed, produced).property("AiRTBDecision"),
            Some("0")
        );
        assert_eq!(
            by_id(y1, actual).property("AiRTBDecision"),
            by_id(&placed, produced).property("AiRTBDecision")
        );
    }
    for node in [&placed, y1] {
        let rtb = named(node, "RTB 1");
        assert_eq!(rtb.block_type, "MCU_Waypoint");
        assert_eq!(rtb.pos_xz(), Some(world((40.0, 20.0))));
        assert_eq!(number(rtb, "YPos"), 3050.0);
        assert_eq!(rtb.property("Area"), Some("1000"));
        assert_eq!(rtb.property("Speed"), Some("660"));
        assert_eq!(rtb.property("Priority"), Some("2"));
        assert_eq!(rtb.objects, [block_ids(node, "MCU_TR_Entity")[0]]);
        assert!(rtb.targets.is_empty());
        assert_eq!(number(named(node, "RTB DELAY"), "Time"), 0.5);
        assert_eq!(named(node, "RTB DELAY").targets, [rtb.index.unwrap()]);
        assert_eq!(named(node, "MISSION END ORDERS").targets,
            [id(node, "Force Complete - High"), id(node, "RTB DELAY")]);
        assert_eq!(number(named(node, "DELAYED END ORDERS"), "Time"), 60.0);
    }
    // Compare all other structure with the ordinary four-plane probe copy.
    // Keep the RTB changes above explicit; discard only their tested nodes.
    let mut ordinary = probe_flight(HVAR, 4, world((40.0, 20.0)), world((50.0, 20.0)), &mut 1);
    ordinary.set_name("Y1");
    // Decision 0019 changes only the placed Y1's Formation density.
    assert_eq!(named(y1, "Formation").property("FormationDensity"), Some("1"));
    assert_eq!(named(&ordinary, "Formation").property("FormationDensity"), Some("0"));
    ordinary.find_by_name_mut("Formation").unwrap().set_property("FormationDensity", "1");
    // The placed copy's Delete repair is asserted separately above.
    let entities = block_ids(&ordinary, "MCU_TR_Entity");
    ordinary
        .find_by_name_mut("Trigger Delete")
        .unwrap()
        .set_objects(entities);
    let mut actual = y1.clone();
    let mut removed: HashSet<_> = [id(y1, "RTB 1"), id(y1, "RTB DELAY")].into_iter().collect();
    for label in ["Y1a", "Y1b", "Y1c", "Y1d"] {
        removed.insert(id(root, &format!("{label} ev13")));
    }
    remove_ids(&mut actual, &removed);
    for node in [&mut actual, &mut ordinary] {
        let planes = block_ids(node, "Plane");
        let originals = block_ids(&source, "Plane");
        for (&plane, original) in planes.iter().zip(originals) {
            by_id_mut(node, plane).set_name(by_id(&source, original).name().unwrap());
        }
        node.for_each_mut(&mut |item| {
            if item.block_type == "MCU_TR_Entity" {
                item.set_name("Plane entity");
                item.children.retain(|child| !(child.block_type == "OnEvents" && child.children.is_empty()));
            }
            // These values have independent literal assertions above.
            if item.block_type == "Plane" {
                item.properties.retain(|(key, _)| key != "AiRTBDecision");
            }
            if item.name() == Some("DELAYED END ORDERS") {
                item.properties.retain(|(key, _)| key != "Time");
            }
            for key in ["XPos", "YPos", "ZPos", "YOri"] {
                if let Some(value) = item.property(key) {
                    item.set_property(key, format!("{:.3}", value.parse::<f64>().unwrap()));
                }
            }
        });
    }
    assert_eq!(
        crate::duplicate::duplicate_template(&actual, &mut 1).0,
        crate::duplicate::duplicate_template(&ordinary, &mut 1).0,
        "Y1 differs only by placement formatting, AiRTBDecision, entity Delete links and the tested RTB chain"
    );
    for name in ["Y2", "Y3"] {
        let copy = named(cell, name);
        assert!(copy.find_by_name("RTB 1").is_none());
        assert!(copy.find_by_name("RTB DELAY").is_none());
        assert_eq!(number(named(copy, "DELAYED END ORDERS"), "Time"), 2.0);
    }

    // Every experiment command in 1M, including X1's independent kills.
    let mut damage_ids = BTreeSet::new();
    for (labels, suffix, seconds, level) in [
        (&["Y1a", "Y2a", "Y3"][..], "DAMAGE", 120.0, "1"),
        (&["X1a", "X1b"][..], "DAMAGE", 180.0, "2"),
        (&["Y1a", "Y1b", "Y1c", "Y1d", "Y2a", "Y2b", "Y2c", "Y2d", "Y3"][..], "DETECTOR", 330.0, "2"),
        (&["X1a", "X1b", "X2a", "X2b", "X3a", "X3b"][..], "DETECTOR", 360.0, "2"),
    ] {
        for label in labels {
            let plane = m_plane_named(root, label);
            let entity = number(plane, "LinkTrId") as i32;
            let name = format!("{label} {suffix}");
            let command = named(root, &name);
            assert_eq!(command.block_type, "MCU_CMD_Damage");
            assert_eq!(command.property("Damage"), Some(level));
            assert_eq!(command.property("Type"), Some("1"));
            assert_eq!(command.objects, [entity]);
            assert!(command.targets.is_empty());
            assert_at(root, &format!("{name} AT"), seconds, &[command.index.unwrap()]);
            damage_ids.insert(command.index.unwrap());
        }
    }
    assert_eq!(block_ids(root, "MCU_CMD_Damage").into_iter().collect::<BTreeSet<_>>(), damage_ids);
    assert_eq!(root.count_block_type("Plane"), 35);
    assert_eq!(root.count_block_type("MCU_Delete"), 48);
    assert_eq!(root.count_block_type("MCU_Icon"), 11);
}

fn assert_repeated_timer_probe(zero: &Il2Entity, one_m: &Il2Entity) {
    // Indexes and locale ids are allocated after the preceding cells. Normalize
    // both allocations, retaining every other key, subtitle and link.
    let original = named(zero, "T-j");
    let copy = named(one_m, "T-j");
    let canonical = |cell: &Il2Entity| {
        let mut cell = crate::duplicate::duplicate_template(cell, &mut 1).0;
        let mut locale_id = 1;
        cell.for_each_mut(&mut |node| {
            if node.property("LCText").is_some() {
                node.set_property("LCText", locale_id.to_string());
                locale_id += 1;
            }
        });
        cell
    };
    assert_eq!(canonical(copy), canonical(original));
    assert_eq!(named(one_m, "PROBE INIT OFF").targets, [id(copy, "J2"), id(copy, "J3")]);
    assert_at(one_m, "T-j ARM", 540.0,
        &[id(copy, "J1"), id(copy, "J2"), id(copy, "J3 ON"), id(copy, "J3"), id(copy, "J5")]);
    assert_switch(one_m, "J1 OFF", "MCU_Deactivate", 545.0, &[id(copy, "J1")]);
    assert_switch(one_m, "J2 ON", "MCU_Activate", 541.0, &[id(copy, "J2")]);
    assert_fan(one_m, "J4 SAME TICK", 540.0, 2, id(copy, "J4"));
    assert_eq!(number(named(copy, "J5"), "Time"), 10.0);
    assert_at(one_m, "J5 RETRIGGER", 545.0, &[id(copy, "J5")]);
    assert_at(one_m, "T-j CLOSE", 600.0, &[id(copy, "T-j CLOSE OFF")]);
    assert_eq!(named(copy, "T-j CLOSE OFF").targets,
        [id(copy, "J1"), id(copy, "J2"), id(copy, "J3"), id(copy, "J4"), id(copy, "J5")]);
}

fn p_fixture(directory: &str, file: &str) -> Il2Entity {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(directory).join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("flight {P_FLIGHT} {file}: cannot read {}: {error}", path.display()));
    parse_group_file(&text)
        .unwrap_or_else(|error| panic!("flight {P_FLIGHT} {file}: cannot parse {}: {error}", path.display()))
}

fn probe_events(entity: &Il2Entity) -> Vec<(i32, i32)> {
    let mut events = Vec::new();
    entity.for_each(&mut |event| {
        if let Some(hook) = read_event(event) { events.push(hook); }
    });
    events
}

fn assert_home_fix_probe(root: &Il2Entity) {
    let cell = named(root, "T-h");
    assert_eq!(cell.count_block_type("Plane"), 16);
    assert_eq!(cell.count_block_type("MCU_TR_Entity"), 16);
    assert_eq!(cell.count_block_type("MCU_Icon"), 4);
    assert_eq!(cell.count_block_type("MCU_Counter"), 2);
    assert_eq!(cell.count_block_type("MCU_Delete"), 20);
    assert_eq!(cell.count_block_type("OnEvent"), 156);
    let mut close = vec![id(root, "T-h CLOSE OFF")];
    for (name, centre, file, labels, aborts, gate_name, home_targets) in [
        ("H1", (50.0, 20.0), "Upstream_Strike_DPRK_Il10_4ship.Group", ["H1a", "H1b", "H1c", "H1d"],
            &[9, 3, 1, 7][..], None, &["Force Complete - High", "WP 2"][..]),
        ("H2", (50.0, 60.0), "Upstream_Strike_DPRK_Il10_4ship.Group", ["H2a", "H2b", "H2c", "H2d"],
            &[9, 3, 1, 7][..], Some("Go Home Once"), &["Force Complete - High", "WP 2"][..]),
        ("H3", (50.0, 100.0), "Upstream_Strike_DPRK_Il10_4ship.Group", ["H3a", "H3b", "H3c", "H3d"],
            &[9, 3, 1, 7][..], None, &["WP 2"][..]),
        ("H4", (130.0, 20.0), "Upstream_Strike_NATO_F80C_4ship.Group", ["H4a", "H4b", "H4c", "H4d"],
            &[9, 3, 1][..], Some("Go Home Once"), &["Force Complete - High", "WP 2"][..]),
    ] {
        let source = p_fixture("src/testdata/p14_probe_inputs/upstream_a31ad0a", file);
        let wrapper = named(cell, name);
        assert_eq!(wrapper.children.len(), 1, "flight {P_FLIGHT} {name}: source wrapper");
        let copy = &wrapper.children[0];
        assert_eq!(copy.name(), Some("Unit Template"));
        assert_eq!(named(copy, "Zone IN").pos_xz(), Some(world(centre)));
        assert_at(root, &format!("{name} START"), 60.0, &[id(copy, "MISSION BEGIN")]);
        assert_m_icon(root, &format!("T-h {name}"), centre);
        let home = id(copy, "Goto WP 2");
        let abort_target = gate_name.map_or(home, |gate| id(copy, gate));
        if let Some(gate) = gate_name {
            assert_counter(copy, gate, 1, 0);
            assert_eq!(named(copy, gate).targets, [home]);
            assert!(named(copy, gate).objects.is_empty());
            assert!(named(copy, "Logic").children.contains(named(copy, gate)));
        } else {
            assert!(copy.find_by_name("Go Home Once").is_none());
        }
        let planes = block_ids(copy, "Plane");
        let entities = block_ids(copy, "MCU_TR_Entity");
        assert_eq!(planes.len(), 4);
        assert_eq!(entities.len(), 4);
        let mut listeners = Vec::new();
        for ((&plane, &entity), label) in planes.iter().zip(&entities).zip(labels) {
            assert_eq!(by_id(copy, plane).name(), Some(label));
            assert_eq!(by_id(copy, entity).name(), Some("Plane entity"));
            assert_eq!(number(by_id(copy, plane), "LinkTrId"), f64::from(entity));
            let mut expected: Vec<_> = aborts.iter().map(|&event| (event, abort_target)).collect();
            for event in [9, 3, 1, 7, 13, 5] {
                let subtitle = assert_m_listener(P_FLIGHT, root, entity, label, event);
                expected.push((event, subtitle));
                listeners.push(subtitle);
            }
            assert_eq!(probe_events(by_id(copy, entity)), expected, "flight {P_FLIGHT} {label}: abort hooks and listeners");
            close.push(assert_m_close(root, label, entity));
        }
        for output in ["Formation 1", "Goto WP 1", "AttackArea 1", "OnAreaAttacked 1",
            "Goto WP 2", "Mission Complete 1", "MISSION END", "DELAYED END ORDERS"] {
            let timer = named(copy, output);
            assert_eq!(timer.block_type, "MCU_Timer");
            let text = format!("{name} {output} fired");
            assert_observation(root, timer.index.unwrap(), &text);
            listeners.push(id(root, &text));
        }
        assert_eq!(listeners.len(), 32);
        let mut targets: Vec<_> = home_targets.iter().map(|target| id(copy, target)).collect();
        targets.push(id(root, &format!("{name} Goto WP 2 fired")));
        assert_eq!(by_id(copy, home).targets, targets);
        // Only abort events are gated. The order report retains its original home link.
        assert_eq!(named(copy, "OnAreaAttacked 1").targets,
            [home, id(root, &format!("{name} OnAreaAttacked 1 fired"))]);
        assert_eq!(named(copy, "Trigger Delete").objects, planes,
            "flight {P_FLIGHT} {name}: upstream Delete still lists Plane blocks");
        assert_eq!(named(copy, "Deactivate Units").objects, entities);
        assert_no_autonomous_m_output(root, copy, &listeners);

        // Undo the added gate independently of the builder, then undo the
        // shared placement/listeners. Every other source field must survive.
        let mut ungated = copy.clone();
        if let Some(gate) = gate_name {
            let gate_id = id(copy, gate);
            let mut restored_hooks = 0;
            ungated.for_each_mut(&mut |event| {
                if read_event(event).is_some_and(|(_, target)| target == gate_id) {
                    event.set_property("TarId", home.to_string());
                    restored_hooks += 1;
                }
            });
            assert_eq!(restored_hooks, 4 * aborts.len());
            remove_ids(&mut ungated, &HashSet::from([gate_id]));
        }
        let restored = restore_m_frozen_copy(&ungated, &source, centre, &listeners);
        let mut expected = source.clone();
        let source_home = id(&expected, "Goto WP 2");
        let source_targets = home_targets.iter().map(|target| id(&expected, target)).collect();
        by_id_mut(&mut expected, source_home).set_targets(source_targets);
        let before = expected.count_block_type("OnEvent");
        expected.for_each_mut(&mut |node| node.children.retain(|event| {
            !read_event(event).is_some_and(|(kind, target)| target == source_home && !aborts.contains(&kind))
        }));
        assert_eq!(before - expected.count_block_type("OnEvent"), 4 * (4 - aborts.len()));
        assert_eq!(restored, expected, "flight {P_FLIGHT} {name}: source differs beyond the table's change");
    }
    assert_at(root, "T-h CLOSE", 960.0, &close);
}

fn assert_fighter_count_probe(root: &Il2Entity) {
    let cell = named(root, "T-f");
    let source = p_fixture("src/testdata/p14_probe_inputs/flight1p", "FighterPack_F80C_4ship.Group");
    assert_eq!(cell.count_block_type("Plane"), 12);
    assert_eq!(cell.count_block_type("MCU_TR_Entity"), 12);
    assert_eq!(cell.count_block_type("MCU_Icon"), 3);
    assert_eq!(cell.count_block_type("MCU_CMD_Damage"), 12);
    assert_eq!(cell.count_block_type("MCU_Delete"), 15);
    assert_eq!(cell.count_block_type("OnEvent"), 64);
    let mut close = vec![id(root, "T-f CLOSE OFF")];
    for (name, centre, labels, aborts, death_targets) in [
        ("F1", (90.0, 20.0), ["F1a", "F1b", "F1c", "F1d"], &[3, 1, 8, 7][..],
            ["Delay Delete 1", "Delay Delete 2", "Delay Delete 3", "Delay Delete 4"]),
        ("F2", (90.0, 60.0), ["F2a", "F2b", "F2c", "F2d"], &[3, 1, 8][..],
            ["Delay Delete 1", "Delay Delete 2", "Delay Delete 3", "Delay Delete 4"]),
        ("F3", (90.0, 100.0), ["F3a", "F3b", "F3c", "F3d"], &[3, 1, 8][..], ["DeathCount"; 4]),
    ] {
        let wrapper = named(cell, name);
        assert_eq!(wrapper.children.len(), 1);
        let copy = &wrapper.children[0];
        assert_eq!(named(copy, "Zone IN").pos_xz(), Some(world(centre)));
        assert_at(root, &format!("{name} START"), 60.0, &[id(copy, "SPAWN UNITS")]);
        assert_m_icon(root, &format!("T-f {name}"), centre);
        assert_counter(copy, "DeathCount", 4, 1);
        assert_eq!(number(named(copy, "COOLDOWN"), "Time"), 180.0);
        assert_eq!(named(copy, "DeathCount").targets,
            [id(copy, "COOLDOWN"), id(root, &format!("{name} DeathCount fired"))]);
        assert_eq!(named(copy, "COOLDOWN").targets,
            [id(copy, "Randomizer:INPUT"), id(root, &format!("{name} COOLDOWN fired"))]);
        let planes = block_ids(copy, "Plane");
        let entities = block_ids(copy, "MCU_TR_Entity");
        assert_eq!(planes.len(), 4);
        assert_eq!(entities.len(), 4);
        assert_eq!(named(copy, "Spawn 1").objects, entities);
        let mut listeners = Vec::new();
        let mut commands = Vec::new();
        for (seat, ((&plane, &entity), (label, death_target))) in
            planes.iter().zip(&entities).zip(labels.into_iter().zip(death_targets)).enumerate() {
            let n = seat + 1;
            let plane = by_id(copy, plane);
            assert_eq!(plane.name(), Some(label));
            assert_eq!(plane.property("Script"), Some("\"LuaScripts\\WorldObjects\\Planes\\f80c10.txt\""));
            assert_eq!(plane.property("Country"), Some("601"));
            assert_eq!(plane.property("AiRTBDecision"), Some("0"));
            assert_eq!(plane.property("NumberInFormation"), Some("0"));
            assert!(by_id(copy, entity).targets.is_empty(), "flight {P_FLIGHT} {label}: Independent seat");
            let done = id(copy, &format!("Mission Complete {n}"));
            let delay = id(copy, &format!("Delay Delete {n}"));
            let target = id(copy, death_target);
            let listener = assert_m_listener(P_FLIGHT, root, entity, label, 13);
            listeners.push(listener);
            let mut expected: Vec<_> = aborts.iter().map(|&event| (event, done)).collect();
            expected.extend([(13, target), (13, listener)]);
            assert_eq!(probe_events(by_id(copy, entity)), expected, "flight {P_FLIGHT} {label}: exact hooks");
            assert_eq!(by_id(copy, done).targets, [id(copy, &format!("CLEANUP {n}"))]);
            assert_eq!(number(by_id(copy, delay), "Time"), 60.0);
            assert_eq!(by_id(copy, delay).targets, [id(copy, &format!("Deactivate {n}")),
                id(copy, "DeathCount"), id(root, &format!("{name} Delay Delete {n} fired"))]);
            assert_eq!(named(copy, &format!("Deactivate {n}")).objects, [entity]);
            close.push(assert_m_close(root, label, entity));
            let damage = named(root, &format!("{label} DAMAGE"));
            assert_eq!(damage.block_type, "MCU_CMD_Damage");
            assert_eq!(damage.property("Damage"), Some("2"));
            assert_eq!(damage.property("Type"), Some("1"));
            assert_eq!(damage.objects, [entity]);
            assert!(damage.targets.is_empty());
            commands.push(damage.index.unwrap());
            let mut inputs = Vec::new();
            root.for_each(&mut |node| {
                if node.targets.contains(&damage.index.unwrap()) { inputs.push(node.index.unwrap()); }
            });
            assert_eq!(inputs, [id(root, &format!("{name} DAMAGE AT"))], "flight {P_FLIGHT} {label}: shared 03:00 timer");
        }
        assert_at(root, &format!("{name} DAMAGE AT"), 180.0, &commands);
        for output in ["DeathCount", "COOLDOWN", "Delay Delete 1", "Delay Delete 2", "Delay Delete 3", "Delay Delete 4"] {
            let text = format!("{name} {output} fired");
            assert_observation(root, id(copy, output), &text);
            listeners.push(id(root, &text));
        }
        assert_no_autonomous_m_output(root, copy, &listeners);
        let restored = restore_m_frozen_copy(copy, &source, centre, &listeners);
        let mut expected = source.clone();
        let before = expected.count_block_type("OnEvent");
        for (entity, death_target) in block_ids(&expected, "MCU_TR_Entity").into_iter().zip(death_targets) {
            let target = id(&expected, death_target);
            by_id_mut(&mut expected, entity).for_each_mut(&mut |node| {
                node.children.retain(|event| !read_event(event).is_some_and(|(kind, _)| kind == 7 && !aborts.contains(&kind)));
                if read_event(node).is_some_and(|(kind, _)| kind == 13) {
                    node.set_property("TarId", target.to_string());
                }
            });
        }
        assert_eq!(before - expected.count_block_type("OnEvent"), 4 * (4 - aborts.len()));
        assert_eq!(restored, expected, "flight {P_FLIGHT} {name}: no changes beyond event 7 removal and death retargeting");
    }
    assert_at(root, "T-f CLOSE", 960.0, &close);
}

fn assert_bingo_aircraft_probe(root: &Il2Entity) {
    let cell = named(root, "T-b");
    assert_eq!(cell.count_block_type("Plane"), 12);
    assert_eq!(cell.count_block_type("MCU_TR_Entity"), 12);
    assert_eq!(cell.count_block_type("MCU_Icon"), 12);
    assert_eq!(cell.count_block_type("MCU_Delete"), 12);
    assert_eq!(cell.count_block_type("MCU_CMD_Damage"), 0);
    assert_eq!(cell.count_block_type("MCU_Waypoint"), 0);
    assert_eq!(cell.count_block_type("OnEvent"), 60);
    let mut close = vec![id(root, "T-b CLOSE OFF")];
    for (name, centre, script, country) in [
        ("B1", (10.0, 20.0), "f80c10", "601"),
        ("B2", (10.0, 28.0), "f84e", "601"),
        ("B3", (10.0, 36.0), "f86a5", "601"),
        ("B4", (10.0, 44.0), "f51d", "601"),
        ("B5", (10.0, 52.0), "b29", "601"),
        ("B6", (10.0, 60.0), "c47b", "601"),
        ("B7", (10.0, 68.0), "mig15bis", "503"),
        ("B8", (10.0, 76.0), "la11", "503"),
        ("B9", (10.0, 84.0), "yak9p", "503"),
        ("B10", (10.0, 92.0), "il10", "503"),
        ("B11", (10.0, 100.0), "tu2", "503"),
        ("B12", (10.0, 108.0), "li2t", "503"),
    ] {
        let copy = named(cell, name);
        assert!(cell.children.contains(copy));
        let source = p_fixture("src/testdata/p14_probe_inputs/flight1p", &format!("{name}.Group"));
        let (plane, entity) = single_aircraft(copy, &format!("flight {P_FLIGHT} T-b {name}"));
        let plane = by_id(copy, plane);
        assert_eq!(plane.name(), Some(name));
        assert_eq!(crate::model_spec::script_id(plane.property("Script").unwrap()), script);
        assert_eq!(plane.property("Country"), Some(country));
        assert_eq!(plane.property("StartType"), Some("0"));
        assert_eq!(plane.property("NumberInFormation"), Some("0"));
        assert_eq!(number(plane, "YPos"), 1500.0);
        assert_eq!(number(plane, "Fuel"), 1.0);
        assert_eq!(plane.pos_xz(), Some(world(centre)));
        assert!(by_id(copy, entity).targets.is_empty());
        assert_at(root, &format!("{name} START"), 60.0, &[id(copy, "MISSION BEGIN")]);
        assert_eq!(named(copy, "Activate Units").objects, [entity]);
        assert!(named(copy, "Waypoints").children.is_empty());
        assert!(copy.find_by_name("Trigger Delete").is_none());
        assert!(copy.find_by_name("DELETE DELAY").is_none());
        let listeners: Vec<_> = [7, 8, 9, 13, 5].into_iter()
            .map(|event| assert_m_listener(P_FLIGHT, root, entity, name, event)).collect();
        assert_eq!(probe_events(by_id(copy, entity)), [7, 8, 9, 13, 5].into_iter().zip(listeners.iter().copied()).collect::<Vec<_>>());
        close.push(assert_m_close(root, name, entity));
        assert_m_icon(root, &format!("T-b {name}"), centre);
        assert_no_autonomous_m_output(root, copy, &listeners);
        assert_eq!(restore_m_frozen_copy(copy, &source, centre, &listeners), source,
            "flight {P_FLIGHT} {name}: generated aircraft changed beyond placement, names and listeners");
    }
    assert_at(root, "T-b CLOSE", 960.0, &close);
}

#[test]
fn flight1p_clock_only_starts_kills_and_closes_the_table_rows() {
    use super::testkit::{TICKS_PER_SECOND, WalkerConfig, WalkerInput, walk};

    let root = generate_probe_1p();
    let log = walk(&root, WalkerConfig::default(), &WalkerInput {
        until: 960 * TICKS_PER_SECOND,
        ..WalkerInput::default()
    });
    let mut expected = Vec::new();
    for label in ["F1a", "F1b", "F1c", "F1d", "F2a", "F2b", "F2c", "F2d", "F3a", "F3b", "F3c", "F3d"] {
        expected.push((180 * TICKS_PER_SECOND, id(&root, &format!("{label} DAMAGE"))));
    }
    for label in ["H1a", "H1b", "H1c", "H1d", "H2a", "H2b", "H2c", "H2d",
        "H3a", "H3b", "H3c", "H3d", "H4a", "H4b", "H4c", "H4d",
        "F1a", "F1b", "F1c", "F1d", "F2a", "F2b", "F2c", "F2d", "F3a", "F3b", "F3c", "F3d",
        "B1", "B2", "B3", "B4", "B5", "B6", "B7", "B8", "B9", "B10", "B11", "B12"] {
        expected.push((960 * TICKS_PER_SECOND, id(&root, &format!("{label} CLOSE DELETE"))));
    }
    let mut actual: Vec<_> = log.pulses.iter().filter_map(|pulse| {
        matches!(by_id(&root, pulse.index).block_type.as_str(), "MCU_Delete" | "MCU_CMD_Damage")
            .then_some((pulse.t, pulse.index))
    }).collect();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected, "flight {P_FLIGHT}: exactly twelve clocked kills and forty individual closes");
    for (names, bring_up, seconds) in [
        (&["H1", "H2", "H3", "H4", "B1", "B2", "B3", "B4", "B5", "B6", "B7", "B8", "B9", "B10", "B11", "B12"][..], "Activate Units", 60.5),
        (&["F1", "F2", "F3"][..], "Spawn 1", 61.0),
    ] {
        for name in names {
            let index = id(named(&root, name), bring_up);
            assert_eq!(log.pulses.iter().filter(|pulse| pulse.index == index).map(|pulse| pulse.t).collect::<Vec<_>>(),
                [(seconds * TICKS_PER_SECOND as f64).round() as u64], "flight {P_FLIGHT} {name}: only one clocked bring-up");
        }
    }
    for zone in block_ids(&root, "MCU_CheckZone") {
        assert!(log.pulses.iter().all(|pulse| pulse.index != zone), "flight {P_FLIGHT}: zone {zone} must never be pulsed");
    }
}

#[test]
fn flight1p_home_gate_passes_only_the_first_abort() {
    use super::testkit::{TICKS_PER_SECOND, WalkerConfig, WalkerInput, walk};

    let root = generate_probe_1p();
    let mut input = WalkerInput { until: 300 * TICKS_PER_SECOND, ..WalkerInput::default() };
    for name in ["H1", "H2", "H3", "H4"] {
        let copy = named(&root, name);
        let entities = block_ids(copy, "MCU_TR_Entity");
        for (entity, event, seconds) in [(entities[0], 9, 240), (entities[1], 3, 242)] {
            let target = probe_events(by_id(copy, entity)).into_iter().find(|&(kind, target)| {
                kind == event && by_id(&root, target).block_type != "MCU_TR_Subtitle"
            }).unwrap().1;
            input.pulses.push((seconds * TICKS_PER_SECOND, target.into()));
        }
    }
    let log = walk(&root, WalkerConfig::default(), &input);
    for (name, expected_seconds) in [
        ("H1", &[240.5, 242.5][..]), ("H2", &[240.5][..]),
        ("H3", &[240.5, 242.5][..]), ("H4", &[240.5][..]),
    ] {
        let home = id(named(&root, name), "Goto WP 2");
        assert_eq!(log.pulses.iter().filter(|pulse| pulse.index == home).map(|pulse| pulse.t).collect::<Vec<_>>(),
            expected_seconds.iter().map(|seconds| (seconds * TICKS_PER_SECOND as f64).round() as u64).collect::<Vec<_>>(),
            "flight {P_FLIGHT} {name}: repeated aborts");
    }
}

#[test]
fn flight1p_fighter_count_and_replacement_need_no_zone_or_player() {
    use super::testkit::{TICKS_PER_SECOND, WalkerConfig, WalkerInput, walk};

    let root = generate_probe_1p();
    let mut input = WalkerInput { until: 500 * TICKS_PER_SECOND, ..WalkerInput::default() };
    for name in ["F2", "F3"] {
        let copy = named(&root, name);
        for (seat, entity) in block_ids(copy, "MCU_TR_Entity").into_iter().enumerate() {
            input.kills.push((180 * TICKS_PER_SECOND, entity.into()));
            // A killed aircraft also reports wounded/critical damage. Model
            // two such inputs 0.1 and 0.2 s later: Mission Complete + CLEANUP
            // restart Delay Delete. F2 counts once about 60 s after the kills;
            // F3 counts at the kills and again about 60 s later (0012 alone).
            let done = id(copy, &format!("Mission Complete {}", seat + 1));
            input.pulses.extend([(180 * TICKS_PER_SECOND + 2, done.into()), (180 * TICKS_PER_SECOND + 4, done.into())]);
        }
    }
    // F1's suspected event 7 is a supplied event, not a simulated game fact.
    // No synthetic kills on its already deactivated first wave.
    let f1 = named(&root, "F1");
    for entity in block_ids(f1, "MCU_TR_Entity") {
        let target = probe_events(by_id(f1, entity)).into_iter().find(|&(event, _)| event == 7).unwrap().1;
        input.pulses.push((63 * TICKS_PER_SECOND, target.into()));
    }
    let log = walk(&root, WalkerConfig::default(), &input);
    for (name, output, expected_seconds) in [
        ("F1", "DeathCount", &[123.6][..]),
        ("F1", "COOLDOWN", &[303.6][..]),
        ("F1", "Spawn 1", &[61.0, 304.1][..]),
        ("F2", "DeathCount", &[240.8][..]),
        ("F2", "COOLDOWN", &[420.8][..]),
        ("F2", "Spawn 1", &[61.0, 421.3][..]),
        ("F3", "DeathCount", &[180.0, 240.8][..]),
        // The second count restarts the same 180 s cooldown, so this control
        // has two counts but one replacement in the scripted interval.
        ("F3", "COOLDOWN", &[420.8][..]),
        ("F3", "Spawn 1", &[61.0, 421.3][..]),
    ] {
        let node = id(named(&root, name), output);
        assert_eq!(log.pulses.iter().filter(|pulse| pulse.index == node).map(|pulse| pulse.t).collect::<Vec<_>>(),
            expected_seconds.iter().map(|seconds| (seconds * TICKS_PER_SECOND as f64).round() as u64).collect::<Vec<_>>(),
            "flight {P_FLIGHT} {name}: {output}");
    }
    for zone in block_ids(&root, "MCU_CheckZone") {
        assert!(log.pulses.iter().all(|pulse| pulse.index != zone), "flight {P_FLIGHT}: replacement requires no zone");
    }
}

#[test]
fn probe_cells_match_the_table() {
    let zero = generate_probe_0();
    assert_eq!(named(&zero, "T-f").count_block_type("Plane"), 0);
    let one_a = generate_probe_1a();
    let one_g = generate_probe_1g();
    let one_n = generate_probe_1n();
    let one_d = generate_probe_1d();
    let one_h = generate_probe_1h();
    let one_j = generate_probe_1j();
    let one_k = generate_probe_1k();
    let one_l = generate_probe_1l();
    let one_m = generate_probe_1m();
    let one_p = generate_probe_1p();
    assert_eq!(zero.count_block_type("Plane"), 4);
    assert_eq!(one_a.count_block_type("Plane"), 0);
    assert_eq!(one_g.count_block_type("Plane"), 10);
    assert_eq!(one_n.count_block_type("Plane"), 4);
    assert_eq!(one_d.count_block_type("Plane"), 4);
    assert_delete_probe(&one_h);
    assert_single_delete_probe(&one_j);
    assert_formation_delete_probe(&one_k);
    assert_app_logic_probe(&one_l);
    assert_upstream_probe(&one_m);
    assert_generated_fix_probe(&one_m);
    assert_larger_rtb_probe(&one_m);
    assert_repeated_timer_probe(&zero, &one_m);
    assert_home_fix_probe(&one_p);
    assert_fighter_count_probe(&one_p);
    assert_bingo_aircraft_probe(&one_p);
    assert_eq!(one_p.count_block_type("Plane"), 40);
    assert_eq!(one_p.count_block_type("MCU_Delete"), 47);
    assert_eq!(one_p.count_block_type("MCU_CMD_Damage"), 12);
    assert_eq!(one_p.count_block_type("MCU_Icon"), 19);

    for (name, coalition) in [("ZA1", "[2]"), ("ZA2", "[1]")] {
        assert_zone(&zero, name, 12_000.0, true, coalition, (0.0, 60.0));
        assert!(named(&zero, "T-a1 ARM").targets.contains(&id(&zero, name)));
        assert_observation(&zero, id(&zero, name), &format!("{name} fired"));
    }
    let ai = named(&zero, "T-a1 AI");
    assert_flight(ai, 4, world((-30.0, 60.0)), world((30.0, 60.0)));
    assert_at(&zero, "T-a1 AI START", 10.0, &[id(ai, "MISSION BEGIN")]);
    let delete = named(&zero, "T-a1 AI CLOSE DELETE");
    assert_eq!(delete.block_type, "MCU_Delete");
    assert_eq!(delete.objects, block_ids(ai, "MCU_TR_Entity"));
    assert!(
        named(&zero, "T-a1 CLOSE")
            .targets
            .contains(&delete.index.unwrap())
    );

    for n in 1..=9 {
        let name = format!("E{n}");
        assert_counter(
            &zero,
            &name,
            if n == 8 { 1 } else { 3 },
            i32::from([2, 7, 9].contains(&n)),
        );
        assert_observation(&zero, id(&zero, &name), &format!("{name} fired"));
    }
    assert_loop(
        &zero,
        "E1 E2",
        60.0,
        2.0,
        6,
        &[id(&zero, "E1"), id(&zero, "E2")],
    );
    assert_pulses(&zero, "E3", &[60.0, 62.0, 66.0], id(&zero, "E3"));
    assert_switch(&zero, "E3 OFF", "MCU_Deactivate", 63.0, &[id(&zero, "E3")]);
    assert_switch(&zero, "E3 ON", "MCU_Activate", 65.0, &[id(&zero, "E3")]);
    for name in ["E4", "E5"] {
        assert_pulses(&zero, name, &[60.0, 62.0, 64.0], id(&zero, name));
    }
    assert!(
        named(&zero, "PROBE INIT OFF")
            .targets
            .contains(&id(&zero, "E5"))
    );
    assert_switch(&zero, "E5 ON", "MCU_Activate", 63.0, &[id(&zero, "E5")]);
    assert_pulses(
        &zero,
        "E6",
        &[60.0, 62.0, 64.0, 66.0, 68.0, 70.0],
        id(&zero, "E6"),
    );
    for (name, at) in [("E4", 63.0), ("E6", 65.0)] {
        let reset = named(&zero, &format!("{name} RESET"));
        assert_eq!(reset.block_type, "MCU_ModifierSetVal");
        for key in ["ParamIndex", "Data0", "Data1", "Data2", "Data3"] {
            assert_eq!(reset.property(key), Some("0"));
        }
        assert_eq!(reset.targets, [id(&zero, name)]);
        assert_at(
            &zero,
            &format!("{name} RESET AT"),
            at,
            &[reset.index.unwrap()],
        );
    }
    assert_fan(&zero, "E7 SAME TICK", 100.0, 3, id(&zero, "E7"));
    assert_fan(&zero, "E8 SAME TICK", 110.0, 2, id(&zero, "E8"));
    assert_pulses(&zero, "E9", &[120.0, 120.05, 120.1], id(&zero, "E9"));
    for name in ["E1", "E2", "E6", "E8"] {
        assert_readout(&zero, id(&zero, name), &format!("{name} fired 2+"), 2);
    }

    assert_eq!(named(&zero, "F1").property("Random"), Some("50"));
    assert_eq!(number(named(&zero, "F1"), "Time"), 0.0);
    assert_loop(&zero, "F1", 150.0, 3.0, 40, &[id(&zero, "F1")]);
    assert_observation(&zero, id(&zero, "F1"), "F1 hit");
    assert_readout(&zero, id(&zero, "F1"), "F1 reached 12", 12);
    assert_readout(&zero, id(&zero, "F1"), "F1 reached 29", 29);
    let outs: Vec<_> = (1..=4).map(|k| id(&zero, &format!("F2 OUT {k}"))).collect();
    let open = named(&zero, "F2 OPEN OUTPUTS");
    assert_eq!(open.block_type, "MCU_Activate");
    assert_eq!(open.targets, outs);
    let mut inputs = vec![open.index.unwrap()];
    for (k, pct) in [25, 33, 50, 100].into_iter().enumerate() {
        let random = named(&zero, &format!("F2 RANDOM {}", k + 1));
        assert_eq!(number(random, "Random"), f64::from(pct));
        assert_eq!(number(random, "Time"), 0.5 * (k + 1) as f64);
        assert!(random.targets.contains(&outs[k]));
        assert_observation(&zero, outs[k], &format!("F2 out {}", k + 1));
        if k < 3 {
            let close = named(&zero, &format!("F2 CLOSE AFTER {}", k + 1));
            assert_eq!(close.block_type, "MCU_Deactivate");
            assert_eq!(close.targets, outs[k + 1..]);
            assert!(random.targets.contains(&close.index.unwrap()));
        }
        inputs.push(random.index.unwrap());
    }
    assert_loop(&zero, "F2", 330.0, 5.0, 40, &inputs);

    // Keep the T-t window and pulse times with one informational Spawn carrier.
    assert_at(&zero, "T-t ARM", 270.0, &[]);
    assert_at(&zero, "T-t CLOSE", 330.0, &[id(&zero, "T-t CLOSE OFF")]);
    for (name, time) in [
        ("T-t T1", 275.0),
        ("T-t T3 SAME TICK", 287.0),
        ("T-t T7 SAME TICK", 325.0),
    ] {
        let node = named(&zero, name);
        assert_at(&zero, name, time, &node.targets);
    }
    assert_pulses(
        &zero,
        "T-t T2",
        &[277.0, 279.0, 281.0, 283.0, 285.0],
        id(&zero, "T-t T2"),
    );
    assert_eq!(
        named(&zero, "T-t T3 SAME TICK").targets,
        [id(&zero, "T-t T3a"), id(&zero, "T-t T3b")]
    );
    let tt = named(&zero, "T-t");
    assert_eq!(tt.count_block_type("MCU_Spawner"), 1);
    assert_eq!(tt.count_block_type("MCU_TR_MissionObjective"), 5);
    tt.for_each(&mut |node| {
        assert!(!node.name().is_some_and(|name| name.starts_with("T-t T4")));
    });
    assert_pulses(&zero, "T-t T5", &[315.0, 320.0], id(&zero, "T-t T5"));
    assert_fan(
        &zero,
        "T-t T7 SAME TICK",
        325.0,
        2,
        id(&zero, "T-t T7 fired"),
    );

    for name in ["J1", "J5"] {
        assert_eq!(number(named(&zero, name), "Time"), 10.0);
        assert!(named(&zero, "T-j ARM").targets.contains(&id(&zero, name)));
    }
    assert_switch(&zero, "J1 OFF", "MCU_Deactivate", 545.0, &[id(&zero, "J1")]);
    assert!(
        named(&zero, "PROBE INIT OFF")
            .targets
            .contains(&id(&zero, "J2"))
    );
    assert!(named(&zero, "T-j ARM").targets.contains(&id(&zero, "J2")));
    assert_switch(&zero, "J2 ON", "MCU_Activate", 541.0, &[id(&zero, "J2")]);
    assert!(
        named(&zero, "PROBE INIT OFF")
            .targets
            .contains(&id(&zero, "J3"))
    );
    assert_eq!(named(&zero, "J3 ON").targets, [id(&zero, "J3")]);
    for name in ["J3 ON", "J3"] {
        assert!(named(&zero, "T-j ARM").targets.contains(&id(&zero, name)));
    }
    assert_fan(&zero, "J4 SAME TICK", 540.0, 2, id(&zero, "J4"));
    assert_eq!(named(&zero, "J4 LATCH OFF").targets, [id(&zero, "J4")]);
    assert!(
        named(&zero, "J4")
            .targets
            .contains(&id(&zero, "J4 LATCH OFF"))
    );
    assert_at(&zero, "J5 RETRIGGER", 545.0, &[id(&zero, "J5")]);
    for name in ["J1", "J2", "J3", "J4", "J5"] {
        assert_observation(&zero, id(&zero, name), &format!("{name} out"));
    }
    for name in ["J4", "J5"] {
        assert_readout(&zero, id(&zero, name), &format!("{name} out 2+"), 2);
    }

    assert_zone(&zero, "Z1", 5000.0, false, "[1]", (-40.0, -40.0));
    assert_eq!(named(&zero, "Z1 ON").targets, [id(&zero, "Z1")]);
    assert_eq!(
        named(&zero, "T-z ARM").targets,
        [id(&zero, "Z1 ON"), id(&zero, "Z1")]
    );
    assert_switch(
        &zero,
        "Z1 RECHECK OFF",
        "MCU_Deactivate",
        610.0,
        &[id(&zero, "Z1")],
    );
    assert_at(
        &zero,
        "Z1 RECHECK ON AT",
        615.0,
        &[id(&zero, "Z1 RECHECK ON"), id(&zero, "Z1")],
    );
    assert_eq!(
        named(&zero, "Z1 RECHECK ON").targets,
        [id(&zero, "Z1"), id(&zero, "Z1 AGAIN OUT")]
    );
    assert_observation(&zero, id(&zero, "Z1"), "Z1 fired");
    assert_observation(&zero, id(&zero, "Z1 AGAIN OUT"), "Z1 fired again");
    assert!(
        named(&zero, "PROBE INIT OFF")
            .targets
            .contains(&id(&zero, "Z1 AGAIN OUT"))
    );

    assert_eq!(zero.count_block_type("MCU_TR_MissionObjective"), 5);
    for root in [&zero, &one_a, &one_n, &one_d, &one_g, &one_h, &one_j, &one_k, &one_l, &one_m, &one_p] {
        assert!(root.find_by_name("T-o").is_none());
        assert!(root.find_by_name("P14 DPRK SPAWN").is_none());
    }
    for root in [&one_a, &one_n, &one_d, &one_g, &one_h, &one_j, &one_k, &one_l, &one_m, &one_p] {
        assert_eq!(root.count_block_type("MCU_TR_MissionObjective"), 0);
    }

    for n in 0..=5 {
        let name = format!("C{n}");
        assert_zone(
            &one_a,
            &name,
            5000.0,
            true,
            "[2]",
            if n == 4 { (40.0, -8.0) } else { (40.0, -20.0) },
        );
        assert_observation(&one_a, id(&one_a, &name), &format!("{name} fired"));
        let starts_off = named(&one_a, "PROBE INIT OFF")
            .targets
            .contains(&id(&one_a, &name));
        assert_eq!(starts_off, (1..=4).contains(&n), "{name} start state");
    }
    assert_at(&one_a, "C0 SENTINEL", 660.0, &[id(&one_a, "C0")]);
    assert_switch(&one_a, "C1 ON", "MCU_Activate", 660.0, &[id(&one_a, "C1")]);
    assert_switch(&one_a, "C2 ON", "MCU_Activate", 660.0, &[id(&one_a, "C2")]);
    assert_at(&one_a, "C2 PULSE", 660.1, &[id(&one_a, "C2")]);
    assert_at(&one_a, "C3 INACTIVE PULSE", 660.0, &[id(&one_a, "C3")]);
    assert_switch(
        &one_a,
        "C2 RECHECK OFF",
        "MCU_Deactivate",
        670.0,
        &[id(&one_a, "C2")],
    );
    assert_switch(
        &one_a,
        "C2 RECHECK ON",
        "MCU_Activate",
        675.0,
        &[id(&one_a, "C2"), id(&one_a, "C2 AGAIN OUT")],
    );
    assert_at(&one_a, "C2 RECHECK PULSE", 675.1, &[id(&one_a, "C2")]);
    assert_observation(&one_a, id(&one_a, "C2 AGAIN OUT"), "C2 fired again");
    assert!(
        named(&one_a, "C2")
            .targets
            .contains(&id(&one_a, "C2 AGAIN OUT"))
    );
    assert!(
        named(&one_a, "PROBE INIT OFF")
            .targets
            .contains(&id(&one_a, "C2 AGAIN OUT"))
    );
    assert_switch(&one_a, "C4 ON", "MCU_Activate", 690.0, &[id(&one_a, "C4")]);
    assert_at(&one_a, "C4 PULSE", 690.1, &[id(&one_a, "C4")]);
    assert_switch(
        &one_a,
        "C4 OFF",
        "MCU_Deactivate",
        705.0,
        &[id(&one_a, "C4")],
    );
    assert_at(&one_a, "C5 PULSE", 630.0, &[id(&one_a, "C5")]);
    let mut c5_inputs = vec![];
    one_a.for_each(&mut |node| {
        if node.block_type == "MCU_Timer" && node.targets.contains(&id(&one_a, "C5")) {
            c5_inputs.push(node.index.unwrap());
        }
    });
    assert_eq!(c5_inputs, [id(&one_a, "C5 PULSE")]);
    assert_readout(&one_a, id(&one_a, "C5"), "C5 fired 2+", 2);
    assert_readout(&one_a, id(&one_a, "C5"), "C5 fired 10+", 10);

    for (name, km) in [("T-n START", (30.0, 30.0)), ("T-n END", (54.0, 30.0))] {
        let zone_name = format!("{name} ZONE");
        assert_zone(&one_a, &zone_name, 2000.0, true, "[2]", km);
        assert!(
            named(&one_a, "T-n ARM")
                .targets
                .contains(&id(&one_a, &zone_name))
        );
        let off = named(&one_a, &format!("{name} OFF"));
        assert_eq!(off.block_type, "MCU_Deactivate");
        assert_eq!(off.targets, [id(&one_a, &zone_name)]);
        assert!(
            named(&one_a, &zone_name)
                .targets
                .contains(&off.index.unwrap())
        );
        assert_observation(&one_a, id(&one_a, &zone_name), name);
    }
    let start = named(&one_a, "T-n START ZONE").pos_xz().unwrap();
    let end = named(&one_a, "T-n END ZONE").pos_xz().unwrap();
    assert_near(end.0 - start.0, 24_000.0);
    assert_near(end.1 - start.1, 0.0);
    assert_zone(&one_a, "ZA4", 12_000.0, true, "[2]", (42.0, 30.0));
    assert!(
        named(&one_a, "T-n ARM")
            .targets
            .contains(&id(&one_a, "ZA4"))
    );
    assert_observation(&one_a, id(&one_a, "ZA4"), "ZA4 fired");

    for (n, start_km, centre_km) in [
        (1, (120.0, 80.0), (130.0, 80.0)),
        (2, (120.0, 50.0), (130.0, 50.0)),
        (3, (120.0, 53.0), (130.0, 53.0)),
        (4, (120.0, 110.0), (130.0, 110.0)),
    ] {
        let name = format!("G{n}");
        let flight = named(&one_g, &name);
        assert_flight(flight, 2, world(start_km), world(centre_km));
        assert_at(
            &one_g,
            &format!("{name} START"),
            60.0,
            &[id(flight, "MISSION BEGIN")],
        );
        let entities = block_ids(flight, "MCU_TR_Entity");
        assert_eq!(
            named(&one_g, &format!("{name} CLOSE DELETE")).objects,
            entities
        );
        for (i, entity_id) in entities.into_iter().enumerate() {
            let entity = by_id(&one_g, entity_id);
            let label = format!("G{n}{}", wingman_letter(i));
            let events = entity
                .children
                .iter()
                .find(|c| c.block_type == "OnEvents")
                .unwrap();
            assert_eq!(
                events.children.len(),
                6,
                "five per-type observations + ev4 readout"
            );
            for event_type in [0, 2, 4, 5, 13] {
                let text = format!("{label} ev{event_type}");
                assert!(
                    events
                        .children
                        .iter()
                        .any(|e| number(e, "Type") == f64::from(event_type)
                            && number(e, "TarId") == f64::from(id(&one_g, &text)))
                );
            }
            let read_name = format!("READ {label} ev4 2+");
            assert_counter(&one_g, &read_name, 2, 0);
            let read = named(&one_g, &read_name);
            assert_eq!(
                read.targets,
                [id(&one_g, &format!("{label} ev4 2+"))]
            );
            assert!(events.children.iter().any(|e| {
                number(e, "Type") == 4.0
                    && number(e, "TarId") == f64::from(read.index.unwrap())
            }));
        }
    }
    let g1_pos = named(named(&one_g, "G1"), "WP 1").pos_xz().unwrap();
    let g2_pos = named(named(&one_g, "G2"), "WP 1").pos_xz().unwrap();
    assert_near(g1_pos.0, g2_pos.0);
    assert_near(g1_pos.1 - g2_pos.1, 30_000.0);
    assert_eq!(named(&one_g, "T-g G1").block_type, "MCU_Icon");
    assert_eq!(named(&one_g, "T-g G1").pos_xz(), Some(g1_pos));
    assert_eq!(named(&one_g, "T-g G1").property("Coalitions"), Some("[1]"));
    assert_at(&one_g, "G2 DELETE AT", 600.0, &[id(&one_g, "G2 DELETE")]);
    assert_eq!(
        named(&one_g, "G2 DELETE").objects,
        block_ids(named(&one_g, "G2"), "MCU_TR_Entity")
    );
    let g3 = named(&one_g, "G3");
    assert_at(
        &one_g,
        "G3 DEACTIVATE AT",
        600.0,
        &[id(g3, "Deactivate Units"), id(g3, "DELETE DELAY")],
    );
    assert_eq!(number(named(g3, "DELETE DELAY"), "Time"), 0.5);
    assert_eq!(
        named(g3, "DELETE DELAY").targets,
        [id(g3, "Trigger Delete")]
    );
    assert_eq!(
        named(g3, "Deactivate Units").objects,
        block_ids(g3, "MCU_TR_Entity")
    );
    assert_eq!(named(g3, "Trigger Delete").objects, block_ids(g3, "Plane"));

    let hunters = named(&one_g, "G1 HUNTER");
    assert_flight_country(
        hunters,
        2,
        world((115.0, 80.0)),
        world((130.0, 80.0)),
        COUNTRY_DPRK,
    );
    assert_at(
        &one_g,
        "G1 HUNTER START",
        60.0,
        &[id(hunters, "MISSION BEGIN")],
    );
    for plane in block_ids(hunters, "Plane") {
        assert_eq!(by_id(hunters, plane).property("AILevel"), Some("4"));
    }
    assert_eq!(
        named(hunters, "WP 1").targets,
        [id(hunters, "AttackArea 1")]
    );
    let template_attack = named(hunters, "AttackArea 1");
    assert_eq!(template_attack.block_type, "MCU_Timer");
    assert_eq!(number(template_attack, "Time"), 0.5);
    assert_eq!(
        template_attack.targets,
        [id(hunters, "AttackArea")]
    );
    assert_eq!(named(hunters, "AttackArea").property("AttackGTargets"), Some("1"));
    let attack = named(&one_g, "G1 HUNTER ATTACK");
    assert_eq!(attack.block_type, "MCU_CMD_AttackTarget");
    assert_eq!(attack.property("AttackGroup"), Some("1"));
    assert_eq!(attack.property("Priority"), Some("1"));
    assert_eq!(attack.objects, [block_ids(hunters, "MCU_TR_Entity")[0]]);
    assert_eq!(attack.targets, [id(&one_g, "G1a entity")]);
    let mut attack_timers = Vec::new();
    for (name, time_s) in [
        ("G1 HUNTER ATTACK AT 1", 90.0),
        ("G1 HUNTER ATTACK AT 2", 180.0),
        ("G1 HUNTER ATTACK AT 3", 270.0),
        ("G1 HUNTER ATTACK AT 4", 360.0),
    ] {
        assert_at(&one_g, name, time_s, &[attack.index.unwrap()]);
        attack_timers.push(id(&one_g, name));
    }
    assert_eq!(
        block_ids(&one_g, "MCU_Timer")
            .into_iter()
            .filter(|&timer| by_id(&one_g, timer).targets.contains(&attack.index.unwrap()))
            .collect::<Vec<_>>(),
        attack_timers
    );
    assert_eq!(one_g.count_block_type("MCU_CMD_AttackTarget"), 1);
    assert_eq!(one_g.count_block_type("MCU_CMD_Damage"), 2);
    for label in ["G4a", "G4b"] {
        let name = format!("{label} DAMAGE");
        let damage = named(&one_g, &name);
        assert_eq!(damage.block_type, "MCU_CMD_Damage");
        assert_eq!(damage.property("Damage"), Some("2"));
        assert_eq!(damage.property("Type"), Some("1"));
        assert_eq!(damage.objects, [id(&one_g, &format!("{label} entity"))]);
        assert!(damage.targets.is_empty());
        assert_at(
            &one_g,
            &format!("{name} AT"),
            240.0,
            &[damage.index.unwrap()],
        );
    }

    assert!(one_n.find_by_name("ZA4").is_none());
    let n2 = named(&one_n, "T-n2");
    assert_eq!(n2.count_block_type("MCU_CheckZone"), 2);
    for (name, km) in [("T-n START", (30.0, 30.0)), ("T-n END", (54.0, 30.0))] {
        let zone = id(&one_n, &format!("{name} ZONE"));
        assert_zone(&one_n, &format!("{name} ZONE"), 5000.0, true, "[2]", km);
        assert!(named(&one_n, "T-n2 ARM").targets.contains(&zone));
        let off = named(&one_n, &format!("{name} OFF"));
        assert_eq!(off.block_type, "MCU_Deactivate");
        assert_eq!(off.targets, [zone]);
        assert!(by_id(&one_n, zone).targets.contains(&off.index.unwrap()));
        assert_observation(&one_n, zone, name);
    }
    let start = named(&one_n, "T-n START ZONE").pos_xz().unwrap();
    let end = named(&one_n, "T-n END ZONE").pos_xz().unwrap();
    assert_near(end.0 - start.0, 24_000.0);
    assert_near(end.1 - start.1, 0.0);

    let reference = reference_block("MCU_TR_ComplexTrigger");
    for (root, prefix, cell, centre, player_country, ai_country, coalition) in [
        (&one_n, "YN", "Y-N", (100.0, 30.0), COUNTRY_USA, COUNTRY_NATO_AI, "[2]"),
        (&one_d, "YD", "Y-D", (140.0, -30.0), COUNTRY_USSR, COUNTRY_DPRK, "[1]"),
    ] {
        assert_eq!(root.count_block_type("MCU_TR_ComplexTrigger"), 2);
        let group = named(root, cell);
        assert_eq!(group.count_block_type("MCU_CheckZone"), 5); // three controls + two AI Zone Outs
        for (suffix, filter) in [("CT", Some(player_country)), ("CT0", None)] {
            let name = format!("{prefix}-{suffix}");
            let trigger = named(root, &name);
            assert_eq!(trigger.block_type, "MCU_TR_ComplexTrigger");
            assert_eq!(trigger.pos_xz(), Some(world(centre)));
            assert_eq!(
                trigger.property("Country"),
                filter.map(|value| value.to_string()).as_deref()
            );
            assert!(trigger.property("ObjectName").is_none());
            assert!(trigger.property("ObjectScript").is_none());
            assert!(trigger.targets.is_empty());
            assert!(trigger.objects.is_empty());
            for (key, value) in [
                ("Enabled", "1"),
                ("Cylinder", "1"),
                ("Radius", "10000"),
                ("DamageThreshold", "1"),
                ("DamageReport", "50"),
                ("CheckVehicles", "0"),
                ("CheckPlanes", "1"),
            ] {
                assert_eq!(trigger.property(key), Some(value));
            }
            let filters: Vec<_> = trigger
                .properties
                .iter()
                .filter(|(key, _)| key.starts_with("EventsFilter"))
                .collect();
            assert_eq!(filters.len(), 21);
            assert_eq!(
                filters,
                reference
                    .properties
                    .iter()
                    .filter(|(key, _)| key.starts_with("EventsFilter"))
                    .collect::<Vec<_>>()
            );
            for (key, value) in filters {
                assert_eq!(
                    value,
                    if matches!(
                        key.as_str(),
                        "EventsFilterEnteredSimple" | "EventsFilterLeftSimple"
                    ) {
                        "1"
                    } else {
                        "0"
                    }
                );
            }
            let trigger_id = trigger.index.unwrap();
            assert!(root.find_by_name(&format!("{name} ON")).is_none());
            for activate in block_ids(root, "MCU_Activate") {
                assert!(
                    !by_id(root, activate).targets.contains(&trigger_id),
                    "{name} must be live without activation"
                );
            }
            let deactivators: Vec<_> = block_ids(root, "MCU_Deactivate")
                .into_iter()
                .filter(|&off| by_id(root, off).targets.contains(&trigger_id))
                .collect();
            assert_eq!(
                deactivators,
                [id(root, &format!("{cell} CLOSE OFF"))],
                "{name} must stay live until cell close"
            );
            let events = &trigger.children[0];
            assert_eq!(events.block_type, "OnEvents");
            assert_eq!(events.children.len(), 2);
            for (event_type, verb, suffix) in [
                (58, "entered", "ENTERED"),
                (60, "left", "LEFT"),
            ] {
                let text = format!("{name} {verb}");
                let relay = named(root, &format!("{name} {suffix}"));
                let relay_id = relay.index.unwrap();
                assert_eq!(relay.block_type, "MCU_Timer");
                assert_eq!(number(relay, "Time"), 0.0);
                assert_eq!(relay.property("Random"), Some("100"));
                assert_eq!(
                    events
                        .children
                        .iter()
                        .filter(|event| {
                            event.block_type == "OnEvent"
                                && number(event, "Type") == f64::from(event_type)
                                && number(event, "TarId") == f64::from(relay_id)
                        })
                        .count(),
                    1
                );
                assert_observation(root, relay_id, &text);
                let mut targets = vec![id(root, &text)];
                if event_type == 58 {
                    for threshold in [2, 3] {
                        let read_text = format!("{text} {threshold}+");
                        assert_readout(root, relay_id, &read_text, threshold);
                        targets.push(id(root, &format!("READ {read_text}")));
                    }
                }
                assert_eq!(relay.targets, targets);
            }
        }
        for (n, time_s) in [(1, 10.0), (2, 325.0), (3, 640.0)] {
            let name = format!("{prefix}-Z{n}");
            assert_zone(root, &name, 10_000.0, true, coalition, centre);
            assert_at(root, &format!("{name} PULSE"), time_s, &[id(root, &name)]);
            assert_observation(root, id(root, &name), &format!("{name} fired"));
        }
        for (suffix, flight_country, start_s, delete_s) in [
            ("A", ai_country, 20.0, 320.0),
            ("B", player_country, 330.0, 630.0),
        ] {
            let name = format!("{prefix}-{suffix}");
            let flight = named(root, &name);
            assert_flight_country(
                flight,
                2,
                world((centre.0 - 30.0, centre.1)),
                world((centre.0 + 30.0, centre.1)),
                flight_country,
            );
            assert_at(
                root,
                &format!("{name} START"),
                start_s,
                &[id(flight, "MISSION BEGIN")],
            );
            let delete = named(root, &format!("{name} CLOSE DELETE"));
            assert_eq!(delete.block_type, "MCU_Delete");
            assert_eq!(delete.objects, block_ids(flight, "MCU_TR_Entity"));
            assert_at(
                root,
                &format!("{name} DELETE AT"),
                delete_s,
                &[delete.index.unwrap()],
            );
            assert!(
                named(root, &format!("{cell} CLOSE"))
                    .targets
                    .contains(&delete.index.unwrap())
            );
        }
    }
}

#[test]
fn probe_run_sheets_match_the_tables() {
    assert_eq!(K14_ORIGIN, (105_934.0, 269_477.0));
    let airfield =
        crate::parser::parse_il2_document(include_str!("../../TemplateExamples/K14 AFB_mp.Group"))
            .unwrap();
    assert_eq!(airfield.count_block_type("Airfield"), 1);
    assert_eq!(named(&airfield, "K-14_Kimpo_AF").pos_xz(), Some(K14_ORIGIN));
    let expected_cells = [
        (Flight::Zero, "T-a1", (0.0, 60.0), 5.0, 360.0),
        (Flight::Zero, "T-e", (0.0, 5.0), 60.0, 150.0),
        (Flight::Zero, "T-f", (0.0, 5.0), 150.0, 540.0),
        (Flight::Zero, "T-t", (0.0, 5.0), 270.0, 330.0),
        (Flight::Zero, "T-j", (0.0, 5.0), 540.0, 600.0),
        (Flight::Zero, "T-z", (-40.0, -40.0), 600.0, 630.0),
        (Flight::OneA, "T-c", (40.0, -20.0), 600.0, 840.0),
        (Flight::OneA, "T-n", (42.0, 30.0), 840.0, 1560.0),
        (Flight::OneN, "T-n2", (42.0, 30.0), 30.0, 1200.0),
        (Flight::OneN, "Y-N", (100.0, 30.0), 10.0, 1200.0),
        (Flight::OneD, "Y-D", (140.0, -30.0), 10.0, 1200.0),
        (Flight::OneG, "T-g2", (130.0, 50.0), 60.0, 900.0),
        (Flight::OneH, "T-d", (130.0, 20.0), 60.0, 480.0),
        (Flight::OneJ, "T-d2", (130.0, 20.0), 60.0, 480.0),
        (Flight::OneK, "T-d3", (130.0, 20.0), 60.0, 1200.0),
        (Flight::OneK, "T-d4", (60.0, 20.0), 60.0, 1200.0),
        (Flight::OneL, "T-r", (130.0, 20.0), 60.0, 1560.0),
        (Flight::OneL, "T-v", (130.0, 52.0), 60.0, 1560.0),
        (Flight::OneL, "T-w", (90.0, 20.0), 60.0, 1560.0),
        (Flight::OneM, "T-u", (130.0, 20.0), 60.0, 960.0),
        (Flight::OneM, "T-x", (10.0, 60.0), 60.0, 960.0),
        (Flight::OneM, "T-y", (50.0, 20.0), 60.0, 960.0),
        (Flight::OneM, "T-j", (0.0, 5.0), 540.0, 600.0),
        (Flight::OneP, "T-h", (50.0, 20.0), 60.0, 960.0),
        (Flight::OneP, "T-f", (90.0, 20.0), 60.0, 960.0),
        (Flight::OneP, "T-b", (10.0, 20.0), 60.0, 960.0),
        (Flight::Two, "T-s1", (50.0, 20.0), 60.0, 2760.0),
        (Flight::Two, "T-s2", (50.0, 60.0), 90.0, 2760.0),
        (Flight::Two, "T-k", (0.0, 5.0), 180.0, 220.0),
        (Flight::Two, "T-k1", (0.0, 5.0), 240.0, 280.0),
        (Flight::Two, "T-l", (-25.0, 50.0), 60.0, 2760.0),
        (Flight::TwoB, "T-s1", (140.0, -90.0), 60.0, 1800.0),
        (Flight::TwoB, "T-s2", (140.0, 0.0), 90.0, 1800.0),
        (Flight::TwoB, "T-s3", (230.0, -90.0), 120.0, 1800.0),
        (Flight::TwoB, "T-s4", (230.0, 0.0), 150.0, 1800.0),
        (Flight::TwoC, "C0a", (140.0, -90.0), 60.0, 1800.0),
        (Flight::TwoC, "C0b", (185.0, 45.0), 65.0, 1800.0),
        (Flight::TwoC, "C0c", (230.0, 0.0), 70.0, 1800.0),
        (Flight::TwoC, "C1a", (140.0, -45.0), 75.0, 1800.0),
        (Flight::TwoC, "C1b", (185.0, -90.0), 80.0, 1800.0),
        (Flight::TwoC, "C1c", (230.0, 45.0), 85.0, 1800.0),
        (Flight::TwoC, "C2a", (140.0, 0.0), 90.0, 1800.0),
        (Flight::TwoC, "C2b", (185.0, -45.0), 95.0, 1800.0),
        (Flight::TwoC, "C2c", (230.0, -90.0), 100.0, 1800.0),
        (Flight::TwoC, "C3a", (140.0, 45.0), 105.0, 1800.0),
        (Flight::TwoC, "C3b", (185.0, 0.0), 110.0, 1800.0),
        (Flight::TwoC, "C3c", (230.0, -45.0), 115.0, 1800.0),
        (Flight::TwoD, "D0", (185.0, -90.0), 60.0, 1800.0),
        (Flight::TwoD, "D1", (230.0, -90.0), 120.0, 1800.0),
        (Flight::TwoD, "D2", (185.0, -45.0), 180.0, 1800.0),
        (Flight::TwoD, "DP", (230.0, 45.0), 240.0, 1800.0),
        (Flight::TwoE, "EA", (230.0, -90.0), 60.0, 1800.0),
        (Flight::TwoE, "EY", (230.0, 0.0), 120.0, 1800.0),
        (Flight::TwoE, "EI", (185.0, 45.0), 180.0, 1800.0),
        (Flight::TwoE, "EC", (140.0, -90.0), 240.0, 1800.0),
        (Flight::TwoF, "PD", (230.0, 0.0), 60.0, 2700.0),
        (Flight::TwoF, "PS", (140.0, 0.0), 390.0, 2700.0),
        (Flight::TwoF, "PL", (230.0, 0.0), 720.0, 2700.0),
        (Flight::TwoF, "WD", (140.0, 0.0), 1050.0, 2700.0),
        (Flight::TwoF, "WS", (230.0, 0.0), 1380.0, 2700.0),
        (Flight::TwoF, "WL", (140.0, 0.0), 1710.0, 2700.0),
    ];
    let expected_cues = [
        (
            Flight::Zero,
            0.0,
            "Flight 0: stay within 20 km of K14 until 11:00",
        ),
        (Flight::Zero, 60.0, "T-e start"),
        (Flight::Zero, 150.0, "T-f F1 start"),
        (
            Flight::Zero,
            270.0,
            "T-t start: note any objective message or map marker in the next 60 s",
        ),
        (Flight::Zero, 330.0, "T-f F2 start"),
        (Flight::Zero, 540.0, "T-j start"),
        (Flight::Zero, 600.0, "T-z start"),
        (
            Flight::Zero,
            660.0,
            "Flight 0 done. End the mission normally",
        ),
        (
            Flight::OneA,
            0.0,
            "T-c: take off, fly to the T-c C icon, orbit within 5 km at 3050 m by 10:00",
        ),
        (Flight::OneA, 720.0, "T-c: fly east into C4 now"),
        (Flight::OneA, 840.0, "T-c done"),
        (
            Flight::OneA,
            840.0,
            "T-n: fly to T-n START, then north through START and END, level at 3050 m, F-80C cruise setting",
        ),
        (
            Flight::OneA,
            1560.0,
            "Flight 1A done. End the mission normally",
        ),
        (
            Flight::OneN,
            0.0,
            "1N: join NATO at the 1N NATO spawn at 04:00 or later. Level flight on at once, hold north at 3050 m, F-80C cruise setting",
        ),
        (
            Flight::OneN,
            1200.0,
            "Flight 1N done. End the mission normally",
        ),
        (
            Flight::OneD,
            0.0,
            "1D: join the 1D USSR spawn at 04:00 or later. Level flight on at once, hold north at 3050 m",
        ),
        (
            Flight::OneD,
            1200.0,
            "Flight 1D done. End the mission normally",
        ),
        (
            Flight::OneG,
            0.0,
            "1G: no flying needed. Stay on the ground at K14 until 15:30",
        ),
        (
            Flight::OneG,
            930.0,
            "Flight 1G done. End the mission normally",
        ),
        (
            Flight::OneH,
            0.0,
            "1H: no flying needed. Stay on the ground at K14 until 05:30",
        ),
        (
            Flight::OneH,
            330.0,
            "Flight 1H done. End the mission normally",
        ),
        (
            Flight::OneJ,
            0.0,
            "1J: no flying needed. Stay on the ground at K14 until 05:30",
        ),
        (
            Flight::OneJ,
            330.0,
            "Flight 1J done. End the mission normally",
        ),
        (
            Flight::OneK,
            0.0,
            "1K: no flying needed. Stay on the ground at K14 until 16:00",
        ),
        (
            Flight::OneK,
            960.0,
            "Flight 1K done. End the mission normally",
        ),
        (
            Flight::OneL,
            0.0,
            "1L: no flying needed. Stay on the ground at K14 until 24:00",
        ),
        (
            Flight::OneL,
            1440.0,
            "Flight 1L done. End the mission normally",
        ),
        (
            Flight::OneM,
            0.0,
            "1M: no flying needed. Stay on the ground at K14 until 14:00",
        ),
        (
            Flight::OneM,
            840.0,
            "Flight 1M done. End the mission normally",
        ),
        (
            Flight::OneP,
            0.0,
            "1P: no flying needed. Stay on the ground at K14 until 14:00",
        ),
        (
            Flight::OneP,
            840.0,
            "Flight 1P done. End the mission normally",
        ),
        (Flight::Two, 0.0, "Flight 2: stay on the ground at K14 until 46:00. Do not damage the AI"),
        (Flight::Two, 180.0, "T-k: seven requests, budget 2"),
        (Flight::Two, 240.0, "T-k1: simultaneous requests, budget 1"),
        (Flight::Two, 600.0, "T-s1 and T-s2 areas now HOT"),
        (Flight::Two, 2760.0, "Flight 2 closed. End the mission normally"),
        (Flight::TwoB, 0.0, "Flight 2B: stay at K14 until 30:00. Do not approach the four AI cells"),
        (Flight::TwoB, 30.0, "T-s3 enemy pair activated on its high altitude orbit"),
        (Flight::TwoB, 60.0, "T-s1 HVAR starts. Empty target, HOT at 10:00, expect one extension"),
        (Flight::TwoB, 90.0, "T-s2 AirAlert starts. Empty target stays COLD, expect 1080 s on station"),
        (Flight::TwoB, 120.0, "T-s3 HVAR starts. Enemy already inside, delete at 09:46"),
        (Flight::TwoB, 150.0, "T-s4 HVAR starts. Enemy appears at 09:16, delete at 12:16"),
        (Flight::TwoB, 556.0, "T-s4 enemy pair activated. Expect Zone IN now"),
        (Flight::TwoB, 586.0, "T-s3 enemy pair deleted. Expect Zone Out then DONE HUB"),
        (Flight::TwoB, 600.0, "T-s1 area now HOT. T-s2, T-s3 and T-s4 stay COLD"),
        (Flight::TwoB, 736.0, "T-s4 enemy pair deleted. Expect Zone Out then DONE HUB"),
        (Flight::TwoB, 1800.0, "Flight 2B closed. Every aircraft has a Delete. End the mission normally"),
        (Flight::TwoC, 0.0, "Flight 2C: spectate if you can. Camera on C0a, then LCtrl+R to record. Leave the server at 15:00"),
        (Flight::TwoC, 870.0, "Flight 2C: leave the server now (15:00)"),
        (Flight::TwoC, 1800.0, "Flight 2C closed. Every aircraft has a Delete. End the mission normally"),
        (Flight::TwoD, 0.0, "Flight 2D: spectate, then LCtrl+R to record. Note the first time the server overload message shows. Leave the server at 15:00"),
        (Flight::TwoD, 60.0, "Flight 2D: D0 starts (4 aircraft flying)"),
        (Flight::TwoD, 120.0, "Flight 2D: D1 starts (8 aircraft flying)"),
        (Flight::TwoD, 180.0, "Flight 2D: D2 starts (12 aircraft flying)"),
        (Flight::TwoD, 240.0, "Flight 2D: DP starts (16 aircraft flying)"),
        (Flight::TwoD, 870.0, "Flight 2D: leave the server now (15:00)"),
        (Flight::TwoD, 1800.0, "Flight 2D closed. Every aircraft has a Delete. End the mission normally"),
        (Flight::TwoE, 0.0, "Flight 2E: spectate, then LCtrl+R to record. Note the first time the server overload message shows. Leave the server at 15:00"),
        (Flight::TwoE, 60.0, "Flight 2E: EA starts (4 aircraft flying)"),
        (Flight::TwoE, 120.0, "Flight 2E: EY starts (8 aircraft flying)"),
        (Flight::TwoE, 180.0, "Flight 2E: EI starts (16 aircraft flying)"),
        (Flight::TwoE, 240.0, "Flight 2E: EC starts (20 aircraft flying)"),
        (Flight::TwoE, 870.0, "Flight 2E: leave the server now (15:00)"),
        (Flight::TwoE, 1800.0, "Flight 2E closed. Every aircraft has a Delete. End the mission normally"),
        (Flight::TwoF, 0.0, "Flight 2F: spectate, then LCtrl+R to record. Check the menu now and then for the server overload message. Leave the server at 38:30"),
        (Flight::TwoF, 60.0, "Flight 2F: PD starts (Pairs, Dense)"),
        (Flight::TwoF, 390.0, "Flight 2F: PS starts (Pairs, Safe)"),
        (Flight::TwoF, 720.0, "Flight 2F: PL starts (Pairs, Loose)"),
        (Flight::TwoF, 1050.0, "Flight 2F: WD starts (Wedge, Dense)"),
        (Flight::TwoF, 1380.0, "Flight 2F: WS starts (Wedge, Safe)"),
        (Flight::TwoF, 1710.0, "Flight 2F: WL starts (Wedge, Loose)"),
        (Flight::TwoF, 2280.0, "Flight 2F: leave the server at 38:30"),
        (Flight::TwoF, 2700.0, "Flight 2F closed. Every aircraft has a Delete. End the mission normally"),
    ];
    assert_eq!(CUES.len(), expected_cues.len());
    for (&cue, &(flight, time_s, text)) in CUES.iter().zip(&expected_cues) {
        assert_eq!(
            cue,
            Cue {
                flight,
                time_s,
                text
            }
        );
    }
    for flight in Flight::ALL {
        let root = build(flight).root;
        let expected: Vec<_> = expected_cells.iter().filter(|c| c.0 == flight).collect();
        assert_eq!(flight.sheet().len(), expected.len());
        for (&cell, &&(_, key, km, arm_s, close_s)) in flight.sheet().iter().zip(&expected) {
            assert_eq!(
                cell,
                Cell {
                    key,
                    centre_km: km,
                    arm_s,
                    close_s
                }
            );
            let group = named(&root, key);
            assert_eq!(group.block_type, "Group");
            assert_eq!(group.pos_xz(), Some(world(km)));
            let arm_name = format!("{key} ARM");
            assert_at(&root, &arm_name, arm_s, &named(&root, &arm_name).targets);
            let close_name = format!("{key} CLOSE");
            let close = named(&root, &close_name);
            assert_at(&root, &close_name, close_s, &close.targets);
            let off = named(&root, &format!("{key} CLOSE OFF"));
            assert_eq!(off.block_type, "MCU_Deactivate");
            assert!(close.targets.contains(&off.index.unwrap()));
            if matches!(key, "Y-N" | "Y-D") {
                root.for_each(&mut |node| {
                    if node.targets.contains(&off.index.unwrap()) {
                        assert_eq!(
                            node.index, close.index,
                            "{key} Complex Triggers must only be deactivated at cell close"
                        );
                    }
                });
            }
            for zone in block_ids(group, "MCU_CheckZone")
                .into_iter()
                .chain(block_ids(group, "MCU_TR_ComplexTrigger"))
            {
                assert!(
                    off.targets.contains(&zone),
                    "{key} leaves check zone {zone} open"
                );
            }
            // Every flight closes through its own Delete as well, even when
            // Y pairs or G2/G3 have already been removed earlier.
            let mut deleted = Vec::new();
            for &delete in close
                .targets
                .iter()
                .filter(|&&i| by_id(&root, i).block_type == "MCU_Delete")
            {
                assert!(!by_id(&root, delete).objects.is_empty());
                deleted.extend_from_slice(&by_id(&root, delete).objects);
            }
            deleted.sort_unstable();
            let mut entities = block_ids(group, "Plane")
                .into_iter()
                .map(|plane| number(by_id(group, plane), "LinkTrId") as i32)
                .collect::<Vec<_>>();
            entities.sort_unstable();
            assert_eq!(
                deleted, entities,
                "{key}: close must delete every aircraft exactly once"
            );
        }
        for (i, &(_, time_s, text)) in expected_cues
            .iter()
            .enumerate()
            .filter(|(_, c)| c.0 == flight)
        {
            let subtitle = named(&root, text);
            assert_eq!(subtitle.block_type, "MCU_TR_Subtitle");
            assert_at(
                &root,
                &format!("CUE {:02}", i + 1),
                time_s,
                &[subtitle.index.unwrap()],
            );
        }
    }
    let one_a = generate_probe_1a();
    let one_n = generate_probe_1n();
    let one_d = generate_probe_1d();
    let one_g = generate_probe_1g();
    let one_h = generate_probe_1h();
    let one_j = generate_probe_1j();
    let one_k = generate_probe_1k();
    let one_l = generate_probe_1l();
    for (root, name, km, coalition) in [
        (&one_a, "T-c C", (40.0, -20.0), "[2]"),
        (&one_a, "T-c C4", (40.0, -8.0), "[2]"),
        (&one_a, "T-n START", (30.0, 30.0), "[2]"),
        (&one_a, "T-n END", (54.0, 30.0), "[2]"),
        (&one_g, "T-g G1", (130.0, 80.0), "[1]"),
        (&one_n, "T-n START", (30.0, 30.0), "[2]"),
        (&one_n, "T-n END", (54.0, 30.0), "[2]"),
        (&one_n, "Y-N", (100.0, 30.0), "[2]"),
        (&one_d, "Y-D", (140.0, -30.0), "[1]"),
        (&one_g, "T-g G2", (130.0, 50.0), "[1]"),
        (&one_g, "T-g G3", (130.0, 53.0), "[1]"),
        (&one_g, "T-g G4", (130.0, 110.0), "[1]"),
        (&one_h, "T-d D0", (130.0, 20.0), "[2]"),
        (&one_h, "T-d D1", (130.0, 28.0), "[2]"),
        (&one_h, "T-d D2", (130.0, 36.0), "[2]"),
        (&one_h, "T-d D3", (130.0, 44.0), "[2]"),
        (&one_h, "T-d D4", (130.0, 52.0), "[2]"),
        (&one_h, "T-d D5", (130.0, 60.0), "[2]"),
        (&one_h, "T-d D6", (130.0, 68.0), "[2]"),
        (&one_h, "T-d D7", (130.0, 76.0), "[2]"),
        (&one_h, "T-d D8", (114.0, 20.0), "[2]"),
        (&one_h, "T-d D9", (114.0, 28.0), "[2]"),
        (&one_j, "T-d2 J0", (130.0, 20.0), "[1]"),
        (&one_j, "T-d2 J1", (130.0, 28.0), "[1]"),
        (&one_j, "T-d2 J2", (130.0, 36.0), "[1]"),
        (&one_j, "T-d2 J3", (130.0, 44.0), "[1]"),
        (&one_j, "T-d2 J4", (130.0, 52.0), "[1]"),
        (&one_j, "T-d2 J5", (130.0, 60.0), "[1]"),
        (&one_j, "T-d2 J6", (130.0, 68.0), "[1]"),
        (&one_k, "T-d3 K0", (130.0, 20.0), "[2]"),
        (&one_k, "T-d3 K1", (130.0, 28.0), "[2]"),
        (&one_k, "T-d3 K2", (130.0, 36.0), "[2]"),
        (&one_k, "T-d3 K3", (130.0, 44.0), "[2]"),
        (&one_k, "T-d3 K4", (130.0, 52.0), "[2]"),
        (&one_k, "T-d3 K5", (130.0, 60.0), "[2]"),
        (&one_k, "T-d3 K6", (130.0, 68.0), "[2]"),
        (&one_k, "T-d3 K7", (130.0, 76.0), "[2]"),
        (&one_k, "T-d4 K8", (60.0, 20.0), "[2]"),
        (&one_k, "T-d4 K9", (60.0, 60.0), "[2]"),
        (&one_k, "T-d4 K10", (60.0, 100.0), "[2]"),
        (&one_l, "T-r R0", (130.0, 20.0), "[2]"),
        (&one_l, "T-r R1", (130.0, 28.0), "[2]"),
        (&one_l, "T-r R2", (130.0, 36.0), "[2]"),
        (&one_l, "T-r R3", (130.0, 44.0), "[2]"),
        (&one_l, "T-v V1", (130.0, 52.0), "[2]"),
        (&one_l, "T-v V2", (130.0, 60.0), "[2]"),
        (&one_l, "T-v V3", (130.0, 68.0), "[2]"),
        (&one_l, "T-w W1", (90.0, 20.0), "[2]"),
        (&one_l, "T-w W2", (90.0, 28.0), "[2]"),
        (&one_l, "T-w W3", (90.0, 36.0), "[2]"),
        (&one_l, "T-w W4", (90.0, 44.0), "[2]"),
        (&one_l, "T-w W5", (90.0, 52.0), "[2]"),
        (&one_l, "T-w W6", (90.0, 60.0), "[2]"),
    ] {
        let mut found = 0;
        root.for_each(&mut |node| {
            if node.block_type == "MCU_Icon" && node.name() == Some(name) {
                found += 1;
                assert_eq!(node.pos_xz(), Some(world(km)));
                assert_eq!(node.property("Coalitions"), Some(coalition));
                assert_eq!(node.property("Enabled"), Some("1"));
                assert_eq!(node.property("IconId"), Some("501"));
            }
        });
        assert_eq!(found, 1, "run-sheet icon {name}");
    }
    assert!(
        named(named(&one_a, "T-n"), "T-n CLOSE OFF")
            .targets
            .contains(&id(&one_a, "ZA4"))
    );
    assert!(
        !named(&one_a, "T-c CLOSE OFF")
            .targets
            .contains(&id(&one_a, "ZA4"))
    );
}

#[test]
fn probe_spawn_lists_match_the_run_sheets() {
    for flight in [Flight::Zero, Flight::OneA, Flight::OneG, Flight::OneH, Flight::OneJ, Flight::OneK, Flight::OneL, Flight::OneM, Flight::OneP, Flight::Two, Flight::TwoB, Flight::TwoD, Flight::TwoE, Flight::TwoF, Flight::Load] {
        assert!(flight.spawns().is_empty());
        assert_eq!(spawn_json(flight), "[]\n");
    }
    for (flight, name, country, km, script, expected_json) in [
        (
            Flight::OneN,
            "1N NATO",
            COUNTRY_USA,
            (-10.0, 30.0),
            "f80c10",
            format!("[{{\"name\":\"1N NATO\",\"country\":{COUNTRY_USA},\"x\":95934,\"z\":299477,\"heading_deg\":0,\"altitude_m\":3050,\"script\":\"f80c10\"}}]\n"),
        ),
        (
            Flight::OneD,
            "1D USSR",
            COUNTRY_USSR,
            (40.0, -30.0),
            "mig15bis",
            format!("[{{\"name\":\"1D USSR\",\"country\":{COUNTRY_USSR},\"x\":145934,\"z\":239477,\"heading_deg\":0,\"altitude_m\":3050,\"script\":\"mig15bis\"}}]\n"),
        ),
    ] {
        assert_eq!(
            flight.spawns(),
            &[Spawn {
                name,
                country,
                km,
                heading_deg: 0.0,
                altitude_m: 3050.0,
                script,
            }]
        );
        let (x, z) = world(flight.spawns()[0].km);
        assert_near(x, 105_934.0 + km.0 * 1000.0);
        assert_near(z, 269_477.0 + km.1 * 1000.0);
        assert_eq!(spawn_json(flight), expected_json);
    }
}

#[test]
fn probe_tt_breadcrumbs_match_the_table() {
    let p = build(Flight::Zero);
    let root = &p.root;
    assert_eq!(p.map.entries.len(), 6, "six T-t breadcrumbs");
    let entry_for = |name: &str| {
        let entries: Vec<_> = p
            .map
            .entries
            .iter()
            .filter(|e| e.source_index == id(root, name))
            .collect();
        assert_eq!(entries.len(), 1, "one registered breadcrumb for {name}");
        let entry = entries[0];
        assert_eq!(entry.source_name, name);
        assert_eq!(entry.source_type, "MCU_Timer");
        assert_eq!(entry.event_type, None);
        assert_eq!(entry.group_path, ["P14_Probe_0", "T-t"]);
        entry
    };
    let objective = |name: &str, n: usize, expected_s: Option<f64>| {
        let entry = entry_for(name);
        assert_eq!(
            entry.carrier,
            TraceCarrier::Objective(ObjectiveStyle::default())
        );
        assert_eq!(entry.spawn_name, None);
        assert_eq!(entry.expected_s, expected_s);
        assert_eq!(entry.pos, (5000.0 + 10.0 * n as f64, 5000.0));
        let node = by_id(root, entry.breadcrumb_index);
        assert_eq!(node.block_type, "MCU_TR_MissionObjective");
        assert_eq!(node.pos_xz(), Some(entry.pos));
        for key in [
            "Coalition", "Success", "TaskType", "IconType", "LCName", "LCDesc",
        ] {
            assert_eq!(node.property(key), Some("0"), "{name} {key}");
        }
        assert_eq!(node.property("Enabled"), Some("1"));
        assert!(node.targets.is_empty());
        assert!(node.objects.is_empty());
        entry.breadcrumb_index
    };
    let spawn = |name: &str, n: usize, expected_s: Option<f64>| {
        let entry = entry_for(name);
        assert_eq!(entry.carrier, TraceCarrier::Spawn);
        assert_eq!(entry.spawn_name, Some(format!("TRACE {n}")));
        assert_eq!(entry.expected_s, expected_s);
        let node = by_id(root, entry.breadcrumb_index);
        assert_eq!(node.block_type, "MCU_Spawner");
        assert_eq!(node.name(), entry.spawn_name.as_deref());
        assert_eq!(node.objects.len(), 1);
        let entity = by_id(root, node.objects[0]);
        let vehicle = by_id(root, number(entity, "MisObjID") as i32);
        assert_eq!(vehicle.name(), entry.spawn_name.as_deref());
        assert_eq!(entry.pos, trace::TRACE_SPAWN_POINT);
        assert_eq!(node.pos_xz(), Some(entry.pos));
        entry.breadcrumb_index
    };
    let t1 = objective("T-t T1", 0, Some(275.0));
    assert!(named(root, "T-t T1").targets.contains(&t1));
    assert_at(root, "T-t T1", 275.0, &named(root, "T-t T1").targets);
    let t2 = objective("T-t T2", 1, None);
    assert!(named(root, "T-t T2").targets.contains(&t2));
    assert_pulses(
        root,
        "T-t T2",
        &[277.0, 279.0, 281.0, 283.0, 285.0],
        id(root, "T-t T2"),
    );
    for (name, n) in [("T-t T3a", 2), ("T-t T3b", 3)] {
        let breadcrumb = objective(name, n, Some(287.0));
        assert!(named(root, name).targets.contains(&breadcrumb));
        assert_eq!(number(named(root, name), "Time"), 0.0);
    }
    assert_at(
        root,
        "T-t T3 SAME TICK",
        287.0,
        &[id(root, "T-t T3a"), id(root, "T-t T3b")],
    );
    assert_eq!(root.count_block_type("MCU_TR_MissionObjective"), 5);
    assert_eq!(named(root, "T-t").count_block_type("MCU_Spawner"), 1);
    let t5 = spawn("T-t T5", 4, None);
    assert!(named(root, "T-t T5").targets.contains(&t5));
    assert_pulses(root, "T-t T5", &[315.0, 320.0], id(root, "T-t T5"));
    let t7 = objective("T-t T7 SAME TICK", 5, Some(325.0));
    assert_fan(root, "T-t T7 SAME TICK", 325.0, 2, t7);
    let mut t7_inputs = Vec::new();
    root.for_each(&mut |node| {
        if node.targets.contains(&t7) {
            t7_inputs.push(node.index.unwrap());
        }
    });
    assert_eq!(
        t7_inputs,
        [id(root, "T-t T7 INPUT 1"), id(root, "T-t T7 INPUT 2")]
    );
    let hand_entries = p.map.entries.clone();
    let (traced, map) = p.traced(TraceCarrier::default());
    for entry in &hand_entries {
        assert_eq!(
            map.entries
                .iter()
                .filter(|e| e.source_index == entry.source_index)
                .count(),
            1,
            "hand-built source was instrumented twice"
        );
        assert!(map.entries.contains(entry));
        assert_eq!(
            by_id(&traced, entry.breadcrumb_index).index,
            Some(entry.breadcrumb_index)
        );
    }
    for (i, cue) in CUES
        .iter()
        .enumerate()
        .filter(|(_, c)| c.flight == Flight::Zero)
    {
        let source = id(&traced, &format!("CUE {:02}", i + 1));
        let entries: Vec<_> = map
            .entries
            .iter()
            .filter(|e| e.source_index == source)
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].expected_s, Some(cue.time_s));
        assert_eq!(entries[0].carrier, TraceCarrier::default());
    }
}

#[test]
fn probe_objective_variants_replay_by_position() {
    use crate::missionlog::{TypedRecord, parse_log_dir, replay, report_markdown};

    // T-o is retired from new probes. Its flown sidecar retains the original
    // positions and expected times needed by this synthetic truncation test.
    let root = Il2Entity::new("Group");
    let map = trace::read_trace_sidecar(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/testdata/missionlog/real_2/P14_Probe_0_traced.trace.json"),
    )
    .unwrap();
    let logs = parse_log_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/testdata/missionlog/objective_variants"),
    )
    .unwrap();
    assert!(logs.malformed_lines.is_empty());
    assert_eq!(logs.unknown_records, 0);
    assert_eq!(
        logs.records.iter().map(|r| r.atype).collect::<Vec<_>>(),
        [8, 8, 8, 8, 8, 7]
    );
    assert_eq!(logs.records[5].typed(), TypedRecord::MissionEnd);
    assert_eq!(logs.records[5].t_ticks, 37_900);
    for record in &logs.records[..5] {
        let TypedRecord::MissionObjective(objective) = record.typed() else {
            panic!("expected an objective record");
        };
        assert!(
            map.entries
                .iter()
                .all(|entry| Some(i64::from(entry.breadcrumb_index)) != objective.id)
        );
    }

    let expected = [
        ("T-o O1", 670.0, (5060.0, 0.0, 5000.0)),
        ("T-o O2", 690.0, (5070.0, 0.0, 5000.0)),
        ("T-o O3", 710.0, (5080.0, 0.0, 5000.0)),
        ("T-o O4", 730.0, (5090.0, 0.0, 5000.0)),
        ("T-o O5", 750.0, (5100.0, 0.0, 5000.0)),
    ];
    // Test the complete fixture and a round ending eight seconds after any
    // earlier variant. Only the positions can identify these runtime OBJIDs.
    for count in 1..=expected.len() {
        let mut partial = logs.clone();
        partial.records.truncate(count);
        let mut end = logs.records[5].clone();
        end.t_ticks = partial.records.last().unwrap().t_ticks + 400;
        partial.records.push(end);
        let result = replay(&partial, &root, Some(&map));
        let report = report_markdown(&result);
        let fired: Vec<_> = result
            .firings
            .iter()
            .filter(|f| !f.times_s.is_empty())
            .collect();
        assert_eq!(fired.len(), count);
        for (firing, &(name, time_s, (x, _, z))) in fired.iter().zip(&expected) {
            assert_eq!(firing.entry.source_name, name);
            assert_eq!(
                firing.entry,
                *map.entries
                    .iter()
                    .find(|entry| entry.source_name == name)
                    .unwrap()
            );
            assert_eq!(firing.entry.pos, (x, z));
            assert_eq!(firing.times_s, [time_s]);
        }
        let objectives: Vec<_> = result
            .timeline
            .iter()
            .filter(|event| event.description.starts_with("Objective:"))
            .collect();
        assert_eq!(objectives.len(), count);
        for (event, &(name, time_s, pos)) in objectives.iter().zip(&expected) {
            assert_eq!(event.seconds, time_s);
            assert_eq!(event.pos, Some(pos));
            assert!(
                event
                    .description
                    .starts_with(&format!("Objective: {name} (OBJID "))
            );
            assert!(report.contains(&format!(
                "| {time_s:.3} | {} | Objective: {name} (OBJID ",
                event.t_ticks
            )));
        }
        let end = result.timeline.last().unwrap();
        assert_eq!(end.description, "Mission end");
        assert_eq!(end.seconds, expected[count - 1].1 + 8.0);
        assert!(report.contains(&format!(
            "| {:.3} | {} | Mission end |",
            end.seconds, end.t_ticks
        )));
    }
}

#[test]
fn probe_traced_every_subtitle_source_has_a_breadcrumb() {
    use std::collections::BTreeMap;

    fn links(node: &Il2Entity) -> Vec<(Option<i32>, i32)> {
        let mut links: Vec<_> = node.targets.iter().map(|&target| (None, target)).collect();
        if matches!(
            node.block_type.as_str(),
            "MCU_TR_Entity" | "MCU_TR_ComplexTrigger"
        ) {
            node.for_each(&mut |event| {
                if event.block_type == "OnEvent" {
                    links.push((
                        Some(number(event, "Type") as i32),
                        number(event, "TarId") as i32,
                    ));
                }
            });
        }
        links
    }

    for (flight, subtitle_count, source_count, entity_count, hand_count, objective_count) in [
        (Flight::Zero, 45, 46, 0, 6, 44),
        (Flight::OneA, 17, 17, 0, 0, 17),
        (Flight::OneN, 15, 15, 0, 0, 15),
        (Flight::OneD, 13, 13, 0, 0, 13),
        (Flight::OneG, 50, 18, 8, 0, 50),
        (Flight::OneH, 22, 22, 20, 0, 22),
        (Flight::OneJ, 16, 16, 14, 0, 16),
        (Flight::OneK, 30, 30, 28, 0, 30),
        (Flight::OneL, 78, 39, 23, 0, 78),
        (Flight::OneM, 185, 85, 35, 0, 185),
        (Flight::OneP, 220, 92, 40, 0, 220),
    ] {
        let probe = build(flight);
        let plain = probe.root.clone();
        let hand_entries = probe.map.entries.clone();
        let hand_ids: HashSet<_> = hand_entries.iter().map(|e| e.breadcrumb_index).collect();
        let spawn_ids: HashSet<_> = hand_entries
            .iter()
            .filter(|entry| entry.carrier == TraceCarrier::Spawn)
            .map(|entry| entry.breadcrumb_index)
            .collect();
        let subtitles: HashSet<_> = block_ids(&plain, "MCU_TR_Subtitle").into_iter().collect();
        let mut seen_subtitles = HashSet::new();
        let mut sources = BTreeMap::<i32, BTreeSet<Option<i32>>>::new();
        // Discover observations from the wiring, independently of Probe::sources.
        plain.for_each(&mut |node| {
            for (event_type, target) in links(node) {
                if subtitles.contains(&target) {
                    seen_subtitles.insert(target);
                    sources
                        .entry(node.index.unwrap())
                        .or_default()
                        .insert(event_type);
                }
            }
        });
        assert_eq!(subtitles.len(), subtitle_count, "{flight:?} subtitles");
        assert_eq!(seen_subtitles, subtitles, "{flight:?} orphan subtitle");
        assert_eq!(sources.len(), source_count, "{flight:?} subtitle sources");
        assert_eq!(
            hand_entries.len(),
            hand_count,
            "{flight:?} hand-built breadcrumbs"
        );

        let mut hand_cell_ids = Vec::new();
        if flight == Flight::Zero {
            named(&plain, "T-t").collect_indexes(&mut hand_cell_ids);
        }
        let (traced, map) = probe.traced(TraceCarrier::default());
        super::testkit::assert_links_resolve(&traced);
        if flight == Flight::OneJ {
            let mut aircraft: Vec<_> = map.entries.iter()
                .filter(|entry| entry.source_type == "MCU_TR_Entity")
                .map(|entry| (entry.source_name.as_str(), entry.event_type))
                .collect();
            aircraft.sort_unstable();
            assert_eq!(
                aircraft,
                [
                    "J0x entity", "J0y entity", "J1x entity", "J1y entity",
                    "J2x entity", "J2y entity", "J3x entity", "J3y entity",
                    "J4x entity", "J4y entity", "J5x entity", "J5y entity",
                    "J6x entity", "J6y entity",
                ].map(|name| (name, Some(13))),
                "1J traces exactly the fourteen named aircraft"
            );
        }
        if flight == Flight::OneK {
            let aircraft: BTreeSet<_> = map.entries.iter()
                .filter(|entry| entry.source_type == "MCU_TR_Entity")
                .map(|entry| (entry.source_name.as_str(), entry.event_type))
                .collect();
            assert_eq!(
                aircraft,
                [
                    "K0a entity", "K0b entity", "K1a entity", "K1b entity",
                    "K2a entity", "K2b entity", "K3a entity", "K3b entity",
                    "K4a entity", "K4b entity", "K5a entity", "K5b entity",
                    "K6a entity", "K6b entity", "K7a entity", "K7b entity",
                    "K8a entity", "K8b entity", "K9a entity", "K9b entity",
                    "K10a entity", "K10b entity", "K8Ha entity", "K8Hb entity",
                    "K9Ha entity", "K9Hb entity", "K10Ha entity", "K10Hb entity",
                ].into_iter().map(|name| (name, Some(13))).collect(),
                "1K traces exactly the twenty-eight named aircraft"
            );
        }
        for root in [&plain, &traced] {
            for subtitle in block_ids(root, "MCU_TR_Subtitle") {
                let subtitle = by_id(root, subtitle);
                assert!(
                    subtitle.targets.is_empty(),
                    "{flight:?} {:?}: subtitle must not forward to Targets",
                    subtitle.name()
                );
            }
        }
        for trigger in block_ids(&plain, "MCU_TR_ComplexTrigger") {
            assert_eq!(by_id(&plain, trigger), by_id(&traced, trigger));
        }
        assert_eq!(
            traced.count_block_type("MCU_TR_MissionObjective"),
            objective_count
        );
        for entry in &map.entries {
            if spawn_ids.contains(&entry.breadcrumb_index) {
                assert_eq!(entry.carrier, TraceCarrier::Spawn);
            } else {
                assert_eq!(entry.carrier, TraceCarrier::default());
            }
        }
        // Include T7's upstream timer and both inputs, not just mapped sources.
        assert_eq!(
            map.entries
                .iter()
                .filter(|e| hand_cell_ids.contains(&e.source_index))
                .collect::<Vec<_>>(),
            hand_entries.iter().collect::<Vec<_>>(),
            "{flight:?} T-t must retain only its hand-built entries"
        );
        if flight == Flight::Zero {
            assert_eq!(
                named(&traced, "T-t"),
                named(&plain, "T-t"),
                "instrument changed T-t"
            );
        }

        let mut entity_sources = 0;
        let mut instrumented_count = 0;
        for (&source_id, event_types) in &sources {
            let source = by_id(&traced, source_id);
            let context = format!("{flight:?} {:?} ({source_id})", source.name());
            let mut expected_links = links(by_id(&plain, source_id));
            if hand_cell_ids.contains(&source_id) {
                // T7's two subtitle inputs share the entry registered to its
                // upstream timer; neither input gets an instrumented breadcrumb.
                assert_eq!(
                    expected_links
                        .iter()
                        .filter(|(_, target)| hand_ids.contains(target))
                        .count(),
                    1,
                    "{context}: one hand-built breadcrumb per subtitle source"
                );
            } else {
                if source.block_type == "MCU_TR_Entity" {
                    entity_sources += 1;
                    let expected_types: &[i32] = match flight {
                        Flight::OneG => &[0, 2, 4, 5, 13],
                        Flight::OneH | Flight::OneJ | Flight::OneK => &[13],
                        Flight::OneL if matches!(source.name(), Some("V1 entity" | "V2 entity" | "V3 entity")) =>
                            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                        Flight::OneL => &[13],
                        Flight::OneM if ["U4", "U5"].iter().any(|name|
                            block_ids(named(&plain, name), "MCU_TR_Entity").contains(&source_id)) =>
                            &[1, 3, 5, 7, 8, 13],
                        Flight::OneM if ["U1", "U2", "U3"].iter().any(|name|
                            block_ids(named(&plain, name), "MCU_TR_Entity").contains(&source_id)) =>
                            &[1, 3, 5, 7, 9, 13],
                        Flight::OneM => &[13],
                        Flight::OneP if block_ids(named(&plain, "T-h"), "MCU_TR_Entity").contains(&source_id) =>
                            &[1, 3, 5, 7, 9, 13],
                        Flight::OneP if block_ids(named(&plain, "T-b"), "MCU_TR_Entity").contains(&source_id) =>
                            &[5, 7, 8, 9, 13],
                        Flight::OneP => &[13],
                        _ => panic!("unexpected entity observation in {flight:?}"),
                    };
                    assert_eq!(
                        event_types.iter().copied().collect::<Vec<_>>(),
                        expected_types.iter().copied().map(Some).collect::<Vec<_>>(),
                        "{context}: entity event types"
                    );
                } else {
                    assert_eq!(
                        event_types.iter().copied().collect::<Vec<_>>(),
                        [None],
                        "{context}"
                    );
                }
                let entries: Vec<_> = map
                    .entries
                    .iter()
                    .filter(|e| e.source_index == source_id)
                    .collect();
                assert_eq!(
                    entries.len(),
                    event_types.len(),
                    "{context}: breadcrumb count"
                );
                for &event_type in event_types {
                    let matching: Vec<_> = entries
                        .iter()
                        .copied()
                        .filter(|e| e.event_type == event_type)
                        .collect();
                    assert_eq!(matching.len(), 1, "{context}: event {event_type:?}");
                    let entry = matching[0];
                    assert!(
                        entry.breadcrumb_index > plain.max_index(),
                        "{context}: breadcrumb must come from instrument"
                    );
                    assert_eq!(entry.source_name, source.name().unwrap(), "{context}");
                    assert_eq!(entry.source_type, source.block_type, "{context}");
                    assert_eq!(entry.carrier, TraceCarrier::default(), "{context}");
                    assert_eq!(
                        by_id(&traced, entry.breadcrumb_index).block_type,
                        "MCU_TR_MissionObjective",
                        "{context}: breadcrumb must exist in the tree"
                    );
                    expected_links.push((event_type, entry.breadcrumb_index));
                    instrumented_count += 1;
                }
            }
            // Compare every link, retaining multiplicity: an extra breadcrumb or
            // duplicate Targets/OnEvent link must fail even if absent from the map.
            expected_links.sort_unstable();
            let mut actual_links = links(source);
            actual_links.sort_unstable();
            assert_eq!(actual_links, expected_links, "{context}: tree breadcrumbs");
        }
        assert_eq!(entity_sources, entity_count, "{flight:?} entity sources");
        assert_eq!(
            map.entries.len(),
            hand_count + instrumented_count,
            "{flight:?} unexpected breadcrumb"
        );

        let mut cue_times = Vec::new();
        for cue in CUES.iter().filter(|cue| cue.flight == flight) {
            let subtitle = id(&traced, cue.text);
            let entries: Vec<_> = map
                .entries
                .iter()
                .filter(|entry| {
                    by_id(&traced, entry.source_index)
                        .targets
                        .contains(&subtitle)
                })
                .collect();
            assert_eq!(entries.len(), 1, "{flight:?} cue {:?}", cue.text);
            assert_eq!(entries[0].carrier, TraceCarrier::default());
            assert_eq!(
                entries[0].expected_s,
                Some(cue.time_s),
                "{flight:?} cue {:?}",
                cue.text
            );
            cue_times.push(entries[0].expected_s.unwrap());
        }

        let path = std::env::temp_dir().join(format!(
            "p14_probe_subtitle_breadcrumbs_{}_{}.trace.json",
            std::process::id(),
            flight.stem()
        ));
        trace::write_trace_sidecar(&path, &map).expect("write probe trace sidecar");
        let restored = trace::read_trace_sidecar(&path);
        std::fs::remove_file(&path).expect("remove probe trace sidecar");
        let restored = restored.expect("read probe trace sidecar");
        for entry in &hand_entries {
            assert_eq!(
                restored
                    .entries
                    .iter()
                    .filter(|e| e.breadcrumb_index == entry.breadcrumb_index)
                    .collect::<Vec<_>>(),
                [entry],
                "{flight:?} sidecar lost or duplicated hand-built breadcrumb {}",
                entry.source_name
            );
        }
        assert_eq!(restored, map, "{flight:?} trace sidecar round trip");
        println!(
            "{flight:?}: {} subtitles, {} source MCUs ({} source/event checks), {entity_sources} entity sources, cue expected_s {cue_times:?}, {hand_count} hand-built breadcrumbs",
            subtitles.len(),
            sources.len(),
            sources.values().map(BTreeSet::len).sum::<usize>()
        );
    }
}
