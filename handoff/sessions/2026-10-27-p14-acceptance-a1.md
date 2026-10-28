# 2026-10-27 — P14 acceptance, flight A1 (a NATO player parked at K14): run sheet

**Status: flown 2026-10-28 (two sessions; results at the end).** The parked part could not be flown on this PC (no respawn); the rest was flown with the player orbiting K14. Decision 0036 (acceptance before step 9); A0 (no pilot) is in `2026-10-27-p14-acceptance-a0.md`.

**Why this flight.** No flight has tested a player who spawns inside an Air Tasking area, or one who stays still. Plan §3.5 lists "a plane already inside when the mission loads is not seen until it re-enters" as an unflown weakness. In A1 the entry trigger is on from Mission Begin and the player spawns inside it later. A1 is also the first flight with a winner: a sortie start, transit, on-station time, extension, end and deletion, the zone front end and the budget.

## Files

- **Code (test code only):** `src/airtask/export_acceptance_flights.rs`, included in `export.rs`'s test module. It holds `acceptance_a1_plan()`, `acceptance_a1_mission()`, the ignored writer `write_p14_acceptance_a1`, and the byte check of every frozen acceptance flight. In `export.rs`, `acceptance_mission()` now calls `traced_acceptance_mission(plan, input)`, and both writers share `write_acceptance_files`. `acceptance_test_plan()`, the base map fixture and all product code are unchanged; A0's frozen files still regenerate byte for byte.
- **Generate:** `cargo test --offline write_p14_acceptance_a1 -- --ignored` writes `target/p14/P14_Acceptance_A1.Group` (1,441,785 bytes), the six locale sidecars, `.trace.json` (246 Air Tasking sources) and `.spawns.json`.
- **Frozen:** `handoff/p14-probes/acceptance-a1/` (`P14_Acceptance_A1.Group`, `.eng`, `.trace.json`, `.spawns.json`). The test `acceptance_flights_regenerate_byte_for_byte` checks them with A0's three files.
- **Mission build** (from the frozen copies, no resaver):
  ```
  python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe handoff/p14-probes/acceptance-a1/P14_Acceptance_A1.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawns handoff/p14-probes/acceptance-a1/P14_Acceptance_A1.spawns.json --no-resave --name P14_Acceptance_A1_20261027 --title "P14 acceptance A1 (NATO player parked at K14)" --out "C:\Machine Intelligence\P14 Flight A1\mission"
  ```
  Output: `C:\Machine Intelligence\P14 Flight A1\mission\P14_Acceptance_A1_20261027.Mission` (1,839,275 bytes) and `.eng` (7,250 bytes). The script added F-80C ground-start planes to `K-14_Kimpo_AF`. Copy both files into `game\data\Multiplayer\Dogfight\` (needs the user's OK).

## What is in the mission

**Air Tasking (seed 7, as A0 except the DPRK target):**

- **One DPRK objective, on Kimpo:** (105,934; 269,477), the spawn point of `K-14_Kimpo_AF` in `TemplateExamples/K14 AFB_mp.Group`. Its cell is `THD01`; its entry trigger counts planes of country 601 only (`CheckPlanes 1`, `CheckVehicles 0`), radius 6 km; its presence check `THD01 INNER` is a 6 km check zone for coalition 2 (NATO).
- **Both DPRK entries aim at it:** `ED01` Yak-9P Airfield Raid 2+2 (**Zone**; copies `SD01`–`SD03`, 4 aircraft) and `ED02` Il-10 Attack on Kimpo 2×4 (**Both**; `SD04`–`SD06`, 8 aircraft). So the clock (`ED02`) and both zone entries (`ZD01`, `ZD02`) act on the one area the player sits in, and two zone entries plus the clock can ask for a third copy while two fly (the budget cap).
- **Why K14 works:** on the 1950-09-15 front (x ≈ 51,000 at that z), K14 (x 105,934) is on DPRK's side, so DPRK sorties aimed at it spawn on their own side (tested). A NATO player parked there is 0 km from the area centre, well inside r_in (6 km). `validate` gives no issue: A0's only warning (D30, on the old DPRK area at (45,000; 280,000)) went with that area.
- NATO's four entries are unchanged (12 copies, country 602). With no DPRK player they cannot launch.
- Timing read from the generated file:

| Part | Values |
|---|---|
| Threshold `THD01` | DWELL 60 s; presence check = `INNER PULSE` 0.1 s after `CHECK ON`; HOLD 120 s, so re-checks every 120.1 s; CLOSE 2 s, CLOSE DELAY 0.05 s |
| Clock, DPRK | FIRST 600 s, PERIOD 1,200 s; JITTER 1/2/3 at 0.1 / 180.1 / 360.1 s (Random 27 / 36 / 57 %); DRAW 0.3 s; `ED02 RANDOM` 0.5 s; attempt 1.5 s; SLOT CLOSE 16.5 s |
| Clock, NATO | same ticks and jitter; SLOT CLOSE 30 s |
| Zone | Chance 50 %. `ZD02` (Il-10) CHANCE at PASS + 0.10 s, MISS WAIT + 1.00 s; `ZD01` (Yak) CHANCE + 0.75 s, MISS WAIT + 1.65 s; FIRE 0.05 s after CHANCE; DENY WAIT 1 s; COOLDOWN 900 s |
| Budget, DPRK | 2 copies; recount `T_count` 0.45 s |
| Yak copy | spawn (131,838; 284,609), 30 km NNE of K14, heading 210°, 3,050 m, WP 500 km/h; on station 300 s; EXTEND 600 s; HARD STOP 1,716 s; LOSSES at 4 |
| Il-10 copy | same spawn, 1,500 m, WP 390 km/h; on station 600 s; EXTEND 600 s; HARD STOP 2,076.92 s; LOSSES at 8 |
| Both | Zone IN and Zone Out 35 km at K14, coalition 2; attack orders `AttackGTargets 1`, `AttackGround 0`, `AttackAir 0`, 1,500 m around K14; DONE HUB → deletion 60.6 s (`MISSION END` 0.1 + `DELAYED END ORDERS` 60 + `DELETE DELAY` 0.5) |

**From the K14 reference group (`--ref`):** the airfield (ground start, F-80C) and its country-601 ground objects, among them **21 AA guns** (6 Bofors L60, 7 M2 90 mm, 5 M1 120 mm, 2 M19, 1 M16). They are vehicles, so the entry trigger ignores them; they will fire at the DPRK raids.

**Fallback air start (`--spawns`):** "P14 A1 NATO air start", F-80C, (85,934; 269,477): 20 km south of K14 at 500 m, heading 0° (north), straight over the area centre. Outside every area's 12 km R.

**Countries in the `.Mission`:** 66 Planes of 602 (48 Air Tasking NATO + the 18 of the wave), 36 of 503, and no AI aircraft of 601. The only 601 aircraft will be the player.

### NATO Fighters Wave 1 on 602 (the user's request)

The Map's own pack, 18 fighters (F-80C and F-51D) in two halves of 9, at (110,000; 280,000) and (130,000; 300,000). Each half is launched by a 16 km `Zone IN` for coalition 1 (DPRK side) planes. In A1 only, all 18 Planes are country 602 (NATO, not a player country). Air Tasking's NATO AI already uses 602.

- **What it removes:** the wave can no longer start a DPRK cycle. `THD01 TRIGGER` filters by country (601), and only a cycle's entry needs it.
- **What it does not remove:** the presence check (`THD01 INNER`) and each sortie's `Zone IN` and `Zone Out` filter by coalition. A wave aircraft within 6 km of K14 at a re-check passes it (keeps the area HOT). Wave aircraft within 35 km see a sortie's `Zone IN` and can hold or (risk b) fire its `Zone Out`.
- **It will launch on every raid.** The DPRK spawn point is 15.5 km from (130,000; 300,000), inside that half's 16 km zone, so that half launches when a DPRK copy is activated. The raid's path to K14 passes 7 km from (110,000; 280,000), 11 km from K14, so the other half launches too. Expect a dogfight over or near K14 on every raid, and losses on both sides.

## Run

1. The user confirms the computer is free. Copy the `.Mission` and `.eng` into `game\data\Multiplayer\Dogfight\` (with the user's OK). `mission_text_log` stays on.
2. Host from the game client as for A0 and 2J (`handoff/sessions/2026-10-13-delete-probes.md`; traps in `docs/il2-loader-and-log-facts.md`): Multiplayer, Create, "Available from internet" off, this mission alone in the rotation.
3. **Join as NATO at `K-14_Kimpo_AF`, F-80C, ground start.** Note the mission time of the spawn (the player's `AType:10` line; mission s = (T − 3) / 50). **Stay parked:** no taxi, no take-off. The engine may stay off.
4. **Live check (optional).** Breadcrumbs are `AType:8` lines with a fixed position: `THD01 INNER` is `POS(5180.0000,0.0000,5000.0000)`, `THD01 CLOSE DELAY` 5170, `CLK D SLOT OPEN` 5000, `ED02 GO` 5040, `SD01 START` 5510, `SD04 START` 5840 (x; y 0, z 5000).
5. **If no `THD01 INNER` by spawn + 90 s:** the spawn did not count as entering. Record that, then use the fallback: end the flight, spawn at "P14 A1 NATO air start", and fly straight and level north at the spawn speed (about 460 km/h) over K14. `THD01 INNER` should log about 60 s after crossing the 6 km ring (the player then about 1.7 km past the centre). Right after it, end the flight and spawn again at K14 on the ground, inside 120 s, so the re-check at INNER + 120.1 s finds the parked plane. Then stay parked as in step 3. If that re-check fails (COLD, trigger re-armed), stay parked anyway: the respawn into a re-armed trigger is a second case of P2.
6. **If the player is killed:** spawn again at K14 on the ground at once, and note both times.
7. **Leave at 45:00 mission time (2,700 s); not before 37:00 (2,220 s).** 37:00 covers the second clock slot (latest 2,160.9 s) and the first copies' deletion; 45:00 adds the second zone cycles and a likely budget refusal (worked example below). Up to 30 AI aircraft fly at once (12 DPRK, 18 wave); this PC ran 20 at about 0.85× wall time and 48 at 0.4×, so 45 min of mission time may take 55–90 min. Read the clock from the log, not the wall.
8. Leave as in A0. Copy the logs with `cp -p` to `src/testdata/missionlog/real_45/` (next free; check first).
9. Replay:
   `il2_mission_utility replay --group handoff/p14-probes/acceptance-a1/P14_Acceptance_A1.Group --trace handoff/p14-probes/acceptance-a1/P14_Acceptance_A1.trace.json --logs src/testdata/missionlog/real_45 --out target/p14/acceptance_a1_report.md`

## Predictions

T0 = the player's spawn time; s = a copy's `START` time. Breadcrumbs log about 0.06 s after the times below (A0: Mission Begin logged at 0.06 s). Each line has an id for the replay report.

| Id | Prediction | How to confirm |
|---|---|---|
| **P1** | Loads; `MFile` names `P14_Acceptance_A1_20261027`; no loader error. The spawn menu offers `K-14_Kimpo_AF` (F-80C, ground) and "P14 A1 NATO air start" (F-80C, air). | log header; screen |
| **P2** | **Entry on spawn: no prediction.** It is what A1 is for. If a spawn inside the switched-on trigger counts as Object Entered, `THD01 INNER` logs at **T0 + 60.1 s** (DWELL 60 + pulse 0.1). If not, neither `THD01 INNER` nor `THD01 CLOSE DELAY` ever logs (the check zone is only switched on after a DWELL) and no DPRK copy starts. The nearest evidence (flight 2J, KA: a plane activated inside a switched-off trigger was not reported when the trigger came on) points to "not seen". | `INNER` time minus the `AType:10` time |
| **P3** | **HOT and presence checks:** from the first `THD01 INNER` on, while the player stays parked and alive, `THD01 INNER` logs every 120.1 s (T0 + 60.1 + 120.1·k) and `THD01 CLOSE DELAY` never logs. | breadcrumb times |
| **P4** | **Zone rolls at each pass while armed** (each 50 %, independent): `ZD02 CHANCE` at INNER + 0.10, then `ZD02 FIRE` and `SD04` (or the next Il-10 copy) `START` at + 0.15; on a miss, `ZD02 MISS WAIT` at + 1.00. `ZD01 CHANCE` at + 0.75, then `ZD01 FIRE` and a Yak `START` at + 0.80; on a miss, `ZD01 MISS WAIT` at + 1.65. After a miss: `COOLDOWN` and `ARM` 900 s later, and the next pass rolls again. After a FIRE: that zone stays quiet until its copy's `DONE HUB` + 900 s (`COOLDOWN`, `ARM`), then rolls at the next pass. The Yak request comes after the Il-10's recount (0.15 + 0.45 = 0.60 < 0.80 s), so both can start at the first pass. | breadcrumbs |
| **P5** | **Clock.** DPRK `CLK D SLOT OPEN` at 600.1 / 780.1 / 960.1 s and 1,800.1 / 1,980.1 / 2,160.1 s, at most once per tick, or not at all (about 20 % of slots). `ED02 GO` at SLOT OPEN + 0.80 s only if the area is HOT, `ED02` has copies left, and no Il-10 copy started since the previous slot opened (`ED02 LAST`; so a slot that follows an Il-10 zone start is quiet). A slot before the first pass finds the area COLD: no GO. Never two GOs in one slot. NATO: `CLK N SLOT OPEN` as in A0; never `EN.. GO`, `SN.. START`, `THN.. INNER` or a `ZN` breadcrumb (no DPRK player). | breadcrumbs |
| **P6** | **START:** each `START` is followed 0.5 s later by the activation of its 4 (Yak) or 8 (Il-10) aircraft (`AType:28` each) at (131,838; 284,609). | log |
| **P7** | **Transit to K14:** that copy's `Zone IN` (fired at `WP 1`, over K14; the parked player is inside 35 km) at about s + 218 s (Yak, 30 km at 500 km/h) or s + 279 s (Il-10, 390 km/h), ± 30 s. | `Zone IN` time |
| **P8** | **On station:** `SDxx ON STATION` at Zone IN + 300.5 s (Yak) or + 600.5 s (Il-10); with the player still parked, the area is HOT, so `SDxx HOT` logs in the same tick (not `COLD`). | breadcrumbs |
| **P9** | **Extension once:** `SDxx EXTEND` and `SDxx DONE HUB` in the same tick, ON STATION + 600 s, although the player stays. No second extension. So `DONE HUB` ≈ s + 1,119 s (Yak) or s + 1,480 s (Il-10). | breadcrumbs |
| **P10** | **Deletion:** `SDxx DELETE DELAY` at DONE HUB + 60.6 s; the copy's aircraft leave the log's object list. | breadcrumbs, log |
| **P11** | **No `HARD STOP`** (s + 1,716 / s + 2,076.92; it needs an arrival about 600 s late). **No `Zone Out`** while the player is parked within 35 km. A `Zone Out` while the player is still parked is a finding: risk b (it fires when ANY watched plane leaves), with the wave as the likely leaver. | breadcrumbs |
| **P12** | **Losses:** a copy that loses all its aircraft (4 / 8, wave and AA) ends at once: `SDxx LOSSES` and `DONE HUB` in one tick, deletion 60.6 s later. Likely for some raids. | breadcrumbs |
| **P13** | **Budget cap:** never a third DPRK `START` while two DPRK copies exist (between a second `START` and the next DPRK `DELETE DELAY`). A recount that finds two flying logs `BUD D COUNT`. A refused zone request: `ZDxx FIRE`, `ZDxx DENY WAIT` 1.0 s later, no `START`; `COOLDOWN` and `ARM` 900 s after the DENY WAIT. A refused clock request: `ED02 GO` with no `SD.. START`. | breadcrumbs |
| **P14** | **No exhaustion** within 45 min (it needs three copies of one entry). | no `EDxx EXHAUSTED` effect |
| **P15** | **The wave:** no wave aircraft before the first DPRK `START`; the half at (130,000; 300,000) appears within seconds of it, the other half as the raid nears K14. It fights the raid. It never starts a DPRK cycle (country 602). | log, Tacview if recorded |
| **P16** | **If the raid attacks the parked player:** whether `AttackGTargets` takes in a parked aircraft is not known. If the player is killed: no Air Tasking logic reacts to the death; the next re-check passes only if another NATO plane (the wave) is within 6 km, otherwise `THD01 CLOSE DELAY` at check + 2.05 s (COLD, `REARM ON` switches the trigger on again); a copy whose on-station time ends while COLD logs `SDxx COLD` and `DONE HUB` with no extension. The respawn at K14 is a second case of P2, into a trigger that was switched on while the player was away. | breadcrumbs, `AType:3`/`AType:10` |
| **P17** | **Replay:** no dead links. | report |

### Worked example (one possible draw)

T0 = 120 s, both first rolls pass, jitter step 1 in every slot. The real draws differ; the rules above decide each line.

| Mission s | Event |
|---|---|
| 120 | player spawns at K14 |
| 180.10 | `THD01 INNER` (PASS 1; P2) |
| 180.25 | `ZD02 FIRE`, `SD04 START` (Il-10 #1) |
| 180.90 | `ZD01 FIRE`, `SD01 START` (Yak #1); the recount finds two: `BUD D COUNT` |
| 300.20, 420.30, … | `THD01 INNER` every 120.1 s |
| ≈ 399 / 459 | `Zone IN`, Yak #1 / Il-10 #1 |
| 600.10 | `CLK D SLOT OPEN`: `ED02` is LAST (Il-10 started at 180) and the gate is shut: no GO |
| ≈ 700 | `SD01 ON STATION`, `SD01 HOT` |
| ≈ 1,060 | `SD04 ON STATION`, `SD04 HOT` |
| ≈ 1,300 / 1,360 | `SD01 EXTEND` + `DONE HUB` / `SD01 DELETE DELAY` |
| ≈ 1,660 / 1,720 | `SD04 EXTEND` + `DONE HUB` / `SD04 DELETE DELAY` |
| 1,800.10 / 1,800.90 | `CLK D SLOT OPEN` / `ED02 GO`, `SD05 START` (Il-10 #2) |
| ≈ 2,200 | `ZD01 COOLDOWN`, `ARM` (SD01's DONE HUB + 900) |
| 2,221.8 | `THD01 INNER`; `ZD01 FIRE`, `SD02 START` (Yak #2) if the roll passes; two flying |
| ≈ 2,560 | `ZD02 COOLDOWN`, `ARM` (SD04's DONE HUB + 900) |
| 2,582.1 | `THD01 INNER`; if `ZD02 CHANCE` passes: `ZD02 FIRE` at 2,582.25, refused (SD05 and SD02 fly), `ZD02 DENY WAIT` at 2,583.25 (P13) |

## Questions for the user (recorded, not decided)

1. **Is it wanted that a player parked at their own airfield inside an area keeps drawing sorties every slot?** (Design question.) The threshold cannot tell a parked or landed player from one loitering in the air: while the player sits at K14, the area stays HOT, each clock slot can start an Il-10 copy, and each zone entry re-arms 15 min after its copy ends, until each entry's three copies are used. A check zone has no altitude or speed filter, so "airborne only" would need another mechanism. A1 shows how it plays out.
2. **The Map's own AI aircraft on a player country** (601 / 501) start Air Tasking sorties in real missions. Export puts Air Tasking's own AI on 602 / 503; should it do the same for the Map's AI aircraft? It changes Generate output, so it is the user's call: decision record `docs/decisions/0037-map-ai-off-player-countries.md` (Proposed).

## Results (flown 2026-10-28)

Flown by Claude on this PC (game `261013`), keyboard only. Two log sessions, both from the frozen mission `P14_Acceptance_A1_20261027`:

- **Run 1**, session `2026-10-28_02-46-26`, logs `src/testdata/missionlog/real_45/` (25 files): the parked spawn (P2).
- **Run 2**, session `2026-10-28_03-00-07`, logs `src/testdata/missionlog/real_46/` (221 files): the fallback air start, then an orbit over K14 instead of parking. Mission end logged at about 2,453 s; mission time kept pace with wall time (0.97×). The game showed "Multiplayer server overload" once, at about 1,900 s.
- Replay of each: `target/p14/acceptance_a1_report.md` (run 2) and `target/p14/acceptance_a1_run1_report.md` (not committed). **No dead links** in either.

**Change from the run sheet.** After the first flight ended (Finish Mission, run 1) and after the player was shot down (run 2), the briefing screen showed "Waiting for mission results…" and never cleared; **Start stayed disabled**, so the player could not spawn again in that session. Run 2 is the server's reload of the mission (a new session), where the air start was the first spawn. With no respawn at K14, the player was not parked after the pass: it orbited over K14 at 500 m (a held left turn with the level-flight helper, about 380 km/h). The orbit drifted, so some presence checks found the player more than 6 km away.

### Run 1 (parked at K14)

| Id | Flown |
|---|---|
| P1 | **yes**: loads; `MFile` names the mission; the spawn menu offered `K-14_Kimpo_AF` (F-80C, ground start) and "P14 A1 NATO air start" |
| P2 | **Not seen.** Spawn at 57.2 s at (105,934.1; 269,477.0), the area centre, country 601. No `THD01 INNER` and no `THD01 CLOSE DELAY` by 155 s (spawn + 98 s), nor later: the only breadcrumbs were Mission Begin's. A ground spawn inside the switched-on entry trigger does not count as entering it. |

### Run 2 (air start, then an orbit over K14)

Times are mission seconds from the breadcrumbs. Player spawn 63.6 s at (85,968; 269,477), 499 m, heading 0°.

| Id | Flown |
|---|---|
| P2 (entry by flying in) | The trigger fired when the player crossed the 6 km ring at speed (about 144.6 s; the player reached 780 km/h). The check at DWELL + 0.1 found the player already 6 km past the centre: `THD01 CLOSE DELAY` 206.74 s (COLD). The trigger re-armed, and each later re-entry gave `THD01 INNER` 60.1 s later: 443.94, 874.28 and 1,898.30 s |
| P3 | **yes**, while the player stayed within 6 km: `THD01 INNER` every **120.00 s** (443.94 → 563.94; 874.28 → … → 1,594.28, seven in a row). Each time the orbit drifted out, the next check failed: `THD01 CLOSE DELAY` at 685.98, 783.04, 1,716.32, 1,797.92 s, then 2,020.34 s after the player's death |
| P4 | **yes.** First pass 443.94: both rolls missed, `ZD02 MISS WAIT` 444.94 (+1.00), `ZD01 MISS WAIT` 445.58 (+1.64). `ZD02 COOLDOWN` 1,344.94 and `ZD01 COOLDOWN` 1,345.58 (miss + 900 s). Pass 1,354.28: `ZD02 CHANCE` +0.10, `ZD02 FIRE` and **`SD05 START` +0.14** (Il-10 #2); `ZD01 MISS WAIT` +1.64 (miss); `ZD01 COOLDOWN` again at 2,255.92. No Yak copy started |
| P5 | **yes.** DPRK tick 1: `CLK D JITTER 1` and `SLOT OPEN` 600.10, **`ED02 GO` and `SD04 START` 600.90** (area HOT since 443.94): Il-10 #1. `JITTER 2` at 780.10 opened nothing (latch). DPRK tick 2 (1,800 s): quiet, no jitter step passed. NATO: `SLOT OPEN` 960.10 (`JITTER 3`) and 1,980.10 (`JITTER 2`), `EN01 RANDOM` after each; never a NATO `GO`, `START`, threshold or zone breadcrumb |
| P6 | Il-10s seen on the map over K14 from about 830 s; activation lines not counted |
| P7 | **yes**: `Zone IN` 861.42 (SD04, s + 260.5) and 1,615.28 (SD05, s + 260.9); predicted s + 279 ± 30 |
| P8 | **yes**: `SD04 ON STATION` 1,461.92 = Zone IN + 600.5; no `SD04 COLD` (the area was HOT) |
| P9 | **Not tested cleanly**: SD04 had ended early (P11) before its extension was due. `SD04 EXTEND` still logged at 2,061.92 (ON STATION + 600), with no second `DONE HUB` |
| P10 | **yes**: `SD04 DELETE DELAY` and `SD05 DELETE DELAY` 1,962.46 = DONE HUB + 60.6 |
| P11 | **`Zone Out` fired**, for both copies, at **1,901.86 s, 3.06 s after the player was shot down** (1,898.80): `SD04 DONE HUB` and `SD05 DONE HUB` in the same tick. No other NATO plane can be placed within 35 km from the text log at that time (no Tacview). Read: the last watched plane leaving by dying fires `Zone Out` |
| P12 | No `LOSSES`. Five Il-10s were destroyed (834.4, 976.5, 1,053.2, 1,212.2, 1,759.1 s; four of them SD04's), three NATO wave aircraft (834.4, 1,530.1, 1,792.4 s) |
| P13 | **yes**: `BUD D COUNT` 1,354.70 after `SD05 START` (two DPRK copies flying); no third start. No refusal came up |
| P14 | **yes**: no exhaustion |
| P15 | Wave aircraft (blue) over K14 from about 900 s and fighting the raid; country 602 cannot fire `THD01 TRIGGER` |
| P16 | **The raid killed the player**: an Il-10 (AID 541696, gunfire) shot the orbiting F-80C at 500 m over K14, 1,898.80 s. No Air Tasking logic reacted to the death itself; the next check failed (`THD01 CLOSE DELAY` 2,020.34). No respawn (see above) |
| P17 | **yes**: no dead links |

### Not predicted

- **A copy's timers keep running after it ends early.** After `Zone Out` → `DONE HUB` (1,901.86) and deletion (1,962.46): `Zone IN` logged again for both copies at 2,035.12 (a NATO plane, not identified, came within 35 km); `SD04 EXTEND` at 2,061.92 (its on-station timer); `SD05 ON STATION` and `SD05 COLD` at 2,215.78 (Zone IN 1,615.28 + 600.5, area COLD). None of them logged a second `DONE HUB` or `DELETE DELAY`, and nothing started. Whether a late `DONE HUB` path could fire for a deleted copy (zone `COOLDOWN`, budget recount) needs a look in the shell's wiring. **Fixed by decision 0038** (2026-10-28, branch `claude/p14-shell-quiet-a24002`, commit `ed4a3a1`, not merged): `DONE HUB OFF` now also switches off the copy's `HARD STOP`, `ON STATION`, `EXTEND`, `LOSSES`, zones and attack timers. A trace found no harmful path before the fix: every late pulse ended at the latched `DONE HUB` or at a breadcrumb.
- **Presence checks run every 120.00 s, not 120.1 s.**

### Still open

- **The parked player (question 1 below)** was not flown: P2 shows a parked spawn starts nothing, and this PC cannot respawn. A player who lands at K14 after flying in (the user, piloted) is the case still to see.
- P9 (one extension, then the turn for home) and a budget refusal (P13) were not seen.
