# 0039. Map undo keeps the Air Tasking plan only for the Air dock's own edits

- **Status:** Accepted
- **Date:** 2026-10-28
- **Decided by:** the user, by approving Claude's step 9 plan ("Approved. Build step 9 as planned; Load clears the pool and keeps the library and settings (spec §6.2)."). The plan named this choice as a change from §6.2.

## Context

Plan §6.2 (`handoff/P14-air-tasking.md`) says to add the `AirPlan` to `struct MapForces`, `map_forces()`, `restore_map_forces`, the Map fingerprint in `settle_undo`, and `map_fingerprint`. Read literally, every Map undo snapshot would then carry a copy of the plan, and every Map Ctrl Z would restore it.

Map's Ctrl Z restores one snapshot, taken before a Clear, Remove, AO change or Air dock edit. Air Tasking settings (budget, schedules, threshold, categories and so on) are edited on the Air Tasking tab, which has its own undo (`air_undo`).

## Decision

1. `MapForces.air` is an `Option<AirPlan>`. Only `record_map_air_undo` sets it; the Air dock tab calls that before "Auto-fill from objectives" and before each ✖. Every other Map snapshot has `air: None`.
2. `restore_map_forces` restores the plan only when the snapshot holds one. This is the same pattern as `MapForces.lines`.
3. `settle_undo` counts the plan in Map's fingerprint only while the pending snapshot holds one. A later Air edit therefore drops an Auto-fill or ✖ undo, instead of letting Ctrl Z throw that edit away. It leaves a Clear undo alone, because restoring that snapshot does not touch the plan.
4. `map_fingerprint`, the unsaved-Map check, counts the plan only while it is included in the base map, which is the only time it changes the generated file.

## Rejected

- **The plan in every Map snapshot (the literal §6.2 reading).** Take a Map Clear, then a change to the DPRK budget on the Air Tasking tab, then a return to Map and Ctrl Z. That would bring back the objectives and also silently undo the budget change, made on another tab with its own undo. Counting the plan in every Map fingerprint, as that reading also asks, would instead drop the Clear undo after any Air Tasking edit, so the user would lose the Clear's undo for an unrelated change. The plan is also cloned into every Map snapshot for no use.
- **No plan in Map undo at all.** Then Map's Ctrl Z could not undo the dock's Auto-fill or ✖, which §6.2 requires.

## Consequences

- Tests:
  - `map_undo_restores_air_pool_after_auto_fill` and `map_undo_restores_air_entry_after_remove`: the dock's edits undo with Map's Ctrl Z.
  - `map_undo_of_a_clear_keeps_later_air_settings`: Clear, then an Air edit, then Ctrl Z brings back the objectives and keeps the edit.
  - `map_undo_is_dropped_by_a_later_air_edit`: after Auto-fill, a weight edit drops the snapshot.
  - `map_unsaved_follows_the_air_plan_only_when_included`.
- §6.2's text is unchanged. This record is the reading that applies.
