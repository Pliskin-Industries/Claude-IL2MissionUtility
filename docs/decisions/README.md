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
| [0024](0024-threshold-presence-check-is-a-pulsed-check-zone.md) | The threshold's presence check is a pulsed check zone (ZonePulse), after flight 2J | Accepted |
| [0025](0025-remove-trigger-reactivate.md) | Remove the TriggerReactivate presence check from the code | Accepted |
| [0026](0026-air-tasking-and-upstream-strikes-and-caps.md) | Air Tasking schedules and budgets upstream's strikes and CAPs instead of placing its own | Accepted (A) |
| [0027](0027-air-tasking-rail-tab-design.md) | The Air Tasking rail tab (step 8): layout, adding entries, behaviour before step 7 | Accepted |
| [0028](0028-upstream-air-groups-feed-the-pool.md) | Upstream's air-group builders feed the pool, as their own step after the upstream merge | Accepted |
| [0029](0029-step-5-clock-lean-design.md) | Step 5 clock: a leaner build of plan §3.7 on flown facts | Accepted |
| [0030](0030-step-6-zone-lean-design.md) | Step 6 zone front end: a leaner build of plan §3.8 on flown facts | Accepted |
| [0031](0031-sortie-categories.md) | Sortie categories: a label in the template file, chosen by the user, and a pool filter | Accepted |
| [0032](0032-air-tasking-totals-limits.md) | Air Tasking totals limits (D36) after the upstream author's load test | Accepted |
| [0033](0033-d30-check-after-the-country-filter.md) | The D30 check after the country-filtered threshold: warn within r_in | Accepted |
| [0034](0034-acceptance-mission-is-a-trace-build.md) | The step 11 acceptance mission is observed by a trace build, not debug subtitles | Accepted |
| [0035](0035-one-air-pack-rooted-at-air-tasking.md) | `build_air_packs` returns one pack rooted at the `Air Tasking` group | Accepted |
| [0036](0036-acceptance-test-before-step-9.md) | Fly the Air Tasking acceptance test before step 9 | Accepted |
| [0037](0037-map-ai-off-player-countries.md) | Move the Map's own AI aircraft off the player countries when Air Tasking is in the mission | Accepted (A) |
| [0038](0038-used-sortie-copy-goes-quiet.md) | A used sortie copy goes quiet at DONE HUB: DONE HUB OFF switches off its timers, zones and counter | Accepted |
| [0039](0039-map-undo-keeps-air-plan-only-for-dock-edits.md) | Map undo keeps the Air Tasking plan only for the Air dock's own edits | Accepted |
| [0040](0040-parked-player-keeps-drawing-sorties.md) | A player parked inside an Air Tasking area keeps drawing sorties | Accepted |
| [0041](0041-trace-build-traces-the-whole-file.md) | A trace build from the UI traces decision points across the whole file, plus the P14 names on Map and Air Tasking | Accepted |
| [0042](0042-dprk-air-start-country.md) | The app's DPRK air start writes country 501, the country Air Tasking watches for DPRK players | Accepted (A) |
| [0043](0043-switch-breadcrumbs-from-inputs.md) | A switch's breadcrumb is pulsed by the switch's own inputs | Accepted |
| [0044](0044-paused-timers-d11-relays.md) | A switched-off timer is paused: the zone and the clock end their timers through D11 relays (supersedes parts of 0029 and 0030) | Accepted |
| [0045](0045-walker-armed-check-zones.md) | The walker models check zones as flown: a pulsed zone stays armed (supersedes 0044's "Armed" term) | Accepted |
| [0046](0046-step-12-test-plan.md) | Step 12 test plan: AI stand-ins first (flight 2M, no pilot), two human pilots last (flight 2N); 12a done; `CounterResetImpl` removed | Accepted |
| [0047](0047-what-n-2-means.md) | N ≥ 2 players: the count resets when anyone leaves (no window); N 1 to 3, one N for both sides; N to start, 1 to hold | Accepted (E) |
| [0048](0048-zone-out-with-several-players.md) | Zone Out with several players: keep the check zone if ALL-out; options if ANY-out (flight 2M: ALL-out) | Superseded by 0050 |
| [0049](0049-counted-plane-dies-inside.md) | A counted plane that dies or is removed inside an area raises no Left (flight 2M): a death inside zeroes the count (Killed → ZERO) | Accepted (B) |
| [0050](0050-ai-on-sides-of-its-own.md) | Watch players only: put Air Tasking's AI (602, 503) on sides 3 and 4 so check zones ignore it, then widen Zone IN / Zone Out: fails (flight 2Q). Option D accepted (flight 2R): a player count per area from Complex Trigger events, count to three then busy | Accepted (D) |
| [0051](0051-which-spawns-the-count-check-sees.md) | Generate refuses a watched player spawn inside a count circle: the Map's air starts and its reference groups' player airfields and coop planes; Airfield-tab and editor spawns cannot be seen | Accepted |
| [0052](0052-how-many-count-areas.md) | The Air Tasking tab's player count: one count radius (10 to 100 km) for every area; the number of areas follows the pool's targets; count circles on the Map; orange warning below the threshold's inner ring | Accepted |
| [0056](0056-review-replay-and-trace-safety.md) | Replay reads one log session and rescans on Run, shows discarded lines, falls back to (T - 3) / 50, refuses a .Group changed since its trace; trace builds check the name and index room first and give each file its own breadcrumb band | Accepted |
| [0057](0057-empty-trace-metadata.md) | An empty trace keeps its old metadata format, so L's frozen base files stay byte-identical | Accepted |

**P14 Air Tasking decisions U1 to U24** are kept in `handoff/P14-air-tasking.md` §2.1 and are not repeated here. New P14 decisions go in this folder.
