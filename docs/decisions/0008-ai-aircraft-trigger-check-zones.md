# 0008. What to do about AI aircraft triggering check zones

- **Status:** Accepted (option 4)
- **Date:** raised 2026-10-03; options discussed 2026-10-07; decided 2026-10-12
- **Decided by:** the user, after flights 1N and 1D

## Context
P14's player threshold assumed that only players trigger a check zone. Flight 0 showed otherwise (`handoff/P14-probe-results.md`, cell T-a1):

- Zone ZA1 watched NATO planes. A 4-ship NATO **AI** flight crossed it and it fired, before any player was in a plane.
- Zone ZA2, in the same place, watched DPRK planes. The same flight crossed it and it stayed silent.

So the coalition filter works. The problem is only AI aircraft **of the side a zone watches**: for example, DPRK AI sorties flying into a NATO-defended area would look like DPRK players arriving.

The game's check zone filters by coalition only. Every shipped example uses `PlaneCoalitions` and nothing else.

## Options

| # | Option | Extra MCUs | Cost |
|---|---|---|---|
| 1 | **Keep the AI out.** The app refuses a tasking area that the other side's AI sortie routes cross (plan rule D30, made an error). | None | Limits where areas can go. Covers only AI the app knows about. |
| 2 | **Mute the zone while enemy AI is there.** Each enemy sortie's start switches the area's check off; its end switches it on. | A few per sortie and area pair | Players in the area are not detected during that time. More wiring to test. |
| 3 | **Accept it.** The rule becomes "enemy aircraft present", not "players present". | None | Changes what the threshold means. AI of both sides could set each other off. |
| 4 | **Filter by country.** Players fly one country and AI another on the same side; a Complex Trigger filters by country. | About the same | Unverified. The user confirmed (2026-10-07) that no player-only filter exists, and suggested this country idea. |

Countries on the Korea map: DPRK side has 501 USSR, 502 PRC, 503 DPRK. NATO side has 601 USA; 602 and 603 are listed but unnamed and may not work.

## Recommendation (Claude, 2026-10-07)
1. Test option 4 in flight 1A with cells Y1 to Y3 (`handoff/sessions/2026-10-07-flight-0-follow-up.md`).
2. If it fails, use option 1 for v1: it adds no MCUs, which fits the fewest-MCUs rule (U13).
3. Keep option 3 in reserve if option 1 makes area placement too restrictive.

## Decision (2026-10-12)
**Option 4: filter by country.** Players fly one country on each side and the app's AI flies another country of the same coalition. The player threshold uses a Complex Trigger filtered to the players' country.

| Side | Players | App's AI |
|---|---|---|
| NATO | 601 (USA) | 602 |
| DPRK | 501 (USSR) | 503 (DPRK) |

Evidence: flights 1N and 1D (`handoff/P14-probe-results.md`). On both sides the filtered trigger stayed silent for the AI's country and fired for the players' country, AI or human. A plane written as country 602 loads and counts as NATO.

## Rejected
- **1, keep the AI out of tasking areas.** Not needed now; it limited where areas could go.
- **2, mute the zone while enemy AI is there.** More wiring, and players go undetected meanwhile.
- **3, accept that AI sets the threshold off.** Changes what the threshold means.

## Consequences
- Step 4 (threshold) is scoped on a Complex Trigger, not on a plain check zone. The block shape is in `TemplateExamples/Reference/ComplexTrigger_reference.Group`.
- Every AI aircraft the app writes for Air Tasking carries the AI country of its side. The built-in sortie templates are 601 or 503 today, so placing a NATO sortie changes its planes to 602.
- A mission maker's own AI on country 601 or 501 would still set the threshold off. The manual must say so.
- 602 has no name in the game's country table. How it shows on the map and in the statistics screen has not been looked at.
- Not tested: a trigger with two `Country` lines, and two or more players (step 12).
