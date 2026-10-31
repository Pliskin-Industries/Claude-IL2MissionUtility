# 0047. What a threshold of N ≥ 2 players means: the count resets when anyone leaves

- **Status:** Accepted (option E)
- **Date:** 2026-10-31
- **Decided by:** the user, 2026-10-31: "E with test", "N to 3", "N for both sides", "N at 3 to start then 1 to hold". This was on Claude's scenario comparison in the step 12 session (`handoff/sessions/2026-10-31-p14-step12-scope.md`). Claude read "N at 3" as the upper limit, with the default staying 1 (D1), because a default of 3 would change Generate output for existing plans.
- **Changes:** U1's "window W". U1 asked for a Complex Trigger, a counter, a window W and a dwell check. The window is dropped; the reset on Left takes its place.

## Context

- **As built (step 4, decisions 0023 to 0025):** `THsaa TRIGGER` reports Entered (58) to the `ARM` latch.
  - The trigger is a Complex Trigger at r_in, filtered to the players' country.
  - `ARM` switches the trigger off and starts the dwell D.
  - Then the presence check `INNER`, a check zone, holds the area HOT or lets it go COLD.
  - N is fixed at 1.
- **Game facts:**
  - Entered (58) and Left (60) come once per aircraft (flights 1N, 1D).
  - A counter fires at its `Counter` value (E1).
  - `MCU_ModifierSetVal` re-arms a counter that has fired (E6).
  - A counter cannot count down.
  - So "N planes inside now" cannot be counted directly; each option is a rule for when to reset a count of entries.
- **The cases compared:**
  - two players arrive together;
  - one loiters and a second arrives minutes later;
  - one circles across the edge;
  - one flies through, and a second arrives just after;
  - a death inside.

## Decision

**E. The count resets when any watched plane leaves.**
- Wiring for N ≥ 2:
  - Entered → `THsaa COUNT` (Counter = N, Dropcount 0) → `THsaa ARM` (as built).
  - Left → `THsaa ZERO` (`MCU_ModifierSetVal`, ParamIndex 0, Data0 0, on `COUNT`).
  - `REARM ON` also pulses `ZERO`, so a failed dwell starts a fresh count.
- **Meaning:** the area arms when N planes of the players' country have entered and none has left since.
- **Cost:** 2 MCUs per (side, area). There is no timer.
- **N runs from 1 to 3; the default is 1.** One N serves both sides. `validate` refuses other values.
- **For N = 1 the wiring and Generate output stay exactly as they are:** Entered → `ARM`, with no counter.
- **N to start, 1 to hold.** The presence check is unchanged: once HOT, one plane of the players' coalition inside r_in keeps the area HOT. The extension (U17) and the zone front end are unchanged.
- **The test.** Flight 2M (decision 0046) checks whether the trigger reports Left when a plane dies inside and when it is deleted inside. It does this alongside cells D1 to D5, flown with E.
- **No safety-net window now.** If 2M shows that a death or a Delete raises no Left, a dead plane's entry stays counted, and the next single arrival arms the area. That is today's N = 1 behaviour, so the effect is mild. The user then decides whether to add a long restarting window (about 2 more MCUs per area).

## Rejected

- **A. A restarting window, W = D (60 s).**
  - It never arms for a loitering player plus a later arrival.
  - It arms with one plane present when that plane circles across the edge, or when a pass-through is followed by an arrival within W.
  - It costs about 4 MCUs per area.
- **A-long, the same with W about 5 min.** More false arms from one plane crossing the edge.
- **B. The fixed bucket of plan §3.9.**
  - A timer loops all mission on every area.
  - Two entries either side of a reset are missed.
- **C. No reset at all** (except at `REARM`): two entries an hour apart would arm the area.
- **One N per side.** The user chose one N for both sides.
- **"N to hold".** A check zone cannot count planes, so it would need a different design and much more logic.

## Known weaknesses (for the manual)

- With N = 3, if one of the first two leaves before the third arrives, the count is lost. A later arrival must make up the full N again. This fails safe: no sortie.
- An AI pair on the players' country counts as two. A mission maker's own AI on 501 or 601 counts.
- A player who spawns inside the area is not counted until they leave and come back (flight A1).
- Unless flight 2M shows otherwise, a plane that dies or is removed inside may stay counted.

## Consequences

- Step 12c builds E with walker tests first. Flight 2M flies it with AI stand-ins, plus the Left cells.
- Step 12e's THRESHOLD control runs from 1 to 3, with no W field.
- Plan §3.9's window design is superseded; §4.2's W applies to nothing in the build.
