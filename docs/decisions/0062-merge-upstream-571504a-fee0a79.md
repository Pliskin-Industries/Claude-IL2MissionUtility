# 0062. Merge upstream `571504a` and `fee0a79` before the alpha; their air-start regrouping changes Map Generate's output

- **Status:** Accepted
- **Date:** 2026-11-06
- **Decided by:** the user, 2026-11-06 ("Merge first", asked before building what is sent to eszyman). Conflict resolution by Claude.

## Context

`eszyman/IL2MissionUtility` had two commits the fork did not have:

- `571504a` (2026-10-28): one harvested coalition group and a ramp start per country; field-spawn aircraft on their own list.
- `fee0a79` (2026-11-02), about 5,900 lines:
  - the Airfield tab owns base-map fields and air starts;
  - Map strikes are wrapped in Exclusive Activation;
  - anti-air is placed in clearings;
  - a dead-end column is stretched to the Zone Out rim.

A build sent to him without them would lack his own newest work.

## Decision

Merge both, on branch `claude/upstream-merge-1102`. Conflicts were resolved by keeping both sides:

- `.gitignore`, `src/main.rs`: both sides' lines.
- `src/airstart.rs`: the fork's DPRK country **501** (decision 0042 A, flown in A3) stays; upstream's comment said 502. His new functions are listed in the module header.
- `src/frontlines.rs`: his skip for an airfield group anchored outside the area, then the fork's Safe formation density (decision 0019). The density test reads his new `(landscape, airfields, count)` return.
- `src/harvest.rs`: both doc comments, with 501.
- `src/ui/map.rs`: the fork's Air Tasking build, sidecars, 0037 A / 0054 country move and trace writing, with his `ref_groups_for_export` paths and mutable strike notes.
- `HANDOFF.md`: the fork's pointer file (CLAUDE.md: do not add to it); his session log stays in his repository.

## Output change

`fee0a79` exports air starts under `Airfields` / `DPRK` or `NATO` instead of one `Air starts` group. That is one more Group, so every later index moves by one in a Map Generate build that has air starts. Nothing else in the frozen builds changed.

The frozen files of 2N (sent to eszyman), A4 (flown) and L (flown) stay as they are. Their byte checks became `assert_frozen_but_air_start_groups` (`src/ui_tests.rs`):

- it puts the new build back in the old layout and compares the whole tree with the frozen file, up to index renumbering;
- it compares the locale files byte for byte.

So the Air Tasking logic and everything else is shown to be unchanged.

## Rejected

- **Send without merging:** eszyman would test a build that lacks his own October and November work.
- **Re-freeze 2N, A4 and L from the new build:** frozen files of flown or sent flights are never rewritten.

## Consequences

- `cargo test --offline`: 1,167 passed. Clippy rose from 306 to 314, all in his code.
- The upstream merge is complete up to `fee0a79`.
