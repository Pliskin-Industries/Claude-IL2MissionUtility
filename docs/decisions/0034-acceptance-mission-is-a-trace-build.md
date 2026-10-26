# 0034. The step 11 acceptance mission is observed by a trace build, not debug subtitles

- **Status:** Proposed
- **Date:** 2026-10-26
- **Decided by:** not yet (the user). Options and recommendation by Claude, from the step 7 scope (`handoff/sessions/2026-10-26-p14-step7.md`). Decision 0030 left this to step 7.

## Context

Plan §8 step 7 has the ignored writer `write_p14_acceptance_mission` set `AirPlan.debug_subtitles = true`, so the clock and the zone show subtitles for slot open, winner, `START`, the end-of-station result, the `DONE HUB` cause, and the zone's chance, fire, refusal, cooldown and re-arm. Step 11 reads its checks "from the replay report first … and from the debug subtitles and video only where the log cannot show it".

No builder emits a subtitle (steps 5 and 6 left them out, 0030). A subtitle needs a locale string, and `MapAirPack` carries only a tree, so subtitles would need a locale side channel through the frozen types. The trace build (step 0T, `trace::instrument`) needs neither: it adds Mission Objective breadcrumbs that the server text log records, and every one logged in flight 0 (t1). Flights 2 to 2J were all read this way.

## Options

- **A. Debug subtitles as planned.** Subtitle MCUs in the clock and the zone when `debug_subtitles` is set, plus a way to carry their locale strings out of `build_air_packs`. More MCUs in the test mission only; a type change.
- **B. A trace build.** The writer generates the base map with the acceptance plan, instruments it with `trace::instrument` (the `CLK`, `ZN`, `TH`, `SN … START`, `DONE HUB`, `EXTEND`, `DELETE DELAY` names and the decision points), and writes the `.Group`, its locale and the trace sidecar under `target/p14/`. The replay report gives exact times. `debug_subtitles` stays unused (the field is frozen; step 9 removes it with the other P14 allows, or keeps it for later).
- **C. Both.**

## Recommendation

**B.** It is the flown method, adds nothing to user output, and needs no type change. A subtitle shows only what the player happens to see; the log shows every pulse.

## Consequences

- The §8 test `acceptance_mission_has_zone_entries_and_debug_subtitles` becomes `acceptance_mission_has_zone_entries_and_trace`: per side at least one `Zone` and one `Both` entry's `ZN` cell, every named breadcrumb present, and `validate(acceptance_test_plan())` with exactly one D30 warning and no error.
- Step 11's text loses its subtitle checks; each becomes a breadcrumb in the replay report. The mission is built from the `.Group` with `tools/p14/build_probe_mission.py --no-resave`, as the probes are.
