# 0032. Air Tasking totals limits (D36) after the upstream author's load test

- **Status:** Accepted (option B)
- **Date:** 2026-10-26
- **Decided by:** the user, 2026-10-26 ("Accept 0032 B, 0033 B, 0034 B and 0035 A. Build step 7 as scoped."), on options and a recommendation by Claude, from the step 7 scope (`handoff/sessions/2026-10-26-p14-step7.md`).

## Context

`validate` warns (orange, never blocks) when a plan adds more than `TotalsLimits` to a mission: 96 aircraft, 1500 MCUs, 40 check zones (`airtask/mod.rs`, plan D36). These were Claude's estimates, to be set "from the Probe 2 load check" (plan §10 item 18).

The load check ran once, on the upstream author's dedicated server (2026-10-14; `handoff/P14-probe-results.md`, "Load test on the upstream author's server"): the old `P14_Load_2` file, 24 copies per side, **240 aircraft, one check zone per copy (48)**, all switched off. Idle: SPS 50.0, tick delay 1.0. With one player in, the tick delay rose as it does for any player. Not measured: two check zones per copy (the current shell, decision 0018), and sorties flying.

What can be flying at once is already capped by the budget (at most 6 copies per side, U20), not by the totals. An idle copy's check zones are never pulsed. So the totals measure mission size and load time more than server load in flight.

The step 7 acceptance plan (all six built-ins, 3 runs each) adds about 84 aircraft, 1,100 MCUs and 40 check zones: it sits on today's check-zone limit.

## Options

- **A. Keep 96 / 1500 / 40.** Conservative; ordinary plans of six to eight entries warn on check zones.
- **B. Set them to what the server ran:** 240 aircraft, the load file's MCU count (measured from the current load probe in the step 7 build and rounded down to 100), 48 check zones. Only proven numbers; a plan of 24 copies per side still warns on check zones, because the current shell has two per copy.
- **C. B, with check zones at 2 per tested copy plus the areas (about 100),** on the reading that idle check zones are never pulsed. Unflown.

## Recommendation

**B.** It uses the only measurement there is and stays a warning. Step 11 notes load time and tick delay next to the totals the plan reports; the limits can move again from that.

## Consequences

- **Values set (2026-10-26):** 240 aircraft, 2,300 MCUs, 48 check zones. The current load probe (`build(Flight::Load)`, 24 copies per side with today's shell) counts 2,349 MCUs, 96 check zones and 240 aircraft; the tested `P14_Load_2` had one check zone per copy and a few MCUs fewer per copy, so 2,349 is rounded down.

- `TotalsLimits::default()` changes (a type default in `mod.rs`: its own small commit by Claude before the step 7 build); no output changes.
- Step 8's totals display reads the limits from the plan; nothing there changes.
