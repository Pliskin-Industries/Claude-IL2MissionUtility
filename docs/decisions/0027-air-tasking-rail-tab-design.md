# 0027. The Air Tasking rail tab (P14 step 8): layout, adding entries, and behaviour before step 7

- **Status:** Accepted
- **Date:** 2026-10-18
- **Decided by:** the user, on Claude's mockup and recommendations

## Context

Step 8 builds the 7th rail tab, Air Tasking (Ctrl 7), before the backend steps 4 to 7. Plan §6.1 (`handoff/P14-air-tasking.md`) was written before the UI redesign settled, and it leaves gaps. Claude made a mockup of the tab at the 1280 × 800 minimum window from the redesign tokens and turn 2 screens (`docs/ui-redesign/design/air-tasking-tab.html`; published as an Artifact) and listed ten places where §6.1 was unclear or conflicted with the redesign rules (`docs/ui-redesign/README.md`).

## Decision

1. **Adding entries.** The tab adds pool entries itself: "Add to pool" on the selected library card with a target list (that side's Map objectives and arrows), and "Auto-fill from objectives" in the pool header. The target list, target resolution, entry status, add, auto-fill and library removal are plan logic: they live in `airtask` (a new `airtask/pool.rs`) and step 9's Map dock calls the same functions. A third way in, upstream's air-group builders, is decision 0028.
2. **Pool table.** Columns: on/off, Sortie (side marker + name, cut with "…", full name on hover), Target, Weight, Mode, Runs, OK, ✖. Weight, Mode and Runs edit in the row. The Side column of §6.1 becomes the marker; Heading override moves to the selected-entry block.
3. **On/off.** `PoolEntry.enabled` is a checkbox in the first column; a switched-off row is grey and left out of Generate.
4. **Slot timeline.** One panel in the centre under the pool, both sides on one 0–3 h axis, drawn by the helper step 9's dock reuses. The Schedule sections keep only their fields.
5. **Before step 7.** The tab is wired to the real `validate`, `totals`, `preview` and `build_air_packs` now and does not change when step 7 fills them. Until then totals read 0, the timeline and plan check are empty, and Generate File writes an Air Tasking group with no sorties. The step 8b tests that need step 7 are written now and marked `#[ignore = "needs step 7"]`; step 7 removes the marks. Nothing ships from the branch before step 7.
6. **Reset** empties the pool, restores every setting to its default and draws a new seed. It keeps the library (user templates stay), as Template Builder's Reset keeps the catalog. Ctrl Z undoes it.
7. **Removing a user template** removes its pool entries too; one Ctrl Z restores both, and the undo label names both.
8. **Header label:** "Map date {d Mon yyyy} · {n} entries", because the Map date decides which sorties are valid.
9. **Hard stop.** A small public helper in `airtask/timing.rs`, next to `derived_timing`, returns an entry's hard-stop time (§4.3: arrival + attack time + margin + extension) for clock and zone spawns; the selected-entry block shows it.
10. **Sections** use `shell::settings_section` (collapsible, one-line summary); Budget, Threshold and Output start open.

The right panel shows the selected pool entry first, then the settings sections (README §4). Map's own readiness check for Air Tasking and Generate Base Map's air packs stay in step 9 (§6.1 "Map readiness", §7).

## Rejected

- §6.1 as written: a nine-column inline table (does not fit the 534 px centre at 1280 px), timelines inside the 304 px Schedule sections, and no way to add an entry until step 9 (which waits for the upstream merge).
- Keeping Generate disabled with a temporary readiness check until step 7: one more change to undo in step 7, for a branch that does not ship before it.
- Leaving the hard stop out until step 7.
- Auto-fill computed in `ui.rs`: plan logic in the UI, which the UI rules forbid.

## Consequences

- `airtask` gains public functions beyond the frozen §5 list (`pool.rs`, one timing helper). No frozen type changes.
- Step 7 must keep `validate` consistent with `pool.rs`'s entry status (orphaned, date) and remove the `needs step 7` ignores.
- Step 9 reuses `pool.rs` and the slot-strip helper.
