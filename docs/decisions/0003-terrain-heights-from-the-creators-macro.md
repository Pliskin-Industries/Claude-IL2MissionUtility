# 0003. Terrain heights come from the creator's 3D-mapping macro

- **Status:** Accepted
- **Date:** 2026-09-26
- **Decided by:** the user

## Context
The app needs a height grid to place units on real ground. The first method was in-app: export T-34 probes, use the editor's *set to ground*, and import the snapped result (`heightprobe.rs`). The original creator (eszyman) is building a macro for 3D terrain mapping.

## Decision
The creator's macro is the expected source of the height grid (issue #1). Building the macro into the app is a long-term goal (#2).

## Rejected
- **Extending the probe-snap method** with 200 m and rough-100 m passes. It is deferred (#3): keep it working, do not add to it.

## Consequences
- Terrain-aware placement (#25) is blocked on #1.
- "Apply terrain heights on export" defaults to **off** (user decision, 2026-09-24).
- Upstream has since baked a Korea height grid into the app (`967cbeb`). Check what it provides before doing more terrain work.
