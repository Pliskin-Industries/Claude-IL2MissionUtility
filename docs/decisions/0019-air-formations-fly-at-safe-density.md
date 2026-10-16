# 0019. AI flights fly their formation at density 1 (Safe), not 0 (Dense)

- **Status:** Proposed
- **Date:** 2026-10-15
- **Decided by:** open; the user rules. Proposed by Claude from flights 2C and 2D

## Context

AI F-80C 4-ships held over an empty target lose aircraft with no attacker. Flight 2C (`handoff/P14-probe-results.md`, "Flight 2C") showed with Tacview that every loss was two wingmen of one flight colliding. That server was overloaded, so flight 2D ("Flight 2D") flew 16 aircraft on a server that kept up, changing only the Formation command's `FormationDensity` (0 Dense, 1 Safe, 2 Loose, by the editor's list order) and, in one cell, the type (Pairs).

Flight 2D, three runs:

| Formation | Nearest wingman, median | Aircraft-seconds under 20 m | Losses |
|---|---|---|---|
| Wedge, 0 Dense (as shipped) | about 100 m | 4, 6, 4 | two aircraft in two runs |
| Wedge, 1 Safe | about 140 m | 0, 0, 0 | none |
| Wedge, 2 Loose | about 500 m | 0, 0, 0 | none |
| Pairs, 0 Dense | about 50–80 m | 13, 26, 26 | two aircraft in one run |

Every one of the six built-in sorties (`TemplateExamples/Historical1950/`) flies density 0: four in Wedge, two (Yak raid, F-51 ArmedRecon) in Pairs. Template Builder writes 0 for every air formation (`src/template.rs`, `air_form`).

## Decision (proposed)

Air formations fly at `FormationDensity = 1` (Safe). The user picks where it applies:

1. **Air Tasking's placed copies only:** the shell sets density 1 on each copy's Formation commands, as it already sets `AiRTBDecision = 0` (decision 0015). No curated file or generator output changes. 0 MCUs. **Recommended first step.**
2. **The six built-in files:** set density 1 in their nine Formation commands. Changes what Template Builder users get from those files (decision D28 kept the curated files out of scope; STATE "Waiting on the user" item 3 already asks about editing them).
3. **Template Builder's default for air formations** (`air_form` density 0 → 1): changes every generated air group's output file.

## Rejected (proposed)

- **Density 2 (Loose).** Also clean, but about 500 m apart the flight no longer looks or acts like a formation; Safe is the smallest change that removed every close pass.
- **Accept the losses (option I).** Two of three healthy-server runs lost two aircraft in the as-shipped Dense Wedge.
- **Fewer aircraft per server (option H).** Flight 2D lost aircraft at Dense with no overload, so overload is not the cause.
- **Change the attack order or the payload (options A, A′, B, C).** Flight 2C showed they do not affect the losses.

## Consequences

- Three runs per cell are few: zero collisions at Safe is not proof it never collides. The spacing (closest 27 m against 4 m at Dense) is the stronger evidence. A later flight with more sorties at Safe would confirm it.
- Option 1 needs a shell change and tests (each copy's Formation commands); the frozen probe files of flights 0 to 2D do not change.
- Pairs at Dense flew closest of all; option 1 would also move the Yak and ArmedRecon copies to Safe.
