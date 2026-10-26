# 0033. The D30 check after the country-filtered threshold: warn within r_in

- **Status:** Accepted (option B)
- **Date:** 2026-10-26
- **Decided by:** the user, 2026-10-26 ("Accept 0032 B, 0033 B, 0034 B and 0035 A. Build step 7 as scoped."), on options and a recommendation by Claude, from the step 7 scope (`handoff/sessions/2026-10-26-p14-step7.md`).

## Context

Plan D30: `validate` warns when a sortie's spawn → target → RTB path passes **within R** (12 km) of an area that triggers the other side, and the warning becomes an **error** if check zones count AI (`strategies.ai_counts`). Flights 0 and 1A showed they do (risk a).

Decisions 0023 and 0024 changed what the other side's AI can do to an area:
- **The entry** is a Complex Trigger filtered to the players' country (radius r_in, 6 km). From step 7 the app's AI flies 602 (NATO) and 503 (DPRK) (0023 item 6), so its own AI never starts a cycle.
- **The presence re-check** is a coalition check zone at r_in, pulsed every H (120 s) while the area is HOT, open about 2 s. Enemy AI inside r_in at a re-check passes like a player (0024's stated weakness). So AI can keep an area HOT that a player made HOT, only while it is inside r_in.
- Nothing in the threshold works at R any more; R sets r_in (R × 0.5) and the step 12 window.

Separately, the shell's Zone IN and Zone Out (35 km, coalition) count enemy AI as "seen" at a target (0018). That is not D30 and stays as stated in the manual.

## Options

- **A. As planned:** path within R, a warning; an error when `ai_counts`.
- **B. Path within r_in, a warning; an error when `ai_counts`.** `ai_counts` stays false by default: after the country filter, AI cannot start a cycle, which is what the error case guarded against.
- **C. Drop D30.** The remaining effect (AI prolongs a player's HOT) is small and short.

## Recommendation

**B.** It warns about the one thing AI can still do (stand inside the zone the re-check uses), and only where it can happen. A warns about passes that cannot touch anything. C hides a known weakness.

## Consequences

- `validate_flags_d30_path_conflict` and `acceptance_test_plan()` place one NATO path within r_in of a DPRK area.
- Step 11's "AI and the other side" check reads: with the D30 warning showing, no sortie starts for that area without a player having entered it.
- The Map preview keeps both rings (R and r_in, plan §6.2); r_in is the one D30 measures.
