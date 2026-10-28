# 0038. A used sortie copy goes quiet at DONE HUB

- **Status:** Accepted
- **Date:** 2026-10-28
- **Decided by:** the user ("Leftover timers and MCUs can cause all kinds of trouble that is unexpected"; then "Go ahead with the plan: extend DONE HUB OFF, fold WATCHDOG OFF, freeze 2F/2H/2I and A0/A1, decision 0038"), on Claude's plan

## Context

Acceptance flight A1, run 2 (2026-10-28; logs `real_46` and results in the A1 session note, `handoff/sessions/2026-10-27-p14-acceptance-a1.md`). The DPRK copies `SD04` and `SD05` ended together at 1,901.86 s (`Zone Out` → `DONE HUB`) and were deleted at 1,962.46 s, as designed. After that, the copies kept logging:
- `Zone IN` for both copies in one tick, at 2,035.12 s.
- `SD04 EXTEND` at 2,061.92 s.
- `SD05 ON STATION` and `SD05 COLD` at 2,215.78 s.
- `HARD STOP` would have come later.

Each late pulse ended at the latched `DONE HUB` or at a breadcrumb, so none was harmful. The cause is in `wrap_sortie`: `DONE HUB OFF` deactivated `DONE HUB` only. Nothing switched off the copy's own timers and zones, so `ON STATION`, `EXTEND` and `HARD STOP` ran out on schedule. `Zone IN` (fired once on arrival) was pulsed again by `WP 1` after `DONE HUB`, then waited for the next NATO plane. Most likely `WP 1` fired when `Force Complete` ended the waypoint, or at the Delete. `WP 1` has no breadcrumb, so the log cannot tell which. A late `WP 1` also re-runs the attack timers, which re-issue the attack order while `RTB DELAY` (also 0.5 s) sends the flight home.

Flight 0 had already settled what a Deactivate does in the game:
- J1: a running timer that is deactivated never fires.
- C4: a zone that was activated, pulsed and then deactivated never fired when the player entered later.
- E5: a deactivated counter ignores its inputs.

The shell tests missed the late firings. They ran the walker with `DeactivateRunningTimer::RunsOn`, and four tests asserted the late `COLD`, `EXTEND` and `HARD STOP` as expected.

## Decision

1. **`SNjj DONE HUB OFF` also switches off the copy**, in the tick `DONE HUB` fires: `HARD STOP`, `ON STATION`, `EXTEND`, `LOSSES`, `Zone IN`, `Zone Out`, `PULSE OUT`, and every timer in the copy that gives an attack order (it targets an `MCU_CMD_AttackArea`). That is `AttackArea 1`, plus `AttackArea 3` or `5` in two-flight templates. AirAlert has two `AttackArea 1` timers: `WP 1` starts both 37 and 39, and `SortieShape.attack_timers` holds only 39, each lead's last one. No MCU is added: a copy still holds the shell's 19.
2. **Left on, on purpose:**
   - `HOT` and `COLD`: only `ON STATION` pulses them, and the threshold's `HOT ON` / `COLD ON` turn them back on anyway.
   - `ON STATION IN` and `Zone Out ReActivate`: only nodes that are now off feed them.
   - The end chain (`MISSION END`, the orders, `Force Complete`, `RTB DELAY`, `Deactivate Units`, `DELETE DELAY`, `DELETE`, `LANDED`), `FLYING` and the budget: they must still run.
   - `WP 1` itself: deactivating a waypoint a flight is flying to has not been flown.
3. **The Watchdog variant** (not the default, never flown) puts `WATCHDOG`, `WATCHDOG POLL` and their two OUT gates on the same list. `WATCHDOG OFF` is removed, so the variant adds 4 MCUs instead of 5.
4. **Flown files freeze, as under 0018, 0019, 0020/0022 and 0025.** Probe flights 2F, 2H and 2I join `Flight::is_frozen`: the byte check drops from 52 to 40 files, and a new test parses them and checks their links. Acceptance flights A0 and A1 are frozen in `export_acceptance_flights.rs`: `acceptance_flights_regenerate_byte_for_byte` is replaced by `frozen_acceptance_files_parse_and_links_resolve`. Both frozen-file checks share `testkit::assert_frozen_files_resolve`. One byte test is adjusted rather than dropped: `flight2e_and_2g_change_only_the_flown_pairs_density_bytes` copies the current `DONE HUB OFF` Targets onto the flown copy in memory before it compares, so the test still isolates the density change; it first asserts that the flown list was the hub alone. The frozen files are not edited.
5. **No probe flight.** The next acceptance flight checks it: after a copy's `DONE HUB`, nothing from that copy may log except `DELETE DELAY`.

## Rejected

- **A new `SNjj QUIET` Deactivate off `DONE HUB`.** Same effect for one more MCU per copy. `DONE HUB OFF` is already a Deactivate fired in the same tick.
- **Also switch off `HOT`, `COLD`, `ON STATION IN` and `Zone Out ReActivate`.** No effect: nothing live can pulse them. The threshold would switch `HOT` and `COLD` back on.
- **Deactivate `WP 1` itself.** Not flown, and the flight may still be on it when the copy ends.
- **Leave it as it is.** Harmless today (the hub is latched and copies are one-use), but the user rules that a used copy must be fully quiet before step 9.
- **Keep the old shell behind a switch so flown files still regenerate.** A second code path kept only for tests (decision 0025's rule).

## Consequences

- Generate output changes for every Air Tasking copy: only each copy's `DONE HUB OFF` Targets grow, by 8 entries, or 9 where the copy has two attack timers (the two-flight templates and AirAlert). In the A1 mission that is the 18 `DONE HUB OFF` blocks (`SD01` to `SD06`, `SN01` to `SN12`): 1 → 10 in `SD01` to `SD06` and `SN01` to `SN03`, 1 → 9 in `SN04` to `SN12`. MCU count is unchanged (2,978); the `.eng` is unchanged; the trace sidecar's entries are unchanged, and only its `edges` list grows.
- The shell's walker tests of what happens after `DONE HUB` now use the flown `Cancels` setting; the other shell walker tests still sweep every setting. `shell_used_copy_is_quiet_after_done_hub` replays A1's two orders of events in the Il-10 copy A1 flew and in AirAlert, with a late `WP 1` both at `Force Complete` and at the Delete.
- A kill in the 60 s between `DONE HUB` and the Delete no longer reaches `LOSSES` or its breadcrumb. The text log still records the kill.
