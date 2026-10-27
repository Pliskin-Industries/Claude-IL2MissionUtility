# 0028. Upstream's air-group builders feed the Air Tasking pool, as their own step after the upstream merge

- **Status:** Accepted
- **Date:** 2026-10-18
- **Decided by:** the user ("Add, autofill, and what eszyman uses to build air groups in the other tab"; then "Both" and "After merge, own step")

## Context

Upstream v0.7 (`eszyman/IL2MissionUtility`, `upstream/main` @ `842d27c`, not merged into this fork) builds air groups two ways:

- **A. Template Builder's "+ Mission ▾"** writes a CAP or a Strike for the selected aircraft lead (`template::apply_mission_package` and the strike helpers in `src/template.rs`; upstream `USER_MANUAL.md` "Missions").
- **B. Map › Forces › Missions** loads a strike `.Group` and snaps it onto the next unused enemy target of one type inside the AO: tanks, truck routes, rail, ships, airbases, bridges or dams (`src/ui/map.rs` `missions_forces`, `add_map_mission`; `template::pick_strike_target`).

When asked how entries get into the pool (decision 0027), the user asked for both of these as well as Add and Auto-fill.

Upstream's CAP and Strike flights are not a shape `airtask::inspect_sortie` accepts today (decision 0017, tested shapes S1–S4 and S7): they carry a `Go Home Once` counter, OnKilled / `ALL DESTROYED` hooks and a disabled Zone Out. The code exists only on `upstream/main`. STATE "Waiting on the user" item 4 already proposes that Air Tasking schedule and budget upstream's strikes and CAPs instead of placing its own.

## Decision

Both A and B feed the pool: a library sortie can be built from an aircraft type, count and mission kind (A), and a pool entry can target a snapped target of one of B's types, next to objectives and arrows.

They are built as **their own step after the upstream merge**, planned together with STATE item 4. Step 8 builds Add to pool and Auto-fill only.

## Rejected

- Holding step 8 until the upstream merge to build all ways in at once: delays the front end the user asked for now.
- Only A or only B: the user chose both.

## Consequences

- The step needs a new tested shape in `airtask::inspect_sortie` (with its own test, U21 / D39), very likely a probe flight, and a `TargetRef` variant or a mapping for snapped targets (a frozen-type change: its own small commit, §10).
- `pool.rs` (decision 0027) is where the new target kinds and the mission-built sorties join the pool, so the tab needs little change.
