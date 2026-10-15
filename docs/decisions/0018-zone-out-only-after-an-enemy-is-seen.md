# 0018. A sortie's Zone Out ends it only after an enemy has been seen inside

- **Status:** Accepted
- **Date:** 2026-10-14
- **Decided by:** the user ("a sortie should stay on station at an empty target. Zone Out should only end it after an enemy has been seen inside"), on Claude's finding from flight 2 and its proposal for the wiring

## Context

Flight 2 (`handoff/P14-probe-results.md`, "Flight 2") flew two placed sorties built by the step 3 shell. Both ended the moment they reached WP 1: the shell switches Zone Out on at WP 1 and pulses it 0.1 s later, and Zone Out (`Closer 0`, 35 km, the other side's coalition) fires at once when no enemy aircraft is inside. So a sortie at a target with nobody around turns for home on arrival.

The six built-in sorties already hold the logic that avoids this. The template pulses **Zone IN** (`Closer 1`, the other side's coalition) after mission start; only when Zone IN fires does it switch Zone Out on and pulse it. Step 3's shell deletes Zone IN and arms Zone Out from WP 1 (plan §3.2, "Kept and rewired").

Flight 0's cell T-z had recorded that an empty `Closer 0` zone does not fire on Activate plus a pulse. That cell switched the zone on and pulsed it in the same tick, from one timer. Flight 2 switched Zone Out on and pulsed it 0.1 s later, as Template Builder does, and it fired at once. The T-z result was most likely an artifact of the same-tick order (an inference from the two wirings, not a separate test).

## Decision

1. **The shell keeps the template's Zone IN** as the copy's "enemy seen" check. It is re-centred on the target with Zone Out's radius, keeps its coalition filter, and its only targets become Zone Out's existing arming pair (the Activate and the 0.1 s pulse timer).
2. **WP 1 of the navigation lead pulses Zone IN**, in place of Zone Out's arming pair.
3. Nothing else in the shell changes: on-station time, `HOT` / `COLD` / `EXTEND`, `LOSSES`, `HARD STOP`, the Delete chain and the budget stay as decision 0017 left them.
4. The load file for the upstream test (eszyman/IL2MissionUtility#2) is rebuilt with this shell before it is sent.
5. A re-fly, flight 2B, checks it: an empty target with the area HOT and with it COLD, an enemy flight present at arrival that then leaves, and one that arrives after the sortie and then leaves. Every input that can end a sortie carries a breadcrumb.

## Rejected

- **Keep it as built** (a sortie at an empty target goes home on arrival). The user ruled against it.
- **A new check zone of the shell's own.** The same count as reusing Zone IN, but a new block to build and test; Zone IN is already in every accepted template with the right shape and filter.
- **One shared "seen" zone per target for all copies.** Fewer check zones, but it cannot tell whether the enemy was seen during this copy's time on station without more state per copy.

## Consequences

- Per copy: 20 MCUs (was 19) and 2 check zones (was 1). At 24 copies per side that is 96 check zones per mission, above the plan's rough warning level of 40 (D36); the load test measures what that costs.
- **AI counts as seen**, not only players (flight 0, risk a). An enemy Air Tasking sortie passing near the target keeps this sortie on station until it leaves. Filtering to players needs the country filter of decision 0008 and is left to step 4.
- An empty target now holds the sortie for its full on-station time, then `COLD` ends it or `HOT` extends it once.

## Correction added 2026-10-14 (MCU counts)

The first Consequences line above says "20 MCUs (was 19)". That was Claude's wording error, caught by the implementer: the shell's count tests measure the MCUs the shell adds, and keeping Zone IN removes one template MCU fewer rather than adding one. Correct: the shell still adds 19 MCUs to a wrapped HVAR copy (18 with a template RTB timer) and removes 8 template MCUs (was 9; 7 with a template RTB timer, was 8), so each copy holds one MCU more than under decision 0017, and 2 check zones. Details: `handoff/P14-step-3c-count-correction.md`.
