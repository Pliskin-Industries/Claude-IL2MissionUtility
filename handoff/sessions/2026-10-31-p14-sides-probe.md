# Sides probe: can Air Tasking's AI sit on sides of its own? (decision 0050)

**Stage 1 flown 2026-10-31** (logs `real_56`): the game accepts countries on sides 3 and 4. Results: `handoff/P14-probe-results.md`, "Sides check S1". **Flight 2Q flown 2026-10-31** (logs `real_57`): AI on sides 3 and 4 fights both real sides, and check zones do not see side 3 at all. Option A of decision 0050 fails. Results: `handoff/P14-probe-results.md`, "Flight 2Q".

## Why

Check zones filter by side only, so AI holds a sortie's Zone IN / Zone Out and the area presence check. With Air Tasking's AI countries (602 NATO, 503 DPRK) on sides 3 and 4, a zone watching side 1 or 2 would see players only, and a front-wide Zone Out becomes useful (the user's direction). Decision 0050 has the full context.

## Stage 1 (done)

- `tools/p14/build_probe_mission.py --coalition COUNTRY:SIDE` (repeatable) rewrites a row of the `Countries` table. It stops on a country not in the table, a malformed value, or a country named twice. Without the option the output is unchanged; with it, only the named rows change (checked by diff).
- Built from a scratch copy of flight 2P's frozen group, with plane A on 602 and B on 503. The frozen files were not touched.

## Stage 2: flight 2Q, no pilot (built)

Seven cells, 45 km or more apart, 28 Yak-9P (armed with guns) in 14 pairs. Each pair is flight 2M's approach-route pair (20 km in from the west, then a 2 km orbit at 500 km/h) with its own country. Every plane has `Engageable = 1` (2M's stand-ins have 0). Code: `build_sides_probe` in `src/airtask/probe.rs`.

**Build the mission with the side table moved:**

```
python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe handoff/p14-probes/flight2q/P14_Probe_2Q_traced.Group --name P14_Probe_2Q_20261031 --title "P14 probe 2Q (AI on sides 3 and 4, no pilot)" --out <folder> --no-resave --coalition 602:3 --coalition 503:4
```

Check that the log header reads `CNTRS:…503:4,…602:3…`. Without the table, every answer below is void.

- **Q1, detection (60, -90) km.**
  - The 602 pair activates at 60 s and orbits at 3,050 m. It enters 6 km at 160.8 s and is deleted at 300 s.
  - The 601 pair activates at 300 s and orbits at 3,550 m. It enters at 400.8 s.
  - Three Closer 1 check zones at the centre, 6 km, each armed by one pulse at 60 s while nobody is inside: `Q1-Z2` watches `[2]`, `Q1-Z3` watches `[3]`, `Q1-Z23` watches `[2,3]`. An armed zone fires when a plane of a side it watches comes in, and stays armed (flight 2K).
  - **Predicted, if the idea works:** `Q1-Z2` at 401 only; `Q1-Z3` at 161; `Q1-Z23` at 161 and, by the walker's unflown re-entry rule, 401.
  - **If `Q1-Z2` fires at about 161:** the side-2 zone still sees the 602 pair, and decision 0050 fails.
  - **If `Q1-Z3` never fires:** suspect the zone's `[3]` filter (the editor cannot set side 3) before the idea.
  - The test walker gets each track's side from `TWO_Q_SIDES` in `probe.rs`, which must match the build's `--coalition` arguments.
- **Q2 to Q7, fighting.** The target pair activates at 60 s and orbits at 3,050 m. The hunter pair activates at 120 s and reaches its orbit, 500 m above the target, at about 250 s. At 270 s the hunter's lead gets `<cell> hunter HUNT`: the Yak source's `MCU_CMD_AttackArea` on the cell centre, `AttackAir 1`, `AttackGround 0`, `AttackGTargets 0`, 5,000 m, `Time` 260 s (to 530 s), `Priority` 1. Until then both pairs fly Priority 2 waypoints, which should keep them from engaging; the targets keep them throughout, so each cell tests one direction only.

  | Cell | Centre km | Hunter | Target | Must |
  |---|---|---|---|---|
  | Q2 | (140, -90) | 602 (side 3) | 501 (DPRK) | attack |
  | Q3 | (230, -90) | 602 (side 3) | 601 (NATO) | not attack |
  | Q4 | (140, -45) | 501 (DPRK) | 602 (side 3) | attack |
  | Q5 | (230, -45) | 601 (NATO) | 602 (side 3) | not attack |
  | Q6 | (140, 0) | 503 (side 4) | 602 (side 3) | attack |
  | Q7 | (230, 0) | 503 (side 4) | 501 (DPRK) | not attack |

  - **Evidence:** hit (`AType:1`) and kill (`AType:3`) lines naming the attacker, plus Tacview.
  - **Reading:** a "not attack" cell fails on any hit by its hunter. An "attack" cell passes on a hit by its hunter after 270 s. Read every hit by its attacker: cells are 45 km apart, and some cross-cell pairs are enemies (for example Q7's 503 hunter and Q5's 602 target), though both fly Priority 2 waypoints.
  - **If no attack cell hits:** the order did not engage, and the flight says nothing about sides. Q4 (DPRK against side 3) is the control for this. An air `AttackArea` has not been flown before: over an empty area a flight circled about 5.7 km out (flight 2C), wider than the 5 km area.
  - **Note:** Q3's and Q5's NATO pairs are AI stand-ins for NATO players. What a human sees (icons, penalties) needs a pilot later.
- **Every cell's safety close at 540 s** deletes its aircraft. Leave the server at 09:30 (570 s).

## After the flight (2026-10-31)

Recorded in the probe results, the facts file and 0050. Still open:
- **Same-side control:** if the user wants option A checked further, add a cell where hunter and target are on one side (601 hunting 603).
- **The user's choice in 0050:** B (a wider zone that still counts AI) or C (no change).

Freeze 2Q only if a later change alters its output.
