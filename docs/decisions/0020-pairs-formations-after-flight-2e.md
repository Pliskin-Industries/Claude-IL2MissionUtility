# 0020. Pairs formations still collide at Safe: what the app writes for Pairs

- **Status:** Proposed
- **Date:** 2026-10-16
- **Decided by:** open, for the user; proposed by Claude from flight 2E

## Context

Decision 0019 puts every AI air formation at `FormationDensity = 1` (Safe). Its evidence was F-80C Wedges only (flight 2D). Flight 2E (`handoff/P14-probe-results.md`, "Flight 2E") flew four built-ins exactly as the app now ships them, three runs, 20 aircraft:

| Built-in | Formation | Aircraft-seconds under 20 m (runs 1, 2, 3) | Closest | Losses |
|---|---|---|---|---|
| F-51 CloseSupport | Wedge, Safe | 0, 0, 0 | 49 m | none |
| Il-10 Kimpo attack | Wedge, Safe | 0, 0, 0 | 23 m | none |
| F-51 ArmedRecon | Pairs, Safe | 2, 4, 2 | 13 m | none |
| Yak airfield raid | Pairs, Safe | 8, 10, 8 | 3.0 m | 2 aircraft in run 1, all 4 in run 3 |

Wedge at Safe held for every type flown. Pairs at Safe did not: in every run the Yak raid's second pair passed within 8 m, 12–13 s after `WP 1`, as it turned onto the attack. Two of the six built-ins fly Pairs (each as two two-ship flights), and Template Builder offers Pairs for any air group.

## Options

- **A. Pairs fly Loose (density 2); every other air formation stays Safe.** One value, chosen per formation type, in the same three places as 0019 (Air Tasking placement, the built-ins, Template Builder). Loose has not been flown with Pairs; with the F-80C Wedge it spread the flight to about 500 m (flight 2D).
- **B. The two Pairs built-ins switch to Wedge** (formation 20) at Safe, and Pairs stays available but Safe. Fixes what the app ships; a user who picks Pairs still gets the colliding setting.
- **C. Template Builder and Air Tasking stop using Pairs** (write Wedge instead). Strongest; removes a formation the user may want for historical reasons (the sources fly elements in pairs).
- **D. Accept the losses for Pairs.** Not recommended: the Yak raid lost half its sorties in three runs.

## Recommendation

Fly one more probe before ruling: the Yak raid as shipped, with its Pairs at Loose (2), against the same raid with Wedge at Safe, 3 runs. If Loose Pairs keeps every pass above 20 m, take A (the smallest change, no MCUs); if not, take B or C.

## Consequences

- Until this is decided, the app keeps writing Pairs at Safe, as 0019 says.
- The game bug draft (`handoff/il2-bug-report-drafts.md`) covers Dense only; Pairs at Safe is the same fault and could be added.
