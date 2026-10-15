# Session 2026-10-15: HVAR sorties lose aircraft over an empty target — options

Status: **proposal, nothing built.** The user rules on the next step (a probe first, then a fix). Branch `claude/p14-hvar-empty-target`, cut from `codex/p14-3b` (`756dac2`).

The problem: in flight 2B (three no-pilot runs, logs `src/testdata/missionlog/real_20`, `real_21`, `real_22`; results `handoff/P14-probe-results.md`, "Flight 2B" and runs 2 and 3), HVAR F-80C 4-ships held on an AttackArea over an empty target lost aircraft with no attacker. AirAlert never did. Decision 0018 keeps sorties on station at empty targets, so this case is now common.

## 1. What the two templates order

`TemplateExamples/Historical1950/`. The two files are the same except for the payload and the AttackArea blocks (a `diff` shows nothing else).

| | HVAR Strike | AirAlert |
|---|---|---|
| Payload | 4: 8 × HVAR + 2 × 165-gal tanks | 2: guns + 2 × 165-gal tanks |
| WP 1 | 3,050 m, 660 km/h, Area 300; targets timer 37 | the same; targets timers 37 **and** 39 |
| Orders after WP 1 | timer 37 (0.5 s) → AttackArea 38: `AttackGTargets = 1`, `AttackArea = 2000`, `Time = 600` | timer 37 (0.5 s) → AttackArea 38: `AttackAir = 1`, `AttackArea = 5000`, `Time = 1080`; timer 39 (0.5 s) → AttackArea 40: `AttackGTargets = 1`, `AttackArea = 1500`, `Time = 120` |
| On-station time (`T_attack` = the largest `Time`, `library.rs` ≈784) | 600 s | 1,080 s |
| Formation | 20, density 0, the same in both | |
| `AILevel` | 2 (Average) in both | |

So both flights' **last** order is a ground-target AttackArea. AirAlert's ground order is given 0.5 s after its air order and lasts 120 s; HVAR's lasts 600 s. Whether the later order replaces the earlier one is still risk m in the plan (UNVERIFIED). AirAlert is therefore not a clean control: it spends 120 s on a ground order, HVAR 600 s.

## 2. What the AttackArea fields control (what the repo knows)

- The fields are `AttackGround`, `AttackAir`, `AttackGTargets` (one of three targets: "Attack Ground", "Attack Air Targets", "Attack Ground Targets", `template.rs` `AttackAreaTarget::label`), `AttackArea` (radius, m), `Time` (s) and `Priority`.
- **An AttackArea has no altitude field.** The only altitude in the order chain is the waypoint's `YPos` (3,050 m here). Nothing in the repo (template.rs, weapon_range.rs, `docs/historical-reference/`, the plan, TemplateExamples) says what height the AI flies while it works an AttackArea. That comes only from flights.
- Other built-in ground-attack templates hold long ground orders too: CloseSupport 900 s, ArmedRecon 1,200 s (low pair at 900 m), Il-10 600 s, Yak 300 s (README table). Only HVAR has been flown on station.

## 3. What the logs show

Every loss, from the `AType:3` records (no attacker, `AID:-1`). Targets: T-s1 at X 245,934 / Z 179,477, T-s4 at 335,934 / 269,477 (`handoff/p14-probes/flight2b/P14_Probe_2B_traced.Group`). "Ground there" is the height of the nearest `AType:31` (meets the ground) record.

| Run | Cell, aircraft | Destroyed | After WP 1 | Height | Ground there | From target | Reading |
|---|---|---|---|---|---|---|---|
| 1 (`real_20`) | T-s1 Red 12 | 09:19.0 | 212 s | 247 m | ~69 m (impact 09:25.1) | 1.2 km | destroyed in the air; pilot bailed out 1.9 s later |
| 1 | T-s1 Red 13 | 09:55.7 | 249 s | 29 m | same tick | 5.4 km | flew into the ground; pilot killed |
| 2 (`real_21`) | T-s1 Red 11 | 08:00.0 | 133 s | 652 m | ~91 m (impact 08:12.1) | 10.9 km | destroyed in the air; pilot killed in the same tick |
| 2 | T-s1 Red 12 | 08:11.8 | 145 s | 181 m | ~179 m, same place | 10.7 km | flew into the ground |
| 2 | T-s4 Red 11 + Red 12 | 09:15.1 | 120 s | 1,382 m | ~982 m (impact 09:26.3) | 12.2 km | both in one tick at one point: collision |
| 3 (`real_22`) | T-s1 Red 12 | 07:27.9 | 101 s | 1,415 m | ~108 m (impact 07:46.9) | 12.4 km | destroyed in the air; pilot bailed out 7.5 s later |

- **Count:** 9 HVAR 4-ship sorties flew in the three runs (T-s1, T-s3, T-s4 each run). 4 lost aircraft (7 aircraft). T-s1, empty for its whole hold, lost aircraft in **every** run. T-s3 (enemy pair already over the target) never lost one; T-s4 once, before its enemy appeared. AirAlert (T-s2): 0 of 3.
- **All 7 losses came 101–249 s after WP 1**, inside the 600 s ground order. In runs 1 and 2 T-s1's survivors stayed until 25:47 (on station plus the extension, ~16 min more) and lost nothing after 249 s.
- **No `AType:2` (damage), no `AType:1` (shot) and no `AType:26` (stores dropped)** in any of the three logs. The run 3 end-of-mission list shows T-s1's three survivors with `RCT:8` and `BUL:1800`: **they never fired a rocket or a round.** They searched; they did not attack.
- **Heights on station** (run 3 end list, 15:47.6, 1 s after T-s1's 600 s order expired): T-s1 at 366–408 m, 3.4 km from the target; T-s2 (AirAlert) at 1,138–1,164 m, 2.9 km from its target. Near T-s1 the ground impacts are at 69–179 m, so the HVARs were roughly 200–330 m above the ground (inference: ground height from the impact records, not a terrain lookup).
- **Most losses were 10–12 km from the target,** well outside the 2 km AttackArea radius. The flight ranges far while it searches.

### Hypotheses (none proven)

- **H1, the order.** A ground-target AttackArea with nothing to find keeps the flight low and manoeuvring for its full `Time`. Fits: the low heights, the timing inside the order, no losses after 249 s or under AirAlert's 120 s order.
- **H2, the weight.** Three aircraft were destroyed **in the air with no damage record** (247, 652 and 1,415 m, 180–1,300 m above the ground), two with a bail-out. That reads like a break-up in a hard pull (over-G) more than terrain. HVAR carries 8 rockets on top of the tanks AirAlert carries. AirAlert does not separate H1 from H2.
- **H3, formation.** One clear collision (run 2, T-s4). Both templates fly formation 20 at density 0.
- These can act together (a heavy aircraft manoeuvring low in close formation). The text log has no positions between events, so only a recording settles it (section 5).

## 4. Candidate fixes

MCU costs are per sortie copy. The 24-copy load file has 24 copies per side.

| | Fix | Where | MCUs | Check zones | What it changes | Risk / open |
|---|---|---|---|---|---|---|
| A | **AirAlert's order pair in the HVAR template:** add an air AttackArea (`AttackAir = 1`, 5 km, 600 s) before the ground order, and shorten the ground order to 120 s | template file only | +2 (one timer, one AttackArea) | 0 | HVAR then behaves as AirAlert did (1,150 m, no losses in 3 runs); on-station time stays 600 s | Shape S2, already flown in 2B. Depends on the unverified override (risk m). The ground attack window shrinks to 120 s even when a target is there |
| A′ | **Ground order first, then an air AttackArea after 120 s** (a 120 s delay on the second order) | template file only | +2 | 0 | Attacks for 120 s, then patrols high for the rest | Not flown. `ON STATION` would start from the second timer, so the hold becomes 120 + 600 s. Template Builder writes per-order delays (`OrderSpec.delay_s`); check that `library.rs` accepts the long timer |
| B | **Shorten the HVAR ground `Time` to 120–180 s** | template file only | 0 | 0 | On-station time becomes 120–180 s (`T_attack` = largest `Time`) | Cuts the hold that decision 0018 just asked for, unless the shell takes on-station time from elsewhere (a ruling). Losses started at 101 s, so 120 s may not be enough |
| C | **The shell gives an air AttackArea 120 s after `ON STATION IN`** to copies whose navigation lead has only a ground order | `shell.rs` | +2 per ground-attack copy (timer, AttackArea) | 0 | Same as A′ but for every ground-attack template (CloseSupport, ArmedRecon, Il-10, Yak) without changing the files | Shell code and tests. Cuts a real attack short at 120 s. Same unverified override as A |
| D | **Give the ground order only if enemy vehicles are in the area:** a vehicle check zone at the target, else an air order | `shell.rs` | about +4 | +1 | Attacks only real targets | 96 → 144 check zones per mission at 24 copies per side (D36 warns at 40). May not see static targets. Not recommended |
| E | **Raise `AILevel` 2 → 3** in the HVAR template | template file only | 0 | 0 | Better AI flying, harder enemy for players | A test of H1–H3, not a fix the user asked for |
| F | **Accept and document** | docs | 0 | 0 | Nothing | Losses continue on every empty target |

All template options change a curated file, which also changes what Template Builder users get outside Air Tasking. Decision D28 kept the curated files out of scope, and STATE "Waiting on the user" item 3 already asks whether to change the six files. A fix should follow the user's ruling there.

**Proposed order:** fly the diagnostic probe below first, because H2 (weight) is as likely as H1 (order) from the text logs, and A, A′ and C only help if H1 holds. If H1 holds, **A′** is the cleanest: 2 MCUs, no shell change, and it keeps a real 2-minute attack. If the user wants every ground-attack template covered without editing the files, **C** does the same in the shell for the same 2 MCUs per copy.

## 5. Probe flight 2C (proposed; the name is unused)

No pilot, same build route as 2B (`--no-resave`, `tools/p14/build_probe_mission.py`). Every copy on an empty target, area COLD. **No Delete before the end:** the mission closes at WP 1 + about 540 s, so the end-of-mission list gives every live aircraft's height and rocket count even without a recording. The losses so far all fell inside that window. Copies at least 40 km apart (losses ranged up to 12.4 km from a target). Three copies per cell; fly it twice for six samples per cell (at the 2B rate, about 4 in 9, six clean copies would be about a 3 % chance).

| Cell | Setup | Tests | Read |
|---|---|---|---|
| C0 | HVAR as shipped | control | losses expected; heights 350–450 m |
| C1 | HVAR orders, payload 2 (no rockets) | H2: is it the weight? | losses as C0 → not the weight |
| C2 | HVAR payload, fix A′ (ground 120 s, then air) | H1 and fix A′ | no losses, end heights near 1,000 m or more → A′ works |
| C3 | AirAlert as shipped | which of its two orders is in force (risk m); its height | a recording shows the patrol pattern |

Count per cell: `AType:3` with `AID:-1` between WP 1 and the close; heights and `RCT` from the end list.

### Getting a Tacview recording (cell T-tv, in the same flight)

What was found:
- **This PC's `game\data\startup.cfg` says `tacviewrecord = 0` today.** It is byte-identical to `startup.cfg.bak-20261015` and was last written at 00:43 on 2026-10-15, four minutes before run 2. So runs 2 and 3 most likely flew with Tacview off, and STATE's "now has `tacviewrecord = 1`" does not hold. Most likely the game rewrote the file from its own settings when it started (inference from the time stamp).
- Tacview's IL-2 guide and the IL-2 forum thread: set `tacviewrecord = 1` under `[KEY = track_record]`, then **in flight press LCtrl+R** to start the track recording; a red camera icon confirms it; files go to `data\Tracks\`. IL-2 records only objects around the recording client (about 10 km, per the forum; not checked here). On this PC the action `record` is bound to `key_LCONTROL+key_R` (`data\input\current.map`, line 40), and `game\data\Tracks\` exists and is empty.
- The server side is the `.sds` line `TacviewRecord = true` ("Allow clients to do TacView tracks recording", as in `data\Multiplayer\_test_cooperative_basic.sds`). It **allows** clients to record; it records nothing by itself. So the upstream author's server alone will not write an `.acmi` either.

Steps:
1. With the game closed, set `tacviewrecord = 1`. Start the game; check that the file still says 1. If the game reset it, look for the setting in the game's own options and set it there.
2. Host, tick the server's TacView box, press **Spectate**, put the camera on C0's first copy, press **LCtrl+R** and look for the red camera icon.
3. After leaving, look in `game\data\Tracks\` for an `.acmi` (and a track).
4. If a spectator records nothing: fly a player air start parked over C0's target (keyboard only, the level-flight helper) and record from it. A NATO player does not trip the HVAR copies' Zone IN, which watches DPRK aircraft.

Pass: an `.acmi` showing C0's flight path and heights through the loss window.

## Sources (outside the repo)

- Tacview, "User Guide – IL-2 Sturmovik": https://tacview.fandom.com/wiki/User_Guide_–_IL‑2_Sturmovik?oldid=710 (seen through a search summary; the page itself refused the fetch)
- IL-2 forum, "Tacview – understand what happened during your last flight", page 3: https://forum.il2sturmovik.com/topic/68918-tacview-–-understand-what-happened-during-your-last-flight/page/3

## After flight 2C (flown 2026-10-15, two runs, Tacview)

Results: `handoff/P14-probe-results.md`, "Flight 2C". In short:

- Every variant (HVAR, no rockets, fix A′, AirAlert) came down from 3,050 m to a few hundred metres above the ground and held an altitude set by the target's place. Flight 2B's height difference was the two targets, not the orders.
- Every loss seen with Tacview (7 aircraft, run 2; none in run 1) was a **collision inside a formation**, in gentle turns, in transit, in the descent and on station. The no-rocket cell lost the most. Wingmen came within 20 m of each other in all four variants.
- MCU timing held to the tick under server overload (mission time at 0.4× wall time). The overload may make the AI fly worse; that is not measured.

So options A, A′, B and C (the order) and the weight hypothesis are set aside. What is left:

| | Option | Where | MCUs | Open |
|---|---|---|---|---|
| G | **A looser formation** in the built-in templates' Formation command (`FormationDensity` 1 or 2 instead of 0, or another `FormationType`) | template files | 0 | What each density value means is not in the repository. Template Builder writes 0 for every air formation (`template.rs`, `air_form`), so a generator change would change its output files: a ruling for the user |
| H | **Fewer aircraft per server**, if the overload causes the collisions | mission size | 0 | Needs a run without the overload message to compare |
| I | **Accept**: the AI's own formation flying; `LOSSES` already ends a sortie that loses every aircraft | none | 0 | Losses continue at an unknown rate on a healthy server |

Proposed next probe (flight 2D, not built): the same empty targets with `FormationDensity` 0, 1 and 2 (and, if useful, Pairs), with few enough aircraft that the server shows no overload message (first find that count with one short run), flown twice with Tacview. Count close passes under 20 m and collisions per variant.

## Flight 2D design (proposed 2026-10-15; waits for the user's go-ahead)

**What `FormationDensity` means.** The repository does not say. The game's own editor does: `game\bin\editor\IL2Editor.exe` holds the Formation command's labels "Formation Density:" and the list **Dense, Safe, Loose**, right after the formation-type names. By the list's order, 0 = Dense, 1 = Safe, 2 = Loose (inference from the order; not checked in the editor's window). The spacing each gives in metres is not known; flight 2D measures it from Tacview. The repository's own files use 1 for bombers (`BomberMissions/DPRK Bombers - Small Formation.Group`, Heavy Wedge) and 1–2 for vehicle columns; Template Builder writes 0 for every air formation.

**Cells.** One HVAR 4-ship each (template as shipped, only its one Formation command changed), 16 AI aircraft in all, a third of flight 2C's 48. No spawn, no enemy, areas COLD.

| Cell | Formation | Density | Target (flight 2C cell, settled height above ground there) | START |
|---|---|---|---|---|
| D0 | 20 Wedge | 0 Dense (control, as shipped) | (185, −90) km (C1b, 498–580 m) | 60 s |
| D1 | 20 Wedge | 1 Safe | (230, −90) km (C2c, 543–565 m) | 120 s |
| D2 | 20 Wedge | 2 Loose | (185, −45) km (C2b, 567–722 m) | 180 s |
| DP | 19 Pairs | 0 Dense | (230, 45) km (C1c, 616–643 m) | 240 s |

- The four targets were picked from flight 2C because their flights settled at a similar height above ground, so the place does not stand in for the formation (flight 2C showed the place sets the height).
- Starts 60 s apart: subtitles give the aircraft count at each start (4, 8, 12, 16), and the host notes the first time "server overload" shows. That gives the test bed's limit without a separate run. If it shows, the run is marked overloaded.
- The host leaves at 15:00, as in flight 2C. Every copy is then at least 373 s on station.
- **Fly it three times**, with Tacview, so each formation has three sorties. Per run about 16 minutes if the server keeps up.

**What to read, per cell and run:** losses (`AType:3`, `AID:-1`); from Tacview the distance from each wingman to its nearest neighbour (median, 5th percentile) in transit, descent and on station; seconds under 20 m (counted as in flight 2C).

**Readings.**
- D0 has close passes or collisions on a healthy server, and D1 or D2 has none: a looser density is the fix (option G); changing the templates or Template Builder's 0 is the user's ruling.
- D0 has no close passes either: the overload was the likely cause (option H), not the formation.
- Three sorties per cell are too few to rank collisions (flight 2C lost aircraft in 4 of 24 sortie-runs); the spacing and the close passes are the measure, the collisions only a check.
- Every cell has close passes: density does not help; Pairs, or option I (accept).
- If the overload message shows before DP starts, the run tells the limit but not the answer; fly the next run with fewer cells.

Prompt for Astra: `handoff/P14-flight-2d-codex-prompt.md`.
