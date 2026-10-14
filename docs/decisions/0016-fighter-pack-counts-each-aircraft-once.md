# 0016. Fighter Pack counts each aircraft once

- **Status:** Accepted (option B)
- **Date:** 2026-10-13
- **Decided by:** the user, 2026-10-14 ("0016 acceptable"). Finding and recommendation by Claude.

## Context
In a Fighter Pack flight every aircraft `n` feeds its flight's `DeathCount` twice (`flights.rs`, `install_pack_logic`):

- its death event (type 13 after decision 0012) targets `DeathCount`;
- its clean-up timer `Delay Delete n` (started by `CLEANUP n` when the aircraft has finished its orders; 60 s by default) targets `Deactivate n` and `DeathCount`.

While the death event was type 4 it never fired, so only the second feed counted. With 0012 both work.

**Every kill counts twice (found in review, 2026-10-13).** Fighter Pack's own hooks (`FIGHTER_EVENTS`, `flights.rs`) send an aircraft to `CLEANUP n` on pilot wounded (1) and critical damage (3), among others. Flight 1L showed that an outright kill raises 13, and then 1 and 3 about 0.06 s later. So a kill ticks `DeathCount` at once through the death event, and again when `Delay Delete n` runs out. A 4-ship would be replaced after two losses. This is read from the wiring and the flight 1L event times; Fighter Pack itself has not been flown.

It also explains why Fighter Pack worked before 0012: the death event never fired, and events 1 and 3 counted each kill once, a clean-up delay late.

**Decision 0012 must not ship without this one.**

## Options

| | A. Leave it | B. The death event targets `Delay Delete n` | C. The death event also cancels `Delay Delete n` |
|---|---|---|---|
| Counts per aircraft | **two for every aircraft that is killed** | always one: the timer is the only feed, and a re-triggered timer restarts and fires once (flight 0, J5) | one |
| New MCUs | none | none; one link per aircraft changes | one `MCU_Deactivate` per aircraft, and more to switch the timer back on for the next wave |
| When a death is counted | at once | after the clean-up delay | at once |

## Decision
**Option B.** Each aircraft's death event targets its own `Delay Delete n` timer instead of `DeathCount`. Built and flown in plan step 0k (flight 1N); the event 13 fix (0012) is not released before this one.

## Consequences
- Under B a replacement flight comes later by the clean-up delay (60 s by default) when its trigger was a death. The cooldown before a replacement is already minutes.
- `Deactivate n` also fires for an aircraft that died. A Deactivate on a dead aircraft does nothing.
- The timer-restart behaviour was measured on game 1.004 (flight 0 and its calibration) and again on 1.005 in flight 1M (two runs): a 10 s timer re-triggered at 5 s fired once, 15 s after its first trigger.
- Generated Fighter Pack files change by one `TarId` per aircraft.
