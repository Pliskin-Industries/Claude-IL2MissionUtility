# 0055. The clock ends a draw through a gate after RANDOM, so a paused RANDOM cannot drain (closes 0044's open clock edge)

- **Status:** Accepted
- **Date:** 2026-11-01
- **Decided by:** the user, 2026-11-01 ("Fix 1–9, and A–C too"), on Claude's triage of Astra's review of `b5acb35..6f02211` (finding C, Astra job `20261101145549-c0f120bc`, item 3). Decision 0044 left this edge open, unchanged, and noted it for the user; this record closes it. 0044 stays as written.

## Context

- In the clock (`src/airtask/clock.rs`), `EXCLUDE` and `AREA OUT` switch an entry's `RANDOM` timer off. A switched-off running timer is paused, not stopped (flight 2K, decision 0044).
- A `RANDOM` that `AREA OUT` paused, and whose entry is excluded before each of the next slots, loses 0.1 s per slot. Its remainder then runs out before `DRAW` (+0.3 s) in a later slot. `DRAW` reopens the outputs and starts `RANDOM` again, so the entry can launch twice in one slot.
- Astra's scenario: two `Both` entries share an area, A first with one run, B last with six, budget two; a cold slot, then zone launches that exhaust A and set B's `LAST` before three slots in a row.

## Options

**A. Make `AVAILABLE` a D11 gate after `RANDOM` (chosen).** 0 MCUs.
- `RANDOM` runs out into `AVAILABLE`, which passes to `OUT` and `CLOSE`. `EXCLUDE` and `AREA OUT` switch the gate, not the timer, so `RANDOM` is never paused. A `RANDOM` that runs out into a closed gate is dropped, as `ATTEMPT END` is dropped by `REROLL` (0044).
- `SLOT OPEN` switches the gate on for the slot. Exhaustion also switches it off for good.
- The final wiring is Astra's to propose and Claude's to review: a walker test (flown timer facts, `WalkerConfig::flown()`) must reproduce the drain before the change and show one launch per slot after it.

**B. Leave it (0044 as decided).** Rejected now that a 0-MCU fix exists.

**C. Stop `RANDOM` through an extra relay per entry.** +1 MCU per entry. Rejected: more MCUs than A.

## Decision

**A.** Built with the walker test above. It uses only flown mechanisms (a paused timer, a D11 relay that drops an event while off), so no flight is required; it rides along in the next no-pilot flight if one is flown.

## Consequences

- **Output changes** for missions with Air Tasking: each clock entry's links change. MCU count is unchanged.
- Trace builds may name different clock roles; the trace role list follows the new wiring.
- Frozen probe and acceptance files are not regenerated.
