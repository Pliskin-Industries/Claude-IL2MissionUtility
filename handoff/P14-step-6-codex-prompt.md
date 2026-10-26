# Delegation prompt for P14 step 6 (zone front end, decision 0030 C)

One job, in the worktree `optimistic-hugle-8e0d98` on `codex/p14-6` (cut from `codex/p14-5`; head after this prompt is committed). Sent with `codex_start`, model `gpt-6-astra`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1500.

---

<task>
Build P14 step 6: the check-zone front end (front end B) in `src/airtask/zone.rs`, as decision 0030 option C (accepted by the user 2026-10-22) and the approved scope describe.

Read first, in this order:
1. `docs/decisions/0030-step-6-zone-lean-design.md` (the design and why), then `handoff/sessions/2026-10-22-p14-step6.md` (the approved scope: the diagram, names, MCU count, API, input checks and the named tests, including the busy-area check). These two win over the plan where they differ.
2. `docs/decisions/0029-step-5-clock-lean-design.md` (the clock as built, and the busy-area B / B′ check it hands to step 6).
3. `handoff/P14-air-tasking.md`: §3.0 (names, relay, latch, start states, re-arm rule, `SideLogic`, build order), §3.3 (budget, `T_count`), §3.5 (threshold as built: `THsaa INNER` is the PASS event), §3.7 (clock as built), §3.8 (the original zone; 0030 C changes its inside, not its behaviour), §8 "Step 6" (the original test list).
4. `docs/il2-loader-and-log-facts.md`, section "Mission logic" (flown facts: a pulse into an inactive relay is dropped; deactivating a running timer cancels it; re-triggering a running timer restarts it).
5. `CLAUDE.md`, `.cursorrules`.
6. Code: `src/airtask/mod.rs` (`AirPlan`, `ZoneSpec`, `ThresholdSpec`, `PoolEntry`, `TriggerMode`, `SideLogic`, `AirSide::label`), `src/airtask/assemble.rs` (`SideCopies`, `EntryCopies`, `CopyIds`, `assemble_side` including the zone spawn distance, `emit_side_logic`, and its tests' `context()`, `finish()`), `src/airtask/shell.rs` (`ShellIds`: `start`, `done_hub`; `group`; `test_support`), `src/airtask/budget.rs` (`recount_time`), `src/airtask/timing.rs` (`derived_timing`), `src/airtask/threshold.rs` (`build_side`; how it reads `pass_subscribers`), `src/airtask/clock.rs` (`build_side` is the style to follow; `waterfall_order`; its test harness, which builds a full side and scripts each copy's `DONE HUB`), `src/airtask/testkit.rs` (`walk`, `WalkerConfig`, `WalkerInput`, `assert_links_resolve`, `walker_every_deactivated_out_has_a_rearm_or_is_single_use`), `src/template.rs` (`mcu`, `timer`, `timer_random`).

What to build (names exact; `s` = `N` or `D`, `ii` = `EntryCopies.number`, two digits):

A. `pub(crate) fn build_side(side: AirSide, plan: &AirPlan, copies: &SideCopies, logic: &mut SideLogic, next_id: &mut i32) -> Result<Vec<Il2Entity>, String>`. Returns one group `Zone` (for the side's Logic tree, `copies.trees[0]`) holding every MCU below, or an empty Vec, registering nothing, when the side has no `Zone` or `Both` entry. Zone entries = `copies.entries` with mode `Zone` or `Both`, in that order. Stage like `clock::build_side`: on error leave `logic` and `next_id` untouched. Emit no subtitle, whatever `plan.debug_subtitles` says (0030, second question).
   - Per zone entry, 11 MCUs:
     - `ZNii CHANCE` (`timer_random`, `Time` 0.1 + stagger, `Random` = `plan.zone.chance_pct`) → `ZNii FIRE ARM`, `ZNii DISARM`, `ZNii FIRE`, in that order.
     - `ZNii MISS WAIT` (timer, `Time` 1.0 + stagger) → `ZNii DISARM`, `ZNii COOLDOWN`.
     - `ZNii DISARM` (Deactivate: `CHANCE`, `MISS WAIT`).
     - `ZNii FIRE ARM` (Activate: `WAITING`, `DENY WAIT`).
     - `ZNii FIRE` (timer 0.05) → `ZNii DENY WAIT`, then the entry's `COPY r` pointers (`CopyIds.pointer`) from the last run to the first (as `ENii GO`, decision 0029).
     - `ZNii DENY WAIT` (timer 1.0) → `ZNii WAITING`.
     - `ZNii GRANTED` (Deactivate: `DENY WAIT`).
     - `ZNii WAITING` (relay) → `ZNii WAITING OFF` (Deactivate: `WAITING`), `ZNii COOLDOWN`.
     - `ZNii COOLDOWN` (timer `plan.zone.cooldown_min`·60) → `ZNii ARM` (Activate: `CHANCE`, `MISS WAIT`).
   - Stagger: let n = the number of zone entries, order = `clock::waterfall_order(plan.seed, side, n)` applied to the zone entries; p = the entry's 0-based position among the zone entries on its area in that order; stagger = p·(T_count + 0.2) with T_count = `budget::recount_time(plan.strategies.census, c)` and c = the number of copies on the side (all entries, every mode).
   - `SideLogic`: `pass_subscribers[area]` += `CHANCE`, `MISS WAIT`; `init_off` += `WAITING`; nothing in `mission_begin`; `links` += (every copy's `shell.start`, `GRANTED`), (every copy's `shell.done_hub`, `WAITING`), and (`entry.exhausted`, each of `CHANCE`, `MISS WAIT`, `COOLDOWN`). Do not edit any MCU the zone does not own.
   - Relays are `template::timer` with `Time` 0 and `Random` 100. Follow `clock.rs` for MCU construction, Targets, positions and grouping.
   - Errors (input validation, plain messages starting "Air Tasking NATO zone: " or "Air Tasking DPRK zone: " via `AirSide::label`): `chance_pct` outside 1..=100; `cooldown_min` not finite or not positive; and the hold check of 0030 item 5: H = `timing::derived_timing(&plan.threshold).hold_s` must be greater than the longest `MISS WAIT` time on the side plus 1 s, else an error that names both numbers.
B. Tests in `zone.rs`, with a full-side harness: `assemble_side` → `clock::build_side` → `zone::build_side` → `threshold::build_side` (with `copies.areas`) → `budget::build_side` → `emit_side_logic`, plus a test-only timer from each copy's `START` to its `DONE HUB` (60 s unless a test says otherwise), as in the clock tests. Make an area HOT with a player-country track loitering inside r_in, as the clock tests do. Use the flown walker settings where a test does not name others: `DeactivateRunningTimer::Cancels`; DepthFirst, and Batched with both target orders, where the scope note says "both orders". You may reuse the clock's test helpers: in `clock.rs` you may only make `mod tests` and helpers inside it `pub(crate)`; nothing else in `clock.rs` changes.
C. The busy-area check (scope note, last block under "Acceptance tests"), for decision 0029's B / B′ question. B′ is built in the test only: for each entry on the area, add one Deactivate MCU listing every other entry's `ENii LAST`, linked from each of the entry's copies' `START`. Paired runs: seeds 0..200, the same seed for B and B′. Scenarios: 2 and 4 entries on one NATO area (all `Both`, the default zone, schedule and runs, a copy ends 20 min after its `START`), budget 1 and 2; a player loitering inside r_in from 1 s to 3 h. Count per run, over [0, 3 h]: launches on the area (`START` pulses); slots lost (a non-quiet slot, that is one whose `CLK N SLOT OPEN` fired, with no `START` between its `SLOT OPEN` and `SLOT OPEN` + slot window + A, while at its `SLOT OPEN` fewer than budget copies were flying (`SNjj FLYING` active) and some entry had copies left); B′ repeats (a `START` caused by the clock, that is in a slot window and after a `GO` of that entry, of an entry whose copy the zone started since the previous `SLOT OPEN`). `#[ignore]` test `zone_busy_area_no_repeat_b_against_b_prime` prints one row per scenario: mean launches B and B′, their difference, mean slots lost B and B′, mean B′ repeats, and whether |difference| < 0.5. It asserts only sanity (paired runs used the same seeds, the B′ transform added one MCU per entry). A non-ignored `zone_busy_area_check_counts` runs the same code for 2 paired runs of one scenario and asserts the counts are consistent (launches ≥ 1, no copy started twice, slots lost ≤ non-quiet slots). Run the ignored test once yourself and paste its table in your report, with its wall time.

Tests: every test named in the scope note's "Acceptance tests", with those exact names, including the new ones (`zone_fire_lists_copy_pointers_last_run_first`, `zone_side_without_zone_entries_emits_nothing`, `zone_refuses_invalid_input`, `zone_does_not_edit_other_builders`, `zone_links_resolve_and_rearm_rule`, `zone_second_pass_while_cooling_starts_nothing`, `zone_chance_rate`, `zone_fire_starts_one_copy`, `zone_busy_area_check_counts`, `zone_busy_area_no_repeat_b_against_b_prime`). Pin the count as a number: 11 MCUs per zone entry, no per-side MCU. The walker ticks every 0.05 s: where the scope note gives times finer than that (0.01 s), use the nearest tick that keeps the order of events, and say so in a comment. Each walker scenario asserts on pulses of named MCUs (`CHANCE`, `FIRE`, `DENY WAIT`, `COOLDOWN`, `ARM`, `START`), not only on totals. For `zone_chance_rate` use 200 independent walker runs with distinct seeds.

Acceptance:
- Baseline first: `cargo build --offline` and `cargo clippy --offline` warnings (expected 68 / 250), `cargo test --offline` (expected 778 passed / 0 failed / 29 ignored). After: no new build or clippy warnings, 0 failed, every named test present (list them), and state the new passed / ignored counts and the wall time the new non-ignored tests add (keep it modest; report any single test over 20 s).
- `flown_probes_regenerate_byte_for_byte_except_flight2` passes with 52 files; nothing under `handoff/p14-probes/` or `src/testdata/` changes.
- No existing test changes. No existing builder's output changes.

Files in scope: `src/airtask/zone.rs`; in `src/airtask/clock.rs` only the test-visibility change in B. Do not touch `mod.rs`, `assemble.rs`, `shell.rs`, `budget.rs`, `threshold.rs`, `timing.rs`, `testkit.rs`, `probe.rs`, or anything outside `src/airtask/`. If the work needs a change outside these (for example a walker capability), stop and report exactly what and why instead of making it.
</task>

<git_protocol>
Same-host mode. The branch `codex/p14-6` is checked out in this worktree and its working tree is clean when you start. Edit files in the working tree only. No git command that writes. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, or the scope note and the code disagree, stop and say what and why. Do not weaken a test to make it pass.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run build, clippy, test and the byte check, and read the counts. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate (`rustfmt src/airtask/zone.rs` is fine); do not run MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now (`clock.rs`, `budget.rs`, `threshold.rs`). No refactor outside the change.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Where this prompt and the code disagree, follow the code and report each difference.
</missing_context_gating>

<structured_output_contract>
Return: 1. Summary (five lines). 2. Command results (baseline and final warnings and tests, byte check, added test time). 3. Files changed. 4. Tests added, by name. 5. MCU counts as built. 6. The busy-area table and its wall time. 7. Choices, differences from the scope note, and residual risks.
</structured_output_contract>
