# 0017. Step 3 settles on Activate-mode sorties, its own Delete and event 13

- **Status:** Accepted
- **Date:** 2026-10-14
- **Decided by:** the user, on Claude's scoping of the held step 3 (`codex/p14-3`, commit `3224286`)

## Context

Step 3 (sortie shell, budget, assembly) was built before flight 0 and held. Merged onto `claude/p14-air-tasking` it does not compile (13 errors, trial merge 2026-10-14):

- 9 errors: it hooks the budget release (`LANDED`, `BUD s MARK`, `BUD s CHECK`) onto the template's own delete timer. Decision 0014 made that chain optional: an Activate-mode group no longer writes `DELETE DELAY` or `Trigger Delete`, and the six built-in sorties are Activate mode. Without a Delete an Activate-mode copy would never leave the budget.
- 3 errors: it counts losses on event 4 (`LossesImpl::Type4`). Decision 0011 counts on 13.
- 1 error: the Probe 2 writer uses a locale constant that commit `199bca8` removed.

What the flights showed that bears on it:
- A Delete that lists the aircraft's **entities** removed them every time it was flown (flights 1H, 1J, 1L, and 1M with a 4-ship placed by `place_air_sortie`). A dead aircraft in the list is harmless (1J).
- Event 13 fired exactly once for every kind of loss flown: a Damage command, AI gunfire, a collision, a crash in a fight, and running out of fuel with a bail-out (flights 1G, 1J, 1K, 1L). Event 4 never fired. The one case not flown is a crash with no damage at all.
- Whether kill events, Deactivate and Delete act on aircraft made by an `MCU_Spawner` is not settled for Air Tasking (risk u). The user's experience is that Spawn mode limits formations and orders.

## Decision

1. **v1 accepts Activate-mode templates only.** `inspect_sortie` refuses a Spawn-mode template with the message "Air Tasking needs an Activate-mode template". Shapes S5 and S6 are dropped, with the repeat-mode sweep and the Spawn-mode sortie in Probe 2.
2. **The shell always writes its own clean-up Delete.** `SNjj DELETE DELAY` (0.5 s), started by the template's `DELAYED END ORDERS`, targets `SNjj DELETE` (an `MCU_Delete` listing every plane's entity), `SNjj LANDED`, `BUD s MARK` and `BUD s CHECK`. A template's own Delete chain, if it has one, is removed from the copy.
3. **Losses count on event 13 only.** One `OnEvent` 13 per plane into `SNjj LOSSES`. `LossesImpl::Latch` is removed: it guarded against a death event firing more than once, which event 13 has never done.
4. **Probe 2 gets cell T-l:** an aircraft lost with no damage (a crash), to check that event 13 fires once and `LOSSES` counts it.
5. Deferred, as recommended: sending a damaged aircraft home on events 3 or 1 (decision 0015) waits until after Probe 2; placing the AI on its own country (decision 0008) is step 4.
6. GPT-6 Astra builds it on `codex/p14-3b`; Claude reviews, re-runs the checks and merges.

## Rejected

- **Reuse the template's Delete when it has one (A2).** Same MCUs in the game, but two code paths, and it depends on placement correcting old files whose Delete lists Plane indexes (decision 0013).
- **Keep Spawn mode in v1.** Needs risk u flown first, the repeat-mode sweep, and two more shapes, for an advantage (respawning one group) that one-shot copies do not use.
- **Keep `LossesImpl::Latch`.** As written it hooks events 4, 2 and 0: 4 never fires, 2 comes late and not always, 0 is not a loss on its own.

## Consequences

- A wrapped copy gets 2 more MCUs than the plan's 17 (the Delete chain), less the template's chain when it had one. Plan §3.2 and the step 3 tests change to match.
- Each Activate-mode copy sits in the mission, switched off, from load until it is used. The Probe 2 load file (24 copies per side) measures what that costs.
- A crash with no damage that raises no event 13 would end its sortie at Zone Out or the hard stop instead; nothing locks.
