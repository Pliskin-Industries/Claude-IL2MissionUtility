# 0020. Pairs formations still collide at Safe: what the app writes for Pairs

- **Status:** Accepted (option A)
- **Date:** proposed 2026-10-16; decided 2026-10-18
- **Decided by:** the user ("Decision A for 0020. Let's mark Loose at WPs as an alternative in the decisions log"), on Claude's proposal from flights 2E, 2F and 2G

## Context

Decision 0019 puts every AI air formation at `FormationDensity = 1` (Safe). Its evidence was F-80C Wedges only (flight 2D). Flight 2E (`handoff/P14-probe-results.md`, "Flight 2E") flew four built-ins exactly as the app now ships them, three runs, 20 aircraft:

| Built-in | Formation | Aircraft-seconds under 20 m (runs 1, 2, 3) | Closest | Losses |
|---|---|---|---|---|
| F-51 CloseSupport | Wedge, Safe | 0, 0, 0 | 54 m | none |
| Il-10 Kimpo attack | Wedge, Safe | 0, 0, 0 | 20 m | none |
| F-51 ArmedRecon | Pairs, Safe | 4, 0, 0 | 17 m | none |
| Yak airfield raid | Pairs, Safe | 6, 6, 12 | 3.3 m | 2 aircraft in run 1, all 4 in run 3 |

Wedge at Safe held for every type flown. Pairs at Safe did not: in every run the Yak raid's second pair passed within 9 m, 12–13 s after `WP 1`, when the lead reversed its turn and the wingman, on the inside, slid straight across its lead at the same height without avoiding it (Tacview; three contacts in two runs). Pairs at Safe holds the wingman only about 65 to 120 m away, close to Pairs at Dense in flight 2D. Two of the six built-ins fly Pairs (each as two two-ship flights), and Template Builder offers Pairs for any air group.

**Flight 2F (2026-10-17, `handoff/P14-probe-results.md`, "Flight 2F")** flew the Yak raid six ways, three runs, at most 8 aircraft at once (1.00× wall time), to test the user's reading that the collisions came from server overload:

| Yak raid flown as | Under 20 m, aircraft-seconds (runs 1, 2, 3) | Closest | Losses |
|---|---|---|---|
| Pairs, Dense | 4, 4, 4 | 13 m | none |
| Pairs, Safe (shipped) | 6, 24, 6 | 3.3 m | the Blue pair in run 3, at `WP 1` + 13 s |
| Pairs, Loose | 0, 0, 0 | 23 m | none |
| Wedge, Dense | 12, 2, 4 | 9.7 m | none |
| Wedge, Safe | 2, 0, 0 | 18 m | none |
| Wedge, Loose | 0, 0, 0 | 24 m | none |

The collision repeated on a server that kept up, at the same second after `WP 1` as in 2E, and every Tacview attitude jump in 2E and 2F starts at a contact: **the cause is the formation, not the load.** Only Loose kept every pass above 20 m. Pairs at Loose holds the wingman about 200 m out (Wedge at Loose about 1,200 m).

**Flight 2G (2026-10-17, `handoff/P14-probe-results.md`, "Flight 2G")** looked for a cheaper workaround that keeps Pairs at Safe, three runs, at most 8 aircraft (1.00×):

| Yak raid at Pairs Safe, plus | Closest (runs 1, 2, 3) | Aircraft lost of 12 | Cost |
|---|---|---|---|
| nothing (the control) | 4.2, 9.4, 2.9 m | 4 | — |
| AILevel 3 (Veteran) | 6.5, 6.0, 1.7 m | 3 | a value; changes every AI fight |
| AILevel 4 (Ace) | 6.2, 4.9, 7.4 m | 6 | a value; changes every AI fight |
| a Formation command to Loose, fired by `WP 1` | 26.9, 18.7, 19.1 m | 0 | 1 MCU per flight |
| AttackArea centre 3 km ahead of `WP 1` | 5.6, 26.1, 4.5 m | 4 | a placement change |

Skill changes nothing: Veteran and Ace wingmen cross at the same `WP 1` + 12–13 s. Moving the area only delays the lead's reversal (its left turn at `WP 1` is the attack entry), and the pair collides at + 23 s. Loose from `WP 1` works, but its clearance comes from the same Loose slot, a little closer than Loose throughout (2F: 23.0–31.8 m).

## Options

- **A. Pairs fly Loose (density 2); every other air formation stays Safe.** One value, chosen per formation type, in the same three places as 0019 (Air Tasking placement, the built-ins, Template Builder). Flight 2F flew it with the Yak raid: no pass under 20 m in three runs, the wingman about 200 m out. Not flown with the F-51 ArmedRecon.
- **B. The two Pairs built-ins switch to Wedge** (formation 20) at Safe, and Pairs stays available but Safe. Fixes what the app ships; a user who picks Pairs still gets the colliding setting. In flight 2F a two-ship Wedge at Safe came to 18 m in two of three runs, so this is marginal for pairs.
- **C. Template Builder and Air Tasking stop using Pairs** (write Wedge instead). Strongest; removes a formation the user may want for historical reasons (the sources fly elements in pairs).
- **D. Accept the losses for Pairs.** Not recommended: the Yak raid lost half its sorties in three runs.
- **E. Pairs fly Safe in transit and switch to Loose at `WP 1`** (one Formation command per flight, fired by the path waypoint; flight 2G's GX). No losses in three runs, closest 18.7–26.9 m. Keeps the shipped look in transit, where no Pairs collision has been recorded, but costs one MCU per flight and gave a little less clearance than A.

## Decision

**A. Pairs fly Loose (density 2); every other AI air formation stays Safe (decision 0019).** One value per formation type, written in the three places 0019 touches: Air Tasking placement, the built-ins, Template Builder. No MCUs. It is a workaround for the game bug of decision 0021 and is listed in its table.

**Recorded alternative: E, Loose at the waypoint.** Pairs fly Safe in transit and get one Formation command to Loose, fired by the path waypoint (`WP 1`), per flight. Flight 2G flew it: no losses, closest 18.7–26.9 m. Not taken now, because A gives as much clearance with no MCUs; E is kept for the case where a tighter pair in transit matters more than one MCU per flight (for example, if Loose transit looks wrong for escort or cover). B, C and D are not taken.

## Recommendation (before the decision)

Flight 2F (above) was that probe. Loose Pairs kept every pass above 20 m and Wedge at Safe did not quite, so **take A**: Pairs fly Loose, every other air formation stays Safe. It is one value per formation type in the three places 0019 already touches, and no MCUs. Flight 2G found nothing cheaper: AI skill and moving the attack area do not help, and E (Loose from `WP 1`) costs an MCU per flight for no better clearance. Optional before or after the ruling: fly the F-51 ArmedRecon at Loose, the other Pairs built-in.

## Consequences

- Until A is built, the app keeps writing Pairs at Safe, as 0019 says. Build A where 0019 is built (`SAFE_AIR_FORMATION_DENSITY` in `src/template.rs` and its uses), marking it as a 0021 workaround in the code.
- Pairs will look wider than period doctrine (the wingman about 200 m out, 17 m below) until the game is fixed.
- The game bug draft (`handoff/il2-bug-report-drafts.md`) now has report 3: wingmen cross over through their lead in turns, at any density. Decision 0021 treats this as a game bug and whatever 0020 chooses as a workaround to remove when it is fixed.

*Note added 2026-10-18 (the decision is unchanged): flight 2H flew the F-51 ArmedRecon at Loose. Loose did not help its covering pair, which weaves in transit and collided once at Loose. Whether A gets an exception for covering flights is decision 0022 (Proposed).*
