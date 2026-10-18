# 0024. Air Tasking schedules and budgets upstream's strikes and CAPs instead of placing its own

- **Status:** Proposed
- **Date:** 2026-10-18
- **Decided by:** not yet (the user). Options and recommendation by Claude, from the upstream merge plan (`handoff/sessions/2026-10-18-upstream-merge.md`).

## Context

Upstream v0.7.0 (`eszyman/IL2MissionUtility`, commits `160619d` and `842d27c`) now builds AI air missions of its own:

- **Template Builder, + Mission:** a **Strike** (formation by role, Goto to an initial point, 3 km ground AttackArea for 2 min with Time on Target beside it, home Goto, Mission Complete; abort events through one `Go Home Once` counter per flight; `ALL DESTROYED` pulses `MISSION END` when every attacker is gone; Zone Out disabled) and a **CAP** (a diamond at low priority until bingo guns, critical damage or a wounded pilot, then home). Bingo fuel is no longer an abort.
- **Map, Forces dock, Missions:** a strike or CAP is placed and dragged on real targets (trucks, trains, ships, airfields, bridges) and written into the base map.
- **Exclusive Activation:** a 3-hour `{n} WINDOW` per plan reopens the other plans if a plan never finishes.

Air Tasking (P14, steps 1a to 3 merged) does the same job a different way: it copies one of the six built-in sorties (`TemplateExamples/Historical1950/`) onto a Map target, wraps it in its own shell (decisions 0017 and 0018: Zone IN as the "enemy seen" check, on-station time, `HOT` / `COLD` / `EXTEND`, its own Delete, losses on event 13) and limits how many sorties each side has alive (the budget, plan U20 and U22). The upstream issue text (`handoff/upstream-issue-draft.md`, last section) already offered the upstream author this direction.

So after the merge the app has two ways to put AI strikes and CAPs on Map targets, with different logic and no shared budget. STATE item 4 asks which one Air Tasking builds on.

## Options

- **A. Air Tasking schedules and budgets upstream's missions.** A strike or CAP that Template Builder or the Map builds becomes an Air Tasking sortie template: Air Tasking decides when it launches (mission clock, players entering an area) and counts it against the side's budget; the mission's own route, attack and go-home logic stay upstream's. Needs: `inspect_sortie` to accept upstream's Strike and CAP shapes, the shell to hook their `MISSION END` (and `ALL DESTROYED`) for budget release instead of rewiring a built-in's Zone IN, and a probe flight of one wrapped Strike and one wrapped CAP.
- **B. Keep both, separate.** Air Tasking keeps placing the six built-ins; upstream's map strikes and CAPs stay outside the budget. Least work; two systems can put aircraft on the same target, and nothing limits the total.
- **C. Air Tasking generates its sorties with upstream's mission builder.** Air Tasking calls `apply_mission_package` for each tasking (the target and its side are known), so every sortie is an upstream Strike or CAP, and the six built-ins become example data only. Closest to "one way to build a mission"; drops the shell logic flights 2 to 2I proved (Zone IN gate, `HOT` / `COLD` / `EXTEND`) unless it is moved onto upstream's shape.
- **D. Decide after one probe flight.** Wrap one upstream Strike and one CAP with today's shell as far as it accepts them, fly it, then choose between A and C.

## Recommendation

**D, then most likely A.** A keeps what both sides have proven: upstream owns the mission itself (route, attack, go-home, which the author fixed and maintains), Air Tasking owns scheduling and the budget (which flights 2 to 2I proved in the game). But how a shell attaches to upstream's `MISSION END`, `ALL DESTROYED` and disabled Zone Out has not been checked, and the shell's Zone IN gate (decision 0018) may not fit a strike that turns Zone Out off. One probe answers that before code is planned.

## Consequences

- Whatever is chosen, the formation rule of decisions 0019 to 0022 already applies to upstream's missions after the merge (`set_air_formation_density` runs on every Template Builder output).
- A and C change what Generate writes for Air Tasking; flown probe files must regenerate byte for byte or be frozen, as decisions 0018 to 0022 did.
- Nothing is sent upstream about this until Air Tasking has a working concept and UI (user rule, 2026-10-18).
- The step 4 threshold work (`codex/p14-4`, decision 0023) builds on the current shell; A or C may move where it attaches.
