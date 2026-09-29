//! Offline mission-log replay. Flight 0 (2026-09-28) confirmed objective
//! positions and mission end; the 50 Hz fallback remains UNVERIFIED. Raw
//! fields survive unknown types, unknown keys and failed typed conversions.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::ast::Il2Entity;
use crate::trace::{TraceCarrier, TraceEntry, TraceMap, read_trace_sidecar};

pub type Position = (f64, f64, f64); // X, Y (altitude), Z.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub t_ticks: i64,
    pub atype: i32,
    /// Original value text, including quotes, in file order. T and AType are
    /// headers, not fields. Duplicate payload keys are retained (first wins).
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedLine {
    pub file: PathBuf,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct MissionLog {
    pub files: Vec<PathBuf>,
    pub records: Vec<Record>,
    pub unknown_records: usize,
    /// Unknown key occurrences, including payload keys of unknown ATypes.
    pub unknown_fields: usize,
    pub malformed_lines: Vec<MalformedLine>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectEvent {
    pub id: Option<i64>,
    pub pos: Option<Position>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectiveEvent {
    pub id: Option<i64>,
    pub pos: Option<Position>,
    pub coalition: Option<i32>,
    pub task_type: Option<i32>,
    pub result: Option<i32>,
    pub icon_type: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpawnRecord<'a> {
    pub id: Option<i64>,
    pub object_type: Option<&'a str>,
    pub name: Option<&'a str>,
    pub country: Option<i32>,
    pub parent_id: Option<i64>,
    pub pos: Option<Position>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerRecord<'a> {
    pub user_id: Option<&'a str>,
    pub nickname: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypedRecord<'a> {
    MissionStart {
        date: Option<&'a str>,
        time: Option<&'a str>,
        mission: Option<&'a str>,
    },
    Kill {
        attacker_id: Option<i64>,
        target_id: Option<i64>,
        pos: Option<Position>,
    },
    Takeoff(ObjectEvent),
    Landing(ObjectEvent),
    MissionEnd,
    MissionObjective(ObjectiveEvent),
    PlayerPlane {
        plane_id: Option<i64>,
        player_id: Option<i64>,
        name: Option<&'a str>,
        object_type: Option<&'a str>,
        country: Option<i32>,
    },
    Spawn(SpawnRecord<'a>),
    Removed(ObjectEvent),
    PlayerJoin(PlayerRecord<'a>),
    PlayerLeave(PlayerRecord<'a>),
    Unknown,
}

impl Record {
    pub fn field(&self, key: &str) -> Option<&str> {
        self.fields.iter().find(|(k, _)| k == key).map(|(_, v)| {
            v.strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .unwrap_or(v)
        })
    }

    fn integer<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.field(key)?.parse().ok()
    }

    pub fn typed(&self) -> TypedRecord<'_> {
        let pos = self.field("POS").and_then(position);
        match self.atype {
            0 => TypedRecord::MissionStart {
                date: self.field("GDate"),
                time: self.field("GTime"),
                mission: self.field("MFile"),
            },
            3 => TypedRecord::Kill {
                attacker_id: self.integer("AID"),
                target_id: self.integer("TID"),
                pos,
            },
            5 => TypedRecord::Takeoff(ObjectEvent {
                id: self.integer("PID"),
                pos,
            }),
            6 => TypedRecord::Landing(ObjectEvent {
                id: self.integer("PID"),
                pos,
            }),
            7 => TypedRecord::MissionEnd,
            8 => TypedRecord::MissionObjective(ObjectiveEvent {
                id: self.integer("OBJID"),
                pos,
                coalition: self.integer("COAL"),
                task_type: self.integer("TYPE"),
                result: self.integer("RES"),
                icon_type: self.integer("ICTYPE"),
            }),
            10 => TypedRecord::PlayerPlane {
                plane_id: self.integer("PLID"),
                player_id: self.integer("PID"),
                name: self.field("NAME"),
                object_type: self.field("TYPE"),
                country: self.integer("COUNTRY"),
            },
            12 => TypedRecord::Spawn(SpawnRecord {
                id: self.integer("ID"),
                object_type: self.field("TYPE"),
                name: self.field("NAME"),
                country: self.integer("COUNTRY"),
                parent_id: self.integer("PID"),
                pos,
            }),
            16 => TypedRecord::Removed(ObjectEvent {
                id: self.integer("BOTID").or_else(|| self.integer("ID")),
                pos,
            }),
            20 => TypedRecord::PlayerJoin(PlayerRecord {
                user_id: self.field("USERID"),
                nickname: self.field("USERNICK"),
            }),
            21 => TypedRecord::PlayerLeave(PlayerRecord {
                user_id: self.field("USERID"),
                nickname: self.field("USERNICK"),
            }),
            _ => TypedRecord::Unknown,
        }
    }
}

fn known_fields(atype: i32) -> &'static [&'static str] {
    match atype {
        0 => &["GDate", "GTime", "MFile"],
        3 => &["AID", "TID", "POS"],
        5 | 6 => &["PID", "POS"],
        8 => &["OBJID", "POS", "COAL", "TYPE", "RES", "ICTYPE"],
        10 => &["PLID", "PID", "NAME", "TYPE", "COUNTRY"],
        12 => &["ID", "TYPE", "NAME", "COUNTRY", "PID", "POS"],
        16 => &["BOTID", "ID", "POS"],
        20 | 21 => &["USERID", "USERNICK"],
        _ => &[],
    }
}

fn position(text: &str) -> Option<Position> {
    let text = text
        .trim()
        .trim_start_matches(['(', '['])
        .trim_end_matches([')', ']']);
    let values: Vec<_> = text
        .split([',', ' ', '\t'])
        .filter(|s| !s.is_empty())
        .map(str::parse::<f64>)
        .collect::<Result<_, _>>()
        .ok()?;
    match values.as_slice() {
        [x, y, z] if values.iter().all(|v| v.is_finite()) => Some((*x, *y, *z)),
        _ => None,
    }
}

/// Fields start at whitespace + identifier + ':' or '(', outside quotes and
/// parentheses/brackets. The server writes POS(...) and TARGETS() without a
/// colon. Preserve their parentheses, unquoted multiword names and file paths.
fn fields(line: &str) -> Vec<(String, String)> {
    let bytes = line.as_bytes();
    let mut starts = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
            }
            continue;
        }
        match b {
            b'"' => quoted = true,
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth != 0
            || quoted
            || (i != 0 && !bytes[i - 1].is_ascii_whitespace())
            || !(b.is_ascii_alphabetic() || b == b'_')
        {
            continue;
        }
        let mut end = i + 1;
        while bytes
            .get(end)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            end += 1;
        }
        if bytes.get(end) == Some(&b':') {
            starts.push((i, end, end + 1));
        } else if bytes.get(end) == Some(&b'(') {
            starts.push((i, end, end));
        }
    }
    starts
        .iter()
        .enumerate()
        .map(|(n, &(start, end, value))| {
            let until = starts.get(n + 1).map_or(line.len(), |&(start, _, _)| start);
            (line[start..end].into(), line[value..until].trim().into())
        })
        .collect()
}

fn append_log(log: &mut MissionLog, path: &Path, text: &str) {
    for (n, line) in text
        .strip_prefix('\u{feff}')
        .unwrap_or(text)
        .lines()
        .enumerate()
    {
        if line.trim().is_empty() {
            continue;
        }
        let payload = fields(line);
        let header = |key: &str| {
            payload
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };
        let (Some(t_ticks), Some(atype)) = (
            header("T").and_then(|s| s.parse::<i64>().ok()),
            header("AType").and_then(|s| s.parse::<i32>().ok()),
        ) else {
            log.malformed_lines.push(MalformedLine {
                file: path.to_owned(),
                line: n + 1,
                text: line.into(),
            });
            continue;
        };
        let record = Record {
            t_ticks,
            atype,
            fields: payload
                .into_iter()
                .filter(|(k, _)| k != "T" && k != "AType")
                .collect(),
        };
        log.unknown_records += usize::from(matches!(record.typed(), TypedRecord::Unknown));
        log.unknown_fields += record
            .fields
            .iter()
            .filter(|(k, _)| !known_fields(atype).contains(&k.as_str()))
            .count();
        log.records.push(record);
    }
}

fn file_number(path: &Path) -> Result<u64, String> {
    let number = path
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.strip_suffix(']'))
        .and_then(|s| s.rsplit_once('['))
        .and_then(|(_, s)| s.parse().ok());
    number.ok_or_else(|| format!("{}: expected a numeric [n] suffix", path.display()))
}

pub fn parse_log_dir(dir: &Path) -> Result<MissionLog, String> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("missionReport")
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
            && path.is_file()
        {
            files.push(path);
        }
    }
    parse_log_files(&files)
}

pub fn parse_log_files(files: &[PathBuf]) -> Result<MissionLog, String> {
    if files.is_empty() {
        return Err("no missionReport files supplied or found".into());
    }
    let mut ordered: Vec<_> = files
        .iter()
        .map(|p| file_number(p).map(|n| (n, p.clone())))
        .collect::<Result<_, _>>()?;
    ordered.sort();
    ordered.dedup();
    let mut log = MissionLog::default();
    for (_, path) in ordered {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        append_log(&mut log, &path, &text);
        log.files.push(path);
    }
    Ok(log)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickRate {
    pub rate: f64,
    pub offset: f64,
    pub samples: usize,
    pub fitted: bool,
}

impl TickRate {
    pub fn seconds(&self, ticks: i64) -> f64 {
        (ticks as f64 - self.offset) / self.rate
    }
}

fn fit_tick_rate(pairs: &[(f64, f64)]) -> TickRate {
    let fallback = TickRate {
        rate: 50.0,
        offset: 0.0,
        samples: pairs.len(),
        fitted: false,
    };
    if pairs.len() < 2 {
        return fallback;
    }
    let count = pairs.len() as f64;
    let sx = pairs.iter().map(|p| p.0).sum::<f64>() / count;
    let ty = pairs.iter().map(|p| p.1).sum::<f64>() / count;
    let variance = pairs.iter().map(|p| (p.0 - sx).powi(2)).sum::<f64>();
    if variance <= f64::EPSILON {
        return fallback;
    }
    let rate = pairs.iter().map(|p| (p.0 - sx) * (p.1 - ty)).sum::<f64>() / variance;
    let offset = ty - rate * sx;
    if !rate.is_finite() || rate <= 0.0 || !offset.is_finite() {
        return fallback;
    }
    TickRate {
        rate,
        offset,
        samples: pairs.len(),
        fitted: true,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineEvent {
    pub t_ticks: i64,
    pub seconds: f64,
    pub description: String,
    pub pos: Option<Position>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Firing {
    pub entry: TraceEntry,
    pub times_s: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeadLink {
    pub from: i32,
    pub to: i32,
    pub fired_s: f64,
    pub delay_s: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnobservedSuccessor {
    pub breadcrumb_index: i32,
    pub upstream: Vec<i32>,
    pub downstream: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnCount {
    pub name: String,
    pub mission_count: usize,
    pub spawned_count: usize,
    /// Only present when there is exactly one mission object with this name.
    pub group_path: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Replay {
    pub files: Vec<PathBuf>,
    pub first_t: Option<i64>,
    pub last_t: Option<i64>,
    pub tick_rate: TickRate,
    pub unknown_records: usize,
    pub unknown_fields: usize,
    pub malformed_lines: Vec<MalformedLine>,
    pub trace_supplied: bool,
    pub firings: Vec<Firing>,
    pub timeline: Vec<TimelineEvent>,
    pub unobserved_successors: Vec<UnobservedSuccessor>,
    pub dead_links: Vec<DeadLink>,
    pub spawn_counts: Vec<SpawnCount>,
}

fn breadcrumb_match(record: &Record, trace: &TraceMap) -> Option<usize> {
    match record.typed() {
        TypedRecord::MissionObjective(objective) => {
            if let Some((x, _, z)) = objective.pos {
                let mut matches = trace.entries.iter().enumerate().filter(|(_, e)| {
                    matches!(e.carrier, TraceCarrier::Objective(_))
                        && (e.pos.0 - x).abs() <= 1.0
                        && (e.pos.1 - z).abs() <= 1.0
                });
                if let Some((i, _)) = matches.next() {
                    return matches.next().is_none().then_some(i);
                }
            }
            // Flight 0, 2026-09-28: OBJID is not the MCU index (473 vs 305).
            // Position is the primary key for objectives; retain OBJID only
            // as a compatibility fallback when no position matches.
            objective.id.and_then(|id| {
                trace.entries.iter().position(|e| {
                    matches!(e.carrier, TraceCarrier::Objective(_))
                        && i64::from(e.breadcrumb_index) == id
                })
            })
        }
        TypedRecord::Spawn(spawn) => {
            let name = spawn.name?;
            let mut matches = trace.entries.iter().enumerate().filter(|(_, e)| {
                e.carrier == TraceCarrier::Spawn && e.spawn_name.as_deref() == Some(name)
            });
            let (i, _) = matches.next()?;
            matches.next().is_none().then_some(i)
        }
        _ => None,
    }
}

pub fn replay(log: &MissionLog, group: &Il2Entity, trace: Option<&TraceMap>) -> Replay {
    let entries = trace.map_or(&[][..], |t| t.entries.as_slice());
    let mut ticks = vec![Vec::new(); entries.len()];
    if let Some(trace) = trace {
        for record in &log.records {
            if let Some(i) = breadcrumb_match(record, trace) {
                ticks[i].push(record.t_ticks);
            }
        }
    }
    // Repeated cues do not identify which firing belongs to expected_s. Leave
    // them out of the fit rather than silently choosing one occurrence.
    let pairs: Vec<_> = entries
        .iter()
        .zip(&ticks)
        .filter_map(|(e, ticks)| {
            let [tick] = ticks.as_slice() else {
                return None;
            };
            e.expected_s
                .filter(|s| s.is_finite() && *s >= 0.0)
                .map(|s| (s, *tick as f64))
        })
        .collect();
    let tick_rate = fit_tick_rate(&pairs);
    let mut timeline = Vec::new();
    for record in &log.records {
        let breadcrumb =
            trace.and_then(|map| breadcrumb_match(record, map).map(|i| &map.entries[i]));
        if let Some((description, pos)) = describe_record(record.typed(), breadcrumb) {
            timeline.push(TimelineEvent {
                t_ticks: record.t_ticks,
                seconds: tick_rate.seconds(record.t_ticks),
                description,
                pos,
            });
        }
    }
    let mut firings = Vec::new();
    for (entry, mut ticks) in entries.iter().cloned().zip(ticks) {
        ticks.sort_unstable();
        let times_s = ticks.iter().map(|t| tick_rate.seconds(*t)).collect();
        for tick in ticks {
            timeline.push(TimelineEvent {
                t_ticks: tick,
                seconds: tick_rate.seconds(tick),
                description: format!("Breadcrumb: {}", entry_label(&entry)),
                pos: None,
            });
        }
        firings.push(Firing { entry, times_s });
    }
    timeline.sort_by_key(|e| e.t_ticks);
    let first_t = log.records.iter().map(|r| r.t_ticks).min();
    let last_t = log.records.iter().map(|r| r.t_ticks).max();
    let (unobserved_successors, dead_links) = trace.map_or_else(
        || (Vec::new(), Vec::new()),
        |trace| successors(trace, &firings, last_t.map(|t| tick_rate.seconds(t))),
    );
    Replay {
        files: log.files.clone(),
        first_t,
        last_t,
        tick_rate,
        unknown_records: log.unknown_records,
        unknown_fields: log.unknown_fields,
        malformed_lines: log.malformed_lines.clone(),
        trace_supplied: trace.is_some(),
        firings,
        timeline,
        unobserved_successors,
        dead_links,
        spawn_counts: spawn_counts(log, group),
    }
}

fn successors(
    trace: &TraceMap,
    firings: &[Firing],
    last_s: Option<f64>,
) -> (Vec<UnobservedSuccessor>, Vec<DeadLink>) {
    let observed: HashMap<_, _> = firings
        .iter()
        .filter(|f| !f.times_s.is_empty())
        .map(|f| (f.entry.breadcrumb_index, &f.times_s))
        .collect();
    let missing: HashSet<_> = firings
        .iter()
        .filter(|f| f.times_s.is_empty())
        .map(|f| f.entry.breadcrumb_index)
        .collect();
    let mut upstream: BTreeMap<i32, BTreeSet<i32>> = BTreeMap::new();
    let mut dead_links = Vec::new();
    for edge in &trace.edges {
        if !missing.contains(&edge.to) {
            continue;
        }
        let Some(times) = observed.get(&edge.from) else {
            continue;
        };
        upstream.entry(edge.to).or_default().insert(edge.from);
        if edge.unconditional
            && edge.delay_s.is_finite()
            && edge.delay_s >= 0.0
            && let Some(&fired_s) = times
                .iter()
                .find(|t| last_s.is_some_and(|last| last - **t >= edge.delay_s + 5.0))
        {
            dead_links.push(DeadLink {
                from: edge.from,
                to: edge.to,
                fired_s,
                delay_s: edge.delay_s,
            });
        }
    }
    let descendants = |start| {
        let mut seen = BTreeSet::new();
        let mut pending = vec![start];
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            pending.extend(
                trace
                    .edges
                    .iter()
                    .filter(|e| e.from == id && missing.contains(&e.to))
                    .map(|e| e.to),
            );
        }
        seen.remove(&start);
        seen
    };
    let reachable: BTreeMap<_, _> = upstream.keys().map(|&id| (id, descendants(id))).collect();
    // Keep only first missing successors. For a missing cycle, choose its
    // smallest breadcrumb ID as the deterministic representative.
    let roots: Vec<_> = reachable
        .keys()
        .copied()
        .filter(|id| {
            !reachable.iter().any(|(other, reach)| {
                other != id && reach.contains(id) && (!reachable[id].contains(other) || other < id)
            })
        })
        .collect();
    let mut claimed: BTreeSet<_> = roots.iter().copied().collect();
    let mut gaps = Vec::new();
    for id in roots {
        let downstream = reachable[&id]
            .iter()
            .copied()
            .filter(|id| claimed.insert(*id))
            .collect();
        gaps.push(UnobservedSuccessor {
            breadcrumb_index: id,
            upstream: upstream[&id].iter().copied().collect(),
            downstream,
        });
    }
    (gaps, dead_links)
}

fn spawn_counts(log: &MissionLog, group: &Il2Entity) -> Vec<SpawnCount> {
    fn walk(
        node: &Il2Entity,
        path: &mut Vec<String>,
        names: &mut BTreeMap<String, Vec<Vec<String>>>,
    ) {
        let is_group = node.block_type == "Group";
        if is_group {
            path.push(node.name().unwrap_or("Group").into());
        }
        if matches!(
            node.block_type.as_str(),
            "Plane" | "Vehicle" | "Ship" | "Train"
        ) && let Some(name) = node.name()
        {
            names.entry(name.into()).or_default().push(path.clone());
        }
        for child in &node.children {
            walk(child, path, names);
        }
        if is_group {
            path.pop();
        }
    }
    let mut names = BTreeMap::new();
    walk(group, &mut Vec::new(), &mut names);
    let mut spawned = HashMap::<String, usize>::new();
    for record in &log.records {
        if let TypedRecord::Spawn(s) = record.typed()
            && let Some(name) = s.name
        {
            *spawned.entry(name.into()).or_default() += 1;
        }
    }
    names
        .into_iter()
        .map(|(name, paths)| SpawnCount {
            mission_count: paths.len(),
            spawned_count: *spawned.get(&name).unwrap_or(&0),
            group_path: (paths.len() == 1).then(|| paths[0].clone()),
            name,
        })
        .collect()
}

fn entry_label(entry: &TraceEntry) -> String {
    let event = entry
        .event_type
        .map_or_else(String::new, |e| format!(", event {e}"));
    format!(
        "{} / {} ({} #{}{event})",
        entry.group_path.join(" / "),
        entry.source_name,
        entry.source_type,
        entry.source_index
    )
}

fn describe_record(
    record: TypedRecord<'_>,
    breadcrumb: Option<&TraceEntry>,
) -> Option<(String, Option<Position>)> {
    let id = |n: Option<i64>| n.map_or_else(|| "?".into(), |n| n.to_string());
    let (description, pos) = match record {
        TypedRecord::MissionStart {
            date,
            time,
            mission,
        } => (
            format!(
                "Mission start: {} {} {}",
                date.unwrap_or("?"),
                time.unwrap_or("?"),
                mission.unwrap_or("?")
            ),
            None,
        ),
        TypedRecord::Kill {
            attacker_id,
            target_id,
            pos,
        } => (
            format!("Kill: {} -> {}", id(attacker_id), id(target_id)),
            pos,
        ),
        TypedRecord::Takeoff(e) => (format!("Takeoff: {}", id(e.id)), e.pos),
        TypedRecord::Landing(e) => (format!("Landing: {}", id(e.id)), e.pos),
        TypedRecord::MissionEnd => ("Mission end".into(), None),
        TypedRecord::Removed(e) => (format!("Removed: {}", id(e.id)), e.pos),
        TypedRecord::MissionObjective(e) => (
            format!(
                "Objective: {} (coalition {:?}, type {:?}, result {:?}, icon {:?})",
                breadcrumb.map_or_else(
                    || id(e.id),
                    |entry| format!("{} (OBJID {})", entry.source_name, id(e.id))
                ),
                e.coalition,
                e.task_type,
                e.result,
                e.icon_type
            ),
            e.pos,
        ),
        TypedRecord::Spawn(e) => (
            format!(
                "Spawn: {} / {} (id {}, country {:?}, parent {})",
                e.name.unwrap_or("?"),
                e.object_type.unwrap_or("?"),
                id(e.id),
                e.country,
                id(e.parent_id)
            ),
            e.pos,
        ),
        TypedRecord::PlayerPlane {
            plane_id,
            player_id,
            name,
            object_type,
            country,
        } => (
            format!(
                "Player plane: {} / {} (plane {}, player {}, country {:?})",
                name.unwrap_or("?"),
                object_type.unwrap_or("?"),
                id(plane_id),
                id(player_id),
                country
            ),
            None,
        ),
        TypedRecord::PlayerJoin(p) => (
            format!(
                "Player join: {} ({})",
                p.nickname.unwrap_or("?"),
                p.user_id.unwrap_or("?")
            ),
            None,
        ),
        TypedRecord::PlayerLeave(p) => (
            format!(
                "Player leave: {} ({})",
                p.nickname.unwrap_or("?"),
                p.user_id.unwrap_or("?")
            ),
            None,
        ),
        TypedRecord::Unknown => return None,
    };
    Some((description, pos))
}

fn markdown(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "&#124;")
        .replace('`', "&#96;")
        .replace('*', "&#42;")
        .replace('_', "&#95;")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
        .replace(['\n', '\r'], " ")
}

pub fn report_markdown(replay: &Replay) -> String {
    let mut out = String::from("# Mission replay\n\n");
    writeln!(
        out,
        "Log files: {}",
        replay
            .files
            .iter()
            .map(|p| markdown(&p.display().to_string()))
            .collect::<Vec<_>>()
            .join(", ")
    )
    .unwrap();
    let tick = |t: Option<i64>| t.map_or_else(|| "none".into(), |t| t.to_string());
    writeln!(
        out,
        "\nFirst T: {}; last T: {}.",
        tick(replay.first_t),
        tick(replay.last_t)
    )
    .unwrap();
    writeln!(
        out,
        "\nTick rate: {:.6} ticks/s; offset: {:.6} ticks; {} ({} cue samples).",
        replay.tick_rate.rate,
        replay.tick_rate.offset,
        if replay.tick_rate.fitted {
            "fitted T = offset + rate * seconds"
        } else {
            "UNVERIFIED fallback"
        },
        replay.tick_rate.samples
    )
    .unwrap();
    writeln!(
        out,
        "\nUnknown records: {}; unknown fields: {}; malformed lines retained: {}.",
        replay.unknown_records,
        replay.unknown_fields,
        replay.malformed_lines.len()
    )
    .unwrap();
    out.push_str("\nFlight 0 (2026-09-28): the objective matched by position, not MCU index, and ended the dogfight round. UNVERIFIED: carrier logging on every firing, tick rate, same-tick multiplicity and log completeness. Observed counts are at least the logged firings; video remains a cross-check.\n");
    if !replay.trace_supplied {
        out.push_str("\nNo trace sidecar: firing and graph diagnostics are unavailable.\n");
    }
    for line in &replay.malformed_lines {
        writeln!(
            out,
            "\nMalformed {}:{}: {}",
            markdown(&line.file.display().to_string()),
            line.line,
            markdown(&line.text)
        )
        .unwrap();
    }
    let label = |id| {
        replay
            .firings
            .iter()
            .find(|f| f.entry.breadcrumb_index == id)
            .map_or_else(
                || format!("breadcrumb #{id}"),
                |f| markdown(&entry_label(&f.entry)),
            )
    };
    out.push_str("\n## Dead links\n\n");
    if replay.dead_links.is_empty() {
        out.push_str("None.\n");
    }
    for dead in &replay.dead_links {
        writeln!(out, "- {} -> {}: fault on an unconditional path (fired at {:.3} s, delay {:.3} s, at least 5 s grace elapsed).", label(dead.from), label(dead.to), dead.fired_s, dead.delay_s).unwrap();
    }
    out.push_str("\n## Unobserved successors\n\nThese are observations, not faults: a closed gate, failed random roll or counter below its threshold may explain them. Downstream missing nodes are grouped under the first missing successor.\n\n");
    if replay.unobserved_successors.is_empty() {
        out.push_str("None.\n");
    }
    let mut grouped = HashSet::new();
    for gap in &replay.unobserved_successors {
        grouped.insert(gap.breadcrumb_index);
        grouped.extend(&gap.downstream);
        writeln!(
            out,
            "- {} (upstream fired: {}).{}",
            label(gap.breadcrumb_index),
            gap.upstream
                .iter()
                .map(|&id| label(id))
                .collect::<Vec<_>>()
                .join("; "),
            if gap.downstream.is_empty() {
                String::new()
            } else {
                format!(
                    " Downstream: {}.",
                    gap.downstream
                        .iter()
                        .map(|&id| label(id))
                        .collect::<Vec<_>>()
                        .join("; ")
                )
            }
        )
        .unwrap();
    }
    out.push_str("\n## Never fired\n\n");
    let never: Vec<_> = replay
        .firings
        .iter()
        .filter(|f| f.times_s.is_empty())
        .collect();
    writeln!(
        out,
        "{} traced sources/events never observed; {} already grouped above.\n",
        never.len(),
        grouped.len()
    )
    .unwrap();
    for firing in never
        .iter()
        .filter(|f| !grouped.contains(&f.entry.breadcrumb_index))
    {
        writeln!(out, "- {}", label(firing.entry.breadcrumb_index)).unwrap();
    }
    out.push_str("\n## Never spawned\n\nCounts are per mission Name (UNVERIFIED mapping from log NAME). Repeated spawn records count again; shared names cannot identify which copy spawned. Group paths are shown only for unique names.\n\n");
    let shortfalls: Vec<_> = replay
        .spawn_counts
        .iter()
        .filter(|s| s.spawned_count < s.mission_count)
        .collect();
    if shortfalls.is_empty() {
        out.push_str("None.\n");
    }
    for count in shortfalls {
        writeln!(
            out,
            "- {}: {} spawned, {} in the mission{}.",
            markdown(&count.name),
            count.spawned_count,
            count.mission_count,
            count
                .group_path
                .as_ref()
                .map_or_else(String::new, |p| format!(
                    " (group: {})",
                    markdown(&p.join(" / "))
                ))
        )
        .unwrap();
    }
    out.push_str("\n## Timeline\n\n");
    for f in replay.firings.iter().filter(|f| !f.times_s.is_empty()) {
        writeln!(
            out,
            "- {}: {} observed firing(s), at {} s.",
            label(f.entry.breadcrumb_index),
            f.times_s.len(),
            f.times_s
                .iter()
                .map(|s| format!("{s:.3}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
    }
    out.push_str("\n| Seconds | T | Event | Position X,Y,Z |\n| ---: | ---: | --- | --- |\n");
    for e in &replay.timeline {
        writeln!(
            out,
            "| {:.3} | {} | {} | {} |",
            e.seconds,
            e.t_ticks,
            markdown(&e.description),
            e.pos
                .map_or_else(String::new, |(x, y, z)| format!("{x:.3}, {y:.3}, {z:.3}"))
        )
        .unwrap();
    }
    out
}

pub const CLI_HELP: &str = "IL2MissionUtility.exe replay --group <file.Group> [--trace <file.trace.json>] --logs <dir | files...> [--out <report.md>]";

pub fn run_cli(args: &[String]) -> Option<i32> {
    if args.first()?.as_str() != "replay" {
        return None;
    }
    Some(match cli_replay(&args[1..]) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}\n\n{CLI_HELP}");
            2
        }
    })
}

fn cli_replay(args: &[String]) -> Result<(), String> {
    let mut group = None;
    let mut trace = None;
    let mut out = None;
    let mut logs = None;
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        i += 1;
        match flag {
            "--group" | "--trace" | "--out" => {
                let dest = match flag {
                    "--group" => &mut group,
                    "--trace" => &mut trace,
                    _ => &mut out,
                };
                if dest.is_some() {
                    return Err(format!("duplicate {flag}"));
                }
                let value = args
                    .get(i)
                    .filter(|s| !s.starts_with("--"))
                    .ok_or_else(|| format!("{flag} needs a path"))?;
                *dest = Some(PathBuf::from(value));
                i += 1;
            }
            "--logs" => {
                if logs.is_some() {
                    return Err("duplicate --logs".into());
                }
                let start = i;
                while i < args.len() && !args[i].starts_with("--") {
                    i += 1;
                }
                if start == i {
                    return Err("--logs needs a directory or files".into());
                }
                logs = Some(args[start..i].iter().map(PathBuf::from).collect::<Vec<_>>());
            }
            _ => return Err(format!("unknown argument: {flag}")),
        }
    }
    let group = group.ok_or("--group is required")?;
    let logs = logs.ok_or("--logs is required")?;
    let text = std::fs::read_to_string(&group).map_err(|e| format!("{}: {e}", group.display()))?;
    let group =
        crate::parser::parse_group_file(&text).map_err(|e| format!("{}: {e}", group.display()))?;
    let trace = trace.as_deref().map(read_trace_sidecar).transpose()?;
    let log = if logs.len() == 1 && logs[0].is_dir() {
        parse_log_dir(&logs[0])?
    } else {
        if logs.iter().any(|p| p.is_dir()) {
            return Err("--logs accepts one directory or a list of files".into());
        }
        parse_log_files(&logs)?
    };
    let report = report_markdown(&replay(&log, &group, trace.as_ref()));
    if let Some(out) = out {
        std::fs::write(&out, report).map_err(|e| format!("{}: {e}", out.display()))?;
    } else {
        print!("{report}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::{ObjectiveStyle, TraceSelect, instrument, write_trace_sidecar};

    fn fixture(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/testdata/missionlog")
            .join(path)
    }

    fn group() -> Il2Entity {
        crate::parser::parse_group_file(&std::fs::read_to_string(fixture("replay.Group")).unwrap())
            .unwrap()
    }

    fn log(text: &str) -> MissionLog {
        let mut log = MissionLog::default();
        append_log(&mut log, Path::new("synthetic[0].txt"), text);
        log
    }

    fn trace_group(group: &mut Il2Entity) -> TraceMap {
        let mut map = TraceMap::default();
        instrument(
            group,
            &TraceSelect {
                indexes: vec![1, 3, 4],
                ..TraceSelect::default()
            },
            &mut 100,
            TraceCarrier::Objective(ObjectiveStyle::default()),
            &mut map,
        );
        map
    }

    struct TempFile(PathBuf);
    impl TempFile {
        fn new(suffix: &str) -> Self {
            static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            Self(std::env::temp_dir().join(format!(
                "il2_replay_{}_{}{suffix}",
                std::process::id(),
                SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            )))
        }
    }
    impl Drop for TempFile {
        fn drop(&mut self) {
            if self.0.is_file() {
                std::fs::remove_file(&self.0).unwrap();
            }
        }
    }

    #[test]
    fn missionlog_parses_synthetic_fixture() {
        let logs = parse_log_dir(&fixture("synthetic")).unwrap();
        assert_eq!(logs.files.len(), 2);
        assert_eq!(file_number(&logs.files[0]).unwrap(), 0);
        assert_eq!(file_number(&logs.files[1]).unwrap(), 1);
        assert_eq!(logs.records.len(), 11);
        assert!(
            logs.records
                .windows(2)
                .all(|r| r[0].t_ticks <= r[1].t_ticks)
        );
        let mut counts = BTreeMap::new();
        for r in &logs.records {
            *counts.entry(r.atype).or_insert(0) += 1;
        }
        assert_eq!(
            counts,
            [0, 3, 5, 6, 8, 10, 12, 16, 20, 21, 999]
                .into_iter()
                .map(|t| (t, 1))
                .collect()
        );
        assert_eq!((logs.unknown_records, logs.unknown_fields), (1, 1));
        assert!(logs.malformed_lines.is_empty());
        assert!(matches!(
            logs.records.last().unwrap().typed(),
            TypedRecord::Unknown
        ));
        let get = |kind| logs.records.iter().find(|r| r.atype == kind).unwrap();
        assert_eq!(
            get(0).typed(),
            TypedRecord::MissionStart {
                date: Some("1950.06.25"),
                time: Some("12:00:00"),
                mission: Some("missions\\Synthetic replay.Mission")
            }
        );
        assert_eq!(
            get(3).typed(),
            TypedRecord::Kill {
                attacker_id: Some(102),
                target_id: Some(101),
                pos: Some((100020.0, 0.0, 200020.0))
            }
        );
        assert_eq!(
            get(5).typed(),
            TypedRecord::Takeoff(ObjectEvent {
                id: Some(101),
                pos: Some((100010.0, 3100.0, 200010.0))
            })
        );
        assert_eq!(
            get(6).typed(),
            TypedRecord::Landing(ObjectEvent {
                id: Some(101),
                pos: Some((100020.0, 0.0, 200020.0))
            })
        );
        assert_eq!(
            get(8).typed(),
            TypedRecord::MissionObjective(ObjectiveEvent {
                id: Some(100),
                pos: Some((5000.0, 0.0, 5000.0)),
                coalition: Some(0),
                task_type: Some(0),
                result: Some(1),
                icon_type: Some(0)
            })
        );
        assert_eq!(
            get(10).typed(),
            TypedRecord::PlayerPlane {
                plane_id: Some(101),
                player_id: Some(7),
                name: Some("Test Pilot"),
                object_type: Some("f80c10"),
                country: Some(601)
            }
        );
        assert_eq!(
            get(12).typed(),
            TypedRecord::Spawn(SpawnRecord {
                id: Some(101),
                object_type: Some("f80c10"),
                name: Some("F-80C"),
                country: Some(601),
                parent_id: Some(0),
                pos: Some((100000.5, 3000.0, 200000.25))
            })
        );
        assert_eq!(
            get(12).fields.last().unwrap(),
            &("FUTURE".into(), "kept raw".into())
        );
        assert_eq!(
            get(16).typed(),
            TypedRecord::Removed(ObjectEvent {
                id: Some(101),
                pos: Some((100020.0, 0.0, 200020.0))
            })
        );
        let player = PlayerRecord {
            user_id: Some("user-1"),
            nickname: Some("Test Pilot"),
        };
        assert_eq!(get(20).typed(), TypedRecord::PlayerJoin(player));
        assert_eq!(get(21).typed(), TypedRecord::PlayerLeave(player));
        let reverse = parse_log_files(&[logs.files[1].clone(), logs.files[0].clone()]).unwrap();
        assert_eq!(reverse.records, logs.records);

        // Numeric ordering really is numeric, including suffixes above 9.
        let two = TempFile::new("[2].txt");
        let ten = TempFile::new("[10].txt");
        std::fs::write(&two.0, "T:2 AType:999 FUTURE:two words\n").unwrap();
        std::fs::write(&ten.0, "T:10 AType:999\n").unwrap();
        let sorted = parse_log_files(&[ten.0.clone(), two.0.clone()]).unwrap();
        assert_eq!(
            sorted.records.iter().map(|r| r.t_ticks).collect::<Vec<_>>(),
            [2, 10]
        );
        assert_eq!((sorted.unknown_records, sorted.unknown_fields), (2, 1));
        assert_eq!(sorted.records[0].field("FUTURE"), Some("two words"));

        let tolerant = log(
            "\u{feff}T:1 AType:12 ID:broken NAME:\"한글 AType:8\" POS:(bad,0,1) NEW:kept\ntruncated line\nT:3 AType:5 PID:nope\n",
        );
        assert_eq!(tolerant.records.len(), 2);
        assert_eq!(tolerant.malformed_lines[0].text, "truncated line");
        assert_eq!(tolerant.malformed_lines[0].line, 2);
        assert_eq!(tolerant.records[0].field("NAME"), Some("한글 AType:8"));
        assert!(matches!(
            tolerant.records[0].typed(),
            TypedRecord::Spawn(SpawnRecord {
                id: None,
                pos: None,
                ..
            })
        ));
        assert_eq!(tolerant.unknown_fields, 1);
        let report = report_markdown(&replay(&logs, &group(), None));
        assert!(report.contains("Unknown records: 1; unknown fields: 1"));
        let report = report_markdown(&replay(&tolerant, &group(), None));
        assert!(report.contains("malformed lines retained: 1"));
        assert!(parse_log_files(&[]).is_err());
    }

    #[test]
    fn missionlog_real_fixture_parses() {
        let logs = parse_log_dir(&fixture("real_1")).unwrap();
        assert!(!logs.records.is_empty());
        assert!(logs.malformed_lines.is_empty());
        let trace = read_trace_sidecar(&fixture("real_1/P14_Probe_0_traced.trace.json")).unwrap();
        let objectives: Vec<_> = logs.records.iter().filter(|r| r.atype == 8).collect();
        assert_eq!(objectives.len(), 1);
        let objective = objectives[0];
        assert_eq!(objective.t_ticks, 3);
        assert_eq!(
            objective.typed(),
            TypedRecord::MissionObjective(ObjectiveEvent {
                id: Some(473),
                pos: Some((5430.0, 0.0, 5000.0)),
                coalition: Some(0),
                task_type: Some(0),
                result: Some(1),
                icon_type: Some(0),
            })
        );
        assert_eq!(objective.field("TARGETS"), Some("()"));
        assert!(trace.entries.iter().all(|e| e.breadcrumb_index != 473));
        let entry = &trace.entries[breadcrumb_match(objective, &trace).unwrap()];
        assert_eq!(entry.breadcrumb_index, 305);
        assert_eq!(entry.source_name, "CUE 01");

        // Attribution uses the sidecar of the file that actually flew, without
        // depending on a regenerated probe or on running the ignored writers.
        let result = replay(&logs, &Il2Entity::new("Group"), Some(&trace));
        let fired: Vec<_> = result
            .firings
            .iter()
            .filter(|f| !f.times_s.is_empty())
            .collect();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].entry, *entry);
        assert_eq!(fired[0].times_s, [0.06]);
        assert!(result.timeline.iter().any(|event| {
            event.t_ticks == 3
                && event
                    .description
                    .starts_with("Objective: CUE 01 (OBJID 473)")
                && event.pos == Some((5430.0, 0.0, 5000.0))
        }));
        assert!(result.timeline.iter().any(|event| {
            event.t_ticks == 3
                && event
                    .description
                    .starts_with("Breadcrumb: P14_Probe_0 / CUE 01")
        }));
        let ends: Vec<_> = logs.records.iter().filter(|r| r.atype == 7).collect();
        assert_eq!(ends.len(), 1);
        assert_eq!(ends[0].typed(), TypedRecord::MissionEnd);
        assert!(
            result
                .timeline
                .iter()
                .any(|event| { event.t_ticks == 405 && event.description == "Mission end" })
        );
        assert_eq!(logs.unknown_records, 8);
        let report = report_markdown(&result);
        let header = report.split("\n## ").next().unwrap();
        assert!(header.contains(&format!("Unknown records: {}", logs.unknown_records)));
    }

    #[test]
    fn replay_unobserved_successor_behind_a_gate_is_not_a_dead_link() {
        let mut group = group();
        let mut off = crate::template::mcu("MCU_Deactivate", "CLOSE GATE", &mut 70, 0.0, 0.0);
        off.set_targets(vec![2]);
        group.children.push(off);
        let map = trace_group(&mut group);
        let logs = parse_log_dir(&fixture("chain")).unwrap();
        let result = replay(&logs, &group, Some(&map));
        assert!(result.dead_links.is_empty());
        assert_eq!(result.unobserved_successors.len(), 1);
        assert_eq!(
            result.unobserved_successors[0],
            UnobservedSuccessor {
                breadcrumb_index: 101,
                upstream: vec![100],
                downstream: vec![102]
            }
        );
        assert_eq!(result.firings[0].times_s, [10.0]);
        assert!(result.firings[1].times_s.is_empty());
        let report = report_markdown(&result);
        assert!(report.contains("## Dead links\n\nNone."));
        assert!(report.contains("observations, not faults"));
        assert!(report.contains("Downstream:"));
    }

    #[test]
    fn replay_dead_link_on_an_unconditional_path() {
        let mut group = group();
        let map = trace_group(&mut group);
        let logs = parse_log_dir(&fixture("chain")).unwrap();
        let result = replay(&logs, &group, Some(&map));
        assert_eq!(
            result.dead_links,
            [DeadLink {
                from: 100,
                to: 101,
                fired_s: 10.0,
                delay_s: 3.0
            }]
        );
        assert_eq!(result.unobserved_successors.len(), 1);
        assert_eq!(result.unobserved_successors[0].downstream, [102]);
        let report = report_markdown(&result);
        let never = report
            .split("## Never fired")
            .nth(1)
            .unwrap()
            .split("## Never spawned")
            .next()
            .unwrap();
        assert!(never.contains("2 traced sources/events never observed; 2 already grouped above"));
        assert!(
            !never.contains("MCU_Timer"),
            "descendants must not be listed twice"
        );
        for (end_tick, is_dead) in [(899, false), (900, true)] {
            let short = log(&format!(
                "T:500 AType:8 OBJID:100\nT:{end_tick} AType:999\n"
            ));
            assert_eq!(
                !replay(&short, &group, Some(&map)).dead_links.is_empty(),
                is_dead
            );
        }
        let fired = log(
            "T:500 AType:8 OBJID:100\nT:700 AType:8 OBJID:101\nT:750 AType:8 OBJID:102\nT:1500 AType:999\n",
        );
        let result = replay(&fired, &group, Some(&map));
        assert!(result.unobserved_successors.is_empty() && result.dead_links.is_empty());
        // Loops and converging missing branches terminate and group each node once.
        let mut cyclic = map.clone();
        cyclic.edges.push(crate::trace::TraceEdge {
            from: 102,
            to: 101,
            unconditional: false,
            delay_s: 0.0,
        });
        cyclic.edges.push(crate::trace::TraceEdge {
            from: 100,
            to: 102,
            unconditional: false,
            delay_s: 0.0,
        });
        let result = replay(&logs, &group, Some(&cyclic));
        assert_eq!(result.unobserved_successors.len(), 1);
        assert_eq!(result.unobserved_successors[0].downstream, [102]);
    }

    #[test]
    fn replay_never_spawned_counts_per_group() {
        let mut text = String::new();
        for id in 0..6 {
            writeln!(text, "T:{id} AType:12 ID:{id} NAME:F-80C TYPE:f80c10").unwrap();
        }
        text.push_str("T:10 AType:12 ID:100 NAME:Only ship TYPE:ship\n");
        let result = replay(&log(&text), &group(), None);
        let get = |name| result.spawn_counts.iter().find(|c| c.name == name).unwrap();
        assert_eq!(get("F-80C").mission_count, 8);
        assert_eq!(get("F-80C").spawned_count, 6);
        assert_eq!(get("F-80C").group_path, None);
        assert_eq!(get("Only truck").spawned_count, 0);
        assert_eq!(
            get("Only truck").group_path,
            Some(vec!["Replay synthetic".into(), "Copy 1".into()])
        );
        assert_eq!(
            get("Only train").group_path,
            Some(vec!["Replay synthetic".into(), "Copy 2".into()])
        );
        assert_eq!(get("Only ship").spawned_count, 1);
        assert_eq!(
            result.spawn_counts.len(),
            4,
            "count objects, not entity translators"
        );
        let report = report_markdown(&result);
        assert!(report.contains("F-80C: 6 spawned, 8 in the mission."));
        assert!(report.contains(
            "Only truck: 0 spawned, 1 in the mission (group: Replay synthetic / Copy 1)"
        ));
        assert!(!report.contains("Only ship: 1 spawned"));
    }

    #[test]
    fn replay_tick_rate_fit() {
        let mut group = group();
        let mut map = trace_group(&mut group);
        for (entry, time) in map.entries.iter_mut().zip([20.0, 30.0, 40.0]) {
            entry.expected_s = Some(time);
        }
        let spawn_source = crate::trace::TraceSource {
            index: 90,
            name: "Spawn cue".into(),
            block_type: "MCU_Timer".into(),
            event_type: None,
            group_path: vec!["Cues".into()],
            expected_s: Some(50.0),
        };
        group.children.extend(crate::trace::breadcrumb(
            TraceCarrier::Spawn,
            &mut map,
            &mut 200,
            spawn_source,
        ));
        let logs = log(
            "T:100 AType:0\nT:1100 AType:8 OBJID:100 POS:(0,0,0)\nT:1600 AType:8 OBJID:999 POS:(5010.8,7,4999.1)\nT:2100 AType:8 POS:(5020,0,5000)\nT:2600 AType:12 NAME:TRACE 3 POS:(1,0,1)\nT:2600 AType:3 AID:2 TID:3\n",
        );
        let result = replay(&logs, &group, Some(&map));
        assert!(result.tick_rate.fitted);
        assert_eq!(result.tick_rate.samples, 4);
        assert!((result.tick_rate.rate - 50.0).abs() <= 0.5);
        assert!((result.tick_rate.offset - 100.0).abs() < 1e-6);
        assert_eq!(
            result
                .firings
                .iter()
                .map(|f| f.times_s.clone())
                .collect::<Vec<_>>(),
            [vec![20.0], vec![30.0], vec![40.0], vec![50.0]]
        );
        assert!(
            result
                .timeline
                .windows(2)
                .all(|w| w[0].t_ticks <= w[1].t_ticks)
        );
        assert_eq!(
            result.timeline.iter().filter(|e| e.t_ticks == 2600).count(),
            3,
            "spawn, breadcrumb and kill in same tick"
        );
        assert!(report_markdown(&result).contains("offset: 100.000000 ticks; fitted"));

        // Same-tick repeated firings are retained; repeated expected cues are
        // excluded from calibration. The other three cues still fit exactly.
        let mut repeated = logs.clone();
        repeated.records.push(logs.records[1].clone());
        let result = replay(&repeated, &group, Some(&map));
        assert_eq!(result.firings[0].times_s, [20.0, 20.0]);
        assert_eq!(result.tick_rate.samples, 3);
        assert_eq!(result.tick_rate.rate, 50.0);
        assert!(!fit_tick_rate(&[]).fitted);
        assert!(!fit_tick_rate(&[(1.0, 50.0), (1.0, 100.0)]).fitted);
        assert!(!fit_tick_rate(&[(1.0, 100.0), (2.0, 50.0)]).fitted);
        // A runtime OBJID may collide with another MCU's index. Position wins,
        // including both 1 m boundaries, regardless of altitude or colon syntax.
        for pos in ["POS(5011,9000,4999)", "POS:(5009,-9000,5001)"] {
            let boundary = log(&format!("T:1 AType:8 OBJID:100 {pos}"));
            assert_eq!(breadcrumb_match(&boundary.records[0], &map), Some(1));
        }
        let outside = log(
            "T:20 AType:8 OBJID:999 POS:(5001.1,0,5000)\nT:20 AType:8 OBJID:999 POS(5000,0,5001.1)\nT:21 AType:12 NAME:Not TRACE 3\n",
        );
        assert!(
            replay(&outside, &group, Some(&map))
                .firings
                .iter()
                .all(|f| f.times_s.is_empty())
        );
        let mut ambiguous = map.clone();
        ambiguous.entries[1].pos = ambiguous.entries[0].pos;
        assert_eq!(
            breadcrumb_match(
                &log("T:1 AType:8 OBJID:101 POS(5000,0,5000)").records[0],
                &ambiguous
            ),
            None
        );

        // Real spawn syntax: all carriers share one position and only NAME
        // identifies TRACE 7. Runtime object IDs are independent of MCU indexes.
        let mut spawn_map = TraceMap::default();
        let mut spawn_group = Il2Entity::new("Group");
        let mut next = 200;
        for n in 0..8 {
            spawn_group.children.extend(crate::trace::breadcrumb(
                TraceCarrier::Spawn,
                &mut spawn_map,
                &mut next,
                crate::trace::TraceSource {
                    index: 90 + n,
                    name: format!("Spawn source {n}"),
                    block_type: "MCU_Timer".into(),
                    event_type: None,
                    group_path: vec!["Spawn probes".into()],
                    expected_s: None,
                },
            ));
        }
        let spawns = log(
            "T:50 AType:12 ID:136192 TYPE:WillysMB COUNTRY:0 NAME:TRACE 7 PID:-1 POS(400000.0000,42.0000,50000.0000)\n",
        );
        assert_eq!(
            spawns.records[0].typed(),
            TypedRecord::Spawn(SpawnRecord {
                id: Some(136192),
                object_type: Some("WillysMB"),
                name: Some("TRACE 7"),
                country: Some(0),
                parent_id: Some(-1),
                pos: Some((400000.0, 42.0, 50000.0)),
            })
        );
        let result = replay(&spawns, &spawn_group, Some(&spawn_map));
        let fired: Vec<_> = result
            .firings
            .iter()
            .filter(|f| !f.times_s.is_empty())
            .collect();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].entry, spawn_map.entries[7]);
        assert_eq!(fired[0].entry.spawn_name.as_deref(), Some("TRACE 7"));
        assert_eq!(fired[0].times_s, [1.0]);
        assert!(result.timeline.iter().any(|event| {
            event.t_ticks == 50
                && event
                    .description
                    .starts_with("Breadcrumb: Spawn probes / Spawn source 7")
        }));
    }

    #[test]
    fn replay_cli_bad_args_exit_2() {
        assert_eq!(run_cli(&[]), None);
        assert_eq!(run_cli(&["--probe-status".into()]), None);
        for args in [
            vec!["replay"],
            vec!["replay", "--group"],
            vec!["replay", "--logs"],
            vec!["replay", "--unknown"],
            vec!["replay", "--group", "x", "--group", "y"],
            vec!["replay", "--logs", "x", "--logs", "y"],
            vec!["replay", "--out", "x", "--out", "y"],
            vec!["replay", "--trace", "x", "--trace", "y"],
            vec!["replay", "--group", "--logs", "x"],
        ] {
            assert_eq!(
                run_cli(&args.into_iter().map(str::to_string).collect::<Vec<_>>()),
                Some(2)
            );
        }
        let missing = TempFile::new("[0].txt");
        let mut args = vec![
            "replay".into(),
            "--group".into(),
            fixture("replay.Group").display().to_string(),
            "--logs".into(),
            missing.0.display().to_string(),
        ];
        assert_eq!(run_cli(&args), Some(2));
        args[4] = fixture("synthetic").display().to_string();
        args.extend([
            "--trace".into(),
            fixture("replay.Group").display().to_string(),
        ]);
        assert_eq!(run_cli(&args), Some(2), "bad sidecar");
        args.truncate(5);
        args.extend(["--out".into(), std::env::temp_dir().display().to_string()]);
        assert_eq!(run_cli(&args), Some(2), "output is a directory");
        args.truncate(5);
        args[2] = missing.0.display().to_string();
        assert_eq!(run_cli(&args), Some(2), "missing group");
    }

    #[test]
    fn replay_cli_writes_report() {
        let out = TempFile::new(".md");
        let out_path = out.0.clone();
        let mut args = vec![
            "replay".into(),
            "--group".into(),
            fixture("replay.Group").display().to_string(),
            "--logs".into(),
            fixture("synthetic").display().to_string(),
            "--out".into(),
            out.0.display().to_string(),
        ];
        assert_eq!(run_cli(&args), Some(0));
        let report = std::fs::read_to_string(&out.0).unwrap();
        assert!(report.starts_with("# Mission replay"));
        assert!(report.contains("Unknown records: 1; unknown fields: 1"));
        assert!(report.contains("First T: 100; last T: 1000."));
        assert!(report.contains("No trace sidecar"));
        let headings: Vec<_> = report
            .lines()
            .filter(|line| line.starts_with("## "))
            .collect();
        assert_eq!(
            headings,
            [
                "## Dead links",
                "## Unobserved successors",
                "## Never fired",
                "## Never spawned",
                "## Timeline"
            ]
        );

        let trace = TempFile::new(".trace.json");
        let mut g = group();
        let map = trace_group(&mut g);
        write_trace_sidecar(&trace.0, &map).unwrap();
        args[4] = fixture("synthetic/missionReport(synthetic)[1].txt")
            .display()
            .to_string();
        args.insert(
            5,
            fixture("synthetic/missionReport(synthetic)[0].txt")
                .display()
                .to_string(),
        );
        args.extend(["--trace".into(), trace.0.display().to_string()]);
        assert_eq!(run_cli(&args), Some(0));
        let report = std::fs::read_to_string(&out.0).unwrap();
        assert!(report.contains("Breadcrumb: Replay synthetic / A"));
        assert!(report.contains("1 observed firing(s)"));
        assert!(!report.contains("No trace sidecar"));
        drop(out);
        assert!(!out_path.exists(), "CLI report cleaned up");
    }
}
