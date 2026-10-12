# 0009. The 2026-10-12 flight counts as flight 1A; gate G1 is met

- **Status:** Accepted
- **Date:** 2026-10-12
- **Decided by:** the user

## Context
Gate G1 holds P14 step 4 (the threshold cell) until flight 1A's results are in (`handoff/P14-air-tasking.md` §10). The plan expected the user to fly 1A from a takeoff at K14.

On 2026-10-12 Claude flew the flight 1A probe on the second PC as a flying check: air start over K14, level-flight helper, 4560 m. The log answered every T-c cell and ZA4. Results: `handoff/P14-probe-results.md`, last section. Logs: `src/testdata/missionlog/real_5/`.

## Decision
That flight counts as flight 1A. Gate G1 is met, and step 4 may start.

Settled by it: c (a zone fires on Activate plus a pulse with the player inside), the re-check (a zone can be used twice), h (deactivating a waiting zone cancels it), o (a pulsed zone fires once), and that a zone fires for a player.

## Rejected
- **Fly 1A again from a takeoff before step 4.** The takeoff does not change any zone result, and the zones are cylinders, so the height does not either.

## Consequences
- **Still open from flight 1A:** risk n (F-80C cruise speed), which was not measured, and the country-filter cells Y1 to Y3, which were not in the build. They go into the reshaped probes with flight 1B.
- **One limit to remember:** that the player was inside C4 rests on the cockpit map, not on a log line.
- The step 4 strategy defaults that depend on these results are set when Fable scopes step 4.
