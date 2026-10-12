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
