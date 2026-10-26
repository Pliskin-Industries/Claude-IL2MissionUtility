# Delegation prompt for P14 step 5 (mission clock, decision 0029 B)

One job, in the worktree `optimistic-hugle-8e0d98` on `codex/p14-5` (cut from `codex/p14-4`; head after this prompt is committed). Sent with `codex_start`, model `gpt-6-astra`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1500.

---

<task>
Build P14 step 5: the mission clock (front end A) in `src/airtask/clock.rs`, as decision 0029 option B (accepted by the user 2026-10-18) and the approved scope describe.

Read first, in this order:
1. `docs/decisions/0029-step-5-clock-lean-design.md` (the design and why), then `handoff/sessions/2026-10-18-p14-step5.md` (the approved scope: the diagram, names, MCU counts, API and the named tests). These two win over the plan where they differ.
2. `handoff/P14-air-tasking.md`: §3.0 (names, relay, latch, pair, make-before-break, start states, re-arm rule, `SideLogic`, build order), §3.3 (budget, `T_count`), §3.5 (threshold as built), §3.6, §3.7 (the original clock; 0029 B changes its inside, not its behaviour), §8 "Step 5" (the original test list).
3. `docs/il2-loader-and-log-facts.md`, section "Mission logic" (flown facts: a pulse into an inactive relay is dropped; deactivating a running timer cancels it). `handoff/P14-probe-results.md` rows J1, J3, J4, f.
4. `CLAUDE.md`, `.cursorrules`.
5. Code: `src/airtask/mod.rs` (`AirPlan`, `Schedule`, `PoolEntry`, `SideLogic`, `AirIssue`, `weighted_random_pct`; `RandomImpl` was removed in commit `f5bb22b`), `src/airtask/assemble.rs` (`SideCopies`, `EntryCopies`, `CopyIds`, `assemble_side`, `emit_side_logic`, and its tests' `context()`, `test_plan()`, `finish()`), `src/airtask/shell.rs` (`ShellIds`, `group`, `test_support`), `src/airtask/budget.rs` (the style to follow for a per-side builder), `src/airtask/threshold.rs` (`build_side`; its test harness and player tracks), `src/airtask/testkit.rs` (`walk`, `WalkerConfig`, `WalkerInput`, `assert_links_resolve`, `walker_every_deactivated_out_has_a_rearm_or_is_single_use`), `src/template.rs` (`mcu`, `timer`, `timer_random`).

What to build (names exact; `s` = `N` or `D`, `ii` = `EntryCopies.number` two digits, `aa` = area number two digits):

A. `pub(crate) fn build_side(side: AirSide, plan: &AirPlan, copies: &SideCopies, logic: &mut SideLogic, next_id: &mut i32) -> Result<Vec<Il2Entity>, String>`. Returns one group `Clock` (for the side's Logic tree, `copies.trees[0]`) holding every MCU below, or an empty Vec, registering nothing, when `plan.schedule[side.idx()].enabled` is false or the side has no `Clock` / `Both` entry. Clock-eligible entries = `copies.entries` with mode `Clock` or `Both` (assemble already dropped disabled, orphaned, date-invalid and wrong-side entries). Stage like `assemble_side`: on error leave `logic` and `next_id` untouched.
   - Per side: `CLK s FIRST` (timer `first_slot_min`·60) → `CLK s TICK` (relay). `TICK` → `CLK s PERIOD` (timer `period_min`·60, → `TICK`), `CLK s SLOT ON` (Activate: `SLOT OPEN`), and `CLK s JITTER k` for k = 1..J (`timer_random`, `Time` = 0.1 + (k−1)·`jitter_step_min`·60, `Random` = p_k), each → `CLK s SLOT OPEN` (relay, a latch). p_k = `weighted_random_pct(&w)[k−1]` with w = J copies of (100 − q) followed by q·J (q = `quiet_pct`); only the first J values are used.
   - `SLOT OPEN` → `CLK s SLOT OPEN OFF` (Deactivate: `SLOT OPEN`), every `ENii AVAILABLE`, `CLK s NO REPEAT` (timer 0.1), `CLK s REROLL ON` (Activate: `REROLL`, `ATTEMPT END`), `CLK s SLOT CLOSE` (timer = slot window), `CLK s DRAW` (timer 0.3).
   - `NO REPEAT` → every `ENii LAST`. `SLOT CLOSE` → `CLK s REROLL OFF` (Deactivate: `REROLL`). `DRAW` → `CLK s LAST CLEAR` (Deactivate: every `ENii LAST`), `CLK s Randomizer:INPUT`, `CLK s ATTEMPT END`.
   - `Randomizer:INPUT` (relay) → `CLK s ReOpen Outputs` (Activate: every `ENii OUT`), then every `ENii RANDOM` in waterfall order. `CLK s ATTEMPT END` (timer A) → `CLK s REROLL` (relay) → `INPUT`, `ATTEMPT END`. `CLK s WIN` (Deactivate: `ATTEMPT END`, `REROLL`). `CLK s AREA aa OUT` (Deactivate: every clock entry's `RANDOM` on area aa), one per area that has a clock entry.
   - Per clock entry: `ENii AVAILABLE` (relay) → `ENii RANDOM ON` (Activate: `RANDOM`); `ENii LAST` (relay) → `ENii EXCLUDE` (Deactivate: `RANDOM`); `ENii LAST SET` (Activate: `LAST`); `ENii RANDOM` (`timer_random`, `Time` 0.5·k for waterfall position k, `Random` from `weighted_random_pct` of the entry weights in waterfall order) → `ENii OUT` and, unless last, `ENii CLOSE` (Deactivate: the `OUT` of every later position); `ENii OUT` (relay) → `ENii HOT`, `ENii COLD`; `ENii HOT` (relay) → `ENii GO`; `ENii COLD` (relay) → `CLK s AREA aa OUT`; `ENii GO` (relay) → `CLK s WIN`, then the entry's `COPY r` pointers from the last run to the first (decision 0029: last first, so an Activate of `COPY r+1` by copy r's `START` cannot meet `GO`'s pulse in one tick under DepthFirst).
   - L = 0.5·(n+1), A = L + 0.5, slot window = min(300, (a + 10)·A); n = clock-eligible entries, a = distinct areas among them.
   - `SideLogic`: `hot_consumers[area]` += `ENii HOT`, `cold_consumers[area]` += `ENii COLD`; `init_off` += every `ENii LAST` and `CLK s REROLL` (the threshold adds the HOT relays itself); `mission_begin` += `CLK s FIRST`; `links` += (every copy's `shell.start`, `ENii LAST SET`) and (`entry.exhausted`, each of `ENii AVAILABLE`, `ENii RANDOM`, `ENii GO`). Do not edit any MCU the clock does not own.
   - Relays are `template::timer` with `Time` 0 and `Random` 100. Follow `budget.rs` for MCU construction, Targets, positions and grouping.
   - Errors (input validation, clear messages, DPRK / NATO in text): a weight outside 1..=10, `jitter_steps` = 0, `quiet_pct` > 100, a non-finite or non-positive `first_slot_min` or `period_min`, a non-finite or negative `jitter_step_min`, or an Error from `validate_schedule`.
B. `pub(crate) fn validate_schedule(side: AirSide, schedule: &Schedule, entries: &[EntryCopies]) -> Vec<AirIssue>`: over the clock-eligible entries, an Error when `period ≤ (J−1)·step + slot window + A + 10 s`, a Warning when `(a + 10)·A > 300 s`. Nothing when there is no clock-eligible entry or the schedule is disabled. Step 7's `validate` will call it per side.
C. `pub(crate) fn waterfall_order(seed: u64, side: AirSide, n: usize) -> Vec<usize>`: a permutation of 0..n from a small fixed generator written in `clock.rs` (SplitMix64 or similar; no new dependency, no hashing of std types), mixing the side so DPRK and NATO differ. The clock's waterfall position of clock-eligible entry i is its place in this order. Step 6 will reuse it.
D. `#[cfg(test)] fn clock_test_plan()` (and helpers) per the scope note: `runs = 6`, budget 6, a test-only 60 s timer from each copy's `START` to its `DONE HUB`. Build the full side as the game would: `assemble_side` → `clock::build_side` → `threshold::build_side` (with `copies.areas`) → `budget::build_side` → `emit_side_logic` (assemble's `finish` shows the last step). Make an area HOT with a player-country track loitering inside r_in (see the threshold tests for the country, coalition and timing). Use the flown walker settings where the test does not name others: `DeactivateRunningTimer::Cancels`; cover both target orders, and Batched as well as DepthFirst where the scope note says so.

Tests: every test named in the scope note's "Acceptance tests", with those exact names, including the new ones (`clock_mcu_counts_follow_jitter_and_areas`, `clock_jitter_odds_fold_the_quiet_roll`, `clock_waterfall_order_is_seeded_and_deterministic`, `clock_refuses_invalid_input`, `clock_init_off_and_mission_begin`, `clock_go_starts_one_copy_per_win`, `clock_one_slot_per_period_when_two_jitter_timers_pass`, `clock_late_attempt_end_after_win_does_not_reroll`) and the walker scenarios (i) to (xiv). Pin the counts as numbers: 11 + 11 + 10 per-entry MCUs and 23 per-side (20 + 3 areas) on the three-entry plan, 55 in all. Each walker scenario asserts on pulses of named MCUs (`START`, `SLOT OPEN`, `GO`, `WIN`), not only on totals. For the statistical scenarios (ix) and (xi) use 200 independent walker runs with distinct seeds; a fixture with a short `first_slot_min` is fine there. Do not build the busy-area B / B′ comparison: it is step 6's.

Acceptance:
- Baseline first: `cargo build --offline` and `cargo clippy --offline` warnings (expected 68 / 250), `cargo test --offline` (expected 745 passed / 0 failed / 29 ignored). After: no new build or clippy warnings, 0 failed, every named test present (list them), and state the new passed / ignored counts and the wall time the new tests add (keep it modest; report any single test over 20 s).
- `flown_probes_regenerate_byte_for_byte_except_flight2` passes with 52 files; nothing under `handoff/p14-probes/` or `src/testdata/` changes.
- No existing test changes. No existing builder's output changes.

Files in scope: `src/airtask/clock.rs` only. Do not touch `mod.rs`, `assemble.rs`, `shell.rs`, `budget.rs`, `threshold.rs`, `testkit.rs`, `probe.rs`, `zone.rs`, or anything outside `src/airtask/`. If the work needs a change outside `clock.rs` (for example a walker capability), stop and report exactly what and why instead of making it.
</task>

<git_protocol>
Same-host mode. The branch `codex/p14-5` is checked out in this worktree and its working tree is clean when you start. Edit files in the working tree only. No git command that writes. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, or the scope note and the code disagree, stop and say what and why. Do not weaken a test to make it pass.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run build, clippy, test and the byte check, and read the counts. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate; do not run MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now (`budget.rs`, `threshold.rs`). No refactor outside the change.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Where this prompt and the code disagree, follow the code and report each difference.
</missing_context_gating>

<structured_output_contract>
Return: 1. Summary (five lines). 2. Command results (baseline and final warnings and tests, byte check, added test time). 3. Files changed. 4. Tests added, by name. 5. MCU counts as built (per entry, per side, the three-entry plan). 6. Choices, differences from the scope note, and residual risks.
</structured_output_contract>
