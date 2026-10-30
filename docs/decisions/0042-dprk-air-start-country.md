# 0042. The app's DPRK air start writes country 501, the country Air Tasking watches for DPRK players

- **Status:** Accepted (option A)
- **Date:** 2026-10-28
- **Decided by:** the user, 2026-10-29 ("correct the DPRK air start faction"), choosing option A, Claude's recommendation. Found by Claude while building acceptance flight A2 (`handoff/sessions/2026-10-28-p14-acceptance-a2.md`).

## Context

- Each NATO Air Tasking area has an entry trigger (`THNaa TRIGGER`, a Complex Trigger) that counts planes of **country 501** only (`threshold::DPRK_PLAYER_COUNTRY`, plan §3.5). A DPRK player on any other country is never seen, so no NATO sortie starts against them.
- The DPRK airfield in the game's own dogfight test mission (`_test_dogfight_IL-3.Mission`) is country 501. A0, A1 and A2 take their spawns from it (`build_probe_mission.py --spawns`).
- The app's own **air start** writes DPRK fake fields on **country 502** (`airstart::COUNTRY_DPRK`). A Map air start placed for DPRK, or one exported from the Airfield tab, therefore gives a player Air Tasking cannot see. A2 could not use the Map's air start for this reason.
- Other parts of the app differ again: the Map's DPRK fighter packs default to 501, and Air Tasking's own DPRK AI flies 503 (decision 0008). The names also disagree: `aircraft::COUNTRIES` reads 501 USSR, 502 DPRK, 503 PRC; decision 0008 reads 501 USSR, 502 PRC, 503 DPRK.
- A player's country comes from the airfield they spawn at. A mission maker can also build DPRK spawns in the editor on 501, 502 or 503.

## Options

- **A. The DPRK air start writes 501.** One constant changes. Air start exports and Map Generate output change for every mission with a DPRK air start, with or without Air Tasking. It matches the game's own dogfight test mission and the Map's DPRK fighter packs.
- **B. Map Generate moves DPRK air starts from 502 to 501 only when the mission has Air Tasking**, as decision 0037 does for AI aircraft. Missions without Air Tasking do not change. The Airfield tab's own air start export still writes 502.
- **C. Air Tasking's NATO areas watch both 501 and 502.** The Complex Trigger would need two `Country` lines, a shape no file in the repo holds and no flight has tested (loader facts, "The Complex Trigger"), or a second trigger per area. Spawns on either country are seen. 503 cannot be added: Air Tasking's own DPRK AI flies it.
- **D. No change.** The manual says that DPRK players must spawn on 501 for Air Tasking to see them. The app's own DPRK air start stays unusable with Air Tasking.

## Recommendation (Claude)

**A**, with a check of the in-game spawn on 501 in the next piloted flight. It costs no MCU, it makes the app's air starts agree with the game's own DPRK airfield and with the Map's fighter packs, and it removes a trap a mission maker cannot see. B is the choice if Generate output for missions without Air Tasking must not change. C is the only option that covers spawns a mission maker builds in the editor on 502, and it needs a flight first.

## Consequences

- **Built** (branch `claude/p14-dprk-air-start-501`): `airstart::COUNTRY_DPRK` is 501. The Airfield tab's DPRK air starts and its DPRK field spawn both take it from `airstart::side_country`, so both write 501 (pinned by `airstart::tests::dprk_country_and_combined_group`). Generate and export output change for every mission with a DPRK air start or field spawn, with or without Air Tasking. Test: `dprk_air_start_is_on_the_country_air_tasking_watches` (a DPRK air start on the Map is written on the country the NATO areas' triggers watch).
- Decision 0037's move of the Map's AI planes off 501 leaves everything under an `Airfield` alone, so the air start keeps 501 when Air Tasking is in the mission.
- **Not covered:** DPRK spawns a mission maker builds in the editor on 502 or 503. Also upstream commit `571504a` (not merged here) makes harvested DPRK airfields default to 502; if it is merged, its default should use this constant.
- **Still to check in the game:** a player spawning at the app's own 501 air start. A2 flew a 501 air start, but that one was written by `build_probe_mission.py --spawns`.
- `USER_MANUAL.md` (Air Tasking) states which countries Air Tasking watches.
- The country names still disagree (`aircraft::COUNTRIES` reads 501 USSR, 502 DPRK; decision 0008 reads 502 PRC, 503 DPRK). This decision does not rename them.
