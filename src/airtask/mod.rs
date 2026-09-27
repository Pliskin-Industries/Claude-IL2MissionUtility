#![allow(dead_code)] // P14: removed in step 9
//! Air Tasking (P14): historical air sorties against Map objectives. See handoff/P14-air-tasking.md.
use crate::frontlines::MapAirPack;
use crate::mapclip::WorldAabb;
use std::collections::BTreeMap;
use std::path::PathBuf;

mod assemble;
mod budget;
mod clock;
mod library;
mod place;
#[cfg(test)]
mod probe;
mod shell;
#[cfg(test)]
mod testkit;
mod threshold;
mod timing;
mod zone;

// What `ui.rs` may call besides the functions at the bottom. The submodules stay private.
// library.rs / timing.rs get these signatures with stub bodies in the 1a skeleton (§5 table).
#[allow(unused_imports)] // P14: removed in step 9
pub use library::{builtin_library, load_user_sortie}; // fn builtin_library() -> Vec<AirSortie>; fn load_user_sortie(path: &std::path::Path) -> Result<AirSortie, String>
#[allow(unused_imports)] // P14: removed in step 9
pub use timing::{derived_timing, f80_transit_s}; // fn derived_timing(t: &ThresholdSpec) -> DerivedTiming; fn f80_transit_s(radius_m: f64) -> f64

/// Index convention for every per-side array (`schedule`, `AirBudget::per_side`, preview slot rows).
pub const DPRK: usize = 0;
pub const NATO: usize = 1;

pub type Ymd = (u16, u8, u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    pub from: Ymd,
    pub to: Ymd,
} // inclusive; also used by in_theatre

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirSide {
    Dprk,
    Nato,
} // map to the existing `eastern: bool` at call sites
impl AirSide {
    pub fn idx(self) -> usize {
        match self {
            AirSide::Dprk => DPRK,
            AirSide::Nato => NATO,
        }
    }
    pub fn letter(self) -> char {
        match self {
            AirSide::Dprk => 'D',
            AirSide::Nato => 'N',
        }
    } // §3.0 names
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirRole {
    AirPatrol,
    GroundAttack,
    Escort,
} // per lead

#[derive(Debug, Clone, PartialEq)]
pub enum SortieSource {
    BuiltIn(&'static str),
    File(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AirSortie {
    // one library item
    pub id: String,
    pub name: String,
    pub source: SortieSource,
    pub side: AirSide,
    pub models: Vec<String>,
    pub planes: usize,
    pub lead_roles: Vec<Vec<AirRole>>,
    pub wp_speed_kmh: f64,
    pub wp_alt_m: f64,
    pub attack_time_s: f64,
    pub valid: DateRange,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetRef {
    Objective { side: AirSide, xz: (f64, f64) },
    Arrow { tail: (f64, f64), tip: (f64, f64) },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TriggerMode {
    #[default]
    Clock,
    Zone,
    Both,
} // D16

#[derive(Debug, Clone, PartialEq)]
pub struct PoolEntry {
    pub sortie_id: String,
    pub target: TargetRef,
    pub weight: u8,
    pub mode: TriggerMode,
    pub runs: u8,
    pub heading_override: Option<f64>,
    pub enabled: bool, // runs: default 3, range 1..=6 (D12)
}
impl PoolEntry {
    // no Default: an entry always has a target
    pub fn new(sortie_id: String, target: TargetRef) -> Self {
        Self {
            sortie_id,
            target,
            weight: 5,
            mode: TriggerMode::Clock,
            runs: 3,
            heading_override: None,
            enabled: true,
        }
    }
} // weight 1..=10 (D15)

#[derive(Debug, Clone, PartialEq)]
pub struct AirBudget {
    pub per_side: [u8; 2],
} // [DPRK, NATO], each 1..=6
impl Default for AirBudget {
    fn default() -> Self {
        Self { per_side: [2, 2] }
    }
} // D5

#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdSpec {
    pub players_n: u8, // 1 in v1; the UI lets it rise from step 12 (U16)
    pub radius_m: f64,
    pub inner_ratio: f64,
    pub window_s: Option<f64>,
    pub dwell_s: Option<f64>,
    pub hold_s: Option<f64>, // None = derived (§4.2)
}
impl Default for ThresholdSpec {
    fn default() -> Self {
        Self {
            players_n: 1,
            radius_m: 12_000.0,
            inner_ratio: 0.5,
            window_s: None,
            dwell_s: None,
            hold_s: None,
        }
    }
} // D1, D2, D3, D4

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DerivedTiming {
    pub window_s: f64,
    pub dwell_s: f64,
    pub hold_s: f64,
    pub inner_m: f64,
} // W, D, H, r_in (§4.2); overrides applied

#[derive(Debug, Clone, PartialEq)]
pub struct Schedule {
    // one per side, indexed DPRK / NATO
    pub enabled: bool,
    pub first_slot_min: f64,
    pub period_min: f64,
    pub jitter_steps: u8,
    pub jitter_step_min: f64,
    pub quiet_pct: u8,
    pub arrive_min: f64,
}
impl Default for Schedule {
    fn default() -> Self {
        Self {
            enabled: true,
            first_slot_min: 10.0,
            period_min: 20.0,
            jitter_steps: 3,
            jitter_step_min: 3.0,
            quiet_pct: 20,
            arrive_min: 5.0,
        }
    }
} // D6, D7

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneSpec {
    pub chance_pct: u8,
    pub cooldown_min: f64,
    pub spawn_dist_km: f64,
}
impl Default for ZoneSpec {
    fn default() -> Self {
        Self {
            chance_pct: 50,
            cooldown_min: 15.0,
            spawn_dist_km: 30.0,
        }
    }
} // D18, D7

#[derive(Debug, Clone, PartialEq)]
pub struct StopSpec {
    pub margin_min: f64,
    pub extension_min: f64,
}
impl Default for StopSpec {
    fn default() -> Self {
        Self {
            margin_min: 10.0,
            extension_min: 10.0,
        }
    }
} // D8

#[derive(Debug, Clone, PartialEq)]
pub struct TotalsLimits {
    pub aircraft: u32,
    pub mcus: u32,
    pub check_zones: u32,
}
impl Default for TotalsLimits {
    fn default() -> Self {
        Self {
            aircraft: 96,
            mcus: 1500,
            check_zones: 40,
        }
    }
} // D36

// Strategy enums (§8 step 0d table). The Default is the primary design.
// ThresholdImpl::ComplexTrigger and CounterResetImpl are reserved for step 12 (§3.9); v1 builders accept only ZoneOnly.
#[allow(dead_code)] // P14 step 12
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThresholdImpl {
    #[default]
    ZoneOnly,
    ComplexTrigger,
}
#[allow(dead_code)] // P14 step 12
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoneOutImpl {
    #[default]
    CheckZone,
    Watchdog,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LossesImpl {
    #[default]
    Type4,
    Latch,
}
#[allow(dead_code)] // P14 step 12
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CounterResetImpl {
    #[default]
    SetVal,
    Ring,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RandomImpl {
    #[default]
    InMission,
    Seeded,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BudgetImpl {
    #[default]
    HeadCount,
    SingleSlot,
} // §3.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CensusImpl {
    #[default]
    Spaced,
    SameTick,
} // §3.3, probe E7
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresenceCheckImpl {
    #[default]
    PulseInside,
    Closer0Race,
} // risk c

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Strategies {
    pub threshold: ThresholdImpl,
    pub zone_out: ZoneOutImpl,
    pub losses: LossesImpl,
    pub counter_reset: CounterResetImpl,
    pub random: RandomImpl,
    pub budget: BudgetImpl,
    pub census: CensusImpl,
    pub presence: PresenceCheckImpl,
    pub ai_counts: bool, // probe result a; true makes the D30 warning an error (§8 step 0d)
}

/// Cross-module wiring for one side (§3.0). Every builder appends; nothing edits another builder's MCU by name.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct SideLogic {
    pub(crate) mission_begin: Vec<i32>, // pulsed by the side's Translator Mission Begin
    pub(crate) init_off: Vec<i32>,      // switched off by AT s INIT OFF (Mission Begin + 0.5 s)
    pub(crate) hot_consumers: BTreeMap<usize, Vec<i32>>, // area index → ENii HOT / SNjj HOT relays (THsaa HOT ON / OFF)
    pub(crate) cold_consumers: BTreeMap<usize, Vec<i32>>, // area index → ENii COLD / SNjj COLD relays (THsaa COLD ON / OFF)
    pub(crate) gates: Vec<i32>, // SNjj START relays; each is its copy's gate (BUD s SHUT / OPEN)
    pub(crate) flags: Vec<i32>, // SNjj FLYING timers, in census order (BUD s CENSUS)
    pub(crate) budget_events: Vec<i32>, // SNjj START relays and the copies' delete timers; each gets BUD s MARK and BUD s CHECK
    pub(crate) links: Vec<(i32, i32)>, // (from, to): emit_side_logic adds `to` to the Targets of `from` (§3.0)
    pub(crate) pass_subscribers: BTreeMap<usize, Vec<i32>>, // area index → ZNii ARMED relays (targets of THsaa INNER, the PASS event)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AirPlan {
    pub library: Vec<AirSortie>, // Default is empty; the UI fills it with the built-ins (step 2)
    pub pool: Vec<PoolEntry>,
    pub budget: AirBudget,
    pub threshold: ThresholdSpec,
    pub schedule: [Schedule; 2], // [DPRK, NATO]
    pub zone: ZoneSpec,
    pub stop: StopSpec,
    pub limits: TotalsLimits,
    pub include_in_base_map: bool,
    pub strategies: Strategies,
    pub seed: u64,             // every Generate-time random choice; see "Seed" below
    pub debug_subtitles: bool, // false; not in the UI; set only by the step 11 test mission writer
}

pub struct AirContext<'a> {
    // supplied by the UI from Map state; no derives needed
    pub east_objectives: &'a [(f64, f64)],
    pub nato_objectives: &'a [(f64, f64)],
    #[allow(clippy::type_complexity)] // permanent: the field type is frozen (plan §5)
    pub arrows: &'a [((f64, f64), (f64, f64))],
    pub front: &'a [(f64, f64)], // the polyline the Map preview colours arrows with (see below)
    pub aabb: WorldAabb,
    pub date: Ymd,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AirLine {
    pub side: AirSide,
    pub entry: usize,
    pub from: (f64, f64),
    pub to: (f64, f64),
} // spawn → target
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AirRing {
    pub side: AirSide,
    pub centre: (f64, f64),
    pub radius_m: f64,
    pub inner: bool,
} // R, or r_in when inner
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AirPoint {
    pub side: AirSide,
    pub entry: usize,
    pub at: (f64, f64),
    pub target: (f64, f64),
} // spawn or RTB dot; RTB draws a thin line to target
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotMark {
    pub t_s: f64,
    pub jitter_span_s: f64,
    pub quiet_pct: u8,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AirPreview {
    pub lines: Vec<AirLine>,
    pub rings: Vec<AirRing>,
    pub spawns: Vec<AirPoint>,
    pub rtbs: Vec<AirPoint>,
    pub slot_times: [Vec<SlotMark>; 2], // [DPRK, NATO], 0–3 h (D29)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AirDrawCounts {
    pub lines: usize,
    pub rings: usize,
    pub spawns: usize,
    pub rtbs: usize,
}
impl AirPreview {
    pub fn counts(&self) -> AirDrawCounts {
        AirDrawCounts {
            lines: self.lines.len(),
            rings: self.rings.len(),
            spawns: self.spawns.len(),
            rtbs: self.rtbs.len(),
        }
    }
}

/// What the plan adds to a mission (D36). Index DPRK / NATO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AirTotals {
    pub copies: [u32; 2],
    pub aircraft: [u32; 2],
    pub mcus: [u32; 2],
    pub check_zones: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    Error,
    Warning,
} // Error blocks Generate; Warning is orange
#[derive(Debug, Clone, PartialEq)]
pub struct AirIssue {
    pub severity: IssueSeverity,
    pub text: String,
    pub rail_only: bool, // true only for "no enabled valid entry": an Error on the Air Tasking tab, an orange note on Map (§6.1)
}

pub fn weighted_random_pct(weights: &[u32]) -> Vec<u8> {
    let mut remaining: u64 = weights.iter().map(|&weight| u64::from(weight)).sum();
    weights
        .iter()
        .enumerate()
        .map(|(i, &weight)| {
            let pct = if i + 1 == weights.len() {
                100
            } else if remaining == 0 {
                0
            } else {
                (100.0 * f64::from(weight) / remaining as f64).round() as u8
            };
            remaining -= u64::from(weight);
            pct
        })
        .collect()
}
pub fn build_air_packs(_plan: &AirPlan, _ctx: &AirContext) -> Result<Vec<MapAirPack>, String> {
    Ok(vec![])
} // step 7
pub fn preview(_plan: &AirPlan, _ctx: &AirContext) -> AirPreview {
    AirPreview::default()
} // step 7
pub fn validate(_plan: &AirPlan, _ctx: &AirContext) -> Vec<AirIssue> {
    vec![]
} // step 7; feeds readiness_checks
pub fn totals(_plan: &AirPlan, _ctx: &AirContext) -> AirTotals {
    AirTotals::default()
} // step 7; counted from build_air_packs output

#[cfg(test)]
mod tests {
    use super::weighted_random_pct;

    #[test]
    fn weighted_random_pct_values() {
        assert_eq!(weighted_random_pct(&[5, 5, 5]), [33, 50, 100]);
        assert_eq!(weighted_random_pct(&[1, 3]), [25, 100]);
        assert_eq!(weighted_random_pct(&[]), Vec::<u8>::new());
    }
}
