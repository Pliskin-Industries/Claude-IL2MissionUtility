# 0037. Move the Map's own AI aircraft off the player countries when Air Tasking is in the mission

- **Status:** Proposed
- **Date:** 2026-10-27
- **Decided by:** open, for the user. Raised while building acceptance flight A1 (`handoff/sessions/2026-10-27-p14-acceptance-a1.md`).

## Context

Each Air Tasking area has an entry trigger (a Complex Trigger) that counts planes of the other side's **player country**: 601 for NATO, 501 for DPRK (plan §3.5). AI aircraft of that country count like a player (flight 1N; flight 2J used AI of 601 as stand-in players). Export already puts Air Tasking's own AI on 602 (NATO) and 503 (DPRK), so it never starts a cycle (decision 0008, step 7).

The Map's other packs do not. In the acceptance base map, the Map's NATO Fighters Wave 1 is 18 fighters of 601 (`duplicate::apply_overrides(…, 601)`). In a real mission, any Map AI aircraft on 601 or 501 that flies into an area for 60 s starts a sortie against it as a player would. The step 11 check "a winner only where the player loitered" then fails for reasons that are not the player.

For A1 the wave was moved to 602 in the test mission only (the user's request). Generate was not changed.

The other checks filter by **coalition** (the presence check `THsaa INNER`, each sortie's `Zone IN` and `Zone Out`). A country change does not touch them: friendly AI near an area still keeps it HOT and can hold a sortie's Zone Out. Only the cycle's entry is affected.

## Options

- **A. When the mission contains Air Tasking, Export moves every Map AI aircraft on 601 to 602 and on 501 to 503.** Missions without Air Tasking are written as today (`map_generate_without_air_matches_baseline` keeps guarding that). Country 602 and 503 have no name in the editor (loader facts), so a mission maker who opens the file sees a number, not "USA".
- **B. Keep the countries; `validate` warns** when an Air Tasking area exists and the Map holds AI aircraft on a player country. No output change; the mission maker decides.
- **C. Move the Map's AI to 602 / 503 always**, with or without Air Tasking. Changes Generate output for every user; Map logic elsewhere that filters by country 601 / 501 (none known in this repo) would change meaning.
- **D. Leave it as it is** and say it in the manual.

## Recommendation (Claude)

**A.** It removes the false starts where they can happen and changes nothing for a mission without Air Tasking. Step 9 wires Air Tasking into Map Generate, so A would be built there. B is the fallback if the user wants the Map's countries left as the mission maker set them.

## Consequences (if A)

- Step 9's Generate path applies the change after the Map packs are built, only when `air_packs` is not empty, and a test pins it.
- Flight A1's result for the wave (it may keep an area HOT or hold a Zone Out) applies to A as well, because those checks filter by coalition.
