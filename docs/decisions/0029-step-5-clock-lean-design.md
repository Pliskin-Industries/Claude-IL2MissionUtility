# 0029. Step 5 clock: a leaner build of plan §3.7 on flown facts

- **Status:** Accepted (option B)
- **Date:** 2026-10-18
- **Decided by:** the user, 2026-10-18 ("Agree, execute"), on Claude's scoping of P14 step 5, with the step 6 check below

## Context

Plan §3.7 (the mission clock, front end A) was drawn on 2026-09-27, before flight 0. It hedges against game behaviour that was then unknown. Flight 0 has since settled it (`handoff/P14-probe-results.md`, `docs/il2-loader-and-log-facts.md`):

- **f:** a timer's `Random` % is rolled each time it is triggered (`RandomImpl::InMission` confirmed). The `Seeded` fallback is not needed.
- **J1:** deactivating a running timer cancels it. The D11 `X` → `X OUT` cancel pattern is not needed; step 4 already dropped it (decision 0023, point 2).
- **J3:** an Activate and a pulse into the same relay in one tick: the pulse is dropped. Every switch must come at least one tick before the pulse it enables.
- **J4:** a relay latch drops a same-tick double.

§3.7 still holds after steps 3 and 4 as built: the copy pointer (`ENii COPY r`, `ENii EXHAUSTED`) is owned by `assemble_side`; the clock registers each entry's `ENii HOT` / `ENii COLD` pair in `SideLogic.hot_consumers` / `cold_consumers`, and the threshold puts the HOT relays in `init_off`; the threshold's U24 timing (D = 60 s, H = 120 s) changes nothing in the clock. Only the internals of the clock can be made smaller. The user's rule (U13): the fewest MCUs and the cleanest design.

## Options

**A. §3.7 as written.** 12 MCUs per entry (11 for the last in the waterfall); per side, by Claude's count of the diagram, 38 plus one `AREA aa OUT` per area: `FIRST`, `TICK`, `PERIOD`, `QUIET ROLL`, a 13-MCU jitter randomizer, 3 `JITTER k DELAY`, and the slot and attempt machinery with its `OUT` relays. Built with both `RandomImpl` variants.

**B. The same behaviour, leaner (recommended).** Five changes, each resting on a flown fact:

1. **Quiet roll and jitter in J timers.** `TICK` → `CLK s SLOT ON` (Activate `SLOT OPEN`) and `CLK s JITTER k` (k = 1..J, `Time = 0.1 + (k−1)·step`, `Random` = p_k). Every `JITTER k` targets `CLK s SLOT OPEN`, a latch (`SLOT OPEN OFF` switches it off; `SLOT ON` re-arms it at the next `TICK`, 0.1 s before the first jitter timer, J3). The first jitter timer that passes opens the slot; later ones find the latch closed. p_k = `weighted_random_pct([100−q; J] ++ [q·J])[k]`: for q = 20 %, J = 3 that is 27 / 36 / 57 %, so each step opens 26.7 % of slots and 20.1 % stay quiet, as §3.7 intends. Replaces `QUIET ROLL`, the jitter randomizer and the `JITTER k DELAY` timers: **5 MCUs instead of 17** (J = 3).
2. **`ATTEMPT END` is cancelled directly** (J1): `CLK s WIN` deactivates `ATTEMPT END` and `REROLL`; `REROLL ON` (at `SLOT OPEN`) re-activates both. `ATTEMPT ARM` and `ATTEMPT END OUT` go (−2). Correct even if J1 failed: a late `ATTEMPT END` meets `REROLL` off.
3. **The entry randomizer has no `CloseInput` and no `Wait for Output`.** Attempts never overlap (only `DRAW` and `ATTEMPT END` start one, at least A apart), so nothing needs closing. `CLK s Randomizer:INPUT` → `ReOpen Outputs` (Activate every `ENii OUT`) and every `ENii RANDOM`; the outputs re-open 0.5 s before the first `RANDOM` can fire (−2). A = L + 0.5 s is kept.
4. **`DRAW` is the 0.3 s timer** (no `DRAW DELAY` → `DRAW` relay), and it pulses `LAST CLEAR` (a Deactivate) itself, so the 0.2 s `LAST CLEAR` timer goes (−2). The accepted exception of §3.7 ("a zone launch in the first 0.2 s of a slot has its `LAST` cleared") becomes 0.3 s.
5. **`ENii LAST SET` is one Activate** (`ENii LAST`), not two switches. `LAST CLEAR` still clears every `LAST` at each slot, so the next slot excludes **every entry started since the last slot opened** (clock or zone), not only the most recent one (−1 per entry). With a single front end the behaviour is the same as §3.7; it differs only when the zone starts two or more different entries between two slots.

And one change to the types: **remove `RandomImpl`** (and `Strategies.random`) from `mod.rs`, and build no `Seeded` variant, in the clock now or the zone in step 6. Claude's own commit before the step (§10). Nothing else uses it.

Counts for B: **11 MCUs per entry (10 for the last), 17 + J per side plus one `AREA aa OUT` per area**, so 20 + a with the default J = 3. The three-entry, three-area plan of the step 5 tests: A 76, B 55 (plus the three `ENii EXHAUSTED` that `assemble_side` already builds, in both).

**B′. B without item 5.** Keeps §3.7's "the most recent entry only" no-repeat: 12 per entry instead of 11.

## Recommendation

B. It is the same clock as §3.7 to a user of the mission, rests only on flown behaviour, and is about a quarter smaller. No probe flight is needed for it: every MCU idiom it uses is flown (relay, latch, Random timer, cancel by Deactivate); the clock as a whole is first flown in step 11.

## Required in either option (not a choice)

- **`ENii GO` lists its copy pointers last run first.** `GO` pulses every `ENii COPY r`; copy r's `START` activates `COPY r+1` in the same tick. In the game J3 drops that second pulse; in the walker's DepthFirst delivery an ascending list would start two copies. Last-first is safe under both.
- The waterfall order is shuffled from `AirPlan.seed` (D15), per side, with a small fixed generator in `clock.rs` (no new dependency); step 6's zone stagger uses the same order.

## Rejected

- **A as written.** 21 more MCUs on the example plan for no behaviour the game needs.
- **Folding the no-repeat into `AVAILABLE`** (the winner switches its own `AVAILABLE` off for a slot). Re-opening it would also re-open an exhausted entry; it needs a second relay anyway.
- **Dropping `ENii OUT`** (closers act on `RANDOM`). Then re-opening the waterfall each attempt would undo the exclusions.

## Consequences

- Plan §3.7 is rewritten as built when step 5 lands, as §3.5 was for step 4; the step 5 test names that pin §3.7's MCUs (`clock_structure_three_entries`, `clock_mcu_counts`, `clock_rearm_edges`, (xiv)) change to match.
- `validate_schedule` keeps its two checks; it reads a side's assembled entries rather than `(plan, ctx)`, so it uses `assemble_side`'s eligibility and nothing is computed twice.

## Decision (2026-10-18)

**B.** Build the lean clock; remove `RandomImpl`; build no `Seeded` variant.

The user asked whether item 5 matters where players fight, since the zone front end will start most sorties on a busy area. It can, so it is to be measured once the zone exists:

- **Step 6 adds a walker check** on one busy area (a player loitering for 3 h, the zone and the clock both on it): paired runs (same seed) of B and B′, 200 per scenario, for 2 and 4 entries on the area and budget 1 and 2. It reports, per 3-hour mission, the launches on the area, the slots lost (quiet while the budget had room and the area was HOT) and B′'s repeats (a clock launch of an entry the zone started since the last slot).
- **The bar:** if B and B′ differ by less than **half a launch per 3-hour mission** (about 5 points of the 9 slots) in every scenario, the choice is negligible and B stays. If any scenario crosses it, the user chooses between variety (B) and a busier sky (B′); switching is one MCU per entry in `clock.rs`.
- Expected, not verified: small on the NATO side (4 built-ins per area under Auto-fill), possibly noticeable on the DPRK side (2), because with the default budget of 2 a busy area is usually at its budget when a slot opens.

