# 0008. What to do about AI aircraft triggering check zones

- **Status:** Proposed. The user has not decided.
- **Date:** raised 2026-10-03; options discussed 2026-10-07
- **Decided by:** the user, after flight 1A

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

## What the decision waits on
- The Complex Trigger reference file from the editor (`STATE.md`, "Waiting on the user").
- Flight 1A's result for zone ZA4, which confirms that a zone fires for a player at all.

The decision shapes step 4 (threshold) only, and step 4 already waits for flight 1A.
