# 0007. Trace breadcrumbs are C0/S0 objectives

- **Status:** Accepted
- **Date:** 2026-10-03
- **Decided by:** flight 0 results, under the plan's 0d table

## Context
Trace builds need a marker that writes a line to the server's text mission log each time a piece of mission logic fires. Three carriers were tried in flight 0 (2026-09-28 and 2026-10-03).

## Decision
`TraceCarrier::Objective(C0/S0)`: a Mission Objective with Coalition 0 and Success 0. It logs `AType:8` on every firing and the round continues.

## Rejected
- **Objective with Success 1.** It ends a dogfight round about 8 s later.
- **Spawned-and-deleted vehicles.** They write no log line.

## Consequences
- `OBJID` in the log is not the MCU index. Match breadcrumbs by position.
- The log cannot show one MCU firing twice in the same tick (cell T7).
- Log clock: tick = 3 + 50 per second.
- The flight 0 video shows nothing on screen in the cockpit view for these breadcrumbs. Map markers are unchecked, because the map was never opened.
- Evidence: `handoff/P14-probe-results.md` and `References/Recordings/flight0-harvest/README.md`.
