//! Air tasking timing formulas are built in the rest of step 1.

use super::{DerivedTiming, ThresholdSpec};

pub fn f80_transit_s(_radius_m: f64) -> f64 {
    0.0
}

pub fn derived_timing(_t: &ThresholdSpec) -> DerivedTiming {
    DerivedTiming::default()
}
