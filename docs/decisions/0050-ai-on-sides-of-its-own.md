# 0050. Watch players only: put Air Tasking's AI on sides of its own

- **Status:** Proposed (stage 1 passed 2026-10-31; flight 2Q built, not flown)
- **Date:** 2026-10-31
- **Decided by:** open. The user raised the problem and asked for the probe ("Yes, plan the probe", "Yes, run stage 1").

## Context

- Discussing 0048, the user named two faults of a sortie's Zone IN / Zone Out (and of Watchdog v2):
  1. **AI keeps a sortie up.** The zones are check zones, which filter by side only. Enemy AI near the target (another Air Tasking sortie, the Map's AI) stops Zone Out firing, so the sortie runs its full time on station after the players have gone: tick drain on the server.
  2. **35 km cannot see the front.** Players who pass through an area and leave are not tracked; the sortie flies out and sits at an empty target.
- The user's direction: a zone that covers the front, 50 km or more, to track human presence.
- The catch: a wider zone counts even more AI. Near the front the other side's AI is usually inside, so sorties would almost never end early.
- What the game offers:
  - A check zone has `Zone`, `Cylinder`, `Closer`, `PlaneCoalitions` (and linked `Objects`). It has no country filter.
  - A Complex Trigger filters by country, but cannot check who is inside: switched on, it does not report a plane already there (flight 2J). Counting its Entered and Left events cannot find "empty" either, because a counter only counts up.
- Air Tasking's own AI already flies countries of its own: 602 (NATO) and 503 (DPRK) (decisions 0008 and 0037). Each mission's `Options` holds a `Countries` table that puts each country on a side.

## Proposal

Move 602 to side 3 and 503 to side 4 in the `Countries` table. A check zone watching side 1 or side 2 would then see players only, and Zone IN, Zone Out and the area presence check (`THsaa INNER`) would ignore AI. A front-wide Zone Out would then track human presence, as the user wants.

This works only if AI on sides 3 and 4 still fights as before: 602 attacks DPRK and spares NATO; 503 attacks NATO and spares DPRK.

## Stage 1 (flown 2026-10-31): the game accepts sides 3 and 4

Sides check S1 (`handoff/P14-probe-results.md`, "Sides check S1"; logs `real_56`):

- the editor opens the file, the game hosts it, and the log header reads `CNTRS:…503:4,…602:3…`;
- planes on 602 and 503 spawn and die normally;
- players see only Eastern Bloc and Western Bloc;
- the editor and Tacview show sides 3 and 4 with no name, and the editor cannot set them.

## Stage 2: flight 2Q (built, not flown)

No pilot, 9.5 minutes, 28 AI planes. Run sheet: `handoff/sessions/2026-10-31-p14-sides-probe.md`; files `handoff/p14-probes/flight2q/`.

- **Q1, detection:** check zones watching side 2, side 3 and both, armed while empty; a 602 pair comes in, and later a 601 pair. Passes if the side-2 zone fires only for the 601 pair.
- **Q2 to Q7, fighting:** one hunter pair with an air attack order, one target pair orbiting.

  | Cell | Hunter | Target | Must |
  |---|---|---|---|
  | Q2 | 602 (side 3) | 501 (DPRK) | attack |
  | Q3 | 602 (side 3) | 601 (NATO) | not attack |
  | Q4 | 501 (DPRK) | 602 (side 3) | attack |
  | Q5 | 601 (NATO) | 602 (side 3) | not attack |
  | Q6 | 503 (side 4) | 602 (side 3) | attack |
  | Q7 | 503 (side 4) | 501 (DPRK) | not attack |

## Options (for the user after flight 2Q)

- **A. AI on sides 3 and 4, and a wider Zone IN / Zone Out** (if 2Q passes). Export writes the two rows when the mission has Air Tasking. The wider radius is Air Tasking's own, not the shared `AIR_ZONE_OUT_M`, so the upstream Template Builder's output stays the same. No extra MCUs. Generate output changes for missions with Air Tasking.
- **B. Keep the sides and widen the zone anyway.** AI counts, so near the front a sortie stays while anyone fights there; only time on station limits it.
- **C. No change.**

## Rejected (discussion of 0048, 2026-10-31)

The user judged that these do not solve either fault:

- ending a sortie on its area's COLD;
- a 35 km Complex Trigger filtered to the players' country;
- a shorter time on station.

## Open before A could be accepted

- **Anti-aircraft fire:** does a side-2 anti-aircraft gun fire on a side-3 plane? The probe builder cannot place guns yet.
- **What a human sees:** map icons and friendly-fire penalties for side-3 AI, which needs a pilot.
- **The editor:** whether an editor Save keeps sides 3 and 4. A mission maker cannot set them there.
- **Upstream AI:** the Map's own AI moved to 602 and 503 (decision 0037) would move with them.
- **0048 is not changed:** the ALL-out result holds for any side filter.
