#![allow(dead_code)] // P14: removed in step 9T
//! Opt-in mission breadcrumbs. Flight 0 ruled out objectives for dogfight rounds;
//! spawn logging on every firing remains UNVERIFIED.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::ast::Il2Entity;
use crate::template::{mcu, timer};

pub const TRACE_X0: f64 = 5_000.0;
pub const TRACE_Z0: f64 = 5_000.0;
/// Northern dry, open cell in the built-in Korea mask (450, 897). This is
/// remote from the planned front, not a guarantee about imported missions.
pub const TRACE_SPAWN_POINT: (f64, f64) = (400_000.0, 50_000.0);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TraceSelect {
    pub block_types: Vec<String>,
    pub name_prefixes: Vec<String>,
    pub indexes: Vec<i32>,
    pub event_types: Vec<i32>,
    pub random_timers: bool,
}

impl TraceSelect {
    pub fn decision_points() -> Self {
        Self {
            block_types: [
                "MCU_CheckZone",
                "MCU_Counter",
                "MCU_TR_MissionBegin",
                "MCU_Spawner",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            random_timers: true,
            ..Self::default()
        }
    }

    fn matches(&self, node: &Il2Entity) -> bool {
        self.block_types.contains(&node.block_type)
            || node.index.is_some_and(|id| self.indexes.contains(&id))
            || node
                .name()
                .is_some_and(|name| self.name_prefixes.iter().any(|p| name.starts_with(p)))
            || (self.random_timers
                && node.block_type == "MCU_Timer"
                && number(node, "Random").is_some_and(|n| n < 100.0))
            || (node.block_type == "MCU_TR_Entity"
                && event_hooks(node)
                    .any(|h| integer(h, "Type").is_some_and(|t| self.event_types.contains(&t))))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectiveStyle {
    pub coalition: i32,
    pub success: i32,
    pub lc_name: Option<i32>,
}

impl Default for ObjectiveStyle {
    fn default() -> Self {
        Self {
            coalition: 0,
            success: 1,
            lc_name: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceCarrier {
    Objective(ObjectiveStyle),
    // Flight 0 (2026-09-28): the first objective breadcrumb ended the Korea
    // dogfight round (MissionType 2), so use the Spawn fallback (risk t3).
    #[default]
    Spawn,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceSource {
    pub index: i32,
    pub name: String,
    pub block_type: String,
    pub event_type: Option<i32>,
    pub group_path: Vec<String>,
    pub expected_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TraceMap {
    pub entries: Vec<TraceEntry>,
    pub edges: Vec<TraceEdge>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceEntry {
    pub breadcrumb_index: i32,
    pub source_index: i32,
    pub source_name: String,
    pub source_type: String,
    pub event_type: Option<i32>,
    pub group_path: Vec<String>,
    pub pos: (f64, f64),
    pub carrier: TraceCarrier,
    pub spawn_name: Option<String>,
    pub expected_s: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceEdge {
    /// Endpoints are breadcrumb indexes, so two event types on one entity
    /// remain distinct. The already-observed source is excluded from the path;
    /// the destination and every intervening MCU are included.
    pub from: i32,
    pub to: i32,
    pub unconditional: bool,
    pub delay_s: f64,
}

pub fn instrument(
    root: &mut Il2Entity,
    sel: &TraceSelect,
    next_id: &mut i32,
    carrier: TraceCarrier,
    map: &mut TraceMap,
) {
    let mut begin_targets = HashSet::new();
    root.for_each(&mut |e| {
        if e.block_type == "MCU_TR_MissionBegin" {
            begin_targets.extend(e.targets.iter().copied());
        }
    });
    // Never revisit newly added blocks, including breadcrumbs registered by a probe.
    let breadcrumbs: HashSet<_> = map.entries.iter().map(|e| e.breadcrumb_index).collect();
    let mut sources = Vec::new();
    collect_sources(
        root,
        sel,
        &mut Vec::new(),
        &begin_targets,
        &breadcrumbs,
        &mut sources,
    );
    if !sources.is_empty() {
        *next_id = (*next_id)
            .max(root.max_index() + 1)
            .max(breadcrumbs.iter().max().copied().unwrap_or(0) + 1);
    }
    for source in sources {
        let source_id = source.index;
        let event_type = source.event_type;
        let blocks = breadcrumb(carrier, map, next_id, source);
        let id = map.entries.last().unwrap().breadcrumb_index;
        root.for_each_mut(&mut |e| {
            if e.index == Some(source_id) {
                if let Some(event_type) = event_type {
                    append_event(e, event_type, id);
                } else {
                    e.append_target(id);
                }
            }
        });
        root.children.extend(blocks);
    }
    rebuild_edges(root, map);
}

fn collect_sources(
    node: &Il2Entity,
    sel: &TraceSelect,
    path: &mut Vec<String>,
    begin_targets: &HashSet<i32>,
    breadcrumbs: &HashSet<i32>,
    out: &mut Vec<TraceSource>,
) {
    let group = node.block_type == "Group";
    if group {
        path.push(node.name().unwrap_or("Group").to_string());
    }
    if let Some(index) = node.index
        && node.block_type.starts_with("MCU_")
        && !breadcrumbs.contains(&index)
        && sel.matches(node)
    {
        let events = if node.block_type == "MCU_TR_Entity" {
            // With no explicit types, an explicitly selected entity traces its
            // wired OnEvents. OnReports are traversed, not converted to events.
            let types = if sel.event_types.is_empty() {
                event_hooks(node)
                    .filter(|h| h.block_type == "OnEvent")
                    .filter_map(|h| integer(h, "Type"))
                    .collect()
            } else {
                sel.event_types.clone()
            };
            let mut seen = HashSet::new();
            types
                .into_iter()
                .filter(|t| seen.insert(*t))
                .map(Some)
                .collect::<Vec<_>>()
        } else {
            vec![None]
        };
        for event_type in events {
            out.push(TraceSource {
                index,
                name: node.name().unwrap_or("").into(),
                block_type: node.block_type.clone(),
                event_type,
                group_path: path.clone(),
                expected_s: (node.block_type == "MCU_Timer" && begin_targets.contains(&index))
                    .then(|| number(node, "Time"))
                    .flatten(),
            });
        }
    }
    for child in &node.children {
        collect_sources(child, sel, path, begin_targets, breadcrumbs, out);
    }
    if group {
        path.pop();
    }
}

fn append_event(node: &mut Il2Entity, event_type: i32, target: i32) {
    // attach_event moves an existing wrapper to the end. Preserve child order
    // here because instrumentation must change only the appended event.
    if let Some(wrapper) = node
        .children
        .iter_mut()
        .find(|e| e.block_type == "OnEvents")
    {
        let mut event = Il2Entity::new("OnEvent");
        event.set_property("Type", event_type.to_string());
        event.set_property("TarId", target.to_string());
        wrapper.children.push(event);
    } else {
        crate::template::attach_event(node, event_type, target);
    }
}

/// Numbering is zero-based and continues after all entries already registered.
/// Returned blocks are siblings; the first block is the source's target.
pub fn breadcrumb(
    carrier: TraceCarrier,
    map: &mut TraceMap,
    next_id: &mut i32,
    source: TraceSource,
) -> Vec<Il2Entity> {
    let n = map.entries.len();
    let grid = (
        TRACE_X0 + 10.0 * (n % 3000) as f64,
        TRACE_Z0 + 10.0 * (n / 3000) as f64,
    );
    assert!(crate::geo::on_map(grid.0, grid.1), "trace grid exhausted");
    let name = format!("TRACE {n}");
    let (blocks, pos, spawn_name) = match carrier {
        TraceCarrier::Objective(style) => {
            // Objectives use the shipped key order, without the Name/Desc
            // properties that the general MCU builder adds.
            let mut objective = Il2Entity::new("MCU_TR_MissionObjective");
            objective.index = Some(*next_id);
            objective.set_property("Index", next_id.to_string());
            *next_id += 1;
            objective.set_targets(Vec::new());
            objective.set_objects(Vec::new());
            objective.set_property("XPos", format!("{:.3}", grid.0));
            objective.set_property("YPos", "0.000");
            objective.set_property("ZPos", format!("{:.3}", grid.1));
            for (key, value) in [
                ("XOri", "0"),
                ("YOri", "0"),
                ("ZOri", "0"),
                ("Enabled", "1"),
            ] {
                objective.set_property(key, value);
            }
            objective.set_property("LCName", style.lc_name.unwrap_or(0).to_string());
            objective.set_property("LCDesc", "0");
            for (key, value) in [
                ("TaskType", 0),
                ("Coalition", style.coalition),
                ("Success", style.success),
                ("IconType", 0),
            ] {
                objective.set_property(key, value.to_string());
            }
            (vec![objective], grid, None)
        }
        TraceCarrier::Spawn => (
            spawn_breadcrumb(&name, next_id),
            TRACE_SPAWN_POINT,
            Some(name),
        ),
    };
    map.push(TraceEntry {
        breadcrumb_index: blocks[0].index.unwrap(),
        source_index: source.index,
        source_name: source.name,
        source_type: source.block_type,
        event_type: source.event_type,
        group_path: source.group_path,
        pos,
        carrier,
        spawn_name,
        expected_s: source.expected_s,
    });
    blocks
}

fn spawn_breadcrumb(name: &str, next_id: &mut i32) -> Vec<Il2Entity> {
    let (x, z) = TRACE_SPAWN_POINT;
    let mut spawn = mcu("MCU_Spawner", name, next_id, x, z);
    spawn.set_property("SpawnAtMe", "0");
    // Property set copied from K14 AFB_mp.Group's WillysMB, Index 117.
    let mut vehicle = Il2Entity::new("Vehicle");
    vehicle.set_name(name);
    vehicle.index = Some(*next_id);
    vehicle.set_property("Index", next_id.to_string());
    *next_id += 1;
    let mut entity = mcu("MCU_TR_Entity", &format!("{name} entity"), next_id, x, z);
    entity.set_property("Enabled", "0");
    entity.set_property("MisObjID", vehicle.index.unwrap().to_string());
    vehicle.set_property("LinkTrId", entity.index.unwrap().to_string());
    vehicle.set_property("XPos", format!("{x:.3}"));
    vehicle.set_property("YPos", "0.000");
    vehicle.set_property("ZPos", format!("{z:.3}"));
    for (key, value) in [
        ("XOri", "0"),
        ("YOri", "0"),
        ("ZOri", "0"),
        (
            "Script",
            r#""LuaScripts\WorldObjects\Vehicles\WillysMB.txt""#,
        ),
        ("Model", r#""graphics\Vehicles\WillysMB\WillysMB.mgm""#),
        ("Desc", "\"\""),
        ("Country", "0"),
        ("NumberInFormation", "0"),
        ("Vulnerable", "0"),
        ("Engageable", "0"),
        ("LimitAmmo", "1"),
        ("AILevel", "2"),
        ("DamageReport", "50"),
        ("DamageThreshold", "1"),
        ("DeleteAfterDeath", "1"),
        ("CoopStart", "0"),
        ("Spotter", "-1"),
        ("BeaconChannel", "0"),
        ("Callsign", "0"),
        ("PayloadId", "0"),
        ("ModMask", "10"),
        ("Fuel", "1"),
        ("Callnum", "0"),
        ("Skin", "\"\""),
        ("BotSkin", "\"\""),
        ("RepairTimeMultiplier", "0"),
        ("RehealTimeMultiplier", "0"),
        ("RearmTimeMultiplier", "0"),
        ("RefuelTimeMultiplier", "0"),
        ("MaintenanceRadius", "10"),
        ("TCode", "\"%20%20%20%20%20%20%20%20\""),
        ("TCodeColor", "\"11111111\""),
        ("TrailerAtStart", "1"),
        ("PinToTerrain", "1"),
    ] {
        vehicle.set_property(key, value);
    }
    spawn.set_objects(vec![entity.index.unwrap()]);
    let mut delay = timer(&format!("{name} DELETE DELAY"), 1.0, next_id, x, z);
    let mut delete = mcu("MCU_Delete", &format!("{name} DELETE"), next_id, x, z);
    // Template Builder's Delete uses object IDs, while Spawner uses entity IDs.
    delete.set_objects(vec![vehicle.index.unwrap()]);
    delay.set_targets(vec![delete.index.unwrap()]);
    let mut report = Il2Entity::new("OnReport");
    report.set_property("Type", "0"); // OnSpawned, as in template.rs.
    report.set_property("CmdId", spawn.index.unwrap().to_string());
    report.set_property("TarId", delay.index.unwrap().to_string());
    let mut wrapper = Il2Entity::new("OnReports");
    wrapper.children.push(report);
    entity.children.push(wrapper);
    vec![spawn, vehicle, entity, delay, delete]
}

impl TraceMap {
    pub fn push(&mut self, entry: TraceEntry) {
        self.entries.push(entry);
    }
}

fn integer(node: &Il2Entity, key: &str) -> Option<i32> {
    node.property(key)?.parse().ok()
}
fn number(node: &Il2Entity, key: &str) -> Option<f64> {
    node.property(key)?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

fn event_hooks(node: &Il2Entity) -> impl Iterator<Item = &Il2Entity> {
    node.children
        .iter()
        .filter(|e| matches!(e.block_type.as_str(), "OnEvents" | "OnReports"))
        .flat_map(|e| &e.children)
        .filter(|e| matches!(e.block_type.as_str(), "OnEvent" | "OnReport"))
}

fn pulse_targets(node: &Il2Entity, event_type: Option<i32>) -> Vec<i32> {
    if node.block_type == "MCU_TR_Entity" {
        event_hooks(node)
            // An event breadcrumb observes OnEvent, never a same-numbered report.
            .filter(|h| {
                event_type
                    .is_none_or(|t| h.block_type == "OnEvent" && integer(h, "Type") == Some(t))
            })
            .filter_map(|h| integer(h, "TarId"))
            .collect()
    } else if matches!(
        node.block_type.as_str(),
        "MCU_Timer" | "MCU_Counter" | "MCU_CheckZone" | "MCU_Waypoint"
    ) || node.block_type.starts_with("MCU_TR_")
    {
        node.targets.clone()
    } else {
        Vec::new() // Activate, Deactivate, ModifierSetVal, Delete and commands are state links.
    }
}

fn index_nodes<'a>(node: &'a Il2Entity, nodes: &mut HashMap<i32, &'a Il2Entity>) {
    if let Some(id) = node.index {
        nodes.insert(id, node);
    }
    for child in &node.children {
        index_nodes(child, nodes);
    }
}

/// Recompute the nearest traced successors, also for hand-built breadcrumbs.
/// Each (node, unconditional) state is visited at its shortest delay. Positive
/// timer loops therefore terminate without hiding a second, ungated route.
pub fn rebuild_edges(root: &Il2Entity, map: &mut TraceMap) {
    let mut nodes = HashMap::new();
    index_nodes(root, &mut nodes);
    let breadcrumbs: HashSet<_> = map.entries.iter().map(|e| e.breadcrumb_index).collect();
    let deactivated: HashSet<_> = nodes
        .values()
        .filter(|e| e.block_type == "MCU_Deactivate")
        .flat_map(|e| e.targets.iter().chain(&e.objects))
        .copied()
        .collect();
    let mut traced: HashMap<i32, Vec<&TraceEntry>> = HashMap::new();
    for e in &map.entries {
        traced.entry(e.source_index).or_default().push(e);
    }
    let mut edges: BTreeMap<(i32, i32, bool), f64> = BTreeMap::new();
    for source in &map.entries {
        let Some(node) = nodes.get(&source.source_index) else {
            continue;
        };
        let mut pending: Vec<_> = pulse_targets(node, source.event_type)
            .into_iter()
            .map(|id| (id, true, 0.0))
            .collect();
        let mut visited: HashMap<(i32, bool), f64> = HashMap::new();
        while let Some((id, unconditional, delay)) = pending.pop() {
            if breadcrumbs.contains(&id) {
                continue;
            }
            let Some(node) = nodes.get(&id) else { continue };
            let time = if node.block_type == "MCU_Timer" {
                number(node, "Time")
            } else {
                None
            };
            let unconditional = unconditional
                && integer(node, "Enabled") != Some(0)
                && time.is_some_and(|t| t >= 0.0)
                && number(node, "Random") == Some(100.0)
                && !deactivated.contains(&id);
            let delay = delay + time.unwrap_or(0.0).max(0.0);
            if !delay.is_finite()
                || visited
                    .get(&(id, unconditional))
                    .is_some_and(|old| *old <= delay)
            {
                continue;
            }
            visited.insert((id, unconditional), delay);
            if let Some(destinations) = traced.get(&id) {
                // Pulsing an entity does not imply one of its events happened.
                for dest in destinations.iter().filter(|e| e.event_type.is_none()) {
                    let edge = edges
                        .entry((
                            source.breadcrumb_index,
                            dest.breadcrumb_index,
                            unconditional,
                        ))
                        .or_insert(delay);
                    *edge = edge.min(delay);
                }
                if node.block_type == "MCU_TR_Entity" {
                    pending.extend(
                        pulse_targets(node, None)
                            .into_iter()
                            .map(|id| (id, false, delay)),
                    );
                }
                continue;
            }
            pending.extend(
                pulse_targets(node, None)
                    .into_iter()
                    .map(|id| (id, unconditional, delay)),
            );
        }
    }
    map.edges = edges
        .into_iter()
        .map(|((from, to, unconditional), delay_s)| TraceEdge {
            from,
            to,
            unconditional,
            delay_s,
        })
        .collect();
}

fn sidecar_path(path: &Path) -> PathBuf {
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("Group"))
    {
        path.with_extension("trace.json")
    } else {
        path.to_owned()
    }
}

/// Accepts a .Group path (like locale::write_sidecars) or an explicit sidecar path.
pub fn write_trace_sidecar(path: &Path, map: &TraceMap) -> Result<(), String> {
    validate_map(map)?;
    let path = sidecar_path(path);
    std::fs::write(&path, encode_map(map)).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn read_trace_sidecar(path: &Path) -> Result<TraceMap, String> {
    let path = sidecar_path(path);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode_map(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn validate_map(map: &TraceMap) -> Result<(), String> {
    let mut ids = HashSet::new();
    for e in &map.entries {
        if !ids.insert(e.breadcrumb_index) {
            return Err("duplicate breadcrumb index".into());
        }
        if !e.pos.0.is_finite()
            || !e.pos.1.is_finite()
            || e.expected_s.is_some_and(|s| !s.is_finite() || s < 0.0)
        {
            return Err("invalid trace position or expected_s".into());
        }
    }
    for e in &map.edges {
        if !ids.contains(&e.from) || !ids.contains(&e.to) {
            return Err("trace edge has an unknown endpoint".into());
        }
        if !e.delay_s.is_finite() || e.delay_s < 0.0 {
            return Err("invalid trace delay_s".into());
        }
    }
    Ok(())
}

fn quote_json(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < '\u{20}' => {
                write!(out, "\\u{:04x}", c as u32).unwrap();
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_option<T: ToString>(v: Option<T>) -> String {
    v.map_or_else(|| "null".into(), |v| v.to_string())
}

fn encode_map(map: &TraceMap) -> String {
    let entries: Vec<_> = map.entries.iter().map(|e| {
        let carrier = match e.carrier {
            TraceCarrier::Spawn => "{\"kind\":\"Spawn\"}".into(),
            TraceCarrier::Objective(s) => format!("{{\"kind\":\"Objective\",\"style\":{{\"coalition\":{},\"success\":{},\"lc_name\":{}}}}}", s.coalition, s.success, json_option(s.lc_name)),
        };
        format!("    {{\"breadcrumb_index\":{},\"source_index\":{},\"source_name\":{},\"source_type\":{},\"event_type\":{},\"group_path\":[{}],\"pos\":[{},{}],\"carrier\":{},\"spawn_name\":{},\"expected_s\":{}}}",
            e.breadcrumb_index, e.source_index, quote_json(&e.source_name), quote_json(&e.source_type),
            json_option(e.event_type), e.group_path.iter().map(|s| quote_json(s)).collect::<Vec<_>>().join(","),
            e.pos.0, e.pos.1, carrier, json_option(e.spawn_name.as_ref().map(|s| quote_json(s))), json_option(e.expected_s))
    }).collect();
    let edges: Vec<_> = map
        .edges
        .iter()
        .map(|e| {
            format!(
                "    {{\"from\":{},\"to\":{},\"unconditional\":{},\"delay_s\":{}}}",
                e.from, e.to, e.unconditional, e.delay_s
            )
        })
        .collect();
    format!(
        "{{\n  \"entries\":[\n{}\n  ],\n  \"edges\":[\n{}\n  ]\n}}\n",
        entries.join(",\n"),
        edges.join(",\n")
    )
}

// No JSON dependency in this crate. This private codec handles only JSON values
// needed for TraceMap; the shape reader below rejects missing or mistyped fields.
#[derive(Debug)]
enum Json {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    fn field(&self, key: &str) -> Result<&Self, String> {
        if let Self::Object(o) = self {
            o.get(key).ok_or_else(|| format!("missing {key}"))
        } else {
            Err("expected JSON object".into())
        }
    }
    fn array(&self) -> Result<&[Self], String> {
        if let Self::Array(a) = self {
            Ok(a)
        } else {
            Err("expected JSON array".into())
        }
    }
    fn string(&self) -> Result<&str, String> {
        if let Self::String(s) = self {
            Ok(s)
        } else {
            Err("expected JSON string".into())
        }
    }
    fn int(&self) -> Result<i32, String> {
        if let Self::Number(n) = self {
            n.parse().map_err(|_| "expected i32".into())
        } else {
            Err("expected JSON integer".into())
        }
    }
    fn float(&self) -> Result<f64, String> {
        if let Self::Number(n) = self {
            n.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| "expected finite number".into())
        } else {
            Err("expected JSON number".into())
        }
    }
    fn boolean(&self) -> Result<bool, String> {
        if let Self::Bool(b) = self {
            Ok(*b)
        } else {
            Err("expected JSON boolean".into())
        }
    }
    fn optional<T>(
        &self,
        read: impl FnOnce(&Self) -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        if matches!(self, Self::Null) {
            Ok(None)
        } else {
            read(self).map(Some)
        }
    }
}

struct JsonReader<'a> {
    rest: &'a str,
}
impl JsonReader<'_> {
    fn ws(&mut self) {
        self.rest = self.rest.trim_start_matches([' ', '\r', '\n', '\t']);
    }
    fn take(&mut self, token: &str) -> bool {
        self.ws();
        if let Some(rest) = self.rest.strip_prefix(token) {
            self.rest = rest;
            true
        } else {
            false
        }
    }
    fn need(&mut self, token: &str) -> Result<(), String> {
        if self.take(token) {
            Ok(())
        } else {
            Err(format!("expected {token}"))
        }
    }
    fn hex(&mut self) -> Result<u32, String> {
        let digits = self.rest.get(..4).ok_or("short Unicode escape")?;
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("bad Unicode escape".into());
        }
        let n = u32::from_str_radix(digits, 16).map_err(|_| "bad Unicode escape")?;
        self.rest = &self.rest[4..];
        Ok(n)
    }
    fn string(&mut self) -> Result<String, String> {
        self.need("\"")?;
        let mut out = String::new();
        loop {
            let c = self.rest.chars().next().ok_or("unterminated JSON string")?;
            self.rest = &self.rest[c.len_utf8()..];
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let c = self.rest.chars().next().ok_or("unfinished JSON escape")?;
                    self.rest = &self.rest[c.len_utf8()..];
                    out.push(match c {
                        '"' | '\\' | '/' => c,
                        'b' => '\u{8}',
                        'f' => '\u{c}',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        'u' => {
                            let mut n = self.hex()?;
                            if (0xd800..=0xdbff).contains(&n) {
                                self.rest = self
                                    .rest
                                    .strip_prefix("\\u")
                                    .ok_or("missing low surrogate")?;
                                let low = self.hex()?;
                                if !(0xdc00..=0xdfff).contains(&low) {
                                    return Err("invalid low surrogate".into());
                                }
                                n = 0x10000 + ((n - 0xd800) << 10) + low - 0xdc00;
                            }
                            char::from_u32(n).ok_or("invalid Unicode escape")?
                        }
                        _ => return Err("invalid JSON escape".into()),
                    });
                }
                c if c < '\u{20}' => return Err("unescaped control in JSON string".into()),
                c => out.push(c),
            }
        }
    }
    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > 64 {
            return Err("JSON nesting too deep".into());
        }
        self.ws();
        if self.rest.starts_with('"') {
            return self.string().map(Json::String);
        }
        if self.take("null") {
            return Ok(Json::Null);
        }
        if self.take("true") {
            return Ok(Json::Bool(true));
        }
        if self.take("false") {
            return Ok(Json::Bool(false));
        }
        if self.take("[") {
            let mut out = Vec::new();
            if !self.take("]") {
                loop {
                    out.push(self.value(depth + 1)?);
                    if self.take("]") {
                        break;
                    }
                    self.need(",")?;
                }
            }
            return Ok(Json::Array(out));
        }
        if self.take("{") {
            let mut out = BTreeMap::new();
            if !self.take("}") {
                loop {
                    let key = self.string()?;
                    self.need(":")?;
                    if out.insert(key, self.value(depth + 1)?).is_some() {
                        return Err("duplicate JSON key".into());
                    }
                    if self.take("}") {
                        break;
                    }
                    self.need(",")?;
                }
            }
            return Ok(Json::Object(out));
        }
        let text = self.rest;
        let bytes = text.as_bytes();
        let mut n = usize::from(bytes.first() == Some(&b'-'));
        match bytes.get(n) {
            Some(b'0') => n += 1,
            Some(b'1'..=b'9') => {
                while bytes.get(n).is_some_and(u8::is_ascii_digit) {
                    n += 1;
                }
            }
            _ => return Err("expected JSON value".into()),
        }
        if bytes.get(n) == Some(&b'.') {
            n += 1;
            let first = n;
            while bytes.get(n).is_some_and(u8::is_ascii_digit) {
                n += 1;
            }
            if n == first {
                return Err("missing fractional digits".into());
            }
        }
        if matches!(bytes.get(n), Some(b'e' | b'E')) {
            n += 1;
            if matches!(bytes.get(n), Some(b'+' | b'-')) {
                n += 1;
            }
            let first = n;
            while bytes.get(n).is_some_and(u8::is_ascii_digit) {
                n += 1;
            }
            if n == first {
                return Err("missing exponent digits".into());
            }
        }
        self.rest = &text[n..];
        Ok(Json::Number(text[..n].into()))
    }
}

fn decode_map(text: &str) -> Result<TraceMap, String> {
    let mut reader = JsonReader {
        rest: text.strip_prefix('\u{feff}').unwrap_or(text),
    };
    let json = reader.value(0)?;
    reader.ws();
    if !reader.rest.is_empty() {
        return Err("trailing JSON data".into());
    }
    let mut map = TraceMap::default();
    for e in json.field("entries")?.array()? {
        let carrier = e.field("carrier")?;
        let carrier = match carrier.field("kind")?.string()? {
            "Spawn" => TraceCarrier::Spawn,
            "Objective" => {
                let s = carrier.field("style")?;
                TraceCarrier::Objective(ObjectiveStyle {
                    coalition: s.field("coalition")?.int()?,
                    success: s.field("success")?.int()?,
                    lc_name: s.field("lc_name")?.optional(Json::int)?,
                })
            }
            _ => return Err("unknown trace carrier".into()),
        };
        let pos = e.field("pos")?.array()?;
        if pos.len() != 2 {
            return Err("pos must be [x,z]".into());
        }
        map.push(TraceEntry {
            breadcrumb_index: e.field("breadcrumb_index")?.int()?,
            source_index: e.field("source_index")?.int()?,
            source_name: e.field("source_name")?.string()?.into(),
            source_type: e.field("source_type")?.string()?.into(),
            event_type: e.field("event_type")?.optional(Json::int)?,
            group_path: e
                .field("group_path")?
                .array()?
                .iter()
                .map(|s| s.string().map(str::to_string))
                .collect::<Result<_, _>>()?,
            pos: (pos[0].float()?, pos[1].float()?),
            carrier,
            spawn_name: e
                .field("spawn_name")?
                .optional(|s| s.string().map(str::to_string))?,
            expected_s: e.field("expected_s")?.optional(Json::float)?,
        });
    }
    for e in json.field("edges")?.array()? {
        map.edges.push(TraceEdge {
            from: e.field("from")?.int()?,
            to: e.field("to")?.int()?,
            unconditional: e.field("unconditional")?.boolean()?,
            delay_s: e.field("delay_s")?.float()?,
        });
    }
    validate_map(&map)?;
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_group_file;
    use crate::serialize::serialize_group;

    fn load(path: &str) -> Il2Entity {
        parse_group_file(
            &std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap(),
        )
        .unwrap()
    }

    fn node(root: &Il2Entity, id: i32) -> &Il2Entity {
        let mut nodes = HashMap::new();
        index_nodes(root, &mut nodes);
        nodes[&id]
    }

    fn source(index: i32) -> TraceSource {
        TraceSource {
            index,
            name: format!("Source {index}"),
            block_type: "MCU_Timer".into(),
            event_type: None,
            group_path: vec!["Test".into()],
            expected_s: None,
        }
    }

    fn selected(root: &mut Il2Entity, indexes: &[i32]) -> TraceMap {
        let mut map = TraceMap::default();
        let mut next = root.max_index() + 1;
        instrument(
            root,
            &TraceSelect {
                indexes: indexes.to_vec(),
                ..TraceSelect::default()
            },
            &mut next,
            TraceCarrier::default(),
            &mut map,
        );
        map
    }

    fn edge(map: &TraceMap, from: i32, to: i32) -> &TraceEdge {
        let crumb = |source| {
            map.entries
                .iter()
                .find(|e| e.source_index == source)
                .unwrap()
                .breadcrumb_index
        };
        map.edges
            .iter()
            .find(|e| e.from == crumb(from) && e.to == crumb(to))
            .unwrap()
    }

    #[test]
    fn trace_instrument_adds_exactly_one_breadcrumb_per_source() {
        let original = load("TemplateExamples/Exclusive_Activation_6plan.Group");
        let sel = TraceSelect::decision_points();
        assert_eq!(
            sel.block_types,
            [
                "MCU_CheckZone",
                "MCU_Counter",
                "MCU_TR_MissionBegin",
                "MCU_Spawner"
            ]
        );
        assert!(sel.random_timers);
        assert!(
            sel.name_prefixes.is_empty() && sel.indexes.is_empty() && sel.event_types.is_empty()
        );
        let mut expected = Vec::new();
        original.for_each(&mut |e| {
            if matches!(
                e.block_type.as_str(),
                "MCU_CheckZone" | "MCU_Counter" | "MCU_TR_MissionBegin" | "MCU_Spawner"
            ) || (e.block_type == "MCU_Timer"
                && e.property("Random").unwrap().parse::<f64>().unwrap() < 100.0)
            {
                expected.push(e.index.unwrap());
            }
        });
        assert!(!expected.is_empty());
        let mut root = original.clone();
        let mut map = TraceMap::default();
        let mut next = original.max_index() + 1;
        instrument(
            &mut root,
            &sel,
            &mut next,
            TraceCarrier::Objective(ObjectiveStyle::default()),
            &mut map,
        );
        assert_eq!(map.entries.len(), expected.len());
        assert_eq!(
            root.count_block_type("MCU_TR_MissionObjective")
                - original.count_block_type("MCU_TR_MissionObjective"),
            expected.len()
        );
        for entry in &map.entries {
            assert!(expected.contains(&entry.source_index));
            let before = node(&original, entry.source_index);
            let after = node(&root, entry.source_index);
            let mut targets = before.targets.clone();
            targets.push(entry.breadcrumb_index);
            assert_eq!(after.targets, targets);
            assert!(entry.breadcrumb_index > original.max_index());
        }
        root.children
            .retain(|e| e.index.is_none_or(|id| id <= original.max_index()));
        root.for_each_mut(&mut |e| {
            if let Some(entry) = map.entries.iter().find(|t| Some(t.source_index) == e.index) {
                assert_eq!(e.targets.last(), Some(&entry.breadcrumb_index));
                e.set_targets(e.targets[..e.targets.len() - 1].to_vec());
            }
        });
        assert_eq!(serialize_group(&root), serialize_group(&original));

        // Caller prefixes are OR rules; directly started cue timers get Time.
        let mut cues = parse_group_file(
            r#"Group { Name = "Cues";
          MCU_TR_MissionBegin { Index = 1; Targets = [2]; }
          MCU_Timer { Index = 2; Name = "CUE first"; Targets = [3]; Time = 275; Random = 100; }
          MCU_Timer { Index = 3; Name = "CUE later"; Time = 2; Random = 100; }
        }"#,
        )
        .unwrap();
        let mut sel = TraceSelect::decision_points();
        sel.name_prefixes.push("CUE ".into());
        let mut map = TraceMap::default();
        instrument(&mut cues, &sel, &mut 10, TraceCarrier::default(), &mut map);
        assert_eq!(
            map.entries
                .iter()
                .find(|e| e.source_index == 2)
                .unwrap()
                .expected_s,
            Some(275.0)
        );
        assert_eq!(
            map.entries
                .iter()
                .find(|e| e.source_index == 3)
                .unwrap()
                .expected_s,
            None
        );
    }

    #[test]
    fn trace_breadcrumbs_are_unique_and_inside_the_map() {
        assert_eq!((TRACE_X0, TRACE_Z0), (5_000.0, 5_000.0));
        assert_eq!((crate::geo::MAP_MIN, crate::geo::MAP_MAX), (0.0, 499_200.0));
        assert_eq!(
            (crate::placement::MAP_MIN, crate::placement::MAP_MAX),
            (40_000.0, 470_000.0)
        );
        let mut map = TraceMap::default();
        let mut next = 10_000;
        let mut ids = HashSet::new();
        let mut positions = HashSet::new();
        for n in 0..6001 {
            let blocks = breadcrumb(
                TraceCarrier::Objective(ObjectiveStyle::default()),
                &mut map,
                &mut next,
                source(n),
            );
            assert_eq!(blocks.len(), 1);
            let e = map.entries.last().unwrap();
            assert!(ids.insert(e.breadcrumb_index));
            assert!(positions.insert((e.pos.0 as i64, e.pos.1 as i64)));
            assert_eq!(
                e.pos,
                (
                    5_000.0 + 10.0 * (n % 3000) as f64,
                    5_000.0 + 10.0 * (n / 3000) as f64
                )
            );
            assert!(crate::geo::on_map(e.pos.0, e.pos.1));
            let placed = crate::placement::MAP_MIN..=crate::placement::MAP_MAX;
            assert!(!(placed.contains(&e.pos.0) && placed.contains(&e.pos.1)));
            assert_eq!(blocks[0].pos_xz(), Some(e.pos));
        }
        assert_eq!(map.entries[3000].pos, (5000.0, 5010.0));
        assert_eq!(map.entries[6000].pos, (5000.0, 5020.0));
    }

    #[test]
    fn trace_entity_source_gets_an_onevent() {
        let original = parse_group_file(
            r#"Group { Name = "Plane group";
          Plane { Index = 1; Name = "Plane"; LinkTrId = 2; }
          MCU_TR_Entity { Index = 2; Name = "Plane entity"; Targets = []; MisObjID = 1;
            OnEvents { OnEvent { Type = 4; TarId = 30; } }
            OnReports { OnReport { Type = 4; CmdId = 6; TarId = 40; } }
          }
        }"#,
        )
        .unwrap();
        let mut root = original.clone();
        let mut map = TraceMap::default();
        instrument(
            &mut root,
            &TraceSelect {
                indexes: vec![2],
                event_types: vec![4, 4],
                ..TraceSelect::default()
            },
            &mut 100,
            TraceCarrier::Objective(ObjectiveStyle::default()),
            &mut map,
        );
        assert_eq!(map.entries.len(), 1);
        assert_eq!(map.entries[0].event_type, Some(4));
        let id = map.entries[0].breadcrumb_index;
        assert!(node(&root, 2).targets.is_empty());
        let events = &root.children[1].children[0];
        assert_eq!(events.children.len(), 2);
        assert_eq!(integer(&events.children[1], "Type"), Some(4));
        assert_eq!(integer(&events.children[1], "TarId"), Some(id));
        root.children[1].children[0].children.pop();
        root.children.pop();
        assert_eq!(serialize_group(&root), serialize_group(&original));
    }

    #[test]
    fn trace_edges_follow_onevents_and_stop_on_loops() {
        let mut root = parse_group_file(
            r#"Group {
          MCU_TR_Entity { Index = 1;
            OnEvents { OnEvent { Type = 4; TarId = 2; } OnEvent { Type = 5; TarId = 4; } }
            OnReports { OnReport { Type = 4; CmdId = 90; TarId = 5; } }
          }
          MCU_Timer { Index = 2; Targets = [2,3]; Time = 0; Random = 100; }
          MCU_Timer { Index = 3; Targets = [2]; Time = 1; Random = 100; }
          MCU_Timer { Index = 4; Targets = []; Time = 2; Random = 100; }
          MCU_Timer { Index = 5; Targets = []; Time = 3; Random = 100; }
        }"#,
        )
        .unwrap();
        let mut map = TraceMap::default();
        instrument(
            &mut root,
            &TraceSelect {
                indexes: vec![1, 3, 4, 5],
                event_types: vec![4],
                ..TraceSelect::default()
            },
            &mut 100,
            TraceCarrier::default(),
            &mut map,
        );
        assert_eq!(edge(&map, 1, 3).delay_s, 1.0);
        assert!(edge(&map, 1, 3).unconditional);
        let entity_crumb = map.entries[0].breadcrumb_index;
        assert_eq!(
            map.edges.iter().filter(|e| e.from == entity_crumb).count(),
            1
        );
        assert!(map.edges.len() <= 2, "loop stops at nearest traced source");
        // An untraced entity's reports are also pulse links; a same-numbered
        // report is deliberately not attributed to the type-4 event above.
        map.entries[0].event_type = None;
        rebuild_edges(&root, &mut map);
        assert_eq!(edge(&map, 1, 4).delay_s, 2.0);
        assert_eq!(edge(&map, 1, 5).delay_s, 3.0);
    }

    #[test]
    fn trace_sidecar_round_trips() {
        let mut map = TraceMap::default();
        let mut next = 100;
        for coalition in 0..=2 {
            for success in 0..=1 {
                let style = ObjectiveStyle {
                    coalition,
                    success,
                    lc_name: (coalition == 2).then_some(123),
                };
                let mut s = source(coalition * 2 + success);
                s.name = "한글 \"quoted\" \\ path\n\t\u{1} 🚀".into();
                s.group_path = vec![String::new(), "A / B".into()];
                s.event_type = Some(4);
                s.expected_s = Some(275.125);
                let b = breadcrumb(TraceCarrier::Objective(style), &mut map, &mut next, s);
                let expected_keys = vec![
                    "Index",
                    "Targets",
                    "Objects",
                    "XPos",
                    "YPos",
                    "ZPos",
                    "XOri",
                    "YOri",
                    "ZOri",
                    "Enabled",
                    "LCName",
                    "LCDesc",
                    "TaskType",
                    "Coalition",
                    "Success",
                    "IconType",
                ];
                assert_eq!(
                    b[0].properties
                        .iter()
                        .map(|(key, _)| key.as_str())
                        .collect::<Vec<_>>(),
                    expected_keys
                );
                assert_eq!(integer(&b[0], "Coalition"), Some(coalition));
                assert_eq!(integer(&b[0], "Success"), Some(success));
                assert_eq!(integer(&b[0], "LCName"), Some(style.lc_name.unwrap_or(0)));
                assert_eq!(integer(&b[0], "LCDesc"), Some(0));
                assert_eq!(integer(&b[0], "TaskType"), Some(0));
                assert_eq!(integer(&b[0], "IconType"), Some(0));
            }
        }
        breadcrumb(TraceCarrier::Spawn, &mut map, &mut next, source(10));
        assert_eq!(
            map.entries.last().unwrap().spawn_name.as_deref(),
            Some("TRACE 6")
        );
        map.edges.push(TraceEdge {
            from: 100,
            to: 101,
            unconditional: false,
            delay_s: 0.05,
        });
        let group =
            std::env::temp_dir().join(format!("il2_trace_roundtrip_{}.Group", std::process::id()));
        write_trace_sidecar(&group, &map).unwrap();
        assert_eq!(
            read_trace_sidecar(&group.with_extension("trace.json")).unwrap(),
            map
        );
        std::fs::remove_file(group.with_extension("trace.json")).unwrap();
        let encoded = encode_map(&map);
        assert_eq!(
            decode_map(&encoded.replace('🚀', "\\ud83d\\ude80")).unwrap(),
            map
        );
        for bad in [
            "{}",
            "{\"entries\":[],\"edges\":[],}",
            "[]",
            "null",
            "{\"entries\":[],\"entries\":[],\"edges\":[]}",
        ] {
            assert!(decode_map(bad).is_err());
        }
        assert!(decode_map(&(encoded.clone() + "garbage")).is_err());
        assert!(decode_map(&encoded.replace("275.125", "1e999")).is_err());
        let mut bad = map.clone();
        bad.entries[0].pos.0 = f64::NAN;
        assert!(write_trace_sidecar(&group, &bad).is_err());
        bad = map.clone();
        bad.edges[0].to = -99;
        assert!(decode_map(&encode_map(&bad)).is_err());
        bad = map.clone();
        bad.push(map.entries[0].clone());
        assert!(decode_map(&encode_map(&bad)).is_err());
    }

    #[test]
    fn trace_edges_skip_untraced_mcus() {
        let mut root = parse_group_file(
            r#"Group {
          MCU_Timer { Index = 1; Targets = [2]; Time = 99; Random = 50; }
          MCU_Timer { Index = 2; Targets = [3]; Time = 2; Random = 100; }
          MCU_Timer { Index = 3; Targets = []; Time = 3; Random = 100; }
        }"#,
        )
        .unwrap();
        let map = selected(&mut root, &[1, 3]);
        assert_eq!(map.edges.len(), 1);
        assert_eq!(edge(&map, 1, 3).delay_s, 5.0);
        assert!(
            edge(&map, 1, 3).unconditional,
            "source already fired, so exclude its random roll and delay"
        );
    }

    #[test]
    fn trace_spawn_carrier_deletes_its_object() {
        assert_eq!(TraceCarrier::default(), TraceCarrier::Spawn);
        let mut map = TraceMap::default();
        let blocks = breadcrumb(TraceCarrier::default(), &mut map, &mut 100, source(1));
        assert_eq!(blocks.len(), 5);
        let [spawn, vehicle, entity, delay, delete] = blocks.as_slice() else {
            unreachable!()
        };
        assert_eq!(spawn.block_type, "MCU_Spawner");
        assert_eq!(spawn.objects, vec![entity.index.unwrap()]);
        assert_eq!(spawn.property("SpawnAtMe"), Some("0"));
        assert_eq!(vehicle.block_type, "Vehicle");
        assert_eq!(vehicle.name(), Some("TRACE 0"));
        assert_eq!(vehicle.property("Country"), Some("0"));
        assert_eq!(vehicle.property("Engageable"), Some("0"));
        assert_eq!(vehicle.property("Vulnerable"), Some("0"));
        assert_eq!(vehicle.property("PinToTerrain"), Some("1"));
        assert_eq!(integer(vehicle, "LinkTrId"), entity.index);
        assert_eq!(integer(entity, "MisObjID"), vehicle.index);
        assert_eq!(entity.property("Enabled"), Some("0"));
        assert_eq!(vehicle.pos_xz(), Some(TRACE_SPAWN_POINT));
        assert_eq!(entity.pos_xz(), Some(TRACE_SPAWN_POINT));
        let hook = event_hooks(entity).next().unwrap();
        assert_eq!(hook.block_type, "OnReport");
        assert_eq!(integer(hook, "Type"), Some(0));
        assert_eq!(integer(hook, "CmdId"), spawn.index);
        assert_eq!(integer(hook, "TarId"), delay.index);
        assert_eq!(delay.property("Time"), Some("1"));
        assert_eq!(delay.targets, vec![delete.index.unwrap()]);
        assert_eq!(delete.block_type, "MCU_Delete");
        assert_eq!(delete.objects, vec![vehicle.index.unwrap()]);
        let sample = load("TemplateExamples/K14 AFB_mp.Group");
        for key in ["Script", "Model", "ModMask", "TrailerAtStart"] {
            assert_eq!(vehicle.property(key), node(&sample, 117).property(key));
        }
        assert_eq!(TRACE_SPAWN_POINT, (400_000.0, 50_000.0));
        let water = crate::watermap::WaterMap::builtin().unwrap();
        assert_eq!(
            water.world_to_cell(TRACE_SPAWN_POINT.0, TRACE_SPAWN_POINT.1),
            Some((450, 897))
        );
        assert!(crate::geo::on_map(TRACE_SPAWN_POINT.0, TRACE_SPAWN_POINT.1));
        assert!(water.is_land_xz(TRACE_SPAWN_POINT.0, TRACE_SPAWN_POINT.1));
        assert!(water.is_open_xz(TRACE_SPAWN_POINT.0, TRACE_SPAWN_POINT.1));
        assert!(!water.is_water_xz(TRACE_SPAWN_POINT.0, TRACE_SPAWN_POINT.1));
    }

    #[test]
    fn trace_edges_do_not_follow_state_links() {
        let mut root = load("TemplateExamples/Eastern_Fighters_Random_3pack_V6.Group");
        crate::flights::configure_aircraft(&mut root, &crate::flights::FlightConfig::default())
            .unwrap();
        let mut map = TraceMap::default();
        let mut next = root.max_index() + 1;
        instrument(
            &mut root,
            &TraceSelect {
                name_prefixes: vec!["Random ".into()],
                block_types: vec!["MCU_Spawner".into()],
                ..TraceSelect::default()
            },
            &mut next,
            TraceCarrier::default(),
            &mut map,
        );
        let randoms: Vec<_> = map
            .entries
            .iter()
            .filter(|e| e.source_name.starts_with("Random "))
            .collect();
        assert_eq!(randoms.len(), 4);
        for random in randoms {
            let source = node(&root, random.source_index);
            let own_out = node(&root, source.targets[0]);
            let own_spawn = own_out.targets[0];
            let outgoing: Vec<_> = map
                .edges
                .iter()
                .filter(|e| e.from == random.breadcrumb_index)
                .collect();
            assert_eq!(outgoing.len(), 1, "closer must not pulse other spawners");
            assert_eq!(
                map.entries
                    .iter()
                    .find(|e| e.breadcrumb_index == outgoing[0].to)
                    .unwrap()
                    .source_index,
                own_spawn
            );
        }
        for kind in [
            "MCU_Activate",
            "MCU_Deactivate",
            "MCU_ModifierSetVal",
            "MCU_Delete",
            "MCU_CMD_AttackArea",
        ] {
            let mut root = parse_group_file(&format!("Group {{ MCU_Timer {{ Index = 1; Targets = [2]; Time = 0; Random = 100; }} {kind} {{ Index = 2; Targets = [3]; }} MCU_Timer {{ Index = 3; Time = 0; Random = 100; }} }}")).unwrap();
            assert!(selected(&mut root, &[1, 3]).edges.is_empty(), "{kind}");
        }
    }

    #[test]
    fn trace_edge_is_unconditional_only_without_gates_and_counters() {
        let original = parse_group_file(
            r#"Group {
          MCU_Timer { Index = 1; Targets = [2]; Time = 0; Random = 100; }
          MCU_Timer { Index = 2; Targets = [3]; Time = 2; Random = 100; }
          MCU_Timer { Index = 3; Targets = []; Time = 3; Random = 100; }
        }"#,
        )
        .unwrap();
        for variant in [
            "plain",
            "gate",
            "counter",
            "random",
            "waypoint",
            "checkzone",
            "destination gate",
            "disabled relay",
            "disabled destination",
        ] {
            let mut root = original.clone();
            match variant {
                "gate" | "destination gate" => {
                    let mut off = mcu("MCU_Deactivate", "OFF", &mut 9, 0.0, 0.0);
                    off.set_targets(vec![if variant == "gate" { 2 } else { 3 }]);
                    root.children.push(off);
                }
                "counter" => root.children[1].block_type = "MCU_Counter".into(),
                "random" => root.children[1].set_property("Random", "50"),
                "waypoint" => root.children[1].block_type = "MCU_Waypoint".into(),
                "checkzone" => root.children[1].block_type = "MCU_CheckZone".into(),
                "disabled relay" => root.children[1].set_property("Enabled", "0"),
                "disabled destination" => root.children[2].set_property("Enabled", "0"),
                _ => {}
            }
            let map = selected(&mut root, &[1, 3]);
            assert_eq!(
                edge(&map, 1, 3).unconditional,
                variant == "plain",
                "{variant}"
            );
            assert_eq!(
                edge(&map, 1, 3).delay_s,
                if matches!(variant, "counter" | "waypoint" | "checkzone") {
                    3.0
                } else {
                    5.0
                }
            );
        }
        // A gated route must not hide an independent unconditional route.
        let mut root = original;
        let mut alternate = timer("alternate", 4.0, &mut 9, 0.0, 0.0);
        alternate.set_targets(vec![3]);
        root.children[0].append_target(9);
        root.children[1].set_property("Random", "50");
        root.children.push(alternate);
        let map = selected(&mut root, &[1, 3]);
        assert_eq!(map.edges.len(), 2);
        assert!(
            map.edges
                .iter()
                .any(|e| e.unconditional && e.delay_s == 7.0)
        );
        assert!(
            map.edges
                .iter()
                .any(|e| !e.unconditional && e.delay_s == 5.0)
        );
    }
}
