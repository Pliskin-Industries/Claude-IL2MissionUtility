# 0030. Step 6 zone front end: a leaner build of plan §3.8 on flown facts

- **Status:** Accepted (option C)
- **Date:** 2026-10-22
- **Decided by:** the user, 2026-10-22 ("Accept decision 0030: option C, and no debug subtitles in step 6 (step 7 decides). Build step 6 as scoped."), on Claude's scoping of P14 step 6

## Context

Plan §3.8 (front end B, the check-zone trigger per `Zone` or `Both` entry) was drawn on 2026-09-27, before flight 0. It hedges against game behaviour that was then unknown, as §3.7 did. Flight 0 has since settled it (`docs/il2-loader-and-log-facts.md`, "Mission logic"):

- **J1:** deactivating a running timer cancels it. Every cancel in §3.8 goes through a `X` → `X OUT` relay (D11) only because J1 was unknown.
- **J3:** an Activate and a pulse into the same MCU in one tick: the pulse is dropped. A switch must come at least one tick before the pulse it enables.
- A pulse into an inactive timer is dropped, so an inactive timer is a closed gate.

§3.8 still holds at its boundaries after steps 3 to 5 as built (details in `handoff/sessions/2026-10-22-p14-step6.md`): the copy pointer and `ENii EXHAUSTED` are `assemble_side`'s; `Zone` and `Both` copies are already placed at the zone distance; the threshold's PASS is `THsaa INNER`, which fires at the first pass and again every H (120 s) while the area stays HOT, and pulses whatever is in `pass_subscribers`. Only the inside of the zone cell can be made smaller. The user's rule (U13): the fewest MCUs and the cleanest design.

## Options

**A. §3.8 as written.** 17 MCUs per zone entry: `ARMED`, `ARMED ARM`, `DISARM`, `CHANCE`, `MISS WAIT`, `MISS WAIT OUT`, `FIRE ARM`, `FIRE OFF`, `FIRE`, `DENY WAIT`, `DENY WAIT OUT`, `GRANTED`, `WAITING`, `WAITING OFF`, `COOLDOWN`, `REARM`, `ARM`.

**B. The same cell with direct cancels (J1).** 13 MCUs.
1. `FIRE OFF` deactivates `MISS WAIT` itself, and `GRANTED` deactivates `DENY WAIT` itself; `MISS WAIT OUT` and `DENY WAIT OUT` go (−2).
2. `ARMED ARM` goes: `ARM` (at the end of the cooldown) re-activates `ARMED` and `MISS WAIT` together (−1). `FIRE ARM` re-activates `WAITING` and `DENY WAIT`, still at the chance roll and 0.05 s before the request, so the second review's R4-2 holds.
3. `REARM` goes: `ENii EXHAUSTED` deactivates `COOLDOWN` itself. A running cooldown is cancelled (J1) and a later pulse into it is dropped, so an exhausted zone never re-arms (−1).

**C. B without the `ARMED` relay (recommended).** 11 MCUs.
4. `THsaa INNER` pulses `ZNii CHANCE` and `ZNii MISS WAIT` directly; the zone is armed while both are on. One `ZNii DISARM` (Deactivate: `CHANCE`, `MISS WAIT`) is pulsed by the outcome: by `CHANCE` when the roll succeeds (this also cancels the running `MISS WAIT`, the job of `FIRE OFF`), and by `MISS WAIT` when it fails. `ARMED` and `FIRE OFF` go (−2).
5. The cost: the cell stays open for up to 1 s plus the stagger after a PASS, not 0 s. A second PASS inside that time would start a second roll. PASS events are at least H apart (H = 120 s by default), so `zone::build_side` refuses a plan whose H is not longer than the longest `MISS WAIT` plus 1 s, with a plain message. With the defaults the margin is about 115 s.

```
 THsaa INNER (PASS) ─► ZNii CHANCE (timer 0.1 s + stagger, Random = chance %)
                    └► ZNii MISS WAIT (timer 1 s + stagger) ─► ZNii COOLDOWN, ZNii DISARM        (the roll failed)
 ZNii CHANCE ─► ZNii FIRE ARM (Activate: WAITING, DENY WAIT)                                     ⟲ re-arm, first
             ├► ZNii DISARM (Deactivate: CHANCE, MISS WAIT)                                       (cancels MISS WAIT, J1)
             └► ZNii FIRE (timer 0.05 s) ─► every ENii COPY r, last run first ─► SNjj START
                                         └► ZNii DENY WAIT (timer 1 s) ─► ZNii WAITING            (refused: U8)
 SNjj START (every copy of entry ii) ─► ZNii GRANTED (Deactivate: DENY WAIT)                       (SideLogic.links)
 SNjj DONE HUB (every copy of entry ii) ─► ZNii WAITING                                            (SideLogic.links)
 ZNii WAITING (relay, starts OFF) ─► ZNii WAITING OFF (Deactivate: WAITING), ZNii COOLDOWN
 ZNii COOLDOWN (timer C) ─► ZNii ARM (Activate: CHANCE, MISS WAIT)                                 ⟲ re-arm
 ENii EXHAUSTED ─► CHANCE, MISS WAIT, COOLDOWN                                                     (SideLogic.links)
```

| | A (§3.8) | B | **C** |
|---|---|---|---|
| MCUs per zone entry | 17 | 13 | **11** |
| `pass_subscribers` per entry | 1 (`ARMED`) | 1 | 2 (`CHANCE`, `MISS WAIT`) |
| Relies on J1 | no | yes | yes |
| New input check | none | none | H > 1 s + longest stagger + 1 s |

Behaviour is the same in all three: one roll per arming, a cooldown after a miss, a refusal or the zone copy's `DONE HUB`, and no re-arming once the entry's copies are used up. Plus the shared `ENii EXHAUSTED` that `assemble_side` already counts.

## Recommendation

**C.** It rests only on flown behaviour (J1, J3, a pulse into an inactive timer is dropped), behaves like §3.8 to a user of the mission, and is about a third smaller. No probe flight is needed: every idiom in it is flown (Random timer, cancel by Deactivate, latch, Activate a tick before the pulse). The zone as a whole is first flown in step 11.

## Required in every option (not a choice)

- **`ZNii FIRE` lists the copy pointers last run first**, as `ENii GO` does (0029): copy r's `START` activates `COPY r+1` in the same tick.
- **Stagger:** entry p on its area (0-based, in the seeded order from `clock::waterfall_order` over the side's zone entries) adds p·(T_count + 0.2) s to `CHANCE` and `MISS WAIT`, with T_count = `budget::recount_time(census, copies on the side)`.
- **No `Seeded` variant** (0029 removed it): the §8 test `zone_seeded_chance_sequence_structure` is dropped.

## A second question: debug subtitles

§8 step 6 asks for `zone_debug_subtitles_only_when_enabled`. No Air Tasking builder emits a subtitle yet (the clock, step 5, does not), a subtitle needs a locale string that `MapAirPack` has no way to carry, and step 11 reads its checks from the trace build's replay report first (flight 0, t1: every Mission Objective breadcrumb logs). **Proposed:** step 6 emits no subtitles; step 7, which owns the export and the acceptance mission, decides whether that mission still needs debug subtitles from the clock and the zone and adds them for both at once.

## Rejected

- **A as written.** Six more MCUs per zone entry for no behaviour the game needs.
- **Roll on every PASS, no cooldown after a miss** (drops `MISS WAIT`). Then the chance is per check (every 2 min), not per arming, and a 50 % zone fires within a few minutes almost every time. That changes D18.
- **Arm `WAITING` and `DENY WAIT` at the PASS** (drops `FIRE`). `FIRE ARM` would then be pulsed while the zone cools down, because an Activate MCU is not gated by the zone's state.

## Consequences

- Plan §3.8 is rewritten as built when step 6 lands, as §3.5 and §3.7 were. The §8 step 6 test names that pin §3.8's MCUs change to match (`zone_structure`, `zone_has_17_mcus_per_entry`, `zone_last_copy_deactivates_rearm_armed_and_fire`).
- The comment on `SideLogic.pass_subscribers` in `mod.rs` changes from "`ZNii ARMED` relays" to "zone timers" (Claude's own commit before the step).

## Decision (2026-10-22)

**C.** Build the 11-MCU zone cell, with the hold check of item 5. Step 6 emits no debug subtitles; step 7 decides whether the acceptance mission needs them from the clock and the zone.
