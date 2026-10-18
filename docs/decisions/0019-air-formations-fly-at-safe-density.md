# 0019. Every AI air formation flies at density 1 (Safe) until Dense is fixed

- **Status:** Accepted
- **Date:** 2026-10-15
- **Decided by:** the user ("Every formation needs safe until Dense is 'fixed'"; options A, B and C), on Claude's proposal from flights 2C and 2D

## Context

AI F-80C 4-ships held over an empty target lose aircraft with no attacker. Flight 2C (`handoff/P14-probe-results.md`, "Flight 2C") showed with Tacview that every loss was two wingmen of one flight colliding. That server was overloaded, so flight 2D ("Flight 2D") flew 16 aircraft on a server that kept up, changing only the Formation command's `FormationDensity` (0 Dense, 1 Safe, 2 Loose, by the editor's list order) and, in one cell, the type (Pairs).

Flight 2D, three runs:

| Formation | Nearest wingman, median | Aircraft-seconds under 20 m | Losses |
|---|---|---|---|
| Wedge, 0 Dense (as shipped) | about 100 m | 4, 6, 4 | two aircraft in two runs |
| Wedge, 1 Safe | about 140 m | 0, 0, 0 | none |
| Wedge, 2 Loose | about 500 m | 0, 0, 0 | none |
| Pairs, 0 Dense | about 50–80 m | 13, 26, 26 | two aircraft in one run |

Before this decision every one of the six built-in sorties (`TemplateExamples/Historical1950/`) flew density 0 (four Wedge, two Pairs), and Template Builder wrote 0 for every air formation (`src/template.rs`, `air_form`).

## Decision

Every AI air formation the app writes or places flies at `FormationDensity = 1` (Safe):

- **A. Air Tasking** sets density 1 on every Formation command of each copy it places, whatever its source template says (as it sets `AiRTBDecision = 0`, decision 0015). 0 MCUs.
- **B. The six built-in sorties** carry density 1 in all nine of their Formation commands.
- **C. Template Builder** writes density 1 by default for every air formation (`air_form`). Ground (vehicle) formations keep their values.

This stands until the game's Dense formation is fixed. The collision is drafted as a game bug in `handoff/il2-bug-report-drafts.md`; when a game update fixes it, a new record can supersede this one.

## Rejected

- **Density 2 (Loose).** Also clean, but about 500 m apart the flight no longer flies as a formation; Safe is the smallest change that removed every close pass.
- **Only one of A, B, C.** The user wants every formation safe: A alone leaves the files and Template Builder Dense; B or C alone leaves a user template placed by Air Tasking Dense.
- **Accept the losses (option I).** Two of three healthy-server runs lost two aircraft in the as-shipped Dense Wedge.
- **Fewer aircraft per server (option H).** Flight 2D lost aircraft at Dense with no overload.
- **Change the attack order or the payload (options A, A′, B, C of the options note).** Flight 2C showed they do not affect the losses.

## Consequences

- **Generated files change on purpose:** every air group Template Builder writes and the six example files now say `FormationDensity = 1`. This overrides the rule that Generate writes the same files for the same inputs, for this one property.
- Flown probe files built from the Dense originals must still regenerate byte for byte or be frozen, as flight 2 was under decision 0018.
- Evidence covers F-80C Wedges only. Pairs at Safe and the propeller types (F-51, Yak, Il-10) are not flown yet; a confirming flight should cover them.
