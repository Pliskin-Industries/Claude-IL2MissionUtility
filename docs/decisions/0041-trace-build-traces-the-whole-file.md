# 0041. A trace build from the UI traces decision points across the whole file

- **Status:** Accepted
- **Date:** 2026-10-28
- **Decided by:** the user, approving Claude's step 9T plan: "Approved. Build step 9T as planned: Map traces decision points across the whole file, plus the P14 names."

## Context

Step 9T (`handoff/P14-air-tasking.md` §8) adds a header "Trace build" checkbox: Generate then calls `trace::instrument` with `decision_points()`, "plus the P14 names on the Air Tasking and Map tabs".

The A1 acceptance build (`src/airtask/export.rs`, `traced_acceptance_mission`) went further and scoped the decision points to the `Air Tasking` group, so that the Map's existing packs (ships, ground drop-ins) got no breadcrumbs only because they hold a counter or a check zone. The spec does not say which reading the UI should follow on the Map tab.

## Decision

1. On every tab, a trace build uses `TraceSelect::decision_points()` over the whole generated file: every `MCU_CheckZone`, `MCU_Counter`, `MCU_TR_MissionBegin`, `MCU_Spawner` and every timer with `Random < 100`.
2. On the Air Tasking and Map tabs it adds the P14 role names, as `TraceSelect::indexes` from `airtask::trace_role_indexes(root)`. That function holds the role list the A1 build used (slot opens, GO, the zone and threshold steps, and each sortie's START to DELETE DELAY). It looks only inside groups named `Air Tasking`.
3. The A1 acceptance test keeps its own Air Tasking-only scoping of the decision points. It now takes the role names from `trace_role_indexes`, so the two cannot drift apart.

## Rejected

- **Decision points only inside Air Tasking on the Map tab (the A1 build's scoping).** A trace build of a base map without Air Tasking would then trace nothing, and faults in the Map's other packs would not show in the replay. A base map has few MCUs outside Air Tasking (the A1 mission: about 115 decision points in all), so the extra breadcrumbs cost little.
- **P14 names on every tab.** Only the Map and Air Tasking tabs write an `Air Tasking` group.

## Consequences

- `ui_tests::map_trace_build_includes_air_tasking_roles` checks that the sorties' START and DONE HUB timers and the check zones get breadcrumbs.
- `airtask::export::tests::trace_role_indexes_name_p14_roles_inside_air_tasking` pins the role list.
- With the box off, no Generate output changes (`trace_checkbox_off_output_matches_baseline`, `existing_outputs_match_baseline`, `map_generate_without_air_matches_baseline`).
