# Decision records

One file per decision. A record says what was decided, why, what was rejected and who decided.

## Rules

- **Never edit a decided record.** To change a decision, add a new record and set the old one's status to `Superseded by NNNN`.
- **One decision per file**, named `NNNN-short-title.md`, numbered in order.
- **Write it when the decision is made**, in the same session. A decision that exists only in a chat is lost.
- **An open question is a record too**, with status `Proposed`. It becomes `Accepted` when the user decides.
- Keep each record under one page. Link to the evidence; do not copy it.

## Template

```markdown
# NNNN. Title

- **Status:** Proposed | Accepted | Superseded by NNNN
- **Date:** YYYY-MM-DD
- **Decided by:** who

## Context
What forced the decision, with links to evidence.

## Decision
What was chosen.

## Rejected
Each option not chosen, and why.

## Consequences
What this makes easier, harder or necessary.
```

## Index

| # | Decision | Status |
|---|---|---|
| [0001](0001-store-files-byte-for-byte.md) | Store every file byte-for-byte in git | Accepted |
| [0002](0002-backlog-in-github-issues.md) | The backlog lives in GitHub Issues | Accepted |
| [0003](0003-terrain-heights-from-the-creators-macro.md) | Terrain heights come from the creator's 3D-mapping macro | Accepted |
| [0004](0004-adopt-upstream-1500m-airstart.md) | Adopt upstream's 1500 m airstart; drop auto altitude | Accepted |
| [0005](0005-roles-and-merge-authority.md) | Fable plans and reviews, Astra builds, the user merges | Accepted |
| [0006](0006-ui-wording-and-defaults.md) | UI wording and defaults | Accepted |
| [0007](0007-trace-carrier-c0-s0-objectives.md) | Trace breadcrumbs are C0/S0 objectives | Accepted |
| [0008](0008-ai-aircraft-trigger-check-zones.md) | AI aircraft trigger check zones: filter the threshold by country | Accepted |
| [0009](0009-flight-1a-of-2026-10-12-meets-gate-g1.md) | The 2026-10-12 flight counts as flight 1A; gate G1 is met | Accepted |
| [0010](0010-reshape-the-remaining-probes-for-an-autopilot-pilot.md) | Reshape the remaining probes for an autopilot pilot | Accepted |
| [0011](0011-losses-count-on-event-13.md) | Air Tasking counts a lost aircraft on OnEvent 13 | Accepted |
| [0012](0012-aircraft-death-event-outside-air-tasking.md) | Which event counts a destroyed aircraft in Template Builder and Fighter Pack: event 13 | Accepted |
| [0013](0013-template-builder-delete-lists-plane-indexes.md) | Template Builder's clean-up Delete lists Plane indexes and removes nothing: list entities | Accepted |
| [0014](0014-no-delete-in-activate-mode.md) | Template Builder writes no Delete for a group that is re-activated | Accepted |
| [0015](0015-air-tasking-sorties-keep-ai-rtb-decision-off.md) | Air Tasking writes its sortie aircraft with `AiRTBDecision = 0` | Accepted |
| [0016](0016-fighter-pack-counts-each-aircraft-once.md) | Fighter Pack counts each aircraft once | Accepted |
| [0017](0017-step-3-activate-mode-own-delete-event-13.md) | Step 3 settles on Activate-mode sorties, its own Delete and event 13 | Accepted |
| [0018](0018-zone-out-only-after-an-enemy-is-seen.md) | A sortie's Zone Out ends it only after an enemy has been seen inside | Accepted |
| [0019](0019-air-formations-fly-at-safe-density.md) | Every AI air formation flies at density 1 (Safe) until Dense is fixed | Accepted |
| [0020](0020-pairs-formations-after-flight-2e.md) | Pairs formations still collide at Safe: Pairs fly Loose (A); Loose at the waypoint is the recorded alternative | Accepted |
| [0021](0021-formation-collisions-are-a-game-bug.md) | AI formation collisions are a game bug; the app's formation settings are workarounds to revisit | Accepted |
| [0022](0022-f51-armed-recon-at-loose.md) | Pairs at Loose does not hold for a covering pair: a flight flying a Cover order stays Safe | Accepted |
| [0023](0023-step-4-threshold-one-complex-trigger-per-area.md) | Step 4 threshold: one Complex Trigger per area, re-activated as the presence check | Accepted |

**P14 Air Tasking decisions U1 to U24** are kept in `handoff/P14-air-tasking.md` §2.1 and are not repeated here. New P14 decisions go in this folder.
