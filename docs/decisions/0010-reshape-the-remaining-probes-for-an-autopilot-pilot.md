# 0010. Reshape the remaining probes for an autopilot pilot

- **Status:** Accepted
- **Date:** 2026-10-12
- **Decided by:** the user

## Context
Claude now flies the probes on the second PC (`handoff/sessions/2026-10-12-il2-korea-on-this-pc.md`). It can hold a level leg, turn and orbit by keyboard. It cannot take off, land or shoot. Three things are left from step 0: risk n (F-80C cruise speed), the country-filter cells Y1 to Y3 (decision 0008, option 4) and flight 1B's cell T-g (kill events, risk g). As written, T-n needs a 2 km zone found by hand, the Y cells need a second NATO country, and T-g needs the player to shoot down two aircraft.

## Decision
Three short flights replace "the rest of 1A" and flight 1B. Each piloted flight is one straight line from an air start.

| Flight | Side, pilot | What it holds |
|---|---|---|
| **1N** | NATO, Claude flies north | Air start 40 km south of T-n START, on the line, at 3050 m. START and END widened to 5 km (on a straight track the radius cancels). 20 km past END: cell Y-N, a Complex Trigger (10 km, planes, country 601) with a plain NATO check zone as control. A NATO AI pair written as country 602 crosses Y-N before the player arrives. |
| **1D** | DPRK, Claude flies | Air start as USSR (501) in a MiG-15, 20 km from cell Y-D: a Complex Trigger (country 501) with a plain DPRK check zone as control. A DPRK-country (503) AI pair crosses first. |
| **1G** | no pilot | T-g with the kills made by the game: G1 is attacked by an ace AI MiG-15 flight; a new flight G4 is destroyed by a `Command:Damage` MCU. G2 (Delete) and G3 (Deactivate, then Delete) stay as planned. |

Supporting changes:
- **Spawn points come from the generator.** Each probe writes a small spawn list next to its `.Group`; `tools/p14/build_probe_mission.py` turns each entry into an air-start spawn airfield copied from the game's own dogfight test mission. This replaces the hand edit used on 2026-10-12.
- **The Complex Trigger block** is written to the shape in `TemplateExamples/Reference/ComplexTrigger_reference.Group`, and the shape guard test covers it.
- **The plan's step 0 tables** change with the code, because the tests check the code against them.
- **Cruise setting for T-n:** Claude reads the F-80C cruise setting from the game's handbook and flies it; the results record the throttle figure used.

## Chosen by the user (2026-10-12)
1. The T-g kills are made by AI and by a Damage command. Flight 1B as written is retired.
2. The country filter is tested on both sides.
3. Astra builds it, under decision 0005.

The exact cells, positions and times are in `handoff/P14-air-tasking.md`, step 0e. The flights are named 1N, 1D and 1G, so that "Probe 2" keeps meaning step 3's budget probe.

## Rejected
- **Keep T-g for the user to fly and shoot.** It would wait on the user's other machine.
- **Test the country filter on the DPRK side only.** The NATO side is the one in doubt, so it is the one worth a flight.

## Risks the build checks first
- A plane written as country 602 may not load. The builder's resaver run shows this at once. If it fails, Y-N is dropped and decision 0008 can only use the country filter for zones that watch the DPRK side.
- An air start may ignore the spawn airfield's heading. Seen so far: K14, heading 0, spawned heading north. If it does, Claude turns onto the line by the map.
- AI MiGs may not shoot down both G1 planes in time. G4 gives a kill either way.

## Consequences
- Verification: `cargo test --offline`; the builder's resaver run for each mission; then the flights, read with `il2_mission_utility replay`.
