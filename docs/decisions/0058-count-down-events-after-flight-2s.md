# 0058. The player count steps down on Left and Finished again, after flight 2S (supersedes 0053)

- **Status:** Accepted (A)
- **Date:** 2026-11-01
- **Decided by:** the user, 2026-11-01 ("Agree, A for now. Add C later if logs show this"), on Claude's record from flight 2S, which 0053 named as its merge gate.

## Context

- 0053 changed the player count (`src/airtask/presence.rs`) from stepping down on Left (60) and Finished (62) to stepping down on Left, Killed (70) and Finished alive (63). Its aim: a wreck's late Finished could no longer empty a count that `CLEAR` had just reopened (busy area, review finding A).
- 0053 said it must not merge if flight 2S showed a Left from a wreck that leaves the circle. **2S showed exactly that** (cell S1, logs `real_59`, Tacview `handoff/tacview/P14_Probe_2S.zip.acmi`):
  - A Yak flying outward was killed about 5,708 m from the centre of a 6 km circle. KILLED stepped the count 2 → 1 at 251.28 s.
  - Its wreck kept moving outward (about 260 m in 2 s), crossed the ring, and raised **LEFT at 253.36 s**. That stepped the count 1 → 0 and fired `EMPTY` while the holder Yak was still inside (it was deleted inside at 420 s).
  - With 0053's wiring, any player shot down near the edge while flying outward is counted out twice. The area empties under the other players. This is more likely than the fault 0053 fixes.
- Other 2S facts that bear on the choice:
  - Under the old wiring the same wreck steps once. It raised Left; it raised no Finished, because it was outside when it was removed (as flight 2R found for planes outside).
  - A Delete inside raises Finished and Finished alive in one tick. Either wiring steps once (S3).
  - How long after a kill the wreck's Finished comes is not fixed. In S1 the wreck was removed 14.9 s after the kill (in the air, about 3,000 m up). In S2 it was removed 3.0 s after the kill, in the same tick as `CLEAR` (607.68 s), and stepped nothing. Flights 2P and 2R measured 15.06 s. The cause of the difference is not known.
  - So the busy-path race (0053's context) has a window of 0 to about 12 s after `CLEAR`. 2S's cell S2 could not test it, because the wreck's Finished came in the same tick as `CLEAR`.

## Options

**A. Go back to Left (60) and Finished (62), the wiring flight 2R flew (chosen).** 0 MCUs.
- Every removal 2R and 2S flew steps once: a wreck inside (Finished), a wreck that crosses out (Left), a Delete (Finished).
- Keeps 0053's fault as a known limitation: a busy area (four or more players) empties completely, a new player enters within about 12 s, and an old wreck's late Finished then empties the count under that player. It needs all three, and S2 shows the wreck's Finished can also come with `CLEAR`, closing the window.
- The `DownEvents::FlownFinished` wiring becomes the production wiring again. Output for missions with Air Tasking returns to `main`'s.

**B. Keep 0053 (Left, Killed, Finished alive).** Rejected by flight 2S: an outbound wreck steps the count twice.

**C. A, plus a gate that drops Finished for about 12 s after `CLEAR`.** Held back: built only if logs show the race (see Consequences). About +3 to +4 MCUs per area (a relay timer that fans Finished out to the `DOWN k` steps, its Deactivate, an on-delay timer, its Activate).
- Closes 0053's fault in the safe direction: a Finished lost in the window leaves the count too high, so the area does not empty early.
- Costs MCUs in every area for a rare case (the MCU economy rule: fewest MCUs and the cleanest design win). Needs a walker test and a no-pilot flight. It also relies on 15.06 s being the longest delay before a wreck's Finished.

**D. A, plus `CLEAR` reopening the count through its own 12 s delay.** +1 MCU per area. Rejected: a player who enters during the delay is not counted. That undercount can later empty the area under a player, the same fault in another place.

## Decision

**A.** `presence::DownEvents::LeftAndFinished` is the wiring Generate builds (export and its readiness check). 0053's Left, Killed and Finished alive survive only as the test-only `KilledAndFinishedAlive`, which builds the flown 2S probe. This record supersedes 0053.

## Consequences

- Generate's output for missions with Air Tasking matches `main` for the count; on `codex/airtask-0053-0055` only 0054 and 0055 change it.
- The walker tests pin the known limit: `count_0058_known_limit_a_late_wreck_empties_a_new_entry` expects the late wreck's `EMPTY`. Building C would flip it. `a_kill_steps_down_once_at_the_wreck_finished` and `a_delete_steps_down_once` replace 0053's other two tests.
- **When to build C.** Look for the race in every flight or server log that has the count in it (flight 2N first). In a trace build it shows as `CLEAR`, then `UP 1`, then `DOWN 1` and `EMPTY` within about 12 s, with no Left from a player and no Delete: the step comes from a wreck removed after `CLEAR`. Without a trace, the sign is a busy area that empties, then a player who enters within seconds and whose AI opponents are deleted while they are still inside. One such case is enough to propose C as a new record.
- Not flown: a plane killed outside whose wreck falls into the circle. If the wreck raises Entered, A counts it up and then down on its Finished, so it does not stick (0053 would have left it counted).
