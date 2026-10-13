# Strike and CAP groups written by upstream's own code

These three `.Group` files were written on 2026-10-13 by the upstream app (`eszyman/IL2MissionUtility`, commit `a31ad0a`), unmodified, so that probe flight 1M can fly what that app generates.

| File | What it is |
|---|---|
| `Upstream_Strike_NATO_F80C_4ship.Group` | Template Builder, four F-80C (one lead, three wingmen), **+ Mission ▾ Strike** |
| `Upstream_Strike_DPRK_Il10_4ship.Group` | the same with four Il-10 |
| `Upstream_CAP_NATO_F86A_4ship.Group` | four F-86A-5, **+ Mission ▾ CAP** |

**How they were made.** A checkout of upstream at `a31ad0a` (worktree `Claude-IL2MissionUtility-up`, local branch `trial/upstream-strike-files`, never pushed) got one added, ignored test, `write_upstream_mission_package_files`. It builds each group the way the Template Builder UI does after the user picks the aircraft and a mission package, with every option at the value the UI leaves it at, and writes it with the app's own serializer. The test is in `write_upstream_mission_package_files.patch`; apply it to that commit and run:

```
cargo test --offline write_upstream_mission_package_files -- --ignored
```

The UI's save dialog was not driven; the test calls the same generation and serialization functions.

**Do not edit these files.** They are evidence for an upstream issue (`handoff/upstream-issue-draft.md`). The probe reads them and adds listeners; any change the probe makes to a copy is listed in the plan's step 0j table.

What the files hold, read from them:
- Every plane: air start at 1,500 m, `Fuel = 1`, `AiRTBDecision = 0`, `DeleteAfterDeath = 1`, `DeleteAfterLand = 1`.
- Strike order chain: Formation, `WP 1` (the initial point), `AttackArea` (3 km, 120 s, ground), an `OnAreaAttacked` report, `WP 2` (home, where the lead started), `Mission Complete 1`, then `MISSION END`.
- Strike abort hooks on all four aircraft, each to the `Goto WP 2` timer: event 9 (bingo bombs), 3 (critical damage), 1 (pilot wounded), 7 (bingo fuel).
- CAP: four patrol waypoints in a loop, and abort hooks 8 (bingo guns), 7, 3, 1 to `Goto WP 5` (home). Nothing follows `WP 5`.
- Bring-up: `Zone IN` leads to the timer `MISSION BEGIN`, which activates the units.
- Clean-up: `Deactivate Units` lists the four entities; `Trigger Delete` lists the four Plane indexes.
