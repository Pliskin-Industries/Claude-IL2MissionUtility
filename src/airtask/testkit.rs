//! Test-only mission-tree checks and the P14 step 1b dry-run walker.

use std::collections::HashSet;

use crate::ast::Il2Entity;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Integer time throughout the queue and public inputs: one tick is 0.05 s.
pub(crate) type Tick = u64;
pub(crate) const TICKS_PER_SECOND: Tick = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TimerRetrigger {
    #[default]
    Restart,
    Ignore,
    Twice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DeactivateRunningTimer {
    #[default]
    RunsOn,
    Cancels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DropcountMeaning {
    #[default]
    ResetOn1,
    ResetOn0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SetValRefires {
    #[default]
    Yes,
    No,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SameTickCounts {
    #[default]
    Each,
    One,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SameTickDelivery {
    #[default]
    DepthFirst,
    Batched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CheckZoneMode {
    #[default]
    Once,
    Repeat,
}

/// These are hypotheses from the step 1b rule table, not game-engine claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct WalkerConfig {
    pub timer_retrigger: TimerRetrigger,                  // risk p
    pub deactivate_running_timer: DeactivateRunningTimer, // risk j (J1)
    pub dropcount_meaning: DropcountMeaning,              // risk e
    pub set_val_refires: SetValRefires,                   // risk e (E6)
    pub same_tick_counts: SameTickCounts,                 // risk r
    pub same_tick_delivery: SameTickDelivery,             // risk q
    pub reverse_targets: bool,                            // risk q
    pub check_zone: CheckZoneMode,                        // risks b, c, o
}

/// Names must be unique among indexed nodes. Use Index for repeated pack names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WalkerNode {
    Name(String),
    Index(i32),
}

impl From<&str> for WalkerNode {
    fn from(name: &str) -> Self {
        Self::Name(name.to_string())
    }
}

impl From<i32> for WalkerNode {
    fn from(index: i32) -> Self {
        Self::Index(index)
    }
}

/// Samples are exactly one second apart. Before its first sample a track is
/// absent; afterwards its last position is held, without interpolation or an
/// implicit departure at the end of the script. Coalition IDs are the file IDs.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScriptedTrack {
    pub coalition: i32,
    pub samples: Vec<(Tick, f64, f64)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct WalkerInput {
    pub until: Tick,
    pub seed: u64,
    pub tracks: Vec<ScriptedTrack>,
    pub pulses: Vec<(Tick, WalkerNode)>,
    /// A Plane or its linked MCU_TR_Entity. Scripted deaths do not simulate
    /// existence/spawning; each entity emits Type 4 at most once per run.
    pub kills: Vec<(Tick, WalkerNode)>,
    pub arrivals: Vec<(Tick, WalkerNode)>,
}

/// Successful output pulses: timers at expiry, zones when satisfied, counters
/// at threshold, other MCUs when invoked. Dropped inputs are not firings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WalkerPulse {
    pub t: Tick,
    pub index: i32,
    pub name: String,
}

/// State after all work (including batched switches) at this tick. Index keys
/// disambiguate repeated names; there is also a snapshot at 0 and at `until`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WalkerSnapshot {
    pub t: Tick,
    pub active: BTreeMap<i32, bool>,
    pub counter_counts: BTreeMap<i32, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WalkerLog {
    pub pulses: Vec<WalkerPulse>,
    pub snapshots: Vec<WalkerSnapshot>,
}

#[derive(Default)]
struct NodeState {
    active: bool,
    running: BTreeSet<u64>,
    count: u32,
    fired: bool,
    last_count: Option<Tick>,
    zone_armed: bool,
    last_zone_fire: Option<Tick>,
}

#[derive(Clone, Copy)]
enum WalkEvent {
    Pulse(i32),
    TimerExpired(i32, u64),
    MissionBegin(i32),
    Kill(i32),
    Arrive(i32),
    SampleZones,
    SampleZone(i32),
}

type TrackUpdate = (usize, (f64, f64));

struct Walker<'a> {
    nodes: BTreeMap<i32, &'a Il2Entity>,
    states: BTreeMap<i32, NodeState>,
    config: WalkerConfig,
    input: &'a WalkerInput,
    scheduled: BTreeMap<Tick, VecDeque<WalkEvent>>,
    pending: VecDeque<WalkEvent>,
    track_updates: BTreeMap<Tick, Vec<TrackUpdate>>,
    positions: Vec<Option<(f64, f64)>>,
    switches: Vec<(i32, bool)>,
    killed: BTreeSet<i32>,
    now: Tick,
    serial: u64,
    rng: u64,
    log: WalkerLog,
}

fn indexed_nodes(root: &Il2Entity) -> BTreeMap<i32, &Il2Entity> {
    fn visit<'a>(node: &'a Il2Entity, nodes: &mut BTreeMap<i32, &'a Il2Entity>) {
        if let Some(id) = node.index {
            assert!(nodes.insert(id, node).is_none(), "duplicate Index {id}");
        }
        for child in &node.children {
            visit(child, nodes);
        }
    }
    let mut nodes = BTreeMap::new();
    visit(root, &mut nodes);
    nodes
}

fn value<T: std::str::FromStr>(node: &Il2Entity, key: &str, default: T) -> T {
    node.property(key).map_or(default, |raw| {
        raw.parse()
            .unwrap_or_else(|_| panic!("invalid {key}={raw} on {:?}", node.index))
    })
}

fn timer_ticks(node: &Il2Entity) -> Tick {
    let seconds: f64 = value(node, "Time", 0.0);
    assert!(seconds.is_finite() && seconds >= 0.0, "invalid timer Time");
    // Quantize once on input, rounding upwards so a sub-tick delay never fires
    // early (the Fighter Pack contains 0.02 s delays). Queue order is integer.
    (seconds * TICKS_PER_SECOND as f64).ceil() as Tick
}

fn state_links(node: &Il2Entity) -> impl Iterator<Item = i32> + '_ {
    node.targets.iter().chain(&node.objects).copied()
}

/// Run through `until` inclusively without modifying the tree. Simultaneous
/// track positions update atomically before MCU work. Initial event order is
/// Mission Begin (index order), forced pulses, kills, arrivals, zone samples;
/// within each input list insertion order breaks ties. Future timers join the
/// end of that tick's queue. DepthFirst descendants precede siblings; Batched
/// descendants join the tail and Activate/Deactivate commit at tick end.
/// Counts, resets and timer bookkeeping still accumulate in delivery order.
/// Panics on invalid/ambiguous references, ComplexTrigger (step 12), or a
/// million processed events (a bounded diagnostic for accidental cycles).
pub(crate) fn walk(root: &Il2Entity, config: WalkerConfig, input: &WalkerInput) -> WalkerLog {
    assert_links_resolve(root);
    let nodes = indexed_nodes(root);
    let mut walker = Walker {
        states: nodes
            .iter()
            .map(|(&id, node)| {
                assert_ne!(node.block_type, "MCU_TR_ComplexTrigger", "step 12 only");
                (
                    id,
                    NodeState {
                        active: value(node, "Enabled", 1) != 0,
                        ..NodeState::default()
                    },
                )
            })
            .collect(),
        nodes,
        config,
        input,
        scheduled: BTreeMap::new(),
        pending: VecDeque::new(),
        track_updates: BTreeMap::new(),
        positions: vec![None; input.tracks.len()],
        switches: Vec::new(),
        killed: BTreeSet::new(),
        now: 0,
        serial: 0,
        rng: input.seed,
        log: WalkerLog::default(),
    };
    walker.scheduled.entry(0).or_default();
    walker.scheduled.entry(input.until).or_default();
    for (&id, node) in &walker.nodes {
        if node.block_type == "MCU_TR_MissionBegin" {
            walker
                .scheduled
                .entry(0)
                .or_default()
                .push_back(WalkEvent::MissionBegin(id));
        }
    }
    for (events, kind) in [
        (&input.pulses, WalkEvent::Pulse as fn(i32) -> WalkEvent),
        (&input.kills, WalkEvent::Kill),
        (&input.arrivals, WalkEvent::Arrive),
    ] {
        for (t, target) in events {
            let id = walker.resolve(target);
            walker.scheduled.entry(*t).or_default().push_back(kind(id));
        }
    }
    for (i, track) in input.tracks.iter().enumerate() {
        for pair in track.samples.windows(2) {
            assert_eq!(
                pair[1].0.checked_sub(pair[0].0),
                Some(TICKS_PER_SECOND),
                "track samples must be one second apart"
            );
        }
        for &(t, x, z) in &track.samples {
            assert!(x.is_finite() && z.is_finite(), "non-finite track position");
            walker.track_updates.entry(t).or_default().push((i, (x, z)));
        }
    }
    for &t in walker.track_updates.keys() {
        walker
            .scheduled
            .entry(t)
            .or_default()
            .push_back(WalkEvent::SampleZones);
    }
    let mut processed = 0;
    while let Some((t, events)) = walker.scheduled.pop_first() {
        if t > input.until {
            break;
        }
        walker.now = t;
        walker.pending = events;
        if let Some(updates) = walker.track_updates.remove(&t) {
            for (i, xz) in updates {
                walker.positions[i] = Some(xz);
            }
        }
        while let Some(event) = walker.pending.pop_front() {
            processed += 1;
            assert!(processed <= 1_000_000, "walker event limit at tick {t}");
            walker.deliver(event);
        }
        for (id, active) in std::mem::take(&mut walker.switches) {
            walker.switch(id, active);
        }
        walker.log.snapshots.push(WalkerSnapshot {
            t,
            active: walker
                .states
                .iter()
                .map(|(&id, s)| (id, s.active))
                .collect(),
            counter_counts: walker
                .states
                .iter()
                .filter(|(id, _)| walker.nodes[id].block_type == "MCU_Counter")
                .map(|(&id, s)| (id, s.count))
                .collect(),
        });
    }
    walker.log
}

impl Walker<'_> {
    fn resolve(&self, target: &WalkerNode) -> i32 {
        match target {
            WalkerNode::Index(id) => {
                assert!(self.nodes.contains_key(id), "unknown walker Index {id}");
                *id
            }
            WalkerNode::Name(name) => {
                let matches: Vec<_> = self
                    .nodes
                    .iter()
                    .filter(|(_, node)| node.name() == Some(name.as_str()))
                    .map(|(&id, _)| id)
                    .collect();
                assert_eq!(
                    matches.len(),
                    1,
                    "walker name {name:?} must identify one node; matches {matches:?}"
                );
                matches[0]
            }
        }
    }

    fn enqueue_now(&mut self, events: Vec<WalkEvent>) {
        match self.config.same_tick_delivery {
            SameTickDelivery::DepthFirst => {
                for event in events.into_iter().rev() {
                    self.pending.push_front(event);
                }
            }
            SameTickDelivery::Batched => self.pending.extend(events),
        }
    }

    fn targets(&self, id: i32) -> Vec<i32> {
        let mut targets = self.nodes[&id].targets.clone();
        if self.config.reverse_targets {
            targets.reverse();
        }
        targets
    }

    fn record(&mut self, id: i32) {
        self.log.pulses.push(WalkerPulse {
            t: self.now,
            index: id,
            name: self.nodes[&id].name().unwrap_or("<unnamed>").to_string(),
        });
    }

    fn fire(&mut self, id: i32) {
        self.record(id);
        let targets = self.targets(id).into_iter().map(WalkEvent::Pulse).collect();
        self.enqueue_now(targets);
    }

    fn switch(&mut self, id: i32, active: bool) {
        let state = self.states.get_mut(&id).expect("state link resolves");
        state.active = active;
        if !active {
            state.zone_armed = false;
            if self.config.deactivate_running_timer == DeactivateRunningTimer::Cancels {
                state.running.clear();
            }
        }
        // Activation alone neither replays a dropped pulse nor pulses a zone.
    }

    fn roll(&mut self, percent: u32) -> bool {
        assert!(percent <= 100, "Random must be a percentage");
        if percent == 100 {
            return true;
        }
        // SplitMix64: local, fixed algorithm; no platform or hash-order seed.
        self.rng = self.rng.wrapping_add(0x9e3779b97f4a7c15);
        let mut n = self.rng;
        n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
        n ^= n >> 31;
        n % 100 < u64::from(percent)
    }

    fn deliver(&mut self, event: WalkEvent) {
        match event {
            WalkEvent::Pulse(id) => self.pulse(id),
            WalkEvent::MissionBegin(id) => self.fire(id), // Enabled is irrelevant.
            WalkEvent::TimerExpired(id, token) => {
                if self.states.get_mut(&id).unwrap().running.remove(&token) {
                    self.fire(id); // RunsOn deliberately ignores current active.
                }
            }
            WalkEvent::Kill(id) => self.kill(id),
            WalkEvent::Arrive(id) => {
                assert_eq!(self.nodes[&id].block_type, "MCU_Waypoint");
                if self.states[&id].active {
                    self.fire(id);
                }
            }
            WalkEvent::SampleZones => {
                let checks = self
                    .nodes
                    .iter()
                    .filter(|(_, n)| n.block_type == "MCU_CheckZone")
                    .map(|(&id, _)| WalkEvent::SampleZone(id))
                    .collect();
                self.enqueue_now(checks);
            }
            WalkEvent::SampleZone(id) => self.check_zone(id),
        }
    }

    fn pulse(&mut self, id: i32) {
        if !self.states[&id].active {
            return;
        }
        let node = self.nodes[&id];
        match node.block_type.as_str() {
            "MCU_Timer" => {
                // Every active trigger draws, even if Ignore subsequently drops
                // a successful roll. A failed roll leaves a running timer alone.
                if !self.roll(value(node, "Random", 100)) {
                    return;
                }
                let delay = timer_ticks(node);
                if delay == 0 {
                    self.fire(id); // A relay never remains running across inputs.
                    return;
                }
                let state = self.states.get_mut(&id).unwrap();
                if !state.running.is_empty() {
                    match self.config.timer_retrigger {
                        TimerRetrigger::Restart => state.running.clear(),
                        TimerRetrigger::Ignore => return,
                        TimerRetrigger::Twice => {}
                    }
                }
                self.serial += 1;
                state.running.insert(self.serial);
                let due = self.now.checked_add(delay).expect("timer tick overflow");
                self.scheduled
                    .entry(due)
                    .or_default()
                    .push_back(WalkEvent::TimerExpired(id, self.serial));
            }
            "MCU_Activate" | "MCU_Deactivate" => {
                self.record(id);
                let active = node.block_type == "MCU_Activate";
                let links = self
                    .targets(id)
                    .into_iter()
                    .chain(node.objects.iter().copied());
                for target in links {
                    if self.config.same_tick_delivery == SameTickDelivery::Batched {
                        self.switches.push((target, active));
                    } else {
                        self.switch(target, active);
                    }
                }
            }
            "MCU_CheckZone" => {
                // Re-pulsing a waiting zone replaces its watch. Once consumes
                // the watch when it fires; Repeat keeps it until deactivation.
                self.states.get_mut(&id).unwrap().zone_armed = true;
                self.check_zone(id);
            }
            "MCU_Counter" => self.count(id),
            "MCU_ModifierSetVal" => {
                assert_eq!(
                    value(node, "ParamIndex", 0),
                    0,
                    "only counter reset is modeled"
                );
                assert_eq!(value(node, "Data0", 0), 0, "only reset to zero is modeled");
                self.record(id);
                for target in state_links(node) {
                    assert_eq!(self.nodes[&target].block_type, "MCU_Counter");
                    let state = self.states.get_mut(&target).unwrap();
                    state.count = 0;
                    if self.config.set_val_refires == SetValRefires::Yes {
                        state.fired = false;
                    }
                }
            }
            // Commands (including Delete/Spawner), entities and waypoints are
            // observable sinks. Only an explicit arrival follows WP Targets.
            _ => self.record(id),
        }
    }

    fn count(&mut self, id: i32) {
        let node = self.nodes[&id];
        let threshold: u32 = value(node, "Counter", 1);
        assert!(threshold > 0, "Counter must be positive");
        let reset = value(node, "Dropcount", 0)
            == i32::from(self.config.dropcount_meaning == DropcountMeaning::ResetOn1);
        let state = self.states.get_mut(&id).unwrap();
        if state.fired
            || (self.config.same_tick_counts == SameTickCounts::One
                && state.last_count == Some(self.now))
        {
            return;
        }
        state.last_count = Some(self.now);
        state.count += 1;
        if state.count >= threshold {
            if reset {
                state.count = 0;
            } else {
                state.fired = true;
            }
            self.fire(id);
        }
    }

    fn check_zone(&mut self, id: i32) {
        let state = &self.states[&id];
        if !state.active
            || !state.zone_armed
            || (self.config.check_zone == CheckZoneMode::Repeat
                && state.last_zone_fire == Some(self.now))
        {
            return;
        }
        let node = self.nodes[&id];
        let raw = node.property("PlaneCoalitions").unwrap_or("[]");
        let inner = raw
            .trim()
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .expect("PlaneCoalitions must be an array");
        let coalitions: Vec<i32> = inner
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().parse().expect("coalition ID"))
            .collect();
        let (x, z) = node.pos_xz().expect("check zone needs XPos/ZPos");
        let radius: f64 = value(node, "Zone", 0.0);
        assert!(radius.is_finite() && radius >= 0.0, "invalid zone radius");
        let inside = self
            .positions
            .iter()
            .zip(&self.input.tracks)
            .any(|(pos, track)| {
                coalitions.contains(&track.coalition)
                    && pos.is_some_and(|(px, pz)| {
                        (px - x).powi(2) + (pz - z).powi(2) <= radius.powi(2)
                    })
            });
        if inside == (value(node, "Closer", 1) != 0) {
            let state = self.states.get_mut(&id).unwrap();
            state.zone_armed = self.config.check_zone == CheckZoneMode::Repeat;
            state.last_zone_fire = Some(self.now);
            self.fire(id);
        }
    }

    fn kill(&mut self, id: i32) {
        let node = self.nodes[&id];
        let entity = if node.block_type == "Plane" {
            let entity_id = value(node, "LinkTrId", -1);
            *self
                .nodes
                .get(&entity_id)
                .expect("plane's LinkTrId must resolve")
        } else {
            assert_eq!(
                node.block_type, "MCU_TR_Entity",
                "kill needs a plane entity"
            );
            assert!(
                self.nodes
                    .values()
                    .any(|plane| plane.block_type == "Plane" && value(plane, "LinkTrId", -1) == id),
                "entity must belong to a Plane"
            );
            node
        };
        assert_eq!(entity.block_type, "MCU_TR_Entity");
        let entity_id = entity.index.unwrap();
        if !self.killed.insert(entity_id) {
            return;
        }
        self.record(entity_id);
        let mut targets = Vec::new();
        for events in entity
            .children
            .iter()
            .filter(|n| n.block_type == "OnEvents")
        {
            for event in events.children.iter().filter(|n| n.block_type == "OnEvent") {
                if value(event, "Type", -1) == 4 {
                    let target = value(event, "TarId", -1);
                    assert!(
                        self.nodes.contains_key(&target),
                        "kill TarId {target} must resolve"
                    );
                    targets.push(WalkEvent::Pulse(target));
                }
            }
        }
        self.enqueue_now(targets);
    }
}

/// Structural D11 check: every deactivated zero-second `... OUT` relay needs
/// an incoming Activate link or an explicit single-use index supplied by its
/// builder. Exemptions are checked for stale/mistyped IDs. This checks wiring,
/// not whether the re-arm path is reachable or runs before the next cycle;
/// builder scenarios must verify that temporal part of the convention.
pub(crate) fn walker_every_deactivated_out_has_a_rearm_or_is_single_use(
    root: &Il2Entity,
    single_use: &[i32],
) -> Result<(), Vec<String>> {
    let nodes = indexed_nodes(root);
    let linked_by = |kind: &str| -> BTreeSet<i32> {
        nodes
            .values()
            .filter(|node| node.block_type == kind)
            .flat_map(|node| state_links(node))
            .collect()
    };
    let off = linked_by("MCU_Deactivate");
    let on = linked_by("MCU_Activate");
    let outs: BTreeSet<i32> = nodes
        .iter()
        .filter(|(id, node)| {
            off.contains(id)
                && node.block_type == "MCU_Timer"
                && node.name().is_some_and(|name| name.ends_with(" OUT"))
                && timer_ticks(node) == 0
                && value(node, "Random", 100) == 100
        })
        .map(|(&id, _)| id)
        .collect();
    let mut errors = Vec::new();
    for &id in &outs {
        if !on.contains(&id) && !single_use.contains(&id) {
            errors.push(format!(
                "{} (Index {id}) is deactivated but has no re-arm or single-use declaration",
                nodes[&id].name().unwrap()
            ));
        }
    }
    for id in single_use {
        if !outs.contains(id) {
            errors.push(format!(
                "single-use Index {id} is not a deactivated OUT relay"
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

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

#[cfg(test)]
mod walker_tests {
    use super::*;

    fn node(kind: &str, id: i32, name: &str, targets: &[i32]) -> Il2Entity {
        let mut next = id;
        let mut node = crate::template::mcu(kind, name, &mut next, 0.0, 0.0);
        node.set_targets(targets.to_vec());
        node
    }

    fn timer(id: i32, name: &str, seconds: &str, targets: &[i32]) -> Il2Entity {
        let mut timer = node("MCU_Timer", id, name, targets);
        timer.set_property("Time", seconds);
        timer.set_property("Random", "100");
        timer
    }

    fn tree(children: Vec<Il2Entity>) -> Il2Entity {
        let mut root = Il2Entity::new("Group");
        root.children = children;
        root
    }

    fn times(log: &WalkerLog, id: i32) -> Vec<Tick> {
        log.pulses
            .iter()
            .filter(|p| p.index == id)
            .map(|p| p.t)
            .collect()
    }

    fn track(coalition: i32, points: &[(f64, f64)]) -> ScriptedTrack {
        ScriptedTrack {
            coalition,
            samples: points
                .iter()
                .enumerate()
                .map(|(i, &(x, z))| (i as Tick * TICKS_PER_SECOND, x, z))
                .collect(),
        }
    }

    fn plane_entities(root: &Il2Entity) -> BTreeSet<i32> {
        let mut entities = BTreeSet::new();
        root.for_each(&mut |node| {
            // Airfield Plane loadout records have no Index or linked entity.
            if node.block_type == "Plane" && node.index.is_some() {
                entities.insert(value(node, "LinkTrId", -1));
            }
        });
        assert!(!entities.is_empty());
        assert!(!entities.contains(&-1));
        entities
    }

    fn grants<'a>(
        root: &Il2Entity,
        log: &'a WalkerLog,
        entities: &BTreeSet<i32>,
    ) -> Vec<&'a WalkerPulse> {
        let nodes = indexed_nodes(root);
        log.pulses
            .iter()
            .filter(|pulse| {
                let node = nodes[&pulse.index];
                matches!(node.block_type.as_str(), "MCU_Activate" | "MCU_Spawner")
                    && node.objects.iter().any(|id| entities.contains(id))
            })
            .collect()
    }

    #[test]
    fn walker_self_test_exclusive_activation_grants_one() {
        let root = crate::parser::parse_group_file(include_str!(
            "../../TemplateExamples/Exclusive_Activation_6plan.Group"
        ))
        .unwrap();
        let original = root.clone();
        let nodes = indexed_nodes(&root);
        for (drop, expected) in [(1, 13), (0, 4)] {
            assert_eq!(
                nodes
                    .values()
                    .filter(|node| node.block_type == "MCU_Counter"
                        && value(node, "Dropcount", -1) == drop)
                    .count(),
                expected
            );
        }
        // Extraction is for identifying the plans only. Walk the untouched
        // file, retaining every original link into its NodeGates mutex.
        let plans = crate::bombers::extract_exclusive_plans(&root).unwrap();
        assert_eq!(plans.len(), 6);
        let mut route = vec![(-1_000_000.0, -1_000_000.0); 10];
        let mut zone_count = 0;
        for plan in &plans {
            let info = crate::bombers::inspect_plan(plan).unwrap();
            assert!(!info.suggested_triggers.is_empty(), "{}", info.name);
            for id in info.suggested_triggers {
                let zone = nodes[&id];
                assert_eq!(zone.block_type, "MCU_CheckZone");
                route.push(zone.pos_xz().unwrap());
                zone_count += 1;
            }
        }
        route.extend([(-1_000_000.0, -1_000_000.0); 5]);
        let input = WalkerInput {
            until: (route.len() as Tick - 1) * TICKS_PER_SECOND,
            seed: 42,
            tracks: vec![track(2, &route), track(1, &route)], // NATO, DPRK
            ..WalkerInput::default()
        };
        let entities: Vec<_> = plans.iter().map(plane_entities).collect();
        for timer_retrigger in [
            TimerRetrigger::Restart,
            TimerRetrigger::Ignore,
            TimerRetrigger::Twice,
        ] {
            for reverse_targets in [false, true] {
                let config = WalkerConfig {
                    timer_retrigger,
                    reverse_targets,
                    ..WalkerConfig::default()
                };
                let log = walk(&root, config, &input);
                let counts: Vec<_> = entities
                    .iter()
                    .map(|ids| grants(&root, &log, ids).len())
                    .collect();
                assert_eq!(
                    counts.iter().filter(|&&n| n > 0).count(),
                    1,
                    "config {config:?}, plan grant counts {counts:?}"
                );
                if config == WalkerConfig::default() {
                    println!("Exclusive: {zone_count} zones, plan grant counts {counts:?}");
                    assert_eq!(log, walk(&root, config, &input));
                }
            }
        }
        assert_eq!(root, original, "walking must not edit the source tree");
    }

    #[test]
    fn walker_self_test_fighter_pack_nodegates_grants_one() {
        let root = crate::pack::generate_pack_at(
            &crate::pack::builtin_template().unwrap(),
            &[
                (80_000.0, 80_000.0),
                (200_000.0, 80_000.0),
                (320_000.0, 80_000.0),
            ],
            "Walker Fighters",
        )
        .unwrap();
        let groups: Vec<_> = (1..=3)
            .map(|i| root.find_by_name(&format!("Group {i}")).unwrap())
            .collect();
        let entities: Vec<_> = groups.iter().map(|group| plane_entities(group)).collect();
        for first in 0..groups.len() {
            // Keep the winner inside its Zone OUT while other tracks challenge
            // both closed gates a second later. Repeat with each gate first.
            let tracks = groups
                .iter()
                .enumerate()
                .map(|(i, group)| {
                    let centre = group.find_by_name("Zone IN").unwrap().pos_xz().unwrap();
                    let arrival = if i == first { 10 } else { 11 };
                    let points: Vec<_> = (0..=20)
                        .map(|t| {
                            if t < arrival {
                                (-1_000_000.0, -1_000_000.0)
                            } else {
                                centre
                            }
                        })
                        .collect();
                    track(2, &points)
                })
                .collect();
            let input = WalkerInput {
                until: 20 * TICKS_PER_SECOND,
                seed: 42,
                tracks,
                ..WalkerInput::default()
            };
            for timer_retrigger in [
                TimerRetrigger::Restart,
                TimerRetrigger::Ignore,
                TimerRetrigger::Twice,
            ] {
                for reverse_targets in [false, true] {
                    let config = WalkerConfig {
                        timer_retrigger,
                        reverse_targets,
                        ..WalkerConfig::default()
                    };
                    let log = walk(&root, config, &input);
                    let counts: Vec<_> = entities
                        .iter()
                        .map(|ids| grants(&root, &log, ids).len())
                        .collect();
                    let mut expected = vec![0; groups.len()];
                    expected[first] = 1;
                    assert_eq!(counts, expected, "gate {}, config {config:?}", first + 1);
                    for (i, group) in groups.iter().enumerate() {
                        if i != first {
                            let zone = group.find_by_name("Zone IN").unwrap().index.unwrap();
                            assert!(!log.snapshots.last().unwrap().active[&zone]);
                        }
                    }
                    if config == WalkerConfig::default() {
                        println!("Fighter gate {} first: grant counts {counts:?}", first + 1);
                    }
                }
            }
        }
    }

    #[test]
    fn walker_random_is_seeded_and_rolled_each_time() {
        let mut random = timer(1, "Random 50", "0", &[2]);
        random.set_property("Random", "50");
        let root = tree(vec![random, timer(2, "Hit", "0", &[])]);
        let input = WalkerInput {
            until: 40 * TICKS_PER_SECOND,
            seed: 42,
            pulses: (0..40)
                .map(|t| (t * TICKS_PER_SECOND, "Random 50".into()))
                .collect(),
            ..WalkerInput::default()
        };
        let log = walk(&root, WalkerConfig::default(), &input);
        let hits = times(&log, 2);
        assert!((12..=28).contains(&hits.len()), "{} hits", hits.len());
        assert_eq!(log, walk(&root, WalkerConfig::default(), &input));
        let other = WalkerInput {
            seed: 43,
            ..input.clone()
        };
        assert_ne!(
            hits,
            times(&walk(&root, WalkerConfig::default(), &other), 2)
        );
        println!("Random 50: {} hits from 40 pulses, seed 42", hits.len());
    }

    fn zone(id: i32, name: &str, closer: bool) -> Il2Entity {
        let mut next = id;
        crate::template::checkzone(name, 100.0, closer, "[2]", &mut next, 0.0, 0.0)
    }

    #[test]
    fn walker_checkzone_track() {
        let inside = (0.0, 0.0);
        let outside = (200.0, 0.0);
        let root = tree(vec![zone(1, "IN", true), zone(2, "OUT", false)]);
        let input = WalkerInput {
            until: 6 * TICKS_PER_SECOND,
            tracks: vec![
                track(
                    2,
                    &[inside, outside, outside, inside, outside, outside, outside],
                ),
                track(
                    2,
                    &[outside, outside, outside, inside, inside, outside, outside],
                ),
                track(1, &[inside; 7]), // The unobserved coalition stays inside.
            ],
            pulses: vec![(40, "IN".into()), (60, "OUT".into())],
            ..WalkerInput::default()
        };
        let log = walk(&root, WalkerConfig::default(), &input);
        assert_eq!(times(&log, 1), vec![60]); // No output before its pulse at 2 s.
        assert_eq!(times(&log, 2), vec![100]); // Last NATO leaves at 5 s.
        let repeat = walk(
            &root,
            WalkerConfig {
                check_zone: CheckZoneMode::Repeat,
                ..WalkerConfig::default()
            },
            &input,
        );
        assert_eq!(times(&repeat, 1), vec![60, 80]);
        assert_eq!(times(&repeat, 2), vec![100, 120]);
        println!("CheckZone: IN at 3 s; OUT at 5 s after the last watched track leaves");
    }

    #[test]
    fn walker_zone_rearm_requires_a_new_pulse_and_samples_are_atomic() {
        let root = tree(vec![
            zone(1, "IN", true),
            node("MCU_Deactivate", 2, "Off", &[1]),
            node("MCU_Activate", 3, "On", &[1]),
        ]);
        let input = WalkerInput {
            until: 100,
            tracks: vec![track(2, &[(0.0, 0.0); 5])],
            pulses: vec![
                (0, 1.into()),
                (20, 2.into()),
                (40, 3.into()),
                (60, 1.into()),
            ],
            ..WalkerInput::default()
        };
        for check_zone in [CheckZoneMode::Once, CheckZoneMode::Repeat] {
            let log = walk(
                &root,
                WalkerConfig {
                    check_zone,
                    ..WalkerConfig::default()
                },
                &input,
            );
            assert_eq!(
                times(&log, 1),
                if check_zone == CheckZoneMode::Once {
                    vec![0, 60]
                } else {
                    vec![0, 60, 80]
                }
            );
        }
        let root = tree(vec![zone(1, "OUT", false)]);
        let input = WalkerInput {
            until: 40,
            tracks: vec![
                track(2, &[(0.0, 0.0), (200.0, 0.0), (200.0, 0.0)]),
                track(2, &[(200.0, 0.0), (0.0, 0.0), (200.0, 0.0)]),
            ],
            pulses: vec![(0, 1.into())],
            ..WalkerInput::default()
        };
        assert_eq!(
            times(&walk(&root, WalkerConfig::default(), &input), 1),
            vec![40]
        );
    }

    #[test]
    fn walker_timer_retrigger_cancellation_and_out_gating() {
        let root = tree(vec![
            timer(1, "Delay", "2", &[2]),
            timer(2, "Hit", "0", &[]),
            node("MCU_Deactivate", 3, "Off", &[1]),
            node("MCU_Activate", 4, "On", &[1]),
        ]);
        for timer_retrigger in [
            TimerRetrigger::Restart,
            TimerRetrigger::Ignore,
            TimerRetrigger::Twice,
        ] {
            let expected = match timer_retrigger {
                TimerRetrigger::Restart => vec![60],
                TimerRetrigger::Ignore => vec![40],
                TimerRetrigger::Twice => vec![40, 60],
            };
            let input = WalkerInput {
                until: 80,
                pulses: vec![(0, 1.into()), (20, 1.into())],
                ..WalkerInput::default()
            };
            let config = WalkerConfig {
                timer_retrigger,
                ..WalkerConfig::default()
            };
            assert_eq!(times(&walk(&root, config, &input), 2), expected);
            for deactivate_running_timer in [
                DeactivateRunningTimer::RunsOn,
                DeactivateRunningTimer::Cancels,
            ] {
                let config = WalkerConfig {
                    deactivate_running_timer,
                    ..config
                };
                let input = WalkerInput {
                    until: 140,
                    pulses: vec![
                        (0, 1.into()),
                        (20, 1.into()),
                        (30, 3.into()),
                        (35, 1.into()),
                        (70, 4.into()),
                        (90, 1.into()),
                    ],
                    ..WalkerInput::default()
                };
                let mut wanted = if deactivate_running_timer == DeactivateRunningTimer::RunsOn {
                    expected.clone()
                } else {
                    vec![]
                };
                wanted.push(130);
                assert_eq!(times(&walk(&root, config, &input), 2), wanted, "{config:?}");
                let mut gated = root.clone();
                gated.find_by_name_mut("Off").unwrap().set_targets(vec![2]);
                gated.find_by_name_mut("On").unwrap().set_targets(vec![2]);
                // The original delay runs; deactivating its OUT blocks output.
                let input = WalkerInput {
                    pulses: vec![
                        (0, 1.into()),
                        (30, 3.into()),
                        (70, 4.into()),
                        (90, 1.into()),
                    ],
                    ..input
                };
                assert_eq!(times(&walk(&gated, config, &input), 2), vec![130]);
            }
        }
        let root = tree(vec![
            timer(1, "Subtick", "0.02", &[2]),
            timer(2, "Hit", "0", &[]),
        ]);
        let input = WalkerInput {
            until: 1,
            pulses: vec![(0, 1.into())],
            ..WalkerInput::default()
        };
        assert_eq!(
            times(&walk(&root, WalkerConfig::default(), &input), 2),
            vec![1]
        );
    }

    #[test]
    fn walker_same_tick_latch_and_target_order() {
        let root = tree(vec![
            timer(1, "Requests", "0", &[2, 2]),
            timer(2, "Latch", "0", &[3, 4]),
            node("MCU_Deactivate", 3, "Close", &[2]),
            timer(4, "Grant", "0", &[]),
        ]);
        let input = WalkerInput {
            until: 1,
            pulses: vec![(0, 1.into())],
            ..WalkerInput::default()
        };
        for same_tick_delivery in [SameTickDelivery::DepthFirst, SameTickDelivery::Batched] {
            for reverse_targets in [false, true] {
                let config = WalkerConfig {
                    same_tick_delivery,
                    reverse_targets,
                    ..WalkerConfig::default()
                };
                let log = walk(&root, config, &input);
                assert_eq!(
                    times(&log, 4).len(),
                    if same_tick_delivery == SameTickDelivery::DepthFirst {
                        1
                    } else {
                        2
                    }
                );
                assert!(!log.snapshots.last().unwrap().active[&2]);
            }
        }
        let mut gate = timer(3, "Gate", "0", &[4]);
        gate.set_property("Enabled", "0");
        let mut activate = node("MCU_Activate", 2, "Open", &[3]);
        activate.set_objects(vec![5]);
        let mut entity = node("MCU_TR_Entity", 5, "Object", &[]);
        entity.set_property("Enabled", "0");
        let root = tree(vec![
            timer(1, "Fanout", "0", &[2, 3]),
            activate,
            gate,
            timer(4, "Grant", "0", &[]),
            entity,
        ]);
        let input = WalkerInput {
            until: 1,
            pulses: vec![(0, 1.into()), (1, 3.into())],
            ..WalkerInput::default()
        };
        for same_tick_delivery in [SameTickDelivery::DepthFirst, SameTickDelivery::Batched] {
            for reverse_targets in [false, true] {
                let config = WalkerConfig {
                    same_tick_delivery,
                    reverse_targets,
                    ..WalkerConfig::default()
                };
                let log = walk(&root, config, &input);
                let expected =
                    if same_tick_delivery == SameTickDelivery::DepthFirst && !reverse_targets {
                        vec![0, 1]
                    } else {
                        vec![1]
                    };
                assert_eq!(times(&log, 4), expected);
                assert!(
                    log.snapshots[0].active[&5],
                    "Objects links also switch state"
                );
            }
        }
    }

    fn counter(id: i32, drop: i32) -> Il2Entity {
        let mut next = id;
        crate::template::counter("Count", 2, drop, &mut next, 0.0, 0.0)
    }

    #[test]
    fn walker_counter_dropcount_and_modifier_reset() {
        for dropcount_meaning in [DropcountMeaning::ResetOn1, DropcountMeaning::ResetOn0] {
            for drop in [0, 1] {
                let reset = drop == i32::from(dropcount_meaning == DropcountMeaning::ResetOn1);
                let mut modifier = crate::template::modifier_set_val("Reset", &mut 2, 0.0, 0.0);
                modifier.set_targets(vec![1]);
                let root = tree(vec![counter(1, drop), modifier]);
                let config = WalkerConfig {
                    dropcount_meaning,
                    ..WalkerConfig::default()
                };
                let input = WalkerInput {
                    until: 3,
                    pulses: (0..4).map(|t| (t, 1.into())).collect(),
                    ..WalkerInput::default()
                };
                let log = walk(&root, config, &input);
                assert_eq!(times(&log, 1), if reset { vec![1, 3] } else { vec![1] });
                assert_eq!(
                    log.snapshots.last().unwrap().counter_counts[&1],
                    if reset { 0 } else { 2 }
                );
                for set_val_refires in [SetValRefires::Yes, SetValRefires::No] {
                    let config = WalkerConfig {
                        set_val_refires,
                        ..config
                    };
                    let input = WalkerInput {
                        until: 7,
                        pulses: vec![
                            (0, 1.into()),
                            (1, 2.into()),
                            (2, 1.into()),
                            (3, 1.into()),
                            (4, 1.into()),
                            (5, 2.into()),
                            (6, 1.into()),
                            (7, 1.into()),
                        ],
                        ..WalkerInput::default()
                    };
                    let log = walk(&root, config, &input);
                    assert_eq!(
                        times(&log, 1),
                        if reset || set_val_refires == SetValRefires::Yes {
                            vec![3, 7]
                        } else {
                            vec![3]
                        },
                        "{config:?}, drop={drop}"
                    );
                }
            }
        }
    }

    #[test]
    fn walker_counter_same_tick_and_inactive_inputs() {
        let root = tree(vec![
            counter(1, 1),
            node("MCU_Deactivate", 2, "Off", &[1]),
            node("MCU_Activate", 3, "On", &[1]),
        ]);
        for same_tick_counts in [SameTickCounts::Each, SameTickCounts::One] {
            for same_tick_delivery in [SameTickDelivery::DepthFirst, SameTickDelivery::Batched] {
                let config = WalkerConfig {
                    same_tick_counts,
                    same_tick_delivery,
                    ..WalkerConfig::default()
                };
                let input = WalkerInput {
                    until: 1,
                    pulses: vec![(0, 1.into()), (0, 1.into()), (1, 1.into())],
                    ..WalkerInput::default()
                };
                let log = walk(&root, config, &input);
                assert_eq!(
                    times(&log, 1),
                    if same_tick_counts == SameTickCounts::Each {
                        vec![0]
                    } else {
                        vec![1]
                    }
                );
                let spaced = WalkerInput {
                    pulses: vec![(0, 1.into()), (1, 1.into())],
                    ..input
                };
                assert_eq!(times(&walk(&root, config, &spaced), 1), vec![1]);
                let inactive = WalkerInput {
                    until: 4,
                    pulses: vec![
                        (0, 2.into()),
                        (1, 1.into()),
                        (2, 3.into()),
                        (3, 1.into()),
                        (4, 1.into()),
                    ],
                    ..WalkerInput::default()
                };
                assert_eq!(times(&walk(&root, config, &inactive), 1), vec![4]);
            }
        }
    }

    #[test]
    fn walker_mission_begin_kills_arrivals_and_commands() {
        let mut plane = node("Plane", 1, "Plane", &[]);
        plane.set_property("LinkTrId", "2");
        let mut entity = node("MCU_TR_Entity", 2, "Plane entity", &[]);
        entity.set_property("Enabled", "0");
        crate::template::attach_event(&mut entity, 4, 10);
        crate::template::attach_event(&mut entity, 2, 11);
        let mut spawn = node("MCU_Spawner", 3, "Spawn", &[11]);
        spawn.set_objects(vec![2]);
        let mut delete = node("MCU_Delete", 4, "Delete", &[11]);
        delete.set_objects(vec![2]);
        let mut begin = node("MCU_TR_MissionBegin", 7, "Begin", &[8]);
        begin.set_property("Enabled", "0");
        let mut silenced = node("MCU_TR_MissionBegin", 9, "Silenced", &[]);
        silenced.set_property("Enabled", "0");
        let root = tree(vec![
            plane,
            entity,
            spawn,
            delete,
            node("MCU_Waypoint", 5, "WP", &[12]),
            node("MCU_CMD_AttackArea", 6, "Order", &[11]),
            begin,
            timer(8, "Started", "0", &[]),
            silenced,
            timer(10, "Killed", "0", &[]),
            timer(11, "Unexpected", "0", &[]),
            timer(12, "Arrived", "0", &[]),
        ]);
        let input = WalkerInput {
            until: 6,
            pulses: vec![(0, 3.into()), (1, 4.into()), (2, 5.into()), (3, 6.into())],
            kills: vec![(4, 1.into()), (5, 2.into())],
            arrivals: vec![(6, 5.into())],
            ..WalkerInput::default()
        };
        let log = walk(&root, WalkerConfig::default(), &input);
        assert_eq!(times(&log, 8), vec![0]);
        assert_eq!(times(&log, 9), vec![0]);
        assert_eq!(times(&log, 10), vec![4]);
        assert!(times(&log, 11).is_empty());
        assert_eq!(times(&log, 12), vec![6]);
        assert_eq!(times(&log, 5), vec![2, 6]);
        assert_eq!(times(&log, 3), vec![0]);
        assert_eq!(times(&log, 4), vec![1]);
        assert_eq!(times(&log, 6), vec![3]);
        assert!(!log.snapshots.last().unwrap().active[&2]);
    }

    #[test]
    fn walker_every_deactivated_out_has_a_rearm_or_is_single_use() {
        let mut root = tree(vec![
            timer(1, "Cycle OUT", "0", &[]),
            timer(2, "Sortie OUT", "0", &[]),
            timer(3, "Other", "0", &[]),
            node("MCU_Deactivate", 4, "Cancel", &[1, 2, 3]),
            node("MCU_Activate", 5, "Rearm", &[1]),
        ]);
        let check = super::walker_every_deactivated_out_has_a_rearm_or_is_single_use;
        assert!(check(&root, &[2]).is_ok());
        let errors = check(&root, &[]).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Sortie OUT (Index 2)"));
        root.find_by_name_mut("Rearm").unwrap().set_objects(vec![2]);
        assert!(check(&root, &[]).is_ok());
        root.find_by_name_mut("Rearm").unwrap().set_targets(vec![]);
        assert!(check(&root, &[]).unwrap_err()[0].contains("Cycle OUT"));
        assert!(check(&root, &[1]).is_ok());
        assert!(check(&root, &[1, 999]).unwrap_err()[0].contains("Index 999"));
        // Group nesting and Objects links must not hide a cancelled OUT.
        let mut off = node("MCU_Deactivate", 8, "Object cancel", &[]);
        off.set_objects(vec![6]);
        root.children
            .push(tree(vec![timer(6, "Nested OUT", "0", &[]), off]));
        assert!(check(&root, &[1]).unwrap_err()[0].contains("Nested OUT"));
        assert!(check(&root, &[1, 6]).is_ok());
    }
}
