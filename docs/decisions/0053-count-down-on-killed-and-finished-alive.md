# 0053. The player count steps down on Killed and Finished alive, not on Finished

- **Status:** Accepted, to ship after no-pilot probe flight 2S
- **Date:** 2026-11-01
- **Decided by:** the user, 2026-11-01 ("Fix 1–9, and A–C too"), on Claude's triage of Astra's review of `b5acb35..6f02211` (finding A, Astra job `20261101145549-c0f120bc`, item 1).

## Context

- The player count per area (decision 0050 D, `src/airtask/presence.rs`) steps down on Left (60) and Finished (62).
- A kill raises Killed (70) at once and Finished (62) about 15.06 s later, when the wreck lands (flights 2P and 2R).
- When the area is busy (four or more entries), every step relay is off. `CLEAR` fires when nobody of that side is inside: 3.06 s after the last plane dies (flight 2M, ALL-out). It returns the count to 0 and switches `UP 1` on.
- **The fault.** The last player in a busy area dies. `CLEAR` reopens the count about 3 s later. A new player enters before the wreck lands, so the count is at 1 and `DOWN 1` is on. The wreck's Finished then arrives and steps the count to 0: `EMPTY` ends a copy while the new player is inside.
- It needs a busy area and an entry within about 12 s of the last death. It is rare, but it ends a sortie under a player.

## Options

**A. Step down on Killed (70) and Finished alive (63) instead of Finished (62) (chosen).** 0 MCUs.
- A kill steps the count down at once, not 15 s later. A Delete or a player who ends the flight alive inside raises 63 (flight 2P: a Delete raises 62 and 63).
- The wreck's late Finished is no longer wired, so it cannot reach a reopened count.
- **Not yet flown:** whether a wreck that falls out of the circle after a kill inside raises Left. If it does, a kill would step the count down twice (Killed, then Left), which leaves the count too low: an area can empty with a player inside. Flight 2S checks this before the change ships.
- **Not known:** what a player who leaves the server in flight raises (flight A3 logs it as destroyed). If it raises Finished only, the count stays too high, which is safe (the area does not empty).

**B. Keep Finished (62); accept the fault.** No change. Rejected: it ends a sortie under a player.

**C. A guard timer that ignores Finished for 15 s after `CLEAR`.** About +2 MCUs per area. Rejected: more MCUs than A for the same result (the MCU economy rule: fewest MCUs and the cleanest design win), and a timing guard depends on the 15.06 s figure holding for every aircraft.

## Decision

**A.** Each count's Complex Trigger has `EventsFilterKilled = 1` and `EventsFilterFinishedAlive = 1`, `EventsFilterFinishedSimple = 0`. Left (60), Killed (70) and Finished alive (63) go to every `DOWN k`. Built now; merged only after flight 2S shows:

1. a kill inside, whose wreck carries out of the circle, raises Killed once and no Left;
2. the busy-path case above: a new entry between `CLEAR` and the old wreck's landing keeps the count at 1 (no `EMPTY`);
3. a Delete inside steps the count down once.

If 2S shows Left for an outbound wreck, this record is superseded before merge.

## Consequences

- **Output changes** for missions with Air Tasking: each count trigger's filter keys and event links change. MCU count is unchanged.
- In counting mode (0 to 3), a death now steps the count down at the kill instead of 15 s later.
- Frozen probe and acceptance files are not regenerated.
