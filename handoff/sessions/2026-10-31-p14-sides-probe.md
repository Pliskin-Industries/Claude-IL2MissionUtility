# Sides probe: can Air Tasking's AI sit on sides of its own? (decision 0050)

**Stage 1 flown 2026-10-31** (logs `real_56`): the game accepts countries on sides 3 and 4. Results: `handoff/P14-probe-results.md`, "Sides check S1". **Stage 2 (flight 2Q) is not built yet.**

## Why

Check zones filter by side only, so AI holds a sortie's Zone IN / Zone Out and the area presence check. With Air Tasking's AI countries (602 NATO, 503 DPRK) on sides 3 and 4, a zone watching side 1 or 2 would see players only, and a front-wide Zone Out becomes useful (the user's direction). Decision 0050 has the full context.

## Stage 1 (done)

- `tools/p14/build_probe_mission.py --coalition COUNTRY:SIDE` (repeatable) rewrites a row of the `Countries` table. It stops on a country not in the table, a malformed value, or a country named twice. Without the option the output is unchanged; with it, only the named rows change (checked by diff).
- Built from a scratch copy of flight 2P's frozen group, with plane A on 602 and B on 503. The frozen files were not touched.

## Stage 2: flight 2Q, no pilot

Cells are at least 40 km apart so that no hunter sees another cell. Pairs come from flight 2M's machinery (`add_threshold_probe_side` pairs and the 2 km orbit). The country is set per pair, and the side table comes from `--coalition 602:3 --coalition 503:4`.

- **Q1, detection.** Three plain check zones at one centre, Closer 1, 6 km, each pulsed every 30 s through its own relay, traced: `Q1-Z2` watches `[2]`, `Q1-Z3` watches `[3]`, `Q1-Z23` watches `[2,3]`. A 602 pair orbits from about 60 s to 240 s, then is deleted. A 601 pair orbits from about 270 s.
  - **Passes if:** `Q1-Z2` fires only once the 601 pair is in, `Q1-Z3` fires for the 602 pair, and `Q1-Z23` fires for both.
- **Q2 to Q7, fighting.** In each cell the target pair orbits the centre at 3,050 m from about 60 s. The hunter pair arrives about 120 s, with an `MCU_CMD_AttackArea` (`AttackAir 1`, `AttackGround 0`, about 5 km, `Time` 300 s) on the centre.

  | Cell | Hunter | Target | Must |
  |---|---|---|---|
  | Q2 | 602 (side 3) | 501 (DPRK) | attack |
  | Q3 | 602 (side 3) | 601 (NATO) | not attack |
  | Q4 | 501 (DPRK) | 602 (side 3) | attack |
  | Q5 | 601 (NATO) | 602 (side 3) | not attack |
  | Q6 | 503 (side 4) | 602 (side 3) | attack |
  | Q7 | 503 (side 4) | 501 (DPRK) | not attack |

  - **Evidence:** hit (`AType:1`) and kill (`AType:3`) lines naming the attacker, plus Tacview.
  - **Reading:** a "not attack" cell fails on any hit. An "attack" cell passes on a hit by a hunter within the order's time; no hit means the side is not seen as an enemy, or the order did not engage (cell Q4 is the control for that: DPRK against side 3).
  - **Note:** cell Q3's NATO pair are AI stand-ins for NATO players. Whether a human sees side-3 AI as a friend (icons, penalties) needs a pilot later.
- Safety close at 540 s deletes every aircraft. Leave the server at 09:30 (570 s).
- Aircraft type does not set the side (a Yak on 601 flew as NATO in 2J and 2M). Use the machinery's Yak-9P (armed with guns) for every pair.

## To build (TDD, as for 2M and 2P)

1. `Flight::TwoQ`, a run sheet constant, and tests first:
   - `flight_2q_cells_match_the_run_sheet`;
   - `flight_2q_predictions` (walker: the Q1 zones by side, with the walker taught the side table);
   - an ignored writer `write_p14_probe_2q`.
2. Build with `--coalition 602:3 --coalition 503:4`. Fly, then record the results here, in the probe results, in the facts file and in 0050.
