# 0045. The walker models check zones as flown: a pulsed zone stays armed

- **Status:** Accepted
- **Date:** 2026-10-29
- **Decided by:** the user, in the design session after flight A2 (Q13: "Check zones (pulse arms and stays armed; switch-on of an armed zone is a check; switch-off keeps the watch): a second commit on this branch").
- **Supersedes:** in [0044](0044-paused-timers-d11-relays.md), the term "Armed: a pulsed check zone keeps watching until it fires". Flight 2K's K5D shows a zone stays armed after it fires. The rest of 0044 stands.

## Context

Probe flight 2K (K5, `handoff/sessions/2026-10-29-p14-flight-2k.md`) showed how IL-2 check zones behave:
- a pulse arms a zone, and it stays armed (K5B);
- switching an armed zone off keeps its watch, and switching it on is a check (K5A);
- this holds even if it fired before it was switched off (K5D);
- a zone that was never pulsed does nothing when switched on (K5E);
- a first pulse with the condition true fires at once (K5C);
- a further pulse into an armed zone whose condition still holds does nothing (K5A).

The walker's `Once` model clears the watch when the zone fires and when it is switched off, and never checks at a switch-on. Under it the threshold's presence check passed 0.1 s after each check (`INNER PULSE`), at 120.1 s intervals. In the game it passes at `CHECK ON`, exactly `HOLD` apart (A2: 160.00 s).

## Decision

- **`CheckZoneMode::Armed`** in `src/airtask/testkit.rs`, set by `WalkerConfig::flown()`, so every builder test uses it. In this model:
  - a pulse arms a zone; a pulse into an armed zone does nothing;
  - an armed zone fires when its condition becomes true, and stays armed;
  - switch-off keeps the watch, and switching an armed zone on is a check.
- **Not flown, and modelled this way:** an armed zone fires again when a plane leaves and comes back, because its condition becomes true again.
- `Once` and `Repeat` stay as hypotheses. The probe tests keep the settings they were written with.
- The walk loop delivers what a zone fires at a batched switch-on in the same tick.
- The test `walker_armed_zones_follow_flight_2k` replays K5A to K5E.

## Rejected

- **Change `Once` itself.** The probe tests were written against it, and their frozen files must keep their meaning.
- **Leave check zones as they are.** The builder tests would then keep `HOLD + 0.1` timings that the game does not have.

## Consequences

- **Builder test times:** threshold passes after the first come exactly `HOLD` apart. A re-entry passes at entry + `DWELL`, not + 0.1 s. Zone tests that count passes move with them.
- **The Watchdog Zone Out variant** (`ZoneOutImpl::Watchdog`, unused, kept for step 12) cannot work. Its 5 s poll re-pulses an armed zone, which the game ignores. Under `Armed` it ends the copy about 10 s after its first watch. `shell_zone_out_watchdog_structure` pins this, and plan §3.9 (risk b) says it needs a redesign before step 12.
- **Plan risks** h, j and o are marked as answered by flight 2K.
