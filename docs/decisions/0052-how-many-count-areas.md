# 0052. How the Air Tasking tab sets the player count: one radius, one area per target

- **Status:** Accepted
- **Date:** 2026-11-01
- **Decided by:** the user, 2026-11-01 ("Approve plan"), on Claude's step 12e plan. Decision 0050 D said "the count circle and the number of areas are set in the Air Tasking tab" (the user: "let's have that area and the number of zones customizable in the wing commander gui"). Claude named the open point, what "the number of zones" means, and offered a second answer ("Areas need a number"); the user approved the plan below.

## Context

- Generate already builds one area per distinct target in a side's pool (`export::areas`, `assemble`). Each area has one threshold and, since decision 0050 D, one player count.
- So the mission maker already sets the number of areas by choosing targets: two assault targets give two areas and two counts (the user's "a zone for each section of the front").
- The count radius was a plan field (`ThresholdSpec.count_radius_m`, 50 km) with no control.

## Decision

- **The number of areas follows the pool's targets.** The tab shows it ("Areas · DPRK n · NATO n") and has no separate number field.
- **One count radius for every area and both sides**: a **Count radius** slider in a new **Player count** section, 10 to 100 km in 1 km steps, default 50 km, with Undo. `validate` refuses a plan radius outside 10 to 100 km (`presence::COUNT_RADIUS_KM`), so the slider never shows a clamped value. The count builder itself takes any positive radius, so the flown probe flight 2R (6 km) builds as flown.
- **Map's Air dock draws the count circles** (layer **Count circles**), so a mission maker can see a spawn inside one.
- **An orange warning** when the count radius is smaller than the threshold's **inner ring** (R × 0.5, 6 km by default): the threshold's trigger and presence check sit on that ring, so a player inside it but outside the count can make the area HOT, and the sortie then finds its area empty and holds for its time on station. It does not block Generate.
  - Corrected before commit (the spec review): the approved plan said "smaller than threshold radius R". R itself only sets the timing, so that comparison warned when nothing was wrong (a count radius between R/2 and R) and named the wrong radius.
- **The THRESHOLD control's N** runs from 1 to 3, one N for both sides (decision 0047).

## Rejected

- **A separate number of areas** (a cap, or merging targets closer than a distance into one area). Not asked for once the targets already give one area each; it would decide which sorties share a count.
- **A radius per area or per side.** One radius is simpler, and nothing yet asks for more.
- **Blocking Generate when the count radius is below the inner ring.** The result (a sortie that holds for its time on station) is today's "empty at arrival" rule (decision 0018), not a broken mission.
