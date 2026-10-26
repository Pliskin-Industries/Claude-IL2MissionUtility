# 0023. Step 4 threshold: one Complex Trigger per area, re-activated as the presence check

- **Status:** Accepted
- **Date:** 2026-10-18
- **Decided by:** the user, 2026-10-18 ("build step 4 as proposed"), on Claude's scoping of P14 step 4

## Context

Plan §3.5 builds the threshold cell from two check zones: an entry zone at R, a dwell, then a pulsed inner zone at r_in as the presence check, re-checked every H. Two later rulings change that:

- **U24 (provisional):** one zone per area. Detect inside r_in, check again one inner-zone transit time later (§10 item 22).
- **0008:** the threshold must ignore the app's own AI. A check zone filters by coalition only; a Complex Trigger filters by country. Players fly 601 (NATO) and 501 (DPRK); the app's AI flies 602 and 503.

What the flights settled (`handoff/P14-probe-results.md`):
- Check zones (flight 1A): activate plus pulse with a player inside fires (c); a zone can be used twice; deactivating a waiting zone cancels it (h); one pulse fires once (o).
- Complex Trigger (flights 1N, 1D): a trigger filtered to the players' country ignores the AI country and fires Object Entered (58) and Object Left (60) once per aircraft, AI or player, on both sides.
- **Not flown:** whether a Complex Trigger that is switched on with a plane **already inside** reports Object Entered (call it k1), and whether it does so again after a switch off and on (k3). The 1N and 1D triggers were live from mission start (`probe.rs`, "Activate on Complex Triggers is unverified").

A Complex Trigger reports changes, not presence. The cell needs a presence check ("is a player inside now?"), so it needs k1.

## Decision

1. **One Complex Trigger per (side, area)**, `THsaa TRIGGER`: radius r_in, the flown 1N shape, `Country` = the other side's player country (`THNaa` watches 501, `THDaa` watches 601). Entry: its Object Entered closes the `ARM` latch, which switches the trigger off and starts `DWELL`. **Presence check: switch the trigger on again**; an Object Entered within the 2 s window is a pass. Re-check every H while HOT, as §3.5. **18 MCUs, 1 Complex Trigger, no check zone per (side, area)** (plan: 23 MCUs and 2 check zones). Diagram: `handoff/sessions/2026-10-18-p14-step4.md`.
2. **Two MCUs fewer than a straight port of §3.5**, both on flown results: the timeout timer is cancelled directly (deactivating a running timer cancels it, flight 0 J1), so it needs no `OUT` relay; and the 1 s `REARM` delay goes (a trigger takes no pulse, so there is no same-tick Activate-plus-pulse to avoid).
3. **Fallback built alongside, as a strategy** (`PresenceCheckImpl::ZonePulse`): the trigger only does the entry; the presence check is a pulsed coalition check zone at r_in, as flown in 1A. 19 MCUs, 1 Complex Trigger, 1 check zone. Its weakness: at a check, enemy AI inside r_in passes like a player.
4. **Timing under U24:** D = ceil to 10 s of the F-80 transit of r_in (60 s at R = 12 km), H = 2D (120 s). `timing::derived_timing` changes to match; the step 12 window W stays on R.
5. **Probe flight 2J (no pilot, flown by Claude) after the build, before step 4 merges:** k1, k3, a switched-off trigger stays silent, the country filter on switch-on, the switch-on delay (sizes the 2 s window), and the step 4 cell itself with a country-601 AI loiterer. If k1 fails, the default becomes `ZonePulse`.
6. **The app's AI country** (NATO copies to 602) is applied in step 7's export, not in `place_air_sortie`, so the flown probe files stay byte-identical. Step 4 provides the country constants.
7. **Unchanged in step 4:** the sortie shell's Zone IN and Zone Out stay coalition check zones (decision 0018), so enemy AI still counts as "seen" at a target.

## Rejected

- **§3.5 as written (two check zones).** Counts the app's AI (0008 rejected that), and 23 MCUs.
- **Two Complex Triggers (§3.5's shape, entry at R and inner at r_in).** 21 MCUs; U24 already chose one zone.
- **Trigger events only, never switched off** (Entered starts the dwell, Left cancels it). No probe needed, but it cannot count: with two players, one leaving cancels the other's dwell. Wrong on any server with more than one player.
- **Hybrid as the default** (option 3 above). Keeps AI out of the entry but lets AI pass every re-check, so an AI flight loitering at the target keeps the area HOT after the player has gone.
- **Check zones only, accept that AI counts** (0008 option 3), **or keep AI out of areas** (0008 option 1). Already rejected in 0008.

## Consequences

- Files beyond `threshold.rs`: `testkit.rs` (the walker learns the Complex Trigger and a country per track), `timing.rs` (U24 timing), and one small type commit in `mod.rs` by Claude: `PresenceCheckImpl { TriggerReactivate (default), ZonePulse }` replaces `{ PulseInside, Closer0Race }` (Closer0Race was ruled out by flight 0), and `ThresholdImpl`'s variants are renamed `OnePlayer` / `Counted`, since v1 now uses a Complex Trigger too.
- Known weaknesses: a straight pass through the centre slower than about 720 km/h passes (6 km radius, 60 s; the two-zone plan tolerated down to about 540 km/h), so propeller players flying straight through make an area HOT; HOT stays up to H (120 s) after the player leaves; a mission maker's own AI on 601 or 501 counts (0008).
- 602 still has no name in the game's country table; how it shows in the statistics screen is not looked at.
