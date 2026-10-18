# 0021. AI formation collisions are a game bug; the app's formation settings are workarounds to revisit

- **Status:** Accepted
- **Date:** 2026-10-17
- **Decided by:** the user ("let's mark this as a bug and one that eventually may get fixed with downstream implications in our AI behavior and MCU needs"), after flight 2F

## Context

Flights 2C to 2F (`handoff/P14-probe-results.md`) traced every AI formation collision with a Tacview track to one manoeuvre: in a turn, an aircraft of the flight crosses its leader's track nearly abeam and at almost the same height, with no avoidance. It happens at every density; the density only changes the clearance. The server is not the cause (flight 2F: 8 aircraft, 1.00× wall time). The draft report is `handoff/il2-bug-report-drafts.md`, report 3.

## Decision

Treat the collisions as a defect in the game's AI, not in the app. Every formation setting or MCU the app writes because of it is a **workaround**:

- marked as such where it is written, with a comment naming this decision (not yet in the code: add it to `SAFE_AIR_FORMATION_DENSITY` and its uses with the next formation change);
- listed below, so it can be removed when the game is fixed;
- kept as small as possible: a value change before an added MCU.

## Workarounds in force or proposed

| Workaround | Where | Status |
|---|---|---|
| Every AI air formation at density 1 (Safe) | Air Tasking placement, the six built-ins, Template Builder (`SAFE_AIR_FORMATION_DENSITY`, `src/template.rs`) | Decision 0019, built on `claude/formation-safe-0019`, not merged |
| Pairs at density 2 (Loose) | the same three places | Decision 0020, Accepted 2026-10-18 (option A), not built |
| A flight flying a Cover order keeps Safe even when Pairs | the same three places | Decision 0022, Proposed (flight 2H: a covering F-51 pair at Loose collided in its transit weave) |
| Pairs at Safe in transit, a Formation command to Loose at `WP 1` (1 MCU per flight) | Air Tasking shell | Flight 2G: no losses, closest 18.7–26.9 m. Decision 0020's recorded alternative (E), not taken: A gives as much for no MCUs |
| AI skill raised to Veteran or Ace | the Planes' `AILevel` | Flight 2G: **rejected**, the wingman crosses and collides at the same second |
| AttackArea centre moved ahead of `WP 1` | Air Tasking placement | Flight 2G: **rejected**, the lead's attack entry still reverses and the pair collides later |

*Rows for flight 2G added on 2026-10-17 (results `handoff/P14-probe-results.md`, "Flight 2G"); the decision itself is unchanged. The rejected rows are kept so they are not tried again.*

## Consequences

- **Downstream, in AI behaviour:** Loose and Safe spread flights wider than the period doctrine (elements flying close and turning together). Escort, cover and mutual support between pairs may look and act looser than intended until the game is fixed.
- **Downstream, in MCU needs:** any MCU added for this bug (for example a Formation command at `WP 1`) costs the budget per flight; prefer value changes.
- **When the game updates:** re-fly the shipped Pairs at Safe Yak raid (flight 2F's PS cell regenerates byte for byte from `handoff/p14-probes/flight2f/`). If the wingman no longer crosses within 20 m in three runs, propose removing the workarounds in the order of the table, last first.
- The game bug report (draft 3) is the user's to post.
