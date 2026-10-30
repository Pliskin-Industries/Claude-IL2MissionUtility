# 0043. A switch's breadcrumb is pulsed by the switch's own inputs

- **Status:** Accepted
- **Date:** 2026-10-29
- **Decided by:** the user, in the design session after flight A2 ("6. A"), choosing option A, Claude's recommendation.

## Context

- A trace build adds a breadcrumb (a silent Mission Objective) to each traced MCU, by adding it to that MCU's Targets (`trace::instrument`).
- For an `MCU_Activate` or `MCU_Deactivate` (a **switch**), Targets are switched on or off, not pulsed. A breadcrumb there is switched on and never logs.
- Flight A2 showed it: the zones' `ZNaa ARM` (an Activate) never logged, while `ZNaa COOLDOWN`, which fires it in the same tick, did. Replay listed `ARM` as unobserved.
- The probe flights had already worked around this by hand (`observe_command` in `airtask/probe.rs` pulses a switch's breadcrumb from its inputs). The general trace build had not.

## Decision

The trace build pulses a switch's breadcrumb from the switch's own inputs:
- every MCU that pulses the switch through its Targets also targets the breadcrumb;
- every `OnEvent` or `OnReport` hook that names the switch, on an entity or a Complex Trigger, gets a copy that names the breadcrumb.

The breadcrumb logs in the tick the switch is pulsed. Its line means "the switch was pulsed", not "the switch acted": a switch that is itself off drops the pulse, and its breadcrumb still logs. The manual says so (Trace build and Replay).

A switch that nothing pulses can never fire, so it gets no breadcrumb.

## Rejected

- **B. Do not trace switches**: drop `ARM` from the traced roles, because `COOLDOWN` logs in the same tick. It fixes `ARM` only, and leaves the trap for any switch traced later.
- **C. Put a zero-second relay in front of each traced switch.** It would log only when the relay fires, but a trace build would then change the mission's logic, not only add breadcrumbs. Trace builds add breadcrumbs and nothing else.

## Consequences

- Every trace build that traces a switch changes. In the app, that is Air Tasking's `ZNaa ARM` on the Map and Air Tasking tabs. Untraced builds do not change.
- Flight A2's build test no longer compares bytes with its frozen files. A2 is frozen as flown and checked by parse and links, as A0 and A1 are since decision 0038.
- The probe flights keep `observe_command`, so their files regenerate byte for byte. Their per-source tracing path now refuses a switch.
- Probe flight 2K (cell K6) is the first in-game check.
