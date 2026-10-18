# 0022. Pairs at Loose does not hold for a covering pair: what the app writes for the F-51 ArmedRecon

- **Status:** Proposed
- **Date:** 2026-10-18
- **Decided by:** open, for the user; proposed by Claude from flight 2H

## Context

Decision 0020 (accepted 2026-10-18, option A) has every Pairs formation fly Loose, from the Yak airfield raid's evidence: at Safe its wingman crosses its lead at the attack-entry reversal and collides (five collisions in nine runs, flights 2E to 2G); at Loose it kept every pass above 20 m with no losses (flights 2F and 2G). A is not built yet.

Flight 2H (`handoff/P14-probe-results.md`, "Flight 2H") flew the other Pairs built-in, the F-51 ArmedRecon (S3), three runs, once at Safe and twice at Loose per run:

| F-51 ArmedRecon at | Closest (each run) | Lost |
|---|---|---|
| Safe, as shipped (2E EA and 2H HS, six runs) | 17, 9.9, 18.4, 20.3 m (and over 20 m in two) | 0 |
| Loose (2H HL1 and HL2, six samples) | 14.4, 19.2, 25.3, **7.7 (collision)**, 12.8, 23.0 m | 2 aircraft, one collision |

The F-51's Blue pair flies a Cover order on the Red (attack) pair and weaves in 50–60° S-turns all through the transit while it descends to the Red pair's height. At Safe its wingman stays on its own side through every reversal. At Loose each reversal swings the wingman 150–400 m across its lead's track; in one run it collided. The Red pair at Loose behaved like the Yak at Loose (crossing at the entry reversal with a vertical step), 12.8 m once.

So A helps a pair that reverses once on arrival (the Yak) and hurts a pair that weaves (a covering pair). The crossings are the game bug of decision 0021 either way.

## Options

- **A1. Build 0020 A as decided: every Pairs formation flies Loose, the F-51 included.** One value per formation type, no exceptions. The Yak's collisions stop; the F-51's covering pair is exposed to the weave crossing (one collision in three runs).
- **A2. Build A with one exception: a flight under a Cover order keeps its formation at Safe.** Placement (`inspect_sortie`) and Template Builder (`OrderKind::Cover`) already know which flight flies a Cover order; the rule is "Pairs → Loose, unless the flight's lead is commanded by an `MCU_CMD_Cover` (its `Objects`)", still a value, no MCUs. It follows the mechanism seen in Tacview (the weave comes from Cover). The combination itself (attack pair Loose, cover pair Safe) has not been flown; each half has (HL Red pairs, HS Blue pairs).
- **A3. Build A, and keep the F-51 ArmedRecon built-in's own pairs at Safe** (an exception for one file). Simplest to state, but a Template Builder user who makes a covering Pairs flight still gets Loose.
- **A4. Hold A and fly more first** (for example the A2 combination, three runs). Costs a flight; the Yak keeps colliding meanwhile in the app as shipped.

## Recommendation

**A2**: Pairs fly Loose except a flight flying a Cover order, which stays Safe. It keeps A's fix for the Yak, avoids the one failure 2H showed, and stays a value change. If the user wants the combination flown before it ships, fly it as flight 2I (one mission, three runs, about 1 h 40 min) after A2 is built.

## Consequences

- Decision 0021's workaround table gets the result as a row; the exception is itself a workaround for the same game bug.
- Bug report 3 (`handoff/il2-bug-report-drafts.md`) gains the covering pair's weave as a second way the crossing happens.
