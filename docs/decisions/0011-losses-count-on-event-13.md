# 0011. Air Tasking counts a lost aircraft on OnEvent 13

- **Status:** Accepted
- **Date:** 2026-10-12
- **Decided by:** the user, on Claude's recommendation after flight 1G

## Context
The plan counted a sortie's losses on each plane entity's OnEvent type 4 (`handoff/P14-air-tasking.md` §3.2, risk g). Flight 1G tested it (`handoff/P14-probe-results.md`): three aircraft died, two by a Damage command and one by gunfire. None raised type 4. Each raised type 13 exactly once, within 0.1 s of the kill, and type 0 and usually type 2 after it. A deleted aircraft raised nothing.

The same flight showed one aircraft still flying 12 minutes after the Delete that should have removed it. That Delete's Objects list also held its dead wingman.

## Decision
1. Air Tasking counts losses on **OnEvent 13**. `LossesImpl`'s default changes to match, and the plan's §3.2, step 3 tests and risk g are rewritten for 13.
2. **One more probe flight, 1H, tests the Delete** before step 3 is settled: a Delete with a dead aircraft in its Objects list, in both orders, against controls, and one Delete per aircraft as the candidate fix. Spec: plan step 0f.

## Rejected
- **Keep type 4 and add a per-plane latch.** Type 4 did not fire at all, so there is nothing to latch.
- **Count on type 0 (pilot killed) or 2 (crashed).** Type 2 came seconds later and not every time; type 0 would miss an aircraft whose pilot bails out, which has not been tested.
- **Settle step 3 without the Delete probe.** The budget's hard limit (U22) releases a slot when an aircraft is deleted. If a Delete can leave an aircraft flying, the limit can be exceeded.

## Consequences
- Step 3 (`codex/p14-3`, held) uses type 4 today and must change before it merges.
- **Outside P14:** Template Builder writes type 4 (`OnPlaneDestroyed`) as the default death event for aircraft (`template.rs`, `EntityEvent::default_for`), and Fighter Pack's `DeathCount` counts on it. If type 4 never fires in the Korea game, those counters never count. That is existing app behaviour and its own decision: changing it changes the `.Group` files Generate writes.
- Not known: whether type 13 fires for an aircraft that crashes with no damage, or whose pilot bails out.
