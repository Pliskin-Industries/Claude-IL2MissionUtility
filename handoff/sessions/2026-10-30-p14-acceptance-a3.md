# 2026-10-30 — P14 acceptance, flight A3 (Map air start, NATO clock launch, two sorties end together): run sheet

**Status: flown 2026-10-30 by Claude (two runs; all predictions met in run 2; results at the end).** Branch `claude/p14-acceptance-a3`, cut from `main` at `8e33e0b`. The user approved the design ("Build it", 2026-10-30) and chose that Claude flies it by keyboard, as in A2.

**Why this flight.** It covers the three open items of P14 step 11 (`handoff/P14-air-tasking.md` §8):
1. **A DPRK player on the app's own Map air start** (`placed_air_starts`, written by Map Generate, no `--spawns`). Since decision 0042 (A) it writes country 501. The player must log as 501, and the NATO area must see them.
2. **A NATO clock launch:** an `ENjj GO` that starts a copy (decision 0029's clock, first time in the game with a pilot).
3. **Two sorties on one target that end together:**
   - two NATO copies (one zone copy, one clock copy) are over the same area;
   - the player leaves beyond 35 km;
   - both `DONE HUB`s log in one tick (`Zone Out`), and both copies are deleted 60.6 s later in one tick;
   - a later zone trigger still starts a sortie on that side.

## Files

- **Code (test code only):**
  - `src/ui_tests.rs`:
    - `build_a3` builds the base map on the Map tab and generates it through `generate_front_file`, the Generate button's own call. The objective, the air start, the pool entries and the settings are set on the app directly, as `build_a2` does.
    - The ignored writer `write_p14_acceptance_a3`.
    - The check `a3_map_build_matches_the_frozen_files_and_the_run_sheet`:
      - a byte check against the frozen files;
      - the build facts in "Timing read from the file".
      - A3 was flown on 2026-10-30, so its files stay as flown. If the trace build changes, the byte check is dropped and the facts stay, as A2's were.
  - `src/airtask/export_acceptance_flights.rs`: `frozen_a3_files_parse_and_links_resolve` (parse and links). It also checks:
    - six `DONE HUB OFF`s;
    - one Airfield, on 501, outside twice the trigger radius;
    - one trigger, on 501.
  - No product code changed.
- **What `build_a3` does in the app:**
  - Map tab, default date **1951-04-22**, built-in terrain heights.
  - One **NATO objective A** at (190,000; 280,000), A2's area A. No DPRK objective, no arrow, **no fighter pack**.
  - One **Map air start**, "P14 A3 DPRK air start":
    - a DPRK bank field (`default_field(…, false)`), aircraft changed to **Yak-9P** (`apply_type`);
    - air start at **2,000 m**, fuel 100 %, heading **090**;
    - at (187,500; 258,000).
  - Two pool entries on A:
    - **EN01 F-80C HVAR Strike, Zone** (copies SN01–03);
    - **EN02 F-51D Close Support, Clock** (copies SN04–06).
  - Air Tasking settings:
    - Threshold R **16 km**;
    - zone chance **100 %**;
    - zone cooldown **5 min**;
    - NATO clock: first slot 10 min and period 20 min (the defaults), **one jitter step, quiet 0 %**;
    - budget 2 / 2 (default);
    - seed 7.
  - Trace build on; Generate Base Map.
- **Generate:** `cargo test --offline write_p14_acceptance_a3 -- --ignored` writes these to `target/p14/`:
  - `P14_Acceptance_A3.Group` (683,978 bytes);
  - six language files (392 bytes each);
  - `.trace.json` (37,747 bytes, 78 breadcrumbs).
  - A second run gives the same bytes. There is no `.spawns.json`: the player spawn is in the `.Group`.
- **Frozen:** `handoff/p14-probes/acceptance-a3/`: `P14_Acceptance_A3.Group`, `.eng`, `.trace.json`.
- **Mission build** (from the frozen copies, no resaver, no `--spawns`):
  ```
  python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe handoff/p14-probes/acceptance-a3/P14_Acceptance_A3.Group --no-resave --name P14_Acceptance_A3_20261030 --title "P14 acceptance A3 (Map air start, NATO clock, two sorties end together)" --out "C:\Machine Intelligence\P14 Flight A3\mission"
  ```
  - Output: `P14_Acceptance_A3_20261030.Mission` (685,851 bytes) and `.eng` (792 bytes). The air start's Airfield and its Planes list pass through unchanged (country 501, Yak-9P, `StartType 0`, altitude 2,000).
  - **Copied** into `C:\Program Files\IL2Series\game\data\Multiplayer\Dogfight\` on 2026-10-30 (identical bytes).
- The `.Mission` takes its Options block from the game's dogfight test mission, as A0–A2 did.

## The map

All coordinates are game metres (x north, z east).

```
 north (x+)
   ^
   |                      ( A )  190,000; 280,000   r_in 8 km, R 16 km, copies' Zone IN/Out 35 km
   |                        ^  left orbit, centred on A
   |   spawn *--- heading 090 --+   (x = 187,500, 2.5 km south of A)
   |   187,500; 258,000      abeam A at 22 km
   |
   |                                 SN01–03 spawn 170,351; 302,669 (30.0 km, heading 311° to A)
   |                                      SN04–06 spawn 161,617; 312,745 (43.3 km, heading 311° to A)
 front (x ≈ 163,000 at A)
```

| | Position (x; z) | From A | Notes |
|---|---|---|---|
| A (THN01, ZN01, clock area 01) | 190,000; 280,000 | — | 27 km north of the front |
| Air start | 187,500; 258,000 | 22.1 km | Outside R (16 km) and the entry trigger (8 km). A spawn inside an area is not seen (A1). |
| SN01–03 (HVAR, zone) | 170,351; 302,669 | 30.0 km | Zone spawn distance 30 km |
| SN04–06 (Close Support, clock) | 161,617; 312,745 | 43.3 km | Clock spawn distance: 520 km/h × 5 min |

- **On the minimap** (10 km squares; the column 23/24 line is at z ≈ 258 km, the row 28/29 line at x ≈ 190 km):
  - The spawn sits on the 23/24 column line, 2.5 km south of the 28/29 row line.
  - A is on the 28/29 row line, 2 km east of the 25/26 column line.
  - The F-80s' and F-51s' target circle also marks A.
- **Heights:** the HVAR copies fly at 3,050 m and the Close Support copies at 1,500 m. **The player flies at 2,000 m.**
- **No Map fighter pack**, and the Map writes no AI aircraft outside Air Tasking. The only Airfield in the file is the air start.

### Timing read from the file

| Part | Value |
|---|---|
| Entry trigger `THN01 TRIGGER` | Complex Trigger, radius 8,000 m, **country 501**, at A |
| Threshold | DWELL 80 s; `INNER PULSE` 0.1 s; HOLD 160 s; CLOSE 2 s; CLOSE DELAY 0.05 s; `INNER` check zone 8,000 m |
| Zone `ZN01` (EN01 only) | `CHANCE` 0.10 s at Random **100 %**; FIRE 0.05 s; DENY WAIT 1 s; MISS WAIT 1 s; **COOLDOWN 300 s**. `SN01 START` → `ZN01 GRANTED`; `SN01 DONE HUB` → `ZN01 WAITING` → COOLDOWN → ARM. |
| NATO clock (EN02 only) | `FIRST` **600 s**; `PERIOD` **1,200 s**; one `JITTER 1` at 0.10 s, Random 100 % (no quiet); `NO REPEAT` 0.1 s; `DRAW` 0.30 s; `ATTEMPT END` 1.50 s; `SLOT CLOSE` 16.50 s. EN02 is drawn only while area 01 is HOT. Slots open at **600.1, 1,800.1, 3,000.1**. |
| No-repeat (LAST) | `DRAW` clears every entry's LAST. `SN04–06 START` sets EN02's LAST. At the next slot's `NO REPEAT` (+0.1 s), a set LAST excludes EN02 from the draw. |
| Budget, NATO | 2 copies (`BUD N COUNT` Counter 2); recount `END` 0.45 s |
| HVAR copy (SN01–03) | 4 × F-80C; 30 km at 660 km/h, so ≈ 164 s to A; on station 600 s; EXTEND 600 s; HARD STOP 1,963.64 s |
| Close Support copy (SN04–06) | 4 × F-51D; 43.3 km at 520 km/h, so ≈ 300 s to A; on station 900 s; EXTEND 600 s; HARD STOP 2,400 s |
| Each copy | `Zone IN` and `Zone Out` 35 km at A; `Zone IN` arms `Zone Out`; `DONE HUB` → `DELETE DELAY` 60.6 s (A2); `DONE HUB OFF` switches off the copy's timers and zones (decision 0038) |

### Choices, and why (approved by the user)

- **The clock: one jitter step and quiet 0 %, not the default three steps and 20 % quiet.**
  - Decision 0029: about 20 % of slots stay quiet, and the jitter puts the slot at 0, 3 or 6 min after the slot time. With the defaults, slot 1 opens at 600.1, 780.1 or 960.1 s, or not at all (1 in 5). The leave cue would move by up to 6 min, and one run in five would need slot 2 (≈ 50 min of flying).
  - With quiet 0 % and one step, **slot 1 opens at 600.1 s for certain** (weights [100, 0]; `clock.rs` accepts quiet 0 with one step). This makes it possible to plan when both copies are over A.
  - **Slot 2 (1,800.1) is the backup.** If slot 1 does launch, slot 2 should be quiet, because `SN04 START` set EN02's LAST (the no-repeat rule). If slot 1 is lost (A COLD at 600 s), slot 2 launches EN02 instead.
  - **Not tested by A3:** the 20 % quiet roll and the jitter steps after the first. Both are plain `Random` timers on the same `TICK`; flight 2L ran five slots with the same one-step, no-quiet settings.
  - Rejected:
    - The defaults with a longer flight: the timeline could not be planned, and a 1-in-5 quiet slot 1 costs 20 min.
    - Two clock entries: slot 2 would then launch the other one, hiding the LAST rule.
- **Two entries (EN01 zone, EN02 clock), not one Both entry.** With one entry, a zone start would set its LAST, and slot 1 right after it would be quiet (the `ENjj LAST` rule). Separate entries keep the zone start and the clock launch independent.
- **One area.** Two copies "on one target" must share A, so that one `Zone Out` radius ends both in the same tick.
- **Zone chance 100 %, cooldown 5 min, R 16 km**, as in A2: a certain first start, a short wait before the later start, and room for orbit drift.
- **HVAR (600 s on station) for the zone and Close Support (900 s) for the clock.** Neither carries air-to-air orders.
- **Leave after `SN01 ON STATION`** (≈ 996 s), not as soon as both copies are over A (≈ 915 s). Leaving earlier could turn A COLD before SN01's on-station time ends. SN01 would then end by `COLD` instead of `Zone Out`, and the two copies would not end together.
- **A Yak-9P at about 500 km/h true** (about 450 km/h on the dial at 2,000 m). In A2, 42 % throttle gave 520–580 km/h true, and the first `INNER` came at S + 174, not the planned S + 192. So expect the leg-in times below to run 5–15 % early. Everything after P is timed from the log, not the stopwatch. The air start is the Map's own, so its type and height are set in the bank entry, not a spawn list.

## Pilot steps

Times are from the spawn, **S** (stopwatch), at **500 km/h true**. Mission time from the log: s = (T − 3) / 50. In A2 the spawn came at 43.32 s; this sheet assumes S ≈ 45 s.

1. **Before starting the game:**
   - `tacviewrecord = 1` under `[KEY = track_record]`, with the game closed (the game rewrites `startup.cfg` on exit);
   - `mission_text_log` stays on.
2. **Host from the client** as for A2: Multiplayer, Create, "Available from internet" off, `P14_Acceptance_A3_20261030` alone in the rotation, "TacView record" ticked.
3. **Spawn:** DPRK side, **"P14 A3 DPRK air start"**, Yak-9P (air start, 2,000 m, heading 090). Start the stopwatch at the spawn. **One spawn only:** this PC cannot spawn twice in one session.
4. **At once:**
   - **LCtrl+R** starts Tacview (red icon top right).
   - **Leave the level-flight helper alone.** The air start may begin with it on, and a first LShift+A then switches it off (A2 run 1 crashed this way). Check the helper's on-screen state before pressing anything.
   - Hold heading 090. Throttle to about **42 %** (about 450 km/h on the dial).
5. **The leg in:**

   | S + | Mission s (S ≈ 45) | Where | What the log should show |
   |---|---|---|---|
   | 1:44 | ≈ 149 | enter A's 8 km ring | — |
   | 2:38 | ≈ 203 | **abeam A** (A 2.5 km to the left, on the 28/29 row line) | — |
   | 3:04 | ≈ 229 (**P**) | in the orbit | `THN01 INNER` (A HOT); `ZN01 CHANCE`, `ZN01 FIRE`, `SN01 START` |

6. **Abeam A (S + 2:38), start a steady left orbit:** the helper on, **LAlt+Z held**, about 3°/s, radius about 2.7 km. The centre is then about 0.2 km from A.
   - Check every minute on the minimap that the orbit stays round A (the copies' target circle). The presence check counts 8 km.
7. **Orbit until the leave cue.** On the way, the log should show:
   - `SN01 Zone IN` ≈ 395;
   - **`CLK N SLOT OPEN` 600.1, `EN02 GO` and `SN04 START` ≈ 600.9** (the clock launch);
   - `SN04 Zone IN` ≈ 901–926.
8. **Leave cue:** `SN01 ON STATION` (≈ 996) with **no `SN01 COLD`**, and `SN04 Zone IN` already logged. Without a live log, leave at mission time **1,000 s** (16:40).
   - Roll out of the orbit on **heading 000 (north)** and fly straight, 2,000 m, same throttle.
9. **About 4 minutes later (≈ 1,230–1,270), 35 km from A:** both copies end. **`SN01 Zone Out`, `SN04 Zone Out`, `SN01 DONE HUB` and `SN04 DONE HUB` in one tick.** Call that time **DH**.
10. **Fly on north for 30 s after DH, then turn left through 180°** (LAlt+Z held, about 60 s) and fly back to A (heading about 180; steer for the copies' old target circle, or the 28/29 row line 2 km east of the 25/26 column line).
    - At DH + 60.6: **`SN01 DELETE DELAY` and `SN04 DELETE DELAY` in one tick.**
    - At DH + 300: `ZN01 COOLDOWN` and `ZN01 ARM`.
    - At 500 km/h you enter A's 8 km ring at about DH + 315. The zone re-arms at DH + 300 and is fed by `INNER`, which comes 80 s after the ring. So any ring entry after about DH + 220 finds ZN01 armed: the return has more than 90 s to spare.
11. **Back over A, orbit left again.** About 80 s after the ring: `THN01 INNER`, then `ZN01 CHANCE`, `ZN01 FIRE` and **`SN02 START`** (the later start), ≈ DH + 380–400 ≈ 1,630.
12. **Stay in the orbit until 1,830 s** (30:30): **`CLK N SLOT OPEN` at 1,800.1 with no `EN02 GO`** (slot 2 quiet; EN02 is LAST).
13. **At about 1,840 s:** Esc, Finish Mission, then leave the server as in A0 (Leave Server on the results screen within about 15 s).
14. **Copy the logs** with `cp -p` to **`src/testdata/missionlog/real_51/`** (the next free; check first). Zip the `.acmi` from `data\Tracks\` into `handoff/tacview/P14_Acceptance_A3.zip.acmi`. The `.acmi` is locked while Tacview records; copy it after leaving.

**Live breadcrumb check** (`AType:8` lines with `POS(x,0.0000,5000.0000)`):

| Breadcrumb | x |
|---|---|
| `CLK N SLOT OPEN` / `EN02 GO` | 5000 / 5010 |
| `ZN01 CHANCE` / `MISS WAIT OUT` / `FIRE` / `DENY WAIT` / `COOLDOWN` / `ARM` | 5020 / 5030 / 5040 / 5050 / 5060 / 5070 |
| `THN01 CLOSE DELAY` / `INNER` | 5080 / 5090 |
| `BUD N COUNT` | 5100 |
| NATO logic Mission Begin | 5110 |
| SN01: Mission Begin, `Zone IN`, `Zone Out`, `START`, `HARD STOP`, `ON STATION`, `COLD`, `EXTEND`, `LOSSES`, `DONE HUB`, `DELETE DELAY` | 5120 to 5220 (step 10) |
| SN02, SN03 (same order) | 5230–5330, 5340–5440 |
| SN04, SN05, SN06 (same order) | 5450–5550, 5560–5660, 5670–5770 |

So the leave cue is x = 5170 (SN01 ON STATION) with no 5180 after it. The end is 5140 + 5470 (Zone Out) and 5210 + 5540 (DONE HUB), all in one tick.

**If you are shot down:** each copy whose `Zone Out` is armed ends about 3 s later (A1 finding 2). Note the time, leave as in step 13, and copy the logs. There is no respawn on this PC.

## Predictions

S is the player's spawn time (the `AType:10` line). **P** is the first `THN01 INNER`, s is a copy's `START`, and DH is the tick of the two `DONE HUB`s. Times assume 500 km/h true; for speed-dependent rows, check the relation in "How to confirm". The game logs on a 0.02 s tick.

| Id | Prediction | How to confirm |
|---|---|---|
| **Q1** | Loads; `MFile` names `P14_Acceptance_A3_20261030`; no loader error. **The DPRK spawn list offers "P14 A3 DPRK air start"** (Yak-9P, air start), the Map's own air start. **The player logs as country 501** at about (187,500; 258,000), 2,000 m. | log header; screen; `AType:10` `COUNTRY:501` |
| **Q2** | **The NATO area sees the Map air start's player:** `THN01 INNER` 80.1 s after the 8 km ring is crossed (P ≈ S + 184). `ZN01 CHANCE` at P + 0.10, then `ZN01 FIRE` and `SN01 START` at P + 0.14. Four F-80C (602) appear at (170,351; 302,669). | breadcrumbs; ring crossing from Tacview; `AType:28` |
| **Q3** | **A stays HOT while you orbit:** `THN01 INNER` every 160.00 s from P, with no `THN01 CLOSE DELAY` before you leave. No `ZN01 CHANCE` at these passes (ZN01 is shut while SN01 flies). | breadcrumbs |
| **Q4** | **The NATO clock launch:** `CLK N SLOT OPEN` at **600.10**; `EN02 GO` at about 600.90 (WIN at `DRAW` + 0.5); `SN04 START` in the same tick or the next; `BUD N COUNT` within 0.5 s (SN01 and SN04 fly). Four F-51D (602) appear at (161,617; 312,745), 43.3 km from A. | breadcrumbs; `AType:28` |
| **Q5** | **Transit:** `SN01 Zone IN` at s + 164 (A2: 166.5), so ≈ P + 166. `SN04 Zone IN` at s + 300, −10 / +25 s (the clock's 5 min arrival; A2's Close Support copy came 12 s late), so ≈ 901–926. | `Zone IN` (5130, 5460) minus `START` |
| **Q6** | **SN01 extends:** `SN01 ON STATION` at SN01 Zone IN + 600.50 (≈ 996), with **no `SN01 COLD`** (A is HOT). Its `EXTEND` timer starts but never logs (Q9). | breadcrumbs |
| **Q7** | **Two sorties on one target end together:** after you fly north, at 35 km from A, **`SN01 Zone Out` and `SN04 Zone Out`, then `SN01 DONE HUB` and `SN04 DONE HUB`, all in one tick (DH)**, about 230–275 s after you leave. No `SN04 ON STATION`, no `SN04 COLD`. | breadcrumbs 5140, 5470, 5210, 5540; your distance from A in Tacview |
| **Q8** | **Deleted together:** `SN01 DELETE DELAY` and `SN04 DELETE DELAY` in one tick, **DH + 60.60**. All eight AI aircraft leave the log then. `BUD N COUNT` does not log (no copy flies). | breadcrumbs 5220, 5550; Tacview |
| **Q9** | **Decision 0038, a used copy is quiet:** after DH, nothing from SN01 or SN04 logs except `DELETE DELAY`. In particular, none of these logs: `SN01 EXTEND` (would be ≈ ON STATION + 600 ≈ 1,597); `SN04 ON STATION` (≈ SN04 Zone IN + 900.5 ≈ 1,810); any second `Zone IN` when you fly back within 35 km; either `HARD STOP` (SN01 ≈ P + 1,964; SN04 ≈ 3,001). | the copies' groups in the Replay timeline |
| **Q10** | **A goes COLD after you leave:** `THN01 CLOSE DELAY` 162.04 s after the last `THN01 INNER` you were within 8 km for (≈ 1,191: at the ≈ 1,029 check you are still about 7.5 km from A, so it normally passes). This does not end either copy; only `Zone Out` does. | breadcrumbs |
| **Q11** | **The zone re-arms after one cooldown** (decisions 0044, 0045): `ZN01 COOLDOWN` and `ZN01 ARM` at **DH + 300.00**; no `ZN01 MISS WAIT OUT`, and no second `ZN01 COOLDOWN`. | breadcrumbs 5060, 5070, 5030 |
| **Q12** | **A later zone start on the same side:** back in A's ring, `THN01 INNER` 80.1 s after re-entry (≈ DH + 395). Then `ZN01 CHANCE`, `ZN01 FIRE` and **`SN02 START`** (+0.10, +0.14). There is **no `BUD N COUNT`**: it logs only at two copies, and SN02 flies alone (SN01 and SN04 were deleted at DH + 60.6). | breadcrumbs |
| **Q13** | **Slot 2 is quiet (the no-repeat rule):** `CLK N SLOT OPEN` at **1,800.10**, A HOT, SN02 flying (so the budget has room), and **no `EN02 GO`, no `SN05 START`**. `SN04 START` (600.9) came after slot 1's `DRAW`, so EN02's LAST is set at slot 2's `NO REPEAT`. *If slot 1 did not launch (A COLD at 600 s), slot 2 launches EN02 instead: `EN02 GO` ≈ 1,800.9 and SN04 (or SN05) `START`.* | breadcrumbs 5000, 5010 |
| **Q14** | **Budget never above two:** at most two NATO copies between a `START` and the next `DELETE DELAY` (SN01 + SN04, then SN02 alone). `SN03`, `SN05` and `SN06` never start. No `DENY WAIT` (no refusal), no `LOSSES`, no `HARD STOP`. | breadcrumbs |
| **Q15** | **Decision 0037:** the only 501 aircraft in the log is the player; every AI aircraft is 602 (F-80C-10, F-51D). No DPRK breadcrumb exists (there is no DPRK side). | log `COUNTRY:` fields; build test |
| **Q16** | **Replay:** no dead links. Never fired: SN03, SN05, SN06, `ZN01 DENY WAIT`, `ZN01 MISS WAIT OUT`, every `HARD STOP`, `LOSSES` and `COLD`, `SN01 EXTEND`, `SN04 ON STATION`. | Replay window |

### Risks

- **Orbit drift.** If the orbit drifts more than 8 km from A at a check, `THN01 CLOSE DELAY` logs (A COLD) and the trigger re-arms; the next entry gives a new `INNER` 80.1 s later.
  - If A is COLD at 600 s, slot 1 finds EN02 shut and rerolls until `SLOT CLOSE` (616.6) with no launch. Slot 2 is then the clock launch (Q13, the italic case), and the flight runs to about 1,900 s.
  - If A is COLD at SN01's on-station end, SN01 ends by `COLD` and not by `Zone Out`, so Q7 sees SN04 end alone. Leaving the orbit before the cue has the same effect.
- **The level-flight helper trap** (A2 run 1): see step 4.
- **The Map air start has not been flown before.** It is a fake field with a Planes list (`StartType 0`, `Number -1`, `Renewable 1`), the same form as the A1/A2 `--spawns` air starts. If it is missing from the spawn list or spawns on the ground, note it and leave. Q1 fails, and the rest cannot be flown.
- **The F-80s or F-51s may attack the Yak.** They carry ground-attack orders (`AttackAir 0`), as in A2, where neither did.
- **Speed.** A2's throttle setting flew faster than planned, so the leg-in runs early (see "Choices"). This changes no outcome:
  - the clock slot is fixed in mission time;
  - the leave cue is read from the log;
  - the re-entry `INNER` would come before `ZN01 ARM` only above about 860 km/h.
- **Server load:** at most 8 AI aircraft fly at once, so the mission should run at about 1.0× wall time.

## Replay

- **In the app:**
  1. Header **Replay…**.
  2. **Pick .Group…** `handoff/p14-probes/acceptance-a3/P14_Acceptance_A3.Group`. The sidecar `.trace.json` is found next to it.
  3. **Log folder…** `src/testdata/missionlog/real_51`.
  4. **Run replay**, then read Dead links, Never fired, Fired and Timeline. Filter the Timeline by group (`… / Air Tasking NATO Sorties / SN01 …`, `… / SN04 …`) for Q9.
- **Command line (fallback):**
  ```
  cargo run --release -- replay --group handoff/p14-probes/acceptance-a3/P14_Acceptance_A3.Group --trace handoff/p14-probes/acceptance-a3/P14_Acceptance_A3.trace.json --logs src/testdata/missionlog/real_51 --out target/p14/acceptance_a3_report.md
  ```

## Checks (2026-10-30, branch `claude/p14-acceptance-a3`)

- `cargo test --offline`: **1046 passed, 0 failed, 36 ignored** (main: 1044 / 35; two new tests and the ignored writer). `existing_outputs_match_baseline` and `map_generate_without_air_matches_baseline` pass.
- Warnings: **81 build, 306 clippy** (unchanged).
- The Map Generate output for the same inputs is unchanged (no product code changed).
- Code review (two axes, Standards and Spec). Fixed:
  - the air-start distance check reads the trigger and Airfield positions and the trigger's radius from the built file, instead of comparing constants;
  - the position comment sits on `A3_SPAWN`, not on the name;
  - the NATO clock is indexed `schedule[AirSide::Nato.idx()]`;
  - the A2 and A3 writers share `copy_flight_files`;
  - the A2 and A3 frozen tests share `assert_done_hub_offs` and `assert_areas_watch_dprk_from_outside`;
  - a position lookup fails with a message;
  - five run-sheet corrections:
    - no `BUD N COUNT` at the later start (Q12);
    - the ring-entry margin for the return (step 10);
    - the speed A2's throttle actually gave;
    - the `SN04 Zone IN` window;
    - A's COLD time (Q10).
  - Kept: the A3 facts are checked in both the UI build test and the frozen-file test, as for A1 and A2.
- The Spec review checked these against the frozen `.Group`:
  - the clock slot times (600.1, 1,800.1, 3,000.1);
  - EN02's LAST being cleared at `DRAW` and set by `SN04 START`, so slot 2 is quiet;
  - `Zone IN` arming `Zone Out`;
  - DH + 60.6 to `DELETE DELAY`;
  - only SN01–03 re-arming ZN01;
  - the spawn distances (30.0 and 43.3 km).
- No game fact was learned before the flight. The flight's facts are in `docs/il2-loader-and-log-facts.md`, "Found on 2026-10-30".

## Results (flown 2026-10-30)

Flown by Claude on this PC (game 1.005), keyboard only, Tacview on, from the frozen mission `P14_Acceptance_A3_20261030` in the Dogfight folder.

- **Run 1**, session `2026-10-30_12-25-15`, logs `src/testdata/missionlog/real_52/` (18 files), Tacview `handoff/tacview/P14_Acceptance_A3_run1.zip.acmi`.
  - Spawn S = 119.24 s at (187,500; 258,037), 1,998 m, `COUNTRY:501`.
  - The NATO area saw the player: `THN01 INNER` 306.74 (S + 187.5), `ZN01 CHANCE` 306.84, `ZN01 FIRE` and `SN01 START` 306.88.
  - Before the orbit I moved the mouse to the top of the screen and clicked, to give the game keyboard focus, then held LAlt+Z. The Yak pulled into a stall and spin and hit the ground at 412.30 s, about 5 km from A. The instructor showed "Don't pull the stick so much!" and "Spin!". Most likely the mouse movement worked the stick: see `docs/il2-loader-and-log-facts.md`, 2026-10-30.
  - Leave Server took the game to the results screen, and the server reloaded the mission into a new session.
- **Run 2**, session `2026-10-30_12-34-44`, logs `src/testdata/missionlog/real_51/` (65 files), Tacview `handoff/tacview/P14_Acceptance_A3_run2.zip.acmi`. The raw `.acmi` files of both runs are in `C:\Machine Intelligence\P14 Flight A3\tracks\`.
  - Spawn **S = 98.74 s**, throttle **58 %** (about 485 km/h indicated, 2,000 m), heading 089.
  - The mouse was not moved in flight. A 3 s test turn showed the helper on (height held).
  - Left orbit begun abeam A at about S + 2:40. Rolled out on heading 358 at about 1,040 s, 7 s before `SN01 ON STATION`. A stayed HOT until its next check, so SN01 still saw A HOT.
  - North to 35 km. A 180° left turn about 36 s after DH, then back south. A **right** orbit over A, because the track was about 1 km east of A's line.
  - Left the server at about 1,892 s. Leaving in flight logged the Yak as destroyed (`AType:3`, `AID:-1`). The results-screen leave looped into a mission reload; `CloseMainWindow` closed the game cleanly.
  - Mission time kept pace with wall time.
- **Replay** (`replay` command on `real_51`, report `target/p14/acceptance_a3_report.md`, not committed): **no dead links**.

### Run 2, prediction by prediction

| Id | Flown |
|---|---|
| Q1 | **yes**: loads; the DPRK spawn list offered "P14 A3 DPRK air start" (Yak-9P, unlimited), the Map's own air start. The player is a Yak-9P on **`COUNTRY:501`** at (187,500; 258,037), 1,998 m. |
| Q2 | **yes: the NATO area sees the Map air start's player.** `THN01 INNER` **279.66** (P = S + 180.9). `ZN01 CHANCE` 279.76 (+0.10); `ZN01 FIRE` and `SN01 START` 279.80 (+0.14). |
| Q3 | **yes**: `THN01 INNER` 279.66, 439.66, 599.66, 759.66, 919.66 and 1,079.66, **exactly 160.00 s apart**. No `CLOSE DELAY` while orbiting, and no `ZN01 CHANCE` at these passes. |
| Q4 | **yes: the NATO clock launch.** `CLK N SLOT OPEN` **600.10**; `EN02 GO` and **`SN04 START` 600.90**; `BUD N COUNT` 601.14 (two copies). |
| Q5 | **yes**: `SN01 Zone IN` 446.28 (s + 166.48); `SN04 Zone IN` 914.94 (s + 314.04, inside 901–926). |
| Q6 | **yes**: `SN01 ON STATION` **1,046.78** = Zone IN + 600.50, no `SN01 COLD`. |
| Q7 | **yes: two sorties on one target ended together.** `SN01 Zone Out`, `SN01 DONE HUB`, `SN04 Zone Out` and `SN04 DONE HUB` all at **1,280.66** (DH), one tick, about 240 s after the roll-out north. No `SN04 ON STATION`, no `SN04 COLD`. |
| Q8 | **yes: deleted together.** `SN01 DELETE DELAY` and `SN04 DELETE DELAY` at **1,341.26** = DH + 60.60, one tick. No `BUD N COUNT`. |
| Q9 | **yes (decision 0038)**: after DH, SN01 and SN04 logged only `DELETE DELAY`. Never logged: `SN01 EXTEND` (due ≈ 1,647), `SN04 ON STATION` (due ≈ 1,815), either `HARD STOP`, or a second `Zone IN` when the player came back within 35 km. |
| Q10 | **yes**: `THN01 CLOSE DELAY` **1,241.70** = the last `INNER` (1,079.66) + 162.04 (the 1,239.66 check failed). |
| Q11 | **yes (decisions 0044, 0045)**: `ZN01 COOLDOWN` and `ZN01 ARM` at **1,580.66** = DH + 300.00; no `MISS WAIT OUT`, no second cooldown. |
| Q12 | **yes: a later zone start on the same side.** Back in A's ring, `THN01 INNER` **1,699.54** (after ARM). `ZN01 CHANCE` 1,699.64; `ZN01 FIRE` and **`SN02 START` 1,699.68**; no `BUD N COUNT` (one copy). `SN02 Zone IN` 1,866.30 (s + 166.62). |
| Q13 | **yes: slot 2 was quiet by the no-repeat rule.** `CLK N SLOT OPEN` **1,800.10** with A HOT (`INNER` 1,699.54 and 1,859.54) and SN02 alone flying, and **no `EN02 GO`, no `SN05 START`**. |
| Q14 | **yes**: at most two NATO copies at once (SN01 + SN04, then SN02 alone). SN03, SN05 and SN06 never started. No `DENY WAIT`, no `LOSSES`, no `HARD STOP`, and no AI aircraft was lost. |
| Q15 | **yes (decision 0037)**: every AI aircraft in the log is 602 (F-80C-10, F-51D); the only 501 objects are the player's Yak and pilot. 13 `AType:28`: the player plus 12 AI (SN01, SN04, SN02). |
| Q16 | **yes**: no dead links. Never fired, as listed: SN03, SN05, SN06, `ZN01 DENY WAIT`, `ZN01 MISS WAIT OUT`, every `HARD STOP` and `LOSSES`, `SN01 COLD`, `SN01 EXTEND`, `SN04 ON STATION`. SN02's later timers were still running at the leave. |

**All three step 11 items are seen in the game:**
- the Map air start's player on 501, seen by a NATO area (Q1, Q2);
- a NATO clock launch (Q4);
- two sorties on one target ending in one tick and deleted together, then a later start on that side (Q7, Q8, Q12).

### Not predicted

- **Run 1's spin** (see Run 1). The run sheet's step 4 warned against LShift+A but not against the mouse. Mouse movement in flight most likely works the stick.
- **The roll-out came 7 s before the leave cue** (1,040 against 1,046.78), to reach heading 000 without a second circle. This changed nothing, because A stays HOT until its next presence check. SN01's `ON STATION` saw A HOT.
