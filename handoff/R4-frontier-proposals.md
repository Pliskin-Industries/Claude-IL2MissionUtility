# R4: frontier feature proposals (P9–P13)

Status 2026-09-24. These came from a frontier-feature review of the whole app. They are **proposals only**: none are scheduled, and none overlap HANDOFF P2–P8 or R3 F1–F9.

Ranked top to bottom. Sizes: S = under a day, M = a few days, L = larger.

## P9. Mission logic checker — M (top pick)

**Problem.** The app exists to take MCU wiring off the mission maker's hands, but the only way to find broken wiring is to fly the mission.
- `bombers::inspect_plan` only checks that at least one `MCU_CheckZone` and one `MCU_Timer` exist, plus a few cleanup/trigger warnings.
- The app's own output already has a silent failure: F1, where a linked flight's cooldown is ignored.
- The IL-2 editor has no debugger.

**Want.** A "Check" panel (or a mode on an existing tab) that takes any `.Group`, including files the app did not write, and lists findings with each object's index and name.

**v0.1 rules** (structural only, so they are simply true or false):
- a `Targets` / `Objects` link that points to an index not in the file;
- an `MCU_CheckZone` with no objects linked;
- a timer or other MCU that nothing ever triggers (no inbound link, and not a mission-begin entry point);
- a trigger loop with no timer in it (it would fire endlessly in the same tick);
- spawn or cooldown settings that will be ignored (the F1 class).

**v0.2.** A dry-run simulator: "player enters zone X at t=0: what fires, what spawns, at what time".

**Acceptance.**
- Every file in `TemplateExamples/` is checked with no crash. Findings on the shipped files are reviewed by hand and are either real or fixed as false positives.
- One test fixture per rule shows the finding, and a clean fixture shows none.
- A file with F1's pattern (linked wingmen + multiple spawns) raises the ignored-cooldown finding.

**Risk.** Some MCU behaviour is undocumented, so false positives are likely. Start with structural rules only.

## P10. Playtest replay — M

**Problem.** There is no way to see what actually happened in a test flight versus what was built.

**Want.** Read IL-2's text mission report (the ATYPE log that community stats tools already parse) back into the app:
- a timeline of spawns, takeoffs, landings and kills;
- spawn positions plotted on the Map tab;
- templates in the loaded mission that never spawned, flagged.

**Acceptance.**
- One real mission report from a test flight parses into a timeline with no unknown-line errors (unknown record types are skipped, not fatal).
- Each spawn in the log is matched to its group in the loaded `.Group` by name where possible.
- A template that never spawned is flagged.
- Test fixtures use a trimmed real log.

**Risk.** The log format is community-documented, not official. The user must turn text logging on in `startup.cfg`. Capture a real log first and build against it.

**Bonus.** Spawn Y values give a free field check for the heightmap work (units spawned underground).

## P11. Historical order of battle from date — M–L

**Problem.** F5 warns about out-of-period types, but the maker still has to research what *should* be there.

**Want.**
- Reference §2.2, §3 and §7 turned into a checked-in data file: types in theatre (from/to dates), active bases, flight sizes, sortie patterns (for example "F-80 flight every 20 min" [S9 p.101]).
- A Map-mode **Historical fill** button that seeds the Fighter Pack and Army Generator pickers with date-valid types for the chosen date and area.
- The source citation written into each generated group's description.
- A coverage indicator for the chosen date, because 1950 is well covered and later years are thin (see R2).

**Acceptance.**
- A 1950 Map date never offers La-9, La-11, or a MiG-15 before 1 Nov 1950.
- Every seeded choice carries a citation that exists in the evidence file.
- The data file is unit-tested against the reference tables.

**Depends on.** F5's in-theatre table (reuse it rather than build a second one).

## P12. Terrain-aware placement — M

**Blocked** until the snapped-grid heightmap is accepted (see the terrain-heightmap memory and the `References/HeightGrid/` tests).

**Want.**
- v0.1: terrain clearance along the **whole** of each flight leg, not just at waypoints. This settles the open question on AGL waypoints.
- Later: line of sight from AAA and searchlight positions to the approach corridor (reference §8.9); Chinese positions placed on reverse slopes (§5 D).

**Design note for the heightmap work now.** Store heights as a queryable grid (`height_at(x, z)`), not only as the relief overlay image.

**Acceptance (v0.1).** A leg crossing a ridge higher than its altitude is flagged, with the ridge position; a test covers it with a synthetic grid.

## P13. Semantic `.Group` diff — S–M

**Want.** A diff between two versions of a mission by meaning, not by text: units added or removed, changed fields (for example a checkzone radius from 5000 to 8000 m), and retargeted MCU links. Objects are matched by index + name.

**First check.** Whether the editor renumbers indices on import/save. If it does, match by name + type + position instead.

**Acceptance.** Diffing a file against itself gives no changes; one edited radius and one retargeted timer are each reported once; a test covers both.

**Risk.** Niche unless the user co-authors MP missions.
