//! P14 step 0b: three solo, test-only probes. Engine outcomes remain unverified.
//! Run the ignored writers after merging 0T to get working trace sidecars.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use crate::ast::Il2Entity;
use crate::locale::{P14_PROBE_TRACE_LCNAME, p14_probe_locale};
use crate::parser::parse_group_file;
use crate::serialize::serialize_group;
use crate::template::{
    attach_event, checkzone, counter, mcu, modifier_set_val, timer, timer_random,
};
use crate::trace::{self, ObjectiveStyle, TraceCarrier, TraceMap, TraceSelect, TraceSource};

const HVAR: &str =
    include_str!("../../TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group");
pub(crate) const K14_ORIGIN: (f64, f64) = (105_934.0, 269_477.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flight {
    Zero,
    OneA,
    OneB,
}

impl Flight {
    fn stem(self) -> &'static str {
        match self {
            Self::Zero => "P14_Probe_0",
            Self::OneA => "P14_Probe_1A",
            Self::OneB => "P14_Probe_1B",
        }
    }

    fn sheet(self) -> &'static [Cell] {
        match self {
            Self::Zero => RUN_SHEET_0,
            Self::OneA => RUN_SHEET_1A,
            Self::OneB => RUN_SHEET_1B,
        }
    }
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
pub(crate) const RUN_SHEET_1B: &[Cell] = &[Cell {
    key: "T-g",
    centre_km: (130.0, 50.0),
    arm_s: 720.0,
    close_s: 1800.0,
}];

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
        flight: Flight::OneB,
        time_s: 0.0,
        text: "T-g: take off and fly to the T-g G1 icon",
    },
    Cue {
        flight: Flight::OneB,
        time_s: 720.0,
        text: "T-g: shoot down both G1 planes at the T-g G1 icon; leave the other flights alone",
    },
    Cue {
        flight: Flight::OneB,
        time_s: 1800.0,
        text: "Flight 1B done. End the mission normally",
    },
];

const A1_START_KM: (f64, f64) = (-30.0, 60.0);
const A1_EXIT_KM: (f64, f64) = (30.0, 60.0);
const C4_KM: (f64, f64) = (40.0, -8.0);
const N_START_KM: (f64, f64) = (30.0, 30.0);
const N_END_KM: (f64, f64) = (54.0, 30.0);
const G1_KM: (f64, f64) = (130.0, 80.0);
const DPRK_SPAWN_KM: (f64, f64) = (170.0, 0.0);
const E_SIX_PULSES: [f64; 6] = [60.0, 62.0, 64.0, 66.0, 68.0, 70.0];
const E3_PULSES: [f64; 3] = [60.0, 62.0, 66.0];
const E_PARTIAL_PULSES: [f64; 3] = [60.0, 62.0, 64.0];
const E9_PULSES: [f64; 3] = [120.0, 120.05, 120.10];
const T2_PULSES: [f64; 5] = [277.0, 279.0, 281.0, 283.0, 285.0];
const T4_PULSES: [f64; 6] = [290.0, 294.0, 298.0, 302.0, 306.0, 310.0];
const T5_PULSES: [f64; 2] = [315.0, 320.0];
const G_EVENT_TYPES: [i32; 5] = [0, 2, 4, 5, 13];

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
    root.find_by_name(name)
        .unwrap_or_else(|| panic!("missing probe MCU {name}"))
}

fn id(root: &Il2Entity, name: &str) -> i32 {
    named(root, name).index.unwrap()
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
    crate::recon::silence_clone_starts(&mut copy);
    let cut = [
        "ENABLE / PULSE IN",
        "Zone IN",
        "Self Deactivate",
        "Zone In ReActivate",
        "COOLDOWN",
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
    remove_ids(&mut copy, &removed);
    copy
}

struct Probe {
    root: Il2Entity,
    next_id: i32,
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
            "Translator Mission Begin",
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
            ("LCText", P14_PROBE_TRACE_LCNAME + 10 + node.index.unwrap()),
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
        for (key, value) in [
            ("Enabled", 1),
            ("LCName", P14_PROBE_TRACE_LCNAME + 10 + node.index.unwrap()),
            ("LCDesc", P14_PROBE_TRACE_LCNAME + 1),
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

    fn ai(
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
        let bring_up = id(&flight, "MISSION BEGIN");
        let entities = block_ids(&flight, "MCU_TR_Entity");
        let zones = block_ids(&flight, "MCU_CheckZone");
        let flight_id = self.add(flight);
        self.at(&format!("{name} START"), start_s, &[bring_up]);
        for zone in zones {
            self.close_node(zone);
        }
        let delete = self.control("MCU_Delete", &format!("{name} CLOSE DELETE"), &[]);
        by_id_mut(&mut self.root, delete).set_objects(entities);
        let close = id(&self.root, &format!("{} CLOSE", self.cell.unwrap().key));
        self.link(close, delete);
        flight_id
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
        trace::instrument(
            &mut self.root,
            &selection,
            &mut self.next_id,
            carrier,
            &mut self.map,
        );
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
    for (name, coalition) in [("ZA1", "[2]"), ("ZA2", "[1]")] {
        let zone = p.zone(name, 12_000.0, true, coalition, centre);
        p.link(p.arm, zone);
        p.observe(zone, &format!("{name} fired"));
    }
    p.ai("T-a1 AI", 4, A1_START_KM, A1_EXIT_KM, 10.0);
    p.icon("T-a1", centre, "[2]");
}

fn build_e(p: &mut Probe) {
    let mut counts = Vec::new();
    for n in 1..=9 {
        let count = p.count(
            &format!("E{n}"),
            if n == 8 { 1 } else { 3 },
            i32::from([2, 7, 9].contains(&n)),
        );
        p.observe(count, &format!("E{n} fired"));
        if [1, 2, 6, 8].contains(&n) {
            p.readout(count, &format!("E{n} fired ≥2"), 2);
        }
        counts.push(count);
    }
    p.pulse_loop(
        "E1 E2",
        E_SIX_PULSES[0],
        2.0,
        E_SIX_PULSES.len(),
        &counts[..2],
    );
    p.pulses("E3", &E3_PULSES, counts[2]);
    p.switch_at("MCU_Deactivate", "E3 OFF", 63.0, &[counts[2]]);
    p.switch_at("MCU_Activate", "E3 ON", 65.0, &[counts[2]]);
    p.pulses("E4", &E_PARTIAL_PULSES, counts[3]);
    p.pulses("E5", &E_PARTIAL_PULSES, counts[4]);
    p.link(p.init_off, counts[4]);
    p.switch_at("MCU_Activate", "E5 ON", 63.0, &[counts[4]]);
    p.pulses("E6", &E_SIX_PULSES, counts[5]);
    for (name, at, count) in [("E4 RESET", 63.0, counts[3]), ("E6 RESET", 65.0, counts[5])] {
        let (x, z) = p.position();
        let mut node = modifier_set_val(name, &mut p.next_id, x, z);
        node.append_target(count);
        let reset = p.add(node);
        p.at(&format!("{name} AT"), at, &[reset]);
    }
    for (name, at, size, count) in [("E7", 100.0, 3, counts[6]), ("E8", 110.0, 2, counts[7])] {
        let relays: Vec<_> = (1..=size)
            .map(|n| {
                let relay = p.relay(&format!("{name} INPUT {n}"), 0.0);
                p.link(relay, count);
                relay
            })
            .collect();
        p.at(&format!("{name} SAME TICK"), at, &relays);
    }
    p.pulses("E9", &E9_PULSES, counts[8]);
}

fn build_f(p: &mut Probe) {
    let (x, z) = p.position();
    let f1 = timer_random("F1", 0.0, 50, &mut p.next_id, x, z);
    let f1 = p.add(f1);
    p.observe(f1, "F1 hit");
    p.readout(f1, "F1 reached 12", 12);
    p.readout(f1, "F1 reached 29", 29);
    p.pulse_loop("F1", 150.0, 3.0, 40, &[f1]);

    let outs: Vec<_> = (1..=4)
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
    for (k, pct) in [25, 33, 50, 100].into_iter().enumerate() {
        let node = timer_random(
            &format!("F2 RANDOM {}", k + 1),
            0.5 * (k + 1) as f64,
            pct,
            &mut p.next_id,
            x,
            z,
        );
        let random = p.add(node);
        p.link(random, outs[k]);
        if k < 3 {
            let close = p.control(
                "MCU_Deactivate",
                &format!("F2 CLOSE AFTER {}", k + 1),
                &outs[k + 1..],
            );
            p.link(random, close);
        }
        inputs.push(random);
    }
    p.pulse_loop("F2", 330.0, 5.0, 40, &inputs);
    for out in outs {
        p.close_node(out);
    }
}

fn build_tt(p: &mut Probe) {
    let objective = TraceCarrier::Objective(ObjectiveStyle::default());
    let t1 = p.at("T-t T1", 275.0, &[]);
    p.observe(t1, "T-t T1 fired");
    p.hand_breadcrumb(t1, &[t1], objective, Some(275.0));

    let t2 = p.relay("T-t T2", 0.0);
    p.observe(t2, "T-t T2 fired");
    p.pulses("T-t T2", &T2_PULSES, t2);
    p.hand_breadcrumb(t2, &[t2], objective, None);

    let mut t3 = vec![];
    for suffix in ['a', 'b'] {
        let relay = p.relay(&format!("T-t T3{suffix}"), 0.0);
        p.observe(relay, &format!("T-t T3{suffix} fired"));
        p.hand_breadcrumb(relay, &[relay], objective, Some(287.0));
        t3.push(relay);
    }
    p.at("T-t T3 SAME TICK", 287.0, &t3);
    for (i, &time) in T4_PULSES.iter().enumerate() {
        let coalition = (i / 2) as i32;
        let success = (i % 2) as i32;
        let name = format!("T-t T4 C{coalition} S{success}");
        let source = p.at(&name, time, &[]);
        p.observe(source, &format!("{name} fired"));
        let carrier = TraceCarrier::Objective(ObjectiveStyle {
            coalition,
            success,
            lc_name: (i == 5).then_some(P14_PROBE_TRACE_LCNAME),
        });
        p.hand_breadcrumb(source, &[source], carrier, Some(time));
    }

    let t5 = p.relay("T-t T5", 0.0);
    p.observe(t5, "T-t T5 fired");
    p.pulses("T-t T5", &T5_PULSES, t5);
    p.hand_breadcrumb(t5, &[t5], TraceCarrier::Spawn, None);

    let t7 = p.at("T-t T7 SAME TICK", 325.0, &[]);
    let subtitle = p.subtitle("T-t T7 fired", 1);
    let mut inputs = vec![];
    for n in 1..=2 {
        let relay = p.relay(&format!("T-t T7 INPUT {n}"), 0.0);
        p.link(t7, relay);
        p.link(relay, subtitle);
        inputs.push(relay);
    }
    // Exactly one carrier, with two direct inputs (no intervening latch).
    p.hand_breadcrumb(t7, &inputs, objective, Some(325.0));
    for source in [t1, t2, t5, t7] {
        p.close_node(source);
    }
}

fn build_j(p: &mut Probe) {
    let j1 = p.relay("J1", 10.0);
    p.link(p.arm, j1);
    p.switch_at("MCU_Deactivate", "J1 OFF", 545.0, &[j1]);
    p.observe(j1, "J1 out");
    let j2 = p.relay("J2", 0.0);
    p.link(p.init_off, j2);
    p.link(p.arm, j2);
    p.switch_at("MCU_Activate", "J2 ON", 541.0, &[j2]);
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
    p.readout(j4, "J4 out ≥2", 2);
    let mut inputs = vec![];
    for n in 1..=2 {
        let input = p.relay(&format!("J4 INPUT {n}"), 0.0);
        p.link(input, j4);
        inputs.push(input);
    }
    p.at("J4 SAME TICK", 540.0, &inputs);
    let j5 = p.relay("J5", 10.0);
    p.link(p.arm, j5);
    p.at("J5 RETRIGGER", 545.0, &[j5]);
    p.observe(j5, "J5 out");
    p.readout(j5, "J5 out ≥2", 2);
    for relay in [j1, j2, j3, j4, j5] {
        p.close_node(relay);
    }
}

/// A second-use observation is gated by the clock, not Counter(2): a zone
/// might repeat while occupied (C5), which is exactly what these flights test.
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
    let z1 = p.zone("Z1", 5000.0, false, "[1]", p.cell.unwrap().centre_km);
    p.link(p.init_off, z1);
    let on = p.control("MCU_Activate", "Z1 ON", &[z1]);
    p.link(p.arm, on);
    p.link(p.arm, z1);
    p.observe(z1, "Z1 fired");
    again(p, z1, "Z1", 610.0, 615.0, 615.0);
    p.icon("T-z", p.cell.unwrap().centre_km, "[2]");
}

fn build_c(p: &mut Probe) {
    let centre = p.cell.unwrap().centre_km;
    let zones: Vec<_> = (0..=5)
        .map(|n| {
            let zone = p.zone(
                &format!("C{n}"),
                5000.0,
                true,
                "[2]",
                if n == 4 { C4_KM } else { centre },
            );
            p.observe(zone, &format!("C{n} fired"));
            if (1..=4).contains(&n) {
                p.link(p.init_off, zone);
            }
            zone
        })
        .collect();
    p.at("C0 SENTINEL", 660.0, &[zones[0]]);
    p.switch_at("MCU_Activate", "C1 ON", 660.0, &[zones[1]]);
    p.switch_at("MCU_Activate", "C2 ON", 660.0, &[zones[2]]);
    p.at("C2 PULSE", 660.1, &[zones[2]]);
    p.at("C3 INACTIVE PULSE", 660.0, &[zones[3]]);
    again(p, zones[2], "C2", 670.0, 675.0, 675.1);
    p.switch_at("MCU_Activate", "C4 ON", 690.0, &[zones[4]]);
    p.at("C4 PULSE", 690.1, &[zones[4]]);
    p.switch_at("MCU_Deactivate", "C4 OFF", 705.0, &[zones[4]]);
    p.at("C5 PULSE", 630.0, &[zones[5]]);
    p.readout(zones[5], "C5 fired ≥2", 2);
    p.readout(zones[5], "C5 fired ≥10", 10);
    p.icon("T-c C", centre, "[2]");
    p.icon("T-c C4", C4_KM, "[2]");
}

fn build_n(p: &mut Probe) {
    for (name, km) in [("T-n START", N_START_KM), ("T-n END", N_END_KM)] {
        let zone = p.zone(&format!("{name} ZONE"), 2000.0, true, "[2]", km);
        let off = p.control("MCU_Deactivate", &format!("{name} OFF"), &[zone]);
        p.link(zone, off);
        p.link(p.arm, zone);
        p.observe(zone, name);
        p.icon(&format!("{name} ICON"), km, "[2]");
        // Editor and map label use the exact run-sheet name; the zone has a suffix.
        let icon = p.root.find_by_name_mut(&format!("{name} ICON")).unwrap();
        icon.set_name(name);
    }
    let za4 = p.zone("ZA4", 12_000.0, true, "[2]", p.cell.unwrap().centre_km);
    p.link(p.arm, za4);
    p.observe(za4, "ZA4 fired");
}

fn build_g(p: &mut Probe) {
    let centre = p.cell.unwrap().centre_km;
    for (n, at) in [(1, G1_KM), (2, centre), (3, centre)] {
        let name = format!("G{n}");
        let flight_id = p.ai(&name, 2, (at.0 - 10.0, at.1), at, 720.0);
        let entities = block_ids(by_id(&p.root, flight_id), "MCU_TR_Entity");
        for (i, &entity) in entities.iter().enumerate() {
            let label = format!("G{n}{}", if i == 0 { 'a' } else { 'b' });
            by_id_mut(&mut p.root, entity).set_name(&format!("{label} entity"));
            for event_type in G_EVENT_TYPES {
                let text = format!("{label} ev{event_type}");
                let subtitle = p.subtitle(&text, 1);
                attach_event(by_id_mut(&mut p.root, entity), event_type, subtitle);
                p.sources.push(p.source(entity, Some(event_type), None));
                if event_type == 4 {
                    let readout = p.count(&format!("READ {text} ≥2"), 2, 0);
                    attach_event(by_id_mut(&mut p.root, entity), event_type, readout);
                    p.observe(readout, &format!("{text} ≥2"));
                }
            }
        }
        if n == 2 {
            let delete = p.control("MCU_Delete", "G2 DELETE", &[]);
            by_id_mut(&mut p.root, delete).set_objects(entities);
            p.at("G2 DELETE AT", 1320.0, &[delete]);
        } else if n == 3 {
            let flight = by_id(&p.root, flight_id);
            let deactivate = id(flight, "Deactivate Units");
            let delay = id(flight, "DELETE DELAY");
            // Preserve the HVAR Delete's Plane-object links and its 0.5 s
            // DELETE DELAY; the probe-owned Deletes use entity links as specified.
            p.at("G3 DEACTIVATE AT", 1320.0, &[deactivate, delay]);
        }
    }
    p.icon("T-g G1", G1_KM, "[1]");
    p.icon("T-g G2/G3", centre, "[1]");
    p.icon("P14 DPRK SPAWN", DPRK_SPAWN_KM, "[1]");
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
            "T-n" => build_n(&mut p),
            "T-g" => build_g(&mut p),
            _ => unreachable!("unknown probe cell"),
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
pub(crate) fn generate_probe_1b() -> Il2Entity {
    build(Flight::OneB).root
}

fn write_probe(flight: Flight, carrier: TraceCarrier) {
    let dir = Path::new("target/p14");
    std::fs::create_dir_all(dir).expect("create target/p14");
    let probe = build(flight);
    let plain = dir.join(format!("{}.Group", flight.stem()));
    let traced = dir.join(format!("{}_traced.Group", flight.stem()));
    let locale = p14_probe_locale(&probe.root);
    let english = crate::locale::encode_locale_utf16le(&crate::locale::serialize_locale(&locale));
    std::fs::write(&plain, serialize_group(&probe.root)).expect("write plain probe");
    let (root, map) = probe.traced(carrier);
    std::fs::write(&traced, serialize_group(&root)).expect("write traced probe");
    for path in [&plain, &traced] {
        std::fs::write(path.with_extension("eng"), &english).expect("write probe English locale");
    }
    let sidecar = traced.with_extension("trace.json");
    if let Err(error) = trace::write_trace_sidecar(&sidecar, &map) {
        // libtest captures println/eprintln on passing tests. Write directly so
        // the required --ignored command reports the stub even without --nocapture.
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
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_0() {
    write_probe(
        Flight::Zero,
        TraceCarrier::Objective(ObjectiveStyle::default()),
    );
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1a() {
    // Change this carrier after flight 0 if its result selects Spawn.
    write_probe(Flight::OneA, TraceCarrier::default());
}

#[test]
#[ignore = "writes target/p14 probe files"]
fn write_p14_probe_1b() {
    write_probe(Flight::OneB, TraceCarrier::default());
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
    let planes = block_ids(flight, "Plane");
    let entities = block_ids(flight, "MCU_TR_Entity");
    assert_eq!(planes.len(), count);
    assert_eq!(entities.len(), count);
    let original = parse_group_file(HVAR).unwrap();
    let original_planes = block_ids(&original, "Plane");
    for (i, &plane_id) in planes.iter().enumerate() {
        let plane = by_id(flight, plane_id);
        let source = by_id(&original, original_planes[i]);
        assert_eq!(
            plane.name(),
            source.name(),
            "keep the first planes in file order"
        );
        let source_pos = source.pos_xz().unwrap();
        let original_lead = by_id(&original, original_planes[0]).pos_xz().unwrap();
        let position = plane.pos_xz().unwrap();
        assert_near(position.0, start.0 + source_pos.0 - original_lead.0);
        assert_near(position.1, start.1 + source_pos.1 - original_lead.1);
        assert_eq!(plane.property("Country"), Some("601"));
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
}

#[test]
fn probe_files_parse_and_links_resolve() {
    use super::testkit::assert_links_resolve;
    for (flight, root) in [
        (Flight::Zero, generate_probe_0()),
        (Flight::OneA, generate_probe_1a()),
        (Flight::OneB, generate_probe_1b()),
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
    for flight in [Flight::Zero, Flight::OneA, Flight::OneB] {
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
            }
        });
        for cue in CUES.iter().filter(|c| c.flight == flight) {
            let subtitle = id(&p.root, cue.text);
            assert!(p.sources.iter().any(|s| s.expected_s == Some(cue.time_s)
                && by_id(&p.root, s.index).targets.contains(&subtitle)));
        }
    }
}

#[test]
fn probe_subtitles_are_unique() {
    for flight in [Flight::Zero, Flight::OneA, Flight::OneB] {
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
        assert_eq!(
            locale.get(P14_PROBE_TRACE_LCNAME),
            Some("T-t T4: named objective")
        );
        let bytes = crate::locale::encode_locale_utf16le(&crate::locale::serialize_locale(&locale));
        assert_eq!(&bytes[..2], &[0xff, 0xfe]);
        assert_eq!(
            crate::locale::parse_locale(&crate::locale::decode_locale_bytes(&bytes).unwrap()),
            locale
        );
    }
}

#[test]
fn probe_cells_match_the_table() {
    let zero = generate_probe_0();
    let one_a = generate_probe_1a();
    let one_b = generate_probe_1b();
    assert_eq!(zero.count_block_type("Plane"), 4);
    assert_eq!(one_a.count_block_type("Plane"), 0);
    assert_eq!(one_b.count_block_type("Plane"), 6);

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
        assert_readout(&zero, id(&zero, name), &format!("{name} fired ≥2"), 2);
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

    // These times and same-tick inputs exist even with the 0T stubs. Only
    // assertions about actual breadcrumb blocks live in the ignored test.
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
    for (i, time) in [290.0, 294.0, 298.0, 302.0, 306.0, 310.0]
        .into_iter()
        .enumerate()
    {
        let name = format!("T-t T4 C{} S{}", i / 2, i % 2);
        let node = named(&zero, &name);
        assert_at(&zero, &name, time, &node.targets);
        assert_observation(&zero, node.index.unwrap(), &format!("{name} fired"));
    }
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
        assert_readout(&zero, id(&zero, name), &format!("{name} out ≥2"), 2);
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
    assert_readout(&one_a, id(&one_a, "C5"), "C5 fired ≥2", 2);
    assert_readout(&one_a, id(&one_a, "C5"), "C5 fired ≥10", 10);

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

    for (n, km) in [(1, (130.0, 80.0)), (2, (130.0, 50.0)), (3, (130.0, 50.0))] {
        let name = format!("G{n}");
        let flight = named(&one_b, &name);
        assert_flight(flight, 2, world((km.0 - 10.0, km.1)), world(km));
        assert_at(
            &one_b,
            &format!("{name} START"),
            720.0,
            &[id(flight, "MISSION BEGIN")],
        );
        let entities = block_ids(flight, "MCU_TR_Entity");
        assert_eq!(
            named(&one_b, &format!("{name} CLOSE DELETE")).objects,
            entities
        );
        for (i, entity_id) in entities.into_iter().enumerate() {
            let entity = by_id(&one_b, entity_id);
            let label = format!("G{n}{}", if i == 0 { 'a' } else { 'b' });
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
                            && number(e, "TarId") == f64::from(id(&one_b, &text)))
                );
            }
            let read_name = format!("READ {label} ev4 ≥2");
            assert_counter(&one_b, &read_name, 2, 0);
            let read = named(&one_b, &read_name);
            assert_eq!(read.targets, [id(&one_b, &format!("{label} ev4 ≥2"))]);
            assert!(events.children.iter().any(|e| number(e, "Type") == 4.0
                && number(e, "TarId") == f64::from(read.index.unwrap())));
        }
    }
    let g1_pos = named(named(&one_b, "G1"), "WP 1").pos_xz().unwrap();
    let g2_pos = named(named(&one_b, "G2"), "WP 1").pos_xz().unwrap();
    assert_near(g1_pos.0, g2_pos.0);
    assert_near(g1_pos.1 - g2_pos.1, 30_000.0);
    assert_eq!(named(&one_b, "T-g G1").block_type, "MCU_Icon");
    assert_eq!(named(&one_b, "T-g G1").pos_xz(), Some(g1_pos));
    assert_eq!(named(&one_b, "T-g G1").property("Coalitions"), Some("[1]"));
    assert_at(&one_b, "G2 DELETE AT", 1320.0, &[id(&one_b, "G2 DELETE")]);
    assert_eq!(
        named(&one_b, "G2 DELETE").objects,
        block_ids(named(&one_b, "G2"), "MCU_TR_Entity")
    );
    let g3 = named(&one_b, "G3");
    assert_at(
        &one_b,
        "G3 DEACTIVATE AT",
        1320.0,
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
        (Flight::OneB, "T-g", (130.0, 50.0), 720.0, 1800.0),
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
            Flight::OneB,
            0.0,
            "T-g: take off and fly to the T-g G1 icon",
        ),
        (
            Flight::OneB,
            720.0,
            "T-g: shoot down both G1 planes at the T-g G1 icon; leave the other flights alone",
        ),
        (
            Flight::OneB,
            1800.0,
            "Flight 1B done. End the mission normally",
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
    for flight in [Flight::Zero, Flight::OneA, Flight::OneB] {
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
            for zone in block_ids(group, "MCU_CheckZone") {
                assert!(
                    off.targets.contains(&zone),
                    "{key} leaves check zone {zone} open"
                );
            }
            // Every flight closes through its own Delete as well, even when
            // G2/G3 have already been removed at 22:00.
            for &delete in close
                .targets
                .iter()
                .filter(|&&i| by_id(&root, i).block_type == "MCU_Delete")
            {
                assert!(!by_id(&root, delete).objects.is_empty());
            }
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
    let one_b = generate_probe_1b();
    for (root, name, km, coalition) in [
        (&one_a, "T-c C", (40.0, -20.0), "[2]"),
        (&one_a, "T-c C4", (40.0, -8.0), "[2]"),
        (&one_a, "T-n START", (30.0, 30.0), "[2]"),
        (&one_a, "T-n END", (54.0, 30.0), "[2]"),
        (&one_b, "T-g G1", (130.0, 80.0), "[1]"),
        (&one_b, "P14 DPRK SPAWN", (170.0, 0.0), "[1]"),
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
#[ignore = "needs step 0T trace bodies"]
fn probe_tt_breadcrumbs_match_the_table() {
    let p = build(Flight::Zero);
    let root = &p.root;
    assert_eq!(
        p.map.entries.len(),
        12,
        "T1, T2, two T3s, six T4s, T5, one T7"
    );
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
        assert!(entry.group_path.iter().any(|part| part == "T-t"));
        entry
    };
    let objective = |name: &str, coalition: i32, success: i32, lc_name: Option<i32>| {
        let entry = entry_for(name);
        assert_eq!(
            entry.carrier,
            TraceCarrier::Objective(ObjectiveStyle {
                coalition,
                success,
                lc_name
            })
        );
        let node = by_id(root, entry.breadcrumb_index);
        assert_eq!(node.block_type, "MCU_TR_MissionObjective");
        assert_eq!(number(node, "TaskType"), 0.0);
        assert_eq!(number(node, "IconType"), 0.0);
        assert_eq!(number(node, "Coalition"), f64::from(coalition));
        assert_eq!(number(node, "Success"), f64::from(success));
        assert_eq!(
            node.property("LCName").map(|v| v.parse::<i32>().unwrap()),
            lc_name
        );
        assert_eq!(node.pos_xz(), Some(entry.pos));
        entry.breadcrumb_index
    };
    let t1 = objective("T-t T1", 0, 1, None);
    assert!(named(root, "T-t T1").targets.contains(&t1));
    assert_at(root, "T-t T1", 275.0, &named(root, "T-t T1").targets);
    assert_eq!(entry_for("T-t T1").expected_s, Some(275.0));
    let t2 = objective("T-t T2", 0, 1, None);
    assert!(named(root, "T-t T2").targets.contains(&t2));
    assert_pulses(
        root,
        "T-t T2",
        &[277.0, 279.0, 281.0, 283.0, 285.0],
        id(root, "T-t T2"),
    );
    for name in ["T-t T3a", "T-t T3b"] {
        let breadcrumb = objective(name, 0, 1, None);
        assert!(named(root, name).targets.contains(&breadcrumb));
        assert_eq!(number(named(root, name), "Time"), 0.0);
    }
    assert_at(
        root,
        "T-t T3 SAME TICK",
        287.0,
        &[id(root, "T-t T3a"), id(root, "T-t T3b")],
    );
    let mut t4_times = vec![];
    for i in 0..6 {
        let name = format!("T-t T4 C{} S{}", i / 2, i % 2);
        let breadcrumb = objective(
            &name,
            i / 2,
            i % 2,
            (i == 5).then_some(P14_PROBE_TRACE_LCNAME),
        );
        let source = named(root, &name);
        let time = 290.0 + f64::from(i) * 4.0;
        assert_at(root, &name, time, &source.targets);
        assert!(source.targets.contains(&breadcrumb));
        assert_eq!(entry_for(&name).expected_s, Some(time));
        t4_times.push(number(source, "Time"));
    }
    assert!(t4_times.windows(2).all(|w| w[1] - w[0] == 4.0));
    let t5 = entry_for("T-t T5");
    assert_eq!(t5.carrier, TraceCarrier::Spawn);
    assert!(
        t5.spawn_name
            .as_deref()
            .is_some_and(|n| n.starts_with("TRACE "))
    );
    assert_eq!(by_id(root, t5.breadcrumb_index).block_type, "MCU_Spawner");
    assert!(named(root, "T-t T5").targets.contains(&t5.breadcrumb_index));
    assert_pulses(root, "T-t T5", &[315.0, 320.0], id(root, "T-t T5"));
    let t7 = objective("T-t T7 SAME TICK", 0, 1, None);
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
    let (traced, map) = p.traced(TraceCarrier::Objective(ObjectiveStyle::default()));
    for entry in &hand_entries {
        assert_eq!(
            map.entries
                .iter()
                .filter(|e| e.source_index == entry.source_index)
                .count(),
            1,
            "T-t was instrumented twice"
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
        assert_eq!(
            entries[0].carrier,
            TraceCarrier::Objective(ObjectiveStyle::default())
        );
    }
}
