# 0040. A player parked inside an Air Tasking area keeps drawing sorties

- **Status:** Accepted
- **Date:** 2026-10-28
- **Decided by:** the user ("We can leave a parked player as triggering as I think that will be a rare instance overall"), on the A1 run sheet's question 1

## Context

Flight A1 (`handoff/sessions/2026-10-27-p14-acceptance-a1.md`, results in `handoff/P14-probe-results.md`, "Flight A1") asked whether a player parked at their own airfield inside the other side's area should keep drawing sorties.

What the game does today:
- A player who **spawns** inside an area is not seen. The entry trigger counts planes that cross into it, and a spawn inside does not count (A1 finding 1).
- A player who **flies in and then lands or parks** inside the area has been seen. While they stay, the presence check keeps the area HOT, so the clock and zone entries can keep starting sorties against it.

## Decision

No change. A player who has been seen and then parks inside an area keeps the area HOT and can keep drawing sorties, within the side's budget. A player who spawns inside is still not seen until they fly in across the ring. The case is expected to be rare.

## Rejected

- **Stop counting a player on the ground** (for example, check speed or altitude in the presence check). It adds MCUs to every area, and no flight has tested such a check.
- **Treat a spawn inside the area as seen.** It needs an extra trigger on spawn, and it would draw sorties onto players at their home airfield, which is the case this question is about.

## Consequences

- No code or Generate output changes.
- The manual (step 10) should say it: a player is seen only after flying into an area, and stays seen while inside it, parked or not.
