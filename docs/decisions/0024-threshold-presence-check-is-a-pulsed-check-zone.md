# 0024. The threshold's presence check is a pulsed check zone (ZonePulse), after flight 2J

- **Status:** Accepted
- **Date:** 2026-10-18
- **Decided by:** the user, in advance: decision 0023 (accepted 2026-10-18) says "If k1 fails, the default becomes `ZonePulse`". Flight 2J showed k1 fails. This record carries out that rule; it changes nothing else in 0023.

## Context

Decision 0023 built the threshold cell on one Complex Trigger per area, filtered to the players' country, and used switching the trigger on again as the presence check (`TriggerReactivate`). That needed k1: a trigger switched on with a filtered plane already inside reports Object Entered. Flight 2J (`handoff/P14-probe-results.md`, "Flight 2J"; logs `src/testdata/missionlog/real_43/`):

- KA: a 601 pair inside a switched-off trigger; switched on three times; no report (its control check zone fired).
- THD01 (`TriggerReactivate`): first check passed 0.02 s after the switch-on (most likely the wingman's entry, not yet reported when the trigger was switched off); the re-check 120 s later reported nothing although the pair was still there, and the area went COLD.
- THD03 (`ZonePulse`): passed every check and failed after its pair was deleted, as designed.

## Decision

`PresenceCheckImpl::ZonePulse` is the default. The cell keeps the Complex Trigger for the entry (it filters by country, so the app's AI cannot start a dwell) and checks presence with a check zone at r_in, pulsed 0.1 s after it is switched on (flight 1A's c). 19 MCUs, 1 Complex Trigger, 1 check zone per (side, area).

## Rejected

- **Keep `TriggerReactivate`.** Fails its re-checks in the game.
- **Trigger events only (Entered starts the dwell, Left cancels it).** Already rejected in 0023: wrong with more than one player.

## Consequences

- At a check, an enemy-coalition AI plane inside r_in passes like a player (0023's stated weakness of the fallback). The entry still needs a player-country plane, so the app's AI alone never starts a cycle; it can only keep an area HOT that a player made HOT, and only while it stays inside r_in.
- Whether to remove `TriggerReactivate` from the code is open: decision 0025 (Proposed).
