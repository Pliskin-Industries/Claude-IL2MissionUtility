# P14 probe results

Step 0 "0d": one row per cell, observed vs the plan's "Confirmed if" column
(`handoff/P14-air-tasking.md` §8 step 0). Evidence is the server's text log,
read with `il2_mission_utility replay` (report: `handoff/P14-probe-replay.md`).

## Flight 0 — 2026-10-03, session `2026-10-03_20-45-36`

Korea dogfight on the user's dedicated server, built with
`tools/p14/build_probe_mission.py` from `claude/p14-air-tasking` @ `c87f492`
(breadcrumbs: C0/S0 objectives). Logs: `src/testdata/missionlog/real_3/`
(files `[1]`–`[118]`; `[0]`, the first ~30 s, was not supplied). The player
joined at 03:51 and spawned at Kimpo at 04:12 (F-80C); the mission was not
ended normally (no `AType:7`). No video.

Clock: the replay fits **T = 3 + 50·s** from 11 cue breadcrumbs (t4 confirmed).
All times below are mission time from Mission Begin.

| Cell | Observed (log) | Confirmed if | Result |
|---|---|---|---|
| T-a1 | **ZA1** (Closer 1, `[2]`, 12 km, 60 km E of K14) fired at **01:57.40**; the player was not in a plane until 04:12, so the 4-ship AI F-80 flight crossing the zone set it off. ZA2 (`[1]`, control) never fired. | ZA1 silent until 06:00 | **a NOT confirmed: AI aircraft trigger a check zone.** (ZA4 / flight 1A still to fly.) |
| T-e E1 | fired once, 01:04 (3rd pulse); "E1 2+" never | fires on 3rd, no "2+" | confirmed |
| T-e E2 | fired 01:04 and 01:10; "E2 2+" at 01:10 (6th pulse) | 3rd, and "2+" at 6th | confirmed (Dropcount 1 resets) |
| T-e E3 | fired 01:06 (2 pulses, Deactivate, Activate, 1 pulse) | recorded | count survives Deactivate/Activate |
| T-e E4 | never fired | does not fire on 3rd (reset worked) | confirmed |
| T-e E5 | never fired (2 pulses while deactivated, Activate, 1 pulse) | recorded | inputs to an inactive counter are not counted |
| T-e E6 | fired 01:04 and 01:10; "E6 2+" at 01:10 | "2+" at the 6th pulse | confirmed (SetVal re-arms a fired counter) |
| T-e E7 | fired 01:40 | fires at 01:40 | confirmed: same-tick pulses counted one by one (r) |
| T-e E8 | fired 01:50; "E8 2+" never | shows, "2+" never | confirmed: once-gate works |
| T-e E9 | fired 02:00.10 | fires at 02:00.10 | confirmed |
| T-f F1 | 19 hits; "reached 12" at 03:39; "reached 29" never | 12–28 hits | confirmed |
| T-f F2 | outputs 1/2/3/4: 9 / 10 / 12 / 9 | each ≥ 1 | confirmed |
| T-t T1 | one line, 04:35 | exactly one | confirmed |
| T-t T2 | five lines, 04:37–04:45, 2 s apart | 5 lines | **confirmed: every firing logs (t1)** |
| T-t T3 | T3a and T3b both at 04:47 | both logged | confirmed (t5) |
| T-t T5 | no line (Spawn carrier) | information | as expected: spawned vehicles write nothing |
| T-t T7 | **one** line at 05:25 for one breadcrumb pulsed twice in a tick | 2 lines = log shows same-tick doubles | one line: the log cannot show a same-tick double firing of one MCU |
| T-j J1 | never fired (10 s timer deactivated at 5 s) | either | **cancels** (deactivating a running timer stops it) |
| T-j J2 | never fired (pulse into an inactive relay, Activate 1 s later) | no replay | **confirmed (j): no replay** |
| T-j J3 | never fired (Activate + pulse in one tick) | recorded | the same-tick pulse is dropped |
| T-j J4 | "J4 out" 09:00; "J4 out 2+" never | out once, no "2+" | **confirmed (q)**; valid because E7 is confirmed (0d rule) |
| T-j J5 | "J5 out" at 09:15 = 15 s after the first trigger; "2+" never | restart / ignore / twice | **restart (p)** |
| T-z Z1 | never fired, neither at 10:00 nor 10:15 | fires within 1 s, both times | **z NOT confirmed: an empty Closer 0 zone does not fire on Activate + pulse** |
| t2 | `OBJID` ≠ MCU index; position exact | — | match objectives by position |
| t3 | Success 0 logs, round continues; Success 1 ends the round (8.1 s) | — | C0/S0 is the default; visibility to players **not yet reported** |
| t6 | 30 cue/cell breadcrumbs logged; `[0]` missing, so CUE 01 and early T-a1 lines unseen | every subtitle has a line | partial (no video) |

## Strategy defaults set by flight 0 (0d table)

| Result | Setting |
|---|---|
| e (E1, E2, E4) | `BudgetImpl::HeadCount` |
| E6 | `CounterResetImpl::SetVal` |
| r (E7), E9 | `CensusImpl::Spaced` stays the default; `SameTick` may be chosen |
| f | `RandomImpl::InMission` |
| j (J2), J1 | D11 as written; deactivating a running timer cancels it |
| p (J5) | restart; `ZoneOutImpl::Watchdog` allowed |
| q (J4, via E7) | relay latches as written |
| t1–t6 | `TraceCarrier::Objective(C0/S0)`, tick rate 50/s, offset 3 |
| a (flight 0 part) | **AI counts.** Per the 0d table the D30 warning becomes an error unless the user decides otherwise (§10 item 15). Flight 1A's ZA4 still to fly. |
| z | not confirmed: `PresenceCheckImpl::Closer0Race` is **not** available as a fallback for c. **Revisited 2026-10-14 (flight 2, decision 0018):** cell T-z switched Z1 on and pulsed it in the same tick, from one timer (`T-z ARM`); flight 2's Zone Out, switched on and pulsed 0.1 s later, fired at once when empty. The T-z result is most likely the same-tick order, not the zone. |

**Gate G0: passed.** No "build stops" row fired (J2 confirmed; J4 confirmed with E7).

Walker defaults to align with the game: `DeactivateRunningTimer::Cancels` (J1),
`TimerRetrigger::Restart` (J5), `SameTickCounts::Each` (E7),
`DropcountMeaning::ResetOn1` (E2), `SetValRefires::Yes` (E6).

## Flight 0 calibration on the second PC — 2026-10-12, session `2026-10-12_14-08-36`

Purpose: show that this PC (Lenovo, integrated graphics, game `261002`, mission hosted by the **game client**, Claude at the controls) gives the same answers as the user's dedicated server did on 2026-10-03. Same probe files, rebuilt as `P14_Probe_0_cal_20261012` with `tools/p14/build_probe_mission.py`. The player joined NATO at K14 in an F-80C at 03:16 and stayed on the ground. The mission was finished from the menu at 16:04 (`AType:7` logged). Logs: `src/testdata/missionlog/real_4/` (81 files, `[0]` included). Replay report: `target/p14/flight0_cal_report.md` (not committed; re-create it with `il2_mission_utility replay`).

Both log sets were replayed with the same binary and the two reports compared line by line.

| What | 2026-10-03 (dedicated server) | 2026-10-12 (this PC) | Verdict |
|---|---|---|---|
| Clock fit | T = 3 + 50·s, 11 cues | T = 3 + 50·s, 12 cues | same |
| T-a1 ZA1 (AI in a `[2]` zone) | fired 01:57.40 | fired 01:57.42 | same, one tick apart |
| T-a1 ZA2 (control) | never | never | same |
| T-e E1 to E9 | as in the table above | identical times | same |
| T-t T1, T2, T3a/b, T7 | 1, 5, 1+1, 1 lines | identical times | same |
| T-j J1 to J5 | J1 to J3 never; J4 09:00; J5 09:15 | identical | same |
| T-z Z1 | never | never | same |
| T-f F1 (random 50 %) | 19 hits, "reached 12" at 03:39 | 19 hits, "reached 12" at 03:51 | random; both inside 12 to 28 |
| T-f F2 outputs 1/2/3/4 | 9 / 10 / 12 / 9 | 7 / 13 / 11 / 9 | random; each at least 1 |
| CUE 01 at 00:00 | not seen (`[0]` missing) | logged at tick 3 | t6 gap closed |
| Mission end | not logged | `AType:7` at 16:04.5 | new |

**Result: this PC reproduces the dedicated-server results.** Every cell with a fixed answer gave the same answer at the same time. Only the two random cells differ, as they should.

Seen on screen (Claude, in the cockpit): the subtitles "F2 out 2", "T-z start" (10:00) and "Flight 0 done. End the mission normally" (11:00) all displayed. The briefing map shows the probe's own icons ("T-z", "T-a1") and no marker for any breadcrumb objective near K14; the breadcrumbs sit at x ≈ 5000 to 5400, z = 5000 in the far corner of the map, and that corner was not inspected.

New log facts from this run:
- A client-hosted mission writes a new log file about every 11 s (the dedicated server: about every 33 s).
- The mission header lists countries `501:1,502:1,503:1,601:2,602:2,603:2`, so 602 and 603 exist as NATO-side countries in a mission built on the dogfight test Options block, although the editor names only USA.

## Flight 1A, flown as a flying check — 2026-10-12, session `2026-10-12_14-58-12`

Flown by Claude on the second PC (game client hosting, game `261002`), from an **air start** over K14 instead of a takeoff. Probe and build: `handoff/p14-probes/flight1a/README.md`. Logs: `src/testdata/missionlog/real_5/` (55 files). The player spawned at 03:35, the mission was finished from the menu at 27:54 (`AType:7` logged). The country-filter cells Y1 to Y3 were **not** in this build.

How it was flown: level-flight helper on, turns with the helper's turn keys, at 4560 m (not 3050 m: the helper was switched on late and holds the height it is given). The zones are cylinders, so the height does not change any zone result. Orbit around the T-c C icon from about 09:30 to 11:45 at 220 to 350 km/h indicated; then east over the T-c C4 icon at about 12:50; then the T-n line, flown **southbound** after a first pass missed it by about 15 km.

| Cell | Observed (log) | Confirmed if | Result |
|---|---|---|---|
| T-c C0 (sentinel) | fired 11:00.00 | shows P is inside | P was inside C at 11:00 |
| T-c C1 (Activate only) | never fired | recorded | Activate alone does not fire a zone with a player inside |
| T-c C2 (Activate + pulse 0.1 s) | fired 11:00.10 | fires within 1 s | **c confirmed:** it fired on the pulse, with the player already inside |
| T-c C2 again (Deactivate 11:10, Activate + pulse 11:15) | fired 11:15.00; "C2 fired again" 11:15.00 | within 1 s of 11:15 | **re-check confirmed:** a check zone can be used twice |
| T-c C3 (pulse while deactivated) | never fired | recorded | a pulse into a deactivated zone does nothing |
| T-c C4 (Activate + pulse 11:30, Deactivate 11:45, P enters after 12:00) | never fired | never fires | **h confirmed**, with one limit: that the player was inside C4 rests on the cockpit map (plane on the C4 icon at about 12:50), not on a log line |
| T-c C5 (one pulse at 10:30) | fired once, 10:30.00; "2+" and "10+" never | once / repeats | **o = fires once** |
| T-n ZA4 (12 km, `[2]`, pulsed 14:00) | fired 22:24.16, as the player entered | fires for P | **a zone fires for a player.** With flight 0: players and AI both trigger it |
| T-n START, END (2 km) | END 22:15.72, START 23:58.98 | speed within 10 % of 732 km/h | **n not measured.** Flown southbound at full throttle and 4560 m, with an S-turn between the zones. The 103.3 s gives 24 km / 103.3 s = 837 km/h, which is not a cruise figure |

**For gate G1 (step 4):** the T-c answers the threshold design needs are in: c, the re-check and h are confirmed, and o is "fires once". **The user ruled on 2026-10-12 that this run counts as flight 1A and gate G1 is met** (`docs/decisions/0009`). Still open from flight 1A: risk n (cruise speed) and the country-filter cells.

Seen on screen: the cues "T-c: fly east into C4 now" (12:00) and "Flight 1A done. End the mission normally" (26:00). Per-event subtitles last 1 s and were not caught in the samples.

## Flight 1G (kill events, no pilot) — 2026-10-12, session `2026-10-12_19-05-42`

Step 0e probe `P14_Probe_1G_traced` (`handoff/p14-probes/flight1g/`), built as `P14_Probe_1G_20261012`, hosted by the game client on the second PC (game `261002`). Claude sat on the ground at K14 from 03:31; the mission was finished from the menu at 26:54. Logs: `src/testdata/missionlog/real_6/` (66 files).

| Flight | What was done to it | Observed (log) | Result |
|---|---|---|---|
| G4 (both planes) | `MCU_CMD_Damage`, Complete, at 04:00 | Kill records at 04:00.02 with attacker −1, for each plane and its pilot. Per plane: event **13** at 04:00.04, event **0** at 04:00.10. G4b also event **2** at 04:15.08 (15 s later); G4a's event 2 never logged. **Event 4 never.** | A Damage command kills at once and raises 13 and 0, once each |
| G1b | shot down by the hunter wingman at 10:57.88 (9.5 min after the first Attack order) | event **13** and event **0** at 10:57.90, event **2** at 11:01.30. **Event 4 never.** | A gun kill raises the same events as the Damage command |
| G1a | attacked, not shot down | no event | see the finding below |
| G2 (`MCU_Delete` at 10:00) | deleted | no event of any type | **Delete raises no event** |
| G3 (Deactivate, then Delete 0.5 s later) | deleted | no event of any type | **the template's Deactivate → Delete chain raises no event** |

**Result g: not confirmed as written; the design needs event 13, not event 4.** The plan expected one type 4 per aircraft lost. No type 4 fired for any of the three aircraft that died. Type 13 fired exactly once for each of them, by gun and by Damage command, and never for a deleted aircraft. No "ev4 2+" readout fired, which says nothing, because no ev4 fired at all. Event 5 never fired; nothing landed.

**Finding: one aircraft outlived its Delete.** G1a (log id 20480) still existed when the mission ended at 26:54, at 200 m, 5 km from K14, with 66 rounds fired. The cell close at 15:00 fired `G1 CLOSE DELETE`, whose Objects are G1a's and G1b's entities; G1b had been dead for four minutes. The hunters' own close Delete, with two live aircraft, worked: neither hunter is in the end-of-mission list. **Not known:** whether a Delete with a dead aircraft in its Objects list skips the live ones, or something else kept G1a. P14's budget counts an aircraft as gone when it is deleted (U22), so this needs its own probe cell before step 3 is settled.

Other facts from this run:
- AI told to attack with `MCU_CMD_AttackTarget` four times (01:30 to 06:00) took until 10:58 to get one kill, with ace skill, and never got the second. An AI gun kill is too slow and unsure to build a test on; the Damage command is exact.
- A Damage-command kill logs `AType:3` with `AID:-1`.
- The log names the hunter's country as 503 and its victim's as 601, so a `Country` changed on a copied flight takes effect.

## Flight 1N (cruise leg and NATO country filter) — 2026-10-12, session `2026-10-12_19-37-02`

Step 0e probe `P14_Probe_1N_traced` (`handoff/p14-probes/flight1n/`), built as `P14_Probe_1N_20261012` with `--spawns`. Flown by Claude from the `1N NATO` air start: spawned at 04:02.22 heading north at 3048 m, level-flight helper on, **throttle 100 % throughout**, 2990 m, 756 to 762 km/h indicated. Finished from the menu at 15:54.8. Logs: `src/testdata/missionlog/real_7/` (34 files). The air start faced north as the spawn list asked, and the track drifted 1.2 km east over 157 km.

**Cell Y-N (country filter).** Times are mission time; each pair is two aircraft, so each trigger event comes twice, about 3 s apart.

| Window | Who crossed | YN-CT (`Country = 601`) | YN-CT0 (no filter) | Plain zones `[2]` |
|---|---|---|---|---|
| A | AI pair, country **602** | **silent** | entered 02:17.68 and 02:20.66; left 03:55.88 and 03:56.20 | Z1 fired 02:17.68 |
| B | AI pair, country **601** | entered 07:27.66 and 07:30.76; left 09:05.86 and 09:06.18 | the same four times | Z1 and Z2 fired 07:27.66 |
| pilot | player, country 601 | entered 11:17.74; left 12:38.92 | the same | Z1, Z2 and Z3 fired 11:17.74 |

**Result: the country filter works on the NATO side.** A Complex Trigger filtered to 601 ignores an aircraft of country 602 and fires for an aircraft of country 601, AI or player. This is option 4 of `docs/decisions/0008`. Also shown:
- **A Complex Trigger sees AI aircraft,** and reports Object Entered and Object Left once per aircraft.
- **Country 602 is usable.** A plane written with `Country = 602` loads, flies, and counts as NATO: the plain `[2]` zone fired for it.
- **A plain check zone fires again for each new arrival.** Z1 was pulsed once, at 00:10, and fired three times, once per window. In flight 1A zone C5 fired once while one player stayed inside. So a pulsed Closer 1 zone fires once per entry into an empty zone, not once for ever.
- The "entered 2+" and "entered 3+" readouts fired at the second and third entries, as wired.

**Cell T-n2 (speed).** START fired at 06:52.14 and END at 08:30.62: 24 km in 98.48 s = **877 km/h** over the ground at 100 % throttle and 2990 m. The crossing of Y-N (a 19.9 km chord, 81.18 s) gives 883 km/h. The game's handbook chart shows about 890 km/h for the F-80C in combat mode at 3000 m, so **the handbook's speed charts are true speed and agree with the game within 2 %.**

**The AI's own speed was measured too.** Both AI pairs took 98.2 s between entering and leaving the 20 km circle: **733 km/h** over the ground, flying the HVAR template's waypoint (`Speed = 660`, 3050 m). The plan's F-80 figure is 732 km/h.

**Result n:** the plan's 732 km/h matches what AI F-80Cs flying the built-in template actually do, to within 1 km/h. A player at full throttle is 20 % faster (877 km/h); the handbook's "cruising speed" for a player is 348 km/h indicated. Which of these the threshold timing should use is a step 4 decision (U3 says "F-80 transit time at cruise"); risk n's fallback is not triggered by the AI figure.

Not done: the cue at 20:00 was not waited for; the mission was ended at 15:54 once every cell had its answer.

## Flight 1D (DPRK-side country filter) — 2026-10-12, session `2026-10-12_19-58-17`

Step 0e probe `P14_Probe_1D_traced` (`handoff/p14-probes/flight1d/`), built as `P14_Probe_1D_20261012` with `--spawns`. Flown by Claude from the `1D USSR` air start in a MiG-15bis (country 501): spawned at 05:46.34 at 3048 m, level-flight helper on, throttle 100 %, about 2960 m, heading 357° (the helper held the heading it had when it was switched on, 3° off north). Finished from the menu at 14:55.9. Logs: `src/testdata/missionlog/real_8/` (31 files).

| Window | Who crossed | YD-CT (`Country = 501`) | YD-CT0 (no filter) | Plain zones `[1]` |
|---|---|---|---|---|
| A | AI pair, country **503** | **silent** | entered 02:17.80 and 02:18.56; left 03:56.12 and 03:56.38 | Z1 fired 02:17.80 |
| B | AI pair, country **501** | entered 07:27.80 and 07:28.56; left 09:06.06 and 09:06.34 | the same four times | Z1 and Z2 fired 07:27.80 |
| pilot | player, country 501 | entered 11:19.74; left 12:22.16 | the same | Z1, Z2 and Z3 fired 11:19.74 |

**Result: the country filter works on the DPRK side too,** with the same pattern as flight 1N. A trigger filtered to USSR (501) ignores DPRK-country (503) aircraft of the same coalition and fires for USSR aircraft, AI or player.

Speeds from this run: the AI pairs (F-80Cs with the country changed) again took 98.3 s across the 20 km circle, 732 km/h. The MiG-15bis at 100 % throttle crossed in 62.42 s; the track was about 4 km off the centre by then, so the chord was about 18.2 km and the speed about 1050 km/h. The handbook gives 1038 km/h maximum continuous and 1078 short-term.

## Flight 1H (the Delete probe, no pilot) — 2026-10-13, three runs

Step 0f probe `P14_Probe_1H_traced` (`handoff/p14-probes/flight1h/`), hosted by the game client on the second PC. **The game had updated to 1.005 (`261013`) that morning.** Nobody spawned; Claude stayed on the briefing screen and left the server after 06:00. The three runs gave the same events at the same times.

| Run | Session | Built as | Left at | Logs |
|---|---|---|---|---|
| 1 | `2026-10-13_14-35-56` | `P14_Probe_1H_20261012`, resaved by the 1.004 resaver the day before | 06:35 | `src/testdata/missionlog/real_9/` |
| 2 | `2026-10-13_14-43-49` | the same files (the server reloaded the mission) | 06:56 | `real_10/` |
| 3 | `2026-10-13_14-53-52` | `P14_Probe_1H_20261013`, **not resaved**: a plain `.Mission` and `.eng` | 06:26 | `real_11/` |

**The controls held, so the flight can be read.** D0 (never touched): both aircraft in the end-of-mission list. D1 (one Delete, two live aircraft, entities in Objects): no event at 04:30 and neither aircraft in the list. D7 (never removed): both raised event 13 at 04:30.04.

| Pair | Done to it | Observed | Result |
|---|---|---|---|
| D8 | 04:00: one `MCU_Delete` whose Objects are the two **Plane** indexes, as `generate_template` writes its `Trigger Delete` | Both aircraft were hit by the detector at 04:30.02 and raised event 13 at 04:30.04 | **A Delete that names Plane blocks removes nothing.** |
| D9 | 04:00: `MCU_Deactivate`, no Delete | No event at 04:30. Both aircraft in the end-of-mission list | A deactivated aircraft still exists. A Damage command does not reach it. |
| D2, D3, D4, D5, D6 | 02:00: `MCU_CMD_Damage` (Complete) with **one** aircraft's entity in Objects | **Both** aircraft of the pair got Kill records at 02:00.02 and raised event 13 at 02:00.04, whichever one was named (D3 named the lead, the others the wingman) | **A Damage command aimed at one aircraft kills its whole formation.** |
| D2 to D6, after that | the 04:00 removals and the 04:30 detector | No second event, and none of the ten in the end-of-mission list | Nothing can be said about a Delete with one live and one dead aircraft: no pair was in that state. |
| D5b, D7a, D7b | killed, named in no Delete | Not in the end-of-mission list | The end-of-mission list holds live aircraft only. A dead aircraft does not raise event 13 again when damaged. |

**What this settles.**
- **Air Tasking's clean-up must list entities in its Deletes.** The six built-in sorties and everything Template Builder generates list Plane indexes in `Trigger Delete`; in the game that Delete does nothing, and the units are left deactivated (`Deactivate Units` lists entities and works). Each spawn that is "cleaned up" this way stays in the mission. This is existing app behaviour and its own decision (`docs/decisions/0013`).
- **Flight 1G's row for G3 must be read again:** "Deactivate, then Delete" raised no event because the aircraft were deactivated, not because they were deleted.
- **Event 13 on a Damage kill is the same on game 1.005 as on 1.004.**

**What it does not settle.** Whether a Delete skips a live aircraft when its list also names a dead one (flight 1G's survivor). Flight 1J (plan step 0g) repeats those rows with single-aircraft flights.

Other facts from these runs:
- `AType:28` is logged once per aircraft when its flight is activated (20 records at 01:00.5).
- The unresaved mission logs `MFile:Multiplayer/Dogfight/P14_Probe_1H_20261013.mission`; the resaved one logs `.msnbin`. While it loads, the game shows "Multiplayer server overload: erratic plane behavior may occur". The events were the same.
- The client-hosted server reloads the mission about 20 s after the results screen. Runs 2 and 3 came from that.

## Flight 1J (the Delete probe with single aircraft, no pilot) — 2026-10-13, session `2026-10-13_15-17-18`

Step 0g probe `P14_Probe_1J_traced` (`handoff/p14-probes/flight1j/`), built as `P14_Probe_1J_20261013` without the resaver, hosted by the game client on the second PC (game `261013`). Nobody spawned; the server was left at 06:31. Logs: `src/testdata/missionlog/real_12/` (14 files). One run.

**The controls held.** J0 (never touched): both aircraft in the end-of-mission list. J1 (one Delete, two live aircraft of two flights): no event at 04:30, neither in the list. J6: J6y raised event 13 at 02:00.04 and not again; J6x raised it at 04:30.04 only. So a Damage command aimed at a 1-ship flight kills that aircraft alone, and the detector works.

| Pair | Killed at 02:00 | Removed at 04:00 by | The live aircraft at 04:30 | Result |
|---|---|---|---|---|
| J2 | J2y | one Delete, Objects = [J2x, J2y] | J2x: no event, not in the list | removed |
| J3 | J3x | one Delete, Objects = [J3x, J3y] | J3y: no event, not in the list | removed |
| J4 | J4y | one Delete per aircraft | J4x: no event, not in the list | removed |
| J5 | J5y | one Delete, Objects = [J5x] | J5x: no event, not in the list | removed |

**Result: a dead aircraft in a Delete's Objects list is harmless**, whether it comes before or after the live one. One Delete that lists every aircraft's entity removes the live ones.

**So flight 1G's survivor has another cause.** What that case had and this flight did not: the two aircraft were one formation; the dead one was shot down by gunfire, not by a Damage command; the survivor had been in a fight (66 rounds fired) and ended the mission low and near K14, which looks like an aircraft that had decided to return to base. None of these has been tested on its own.

## Flight 1K (what can keep an aircraft from being deleted, no pilot) — 2026-10-13, two runs

Step 0h probe `P14_Probe_1K_traced` (`handoff/p14-probes/flight1k/`), built as `P14_Probe_1K_20261013` without the resaver, hosted by the game client on the second PC (game `261013`). Nobody spawned.

| Run | Session | Left at | Logs |
|---|---|---|---|
| 1 | `2026-10-13_16-06-08` | 16:41, the full flight | `src/testdata/missionlog/real_13/` (46 files) |
| 2 | `2026-10-13_16-25-52` | 05:02, after the 04:30 detector; rows K9 and K10 not reached | `real_14/` (17 files) |

**Cell T-d3, one thing at a time.** "Removed" means no event at the 04:30 detector and not in the end-of-mission list. "Left" means the aircraft was still there afterwards.

| Pair | Done to it | Run 1 | Run 2 | Result |
|---|---|---|---|---|
| K0 | nothing | both died at 03:06.4 in the same spot at 1,780 m, no attacker: a collision | both in the list | the control held in run 2 |
| K1 | 04:00 Delete of the lead only | lead removed; wingman died to the detector | the same | **a Delete removes the aircraft it names, not the formation** |
| K2 | 04:00 Delete of the wingman only | wingman removed; lead died to the detector | the same | the same |
| K3 | 02:00 Delete of the wingman; 04:00 Delete of both | both removed | both removed | a formation that has lost a member, with a list that still names it, deletes cleanly |
| K4 | 02:00 Damage level 1 on the lead; 04:00 Delete of both; 04:30 detector on both | **K4a left:** no kill record and no event at 04:30, in the list at 16:41 at 613 m, 3 km from K14. K4b removed | **K4b left:** no kill record and no event at 04:30, in the list at 05:02 at 4,000 m, flying south-west. K4a removed | **one aircraft of a damaged pair survived both the Delete and the kill command, a different one each run** |
| K5 | the same at level 0 | both removed | both removed | level 0 did not stop the Delete at 04:00 |
| K6 | 02:00 Damage level 1, nothing else | both in the list at 16:41, at 310 and 450 m, about 7 km from K14 | both in the list at 05:02 at 4,000 m, flying south-west, 12 km apart | level 1 does not kill. Both aircraft turn for home, and not together: 12 km apart on the same track in run 2, which is about a minute |
| K7 | 02:00 Damage level 0, nothing else | both in the list at 16:41 at 4,000 m, 43 and 51 km south-west of their centre | both in the list at 05:02 at 1,200 m, at their centre | level 0 does not kill. They carried on, and turned for home some time after 05:00 |

**Cell T-d4, the fight.**

| Pair | Run 1 | Run 2 |
|---|---|---|
| K8 (Delete at 04:00 while under attack, detector at 04:30) | both removed. One hunter died at 04:14.6 at 108 m with no attacker | **K8b left:** no kill record, no event, in the list at 05:02. K8a removed |
| K9 (one Delete for both at 15:00) | neither had died; both removed at 15:00 | not reached |
| K10 (one Delete per aircraft at 15:00) | K10a died at 07:05.9 at 550 m with no attacker; K10b and the hunter lead shot each other down at 07:47 and 07:48; nothing was left to delete | not reached |

**What this settles.**
- A Delete acts on the aircraft it names. Naming the lead does not remove the wingman.
- A formation that has lost a member deletes cleanly, so "one formation with a missing member" is not what kept flight 1G's aircraft.
- **The flight 1G survivor is reproduced: four survivors in three runs** (G1a in flight 1G; K4a in run 1; K4b and K8b in run 2). Each time the aircraft answered neither the Delete nor, where one was sent, a Damage command at level 2.

**The likely cause (a reading, not yet tested on its own).** Every survivor was an aircraft that had been damaged, or was in a fight where it could have been, and each sortie plane carries `AiRTBDecision = 1`. K6 shows that damaged aircraft turn for home one at a time, about a minute apart. K4's Delete came two minutes after the damage: the aircraft that left first survived, and in each run that was the one that dropped its stores first (`AType:26`: K4a at 02:56 in run 1, K4b at 02:58 in run 2). So the reading is: **once an AI aircraft has decided by itself to return to base, mission commands sent to it no longer reach it.** K5 fits: at level 0 nobody had turned for home by 04:00 and both were removed.

**Not known.** Whether `AiRTBDecision = 0` prevents it. Whether such an aircraft is ever removed (each plane has `DeleteAfterLand = 1`; none had landed by 16:41). Whether it still raises event 13 if it is shot down. Whether a Deactivate reaches it.

Other facts from these runs:
- `Damage = 1` and `Damage = 0` on `MCU_CMD_Damage` are the editor's two lower levels. Neither kills an F-80C. As with level 2, the whole formation gets it.
- `AType:26` (with a new `TID`) is written when an aircraft drops or fires its stores: 47 to 64 s after level 1 damage, and when a fight starts. That it is a drop is a reading: the survivor's end-of-mission record shows no rockets left.
- `AType:24`, `25`, `27` and `31` appeared for the first time; they were not read.
- AI against AI with ace hunters: in run 1 one pair was untouched for 15 minutes and one pair was destroyed in 8. It is no way to set up a test.

## Flight 1L (the app's own logic, no pilot) — 2026-10-13, session `2026-10-13_17-17-19`

Step 0i probe `P14_Probe_1L_traced` (`handoff/p14-probes/flight1l/`), built as `P14_Probe_1L_20261013` without the resaver, hosted by the game client on the second PC (game `261013`). Nobody spawned; the server was left at 24:51. Logs: `src/testdata/missionlog/real_15/` (54 files). One run.

### Cell T-w: what `generate_template` writes (decisions 0012 and 0013)

Each group came from `generate_template` (two independent F-80Cs, no orders) and was started by pulsing its own bring-up node. A Damage command aimed at a unit's entity does reach a spawned copy.

| Group | Wiring | What happened |
|---|---|---|
| W1 | repeat spawn, death event **4** (as generated) | Wave 1 spawned at 01:00.6 and was killed at 03:00.1 (event 13 on both). `DeathCount` never fired. No second wave: nothing died at 06:00 and nothing is in the end-of-mission list. |
| W2 | the same with death event **13** | Wave 1 killed at 03:00.1; `DeathCount` fired at 03:00.1; `COOLDOWN` at 04:00.1; wave 2 killed by the detector at 06:00.1; `DeathCount` and `COOLDOWN` fired again (07:00.1); wave 3 is in the end-of-mission list. |
| W3 | repeat spawn, `Trigger Delete` with Plane indexes (as generated) | Clean-up at 03:02, new wave from 04:00.6, killed at 06:00.1, next wave at 07:00. **The end-of-mission list holds two W3a and two W3b:** the live wave and the first wave, still there, switched off. |
| W4 | the same with `Trigger Delete` listing entities | The same sequence. The end-of-mission list holds one W4a and one W4b: the first wave was really deleted, and the spawner still worked. |
| W5 | Activate mode, Plane indexes (as generated) | Clean-up at 03:02, `MISSION BEGIN` pulsed again at 04:00: both aircraft died to the detector at 06:00.1. The group came back. |
| W6 | Activate mode, entities | The same pulses: nothing died at 06:00 and nothing is in the list. **The group did not come back.** |

**Result for 0012: confirmed.** With event 4 the death counter never counts and no further wave comes; with event 13 the loop runs as designed.

**Result for 0013.** In Spawn mode the corrected Delete removes each finished wave and the next wave still spawns; today's Delete leaves every finished wave in the mission. In Activate mode the corrected Delete ends the group for good at its first clean-up, where today it comes back. So the fix that keeps today's behaviour is: Spawn mode lists entities; Activate mode writes no Delete.

### Cell T-v: every entity event, three ways to end

| Aircraft | What happened | Events raised (type, time) |
|---|---|---|
| V1, killed by a level 2 Damage command at 02:00 | died at once | 12 (damaged) and 13 (killed) at 02:00.04; 0 (pilot killed), 1 (pilot wounded) and 3 (critical damage) at 02:00.10. Never 2 or 4. |
| V2, written with `Fuel = 0.03` | the pilot bailed out at 01:21 (`AType:18`); the aircraft hit the ground at 01:43 | 7 (bingo fuel) at 01:00.6; 12 and 3 at 01:03.6; **13 at 01:19.4**; 2 (crashed) at 01:43.0. Never 4. |
| V3, level 1 damage at 02:00, left alone | dropped its stores at 02:57, flew home, touched down at K14 at 24:07 | 3 at 02:00.10; 9 (bingo bombs) at 02:59.8; **5 (landed) at 24:20.1**. Still in the end-of-mission list 31 s later. Never 13. |

- **Event 13 fires for every way an aircraft was lost** in these flights: a Damage command, AI gunfire (1G), a collision and a crash in a fight (1K), and running out of fuel with a bail-out. **Event 4 has never fired.**
- **Event 7 fired on V1 and V3 at 01:03.6, three seconds after they were activated, with `Fuel = 1`.** What the game means by it is not known. Any logic that sends a flight home on event 7 would act at once. Flight 1M tests this on upstream's strike and CAP.
- Event 5 marks a landing. The aircraft is removed some time after it (`DeleteAfterLand = 1`): see R0b and R3a below.

### Cell T-r: the AI's own decision to return to base

All four pairs took level 1 damage at 02:00. With `AiRTBDecision = 1` each aircraft dropped its stores 61 to 69 s later (`AType:26`); with `AiRTBDecision = 0` none did.

| Pair | `AiRTBDecision` | At 05:00 | What happened |
|---|---|---|---|
| R0 | 1 | Delete of both | R0a removed. **R0b was not:** no kill at the 05:30 detector, and the log shows it on the ground at K14 at 17:43 (`AType:31`). It is not in the end-of-mission list at 24:51, so the game removed it after it landed. |
| R1 | **0** | Delete of both | both removed |
| R2 | **0** | nothing | both stayed in their target area. R2a is in the end-of-mission list there at 1,400 m; R2b died there at 24:38.8 with no command sent. |
| R3 | 1 | Deactivate of both | R3b is in the end-of-mission list at 4,000 m on the way home, where it was at 05:00: the Deactivate reached it. **R3a was not reached:** it touched down at K14 at 22:12 and was gone by 24:51. |

**Result.** With `AiRTBDecision = 1`, one aircraft of each damaged pair stops answering mission commands (Delete, Deactivate, Damage) and one keeps answering; which one varies (K4a, K4b, R0b, R3a: twice the lead, twice the wingman). The one that stops answering flies to the nearest friendly airfield, lands, and is removed by the game a few minutes later. **With `AiRTBDecision = 0` both aircraft stay on their task and both can be deleted.** One pair each, one run.

For Air Tasking this means sortie aircraft are written with `AiRTBDecision = 0`, and going home after damage is done by the sortie's own logic (an event to a waypoint), which keeps the aircraft under control. The earlier reading ("an aircraft that has decided to go home stops taking commands") was half right: it is one aircraft of the pair, not both.

Other facts from this run:
- `AType:18` is a bail-out (`BOTID` is the pilot). `AType:31` is written when an aircraft meets the ground, crashing or landing; `AType:27` came with the two crashes and not with the landings.
- An aircraft with no orders stays within 2 km of where it was spawned.

## Flight 1M (upstream's strike and CAP; the fixes as generated) — 2026-10-14, two runs

Step 0j probe `P14_Probe_1M_traced` (`handoff/p14-probes/flight1m/`), built as `P14_Probe_1M_20261013` without the resaver, hosted by the game client on the second PC (game `261013`). Nobody spawned. The game showed "#31: Master server response timeout" while loading; the local server ran normally.

| Run | Session | Ended | Logs |
|---|---|---|---|
| 1 | `2026-10-14_09-20-47` | left at 13:08 | `src/testdata/missionlog/real_16/` (36 files) |
| 2 | `2026-10-14_09-37-32` | the server reloaded the mission as run 1 was left; it ran unattended for 93 minutes and was left then. Only its first 16 minutes are read (the cells close at 16:00) | `real_17/` (185 files) |

### Cell T-u: upstream's own strike and CAP (commit `a31ad0a`)

Every plane in these files carries `AiRTBDecision = 0`, `Fuel = 1`, an air start at 1,500 m, and abort hooks that send the flight home.

| Group | Run 1 | Run 2 | Result |
|---|---|---|---|
| U1, F-80C strike as written | event 7 (bingo fuel) on all four at 01:03.7; `Goto WP 2` (home) at 01:04.2; `AttackArea` never; all four still flying at 13:08 | the same: event 7 at 01:03.6, home at 01:04.1; then two aircraft died at 01:41.8 with no enemy near | **the strike turns for home 3.6 s after it starts and never attacks** |
| U3, the same strike without its four event 7 hooks | event 7 still raised at 01:03.6 (no hook); `AttackArea` 02:28.4; out of bombs (9) on all four 03:20.2 to 03:20.7; home once at 03:21.2; `Mission Complete` 05:52.4; `MISSION END` 05:52.5 | the same within a few seconds; `Mission Complete` 05:48.9 | control: without the event 7 abort the strike works as designed |
| U2, Il-10 strike as written | no event 7. `AttackArea` 03:03.6. Out of bombs on all four 04:00.9 to 04:01.5; `Goto WP 2` fired at 04:02.0, again at 04:06.3 (U2d critical damage) and 05:34.9 (U2c critical damage; its crew bailed out and it crashed at 05:57). **Never reached home:** the three survivors were 2.5 km from the target at 800 m at 13:08 | no event 7. Out of bombs 04:01.4 to 04:02.2; `Goto WP 2` fired at 04:02.2 and 04:02.7. No `Mission Complete` before the 16:00 close | **the Il-10 strike attacks but does not go home** |
| U4, F-86A CAP as written | event 7 on all four at 01:03.6; `Goto WP 5` (home) at 01:04.1; two aircraft died at 01:53 with no enemy near | the same; two died at 01:52.7 | **the CAP turns for home at once** |
| U5, the same CAP without its event 7 hooks | event 7 raised, no hook; patrol under way (`Goto WP 2` at 01:27.4) | the same | control: the CAP patrols |

**Reading.**
- **Event 7 fires on the jets (F-80C, F-86A) 3.1 s after they are activated, with full tanks, and never on the Il-10.** Upstream's strike and CAP treat it as "go home", so every jet strike and jet CAP aborts at once. Why the game raises it is not known.
- **The go-home step fails when it fires more than once.** Each abort event pulses one timer, `Goto WP 2`, which sends `Force Complete - High` and the home waypoint to the lead. The F-80 control's four "out of bombs" arrived within 0.5 s, the timer fired once, and the flight went home and completed, twice. The Il-10s' arrived over about a second, the timer fired two or three times, and the flight never reached home, twice. The likely mechanism (a reading): the second firing's Force Complete ends the home waypoint the flight is already flying to, and sending the same waypoint again does not restart it, so the flight is left with no orders and stays where it is (flight 1L: an aircraft with no orders stays within 2 km).
- **The sudden abort may cause collisions:** the aborted F-86 CAP lost two aircraft about 49 s after its abort in both runs, and the aborted F-80 strike lost two 38 s after it in run 2, each time with no enemy near. That two aircraft collided is a reading from the timing; it was not watched.

### Cell T-x: the changed generator, as generated

| Group | Run 1 | Run 2 | Result |
|---|---|---|---|
| X1, repeat spawn | wave 1 killed 03:00.1; `DeathCount` 03:00.1; `COOLDOWN` 04:00.1; wave 2 killed 06:00.1; again; wave 3 in the end-of-mission list | the same | **0012 confirmed as generated** |
| X2, repeat spawn, clean-up then respawn | clean-up 03:00; new wave from 04:00.6; killed 06:00.1; **one X2a and one X2b in the end-of-mission list** | the same events | **0014, Spawn mode, confirmed:** the finished wave is gone (flight 1L's unfixed W3 left two of each) |
| X3, Activate mode | no `DELETE DELAY` or `Trigger Delete` in the file; clean-up 03:00; `MISSION BEGIN` again 04:00; both died to the detector 06:00.1 | the same | **0014, Activate mode, confirmed:** the group comes back |

### Cell T-y: return to base, larger groups (run 1; run 2 the same in the events)

| Row | What happened | Result |
|---|---|---|
| Y1, 4-ship placed by `place_air_sortie` (`AiRTBDecision = 0`), level 1 damage, Delete of all four at 05:00 | no event at 05:30, none in the end-of-mission list | **0015 confirmed for a 4-ship:** all four removed |
| Y2, the same 4-ship with `AiRTBDecision = 1` | all four dropped their stores 02:49 to 03:08. At 05:00 the Delete removed Y2d only. Y2a, Y2b and Y2c answered neither the Delete nor the 05:30 detector, flew to K14; Y2c touched down at 12:30 | with the setting on, **three of four** stop answering |
| Y3, one aircraft, `AiRTBDecision = 1` | dropped its stores at 03:06; removed by the Delete at 05:00 | a lone damaged aircraft stays reachable |

So in a damaged formation with `AiRTBDecision = 1`, all aircraft but one stop answering commands (pairs: one of two, flights 1K and 1L; 4-ship: three of four; single: none). Which one stays reachable varies.

### Cell T-j: timers on game 1.005

J5 (a 10 s timer re-triggered at 5 s) fired once, at 09:15.0, 15 s after its first trigger, in both runs. J1 to J3 never fired. **The same as game 1.004 (flight 0):** a re-triggered running timer restarts and fires once. Decision 0016 rests on this.

## Flight 1P (upstream's go-home fix, Fighter Pack's count, event 7 by type) — 2026-10-14, session `2026-10-14_14-52-40`

Step 0k probe `P14_Probe_1P_traced` (`handoff/p14-probes/flight1p/`), built as `P14_Probe_1P_20261014` without the resaver, hosted by the game client on the second PC (game `261013`). Nobody spawned. Left at 14:05; the server did not reload the mission. Logs: `src/testdata/missionlog/real_18/` (36 files). The plan calls this flight "1N"; it was renamed because flight 1N was flown on 2026-10-12 and its files must not change.

Two details differ from the plan's tables, both decided before the build. In cell T-f the 03:00 kill is one level 2 Damage command **per aircraft**, because Fighter Pack aircraft are independent and a command on the lead would kill only the lead (flight 1H). Cell T-b's twelve aircraft come from `bundled_catalog()` (what Template Builder offers), not `builtin_plane_catalog()`, which holds only the seven fighters; the catalog's second B-29 model, `simpleb29`, was left out.

### Cell T-f: Fighter Pack's count (decision 0016)

One generated 4-ship F-80C Fighter Pack flight per row (frozen in `src/testdata/p14_probe_inputs/flight1p/`): `DeathCount` 4, `COOLDOWN` 180 s, `Delay Delete n` 60 s. All twelve aircraft died to the Damage commands at 03:00.0, and each raised 13 once.

| Row | Wiring | `Delay Delete n` | `DeathCount` | Replacement | Result |
|---|---|---|---|---|---|
| F1 | as generated, with 0016 | each once, 04:00.7 | **once**, 04:00.7 | `COOLDOWN` 07:00.7; four aircraft activated 07:01.2 | **0016 confirmed as generated** |
| F2 | F1 without the four event 7 hooks | each once, 04:00.7 | **once**, 04:00.7 | the same | the same as F1 |
| F3 | F2 with the death events back on `DeathCount` (0012 alone) | each once, 04:00.7 | **twice**, 03:00.0 and 04:00.7 | `COOLDOWN` once, 07:00.7 (the second count restarted it) | control: **each kill is counted twice** |

- **The suspected event 7 fault did not happen.** No F1 aircraft, and none of its replacements, reached `CLEANUP n` on its own: no `Delay Delete n` fired except after the kills. So the Fighter Pack flight did not go home on event 7. (Only event 13 was traced on these aircraft; an event 7 would have shown as a `Delay Delete n` at about 02:04.)
- In F3 the double count did not bring the replacement early, because all four died at once and the second count only restarted `COOLDOWN`. With losses spread over time, two aircraft lost would fill the counter of four (decision 0016).

### Cell T-h: upstream's go-home step

Copies of upstream's strike files (`src/testdata/p14_probe_inputs/upstream_a31ad0a/`), placed and started as in flight 1M.

| Row | Change | What happened | Result |
|---|---|---|---|
| H1, Il-10 | none (control) | `AttackArea` 03:03.5; out of bombs (9) 04:02.4 to 04:03.8; `Goto WP 2` fired **four times** (04:03.7, 04:04.3, 04:11.4, 05:22.5), the last two after critical damage (3). H1c's and H1a's crews bailed out (kills at 04:28.4 and 05:30.4, no attacker). No `Mission Complete`; H1b and H1d were still near the target at about 550 m at 14:05 | **the same as flight 1M:** the flight does not go home |
| H2, Il-10 | once-only gate before `Goto WP 2` | out of bombs 04:05.7 to 04:08.6; `Goto WP 2` **once**, 04:06.2; `Mission Complete` 07:28.9; `MISSION END` 07:29.0; all four back at their start point at 1,500 m | **the gate works:** the flight goes home and completes |
| H3, Il-10 | `Goto WP 2` without the Force Complete | out of bombs 04:01.3 to 04:01.6; `Goto WP 2` once, 04:02.1; `Mission Complete` 07:32.6; all four home | went home, but **not tested** for a timer that fires more than once: the four events came within 0.3 s |
| H4, F-80C | event 7 hooks removed, plus the gate | event 7 on all four at 01:03.6 (no hook); `AttackArea` 02:28.4; **no weapon released** (no `AType:25` for H4); `OnAreaAttacked` 04:28.5 when the attack order ran out; `Goto WP 2` 04:29.0; `Mission Complete` 06:28.2; all four home | the whole fix lets a jet strike run to `Mission Complete`; the gate was **not exercised** (no abort event fired) |

### Cell T-b: which aircraft raise event 7

Twelve single aircraft generated by `generate_template` (one seat, Activate mode, air start at 1,500 m, `Fuel = 1`, `PayloadId = 0`, no orders), activated at 01:00.

| Event | Aircraft |
|---|---|
| 7 (bingo fuel) | **none at activation.** B3 (F-86A-5) at 13:58.8, after 13 minutes of flight |
| 9 (bingo bombs) | all twelve at 01:02.7 |
| 8 (bingo main guns) | B5 (B-29), B6 (C-47B), B12 (Li-2T) at 01:02.7 |
| 13, 5 | none |

**Reading.** Event 7 at activation is not a property of the aircraft type.
- **Raised it at once:** upstream's F-80C strike (`PayloadId = 5`, flights 1M and 1P), upstream's F-86A CAP (`PayloadId = 0`, flight 1M), and flight 1L's single F-80Cs V1 and V3 (`PayloadId = 4`, `AiRTBDecision = 1`, air start at 3,050 m).
- **Did not:** B1 (F-80C) and B3 (F-86A) here, and the Fighter Pack F-80Cs (`PayloadId = 0`, air start at 1,250 m).

Every F-80C that raised it carried a payload, but the F-86A CAP did not, so the payload alone does not explain it. **Why the game raises event 7 is still not known.** Events 8 and 9 at activation mark an aircraft that has no guns of that kind, or no bombs.

## Flight 2 (Probe 2: sortie shell, budget, loss with no damage) — 2026-10-14, session `2026-10-14_20-22-34`

Step 3b probe `P14_Probe_2_traced` (`handoff/p14-probes/flight2/`), built from `codex/p14-3b` (`129bf0d`) as `P14_Probe_2_20261014` without the resaver, hosted by the game client on the second PC (game `261013`). Nobody spawned. The mission ended at 47:04, after the 46:00 close, when the game left the server; it was not reloaded. Logs: `src/testdata/missionlog/real_19/` (97 files). Replay: no dead links.

### Cells T-k and T-k1: the head-count budget in the game

Stand-in copies built with `budget.rs`; each request is a timer pulse into a copy's `START`.

| Cell | Request or deletion | Planned result | Observed | Result |
|---|---|---|---|---|
| T-k, N = 2 | requests 03:00 and 03:02 | both start | SN01 and SN02 `START`; at 03:02.14 the census reaches 2 and `COUNT` fires | as planned |
| | requests 03:04 and 03:06 | dropped | no `START` | as planned |
| | both copies deleted at 03:30 (one tick) | gates reopen | `REOPEN OUT` at 03:30.5 | as planned (review finding C1 fixed in the game) |
| | requests 03:32 and 03:34 | both start | SN05 and SN06 `START`; `COUNT` at 03:34.34 | as planned |
| | request 03:36 | dropped | no `START` | as planned |
| T-k1, N = 1 | two requests in one tick, 04:00 | 1 or 2 start | **both start** (SN01, SN02) | the documented overshoot: **the game passes two same-tick pulses through an open gate** |
| | request 04:05 | dropped | no `START` | as planned |
| | one copy deleted at 04:20 | gates stay shut (one copy still exists) | census finds SN02 `FLYING`; no `REOPEN OUT` | as planned |
| | request 04:22 | dropped | no `START` | as planned |
| | every other copy deleted at 04:30 | gates reopen | `REOPEN OUT` at 04:30.4 | as planned |
| | request 04:32 | starts | SN05 `START` | as planned |

Every recount ran `CHECK`, `CENSUS` (+0.04 s), the `FLYING` answers 0.04 s apart, and `END`, with the times `budget.rs` writes. `AGAIN` never fired: no event arrived during a recount.

### Cells T-s1 and T-s2: the sortie shell

One placed HVAR sortie (started 01:00) and one placed AirAlert sortie (started 01:30), each with a stand-in area switched HOT at 10:00.

| | `START` | Activated | `DONE HUB` | `DELETE DELAY` | `EXTEND` | Aircraft at the end |
|---|---|---|---|---|---|---|
| T-s1 HVAR | 01:00.0 | 01:00.5, 3,050 m | **05:46.4** | 06:47.0 (+60.6 s) | 25:46.8, into a closed hub | none: deleted |
| T-s2 AirAlert | 01:30.0 | 01:30.5, 3,050 m | **06:09.1** | 07:09.7 (+60.6 s) | 34:10.0, into a closed hub | none: deleted |

- **Both sorties ended the moment they reached WP 1.** Transit 286 s and 279 s, against the plan's 300 s (within ±20 %). Nothing else could have fed `DONE HUB` then: no loss, the hard stop was far off, and the area's `COLD` relay acts only at the end of on-station time. So **Zone Out fired as soon as WP 1 armed it, because no aircraft of the other coalition was within 35 km of the target.** This is what §3.2 wires (Zone Out ends the sortie when no enemy is near the target), but it means **a sortie that arrives at an empty target turns for home at once.** It needs a ruling before step 3 merges (see "Open").
- `ON STATION` still started on arrival and ran its time; `EXTEND` then pulsed a `DONE HUB` that was already closed, and nothing happened. The latch works as designed.
- **Deletion works:** `DELETE DELAY` came 60.6 s after `DONE HUB` in both sorties, as §3.2 states, and none of the eight aircraft is in the end-of-mission list (decision 0017's own Delete, listing entities).
- Not answered, because the sorties never stayed on station: the AttackArea on target, AirAlert's two AttackAreas (D28), what the aircraft do during the extension (risk v), and the RTB turn.

### Cell T-l1: an aircraft lost with no damage

One F-80C, `Fuel = 0`, `AiRTBDecision = 0`, written at 150 m. The ground at its start point is about 300 m high, and `YPos` is height above sea level, so the aircraft started below the ground: it was in the air (`AType:28`, 305.7 m) at 01:00.0, met the ground (`AType:31`) at 01:00.5, raised event 2 (crashed) at 01:00.9 and was destroyed with no attacker (`AType:3`, `AID:-1`) at 01:03.0. **Event 13 fired once and `LOSSES` (Counter 1) counted it, at 01:03.0.** Events 0 and 5 never fired.

So event 13 fires for a crash with no damage and no attacker. The crash was an impact at the start rather than a glide into the ground; a dead-stick glide has not been seen.

### Open

1. **Zone Out on an empty target. Ruled 2026-10-14, decision 0018:** a sortie stays on station at an empty target; Zone Out ends it only after an enemy has been seen inside (the template's Zone IN is kept for that). Flight 2B re-flies the sortie cells. The original note: As built, a sortie whose target has no enemy aircraft within 35 km ends on arrival. Either that is intended (a sortie only stays while there is something to fight), or Zone Out should only end a sortie after an enemy has been seen inside it, which costs MCUs. Flight 2 could not test the on-station part of the shell either way: a re-fly needs an enemy AI flight over each target (AI aircraft trigger check zones, flight 0).
2. **The load file** (24 copies per side) was not flown here: this PC is a test bed, not the real host. It went to the upstream author, who hosts the server: https://github.com/eszyman/IL2MissionUtility/issues/2 (mission text log and Tacview recording requested).

## Step 0 after the flights of 2026-10-12 to 2026-10-14

| Risk | Answer | From |
|---|---|---|
| a | Players and AI both trigger a check zone | flights 0, 1A |
| c, re-check, h, o | confirmed; a pulsed zone fires once per entry into an empty zone | flights 1A, 1N |
| g | **changed:** a lost aircraft raises event 13 (and 0, 2), never 4; Delete raises none | flight 1G |
| n | AI F-80Cs on the built-in template: 733 km/h, against the plan's 732 | flights 1N, 1D |
| country filter (decision 0008, option 4) | works on both sides, for AI and players; country 602 is usable for NATO AI | flights 1N, 1D |
| new | **with `AiRTBDecision = 1`, all but one aircraft of a damaged formation stop answering Delete, Deactivate and Damage commands** (one of two, three of four), fly home, land and are removed by the game. With `AiRTBDecision = 0` every aircraft stays on task and can be deleted (pair and 4-ship) | flights 1G, 1K, 1L, 1M |
| g, again | event 13 fires for every kind of loss tried (command, gunfire, collision, crash, fuel and bail-out); event 4 never. In `generate_template`'s respawn loop the counter only works on 13 | flights 1G, 1K, 1L |
| new | `Trigger Delete` with entities: fine in Spawn mode, ends the group for good in Activate mode. The fixed generator (decisions 0012, 0014) works as generated | flights 1L, 1M |
| upstream | event 7 fires on upstream's F-80C strike and F-86A CAP 3 s after activation, and they abort on it at once. A go-home step that fires twice leaves the flight with no orders | flight 1M |
| upstream, fix | a once-only counter before the go-home timer makes the Il-10 strike go home and complete; without its event 7 hooks the F-80C strike also completes | flight 1P |
| new | event 7 at activation is not a property of the type: twelve single aircraft as Template Builder writes them, and the Fighter Pack F-80Cs, did not raise it. The cause is not known | flight 1P |
| new | decision 0016 confirmed: a Fighter Pack flight counts each loss once, a clean-up delay late; with the death event on `DeathCount` it counts each loss twice | flight 1P |
| new | a dead aircraft in a Delete's list is harmless; a Delete removes the aircraft it names, not the formation | flights 1J, 1K |
| new | a Delete that lists Plane indexes (as Template Builder and the built-in sorties write it) removes nothing; a Delete that lists entities works | flight 1H |
| new | a Damage command aimed at one aircraft kills its whole formation | flight 1H |
| new | the head-count budget works in the game as `budget.rs` writes it, at N = 2 and N = 1; two same-tick requests both pass an open gate (the documented overshoot), and the next recount keeps the gates shut until both copies are deleted | flight 2 |
| new | a sortie whose target has no enemy aircraft within 35 km ends at WP 1 arrival (Zone Out fires at once); the shell's own Delete removes all its aircraft 60.6 s after `DONE HUB` | flight 2 |
| g, again | event 13 fires once for a crash with no damage and no attacker (event 2 first); events 0 and 5 do not | flight 2 |

Not tested by any flight: two or more players (step 12), and a Complex Trigger with more than one `Country` line.
