# 0049. A counted plane that dies or is removed inside an area

- **Status:** Proposed
- **Date:** 2026-10-31
- **Decided by:** open; the user decides. Claude recommends B.

## Context

- Decision 0047 E counts the Complex Trigger's Entered events toward N and zeroes the count on Left.
- **Flight 2M** (`handoff/P14-probe-results.md`, "Flight 2M") showed the trigger raises **no Left** for a plane deleted or killed inside it. Only a fly-out raises Left.
- So with N = 2, a player who enters and is shot down inside stays counted. The next single arrival arms the area, as if N were 1, once. The stale entry lasts until someone flies out or the area closes.
- In a dogfight, players are often shot down over the objectives.
- A player who leaves the server in flight is logged as destroyed (flight A3). Whether that raises Left or Killed is not flown.
- The trigger has an `EventsFilterKilled` key. Its event number is one of the 19 not yet known (`docs/il2-loader-and-log-facts.md`, "The Complex Trigger").

## Options

**A. Accept it.** No change; the manual states it. 0 MCUs.

**B. A death also zeroes the count (recommended).**
- Set `EventsFilterKilled = 1` on the N ≥ 2 triggers and link the Killed event to the existing `ZERO`. 0 MCUs. N = 1 is unchanged.
- A death inside then fails safe: everyone still inside must be counted again by new entries, so no sortie comes from a stale count.
- **Needs:**
  - Killed's event number. Claude can read it from the Mission Editor through the test rig (a trigger with the Killed slot linked, saved and read back), with no work for the user.
  - A short no-pilot check that a kill inside raises it, for example a Damage kill in one cell, or a row in flight 2N.
- Deletes are not covered, but Air Tasking never deletes a player.

**C. A safety window.**
- A restarting timer zeroes the count W after the last entry (for example 5 min). +2 MCUs per area.
- It bounds a stale count to W.
- It also forgets a player who loiters longer than W, which brings back the weakness 0047 rejected in option A.

## Decision

Not made.

## Consequences

- B adds one editor read and one game check before step 12 ends, and changes the N ≥ 2 trigger output only (N = 1 output unchanged).
- A changes nothing; the manual names the weakness.
- C adds 2 MCUs per area for N ≥ 2.
