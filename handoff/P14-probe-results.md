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
| z | not confirmed: `PresenceCheckImpl::Closer0Race` is **not** available as a fallback for c. |

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
