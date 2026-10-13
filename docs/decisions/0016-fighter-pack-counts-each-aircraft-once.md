# 0016. Fighter Pack counts each aircraft once

- **Status:** Proposed
- **Date:** 2026-10-13
- **Decided by:** open; the user decides. Finding and recommendation by Claude.

## Context
In a Fighter Pack flight every aircraft `n` feeds its flight's `DeathCount` twice (`flights.rs`, `install_pack_logic`):

- its death event (type 13 after decision 0012) targets `DeathCount`;
- its clean-up timer `Delay Delete n` (started by `CLEANUP n` when the aircraft has finished its orders; 60 s by default) targets `Deactivate n` and `DeathCount`.

While the death event was type 4 it never fired, so only the second feed counted. With 0012 both work. An aircraft that has finished its orders and is shot down before `Delay Delete n` runs out is then counted twice, and its flight is replaced one aircraft early.

## Options

| | A. Leave it | B. The death event targets `Delay Delete n` | C. The death event also cancels `Delay Delete n` |
|---|---|---|---|
| Counts per aircraft | one, or two if it is shot down in the clean-up window | always one: the timer is the only feed, and a re-triggered timer restarts and fires once (flight 0, J5) | one |
| New MCUs | none | none; one link per aircraft changes | one `MCU_Deactivate` per aircraft, and more to switch the timer back on for the next wave |
| When a death is counted | at once | after the clean-up delay | at once |

## Decision
None yet. Claude recommends B.

## Consequences
- Under B a replacement flight comes later by the clean-up delay (60 s by default) when its trigger was a death. The cooldown before a replacement is already minutes.
- `Deactivate n` also fires for an aircraft that died. A Deactivate on a dead aircraft does nothing.
- The timer-restart behaviour was measured on game 1.004 (flight 0 and its calibration). Flight 1M repeats that cell on 1.005.
- Generated Fighter Pack files change by one `TarId` per aircraft.
