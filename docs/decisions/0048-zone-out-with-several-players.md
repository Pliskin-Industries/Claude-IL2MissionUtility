# 0048. Zone Out with several players

- **Status:** Superseded by [0050](0050-ai-on-sides-of-its-own.md) (2026-10-31). Option D there ends each copy on its area's player count. Generate builds no Zone Out, so the question here is moot; the Watchdog variant is removed. The flown probe flights keep their Zone Out shell (the `CheckZone` strategy) only to build what was flown.
- **Date:** 2026-10-31
- **Decided by:** open

## Context

- Each sortie copy ends early through its template's Zone Out: a Closer 0 check zone of 35 km on the target, armed only after Zone IN has seen an enemy (decision 0018). With one watched plane it fires when that plane leaves (flights 2B, A3) or dies (A1: 3.06 s after the kill).
- **Risk b:** with two or more watched planes, does it fire when ALL have left, or when ANY one leaves? No flight has had two watched planes leave at different times. Plan §3.9 put this in step 12 (cell T-b).
- Flight 2M answers it with AI stand-ins, no pilot (decision 0046), in cells B1 to B4: one of two watched planes is deleted, killed, flies out, or is a pair's wingman; the other leaves later.
- The fallback built in step 3, `ZoneOutImpl::Watchdog`, cannot work. It pulses the zone every 5 s to refresh a 10 s timer, but a pulse into an armed zone whose condition still holds does nothing (flight 2K, K5A; decision 0045).
- Switching an armed zone off and on **is** a check, even after it has fired (2K, K5D; the threshold's presence check runs on this).

## Options

**If flight 2M shows ALL-out** (Zone Out fires only when the last watched plane has gone):

- **Keep `ZoneOutImpl::CheckZone`.** Remove the `Watchdog` variant and its test in a mechanical commit; no output changes.

**If flight 2M shows ANY-out:**

1. **Watchdog v2.** Zone Out becomes a Closer 1 zone, armed by the existing pulse. A 5 s loop switches it off and, one tick later, on; each switch-on is a check. Each pass restarts a 10 s timer (J5), and that timer runs out into `DONE HUB`. It keeps the 35 km radius and ends a copy about 10 s after the last watched plane has gone. `DONE HUB OFF` pauses the loop and the timer for good (0038, 0044). Cost: about 5 MCUs per copy, and a loop that runs while each copy is on station.
2. **End on the area's COLD.** After on station starts, the area's next COLD (the threshold's FAIL) ends the copy. Cost: about 1 MCU per copy. But it watches r_in (6 km), not 35 km, and it can be up to `HOLD` + 2 s (about 2 min) late.
3. **Accept ANY-out.** No change; the manual says the first player to leave ends the sorties on that area. The area stays HOT while others remain, so the clock and the zone front end can launch again.

## Flight 2M result (2026-10-31)

**ALL-out.** In all four cells B the Closer 0 zone fired only when the last watched group was gone (480.02 s, and 700.02 s in MB3). It did not fire when the first group was deleted, killed, flown out, or was a pair's wingman deleted alone. Evidence: `handoff/P14-probe-results.md`, "Flight 2M"; logs `real_54`. AI on 601 stood in for players.

## Decision

Not made yet. **Claude recommends: keep `ZoneOutImpl::CheckZone`.** Remove the unused `Watchdog` variant and its test in a mechanical commit, with no output change. Flight 2N checks ALL-out with two players. The user decides.

## Consequences

- Step 12d does what this record decides. If an option changes the shell, its walker tests come first: the structure, a copy that stays while one watched plane remains, a copy that ends after the last one leaves, and a copy that stays quiet after `DONE HUB`.
- Flight 2N (two humans) checks the result with players.
