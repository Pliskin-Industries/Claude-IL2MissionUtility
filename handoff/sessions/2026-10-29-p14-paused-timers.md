# 2026-10-29 — P14 paused timers: the fix plan (hand-over)

**Status:** probe flight 2K flown and recorded; the fix is designed, not built. Branch `claude/p14-paused-timers` (worktree `.claude/worktrees/p14-acceptance-a2`), cut from `claude/p14-acceptance-a2` at `51228f2`. Not pushed. `STATE.md` is left to the coordinating session.

## How we got here

- **Flight A2** (`handoff/sessions/2026-10-28-p14-acceptance-a2.md`, results) saw each zone's `MISS WAIT` fire 0.88 s after its `ARM`, which gave a second cooldown after every FIRE.
- The user ran a design session (grill-with-docs) and chose to probe first: "probe it first until the root cause is accurately identified".
- **Probe flight 2K** (`handoff/sessions/2026-10-29-p14-flight-2k.md`, `handoff/P14-probe-results.md` "Flight 2K", facts file "Found on 2026-10-29 (flight 2K)") settled it:
  - a switched-off running timer is **paused**, and switching it on **resumes** it with the time it had left;
  - a pulse restarts a resumed timer, and a pulse while it is off is dropped;
  - a failed random roll does not leave a running timer running;
  - a pulsed check zone stays **armed**, and switching it on is a check; a zone never pulsed does nothing.

## Commits on this branch

1. `68c8b18` Trace build: a switch's breadcrumb is pulsed by its inputs (decision 0043). A2 is frozen as flown.
2. `7c10bbb` Probe flight 2K (writer, frozen files, run sheet).
3. `02278d7` 2K flown: logs `real_49`, Tacview, results, facts.

## Decisions taken in the design session (the user's answers)

| # | Question | Answer |
|---|---|---|
| Q1 | Treat "paused" as fact, or probe? | Probe first (done: 2K) |
| Q2 | Terms | switch off / on, paused, resumes, cancel = the D11 relay; armed (check zones) |
| Q3 | `ARM` breadcrumb | Fix it (done: decision 0043) |
| Q4 | 2K cells | K1–K6 as built; light load (one AI pair); piloting allowed if needed |
| Q5 | Where the terms go | `USER_MANUAL.md`, under Troubleshooting |
| Q6 | Switch breadcrumbs | Option A, fed from inputs (done) |
| Q7 | Order of work | Breadcrumb fix → 2K → results → fix |
| Q8 | A2 byte check | Dropped; A2 frozen as flown (done) |
| Q9 | Zone `MISS WAIT` fix | **A: restore D11.** A zero-second `ZNii MISS WAIT OUT` between `MISS WAIT` and its targets (`COOLDOWN`, `DISARM`); `DISARM`, `ARM` and the entry's exhaustion switch the relay, never `MISS WAIT`. +1 MCU per zone entry (11 → 12) |
| Q10 | Clock `ATTEMPT END` fix | **Open.** Proposed: `WIN` and `REROLL ON` stop switching `ATTEMPT END`; it runs out into `REROLL` (already switched off by `WIN`), which drops the pulse. The user is "nervous about timers running out without being stopped". Answer given: nothing can stop a running timer (switch-off pauses it, 2K). The D11 relay is the safe way, and `REROLL` is that relay here. An extra `CLK s ATTEMPT END OUT` relay would cost 1 MCU per side; Claude recommends against it. **Ask the user before building Q10.** |
| Q11 | Clock `RANDOM` timers | No change (K4: a failed re-roll stops a running timer; `DRAW` re-pulses 0.3 s after `RANDOM ON`, with at least 0.5 s left). **Test first**: a walker test that pins it |
| Q12 | Threshold `CLOSE DELAY` one-tick edge; `INNER PULSE` | No change; document in 0044 (`INNER PULSE` is needed for the first check: a never-pulsed zone does not check on switch-on) |
| Q13 | Walker | Timers: add "paused, resumes on switch-on" as the flown setting for the builder tests (shell, clock, zone, budget, threshold), in this change. Check zones (pulse arms and stays armed; switch-on of an armed zone is a check; switch-off keeps the watch): a **second commit** on this branch |
| Q14 | Terms | A short "IL-2 timer and check-zone terms" part in `USER_MANUAL.md` Troubleshooting; the plan's D11 row (`handoff/P14-air-tasking.md` §2, line ≈124) marked as revised by 0044 |
| Q15 | In-game proof | Probe flight **2L**, no pilot, after the fix: an AI pair on 501 orbiting one NATO area (AI on the player country triggers the area, as in 2J); zone chance 100 %, a short cooldown, a clock entry. Show a zone rolling again after **one** cooldown, and two clock slots in a row. Light load |

## Remaining work, in order

1. **Ask the user Q10** (an extra `ATTEMPT END OUT` relay or not).
2. **Walker (TDD).** In `src/airtask/testkit.rs`, add `DeactivateRunningTimer::Pauses` (store the time left when switched off, reschedule it when switched on). Make it the flown setting where the builder tests use their flown configs: `shell.rs` `flown_configs()`, `clock.rs` (≈518, 2078), `zone.rs` and the others. Leave `probe.rs`'s historical tests on their settings. Then write the failing tests:
   - a zone that fired and cooled down rolls at its next pass (fails today: `MISS WAIT` fires 0.9 s after `ARM`);
   - after a won slot, the next slot runs one attempt with no early redraw (fails today);
   - a paused `RANDOM` cannot come up out of turn (should pass: Q11).
3. **Fix:**
   - zone (Q9), in `src/airtask/zone.rs` ≈110–170 (roles, links, `pass_subscribers` stay on `CHANCE` and `MISS WAIT`; exhaustion targets `MISS WAIT OUT`);
   - clock (Q10 as decided), in `src/airtask/clock.rs` ≈305 (`REROLL ON`) and ≈321 (`WIN`), and the comment at ≈365.
   - Update the trace role lists (`export.rs` ≈306: the zone roles traced) so `MISS WAIT OUT` is the miss breadcrumb.
   - Fix the tests and counts that pin the old wiring (zone MCU count 11 → 12, totals).
4. **Records:**
   - decision **0044** (the next free number; check branches first), superseding the "J1 cancels" parts of 0029 (clock) and 0030 (zone);
   - plan §3.7 and §3.8 "As built" notes and MCU counts, and D11;
   - `USER_MANUAL.md` terms (Q14);
   - `docs/decisions/README.md`.
5. **Walker check-zone model** (Q13, second commit). Expect threshold test times to move from 120.1 / 160.1 s to `HOLD` exactly.
6. **Checks:** `cargo test --offline`, `cargo build`, `cargo clippy` (warnings no higher than 81 / 306), and the two-axis code-review skill on the diff. Commit per step. No push.
7. **Probe 2L** (Q15): writer, frozen files, run sheet with predictions, build with `tools/p14/build_probe_mission.py --no-resave`, copy into the Dogfight folder (approved with the plan), host and spectate with Tacview (LCtrl+R), logs to the next free `src/testdata/missionlog/real_NN` (`real_50`), replay, results, commit.

## Also pending for the user (not this branch's work)

- **Decision 0042 (Proposed):** the DPRK air start is 502, but Air Tasking watches 501. New evidence from the session "Check Template Builder's aircraft death event": upstream commit `571504a` (not merged) makes harvested DPRK airfields default to 502 as well. So option A (the air-start constant) does not cover harvested fields; options B extended to harvested fields, or C, would. Add this to 0042 when the user's choice is recorded.

## Traps met in this session

- **Don't press LShift+A after an air start:** the helper is already on, and one press switched it off and crashed the Yak (A2 run 1).
- The Create page reads the mission list when it opens; the server rotation keeps the last mission.
- **Leaving after the results screen can hang at "0 % please wait".** Copy the logs, then `(Get-Process IL2Series).CloseMainWindow()` closes the game cleanly.
- The live `.acmi` is locked while recording.
- A live log reader that maps breadcrumb positions to trace names helps. The scratch version is not in the repo; rebuild it from the `.trace.json`.
