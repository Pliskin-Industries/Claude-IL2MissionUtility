//! F-80 transit and threshold timings, shared by the P14 builders and UI.

use super::{DerivedTiming, ThresholdSpec};
use crate::model_spec;

/// Probe T-n may calibrate P14 without changing Template Builder's cruise data.
pub const F80_CRUISE_OVERRIDE_KMH: Option<f64> = None;

/// Seconds to fly the diameter of a zone at F-80 cruise speed.
pub fn f80_transit_s(radius_m: f64) -> f64 {
    f80_transit_with_override(radius_m, F80_CRUISE_OVERRIDE_KMH)
}

fn f80_transit_with_override(radius_m: f64, cruise_override_kmh: Option<f64>) -> f64 {
    let speed_kmh = cruise_override_kmh.unwrap_or_else(|| {
        model_spec::spec_for("f80c10")
            .and_then(|spec| spec.cruise_kmh)
            .expect("the F-80C model must have a cruise speed") as f64
    });
    2.0 * radius_m / (speed_kmh / 3.6)
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

#[cfg(test)]
mod tests {
    use super::*;

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
