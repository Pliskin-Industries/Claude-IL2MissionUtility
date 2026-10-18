//! One player-country trigger per area, with periodic presence checks (0023).

use std::collections::BTreeMap;

use super::timing::derived_timing;
use super::{AirSide, PresenceCheckImpl, SideLogic, Strategies, ThresholdImpl, ThresholdSpec};
use crate::ast::Il2Entity;
use crate::{parser, template};

pub(crate) const NATO_PLAYER_COUNTRY: i32 = 601;
pub(crate) const DPRK_PLAYER_COUNTRY: i32 = 501;
#[allow(dead_code)] // P14 step 7
pub(crate) const NATO_AI_COUNTRY: i32 = 602;
#[allow(dead_code)] // P14 step 7
pub(crate) const DPRK_AI_COUNTRY: i32 = 503;

/// Object Entered, confirmed in flights 1N and 1D. See docs/il2-loader-and-log-facts.md.
pub(crate) const OBJECT_ENTERED: i32 = 58;
/// Object Left, confirmed in flights 1N and 1D. See docs/il2-loader-and-log-facts.md.
pub(crate) const OBJECT_LEFT: i32 = 60;

/// Build after consumers and subscribers register, before emit_side_logic.
/// The slice is enumerated from zero, but registry keys are the 1-based area
/// numbers already passed by assemble_side to ShellIds::register.
/// All MCUs use the area centre, as do the shell check zones.
/// Reject invalid inputs before allocating IDs or changing the registry.
pub(crate) fn build_side(
    side: AirSide,
    areas: &[(f64, f64)],
    spec: &ThresholdSpec,
    strategies: &Strategies,
    logic: &mut SideLogic,
    next_id: &mut i32,
) -> Result<Vec<Il2Entity>, String> {
    if spec.players_n != 1 {
        return Err("Air Tasking threshold requires exactly one player in step 4".into());
    }
    if strategies.threshold == ThresholdImpl::Counted {
        return Err("Air Tasking Counted threshold is reserved for step 12".into());
    }
    if !spec.radius_m.is_finite() || spec.radius_m <= 0.0 {
        return Err("Air Tasking threshold radius must be finite and greater than zero".into());
    }
    if !spec.inner_ratio.is_finite() || spec.inner_ratio <= 0.0 || spec.inner_ratio > 1.0 {
        return Err("Air Tasking threshold inner ratio must be finite and in (0, 1]".into());
    }
    let timing = derived_timing(spec);
    for (name, seconds) in [("dwell", timing.dwell_s), ("hold", timing.hold_s)] {
        if !seconds.is_finite() || seconds <= 0.0 {
            return Err(format!(
                "Air Tasking threshold effective {name} must be finite and greater than zero"
            ));
        }
    }
    for (index, &(x, z)) in areas.iter().enumerate() {
        if !x.is_finite() || !z.is_finite() {
            return Err(format!(
                "Air Tasking threshold area {} coordinates must be finite",
                index + 1
            ));
        }
    }
    if areas.is_empty() {
        return Ok(Vec::new());
    }
    // Clone the flown 1N shape, including both Enabled keys and key order.
    let reference = parser::parse_il2_document(include_str!(
        "../../TemplateExamples/Reference/ComplexTrigger_reference.Group"
    ))
    .expect("the Complex Trigger reference parses");
    let mut trigger = reference
        .find_by_name("Translator Complex Trigger")
        .expect("the reference contains a Complex Trigger")
        .clone();
    trigger
        .properties
        .retain(|(key, _)| !matches!(key.as_str(), "ObjectScript" | "ObjectName"));
    trigger.children.clear();
    let (own_country, watched_country) = match side {
        AirSide::Nato => (NATO_PLAYER_COUNTRY, DPRK_PLAYER_COUNTRY),
        AirSide::Dprk => (DPRK_PLAYER_COUNTRY, NATO_PLAYER_COUNTRY),
    };
    let coalition = crate::aircraft::plane_coalitions_for_country(own_country);
    trigger.set_property("Country", watched_country.to_string());
    trigger.set_property("Radius", format!("{:.0}", timing.inner_m));
    trigger.set_property("Cylinder", "1");
    trigger.set_property("CheckPlanes", "1");
    let reactivate = strategies.presence == PresenceCheckImpl::TriggerReactivate;
    let mut result = Vec::new();
    for (index, &(x, z)) in areas.iter().enumerate() {
        let area = index + 1;
        let prefix = format!("TH{}{:02}", side.letter(), area);
        let mut cell = Vec::new();
        let mut ids = BTreeMap::new();
        for (role, kind, time) in [
            ("TRIGGER", "MCU_TR_ComplexTrigger", 0.0),
            ("ARM", "MCU_Timer", 0.0),
            ("ENTRY OFF", "MCU_Deactivate", 0.0),
            ("DWELL", "MCU_Timer", timing.dwell_s),
            ("CHECK ON", "MCU_Activate", 0.0),
            ("CLOSE", "MCU_Timer", 2.0),
            ("CLOSE OFF", "MCU_Deactivate", 0.0),
            ("CLOSE DELAY", "MCU_Timer", 0.05),
            ("SEEN", "MCU_Timer", 0.0),
            ("INNER", "MCU_CheckZone", 0.0),
            ("INNER PULSE", "MCU_Timer", 0.1),
            ("PASS OFF", "MCU_Deactivate", 0.0),
            ("HOT ON", "MCU_Activate", 0.0),
            ("COLD OFF DELAY", "MCU_Timer", 0.05),
            ("COLD OFF", "MCU_Deactivate", 0.0),
            ("HOLD", "MCU_Timer", timing.hold_s),
            ("COLD ON", "MCU_Activate", 0.0),
            ("HOT OFF DELAY", "MCU_Timer", 0.05),
            ("HOT OFF", "MCU_Deactivate", 0.0),
            ("REARM ON", "MCU_Activate", 0.0),
        ] {
            if (reactivate && matches!(role, "INNER" | "INNER PULSE"))
                || (!reactivate && role == "SEEN")
            {
                continue;
            }
            let name = format!("{prefix} {role}");
            let node = match kind {
                "MCU_TR_ComplexTrigger" => {
                    let mut node = trigger.clone();
                    node.index = Some(*next_id);
                    node.set_property("Index", next_id.to_string());
                    *next_id += 1;
                    node.set_name(&name);
                    node.set_property("XPos", format!("{x:.3}"));
                    node.set_property("ZPos", format!("{z:.3}"));
                    node
                }
                "MCU_Timer" => template::timer(&name, time, next_id, x, z),
                "MCU_CheckZone" => {
                    template::checkzone(&name, timing.inner_m, true, coalition, next_id, x, z)
                }
                _ => template::mcu(kind, &name, next_id, x, z),
            };
            ids.insert(role, node.index.unwrap());
            cell.push(node);
        }
        let consumers = |map: &BTreeMap<usize, Vec<i32>>| {
            let mut ids = map.get(&area).cloned().unwrap_or_default();
            ids.sort_unstable();
            ids
        };
        let hot = consumers(&logic.hot_consumers);
        let cold = consumers(&logic.cold_consumers);
        let subscribers = consumers(&logic.pass_subscribers);
        let mut check = vec![ids["CHECK ON"], ids["CLOSE"]];
        let (check_on, close_off, pass_off) = if reactivate {
            (
                vec![ids["SEEN"], ids["CLOSE DELAY"], ids["TRIGGER"]],
                vec![ids["TRIGGER"], ids["SEEN"]],
                vec![ids["CLOSE DELAY"], ids["TRIGGER"], ids["SEEN"]],
            )
        } else {
            check.push(ids["INNER PULSE"]);
            (
                vec![ids["CLOSE DELAY"], ids["INNER"]],
                vec![ids["INNER"]],
                vec![ids["CLOSE DELAY"], ids["INNER"]],
            )
        };
        let mut pass = vec![
            ids["PASS OFF"],
            ids["HOT ON"],
            ids["COLD OFF DELAY"],
            ids["HOLD"],
        ];
        pass.extend(subscribers);
        let mut edges = BTreeMap::from([
            ("ARM", vec![ids["ENTRY OFF"], ids["DWELL"]]),
            ("ENTRY OFF", vec![ids["TRIGGER"], ids["ARM"]]),
            ("DWELL", check.clone()),
            ("HOLD", check),
            ("CHECK ON", check_on),
            ("CLOSE", vec![ids["CLOSE OFF"], ids["CLOSE DELAY"]]),
            ("CLOSE OFF", close_off),
            ("PASS OFF", pass_off),
            (if reactivate { "SEEN" } else { "INNER" }, pass),
            ("HOT ON", hot.clone()),
            ("COLD OFF DELAY", vec![ids["COLD OFF"]]),
            ("COLD OFF", cold.clone()),
            (
                "CLOSE DELAY",
                vec![ids["COLD ON"], ids["HOT OFF DELAY"], ids["REARM ON"]],
            ),
            ("COLD ON", cold),
            ("HOT OFF DELAY", vec![ids["HOT OFF"]]),
            ("HOT OFF", hot.clone()),
            ("REARM ON", vec![ids["ARM"], ids["TRIGGER"]]),
        ]);
        if !reactivate {
            edges.insert("INNER PULSE", vec![ids["INNER"]]);
        }
        for node in &mut cell {
            if node.index == Some(ids["TRIGGER"]) {
                template::attach_event(node, OBJECT_ENTERED, ids["ARM"]);
                if reactivate {
                    template::attach_event(node, OBJECT_ENTERED, ids["SEEN"]);
                }
            } else {
                let role = node.name().unwrap().strip_prefix(&prefix).unwrap().trim();
                node.set_targets(edges.remove(role).expect("every cell MCU has its edges"));
            }
        }
        // ShellIds::register already owns its HOT start state. Fill in missing
        // consumers (for example a clock pair), without registering any twice.
        for id in hot.into_iter().chain(reactivate.then(|| ids["SEEN"])) {
            if !logic.init_off.contains(&id) {
                logic.init_off.push(id);
            }
        }
        result.extend(cell);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airtask::assemble::emit_side_logic;
    use crate::airtask::library::{by_id, nodes, number};
    use crate::airtask::shell::test_support::{active, mcus, named, ticks, times};
    use crate::airtask::testkit::*;
    use std::collections::BTreeSet;

    const CENTRE: (f64, f64) = (123_000.0, 234_000.0);

    struct Harness {
        root: Il2Entity,
        logic: SideLogic,
        side: AirSide,
        presence: PresenceCheckImpl,
    }

    fn harness(
        side: AirSide,
        areas: &[(f64, f64)],
        spec: ThresholdSpec,
        presence: PresenceCheckImpl,
    ) -> Harness {
        let mut next = 1;
        let mut root = Il2Entity::new("Group");
        let mut logic = SideLogic::default();
        for area in 1..=areas.len() {
            // Two consumers and subscribers expose incomplete fanouts. One HOT
            // is already registered by its owner, as ShellIds::register does.
            for consumer in 1..=2 {
                for role in ["HOT", "COLD", "SUBSCRIBER"] {
                    let node = template::timer(
                        &format!("Consumer {area} {consumer} {role}"),
                        0.0,
                        &mut next,
                        0.0,
                        0.0,
                    );
                    let id = node.index.unwrap();
                    let map = match role {
                        "HOT" => &mut logic.hot_consumers,
                        "COLD" => &mut logic.cold_consumers,
                        _ => &mut logic.pass_subscribers,
                    };
                    map.entry(area).or_default().insert(0, id); // Builder sorts sets.
                    if role == "HOT" && consumer == 1 {
                        logic.init_off.push(id);
                    }
                    root.children.push(node);
                }
            }
        }
        let mut cells = Il2Entity::new("Group");
        cells.set_name("Cells");
        cells.children = build_side(
            side,
            areas,
            &spec,
            &Strategies {
                presence,
                ..Strategies::default()
            },
            &mut logic,
            &mut next,
        )
        .unwrap();
        root.children.push(cells);
        let init =
            emit_side_logic(side, &logic, std::slice::from_mut(&mut root), &mut next).unwrap();
        root.children.extend(init);
        Harness {
            root,
            logic,
            side,
            presence,
        }
    }

    fn default_harness(presence: PresenceCheckImpl) -> Harness {
        harness(AirSide::Nato, &[CENTRE], ThresholdSpec::default(), presence)
    }

    fn cell<'a>(h: &'a Harness, area: usize, role: &str) -> &'a Il2Entity {
        named(&h.root, &format!("TH{}{:02} {role}", h.side.letter(), area))
    }

    fn id(h: &Harness, role: &str) -> i32 {
        cell(h, 1, role).index.unwrap()
    }

    fn sorted(ids: &[i32]) -> Vec<i32> {
        let mut ids = ids.to_vec();
        ids.sort_unstable();
        ids
    }

    fn assert_structure(h: &Harness, area: usize, centre: (f64, f64)) {
        let reactivate = h.presence == PresenceCheckImpl::TriggerReactivate;
        let node_id = |role: &str| cell(h, area, role).index.unwrap();
        let ids = |roles: &[&str]| roles.iter().map(|role| node_id(role)).collect::<Vec<_>>();
        let mut check = ids(&["CHECK ON", "CLOSE"]);
        if !reactivate {
            check.push(node_id("INNER PULSE"));
        }
        let mut pass = ids(&["PASS OFF", "HOT ON", "COLD OFF DELAY", "HOLD"]);
        pass.extend(sorted(&h.logic.pass_subscribers[&area]));
        let mut expected = vec![
            ("TRIGGER", "MCU_TR_ComplexTrigger", vec![]),
            ("ARM", "MCU_Timer", ids(&["ENTRY OFF", "DWELL"])),
            ("ENTRY OFF", "MCU_Deactivate", ids(&["TRIGGER", "ARM"])),
            ("DWELL", "MCU_Timer", check.clone()),
            ("HOLD", "MCU_Timer", check),
            (
                "CHECK ON",
                "MCU_Activate",
                if reactivate {
                    ids(&["SEEN", "CLOSE DELAY", "TRIGGER"])
                } else {
                    ids(&["CLOSE DELAY", "INNER"])
                },
            ),
            ("CLOSE", "MCU_Timer", ids(&["CLOSE OFF", "CLOSE DELAY"])),
            (
                "CLOSE OFF",
                "MCU_Deactivate",
                if reactivate {
                    ids(&["TRIGGER", "SEEN"])
                } else {
                    ids(&["INNER"])
                },
            ),
            (
                "CLOSE DELAY",
                "MCU_Timer",
                ids(&["COLD ON", "HOT OFF DELAY", "REARM ON"]),
            ),
            (
                "PASS OFF",
                "MCU_Deactivate",
                if reactivate {
                    ids(&["CLOSE DELAY", "TRIGGER", "SEEN"])
                } else {
                    ids(&["CLOSE DELAY", "INNER"])
                },
            ),
            (
                "HOT ON",
                "MCU_Activate",
                sorted(&h.logic.hot_consumers[&area]),
            ),
            ("COLD OFF DELAY", "MCU_Timer", ids(&["COLD OFF"])),
            (
                "COLD OFF",
                "MCU_Deactivate",
                sorted(&h.logic.cold_consumers[&area]),
            ),
            (
                "COLD ON",
                "MCU_Activate",
                sorted(&h.logic.cold_consumers[&area]),
            ),
            ("HOT OFF DELAY", "MCU_Timer", ids(&["HOT OFF"])),
            (
                "HOT OFF",
                "MCU_Deactivate",
                sorted(&h.logic.hot_consumers[&area]),
            ),
            ("REARM ON", "MCU_Activate", ids(&["ARM", "TRIGGER"])),
        ];
        if reactivate {
            expected.push(("SEEN", "MCU_Timer", pass));
        } else {
            expected.extend([
                ("INNER", "MCU_CheckZone", pass),
                ("INNER PULSE", "MCU_Timer", ids(&["INNER"])),
            ]);
            let inner = cell(h, area, "INNER");
            assert_eq!(inner.property("Closer"), Some("1"));
            assert_eq!(inner.property("Cylinder"), Some("1"));
            assert_eq!(number(inner, "Zone"), Ok(6000.0));
            assert_eq!(
                inner.property("PlaneCoalitions"),
                Some(if h.side == AirSide::Nato {
                    "[1]"
                } else {
                    "[2]"
                })
            );
        }
        for (role, kind, targets) in &expected {
            let node = cell(h, area, role);
            assert_eq!(&node.block_type, kind, "{role}");
            assert_eq!(&node.targets, targets, "{role}");
            assert!(node.objects.is_empty(), "{role}");
            assert_eq!(node.pos_xz(), Some(centre), "{role}");
            if *role != "TRIGGER" {
                assert!(node.children.is_empty(), "{role}");
            }
        }
        let prefix = format!("TH{}{:02} ", h.side.letter(), area);
        let actual: BTreeSet<_> = nodes(named(&h.root, "Cells"))
            .into_iter()
            .filter_map(|node| node.name()?.strip_prefix(&prefix))
            .collect();
        assert_eq!(actual, expected.iter().map(|(role, _, _)| *role).collect());
        for (role, time) in [
            ("ARM", 0.0),
            ("DWELL", 60.0),
            ("HOLD", 120.0),
            ("CLOSE", 2.0),
            ("CLOSE DELAY", 0.05),
            ("COLD OFF DELAY", 0.05),
            ("HOT OFF DELAY", 0.05),
        ] {
            let node = cell(h, area, role);
            assert_eq!(number(node, "Time"), Ok(time), "{role}");
            assert_eq!(node.property("Random"), Some("100"), "{role}");
        }
        assert_eq!(
            number(
                cell(h, area, if reactivate { "SEEN" } else { "INNER PULSE" }),
                "Time"
            ),
            Ok(if reactivate { 0.0 } else { 0.1 })
        );
        let trigger = cell(h, area, "TRIGGER");
        assert_eq!(number(trigger, "Radius"), Ok(6000.0));
        assert_eq!(trigger.property("Cylinder"), Some("1"));
        assert_eq!(trigger.property("CheckPlanes"), Some("1"));
        assert_eq!(
            trigger.property("Country"),
            Some(if h.side == AirSide::Nato {
                "501"
            } else {
                "601"
            })
        );
        assert_eq!(trigger.property("ObjectScript"), None);
        assert_eq!(trigger.property("ObjectName"), None);
        let reference = parser::parse_il2_document(include_str!(
            "../../TemplateExamples/Reference/ComplexTrigger_reference.Group"
        ))
        .unwrap();
        let reference = named(&reference, "Translator Complex Trigger");
        let reference_properties: Vec<_> = reference
            .properties
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "ObjectScript" | "ObjectName"))
            .collect();
        assert_eq!(
            trigger
                .properties
                .iter()
                .map(|(k, _)| k)
                .collect::<Vec<_>>(),
            reference_properties
                .iter()
                .map(|(k, _)| k)
                .collect::<Vec<_>>()
        );
        for (key, value) in reference_properties {
            if !matches!(
                key.as_str(),
                "Index" | "Name" | "XPos" | "ZPos" | "Radius" | "Country"
            ) {
                assert_eq!(trigger.property(key), Some(value.as_str()), "{key}");
            }
        }
        assert_eq!(
            trigger
                .properties
                .iter()
                .filter(|(k, v)| k == "Enabled" && v == "1")
                .count(),
            2
        );
        assert_eq!(trigger.children.len(), 1);
        assert_eq!(trigger.children[0].block_type, "OnEvents");
        let events: Vec<_> = trigger.children[0]
            .children
            .iter()
            .map(|event| template::read_event(event).unwrap())
            .collect();
        let mut expected_events = vec![(58, node_id("ARM"))];
        if reactivate {
            expected_events.push((58, node_id("SEEN")));
        }
        assert_eq!(events, expected_events);
        for node in nodes(named(&h.root, "Cells")) {
            for key in ["Name", "Desc"] {
                if let Some(text) = node.property(key) {
                    assert!(text.is_ascii() && !text.contains([';', '=', '{', '}', '\'']));
                }
            }
        }
        assert!(h.logic.mission_begin.is_empty());
        assert!(h.logic.links.is_empty());
        let mut expected_off: BTreeSet<_> =
            h.logic.hot_consumers.values().flatten().copied().collect();
        if reactivate {
            expected_off.extend(
                (1..=h.logic.hot_consumers.len()).map(|a| cell(h, a, "SEEN").index.unwrap()),
            );
        }
        assert_eq!(
            h.logic.init_off.len(),
            expected_off.len(),
            "no duplicated HOT registration"
        );
        assert_eq!(
            h.logic.init_off.iter().copied().collect::<BTreeSet<_>>(),
            expected_off
        );
    }

    #[test]
    fn threshold_structure() {
        assert_structure(
            &default_harness(PresenceCheckImpl::TriggerReactivate),
            1,
            CENTRE,
        );
    }

    #[test]
    fn threshold_countries() {
        for (side, country) in [(AirSide::Nato, "501"), (AirSide::Dprk, "601")] {
            let h = harness(
                side,
                &[CENTRE],
                ThresholdSpec::default(),
                PresenceCheckImpl::TriggerReactivate,
            );
            assert_eq!(cell(&h, 1, "TRIGGER").property("Country"), Some(country));
            assert_structure(&h, 1, CENTRE);
        }
        assert_eq!((NATO_AI_COUNTRY, DPRK_AI_COUNTRY), (602, 503));
    }

    #[test]
    fn threshold_hold_does_not_switch_hot_off() {
        for presence in [
            PresenceCheckImpl::TriggerReactivate,
            PresenceCheckImpl::ZonePulse,
        ] {
            let h = default_harness(presence);
            assert_eq!(cell(&h, 1, "HOLD").targets, cell(&h, 1, "DWELL").targets);
            for role in ["HOLD", "CHECK ON", "CLOSE", "CLOSE OFF"] {
                assert!(!cell(&h, 1, role).targets.contains(&id(&h, "HOT OFF")));
                assert!(!cell(&h, 1, role).targets.contains(&id(&h, "HOT OFF DELAY")));
            }
            assert_eq!(cell(&h, 1, "HOT OFF DELAY").targets, [id(&h, "HOT OFF")]);
            assert_eq!(
                nodes(named(&h.root, "Cells"))
                    .into_iter()
                    .filter(|n| n.targets.contains(&id(&h, "HOT OFF DELAY")))
                    .map(|n| n.index.unwrap())
                    .collect::<Vec<_>>(),
                [id(&h, "CLOSE DELAY")]
            );
        }
    }

    #[test]
    fn threshold_has_18_mcus_one_trigger_no_check_zone() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        let cells = named(&h.root, "Cells");
        assert_eq!(mcus(cells), 18);
        assert_eq!(cells.count_block_type("MCU_TR_ComplexTrigger"), 1);
        assert_eq!(cells.count_block_type("MCU_CheckZone"), 0);
    }

    #[test]
    fn threshold_zone_pulse_has_19_mcus_one_trigger_one_check_zone() {
        let h = default_harness(PresenceCheckImpl::ZonePulse);
        let cells = named(&h.root, "Cells");
        assert_eq!(mcus(cells), 19);
        assert_eq!(cells.count_block_type("MCU_TR_ComplexTrigger"), 1);
        assert_eq!(cells.count_block_type("MCU_CheckZone"), 1);
    }

    #[test]
    fn threshold_zone_pulse_structure() {
        for side in [AirSide::Nato, AirSide::Dprk] {
            assert_structure(
                &harness(
                    side,
                    &[CENTRE],
                    ThresholdSpec::default(),
                    PresenceCheckImpl::ZonePulse,
                ),
                1,
                CENTRE,
            );
        }
    }

    #[test]
    fn threshold_timing_follows_radius() {
        for presence in [
            PresenceCheckImpl::TriggerReactivate,
            PresenceCheckImpl::ZonePulse,
        ] {
            for (spec, radius, dwell, hold) in [
                (
                    ThresholdSpec {
                        radius_m: 16_000.0,
                        ..ThresholdSpec::default()
                    },
                    8000.0,
                    80.0,
                    160.0,
                ),
                (
                    ThresholdSpec {
                        window_s: Some(7.0),
                        dwell_s: Some(35.0),
                        ..ThresholdSpec::default()
                    },
                    6000.0,
                    35.0,
                    70.0,
                ),
                (
                    ThresholdSpec {
                        dwell_s: Some(35.0),
                        hold_s: Some(45.0),
                        ..ThresholdSpec::default()
                    },
                    6000.0,
                    35.0,
                    45.0,
                ),
            ] {
                let h = harness(AirSide::Nato, &[CENTRE], spec, presence);
                assert_eq!(number(cell(&h, 1, "TRIGGER"), "Radius"), Ok(radius));
                assert_eq!(number(cell(&h, 1, "DWELL"), "Time"), Ok(dwell));
                assert_eq!(number(cell(&h, 1, "HOLD"), "Time"), Ok(hold));
                if presence == PresenceCheckImpl::ZonePulse {
                    assert_eq!(number(cell(&h, 1, "INNER"), "Zone"), Ok(radius));
                }
            }
        }
    }

    #[test]
    fn threshold_refuses_more_than_one_player() {
        for players_n in [0, 2, 3, 255] {
            let mut logic = SideLogic::default();
            let mut next = 42;
            let error = build_side(
                AirSide::Nato,
                &[CENTRE],
                &ThresholdSpec {
                    players_n,
                    ..ThresholdSpec::default()
                },
                &Strategies::default(),
                &mut logic,
                &mut next,
            )
            .unwrap_err();
            assert!(error.contains("exactly one player"));
            assert_eq!(next, 42);
            assert_eq!(logic, SideLogic::default());
        }
    }

    #[test]
    fn threshold_refuses_counted() {
        let mut logic = SideLogic::default();
        let mut next = 42;
        let error = build_side(
            AirSide::Nato,
            &[CENTRE],
            &ThresholdSpec::default(),
            &Strategies {
                threshold: ThresholdImpl::Counted,
                ..Strategies::default()
            },
            &mut logic,
            &mut next,
        )
        .unwrap_err();
        assert!(error.contains("Counted") && error.contains("step 12"));
        assert_eq!(next, 42);
        assert_eq!(logic, SideLogic::default());
    }

    #[test]
    fn threshold_refuses_invalid_input() {
        let assert_invalid = |spec: ThresholdSpec, areas: &[(f64, f64)], message: &str| {
            let mut logic = SideLogic {
                init_off: vec![9],
                mission_begin: vec![10],
                hot_consumers: BTreeMap::from([(1, vec![11])]),
                ..SideLogic::default()
            };
            let before = logic.clone();
            let mut next = 42;
            let error = build_side(
                AirSide::Nato,
                areas,
                &spec,
                &Strategies::default(),
                &mut logic,
                &mut next,
            )
            .unwrap_err();
            assert_eq!(error, message, "{spec:?}, {areas:?}");
            assert_eq!(next, 42, "{spec:?}, {areas:?}");
            assert_eq!(logic, before, "{spec:?}, {areas:?}");
        };
        for radius_m in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            assert_invalid(
                ThresholdSpec {
                    radius_m,
                    ..ThresholdSpec::default()
                },
                &[CENTRE],
                "Air Tasking threshold radius must be finite and greater than zero",
            );
        }
        for inner_ratio in [f64::NAN, 0.0, -0.5, 1.5] {
            assert_invalid(
                ThresholdSpec {
                    inner_ratio,
                    ..ThresholdSpec::default()
                },
                &[CENTRE],
                "Air Tasking threshold inner ratio must be finite and in (0, 1]",
            );
        }
        for dwell_s in [f64::NAN, 0.0, -1.0] {
            assert_invalid(
                ThresholdSpec {
                    dwell_s: Some(dwell_s),
                    ..ThresholdSpec::default()
                },
                &[CENTRE],
                "Air Tasking threshold effective dwell must be finite and greater than zero",
            );
        }
        for hold_s in [f64::INFINITY, 0.0] {
            assert_invalid(
                ThresholdSpec {
                    hold_s: Some(hold_s),
                    ..ThresholdSpec::default()
                },
                &[CENTRE],
                "Air Tasking threshold effective hold must be finite and greater than zero",
            );
        }
        for area in [(f64::NAN, CENTRE.1), (CENTRE.0, f64::INFINITY)] {
            assert_invalid(
                ThresholdSpec::default(),
                &[CENTRE, area],
                "Air Tasking threshold area 2 coordinates must be finite",
            );
        }
    }

    #[test]
    fn threshold_links_resolve_and_rearm_rule() {
        for presence in [
            PresenceCheckImpl::TriggerReactivate,
            PresenceCheckImpl::ZonePulse,
        ] {
            let h = default_harness(presence);
            assert_links_resolve(&h.root);
            walker_every_deactivated_out_has_a_rearm_or_is_single_use(&h.root, &[]).unwrap();
            for trigger in nodes(&h.root)
                .into_iter()
                .filter(|n| n.block_type == "MCU_TR_ComplexTrigger")
            {
                for event in &trigger.children[0].children {
                    by_id(&h.root, template::read_event(event).unwrap().1);
                }
            }
            // This cell cancels timers directly, so also check beyond OUT relays.
            let activated: BTreeSet<_> = nodes(named(&h.root, "Cells"))
                .into_iter()
                .filter(|n| n.block_type == "MCU_Activate")
                .flat_map(|n| &n.targets)
                .collect();
            for off in nodes(named(&h.root, "Cells"))
                .into_iter()
                .filter(|n| n.block_type == "MCU_Deactivate")
            {
                assert!(off.targets.iter().all(|target| activated.contains(target)));
            }
        }
    }

    #[test]
    fn threshold_empty_areas_and_one_based_registry_keys() {
        let mut next = 42;
        let mut logic = SideLogic {
            init_off: vec![9],
            mission_begin: vec![10],
            ..SideLogic::default()
        };
        let before = logic.clone();
        assert!(
            build_side(
                AirSide::Nato,
                &[],
                &ThresholdSpec::default(),
                &Strategies::default(),
                &mut logic,
                &mut next
            )
            .unwrap()
            .is_empty()
        );
        assert_eq!(logic, before);
        assert_eq!(next, 42);
        for presence in [
            PresenceCheckImpl::TriggerReactivate,
            PresenceCheckImpl::ZonePulse,
        ] {
            let other = (CENTRE.0 + 50_000.0, CENTRE.1 - 50_000.0);
            let h = harness(
                AirSide::Nato,
                &[CENTRE, other],
                ThresholdSpec::default(),
                presence,
            );
            assert_structure(&h, 1, CENTRE);
            assert_structure(&h, 2, other);
            assert_eq!(
                mcus(named(&h.root, "Cells")),
                if presence == PresenceCheckImpl::TriggerReactivate {
                    36
                } else {
                    38
                }
            );
        }
    }

    fn game_config() -> WalkerConfig {
        // Flight 0 J1, also used by the flight 2 probe walk schedules.
        WalkerConfig {
            deactivate_running_timer: DeactivateRunningTimer::Cancels,
            ..WalkerConfig::default()
        }
    }

    fn game_configs() -> Vec<WalkerConfig> {
        crate::airtask::shell::test_support::configs()
            .into_iter()
            .map(|config| WalkerConfig {
                deactivate_running_timer: DeactivateRunningTimer::Cancels,
                ..config
            })
            .collect()
    }

    fn track(
        country: i32,
        from: u64,
        until: u64,
        position: impl Fn(u64) -> (f64, f64),
    ) -> ScriptedTrack {
        ScriptedTrack {
            coalition: if country >= 600 { 2 } else { 1 },
            country,
            samples: (from..=until)
                .map(|second| {
                    let (x, z) = position(second);
                    (second * TICKS_PER_SECOND, x, z)
                })
                .collect(),
        }
    }

    fn orbit(second: u64) -> (f64, f64) {
        let angle = second as f64 / 10.0;
        (
            CENTRE.0 + 1000.0 * angle.cos(),
            CENTRE.1 + 1000.0 * angle.sin(),
        )
    }

    fn outside() -> (f64, f64) {
        (CENTRE.0 + 13_000.0, CENTRE.1)
    }

    fn run(h: &Harness, config: WalkerConfig, until: f64, tracks: Vec<ScriptedTrack>) -> WalkerLog {
        walk(
            &h.root,
            config,
            &WalkerInput {
                until: ticks(until),
                tracks,
                ..WalkerInput::default()
            },
        )
    }

    fn assert_passes(h: &Harness, log: &WalkerLog, expected: &[f64]) {
        let expected: Vec<_> = expected.iter().map(|&t| ticks(t)).collect();
        for role in [
            if h.presence == PresenceCheckImpl::TriggerReactivate {
                "SEEN"
            } else {
                "INNER"
            },
            "PASS OFF",
            "HOT ON",
            "COLD OFF DELAY",
        ] {
            let expected = if role == "COLD OFF DELAY" {
                expected.iter().map(|t| t + 1).collect()
            } else {
                expected.clone()
            };
            assert_eq!(times(log, id(h, role)), expected, "{role}");
        }
        for &subscriber in &h.logic.pass_subscribers[&1] {
            assert_eq!(times(log, subscriber), expected);
        }
    }

    fn assert_cold(h: &Harness, log: &WalkerLog) {
        for snapshot in log.snapshots.iter().filter(|s| s.t >= ticks(0.5)) {
            assert!(
                h.logic.hot_consumers[&1]
                    .iter()
                    .all(|id| !snapshot.active[id])
            );
            assert!(
                h.logic.cold_consumers[&1]
                    .iter()
                    .all(|id| snapshot.active[id])
            );
        }
    }

    #[test]
    fn threshold_one_zone_loiter_passes() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        for config in game_configs() {
            for same_tick_delivery in [SameTickDelivery::DepthFirst, SameTickDelivery::Batched] {
                let config = WalkerConfig {
                    same_tick_delivery,
                    ..config
                };
                let log = run(
                    &h,
                    config,
                    70.0,
                    vec![track(DPRK_PLAYER_COUNTRY, 1, 70, orbit)],
                );
                let passes = times(&log, id(&h, "SEEN"));
                assert_eq!(passes.len(), 1, "{config:?}");
                assert!(
                    (ticks(60.0)..=ticks(62.1)).contains(&(passes[0] - ticks(1.0))),
                    "{config:?}"
                );
                // Samples precede future timer expiries at the same tick, so
                // the 61 s DWELL activation is first evaluated at 62 s.
                assert_passes(&h, &log, &[62.0]);
                assert!(times(&log, id(&h, "CLOSE DELAY")).is_empty());
                assert_eq!(times(&log, id(&h, "ARM")), [ticks(1.0)]);
                assert_eq!(times(&log, id(&h, "DWELL")), [ticks(61.0)]);
            }
        }
    }

    #[test]
    fn threshold_one_zone_straight_pass_fails() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        for config in game_configs() {
            let log = run(
                &h,
                config,
                130.0,
                vec![track(DPRK_PLAYER_COUNTRY, 1, 130, |s| {
                    (CENTRE.0 - 6000.0 + (s - 1) as f64 * (732.0 / 3.6), CENTRE.1)
                })],
            );
            assert_passes(&h, &log, &[]);
            assert_eq!(times(&log, id(&h, "ARM")), [ticks(1.0)]);
            assert_eq!(times(&log, id(&h, "CLOSE DELAY")), [ticks(63.05)]);
            assert_cold(&h, &log);
        }
    }

    #[test]
    fn threshold_one_zone_edge_orbit_fails() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        let log = run(
            &h,
            game_config(),
            600.0,
            vec![track(DPRK_PLAYER_COUNTRY, 1, 600, |s| {
                let angle = s as f64 / 30.0;
                (
                    CENTRE.0 + 9000.0 * angle.cos(),
                    CENTRE.1 + 9000.0 * angle.sin(),
                )
            })],
        );
        assert_passes(&h, &log, &[]);
        for role in ["TRIGGER", "ARM", "DWELL", "CLOSE DELAY"] {
            assert!(times(&log, id(&h, role)).is_empty());
        }
        assert_cold(&h, &log);
    }

    #[test]
    fn threshold_ai_country_does_not_count() {
        for (side, country) in [
            (AirSide::Nato, DPRK_AI_COUNTRY),
            (AirSide::Dprk, NATO_AI_COUNTRY),
        ] {
            let h = harness(
                side,
                &[CENTRE],
                ThresholdSpec::default(),
                PresenceCheckImpl::TriggerReactivate,
            );
            let log = run(
                &h,
                game_config(),
                600.0,
                vec![track(country, 1, 600, orbit)],
            );
            assert_passes(&h, &log, &[]);
            assert!(times(&log, id(&h, "TRIGGER")).is_empty());
            assert!(times(&log, id(&h, "ARM")).is_empty());
            assert_cold(&h, &log);
        }
    }

    fn leave_and_return() -> ScriptedTrack {
        track(DPRK_PLAYER_COUNTRY, 1, 400, |s| {
            if (100..250).contains(&s) {
                outside()
            } else {
                orbit(s)
            }
        })
    }

    #[test]
    fn threshold_hot_and_cold_are_never_both_off() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        for config in game_configs() {
            let log = run(&h, config, 400.0, vec![leave_and_return()]);
            assert_passes(&h, &log, &[62.0, 311.0]);
            assert_eq!(times(&log, id(&h, "COLD ON")), [ticks(184.05)]);
            for (&hot, &cold) in h.logic.hot_consumers[&1]
                .iter()
                .zip(&h.logic.cold_consumers[&1])
            {
                let mut overlap = 0;
                for tick in ticks(0.5)..=ticks(400.0) {
                    let (hot, cold) = (active(&log, hot, tick), active(&log, cold, tick));
                    assert!(hot || cold, "both off at {tick}, {config:?}");
                    overlap = if hot && cold { overlap + 1 } else { 0 };
                    assert!(overlap <= 1, "overlap exceeds 0.06 s at {tick}, {config:?}");
                }
            }
        }
    }

    #[test]
    fn threshold_pass_and_timeout_in_one_tick_is_a_pass() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        for config in game_configs() {
            for same_tick_delivery in [SameTickDelivery::DepthFirst, SameTickDelivery::Batched] {
                let config = WalkerConfig {
                    same_tick_delivery,
                    ..config
                };
                // Entry at 1, check at 61, return exactly when CLOSE expires at
                // 63. The sample and timeout are both real walker events.
                let log = run(
                    &h,
                    config,
                    70.0,
                    vec![track(DPRK_PLAYER_COUNTRY, 1, 70, |s| {
                        if s == 1 || s >= 63 {
                            orbit(s)
                        } else {
                            outside()
                        }
                    })],
                );
                assert_eq!(times(&log, id(&h, "CLOSE")), [ticks(63.0)]);
                assert_passes(&h, &log, &[63.0]);
                for role in ["CLOSE DELAY", "COLD ON", "HOT OFF", "REARM ON"] {
                    assert!(times(&log, id(&h, role)).is_empty(), "{role}, {config:?}");
                }
                assert!(h.logic.hot_consumers[&1].iter().all(|&hot| active(
                    &log,
                    hot,
                    ticks(70.0)
                )));
            }
        }
    }

    #[test]
    fn threshold_timeout_then_pass_in_one_tick_is_one_outcome() {
        let mut h = default_harness(PresenceCheckImpl::TriggerReactivate);
        let mut next = nodes(&h.root)
            .into_iter()
            .filter_map(|node| node.index)
            .max()
            .unwrap()
            + 1;
        let mut late_pass = template::timer("Late pass", 1.95, &mut next, CENTRE.0, CENTRE.1);
        late_pass.set_targets(vec![id(&h, "SEEN")]);
        let late_pass_id = late_pass.index.unwrap();
        h.root.children.push(late_pass);
        let input = WalkerInput {
            // Include HOLD's expiry after a pass, before that recheck closes.
            until: ticks(183.0),
            tracks: vec![track(DPRK_PLAYER_COUNTRY, 1, 183, |s| {
                if s == 1 { orbit(s) } else { outside() }
            })],
            // DWELL schedules CLOSE at 61.0. Starting this timer afterwards
            // queues its 63.0 expiry behind CLOSE, in either delivery mode.
            pulses: vec![(ticks(61.05), late_pass_id.into())],
            ..WalkerInput::default()
        };
        for config in game_configs() {
            for same_tick_delivery in [SameTickDelivery::DepthFirst, SameTickDelivery::Batched] {
                let config = WalkerConfig {
                    same_tick_delivery,
                    ..config
                };
                let log = walk(&h.root, config, &input);
                assert_eq!(times(&log, id(&h, "CLOSE")), [ticks(63.0)], "{config:?}");
                assert_eq!(times(&log, late_pass_id), [ticks(63.0)], "{config:?}");
                let pulse_position = |target| {
                    log.pulses
                        .iter()
                        .position(|pulse| pulse.t == ticks(63.0) && pulse.index == target)
                        .expect("timer expiry appears at 63 s")
                };
                assert!(
                    pulse_position(id(&h, "CLOSE")) < pulse_position(late_pass_id),
                    "CLOSE must expire before the late pass at 63 s, {config:?}"
                );
                assert_eq!(times(&log, id(&h, "DWELL")), [ticks(61.0)], "{config:?}");
                assert_eq!(times(&log, id(&h, "TRIGGER")), [ticks(1.0)], "{config:?}");
                assert_eq!(
                    times(&log, id(&h, "HOT ON")).len() + times(&log, id(&h, "CLOSE DELAY")).len(),
                    1,
                    "exactly one outcome for the 63 s check, {config:?}"
                );
                match same_tick_delivery {
                    SameTickDelivery::DepthFirst => {
                        // CLOSE OFF has already closed SEEN when the late
                        // pulse reaches it, so only the timeout can finish.
                        assert_passes(&h, &log, &[]);
                        for role in ["CLOSE DELAY", "COLD ON", "REARM ON"] {
                            assert_eq!(
                                times(&log, id(&h, role)),
                                [ticks(63.05)],
                                "{role}, {config:?}"
                            );
                        }
                        for role in ["HOT OFF DELAY", "HOT OFF"] {
                            assert_eq!(
                                times(&log, id(&h, role)),
                                [ticks(63.1)],
                                "{role}, {config:?}"
                            );
                        }
                        assert!(times(&log, id(&h, "HOLD")).is_empty(), "{config:?}");
                        assert_cold(&h, &log);
                    }
                    SameTickDelivery::Batched => {
                        // SEEN still passes; PASS OFF cancels the running
                        // CLOSE DELAY when deactivations commit at tick end.
                        assert_passes(&h, &log, &[63.0]);
                        assert_eq!(times(&log, id(&h, "HOLD")), [ticks(183.0)], "{config:?}");
                        for role in [
                            "CLOSE DELAY",
                            "COLD ON",
                            "HOT OFF DELAY",
                            "HOT OFF",
                            "REARM ON",
                        ] {
                            assert!(times(&log, id(&h, role)).is_empty(), "{role}, {config:?}");
                        }
                    }
                }
                for (&hot, &cold) in h.logic.hot_consumers[&1]
                    .iter()
                    .zip(&h.logic.cold_consumers[&1])
                {
                    for tick in ticks(63.0)..=input.until {
                        assert!(
                            active(&log, hot, tick) || active(&log, cold, tick),
                            "consumer pair {hot}/{cold} both off at {tick}, {config:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn threshold_stays_hot_through_a_successful_recheck() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        for config in game_configs() {
            let log = run(
                &h,
                config,
                600.0,
                vec![track(DPRK_PLAYER_COUNTRY, 1, 600, orbit)],
            );
            assert_passes(&h, &log, &[62.0, 183.0, 304.0, 425.0, 546.0]);
            assert!(times(&log, id(&h, "COLD ON")).is_empty());
            for snapshot in log.snapshots.iter().filter(|s| s.t >= ticks(62.0)) {
                assert!(
                    h.logic.hot_consumers[&1]
                        .iter()
                        .all(|id| snapshot.active[id])
                );
            }
        }
    }

    #[test]
    fn threshold_pass_leave_pass_again() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        for config in game_configs() {
            let log = run(&h, config, 400.0, vec![leave_and_return()]);
            assert_passes(&h, &log, &[62.0, 311.0]);
            assert_eq!(times(&log, id(&h, "ARM")), [ticks(1.0), ticks(250.0)]);
            assert_eq!(times(&log, id(&h, "CLOSE DELAY")), [ticks(184.05)]);
            assert_eq!(times(&log, id(&h, "REARM ON")), [ticks(184.05)]);
            assert!(h.logic.hot_consumers[&1].iter().all(|&hot| !active(&log, hot, ticks(200.0)) && active(&log, hot, ticks(311.0))));
        }
    }

    #[test]
    fn threshold_second_plane_after_close_does_not_pass() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        for config in game_configs() {
            let log = run(
                &h,
                config,
                140.0,
                vec![
                    track(DPRK_PLAYER_COUNTRY, 1, 140, |s| {
                        if s == 1 { orbit(s) } else { outside() }
                    }),
                    track(DPRK_PLAYER_COUNTRY, 64, 140, orbit),
                ],
            );
            assert_eq!(times(&log, id(&h, "ARM")), [ticks(1.0), ticks(64.0)]);
            assert_eq!(times(&log, id(&h, "CLOSE DELAY")), [ticks(63.05)]);
            assert_passes(&h, &log, &[125.0]); // A new, full dwell is mandatory.
        }
    }

    #[test]
    fn threshold_two_planes_pass_once_per_check() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        // Latches use DepthFirst, as the existing shell/budget game scenarios
        // do. Batched deliberately defers switches and does not latch two
        // same-tick inputs (walker_same_tick_latch_and_target_order).
        for config in game_configs() {
            let log = run(
                &h,
                config,
                600.0,
                vec![
                    track(DPRK_PLAYER_COUNTRY, 1, 600, orbit),
                    track(DPRK_PLAYER_COUNTRY, 1, 600, |s| orbit(s + 10)),
                ],
            );
            assert_eq!(times(&log, id(&h, "ARM")), [ticks(1.0)]);
            assert_passes(&h, &log, &[62.0, 183.0, 304.0, 425.0, 546.0]);
            assert_eq!(
                times(&log, id(&h, "HOLD")),
                [182.0, 303.0, 424.0, 545.0].map(ticks)
            );
            assert_eq!(
                times(&log, id(&h, "CHECK ON")),
                [61.0, 182.0, 303.0, 424.0, 545.0].map(ticks)
            );
        }
    }

    #[test]
    fn threshold_batched_two_planes_exposes_latch_hypothesis() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        for reverse_targets in [false, true] {
            let config = WalkerConfig {
                same_tick_delivery: SameTickDelivery::Batched,
                reverse_targets,
                ..game_config()
            };
            let log = run(
                &h,
                config,
                70.0,
                vec![
                    track(DPRK_PLAYER_COUNTRY, 1, 70, orbit),
                    track(DPRK_PLAYER_COUNTRY, 1, 70, |s| orbit(s + 10)),
                ],
            );
            // The binding 18-MCU graph has same-tick latches, not counters.
            // Preserve the walker's Batched hypothesis rather than suppressing
            // per-aircraft Entered events to make it look like a single pass.
            assert_eq!(times(&log, id(&h, "ARM")), [ticks(1.0); 2]);
            assert_eq!(times(&log, id(&h, "SEEN")), [ticks(62.0); 2]);
            assert_eq!(times(&log, id(&h, "HOT ON")), [ticks(62.0); 2]);
        }
    }

    #[test]
    fn threshold_presence_before_init_off_exposes_startup_gap() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        let log = run(
            &h,
            game_config(),
            70.0,
            vec![track(DPRK_PLAYER_COUNTRY, 0, 70, orbit)],
        );
        // The required mission-load trigger is live before the shared 0.5 s
        // INIT OFF. A plane already inside reaches SEEN before it is disabled,
        // bypasses the dwell, then INIT OFF closes HOT after COLD was closed.
        // Keep this scope limitation visible without changing the prescribed
        // graph or relying on Enabled for a timer.
        assert_eq!(times(&log, id(&h, "SEEN")), [ticks(0.0), ticks(61.0)]);
        for (&hot, &cold) in h.logic.hot_consumers[&1]
            .iter()
            .zip(&h.logic.cold_consumers[&1])
        {
            assert!(!active(&log, hot, ticks(0.5)));
            assert!(!active(&log, cold, ticks(0.5)));
            assert!(active(&log, hot, ticks(61.0)));
        }
    }

    #[test]
    fn threshold_silent_trigger_never_passes() {
        let h = default_harness(PresenceCheckImpl::TriggerReactivate);
        for config in game_configs() {
            let config = WalkerConfig {
                trigger_activation: TriggerActivation::Silent,
                ..config
            };
            let log = run(
                &h,
                config,
                600.0,
                vec![track(DPRK_PLAYER_COUNTRY, 1, 600, orbit)],
            );
            assert_eq!(times(&log, id(&h, "ARM")), [ticks(1.0)]);
            assert_passes(&h, &log, &[]);
            assert_eq!(times(&log, id(&h, "CLOSE DELAY")), [ticks(63.05)]);
            assert_cold(&h, &log);
        }
    }

    #[test]
    fn threshold_zone_pulse_loiter_passes() {
        let h = default_harness(PresenceCheckImpl::ZonePulse);
        for config in game_configs() {
            for same_tick_delivery in [SameTickDelivery::DepthFirst, SameTickDelivery::Batched] {
                let config = WalkerConfig {
                    same_tick_delivery,
                    trigger_activation: TriggerActivation::Silent,
                    ..config
                };
                let log = run(
                    &h,
                    config,
                    400.0,
                    vec![track(DPRK_PLAYER_COUNTRY, 1, 400, orbit)],
                );
                assert_passes(&h, &log, &[61.1, 181.2, 301.3]);
                assert!(times(&log, id(&h, "CLOSE DELAY")).is_empty());
                assert_eq!(times(&log, id(&h, "TRIGGER")), [ticks(1.0)]);
            }
        }
    }

    #[test]
    fn threshold_zone_pulse_ai_passes_a_recheck() {
        for (side, player, ai) in [
            (AirSide::Nato, DPRK_PLAYER_COUNTRY, DPRK_AI_COUNTRY),
            (AirSide::Dprk, NATO_PLAYER_COUNTRY, NATO_AI_COUNTRY),
        ] {
            for presence in [
                PresenceCheckImpl::TriggerReactivate,
                PresenceCheckImpl::ZonePulse,
            ] {
                let h = harness(side, &[CENTRE], ThresholdSpec::default(), presence);
                let log = run(
                    &h,
                    game_config(),
                    400.0,
                    vec![
                        track(
                            player,
                            1,
                            400,
                            |s| if s < 100 { orbit(s) } else { outside() },
                        ),
                        track(ai, 1, 400, |s| orbit(s + 10)),
                    ],
                );
                if presence == PresenceCheckImpl::ZonePulse {
                    assert_passes(&h, &log, &[61.1, 181.2, 301.3]);
                    assert!(times(&log, id(&h, "COLD ON")).is_empty());
                    assert!(h.logic.hot_consumers[&1].iter().all(|&hot| active(
                        &log,
                        hot,
                        ticks(400.0)
                    )));
                } else {
                    assert_passes(&h, &log, &[62.0]);
                    assert_eq!(times(&log, id(&h, "COLD ON")), [ticks(184.05)]);
                    assert!(h.logic.hot_consumers[&1].iter().all(|&hot| !active(
                        &log,
                        hot,
                        ticks(400.0)
                    )));
                }
            }
        }
    }
}
