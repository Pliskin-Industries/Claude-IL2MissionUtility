# 0044. A switched-off timer is paused: the zone and the clock end their timers through D11 relays

- **Status:** Accepted
- **Date:** 2026-10-29
- **Decided by:** the user. In the design session after flight A2: Q9 A (restore D11 in the zone), Q11 (no change for the clock's `RANDOM` timers), Q12 (no change in the threshold), Q13 (the walker), Q14 (the terms). In the fix session: Q10 "Drop the links" (Claude's recommendation), and "Add PASS IN gate" (Claude's recommendation, which revises Q9).
- **Supersedes:** in [0029](0029-step-5-clock-lean-design.md), point 2 ("`ATTEMPT END` is cancelled directly"). In [0030](0030-step-6-zone-lean-design.md), the parts of option C that rely on J1 cancelling a running timer: point 4 (`DISARM` "cancels the running `MISS WAIT`") and the exhaustion list `CHANCE, MISS WAIT, COOLDOWN`. The rest of 0029 and 0030 stands.

## Context

- 0029 and 0030 rested on flight 0's J1: "deactivating a running timer cancels it". J1 only saw a switched-off timer that was never switched on again.
- **Flight A2** (2026-10-29): each zone's `MISS WAIT` (1 s) fired 0.88 s after `ARM` switched it back on, 300 s after `DISARM` had switched it off mid-run. Each zone was shut for two cooldowns after every FIRE.
- **Probe flight 2K** (2026-10-29, `handoff/sessions/2026-10-29-p14-flight-2k.md`) settled the game's rules:
  - switching off a running timer **pauses** it, and switching it on **resumes** it with the time it had left (K1A, K1B, K1C);
  - a pulse restarts a resumed timer (K2A), and a pulse while it is off is dropped (K3A);
  - a failed random roll does not leave a running timer running (K4: none of 20 fired early);
  - a pulsed check zone stays armed, and switching an armed zone on is a check (K5).
- Nothing in IL-2 stops a running timer. The safe way to end one is the plan's D11 pattern: let it run out into a zero-second relay that is switched off, so the relay drops its pulse.

## Terms

These are the words used in the code, the tests, the plan and `USER_MANUAL.md` (Troubleshooting).

- **Switch off / switch on:** an `MCU_Deactivate` / `MCU_Activate` acting on an MCU.
- **Paused:** a running timer that was switched off. It keeps the time it had left.
- **Resumes:** a paused timer that is switched on runs out its time left.
- **Cancel** means the D11 relay: the timer runs out into a switched-off zero-second relay, which drops the pulse. Switching the timer itself off is not a cancel.
- **Armed** (check zones): a pulsed check zone keeps watching until it fires.

## Decision

### Zone (`src/airtask/zone.rs`): 13 MCUs per zone entry (was 11)

`CHANCE` and `MISS WAIT` are never switched. Two zero-second relays take their place in the switch lists:

- **`ZNii PASS IN`** takes the threshold's pass and feeds `CHANCE` and `MISS WAIT`. It is the area's pass subscriber. While the zone is disarmed it is off, so a pass during a cooldown starts nothing.
- **`ZNii MISS WAIT OUT`** sits between `MISS WAIT` and its targets (`COOLDOWN`, `DISARM`). After a won roll it is off, so the `MISS WAIT` still running drops out there.
- `DISARM`, `ARM` and `ENii EXHAUSTED` switch `PASS IN` and `MISS WAIT OUT` (exhaustion also `COOLDOWN`, which then stays paused for good).

```
 THsaa INNER (PASS) ─► ZNii PASS IN (relay) ─► ZNii CHANCE (timer 0.1 s + stagger, Random = chance %)
                                            └► ZNii MISS WAIT (timer 1 s + stagger) ─► ZNii MISS WAIT OUT (relay) ─► ZNii COOLDOWN, ZNii DISARM
 ZNii CHANCE ─► ZNii FIRE ARM, ZNii DISARM (Deactivate: PASS IN, MISS WAIT OUT), ZNii FIRE        (rest unchanged from 0030)
 ZNii COOLDOWN ─► ZNii ARM (Activate: PASS IN, MISS WAIT OUT)                                      ⟲ re-arm
 ENii EXHAUSTED ─► PASS IN, MISS WAIT OUT, COOLDOWN                                                (SideLogic.links)
```

### Clock (`src/airtask/clock.rs`): no MCU change

`CLK s WIN` and `CLK s REROLL ON` no longer switch `ATTEMPT END`. Nothing switches it. After a win (or `SLOT CLOSE`) it runs out into `CLK s REROLL`, which `WIN` (or `REROLL OFF`) has switched off. `REROLL` is the D11 relay here. `SLOT OPEN`'s `REROLL ON` switches it on again before `DRAW` restarts `ATTEMPT END`.

### Not changed

- **The clock's `ENii RANDOM` timers (Q11).** `AREA OUT` and `EXCLUDE` pause them mid-run. `RANDOM ON` resumes them at the next `SLOT OPEN`, and `DRAW` re-rolls them 0.3 s later, with at least 0.5 s left after an `AREA OUT`. A passing roll restarts the timer, and a failed one stops it (K4), so the remainder cannot fire out of turn. A walker test pins this (`clock_paused_random_cannot_fire_out_of_turn`).
- **`ZNii DENY WAIT`.** `GRANTED` pauses it. The next `FIRE ARM` resumes it, and `FIRE` restarts it 0.05 s later, long before the paused second runs out.
- **The threshold (Q12).** `PASS OFF` switches off `CLOSE DELAY` (0.05 s). `CLOSE OFF` switches `INNER` off as `CLOSE` starts `CLOSE DELAY`, so `CLOSE DELAY` can only be running at a pass that falls in the same tick as `CLOSE`, if the game applies switches at the end of a tick. It is then paused with 0.05 s left. Each later `CHECK ON` resumes it: while the plane stays, the check passes at the switch-on and `PASS OFF` pauses it again; once the plane has gone, it runs out 0.05 s after `CHECK ON`, and the area goes COLD about 2 s early. This one-tick edge is left as it is. `INNER PULSE` stays: a check zone that was never pulsed does not check when switched on (K5E), so the first check needs it. Later checks pass at the switch-on (K5A), which is why passes come exactly `HOLD` apart in the game.
- **The input check** `H > longest MISS WAIT + 1 s` stays: `PASS IN` is open for the same time after a pass as `CHANCE` and `MISS WAIT` were.

### Walker (`src/airtask/testkit.rs`)

`WalkerConfig::flown()` sets the two timer facts that 2K settled: `DeactivateRunningTimer::Pauses` and `FailedRoll::Stops`. The builder tests (shell, clock, zone, budget, threshold, assemble) run on it. The probe tests keep the settings they were written with. The check-zone model (pulse arms, switch-on of an armed zone is a check) is a separate commit.

## Rejected

- **Q9 as first decided: `MISS WAIT OUT` only (+1 MCU, 12 per entry).** `MISS WAIT` would then be on during the cooldown. The threshold passes every `HOLD` while a plane stays, and each pass would start `MISS WAIT`. A pass in the last `1 s + stagger` before `ARM` makes `MISS WAIT` run out after `ARM` has switched `MISS WAIT OUT` on: a miss nobody rolled, and a second cooldown. Passes come exactly `HOLD` apart, so when the cooldown is a whole multiple of `HOLD` (for example 10 min with `HOLD` 120 s), that pass and `ARM` fall in the same tick, and the outcome depends on the game's order within the tick. A walker test pins the gate (`zone_pass_in_the_last_second_of_a_cooldown_is_dropped`).
- **An extra `CLK s ATTEMPT END OUT` relay (+1 MCU per side).** Same behaviour as letting `ATTEMPT END` run out into `REROLL`, which is already off after a win.
- **Keep the J1 wiring.** Wrong in the game: A2's second cooldown.

## Consequences

- **Output changes.** For the same inputs, Generate writes a zone with 2 more MCUs per zone entry and new links, and a clock with two fewer links. The export test plan has 242 / 199 MCUs (was 240 / 199); the acceptance plan 422 / 678 (was 418 / 674).
- **Trace builds** trace `ZNii MISS WAIT OUT` instead of `ZNii MISS WAIT`: `MISS WAIT` now runs out after every pass, and only `MISS WAIT OUT` means a miss.
- **Frozen files** (A0, A1, A2 and the probes) are not regenerated. A2 is frozen as flown.
- **Probe flight 2L** (no pilot) checks the fix in the game: a zone rolls again after one cooldown, and two clock slots in a row.
- **An open edge in the clock (not changed, noted for the user).** `EXCLUDE` pauses a resumed `RANDOM` 0.1 s after `SLOT OPEN`. A `RANDOM` that `AREA OUT` paused, and whose entry the zone then starts again before each of the next slots, loses 0.1 s per slot. After two such slots its remainder is 0.3 s, and in the next slot where it is not excluded it runs out at `DRAW`'s tick or before it. This needs an `AREA OUT` pause and then zone starts of the same entry in consecutive periods. It is not modelled in a walker test.
