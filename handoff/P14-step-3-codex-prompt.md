<task>
Build P14 step 3 (sortie shell, budget and assembly) in this repository (Rust 2024 binary crate, egui, nom). Steps 1a, 1, 1b, 0T, 0b and 2 are already on this branch.

This step is built **before** the flight 0 results (gate G0), on a held branch, at the user's request. Build every strategy variant the step names. Where a default depends on a probe result, use the plan's primary ("If confirmed") choice and mark the line with `// P14 G0: default pending flight 0`. Do not guess any probe outcome beyond that.

Read these first, in this order:
1. `handoff/P14-air-tasking.md`: §8 "General rules for every step"; §8 "Step 3. Sortie shell, budget and assembly" (the whole section: structural, budget-structure and walker-scenario acceptance, and the Probe 2 writers); §3.0 "Conventions"; §3.2 (the shell), §3.3 (the budget, head count, census, the documented overshoot), §3.4 (the extension), §3.7 (the per-entry copy pointer, randomness); §4 (timing: T_attack, T_final, the §4.3 arrival time); §5 "Data model" (`SideLogic`, `SideCopies`, the strategy enums `ZoneOutImpl`, `LossesImpl`, `BudgetImpl`, `CensusImpl`, the "Builders" list); §8 step 0 "0d" result table (which result sets which default); §9 risks e, g, j, p, q, r, u, v; §10 items 16, 22 (U24), 24, and decisions U13, U17, U22, D28, D36, D38, R4-4, T1, T5, T10, C1.
2. `CLAUDE.md` and `.cursorrules`.
3. `src/airtask/mod.rs` (frozen types), the three stubs `src/airtask/shell.rs`, `budget.rs`, `assemble.rs`, `src/airtask/library.rs` and `place.rs` (step 2: `inspect_sortie`, `SortieShape`, `place_air_sortie`), `src/airtask/testkit.rs` (the walker `walk`, `WalkerConfig`, `WalkerInput`, `assert_links_resolve`, `walker_every_deactivated_out_has_a_rearm_or_is_single_use`), `src/airtask/timing.rs`, `src/airtask/probe.rs` (the subtitle and writer helpers to reuse for Probe 2), `src/trace.rs` (`instrument` for the traced Probe 2 file), `src/template.rs` (`generate_template` and the MCU builders), and the six built-ins under `TemplateExamples/Historical1950/`.

What to build (files in scope only):
- `src/airtask/shell.rs`: the per-copy shell (strip Zone IN and the start machinery, `START`, `DONE HUB` and its inputs, `ON STATION IN` / `ON STATION`, `HOT` / `COLD` / `EXTEND`, `HARD STOP`, `LOSSES`, `STARTED ON/OFF`, `FLYING`, `LANDED`, the delete-timer registration), the Spawn-mode and repeat-mode handling, the orphan sweep, the `ZoneOutImpl` and `LossesImpl` variants.
- `src/airtask/budget.rs`: `build_side` with the 16 MCUs of §3.3 (14 at N = 1), the `BudgetImpl` and `CensusImpl` variants.
- `src/airtask/assemble.rs`: `assemble_side`, the copy pointer, `ENii EXHAUSTED`, `emit_side_logic` and the application of `SideLogic.links`.
- The ignored writers `write_p14_sortie_probe` (in `shell.rs`, under `#[cfg(test)]`) and `write_p14_load_probe`, as the step describes, into `target/p14/` (traced file plus `.trace.json` and `.eng`, as the 0b writers do).

Frozen interfaces: the types, signatures and `pub use` lines of `src/airtask/mod.rs` stay as they are. If the step cannot be built without changing one of them, or without changing a file outside scope (including `library.rs`, `place.rs`, `testkit.rs` or `probe.rs`), stop and report exactly what and why instead. Fable makes `mod.rs` changes as separate commits.

Acceptance. Each line is a named test or a command result:
- Every test the step names, each defined exactly as there (46):
  - structural: `shell_strips_zone_in_and_start`, `shell_end_timer_only_from_done_hub`, `shell_done_hub_inputs`, `shell_delete_timer_releases_the_budget`, `shell_one_on_station_per_sortie`, `shell_on_station_with_the_escort_first`, `shell_wp1_arms_zone_out`, `shell_losses_counts_every_plane`, `shell_end_of_station_pair`, `shell_timing_values`, `shell_start_is_the_gate`, `shell_adds_17_mcus_per_copy`, `shell_keeps_template_rtb_timer`, `shell_delete_timer_still_targets_delete`, `shell_wraps_a_spawn_mode_template`, `shell_sweeps_repeat_mode_machinery`, `shell_orphan_sweep_keeps_everything_reachable`, `shell_refuses_a_second_link_into_the_end_timer`, `shell_zone_out_watchdog_structure`, `shell_losses_latch_counts_each_plane_once_across_types_4_2_0`, `assemble_copy_pointer_structure`, `assemble_emit_side_logic`, `assemble_applies_links`, `assemble_exhausted_is_one_deactivate_per_entry`;
  - budget structure: `budget_structure`, `budget_mcu_count_does_not_depend_on_n`, `budget_n1_has_no_counter`, `budget_single_slot_structure`, `budget_census_same_tick_structure`;
  - walker scenarios (each also run with `ReverseTargets` and with the three re-trigger modes, inside the test): `shell_done_fires_once_when_on_station_end_and_zone_out_coincide`, `shell_losses_all_planes_triggers_done`, `shell_cold_area_ends_at_on_station_end`, `shell_hot_area_extends_once_then_ends`, `shell_late_flight_ends_at_hard_stop`, `shell_airalert_on_station_starts_once`, `budget_grants_2_refuses_3rd_then_grants_after_deletion`, `budget_two_deletions_in_one_tick_reopen`, `budget_two_deletions_50_ms_apart_reopen`, `budget_deletion_during_recount_runs_a_second_recount`, `budget_request_during_recount_is_dropped`, `budget_never_locks_when_nothing_flies`, `budget_requests_in_one_tick_overshoot_then_heal`, `budget_same_copy_started_twice_in_one_tick`, `budget_single_slot_grants_1`, `assemble_side_numbers_copies_and_uses_spawn_rule`;
  - `shell_and_budget_pass_the_rearm_rule`: `walker_every_deactivated_out_has_a_rearm_or_is_single_use` passes on every wrapped built-in copy and on the budget (N = 1, 2, 3), with the single-use exemptions the builders declare.
- `write_p14_sortie_probe` and `write_p14_load_probe` exist, are `#[ignore]`d, and run: `cargo test --offline write_p14_ -- --ignored` passes and writes their files.
- The MCU counts (17 per copy, 16 in the budget, 14 at N = 1) come out as the plan states. If one does not, stop and report the count and the reason; do not change a test to match.
- `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline`: at least 583 passed (537 now plus the 46 named tests), 0 failed, 11 ignored (9 now plus the two writers).
- The step 1 baseline tests stay green: no Generate output changes.

Files in scope: `src/airtask/shell.rs`, `src/airtask/budget.rs`, `src/airtask/assemble.rs`.

Do not touch: every other file. In particular `src/airtask/mod.rs` and the other `src/airtask/*.rs`, `src/trace.rs`, `src/missionlog.rs`, `src/ui.rs`, `src/template.rs`, `src/placement.rs`, `src/weapon_range.rs`, `src/mapnet.rs`, `src/ast.rs`, `src/testdata/`, `TemplateExamples/`, `handoff/`, `docs/`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`.
</task>

<git_protocol>
Same-host mode. The orchestrator has already checked out the branch `codex/p14-3` in this worktree. Edit files in the working tree only. Do not run any git command that writes (no add, commit, branch, checkout, stash, reset, restore, worktree). `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start (other than this prompt file), stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Do not stop after a partial change. Every acceptance line must hold before you report, or you stop and report under the rules above.
</completeness_contract>

<verification_loop>
Before you report, run `cargo build --offline`, `cargo clippy --offline`, `cargo test --offline` and the ignored writers, and read the warning counts and the test totals from their output. If a count is above its limit or a test fails, fix it inside the files in scope and run them again. Do not silence a warning in existing code. New code adds no warnings.
</verification_loop>

<action_safety>
Stay inside the files in scope. No refactor, rename, reformat or cleanup of existing code. Do not run `cargo fmt` on the whole crate; format only the code you add.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Read the code and the templates. Where the plan and the code disagree (names, indexes, values), follow the code and the files for facts, follow the plan for semantics, and list each difference in the report. U24 (one check zone per area) is applied in step 4, not here; the shell's copy has no check zone except Zone Out.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary of the change (5 lines at most).
2. The command results: build warnings, clippy warnings, tests passed / failed / ignored, and the files the writers produced with their sizes.
3. The public API you added in the three files (types and signatures).
4. The MCUs one wrapped HVAR copy adds (names and count), and the budget's MCUs at N = 3 and N = 1.
5. Each strategy variant, how it is selected, and which defaults are marked `P14 G0`.
6. For each walker scenario: the stimulus and the firings or gate states it asserts.
7. Each choice you made where the plan gave no exact shape, differences between the plan and the code, and residual risks.
</structured_output_contract>
