//! A player count per area (decision 0050, option D): a Complex Trigger on the
//! other side's player country steps a relay chain from 0 to 3, then busy.
//! It replaces "is an enemy inside?" check zones, which cannot tell AI from
//! players, for ending sorties.

use std::collections::BTreeMap;

use super::threshold::{self, OBJECT_ENTERED, OBJECT_FINISHED, OBJECT_LEFT};
use super::{AirSide, SideLogic};
use crate::ast::Il2Entity;
use crate::template;

/// Steps counted before an area turns busy (the user: "count to three then busy").
pub(crate) const COUNT_STEPS: usize = 3;
/// One tick between switching the old step off and the new step on. One event
/// reaches every step's relay in turn; switching the next step on at once would
/// let the same event step the count again.
const STEP_DELAY_S: f64 = 0.05;

/// Build one count per area: 25 MCUs, one of them a check zone. Returns the
/// nodes and each area's `EMPTY` relay, in area order.
///
/// - Entered (58) moves the count up one step; Left (60) and Finished (62) move
///   it down. Killed (70) is not used: a kill also raises Finished when the
///   wreck lands (flight 2P), and counting both would take two steps.
/// - Only the current step's `UP` and `DOWN` relays are on. A step change
///   switches the old step's pair off at once and the new one on a tick later
///   (`ON k DELAY`), so one event moves the count one step whatever order it
///   reaches the relays in. A second event in the same tick is lost.
/// - A fourth entry turns the area busy: every relay goes off and `CLEAR`, a
///   Closer 0 check zone on the other side's coalition, is switched on and
///   pulsed. It fires when nobody of that side is inside, AI included (ALL-out,
///   flight 2M), switches itself off and returns the count to 0.
/// - `EMPTY` fires on the step from 1 to 0 and when busy clears.
///
/// Mission makers keep player spawns outside every circle (decision 0050): a
/// player who appears inside a trigger is not reported as entering (flight A1).
pub(crate) fn build_side(
    side: AirSide,
    areas: &[(f64, f64)],
    radius_m: f64,
    logic: &mut SideLogic,
    next_id: &mut i32,
) -> Result<(Vec<Il2Entity>, Vec<i32>), String> {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return Err("Air Tasking player count radius must be finite and greater than zero".into());
    }
    for (index, &(x, z)) in areas.iter().enumerate() {
        if !x.is_finite() || !z.is_finite() {
            return Err(format!(
                "Air Tasking player count area {} coordinates must be finite",
                index + 1
            ));
        }
    }
    let mut trigger = threshold::player_trigger(side, radius_m);
    trigger.set_property("EventsFilterFinishedSimple", "1");
    let coalitions = threshold::enemy_coalitions(side);
    let top = COUNT_STEPS + 1; // UP {top} is the entry that turns the area busy.
    let mut result = Vec::new();
    let mut empties = Vec::new();
    for (index, &(x, z)) in areas.iter().enumerate() {
        let prefix = format!("PC{}{:02}", side.letter(), index + 1);
        let mut cell = Vec::new();
        let mut ids: BTreeMap<String, i32> = BTreeMap::new();
        let mut roles: Vec<(String, &str, f64)> = vec![("TRIGGER".into(), "MCU_TR_ComplexTrigger", 0.0)];
        roles.extend((1..=top).map(|k| (format!("UP {k}"), "MCU_Timer", 0.0)));
        roles.extend((1..=COUNT_STEPS).map(|k| (format!("DOWN {k}"), "MCU_Timer", 0.0)));
        roles.extend((0..=COUNT_STEPS).map(|k| (format!("ON {k}"), "MCU_Activate", 0.0)));
        roles.extend((0..=COUNT_STEPS).map(|k| (format!("ON {k} DELAY"), "MCU_Timer", STEP_DELAY_S)));
        roles.extend((0..=COUNT_STEPS).map(|k| (format!("OFF {k}"), "MCU_Deactivate", 0.0)));
        roles.extend([
            ("CLEAR".into(), "MCU_CheckZone", 0.0),
            ("CLEAR ON".into(), "MCU_Activate", 0.0),
            ("CLEAR PULSE".into(), "MCU_Timer", 0.1),
            ("CLEAR OFF".into(), "MCU_Deactivate", 0.0),
            ("EMPTY".into(), "MCU_Timer", 0.0),
        ]);
        for (role, kind, time) in &roles {
            let name = format!("{prefix} {role}");
            let node = match *kind {
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
                "MCU_Timer" => template::timer(&name, *time, next_id, x, z),
                "MCU_CheckZone" => template::checkzone(&name, radius_m, false, coalitions, next_id, x, z),
                _ => template::mcu(kind, &name, next_id, x, z),
            };
            ids.insert(role.clone(), node.index.unwrap());
            cell.push(node);
        }
        let id = |role: &str| ids[role];
        let step_set = |k: usize| {
            if k == 0 {
                vec![id("UP 1")]
            } else {
                vec![id(&format!("UP {}", k + 1)), id(&format!("DOWN {k}"))]
            }
        };
        let mut edges: BTreeMap<String, Vec<i32>> = BTreeMap::new();
        for k in 1..=COUNT_STEPS {
            edges.insert(format!("UP {k}"), vec![id(&format!("OFF {}", k - 1)), id(&format!("ON {k} DELAY"))]);
            let mut down = vec![id(&format!("OFF {k}")), id(&format!("ON {} DELAY", k - 1))];
            if k == 1 {
                down.push(id("EMPTY"));
            }
            edges.insert(format!("DOWN {k}"), down);
        }
        edges.insert(
            format!("UP {top}"),
            vec![id(&format!("OFF {COUNT_STEPS}")), id("CLEAR ON"), id("CLEAR PULSE")],
        );
        for k in 0..=COUNT_STEPS {
            edges.insert(format!("ON {k} DELAY"), vec![id(&format!("ON {k}"))]);
            edges.insert(format!("ON {k}"), step_set(k));
            edges.insert(format!("OFF {k}"), step_set(k));
        }
        edges.insert("CLEAR ON".into(), vec![id("CLEAR")]);
        edges.insert("CLEAR PULSE".into(), vec![id("CLEAR")]);
        edges.insert("CLEAR".into(), vec![id("CLEAR OFF"), id("ON 0 DELAY"), id("EMPTY")]);
        edges.insert("CLEAR OFF".into(), vec![id("CLEAR")]);
        edges.insert("EMPTY".into(), vec![]);
        for node in &mut cell {
            if node.index == Some(id("TRIGGER")) {
                for k in 1..=top {
                    template::attach_event(node, OBJECT_ENTERED, id(&format!("UP {k}")));
                }
                for event in [OBJECT_LEFT, OBJECT_FINISHED] {
                    for k in 1..=COUNT_STEPS {
                        template::attach_event(node, event, id(&format!("DOWN {k}")));
                    }
                }
            } else {
                let role = node.name().unwrap().strip_prefix(&prefix).unwrap().trim().to_string();
                node.set_targets(edges.remove(&role).expect("every count MCU has its edges"));
            }
        }
        // Only step 0's UP 1 starts on (AT s INIT OFF switches the rest off).
        logic.init_off.extend((2..=top).map(|k| id(&format!("UP {k}"))));
        logic.init_off.extend((1..=COUNT_STEPS).map(|k| id(&format!("DOWN {k}"))));
        empties.push(id("EMPTY"));
        result.extend(cell);
    }
    Ok((result, empties))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airtask::assemble::emit_side_logic;
    use crate::airtask::shell::test_support::{flown_configs, named, ticks, times};
    use crate::airtask::testkit::*;
    use crate::airtask::threshold::{DPRK_AI_COUNTRY, DPRK_PLAYER_COUNTRY};

    const CENTRE: (f64, f64) = (123_000.0, 234_000.0);
    const OTHER: (f64, f64) = (323_000.0, 234_000.0);
    const RADIUS_M: f64 = 6000.0;
    const DPRK_SIDE: i32 = 1;

    struct Harness {
        root: Il2Entity,
        logic: SideLogic,
        empties: Vec<i32>,
    }

    fn harness(areas: &[(f64, f64)]) -> Harness {
        let mut next = 1;
        let mut logic = SideLogic::default();
        let (nodes, empties) =
            build_side(AirSide::Nato, areas, RADIUS_M, &mut logic, &mut next).unwrap();
        let mut root = Il2Entity::new("Group");
        root.children = nodes;
        let init = emit_side_logic(
            AirSide::Nato,
            &logic,
            std::slice::from_mut(&mut root),
            &mut next,
        )
        .unwrap();
        root.children.extend(init);
        Harness {
            root,
            logic,
            empties,
        }
    }

    fn node<'a>(h: &'a Harness, area: usize, role: &str) -> &'a Il2Entity {
        named(&h.root, &format!("PCN{area:02} {role}"))
    }

    fn id(h: &Harness, area: usize, role: &str) -> i32 {
        node(h, area, role).index.unwrap()
    }

    /// A plane of `country` inside `centre`'s circle during `inside` (seconds)
    /// and 100 km away otherwise, sampled every second from 1 s to `until`.
    fn plane(
        country: i32,
        centre: (f64, f64),
        inside: std::ops::Range<u32>,
        until: u32,
    ) -> ScriptedTrack {
        ScriptedTrack {
            coalition: DPRK_SIDE,
            country,
            samples: (1..=until)
                .map(|second| {
                    let away = if inside.contains(&second) {
                        0.0
                    } else {
                        100_000.0
                    };
                    (ticks(f64::from(second)), centre.0 + away, centre.1)
                })
                .collect(),
        }
    }

    fn player(inside: std::ops::Range<u32>, until: u32) -> ScriptedTrack {
        plane(DPRK_PLAYER_COUNTRY, CENTRE, inside, until)
    }

    fn run(h: &Harness, config: WalkerConfig, until: u32, input: WalkerInput) -> WalkerLog {
        walk(
            &h.root,
            config,
            &WalkerInput {
                until: ticks(f64::from(until)),
                ..input
            },
        )
    }

    fn empties(h: &Harness, log: &WalkerLog, area: usize) -> Vec<Tick> {
        times(log, h.empties[area - 1])
    }

    /// The flown settings, each with both same-tick delivery models: one event
    /// reaches the step relays in either order.
    fn configs() -> Vec<WalkerConfig> {
        flown_configs()
            .into_iter()
            .flat_map(|config| {
                [SameTickDelivery::DepthFirst, SameTickDelivery::Batched].map(|same_tick_delivery| {
                    WalkerConfig { same_tick_delivery, ..config }
                })
            })
            .collect()
    }

    fn tracks(tracks: Vec<ScriptedTrack>) -> WalkerInput {
        WalkerInput {
            tracks,
            ..WalkerInput::default()
        }
    }

    #[test]
    fn count_structure() {
        let h = harness(&[CENTRE, OTHER]);
        assert_eq!(h.empties.len(), 2);
        for (area, centre) in [(1, CENTRE), (2, OTHER)] {
            let ids = |roles: &[&str]| {
                roles
                    .iter()
                    .map(|role| id(&h, area, role))
                    .collect::<Vec<_>>()
            };
            let expected: Vec<(&str, &str, Vec<i32>)> = vec![
                ("UP 1", "MCU_Timer", ids(&["OFF 0", "ON 1 DELAY"])),
                ("UP 2", "MCU_Timer", ids(&["OFF 1", "ON 2 DELAY"])),
                ("UP 3", "MCU_Timer", ids(&["OFF 2", "ON 3 DELAY"])),
                (
                    "UP 4",
                    "MCU_Timer",
                    ids(&["OFF 3", "CLEAR ON", "CLEAR PULSE"]),
                ),
                ("DOWN 1", "MCU_Timer", ids(&["OFF 1", "ON 0 DELAY", "EMPTY"])),
                ("DOWN 2", "MCU_Timer", ids(&["OFF 2", "ON 1 DELAY"])),
                ("DOWN 3", "MCU_Timer", ids(&["OFF 3", "ON 2 DELAY"])),
                ("ON 0 DELAY", "MCU_Timer", ids(&["ON 0"])),
                ("ON 1 DELAY", "MCU_Timer", ids(&["ON 1"])),
                ("ON 2 DELAY", "MCU_Timer", ids(&["ON 2"])),
                ("ON 3 DELAY", "MCU_Timer", ids(&["ON 3"])),
                ("ON 0", "MCU_Activate", ids(&["UP 1"])),
                ("OFF 0", "MCU_Deactivate", ids(&["UP 1"])),
                ("ON 1", "MCU_Activate", ids(&["UP 2", "DOWN 1"])),
                ("OFF 1", "MCU_Deactivate", ids(&["UP 2", "DOWN 1"])),
                ("ON 2", "MCU_Activate", ids(&["UP 3", "DOWN 2"])),
                ("OFF 2", "MCU_Deactivate", ids(&["UP 3", "DOWN 2"])),
                ("ON 3", "MCU_Activate", ids(&["UP 4", "DOWN 3"])),
                ("OFF 3", "MCU_Deactivate", ids(&["UP 4", "DOWN 3"])),
                ("CLEAR ON", "MCU_Activate", ids(&["CLEAR"])),
                ("CLEAR PULSE", "MCU_Timer", ids(&["CLEAR"])),
                (
                    "CLEAR",
                    "MCU_CheckZone",
                    ids(&["CLEAR OFF", "ON 0 DELAY", "EMPTY"]),
                ),
                ("CLEAR OFF", "MCU_Deactivate", ids(&["CLEAR"])),
                ("EMPTY", "MCU_Timer", vec![]),
            ];
            for (role, kind, targets) in &expected {
                let n = node(&h, area, role);
                assert_eq!(
                    (&n.block_type, &n.targets),
                    (&kind.to_string(), targets),
                    "{area} {role}"
                );
                assert_eq!(n.pos_xz(), Some(centre), "{area} {role}");
            }
            assert_eq!(h.empties[area - 1], id(&h, area, "EMPTY"));
            for k in 0..=COUNT_STEPS {
                assert_eq!(
                    crate::airtask::library::number(node(&h, area, &format!("ON {k} DELAY")), "Time"),
                    Ok(STEP_DELAY_S)
                );
            }
            assert_eq!(
                crate::airtask::library::number(node(&h, area, "CLEAR PULSE"), "Time"),
                Ok(0.1)
            );
            let clear = node(&h, area, "CLEAR");
            assert_eq!(
                (
                    clear.property("Closer"),
                    clear.property("Zone"),
                    clear.property("PlaneCoalitions")
                ),
                (Some("0"), Some("6000"), Some("[1]")),
                "{area}: everyone of the DPRK side, AI included"
            );
            // The trigger: DPRK players, 6 km, Entered up and Left or Finished down.
            let trigger = node(&h, area, "TRIGGER");
            assert_eq!(trigger.block_type, "MCU_TR_ComplexTrigger");
            assert_eq!(
                (
                    trigger.property("Country"),
                    trigger.property("Radius"),
                    trigger.pos_xz()
                ),
                (Some("501"), Some("6000"), Some(centre))
            );
            for (key, value) in [
                ("EventsFilterEnteredSimple", "1"),
                ("EventsFilterLeftSimple", "1"),
                ("EventsFilterFinishedSimple", "1"),
                ("EventsFilterKilled", "0"),
            ] {
                assert_eq!(trigger.property(key), Some(value), "{area} {key}");
            }
            let mut hooks: Vec<(i32, i32)> = Vec::new();
            trigger.for_each(&mut |event| {
                if event.block_type == "OnEvent" {
                    hooks.push(crate::template::read_event(event).unwrap());
                }
            });
            let mut expected_hooks: Vec<(i32, i32)> = Vec::new();
            for role in ["UP 1", "UP 2", "UP 3", "UP 4"] {
                expected_hooks.push((58, id(&h, area, role)));
            }
            for event in [60, 62] {
                for role in ["DOWN 1", "DOWN 2", "DOWN 3"] {
                    expected_hooks.push((event, id(&h, area, role)));
                }
            }
            assert_eq!(hooks, expected_hooks, "{area}");
            // Every relay but UP 1 starts off.
            for role in ["UP 2", "UP 3", "UP 4", "DOWN 1", "DOWN 2", "DOWN 3"] {
                assert!(
                    h.logic.init_off.contains(&id(&h, area, role)),
                    "{area} {role} starts off"
                );
            }
            for role in ["UP 1", "CLEAR", "EMPTY"] {
                assert!(
                    !h.logic.init_off.contains(&id(&h, area, role)),
                    "{area} {role} starts on"
                );
            }
            let prefix = format!("PCN{area:02} ");
            let count = h
                .root
                .children
                .iter()
                .filter(|n| n.name().is_some_and(|name| name.starts_with(&prefix)))
                .count();
            assert_eq!(count, 25, "{area}: 25 MCUs, one of them a check zone");
        }
        assert_eq!(h.logic.init_off.len(), 12);
    }

    #[test]
    fn rejects_bad_inputs_before_allocating() {
        for (areas, radius, message) in [
            (
                vec![CENTRE],
                0.0,
                "Air Tasking player count radius must be finite and greater than zero",
            ),
            (
                vec![CENTRE],
                f64::NAN,
                "Air Tasking player count radius must be finite and greater than zero",
            ),
            (
                vec![CENTRE, (f64::NAN, 0.0)],
                RADIUS_M,
                "Air Tasking player count area 2 coordinates must be finite",
            ),
        ] {
            let mut logic = SideLogic::default();
            let mut next = 7;
            let error =
                build_side(AirSide::Nato, &areas, radius, &mut logic, &mut next).unwrap_err();
            assert_eq!(error, message);
            assert_eq!((next, logic.init_off.len()), (7, 0));
        }
    }

    #[test]
    fn one_player_in_and_out_empties_once() {
        let h = harness(&[CENTRE]);
        for config in configs() {
            let log = run(&h, config, 100, tracks(vec![player(10..50, 100)]));
            assert_eq!(empties(&h, &log, 1), [ticks(50.0)], "{config:?}");
        }
    }

    #[test]
    fn the_area_empties_when_the_last_player_leaves() {
        let h = harness(&[CENTRE]);
        for config in configs() {
            let log = run(
                &h,
                config,
                120,
                tracks(vec![player(10..50, 120), player(20..80, 120)]),
            );
            assert_eq!(empties(&h, &log, 1), [ticks(80.0)], "{config:?}");
        }
    }

    /// A Delete raises Finished at once; a kill raises it when the wreck lands.
    #[test]
    fn a_delete_and_a_kill_count_down() {
        let h = harness(&[CENTRE]);
        for config in configs() {
            let input = WalkerInput {
                track_ends: vec![
                    (ticks(40.0), 0, TrackEnd::Deleted),
                    (ticks(60.0), 1, TrackEnd::Killed),
                ],
                ..tracks(vec![player(10..200, 200), player(20..200, 200)])
            };
            let log = run(&h, config, 200, input);
            assert_eq!(
                empties(&h, &log, 1),
                [ticks(60.0) + WRECK_FINISHED_TICKS],
                "{config:?}"
            );
        }
    }

    #[test]
    fn ai_of_the_same_side_does_not_count() {
        let h = harness(&[CENTRE]);
        for config in configs() {
            let ai = plane(DPRK_AI_COUNTRY, CENTRE, 10..50, 100);
            let log = run(&h, config, 100, tracks(vec![ai]));
            assert!(times(&log, id(&h, 1, "UP 1")).is_empty(), "{config:?}");
            assert!(empties(&h, &log, 1).is_empty(), "{config:?}");
        }
    }

    #[test]
    fn each_area_counts_its_own_players() {
        let h = harness(&[CENTRE, OTHER]);
        for config in configs() {
            let elsewhere = plane(DPRK_PLAYER_COUNTRY, OTHER, 10..50, 100);
            let log = run(
                &h,
                config,
                100,
                tracks(vec![player(10..30, 100), elsewhere]),
            );
            assert_eq!(empties(&h, &log, 1), [ticks(30.0)], "{config:?}");
            assert_eq!(empties(&h, &log, 2), [ticks(50.0)], "{config:?}");
        }
    }

    /// Three steps, then busy: leaves no longer count, and only an empty
    /// circle (AI included) clears it. Counting then starts again from 0.
    #[test]
    fn busy_after_three_clears_only_when_nobody_is_left() {
        let h = harness(&[CENTRE]);
        let four = || {
            (0..4)
                .map(|i| player(10 + 10 * i..100 + 10 * i, 300))
                .collect::<Vec<_>>()
        };
        for config in configs() {
            let mut players = four();
            players.push(player(150..170, 300));
            let log = run(&h, config, 300, tracks(players));
            assert_eq!(
                times(&log, id(&h, 1, "UP 4")),
                [ticks(40.0)],
                "{config:?}: busy at the fourth entry"
            );
            // The fourth leaves at 130; the fifth comes and goes after busy cleared.
            assert_eq!(
                empties(&h, &log, 1),
                [ticks(130.0), ticks(170.0)],
                "{config:?}"
            );

            // Same players, with DPRK AI inside until 200: busy holds until it leaves.
            let mut with_ai = four();
            with_ai.push(plane(DPRK_AI_COUNTRY, CENTRE, 5..200, 300));
            let log = run(&h, config, 300, tracks(with_ai));
            assert_eq!(empties(&h, &log, 1), [ticks(200.0)], "{config:?}");
        }
    }

    /// Two leaves in one tick, and two entries in one tick. The next step comes
    /// on a tick later, so in either walker model the second event finds no
    /// relay on and is lost. Two leaves leave the count too high (the area never
    /// empties: safe); two entries leave it too low (it empties at the first
    /// leave while one player is still inside). Flight 2R (cell R3) flies the
    /// leaves; same-tick entries need two planes within about a metre of each
    /// other along the track at the boundary.
    #[test]
    fn two_events_in_one_tick() {
        let h = harness(&[CENTRE]);
        for (same_tick_delivery, leaves, entries) in [
            (SameTickDelivery::DepthFirst, vec![], vec![ticks(80.0)]),
            (SameTickDelivery::Batched, vec![], vec![ticks(80.0)]),
        ] {
            let config = WalkerConfig {
                same_tick_delivery,
                ..WalkerConfig::default()
            }
            .flown();
            let input = WalkerInput {
                track_ends: vec![
                    (ticks(60.0), 0, TrackEnd::Deleted),
                    (ticks(60.0), 1, TrackEnd::Deleted),
                ],
                ..tracks(vec![player(10..200, 200), player(20..200, 200)])
            };
            let log = run(&h, config, 200, input);
            assert_eq!(
                empties(&h, &log, 1),
                leaves,
                "{config:?}: two leaves in one tick"
            );
            let log = run(
                &h,
                config,
                200,
                tracks(vec![player(10..80, 200), player(10..90, 200)]),
            );
            assert_eq!(
                empties(&h, &log, 1),
                entries,
                "{config:?}: two entries in one tick"
            );
        }
    }
}
