# 0012. Which event counts a destroyed aircraft in Template Builder and Fighter Pack

- **Status:** Accepted (option B)
- **Date:** 2026-10-12
- **Decided by:** the user, 2026-10-13 ("0012 go with recommendation"), after flight 1L. Findings and recommendation by Claude.

## Context
Flight 1G (`handoff/P14-probe-results.md`, "Flight 1G"; logs `src/testdata/missionlog/real_6/`; game `261002`) killed three AI aircraft, two by Damage command and one by gunfire. Each raised OnEvent **13** once, then 0 and usually 2. **None raised type 4.** Decision 0011 moved Air Tasking to 13 and left the rest of the app for this record.

**What the app writes on type 4 today** (`EntityEvent::default_for(Plane)`, `template.rs:601`):

| Where | Wiring | In game, if type 4 never fires |
|---|---|---|
| Template Builder, Spawn with **Allow multiple spawns** (`template.rs:3143`) | each aircraft → `DeathCount` (count = seats) → `COOLDOWN` → `Trigger Spawner` | The counter never fills, so no next wave while the player stays inside. Leaving the zone still cleans up and a new entry spawns again. A group that mixes aircraft and ground units has the same fault, because the counter needs every seat. |
| Fighter Pack (`flights.rs:396`, `:764`, `:826`) | one `DeathCount` per flight → `COOLDOWN` → randomizer → next flight. Each aircraft feeds it twice: its type 4 event, and its `Delay Delete n` timer | An aircraft is still counted if it first reports events 1, 3, 7 or 8 or ends its orders (`Mission Complete n` → `CLEANUP n` → `Delay Delete n`, 60 s by default). An aircraft killed with none of those is never counted, and that flight is never replaced while the player is inside. |
| A user's **OnPlaneDestroyed** event hook | → Force Complete or an order | Never runs. |
| Air Tasking Repeat sorties (`airtask/library.rs:437`, `testkit.rs:624`) | accepts only Type 4 → `DeathCount`; the test walker fires type 4 on a kill | Same as Template Builder. Any change below must be made here too, or new Repeat sorties are refused. |

These effects are read from the wiring. None was flown.

**Shipped examples with type 4 on aircraft** (16 files, 827 aircraft; no example has type 13 on an aircraft):

- → `DeathCount`: `Eastern_Fighters_Random_3pack_V6` (48), `_5pack` (100), `_5pack_V6` (80), `_8pack` (160), `_10pack` (200), `Weastern_Fighters_Random_3pack` (51), `ImprovedCooldownLogic` (4), and the three Scramble copies, 4 of 7 aircraft each (`DPRK_Scramble`, `BomberMissions/DPRK_Scramble`, `GroundUnits/DropIns/DPRK Scramble`).
- → `DeathCount`, `Destroyed Counter` or `DestroyCounter - Bombers`: `Exclusive` (49), `Exclusive_Activation_6plan` (53 of 56), `BomberMissions/B29Mission` (17), `BomberMissions/Linked_Bomber_Missions_4plan` (41), `BomberMissions/DPRK Bombers - Small Formation` (8).
- → `Force Complete - High`: `CorrectedFlightfromTemplate` (4).
- **`Historical1950/` (the six built-in sorties): none.** They are not Repeat sorties and hold no OnEvent. The comment at `library.rs:110` describes Repeat sorties a user builds.

Tabs that copy a loaded template (Exclusive, pack copies, Recon, Duplicate) pass these events through unchanged. No option below alters them.

## Decision
None yet. Claude recommends option B, after one no-pilot probe flight that answers the three unknowns under Consequences.

## Options

| | A. Keep type 4 | B. Aircraft default becomes 13 | C. Attach 4 and 13 |
|---|---|---|---|
| Generated files | No change | One line per aircraft: `Type = 4;` becomes `Type = 13;`. Same MCUs, indexes and targets. | One more `OnEvent` block per aircraft. No new MCU. |
| Which files | none | Template Builder files with Allow multiple spawns, and every Fighter Pack file. Baselines `fighter.Group` (27 lines) and `base_map.Group` (18) are re-recorded. | Same files as B |
| In game on `261002` | The faults in the table above stay | The counters count a kill | Same as B |
| Main risk | A feature in the manual does not work for aircraft | Type 13 is untested for some deaths; Fighter Pack can count one aircraft twice | If one death ever raises both events, every loss counts twice and the next wave spawns early |

## Consequences
- **Not known, and needed before B or C:** whether type 13 fires for an aircraft that crashes with no damage, or whose pilot bails out; and whether events 1 or 3 fire on a kill (1G listened for 0, 2, 4, 5 and 13 only).
- **Fighter Pack double count.** With a death event that fires, an aircraft that enters `CLEANUP n` and is then killed inside the `Delay Delete n` window is counted twice. The counter resets when it fills, so two such aircraft end a 4-ship's slot early. The same fault is on paper with type 4. Template Builder has one feed per aircraft and does not have it.
- B and C also need: `inspect_hooks` and the test walker to accept the new event (C: two events per aircraft), three tests in `template.rs`, and `USER_MANUAL.md` lines 98 and 114. An old template loaded into Template Builder is rewritten with the new event, because the loader ignores the type of a `DeathCount` event.
- The 16 example files keep type 4 under every option. Editing them is a mechanical change with its own commit, and it changes the `exclusive.Group` baseline.
- Not known: whether type 4 fired in an earlier game version. The evidence is one flight, three aircraft, one game version.

## Evidence added 2026-10-13 (flight 1L)
The unknowns listed under Consequences were tested (`handoff/P14-probe-results.md`, "Flight 1L"; logs `src/testdata/missionlog/real_15/`).

- **The app's own respawn loop.** A repeat-spawn group from `generate_template`, as generated (event 4): the first wave was killed and `DeathCount` never fired; no second wave came. The same group with the event changed to 13: `DeathCount` fired at the kill, `COOLDOWN` ran, the next wave spawned, and the cycle repeated.
- **Other ways to be lost.** Event 13 fired for a fuel-starved aircraft at the moment its pilot bailed out, and (flight 1K) for a mid-air collision and a crash during a fight. Event 4 fired for none of them.
- **Events 1 and 3 on a kill:** both fire, 0.06 s after 13. Fighter Pack's own hooks on 1, 3, 7 and 8 are a separate matter: event 7 also fired three seconds after activation on aircraft with full tanks, which flight 1M looks at.

Claude's recommendation became option B without a further probe, and the user accepted it the same day. The change is tested again in flight 1M, on groups generated by the changed code.

