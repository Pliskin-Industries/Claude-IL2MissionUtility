# 2026-10-28 — P14 acceptance, flight A2 (a DPRK player through three NATO areas): run sheet

**Status: flown 2026-10-29 by Claude (two runs; results at the end).** Branch `claude/p14-acceptance-a2`, cut from `main` at `7e095f2`. The user approved the design and the copy into the game (2026-10-28: "Approved. Build A2 as planned: 15 km spacing, R 16 km, zone chance 100 %, cooldown 5 min, Zone-only entries, --spawns air start on 501, and decision 0042 recorded as Proposed.").

**Why this flight.**
- It is the first piloted flight of a mission made by the app itself (Map Generate with Air Tasking, Trace build on), and the first NATO-side flight.
- It should show, for the first time in the game:
  - a **budget refusal**;
  - a sortie that ends its time on station with its area COLD and turns for home with no extension;
  - the **single extension** and then the turn for home (A1's P9).
- It also checks decision 0038 (after a copy's `DONE HUB`, nothing from it logs except `DELETE DELAY`) and decision 0037 (the Map's own AI planes are not on 601 / 501).

## Files

- **Code (test code only):**
  - `src/ui_tests.rs`:
    - `build_a2` builds the base map on the Map tab and generates it through `generate_front_file`, the Generate button's own call. Place NATO is clicked. The objectives, the pool entries and the settings are set on the app directly, as the other Map tests do, with the values a user would enter.
    - The ignored writer `write_p14_acceptance_a2`.
    - The check `a2_map_build_watches_three_nato_areas_for_dprk_players`: the build facts below. Until A2 was flown it was also a byte check against the frozen files; since the switch-breadcrumb fix (branch `claude/p14-paused-timers`) A2 is frozen as flown, checked by parse and links.
  - `src/airtask/export_acceptance_flights.rs`: `frozen_a2_files_parse_and_links_resolve` (parse and links, as for A0 and A1).
  - No product code changed.
- **What `build_a2` does in the app:**
  - Map tab, default date **1951-04-22**.
  - NATO objectives A, B, C (below); no DPRK objective, no arrow.
  - Forces › Fighters › **Place NATO** (the Map's own NATO fighter pack).
  - Three pool entries, all **Zone**:
    - A: F-80C HVAR Strike;
    - B: F-51D Close Support;
    - C: F-80C HVAR Strike.
  - Air Tasking settings:
    - Threshold R **16 km**;
    - zone chance **100 %**;
    - zone cooldown **5 min**;
    - budget 2 / 2 (default);
    - seed 7.
  - Trace build on; Generate Base Map. Terrain heights are applied (built-in store).
- **Generate:** `cargo test --offline write_p14_acceptance_a2 -- --ignored` writes these to `target/p14/`:
  - `P14_Acceptance_A2.Group` (1,324,434 bytes);
  - six language files;
  - `.trace.json` (209 breadcrumbs);
  - `.spawns.json`.
  - A second run gives the same bytes.
- **Frozen:** `handoff/p14-probes/acceptance-a2/`: `P14_Acceptance_A2.Group`, `.eng`, `.trace.json`, `.spawns.json`.
- **Mission build** (from the frozen copies, no resaver):
  ```
  python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe handoff/p14-probes/acceptance-a2/P14_Acceptance_A2.Group --spawns handoff/p14-probes/acceptance-a2/P14_Acceptance_A2.spawns.json --no-resave --name P14_Acceptance_A2_20261028 --title "P14 acceptance A2 (DPRK player through three NATO areas)" --out "C:\Machine Intelligence\P14 Flight A2\mission"
  ```
  - Output: `P14_Acceptance_A2_20261028.Mission` (1,328,017 bytes) and `.eng` (762 bytes). The loader-safety check passed.
  - **Copied** into `C:\Program Files\IL2Series\game\data\Multiplayer\Dogfight\` on 2026-10-28 (identical bytes).
- The `.Mission` takes its Options block from the game's dogfight test mission (as A0 and A1), so the game shows 1 July 1953, 12:30. Air Tasking does not use the date.

### Why the Map's own air start is not used (decision 0042, Proposed)

The app's DPRK air start writes **country 502**. The NATO areas' entry triggers count **501** only, so a player spawned there would never be seen. A2 therefore takes its air start from the game's own DPRK airfield (501) with `--spawns`, as A1 did. Decision 0042 asks the user how to make the two agree.

## The line and the areas

All coordinates are game metres (x north, z east). The player flies **east (heading 090)** along x = 190,000.

```
 north (x+)
   ^
   |   spawn            A                B                C        (x = 190,000)
   |   z 258,000        z 280,000        z 295,000        z 310,000
   |   *--- 22 km ----->(  )---- 15 km -->(  )---- 15 km -->(  )--> heading 090
   |                    r_in 8 km        r_in 8 km        r_in 8 km   (R 16 km; the R rings overlap)
   |                  ^ 27 km N        ^ 22 km N        ^ 17 km N of the 1951-04-22 front
   |                  copies from 311° copies from 313° copies from 315° (30 km SE of the area)
 front (x ≈ 163,000 at A, 168,000 at B, 173,000 at C)
```

| | Centre (x; z) | From spawn | Ring in / out (8 km) | North of front | Copies spawn at (x; z), heading to target |
|---|---|---|---|---|---|
| Spawn | 190,000; 258,000 | 0 | — | 36 km | — |
| A (THN01, ZN01, SN01–03 HVAR) | 190,000; 280,000 | 22 km | 14 / 30 km | 27 km | 170,351; 302,669 (on the front), 311° |
| B (THN02, ZN02, SN04–06 Close Support) | 190,000; 295,000 | 37 km | 29 / 45 km | 22 km | 169,564; 316,963 (5 km south of it), 313° |
| C (THN03, ZN03, SN07–09 HVAR) | 190,000; 310,000 | 52 km | 44 / 60 km | 17 km | 168,742; 331,168 (8 km south of it), 315° |

- The spawn is 22 km from A: outside every area's R (16 km) and every entry trigger (8 km). A spawn inside an area is not seen (A1 finding 1).
- Copy heights: HVAR at 3,050 m, Close Support at 1,500 m. **The player flies at 2,000 m**, between the two.
- **The Map's NATO fighter pack** (two waves, 60 planes, all on 602) launches only for DPRK-side planes within 16 km of its points. The nearest point is 109 km from the line, so it never launches.

### Timing read from the file

| Part | Value |
|---|---|
| Entry trigger `THNaa TRIGGER` | Complex Trigger, radius 8,000 m, country 501, at the area centre |
| Threshold | DWELL 80 s; `INNER PULSE` 0.1 s; HOLD 160 s; CLOSE 2 s; CLOSE DELAY 0.05 s; `INNER` check zone 8,000 m, coalition 1 |
| Zone | `CHANCE` 0.10 s at Random **100 %**; FIRE 0.05 s; DENY WAIT 1 s; MISS WAIT 1 s; **COOLDOWN 300 s**; one entry per area, so no stagger |
| Budget, NATO | 2 copies (`BUD N COUNT` Counter 2); recount `END` 0.60 s; `SNjj FLYING` 0.05·j s |
| Clock | none: every entry is Zone, so no NATO clock is built; no DPRK side at all |
| HVAR copy (SN01–03, SN07–09) | 4 × F-80C; zone spawn 30 km, WP 660 km/h, so ≈ 164 s to the target; on station 600 s; EXTEND 600 s; HARD STOP 1,963.64 s |
| Close Support copy (SN04–06) | 4 × F-51D; WP 520 km/h, so ≈ 208 s; on station 900 s; EXTEND 600 s; HARD STOP 2,307.69 s |
| Each copy | `Zone IN` and `Zone Out` 35 km at its area, coalition 1; `DONE HUB` → `DELETE DELAY` 60.6 s; `DONE HUB OFF` switches off the copy's timers and zones (decision 0038) |

### Choices, and why (approved by the user)

- **15 km apart, not 30.**
  - Each copy's `Zone IN` sees the player when it reaches its area. Its `Zone Out` then ends the copy when the player is 35 km past that area, about 264 s after the area went HOT at 450 km/h. The copy is deleted 60.6 s later, about 325 s after HOT.
  - At 30 km spacing, C asks about 480 s after A, when A's copy is already gone, so there would be no refusal.
  - The refusal needs spacing under about 20 km: 20.3 km at 450 km/h, 19.5 km at 720 km/h. This assumes the copy's `Zone IN` fires before the player is 35 km past its area, which it does here.
- **R 16 km (r_in 8 km), not the default 12.**
  - A straight pass at 450 km/h leaves the 8 km ring 48 s after the presence check (36 s at 12 km).
  - The orbit at C may drift 6.5 km instead of 4.5 km. Drift spoiled A1's orbit.
- **Zone chance 100 %.** At the default 50 %, all three areas would pass only one time in eight.
- **Cooldown 5 min.**
  - C asks again 320 s after its first refusal, while both copies still fly: a second refusal, which shows the zone re-arms after a refusal (U8).
  - It asks once more 640 s after, when A's copy is gone: granted.
  - It shortens the orbit by about 5 min.
- **Zone-only entries**, so the clock adds nothing to the timeline. **HVAR on A and C** (600 s on station, the shortest); **Close Support on B**. AirAlert (a fighter patrol) and Armed Recon (a fighter escort) are left out: either could go after the Yak.
- **Yak-9P at about 450 km/h true.** A MiG-15bis is too fast: a pass faster than about 720 km/h through the centre is out of the 8 km ring before the check.

## Pilot steps

Times are from the spawn, **S** (stopwatch), at **450 km/h true**. At 2,000 m that is about **410 km/h on the Yak's dial**. Mission time from the log: s = (T − 3) / 50.

1. **Before starting the game:**
   - `tacviewrecord = 1` under `[KEY = track_record]`, with the game closed (the game rewrites `startup.cfg` on exit);
   - `mission_text_log` stays on.
2. **Host from the client** as for A0 and A1 (`handoff/sessions/2026-10-13-delete-probes.md`): Multiplayer, Create, "Available from internet" off, `P14_Acceptance_A2_20261028` alone in the rotation. Tick the server's "TacView record" box.
3. **Spawn:** DPRK side, **"P14 A2 DPRK air start"**, Yak-9P (air start, 2,000 m, heading 090). Start the stopwatch at the spawn. **One spawn only:** this PC cannot spawn twice in one session.
4. **At once:**
   - **LCtrl+R** starts Tacview (red icon top right). If no icon shows, fly on: the text log is the main record.
   - Level-flight helper on (**LShift+A**; an air start may begin with it on, so a first press may say "Helper disabled": press again).
   - Hold heading 090. Throttle to about 410 km/h on the dial.
   - Do not turn or climb.
5. **The straight leg:**

   | S + | Where | What the log should show |
   |---|---|---|
   | 1:52 | enter A's ring | — |
   | 3:12 | 2 km past A | `THN01 INNER` (A HOT); `ZN01 FIRE`, `SN01 START` |
   | 3:52 | enter B's ring | — |
   | 5:12 | 2 km past B | `THN02 INNER` (B HOT); `SN04 START`; `BUD N COUNT` |
   | 5:52 | enter C's ring | `THN01 CLOSE DELAY` (A COLD) at 5:54 |

6. **Optional last leg, the orbit (recommended):**
   - At **S + 6:56** you are over C's centre (52 km from the spawn). Start a steady **left orbit**: the helper on, **LAlt+Z held**, about 4°/s, radius about 1.5–1.8 km.
   - Note where the orbit starts on the minimap. Check every minute that you stay within 5 km of that point; the presence check counts 8 km.
   - At 7:12, `THN03 INNER` (C HOT) and **the first refusal** should log.
7. **Stay in the orbit until `SN07 EXTEND` and `SN07 DONE HUB` log**: about **S + 40:37** (≈ 34 min of orbit), then 1 more minute.
   - Live check (optional): breadcrumbs are `AType:8` lines with `POS(x,0.0000,5000.0000)`:

     | Breadcrumb | x |
     |---|---|
     | `THN01/02/03 INNER` | 6030 / 6050 / 6070 |
     | `THN01/02/03 CLOSE DELAY` | 6020 / 6040 / 6060 |
     | `ZN03 FIRE` | 5980 |
     | `ZN03 DENY WAIT` | 5990 |
     | `ZN03 COOLDOWN` | 6000 |
     | `BUD N COUNT` | 6080 |
     | `SN01 START` | 6130 |
     | `SN01 ON STATION` / `COLD` / `DONE HUB` / `DELETE DELAY` | 6150 / 6160 / 6190 / 6200 |
     | `SN04 START` | 6460 |
     | `SN07 START` | 6790 |
     | `SN07 ON STATION` | 6810 |
     | `SN07 EXTEND` | 6830 |
     | `SN07 DONE HUB` | 6850 |
     | Copies' `Zone IN` / `Zone Out` (SN01, SN04, SN07) | 6110 / 6120, 6440 / 6450, 6770 / 6780 |
8. **Leave at about S + 41:40:** level out, **turn north (heading 000)** and fly straight for 5 min.
   - Do not fly back west: A and B would see you again and start new copies.
   - This takes you out of the 35 km of all three areas, so the used copies' `Zone Out`s are tested to stay silent.
9. **At S + 47:00:** Esc, Finish Mission, then leave the server as in A0 (Leave Server on the results screen within about 15 s).
10. **Copy the logs** with `cp -p` to **`src/testdata/missionlog/real_47/`** (the next free; check first). Copy the Tacview `.acmi` from `data\Tracks\` to `C:\Machine Intelligence\P14 Flight A2\`.

**If you are shot down:**
- Each copy whose `Zone Out` is armed ends about 3 s later (`Zone Out`, `DONE HUB`; A1 finding 2).
- Note the time, leave as in step 9, and copy the logs. There is no respawn on this PC.

**If you skip the orbit and fly straight on:**
- A, B and C go HOT and then COLD, and C is refused once (P5).
- `SN01` and `SN04` end by `Zone Out` when you are 35 km past their areas, at about S + 7:36 and S + 9:36 (`DONE HUB` with no `ON STATION`), and are deleted 60.6 s later.
- C never asks again.

## Predictions

S is the player's spawn time (the `AType:10` line), s a copy's `START` time. Times assume 450 km/h true from the spawn. For speed-dependent rows, check the relation in the "How to confirm" column; the S + times are only a guide. A1 measured breadcrumbs about 0.06 s after the nominal time, a pass every H to the hundredth (120.00 s), and a failed check H + 2.04 s after the last pass.

| Id | Prediction | How to confirm |
|---|---|---|
| **P1** | Loads; `MFile` names `P14_Acceptance_A2_20261028`; no loader error. The DPRK spawn list offers "P14 A2 DPRK air start" (Yak-9P, air). The player is country 501. | log header; screen; `AType:10` |
| **P2** | **Each area goes HOT** once flown into: `THNaa INNER` 80.1 s after the ring is crossed. S + 192.1 (A), 312.1 (B), 432.1 (C). Gaps between them: 120 s at 450 km/h (15 km). | `INNER` times; ring crossing from Tacview (8 km from the centre) |
| **P3** | **A and B go COLD after the plane leaves:** `THN01 CLOSE DELAY` at THN01 INNER + 162.04 s (≈ S + 354.2), `THN02 CLOSE DELAY` at THN02 INNER + 162.04 (≈ S + 474.2). No second `INNER` for A or B. | breadcrumbs |
| **P4** | **Starts:** at each INNER, `ZNaa CHANCE` + 0.10, `ZNaa FIRE` + 0.15. At A and B, the copy's `START` in the same tick: `SN01 START` (A), `SN04 START` (B). Each `START` brings up its 4 aircraft (`AType:28`) 0.5 s later at the copy's spawn point (table above). After `SN04 START`, `BUD N COUNT` at + 0.25 s (two flying). | breadcrumbs, `AType:28` |
| **P5** | **The first budget refusal (C):** at `THN03 INNER` (≈ S + 432.1), `ZN03 FIRE` at + 0.15 with **no `SN07 START`**, and `ZN03 DENY WAIT` 1.0 s after the FIRE. `ZN03 COOLDOWN` 300 s after the DENY WAIT. | breadcrumbs |
| **P6** | **Transit:** each copy's `Zone IN` at s + 164 (HVAR) or s + 208 (Close Support), −20 / +30 s; the copy's few seconds of orders before it heads for its waypoint are inside that window. A1's Il-10s came 18 s early. The player is within 35 km of the area each time (22 km from A, 15 from B, over C), so `Zone IN` fires at the copy's arrival. | `Zone IN` (6110, 6440, 6770) minus `START` |
| **P7** | **C stays HOT while you orbit:** `THN03 INNER` every 160.00 s from the first (S + 432.1, 592.1, 752.1, 912.1, 1,072.1, …). No `THN03 CLOSE DELAY` before you leave. | breadcrumbs |
| **P8** | **Second refusal (C):** at the first `THN03 INNER` after `ZN03 COOLDOWN` (INNER_C + 320 s, ≈ S + 752.1), `ZN03 FIRE` and `ZN03 DENY WAIT` 1 s later, no START. SN01 and SN04 both still fly. `ZN03 COOLDOWN` again 300 s later. | breadcrumbs |
| **P9** | **A and B end COLD, no extension:** `SN01 ON STATION` at SN01 Zone IN + 600.5 (≈ S + 957). `SN01 COLD` and `SN01 DONE HUB` in the same tick, no `SN01 EXTEND`. Same for SN04 at Zone IN + 900.5 (≈ S + 1,421). | breadcrumbs |
| **P10** | **Deletion:** `SNjj DELETE DELAY` at that copy's DONE HUB + 60.6 s (SN01 ≈ S + 1,017.5; SN04 ≈ S + 1,481.6). `ZN01` / `ZN02 COOLDOWN` 300 s after their copy's DONE HUB; their areas are COLD, so nothing follows. | breadcrumbs |
| **P11** | **Third ask granted (C):** at INNER_C + 640 s (≈ S + 1,072.1), after `SN01 DELETE DELAY`: `ZN03 FIRE` and **`SN07 START`** in the same tick, no `ZN03 DENY WAIT`. `BUD N COUNT` within 0.5 s (SN04 and SN07 fly). Margin: SN01 is deleted about 55 s before this pass. | breadcrumbs |
| **P12** | **The single extension, then home (C):** `SN07 Zone IN` ≈ s + 164 (you are over C). `SN07 ON STATION` at Zone IN + 600.5 (≈ S + 1,837) with **no `SN07 COLD`** (C is HOT). **`SN07 EXTEND` and `SN07 DONE HUB` in the same tick, 600 s later** (≈ S + 2,437). The F-80s turn for home. No second extension. `SN07 DELETE DELAY` 60.6 s after. | breadcrumbs; Tacview (the F-80s leave C) |
| **P13** | **Decision 0038: a used copy is quiet.** After each copy's `DONE HUB`, nothing from that copy logs except its `DELETE DELAY` (and `ZNaa COOLDOWN`, which is the zone's). In particular, none of these logs, although each would without 0038: `SN01 HARD STOP` (≈ S + 2,156); `SN04 HARD STOP` (≈ S + 2,620); `SN01`, `SN04` or `SN07 Zone Out` when you fly north out of their 35 km (≈ S + 2,644 / 2,753 / 2,780); a second `Zone IN`; any `EXTEND`, `ON STATION` or `COLD`. | the copy's group in the Replay timeline |
| **P14** | **After you leave C:** one more `THN03 INNER` (≈ S + 2,512, still inside 8 km), then `THN03 CLOSE DELAY` (≈ S + 2,674; C COLD). `ZN03 COOLDOWN` ≈ S + 2,737, with no pass after it: no `SN08 START`. | breadcrumbs |
| **P15** | **Budget never above two:** never three NATO copies between a `START` and the next `DELETE DELAY`. `SN02`, `SN03`, `SN05`, `SN06`, `SN08` and `SN09` never start. No `HARD STOP`, no `LOSSES` (unless something shoots the F-80s or F-51s). | breadcrumbs |
| **P16** | **Decision 0037:** the file holds no AI aircraft on 601 or 501 (checked by the build test). In the log, the only 501 aircraft is the player, and every NATO aircraft is 602. The Map's NATO fighter pack never activates (no `AType:28` for it): its zones are 109 km or more from the line. No DPRK breadcrumb exists (there is no DPRK side). | log `COUNTRY:` fields; build test |
| **P17** | **Replay:** no dead links. Never fired: SN02, 03, 05, 06, 08 and 09, the fighter pack's logic, `ZN01` / `ZN02 DENY WAIT`, and every `HARD STOP` and `LOSSES`. | Replay window |

### Risks

- **The F-80s or F-51s may attack the Yak.** They carry ground-attack orders (`AttackGTargets 1`, `AttackAir 0`), but they are fighters and pass within 10–20 km of the player. If the player dies, each armed `Zone Out` fires about 3 s later.
- **Orbit drift.** If the orbit drifts more than 8 km from C at a check, `THN03 CLOSE DELAY` logs (C COLD), the trigger re-arms, and the next entry gives a new `INNER` 80.1 s later. Later C times then shift. A drift more than 5 km east of C (35 km from A) also lets `SN01 Zone Out` fire before its on-station time ends (SN01 then ends early and is deleted sooner; P9 for A changes, P11 can come at the second ask).
- **Speed.** Faster than about 500 km/h true shrinks P11's margin (SN01 must be gone before C's third ask); slower is fine.
- **Server load:** at most 8 AI aircraft fly at once, so the mission should run at about 1.0× wall time (flight 2F).

## Replay

- **In the app:**
  1. Header **Replay…**.
  2. **Pick .Group…** `handoff/p14-probes/acceptance-a2/P14_Acceptance_A2.Group`. The sidecar `.trace.json` is found next to it.
  3. **Log folder…** `src/testdata/missionlog/real_47`.
  4. **Run replay**, then read Dead links, Never fired, Fired and Timeline. Filter the Timeline by group (`… / Air Tasking NATO Sorties / SN07 …`) for P13.
  5. After Close, the Map's Replay layer shows the spawns and kills.
- **Command line (fallback):**
  ```
  cargo run --release -- replay --group handoff/p14-probes/acceptance-a2/P14_Acceptance_A2.Group --trace handoff/p14-probes/acceptance-a2/P14_Acceptance_A2.trace.json --logs src/testdata/missionlog/real_47 --out target/p14/acceptance_a2_report.md
  ```

## Checks (2026-10-28, branch `claude/p14-acceptance-a2`)

- `cargo test --offline`: **1031 passed, 0 failed, 33 ignored** (main: 1029 / 32; two new tests and the ignored writer). `existing_outputs_match_baseline` and `map_generate_without_air_matches_baseline` pass.
- Warnings: **81 build, 306 clippy** (unchanged).
- Code review (two axes, Standards and Spec). Fixed:
  - `build_a2` uses `generate_map`;
  - Place NATO is one helper shared with the 0037 test;
  - checks that compared constants now read the built file;
  - the pack-zone check counts what it checked;
  - no second distance helper;
  - `A2_STEM` named once per module;
  - a sturdier spawn-list scan;
  - honest comments on what is clicked and what is set;
  - four run-sheet corrections (Zone Out timing, the spacing limit, the orbit drift, the orbit radius).
  - Kept: the A2 facts are checked in both the UI build test and the frozen-file test, as for A1 (one checks what the app builds, the other what was frozen).
- No game fact was learned (nothing was flown), so `docs/il2-loader-and-log-facts.md` is unchanged.

## Results (flown 2026-10-29)

Flown by Claude on this PC (game `261013`), keyboard only, Tacview on, from the frozen mission `P14_Acceptance_A2_20261028` in the Dogfight folder.

- **Run 1**, session `2026-10-29_09-36-50`, logs `src/testdata/missionlog/real_48/` (9 files), Tacview `handoff/tacview/P14_Acceptance_A2_run1.zip.acmi`.
  - The air start began with the level-flight helper on.
  - The first LShift+A, meant as a check, switched it off. The Yak rolled right into a climbing turn and then a spiral dive, which the arrow keys could not stop. It hit the ground at 176.8 s, 3.5 km from the spawn.
  - No Air Tasking breadcrumb fired: the player never reached A's ring.
  - Leave Server took the game to the results screen, and the server reloaded the mission into a new session.
- **Run 2**, session `2026-10-29_09-42-07`, logs `src/testdata/missionlog/real_47/` (109 files), Tacview `handoff/tacview/P14_Acceptance_A2_run2.zip.acmi` (the raw `.acmi` is also in `C:\Machine Intelligence\P14 Flight A2\tracks\`).
  - Spawn S = 43.32 s.
  - The helper was left alone; LCtrl+R started Tacview.
  - The throttle went from 85 % down to 42 %, at about 450–500 km/h indicated, 2,000 m, heading 089.
  - Then the orbit at C, a return to it, and the exit north. Left at about 3,050 s.
  - Mission time kept pace with wall time.
- **Replay** (`replay` command on `real_47`, report `target/p14/acceptance_a2_report.md`, not committed): **no dead links**.

### Run 2, prediction by prediction

| Id | Flown |
|---|---|
| P1 | **yes**: loads; `MFile` names the mission; the DPRK spawn list offered "P14 A2 DPRK air start" (Yak-9P, air start); the player is country 501 at (190,000; 258,037), 1,999 m |
| P2 | **yes**: `THN01 INNER` 217.54 (S + 174.2), `THN02 INNER` 312.76 (+ 95.2 s), `THN03 INNER` 414.12 (+ 101.4 s). The plane flew faster than planned (about 520–580 km/h true), hence S + 174 not S + 192 |
| P3 | **yes, to the hundredth**: `THN01 CLOSE DELAY` 379.58 = INNER + 162.04; `THN02 CLOSE DELAY` 474.80 = INNER + 162.04 |
| P4 | **yes**: `ZNaa CHANCE` + 0.10, `ZNaa FIRE` and `SNjj START` + 0.14 (`SN01` at A, `SN04` at B); `BUD N COUNT` 313.14, 0.24 s after `SN04 START` |
| P5 | **yes: the first budget refusal in the game.** `ZN03 FIRE` 414.26 with no START; `ZN03 DENY WAIT` 415.26 (+ 1.00) |
| P6 | **yes**: `Zone IN` SN01 384.18 (s + 166.5), SN04 532.94 (s + 220.0), SN07 1,471.72 (s + 166.1) |
| P7 | **Not as planned.** The orbit started about 3 km east of C and went round on its east side. The 574 s check found the player more than 8 km out: `THN03 CLOSE DELAY` 576.16, and the trigger re-armed. Two later entries failed their checks (685.56, 957.54): one was made while turning away; for the other, the player was already inside when the trigger re-armed (flight 2J: not reported). After flying 16 km out and back in, `THN03 INNER` 1,305.52, then **every 160.00 s, ten in a row**, to 2,745.52 |
| P8 | **Not seen.** SN01 had ended early (P9), so the budget had room. C was COLD when `ZN03` re-armed, and `ZN03` was disarmed again 0.88 s after re-arming (see "Not predicted") |
| P9 | **SN04: yes, the first clean COLD end.** `SN04 ON STATION` 1,433.44 = Zone IN + 900.50, with `SN04 COLD` and `SN04 DONE HUB` in the same tick and no EXTEND. **SN01 ended by `Zone Out` at 431.64**: the start of the orbit took the player more than 35 km from A (the risk the run sheet lists) |
| P10 | **yes**: `DELETE DELAY` at DONE HUB + 60.60 for all three (SN01 492.24, SN04 1,494.04, SN07 2,732.82); `ZN01 COOLDOWN` 731.64 and `ZN02 COOLDOWN` 1,733.44, each at its copy's DONE HUB + 300.00 |
| P11 | **yes**, at the first C pass after `ZN03` was armed (1,305.52, after the return to C): `ZN03 FIRE` and **`SN07 START` 1,305.66**, no DENY WAIT; `BUD N COUNT` 1,306.04 (SN04 and SN07) |
| P12 | **yes: the single extension, then home, first seen in the game.** `SN07 ON STATION` 2,072.22 = Zone IN + 600.50, no `SN07 COLD`; **`SN07 EXTEND` and `SN07 DONE HUB` 2,672.22**, 600.00 s later; no second extension |
| P13 | **yes (decision 0038).** After each copy's DONE HUB only its `DELETE DELAY` logged. Not logged, though each was due without 0038: `SN01 ON STATION` (984.7), `SN01 HARD STOP` (2,181.3), `SN04 HARD STOP` (2,620.6), and every `Zone Out` of SN01, SN04 and SN07 when the player flew north out of their 35 km (by about 3,000 s). Replay lists them as unobserved successors |
| P14 | **yes**: last `THN03 INNER` 2,745.52, `THN03 CLOSE DELAY` 2,907.56 (+ 162.04); `ZN03 COOLDOWN` 2,972.22 (SN07 DONE HUB + 300.00); no `SN08 START` |
| P15 | **yes**: at most two NATO copies at once (SN01 + SN04, then SN04 + SN07); SN02, 03, 05, 06, 08, 09 never started; no `LOSSES`, no `HARD STOP`; no AI aircraft was lost |
| P16 | **yes (decision 0037)**: every AI aircraft in the log is 602 (F-80C-10, F-51D); the only 501 objects are the player's Yak and pilot. The Map's NATO fighter pack never spawned (its flights show 0 in Never spawned) |
| P17 | **yes**: no dead links |

### Not predicted

- **A deactivated running timer is paused, not cancelled: activating it again resumes it.**
  - At each successful roll, `ZNaa CHANCE` deactivates `ZNaa MISS WAIT` (1 s) about 0.1 s after the pass started it.
  - When the zone re-arms (`ZNaa ARM`, 300 s later), `MISS WAIT` fires **0.88 s after ARM**: the rest of its 1 s. Seen four times: `ZN03` 716.14, `ZN01` 732.52, `ZN02` 1,734.32, `ZN03` 2,973.10.
  - `MISS WAIT` then disarms the zone and starts another cooldown (`ZN03 COOLDOWN` 1,016.14, `ZN01` 1,032.52, `ZN02` 2,034.32). So after every FIRE, granted or refused, a zone entry stays shut for **two cooldowns, not one**.
  - The zone front end (decision 0030, "direct cancels (J1)") assumed a cancel. Flight 0's J1 saw only that a deactivated timer never fired; it was never re-activated.
  - Any design that deactivates a running timer and later re-activates it needs a check. Candidates: `ZNaa GRANTED` → `DENY WAIT` (re-armed by `FIRE ARM`), and the clock's `WIN` → `ATTEMPT END` (re-armed by `REROLL ON`).
  - Not fixed in this branch: a follow-up for the user.
- **`ZNaa ARM` (an `MCU_Activate`) never logs its breadcrumb**, while `COOLDOWN` does. The trace build seems to *activate* the objective that an Activate targets, rather than trigger it. Replay lists `ZNaa ARM` as unobserved. A trace limitation, not a logic fault.
- **Orbiting by helper turns at about 450 km/h indicated:** LAlt+X / LAlt+Z about 3°/s, radius about 2.7 km. The orbit held C for 25 minutes once it was centred on the area; the F-80s' target circle on the minimap showed where C was.
