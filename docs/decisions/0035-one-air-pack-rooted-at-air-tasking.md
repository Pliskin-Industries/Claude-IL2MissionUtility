# 0035. `build_air_packs` returns one pack rooted at the `Air Tasking` group

- **Status:** Proposed
- **Date:** 2026-10-26
- **Decided by:** not yet (the user). Options and recommendation by Claude, from the step 7 scope (`handoff/sessions/2026-10-26-p14-step7.md`).

## Context

`build_air_packs(plan, ctx) -> Result<Vec<MapAirPack>, String>` is frozen (plan §5); step 8 calls it. Plan §3.0 draws one `Air Tasking` group holding each side's Logic and Sorties groups, and Load recognises it by name (§7).

Each pack is stamped on its own (`stamp_rooted_packs` re-indexes one tree at a time). A side's Logic links into its Sorties, so they must travel in one pack.

Step 8 (`codex/p14-8`, `src/airtask/standalone.rs`) writes the standalone file by wrapping all packs in a new `Air Tasking` group and re-indexing from 1. Its test (`air_tasking_generate_file_writes_air_pack`, ignored until step 7) expects exactly one group named `Air Tasking`.

## Options

- **A. One pack whose root is the `Air Tasking` group** (the four sub-groups of §3.0), or an empty `Vec`. The base map gets one `Air Tasking` group, as §3.0 and §7 say. Step 8's `build_air_group` re-indexes that root instead of wrapping it (a few lines in `standalone.rs`, step 8's file).
- **B. One pack per side, rooted at `Air Tasking DPRK` / `Air Tasking NATO`,** each holding that side's Logic and Sorties. Step 8 unchanged. The base map gets two top-level groups (Load recognises the prefix); §3.0's tree changes.

## Recommendation

**A.** It matches the documented tree and the "one Air Tasking group" checks on both paths, and makes step 8's standalone builder simpler. The cost is one coordinated change in step 8's file, made by the step 8 session.

## Consequences

- Under A the step 8 session changes `build_air_group` when it rebases onto this work; until then its ignored test stays ignored.
- `Vec<MapAirPack>` holds zero or one item; `FrontOptions.air_packs` keeps the `Vec` so the stamping code matches the other packs.
