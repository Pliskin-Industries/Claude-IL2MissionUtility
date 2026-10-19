//! F-80 transit and threshold timings, shared by the P14 builders and UI.

use super::{AirPlan, AirSortie, DerivedTiming, ThresholdSpec, TriggerMode};
use crate::model_spec;

/// Probe T-n may calibrate P14 without changing Template Builder's cruise data.
pub const F80_CRUISE_OVERRIDE_KMH: Option<f64> = None;

/// Seconds to fly the diameter of a zone at F-80 cruise speed.
pub fn f80_transit_s(radius_m: f64) -> f64 {
    2.0 * radius_m / (f80_cruise_kmh().kmh / 3.6)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F80Cruise {
    pub kmh: f64,
    pub measured: bool,
}

/// Cruise speed and whether it comes from the measured P14 override.
pub fn f80_cruise_kmh() -> F80Cruise {
    cruise_with_override(F80_CRUISE_OVERRIDE_KMH)
}

fn cruise_with_override(cruise_override_kmh: Option<f64>) -> F80Cruise {
    let speed_kmh = cruise_override_kmh.unwrap_or_else(|| {
        model_spec::spec_for("f80c10")
            .and_then(|spec| spec.cruise_kmh)
            .expect("the F-80C model must have a cruise speed") as f64
    });
    F80Cruise { kmh: speed_kmh, measured: cruise_override_kmh.is_some() }
}

/// Window/dwell default independently to the rounded transit. Hold defaults to
/// twice the effective window, including a manual window override (H = 2W).
pub fn derived_timing(t: &ThresholdSpec) -> DerivedTiming {
    let rounded_s = (f80_transit_s(t.radius_m) / 10.0).ceil() * 10.0;
    let window_s = t.window_s.unwrap_or(rounded_s);
    DerivedTiming {
        window_s,
        dwell_s: t.dwell_s.unwrap_or(rounded_s),
        hold_s: t.hold_s.unwrap_or(2.0 * window_s),
        inner_m: t.radius_m * t.inner_ratio,
    }
}

/// Final cutoff after arrival, attack, margin and the one permitted extension (§4.3).
/// Both uses the zone distance, just as the sortie assembly does.
pub fn hard_stop(sortie: &AirSortie, plan: &AirPlan, mode: TriggerMode) -> Option<f64> {
    if !sortie.wp_speed_kmh.is_finite() || sortie.wp_speed_kmh <= 0.0 {
        return None;
    }
    let after_arrival = sortie.attack_time_s + (plan.stop.margin_min + plan.stop.extension_min) * 60.0;
    let arrival = match mode {
        TriggerMode::Clock => plan.schedule[sortie.side.idx()].arrive_min * 60.0,
        TriggerMode::Zone | TriggerMode::Both => plan.zone.spawn_dist_km * 1000.0 / (sortie.wp_speed_kmh / 3.6),
    };
    Some(arrival + after_arrival)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f80_transit_with_override(radius_m: f64, cruise_override_kmh: Option<f64>) -> f64 {
        2.0 * radius_m / (cruise_with_override(cruise_override_kmh).kmh / 3.6)
    }

    #[test]
    fn hard_stop_matches_hvar_and_armed_recon_defaults() {
        let library = crate::airtask::builtin_library();
        let plan = AirPlan::default();
        assert_eq!(hard_stop(&library[1], &plan, TriggerMode::Clock), Some(2100.0));
        for mode in [TriggerMode::Zone, TriggerMode::Both] {
            assert!((hard_stop(&library[1], &plan, mode).unwrap() - 1963.6363636).abs() < 0.0001);
        }
        assert_eq!(hard_stop(&library[3], &plan, TriggerMode::Clock), Some(2700.0));
    }

    #[test]
    fn hard_stop_uses_side_schedule_speed_and_stop_settings() {
        let sortie = &crate::airtask::builtin_library()[5];
        let mut plan = AirPlan::default();
        plan.schedule[sortie.side.idx()].arrive_min = 7.0;
        plan.zone.spawn_dist_km = 39.0;
        plan.stop.margin_min = 2.0;
        plan.stop.extension_min = 3.0;
        assert_eq!(hard_stop(sortie, &plan, TriggerMode::Clock), Some(1320.0));
        assert_eq!(hard_stop(sortie, &plan, TriggerMode::Zone), Some(1260.0));
        assert_eq!(hard_stop(sortie, &plan, TriggerMode::Both), Some(1260.0));
    }

    #[test]
    fn hard_stop_rejects_nonpositive_and_nonfinite_spawn_speeds() {
        let mut sortie = crate::airtask::builtin_library().remove(1);
        sortie.source = super::super::SortieSource::File("zero-speed.Group".into());
        for speed in [0.0, -1.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            sortie.wp_speed_kmh = speed;
            for mode in [TriggerMode::Clock, TriggerMode::Zone, TriggerMode::Both] {
                assert_eq!(hard_stop(&sortie, &AirPlan::default(), mode), None);
            }
        }
    }

    #[test]
    fn f80_cruise_reports_value_and_measured_flag() {
        assert_eq!(f80_cruise_kmh(), F80Cruise { kmh: 732.0, measured: false });
        assert_eq!(cruise_with_override(Some(700.0)), F80Cruise { kmh: 700.0, measured: true });
        assert_eq!(cruise_with_override(None), F80Cruise { kmh: 732.0, measured: false });
    }

    #[test]
    fn f80_transit_and_derived_timing() {
        assert!((f80_transit_s(12_000.0) - 118.0).abs() < 0.1);
        for (radius_m, window_s) in [(10_000.0, 100.0), (12_000.0, 120.0), (16_000.0, 160.0)] {
            assert_eq!(
                derived_timing(&ThresholdSpec {
                    radius_m,
                    ..ThresholdSpec::default()
                }),
                DerivedTiming {
                    window_s,
                    dwell_s: window_s,
                    hold_s: 2.0 * window_s,
                    inner_m: radius_m / 2.0
                }
            );
        }
        let mut t = ThresholdSpec {
            window_s: Some(70.0),
            ..ThresholdSpec::default()
        };
        assert_eq!(
            derived_timing(&t),
            DerivedTiming {
                window_s: 70.0,
                dwell_s: 120.0,
                hold_s: 140.0,
                inner_m: 6000.0,
            }
        );
        t.window_s = None;
        t.dwell_s = Some(85.0);
        assert_eq!(
            derived_timing(&t),
            DerivedTiming {
                window_s: 120.0,
                dwell_s: 85.0,
                hold_s: 240.0,
                inner_m: 6000.0,
            }
        );
        t.window_s = Some(75.0);
        t.hold_s = Some(190.0);
        t.inner_ratio = 0.4;
        assert_eq!(
            derived_timing(&t),
            DerivedTiming {
                window_s: 75.0,
                dwell_s: 85.0,
                hold_s: 190.0,
                inner_m: 4800.0,
            }
        );
        t.window_s = Some(0.0);
        t.dwell_s = Some(0.0);
        t.hold_s = Some(0.0);
        let derived = derived_timing(&t);
        assert_eq!(
            (derived.window_s, derived.dwell_s, derived.hold_s),
            (0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn timing_override_wins_and_model_spec_is_not_written() {
        let cruise = model_spec::spec_for("f80c10").unwrap().cruise_kmh;
        assert_eq!(F80_CRUISE_OVERRIDE_KMH, None);
        assert!((f80_transit_with_override(12_000.0, Some(700.0)) - 123.4).abs() < 0.1);
        assert!((f80_transit_with_override(12_000.0, None) - 118.0).abs() < 0.1);
        assert_eq!(model_spec::spec_for("f80c10").unwrap().cruise_kmh, cruise);

        // Pin the production module's only catalogue access to its read-only API.
        let source = include_str!("timing.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        let accesses: Vec<_> = source
            .lines()
            .filter(|line| line.contains("model_spec::"))
            .map(str::trim)
            .collect();
        assert_eq!(accesses, ["model_spec::spec_for(\"f80c10\")"]);
        assert!(!source.contains("&mut"));
        assert!(!source.contains("unsafe"));
        assert!(!source.contains(".cruise_kmh ="));
    }
}
