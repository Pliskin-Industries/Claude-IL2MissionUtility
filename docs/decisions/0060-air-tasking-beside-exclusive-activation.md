# 0060. Air Tasking runs beside upstream's Exclusive Activation; the proposal explains how it could replace it

- **Status:** Accepted; its timing (send after real-operation logs) superseded by [0063](0063-send-alpha-and-proposal-to-eszyman.md) (2026-11-06)
- **Date:** 2026-11-05
- **Decided by:** the user, 2026-11-05 ("Let's keep B but offer an explanation of how air tasking could replace all of it"). Options and recommendation by Claude.
- **Supersedes:** [0026](0026-air-tasking-and-upstream-strikes-and-caps.md), in its third point (Air Tasking replaces Map › Missions and Exclusive Activation) and in how the proposal is framed. 0026's other points stand, and so does [0028](0028-upstream-air-groups-feed-the-pool.md).

## Context

0026 (2026-10-28) made Air Tasking the orchestrator of AI air activity and planned to propose that it replace upstream's Map › Missions and Exclusive Activation. Since then:

- **Upstream built more of its own scheduler.** `eszyman/IL2MissionUtility` commit `fee0a79` (2026-11-02) makes Map Generate write an Exclusive Activation group whenever two or more strike plans are on the map. It also splits a file that holds several plans onto separate targets. Asking him to retire that asks him to drop work of the same week.
- **Both already run side by side in the fork.** One Map Generate writes Map missions under Exclusive Activation and Air Tasking under its budget. Decision 0037 keeps the Map's own AI from starting Air Tasking areas. Nothing limits the total number of aircraft across the two.
- **The proposal was never sent.** `handoff/upstream-air-tasking-proposal-draft.md` predates step 12. Issue eszyman/IL2MissionUtility#3 (flight 2N) showed him Air Tasking before any proposal.
- **Two upstream commits are not merged:** `571504a` and `fee0a79`, about 6,000 lines, mostly in the Map and Airfield tabs.

## Options

- **A. Keep 0026.** Propose that Air Tasking replace Map › Missions and Exclusive Activation. One system, but it asks him to retire what he just built.
- **B. Two modes on purpose (chosen).**
  - Exclusive Activation stays for scripted plans that run one at a time.
  - Air Tasking is the mode driven by where players fly: areas, the clock, the budget.
  - The integration that serves both stays (0026 points 1 and 2, 0028): upstream's Strike and CAP groups as Air Tasking sortie templates, and his target types as Air Tasking targets.
  - The proposal is an addition to his work, not a replacement.
- **C. Fork only.** Release the alpha in the fork and set no direction until real operations show how it plays.

## Decision

**B.** The proposal also explains, as an option for him and not a request, how Air Tasking could take over Map › Missions and Exclusive Activation completely:

| Upstream piece | What does that job in Air Tasking |
|---|---|
| Exclusive Activation: one plan runs at a time | The budget per side. A budget of 1 is the same lock; a higher budget lets more flights run at once. |
| A plan's Zone In starts it when a player enters | The area round each target goes HOT when N players of the other side are inside it (N from 1 to 3). |
| The 3-hour `{n} WINDOW` reopens the other plans if a plan never finishes | The copy's hard stop, then its Delete, frees the budget slot. |
| Map › Missions snaps a strike onto the next unused target of a type | A pool entry with a target of that type (0028). |
| `MISSION END` / `ALL DESTROYED` ends a strike | Air Tasking hooks them to free the slot (0026 point 1). |
| Several plans in one file, split onto separate targets | Several pool entries, each with its own target and weight. |

The proposal names the one gap: Air Tasking's clock launches only over a HOT area, so a plan meant to run with no player nearby needs an "always on" area option that does not exist yet.

The rewritten proposal is sent after the alpha has logs from real operations, and only when the user says so.

## Rejected

- **A:** it asks the upstream author to drop a scheduler he is still building, before he has seen Air Tasking fly.
- **C alone:** it leaves the fork with no stated direction for the integration code (0028) and the plan check below.

## Consequences

- **To build:** a plan check that warns when a map has both Map missions under Exclusive Activation and Air Tasking, since their aircraft add up with no shared limit. It is a warning, not a block. It needs its own small plan and test, and it does not change Generate output.
- `handoff/upstream-air-tasking-proposal-draft.md` is rewritten to match: Air Tasking as an addition, with the table above and the gap.
- Merging `571504a` and `fee0a79` stays a separate decision for the user (STATE, "Waiting on the user").
- Integration code (0028: Strike and CAP shapes in `inspect_sortie`, the target kinds, a probe flight) still waits for his answer.
