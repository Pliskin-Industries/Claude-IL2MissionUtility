# 0050. Watch players only: put Air Tasking's AI on sides of its own

- **Status:** Proposed. Option A fails (flight 2Q). The user chose option D for testing (2026-10-31). Flight 2R flew it as built, every cell as predicted; the user accepts D or not.
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

## Flight 2Q result (flown 2026-10-31)

`handoff/P14-probe-results.md`, "Flight 2Q"; logs `real_57`.

- **AI on a side of its own is everyone's enemy.**
  - 602 (side 3) fired on 601 (NATO), and 601 fired on 602.
  - 503 (side 4) fired on 501 (DPRK).
  - The attack cells fired too.
- **Check zones did not see side-3 planes at all,** even zones watching `[3]`. A side-2 zone ignored them, but that does not help when they shoot their own side's players.
- **Caveat:** no cell had a hunter and target on the same side, so 2Q alone cannot rule out that an air attack order fires at any aircraft. If the user wants option A kept alive, a same-side control cell is the next check: for example, 601 hunting 603, both on side 2.

## Options (for the user after flight 2Q)

- **A. AI on sides 3 and 4, and a wider Zone IN / Zone Out.** **Fails (flight 2Q):** such AI attacks its own side. Export writes the two rows when the mission has Air Tasking. The wider radius is Air Tasking's own, not the shared `AIR_ZONE_OUT_M`, so the upstream Template Builder's output stays the same. No extra MCUs. Generate output changes for missions with Air Tasking.
- **B. Keep the sides and widen the zone anyway.** AI counts, so near the front a sortie stays while anyone fights there; only time on station limits it.
- **C. No change.**
- **D. A player count per area, built from Complex Trigger events (chosen for testing).** See below.

## Option D (the user's choices, 2026-10-31)

Different countries on the same side is already Air Tasking's set-up: AI flies 602 and 503, players 601 and 501. Only the Complex Trigger filters by country, and it reports events, not who is inside. D builds "who is inside" from those events.

- **One count per area** ("a zone for each section of the front": two assault areas, two counts). Each area's sorties listen only to their own area's count.
- **The count circle and the number of areas are set in the Air Tasking tab** (step 12e). Default radius 50 km. Circles may overlap.
- **No player spawn inside a count circle.** Generate refuses a mission with an airfield or air start inside any area's circle. The user: "A plane will not spawn in that zone." So the trigger never misses a player who appears inside it (flight A1).
- **Counting.** A Complex Trigger filtered to the enemy players' country:
  - Entered (58) moves the count up one step.
  - Left (60) and Finished (62) move it down one step. Correction (2026-10-31, while building): not Killed (70). A kill raises 70 at once and 62 when the wreck lands, about 15 s later (flight 2P), so counting both would take two steps. A kill therefore counts down about 15 s late.
- **Count to three, then busy** (the user). A fourth entry switches the area to busy. In busy mode leaves do not lower the count. Busy clears only when a side-only Closer 0 check zone over the same circle finds nobody, AI included: today's Zone Out behaviour (ALL-out, flight 2M).
- **A sortie ends when its area empties while it is on station:** the count drops from 1 to 0, or busy clears.
  - **An empty area at arrival:** the sortie carries on with its mission (the user; decision 0018 stands).
  - **Replaces** the per-copy Zone IN and Zone Out, two check zones per copy.
- **Rejected in the discussion:**
  - **One count per side:** the user wants sections of the front tracked apart.
  - **Busy at activation:** enemy AI near the front would hold every active area's sorties again.
  - **A larger cap (10):** the user chose three.
- **Built for testing (2026-10-31):** `src/airtask/presence.rs`, 26 MCUs per area; probe flight 2R (`handoff/sessions/2026-10-31-p14-flight-2r.md`).
  - Each step comes on one tick after the old one goes off, so an event within about 0.05 to 0.1 s of a step change is lost. A lost entry ends the count low (the area empties early); a lost leave ends it high (safe).
  - `CLEAR` is armed 2 s after busy, so the fourth plane is well inside first.
- **Flight 2R (2026-10-31, logs `real_58`):** the count chain works in the game.
  - One step per entry; Left and Finished down; a kill counts down when the wreck lands.
  - Busy cleared by an empty circle twice in a row.
  - Two leaves in one tick count once (safe).
  - A plane killed or deleted outside after its Left raises nothing.
- **Still to fly:**
  - two entries close together (the unsafe direction);
  - what a player who quits in flight raises (flight 2N).
- **After 2R, if it passes:**
  - wire `EMPTY` into the sorties in place of Zone IN and Zone Out;
  - add Generate's no-spawn-inside check;
  - add the 50 km default and the Air Tasking tab settings.
- **Note:** the rejected "35 km Complex Trigger" below is a trigger with no count; D adds the count.

## Rejected (discussion of 0048, 2026-10-31)

The user judged that these do not solve either fault:

- ending a sortie on its area's COLD;
- a 35 km Complex Trigger filtered to the players' country;
- a shorter time on station.

## Open items for A (moot after flight 2Q)

- **Anti-aircraft fire:** does a side-2 anti-aircraft gun fire on a side-3 plane? The probe builder cannot place guns yet.
- **What a human sees:** map icons and friendly-fire penalties for side-3 AI, which needs a pilot.
- **The editor:** whether an editor Save keeps sides 3 and 4. A mission maker cannot set them there.
- **Upstream AI:** the Map's own AI moved to 602 and 503 (decision 0037) would move with them.
- **0048 is not changed:** the ALL-out result holds for any side filter.
