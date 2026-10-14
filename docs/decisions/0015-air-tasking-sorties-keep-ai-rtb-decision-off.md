# 0015. Air Tasking writes its sortie aircraft with `AiRTBDecision = 0`

- **Status:** Accepted
- **Date:** 2026-10-13
- **Decided by:** the user, 2026-10-13 ("0015 go with recommendation"). Finding and recommendation by Claude.

## Context
Flights 1G, 1K and 1L (`handoff/P14-probe-results.md`): with `AiRTBDecision = 1`, a damaged pair of AI aircraft splits. One of the two stops answering Delete, Deactivate and Damage commands, flies to the nearest friendly airfield, lands, and is removed by the game a few minutes later. Which of the two it is varies. With `AiRTBDecision = 0`, a damaged pair stayed on its task and both aircraft were deleted on command (one pair, one run).

The air budget (U20, U22) counts a sortie as finished when its clean-up has run. An aircraft that no longer answers commands would stay in the air, uncounted, for up to twenty minutes.

## Decision
Every aircraft Air Tasking places is written with `AiRTBDecision = 0`. Going home after damage is part of the sortie's own logic (an entity event to the exit waypoint), which keeps the aircraft under the mission's control.

## Rejected
- **Leave it at 1 and count on the landing.** The aircraft raises event 5 when it lands and is removed after that, but only if it reaches an airfield; one that is shot down on the way raises 13, and one that does neither is never accounted for.
- **Leave it at 1 and delete twice.** The aircraft that stops answering ignored a Delete and a Damage command 30 s apart.

## Consequences
- The six built-in sorties carry `AiRTBDecision = 1` on every plane; `place_air_sortie` sets 0 on the copies it places and leaves the files alone.
- A damaged AI aircraft keeps fighting until the sortie's logic sends it home. Which events do that (3 critical damage, 1 pilot wounded) is part of step 3.
- Not tested: more than one pair, a 4-ship, and a single aircraft with the setting on.

## Evidence added 2026-10-14 (flight 1M)
Built and flown: a 4-ship placed by `place_air_sortie` (setting 0, written by the code) was damaged and then deleted on command, all four. The same 4-ship with the setting on: three of the four ignored the Delete and a Damage command and flew home. A single damaged aircraft with the setting on stayed reachable. The decision stands; the case for it is stronger than the pairs showed.

