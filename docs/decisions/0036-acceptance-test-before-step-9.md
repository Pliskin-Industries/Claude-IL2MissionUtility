# 0036. Fly the Air Tasking acceptance test before step 9

- **Status:** Accepted (option A)
- **Date:** 2026-10-27
- **Decided by:** the user, 2026-10-27 ("Test now."), on a recommendation by Claude after PR #33 (steps 7 and 8) reached `main`.

## Context

Plan §10 orders the remaining work 9 → 9T → 10 → 11: the step 11 in-game acceptance test comes after the Map dock Air tab (9), the trace and replay UI (9T) and the docs (10).

After step 7, everything the game will run already exists:

- `build_air_packs` builds the whole Air Tasking logic: both sides, all six built-ins, clock, threshold, zone, budget, shell.
- The ignored writer `write_p14_acceptance_mission` (step 7, decision 0034) writes the acceptance mission as a trace build, with its `.trace.json`.
- The replay command line (step 0T) reads the server log against that sidecar.

Steps 9 and 9T change only the user's path to that mission: Map Generate adds the packs to the user's map file, and the UI shows the trace and the report. They do not change what `build_air_packs` writes. Steps 3 to 6 were each flown on their own in probe flights; their parts have never run together in the game.

## Options

- **A. Test now, in two stages.**
  1. Flight A0: Claude flies the acceptance mission on this PC with no pilot. It checks that the mission loads, that the clock opens its slots on time, and that nothing launches over an empty area.
  2. The step 11 piloted sessions: the user, solo, as plan §8 step 11 and U14 describe.
  Then step 9. After step 9, one short load check of a mission made in the app (Map Generate with the Air Tasking pack) covers the wiring.
- **B. Keep the plan order.** Build 9, 9T and 10 first, then fly step 11.

## Choice

**A.** A logic bug found now is fixed before UI work is built on it. Step 9 does not change the MCUs, so the test is not repeated after it, apart from the short load check.

## Consequences

- Step 11's "trace build (step 9T)" is met by the step 7 writer and the replay command line; its checks are read from the command-line report.
- The flown inputs are frozen in `handoff/p14-probes/acceptance-a0/` (`P14_Acceptance_1.Group`, `.eng`, `.trace.json`), as for every probe flight. Flight names stay unique: A0 for the no-pilot check, then A1 onward for the piloted sessions.
- The piloted file needs player air starts (`--spawns`), chosen with the user before those sessions; the Air Tasking content stays the same.
- If A0 or the piloted sessions find a logic bug, it is fixed and re-flown under a new flight name before step 9 starts.
