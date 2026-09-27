#![allow(dead_code)] // P14: removed in step 9T
//! Mission trace instrumentation and sidecar interfaces; bodies are built in step 0T.

use std::path::Path;

use crate::ast::Il2Entity;

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
        Self::default()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceCarrier {
    Objective(ObjectiveStyle),
    Spawn,
}

impl Default for TraceCarrier {
    fn default() -> Self {
        Self::Objective(ObjectiveStyle::default())
    }
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
    pub from: i32,
    pub to: i32,
    pub unconditional: bool,
    pub delay_s: f64,
}

pub fn instrument(
    _root: &mut Il2Entity,
    _sel: &TraceSelect,
    _next_id: &mut i32,
    _carrier: TraceCarrier,
    _map: &mut TraceMap,
) {
}

pub fn breadcrumb(
    _carrier: TraceCarrier,
    _map: &mut TraceMap,
    _next_id: &mut i32,
    _source: TraceSource,
) -> Vec<Il2Entity> {
    vec![]
}

impl TraceMap {
    pub fn push(&mut self, _entry: TraceEntry) {}
}

pub fn write_trace_sidecar(_path: &Path, _map: &TraceMap) -> Result<(), String> {
    Err("write_trace_sidecar is built in step 0T".to_string())
}
