//! Test-only generators for the P14 in-game probes (steps 0b/0e/0f/0g/0h/0i/0j).
//! Flights 0, 1A, 1G, 1N and 1D were flown; their results are recorded in
//! `handoff/P14-probe-results.md`. Committed copies of the flown files are in
//! `handoff/p14-probes/`. Flight 1B was retired by decision 0010.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::sync::OnceLock;

use crate::ast::Il2Entity;
use crate::locale::{P14_PROBE_LC_START, p14_probe_locale};
use crate::parser::parse_group_file;
use crate::serialize::serialize_group;
use crate::template::{
    BringUp, FlightRole, TemplateOptions, TemplateSeat, attach_event, builtin_plane_catalog,
    checkzone, counter, generate_template, mcu, modifier_set_val, timer, timer_random,
};
use crate::trace::{self, ObjectiveStyle, TraceCarrier, TraceMap, TraceSelect, TraceSource};

const HVAR: &str =
    include_str!("../../TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group");
const W_INPUT_DIR: &str = "src/testdata/p14_probe_inputs/flight1l";
const X_INPUT_DIR: &str = "src/testdata/p14_probe_inputs/flight1m";
const U_INPUT_DIR: &str = "src/testdata/p14_probe_inputs/upstream_a31ad0a";
const U_NATO_STRIKE_FILE: &str = "Upstream_Strike_NATO_F80C_4ship.Group";
const U_DPRK_STRIKE_FILE: &str = "Upstream_Strike_DPRK_Il10_4ship.Group";
const U_CAP_FILE: &str = "Upstream_CAP_NATO_F86A_4ship.Group";
const COMPLEX_TRIGGER: &str =
    include_str!("../../TemplateExamples/Reference/ComplexTrigger_reference.Group");
const DAMAGE: &str = include_str!("../../TemplateExamples/Reference/CommandDamage_reference.Group");
const ATTACK_TARGET: &str =
    include_str!("../../TemplateExamples/CorrectedFlightfromTemplate.Group");
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
const STRIKE_HOME_TIMER: &str = "Goto WP 2";
const CAP_HOME_TIMER: &str = "Goto WP 5";

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
}

impl Flight {
    const ALL: [Self; 10] = [
        Self::Zero, Self::OneA, Self::OneN, Self::OneD, Self::OneG, Self::OneH, Self::OneJ,
        Self::OneK, Self::OneL, Self::OneM,
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
        }
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
            Self::Zero | Self::OneA | Self::OneG | Self::OneH | Self::OneJ | Self::OneK
                | Self::OneL | Self::OneM => &[],
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
const G_EVENT_TYPES: [i32; 5] = [
    EVENT_PILOT_KILLED,
    EVENT_PLANE_CRASHED,
    EVENT_PLANE_DESTROYED,
    EVENT_PLANE_LANDED,
    EVENT_KILLED,
];

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

#[derive(Clone, Copy)]
enum AiRtbDecision {
    Enabled,
    Disabled,
}

impl AiRtbDecision {
    fn property(self) -> &'static str {
        match self {
            Self::Enabled => "1",
            Self::Disabled => "0",
        }
    }
}

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

    fn event_types(&self) -> [i32; 6] {
        match self {
            Self::Strike => [EVENT_PLANE_BINGO_BOMBS, EVENT_PLANE_CRITICAL_DAMAGE,
                EVENT_PILOT_WOUNDED, EVENT_PLANE_BINGO_FUEL, EVENT_KILLED, EVENT_PLANE_LANDED],
            Self::Cap => [EVENT_PLANE_BINGO_MAIN_MG, EVENT_PLANE_BINGO_FUEL,
                EVENT_PLANE_CRITICAL_DAMAGE, EVENT_PILOT_WOUNDED, EVENT_KILLED, EVENT_PLANE_LANDED],
        }
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
    let (src, cached) = match block {
        "MCU_TR_ComplexTrigger" => (COMPLEX_TRIGGER, &COMPLEX_TRIGGER_NODE),
        "MCU_CMD_Damage" => (DAMAGE, &DAMAGE_NODE),
        "MCU_CMD_AttackTarget" => (ATTACK_TARGET, &ATTACK_TARGET_NODE),
        _ => panic!("unknown probe reference {block}"),
    };
    cached.get_or_init(|| {
        let reference = crate::parser::parse_il2_document(src).expect("probe MCU reference parses");
        let ids = block_ids(&reference, block);
        assert_eq!(ids.len(), 1, "one reference {block}");
        by_id(&reference, ids[0]).clone()
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
        let mut damage = self.reference_mcu("MCU_CMD_Damage", name);
        damage.set_property("Damage", level.property());
        damage.set_objects(vec![entity]);
        let damage = self.add(damage);
        self.at(&format!("{name} AT"), time_s, &[damage]);
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
    root.find_by_name(node_name)
        .unwrap_or_else(|| panic!("flight 1M {group_name}: missing node {node_name}"))
        .index.unwrap_or_else(|| panic!("flight 1M {group_name}: node {node_name} missing Index"))
}

fn frozen_m_group(
    name: &str,
    directory: &str,
    file: &str,
    centre_km: (f64, f64),
    next_id: &mut i32,
) -> Il2Entity {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(directory).join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("flight 1M {name}: cannot read {}: {error}", path.display()));
    let source = parse_group_file(&text)
        .unwrap_or_else(|error| panic!("flight 1M {name}: cannot parse {}: {error}", path.display()));
    let zone = m_node_id(&source, name, TEMPLATE_ZONE_IN);
    let anchor = by_id(&source, zone).pos_xz()
        .unwrap_or_else(|| panic!("flight 1M {name}: Zone IN missing position"));
    m_node_id(&source, name, TEMPLATE_TRANSLATOR_MISSION_BEGIN);
    let (mut copy, _) = crate::duplicate::duplicate_template(&source, next_id);
    crate::placement::move_anchor_to(&mut copy, anchor, world(centre_km));
    crate::recon::silence_clone_starts(&mut copy);
    copy
}

fn build_u(p: &mut Probe) {
    let cell = p.cell.expect("flight 1M: missing T-u cell");
    let close = id(&p.root, &format!("{} CLOSE", cell.key));
    for row in U_GROUPS {
        let name = row.name;
        // The named wrapper is probe-owned; even the source Group Name stays intact.
        let mut wrapper = group(name, &mut p.next_id);
        let mut copy = frozen_m_group(name, U_INPUT_DIR, row.source_file, row.centre_km, &mut p.next_id);
        let planes = block_ids(&copy, "Plane");
        let entities = block_ids(&copy, "MCU_TR_Entity");
        assert_eq!(planes.len(), 4, "flight 1M {name}: expected four Plane blocks");
        assert_eq!(entities.len(), 4, "flight 1M {name}: expected four plane entities");
        let home = m_node_id(&copy, name, row.orders.home_timer()).to_string();
        if matches!(row.change, UpstreamChange::RemoveEvent7AbortHooks) {
            for &entity in &entities {
                let mut removed = 0;
                by_id_mut(&mut copy, entity).for_each_mut(&mut |node| {
                    if node.block_type == "OnEvents" {
                        node.children.retain(|event| {
                            let remove = event.block_type == "OnEvent"
                                && event.property("Type") == Some(EVENT_PLANE_BINGO_FUEL.to_string().as_str())
                                && event.property("TarId") == Some(home.as_str());
                            removed += usize::from(remove);
                            !remove
                        });
                    }
                });
                assert_eq!(removed, 1,
                    "flight 1M {name}: entity {entity} missing one event 7 hook to {}", row.orders.home_timer());
            }
        }
        m_node_id(&copy, name, TEMPLATE_MISSION_BEGIN);
        let outputs: Vec<_> = row.orders.timers().iter().map(|&timer_name| {
            let index = m_node_id(&copy, name, timer_name);
            assert_eq!(by_id(&copy, index).block_type, "MCU_Timer",
                "flight 1M {name}: {timer_name} must be a timer");
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

fn build(flight: Flight) -> Probe {
    let mut p = Probe::new(flight);
    for &cell in flight.sheet() {
        p.start_cell(cell);
        match cell.key {
            "T-a1" => build_a1(&mut p),
            "T-e" => build_e(&mut p),
            "T-f" => build_f(&mut p),
            "T-t" => build_tt(&mut p),
            "T-j" => build_j(&mut p),
            "T-z" => build_z(&mut p),
            "T-c" => build_c(&mut p),
            "T-n" => build_n(&mut p, NVariant::Tn),
            "T-n2" => build_n(&mut p, NVariant::Tn2),
            "Y-N" => build_y(&mut p, Side::Nato),
            "Y-D" => build_y(&mut p, Side::Dprk),
            "T-g2" => build_g(&mut p),
            "T-d" => build_d(&mut p),
            "T-d2" => build_d2(&mut p),
            "T-d3" => build_d3(&mut p),
            "T-d4" => build_d4(&mut p),
            "T-r" => build_r(&mut p),
            "T-v" => build_v(&mut p),
            "T-w" => build_w(&mut p),
            "T-u" => build_u(&mut p),
            "T-x" => build_x(&mut p),
            "T-y" => build_larger_rtb(&mut p),
            _ => panic!("unknown probe cell {}", cell.key),
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
    if let Err(error) = trace::write_trace_sidecar(&sidecar, &map) {
        // libtest captures println/eprintln on passing tests. Write directly so
        // the required --ignored command reports sidecar errors without --nocapture.
        use std::io::Write;
        writeln!(
            std::io::stderr().lock(),
            "{}: {error}; Group and .eng files were still written",
            sidecar.display()
        )
        .expect("report trace sidecar error");
    }
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
        sidecar,
        plain.with_extension("spawns.json"),
        traced.with_extension("spawns.json"),
    ] {
        writeln!(
            report,
            "WROTE {} ({} bytes)",
            path.display(),
            std::fs::metadata(&path).unwrap().len()
        )
        .unwrap();
    }
    std::io::Write::write_all(&mut std::io::stdout().lock(), report.as_bytes()).unwrap();
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
            let generated_planes: Vec<_> = ["T-w", "T-x"].into_iter()
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

fn assert_m_listener(root: &Il2Entity, entity: i32, label: &str, event_type: i32) -> i32 {
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
    assert_eq!(matching, 1, "flight 1M {label}: one ev{event_type} listener");
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
                let subtitle = assert_m_listener(root, *entity, label, event_type);
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
            listeners.push(assert_m_listener(root, entity, label, 13));
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
    super::place::place_air_sortie(&mut placed, &super::place::SortiePlacement {
        target: world((50.0, 20.0)), heading_deg: 0.0, spawn_dist_m: 10_000.0,
        rtb: world((40.0, 20.0)),
    }).unwrap();
    let y1 = named(cell, "Y1");
    for (produced, actual) in block_ids(&placed, "Plane").into_iter().zip(block_ids(y1, "Plane")) {
        assert_eq!(by_id(&placed, produced).property("AiRTBDecision"), Some("0"));
        assert_eq!(by_id(y1, actual).property("AiRTBDecision"), by_id(&placed, produced).property("AiRTBDecision"));
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
    assert_eq!(crate::duplicate::duplicate_template(&actual, &mut 1).0,
        crate::duplicate::duplicate_template(&ordinary, &mut 1).0,
        "Y1 differs only by placement formatting, AiRTBDecision and the tested RTB chain");
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

#[test]
fn probe_cells_match_the_table() {
    let zero = generate_probe_0();
    let one_a = generate_probe_1a();
    let one_g = generate_probe_1g();
    let one_n = generate_probe_1n();
    let one_d = generate_probe_1d();
    let one_h = generate_probe_1h();
    let one_j = generate_probe_1j();
    let one_k = generate_probe_1k();
    let one_l = generate_probe_1l();
    let one_m = generate_probe_1m();
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
    for root in [&zero, &one_a, &one_n, &one_d, &one_g, &one_h, &one_j, &one_k, &one_l, &one_m] {
        assert!(root.find_by_name("T-o").is_none());
        assert!(root.find_by_name("P14 DPRK SPAWN").is_none());
    }
    for root in [&one_a, &one_n, &one_d, &one_g, &one_h, &one_j, &one_k, &one_l, &one_m] {
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
    for flight in [Flight::Zero, Flight::OneA, Flight::OneG, Flight::OneH, Flight::OneJ, Flight::OneK, Flight::OneL, Flight::OneM] {
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
