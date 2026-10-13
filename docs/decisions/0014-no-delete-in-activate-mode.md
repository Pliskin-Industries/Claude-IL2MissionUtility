# 0014. Template Builder writes no Delete for a group that is re-activated

- **Status:** Proposed
- **Date:** 2026-10-13
- **Decided by:** open; the user decides. Finding and recommendation by Claude. Refines decision 0013.

## Context
Decision 0013 makes `Trigger Delete` list the units' entities, so that it works. Flight 1L ran both bring-up modes with that change (`handoff/P14-probe-results.md`, "Flight 1L", groups W3 to W6):

- **Spawn mode:** each finished wave is really removed and the next wave still spawns. Today every finished wave stays in the mission, switched off: the end-of-mission list held both.
- **Activate mode:** the group is deleted at its first clean-up and does not come back when its bring-up is pulsed again. Today it comes back, because the Delete does nothing and the units are only switched off.

In Activate mode the generator re-arms the zone after a clean-up, so a group that comes back when players return is what the logic is built for.

## Options

| | A. 0013 as written, both modes | B. Spawn mode lists entities; Activate mode writes no Delete |
|---|---|---|
| Activate-mode group after players leave and return | gone for good | comes back, as today |
| Generated files, Activate mode | one list changes | `DELETE DELAY` and `Trigger Delete` are no longer written: two MCUs fewer per group |
| Generated files, Spawn mode | one list changes | one list changes |
| Units left in the mission | none | an Activate-mode group stays, switched off, while it waits to be used again; that is one group, not one per wave |

## Decision
None yet. Claude recommends B: it keeps what players see today and removes two MCUs that did nothing.

## Consequences
- The six built-in sorties are Activate-mode groups. Under B a regenerated sortie has no Delete; Air Tasking places one-shot copies and writes its own Delete with entities, so its sortie-shape check (`airtask/library.rs`, which expects one Delete) changes with this.
- Fighter Pack generates in Spawn mode, so it gets the working Delete.
- `USER_MANUAL.md` line 121 describes the clean-up chain and needs the new wording.
