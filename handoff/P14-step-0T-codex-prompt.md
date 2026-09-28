<task>
Build P14 step 0T (trace builds and the replay core) in this repository (Rust 2024 binary crate, egui, nom). Steps 1a and 1 are already on this branch; `src/trace.rs` and `src/missionlog.rs` hold the 1a stubs, and `main.rs` already has `mod trace; mod missionlog;` and the `missionlog::run_cli` hook.

Read these first, in this order:
1. `handoff/P14-air-tasking.md`: §8 "General rules for every step"; §8 "Step 0T. Trace builds and replay core (P10)" (the whole section, 0T-1, 0T-2, the CLI, the acceptance list); §9 risks t1–t6; §3.0 "Conventions".
2. `handoff/P14-review-fable-astra.md`: findings C9 (state links vs pulse links), T8 / R5-5 (event types in `TraceSelect` and `TraceEntry`). They are already folded into the plan; read them for the reasoning.
3. `CLAUDE.md` and `.cursorrules`.
4. `src/trace.rs`, `src/missionlog.rs` (the stubs), `src/main.rs` (the CLI hooks), `src/heightprobe.rs` (`run_cli`: the CLI style to follow), `src/ast.rs` (`Il2Entity`), `src/template.rs` (`mcu`, `timer`, `timer_random`, `attach_event` builders), `src/flights.rs` (`build_randomizer` and its Deactivate closers), `src/geo.rs` ≈23-24 and `src/placement.rs` ≈30-43 (the bounds), `src/watermap.rs` (dry-land check), `src/baseline_tests.rs` and `src/testdata/p14_baseline/` (the step 1 fixtures), `TemplateExamples/K14 AFB_mp.Group` ≈1519-1570 (the WillysMB `NOICON` vehicle and its entity).

What to build:
- `src/trace.rs`: every item of 0T-1. Fill the stub bodies: `TraceSelect::decision_points()` (the base set of the plan; callers add name prefixes), `instrument`, `breadcrumb`, `TraceMap::push`, `write_trace_sidecar`, both carriers (`Objective(style)` and `Spawn`), the trace grid constants `TRACE_X0` / `TRACE_Z0` and `TRACE_SPAWN_POINT`, and the `edges` walk (pulse links only, visited set, `unconditional` and `delay_s` exactly as the plan defines them). Add a sidecar reader so the round-trip test and the CLI's `--trace` can load it.
- `src/missionlog.rs`: every item of 0T-2: `MissionLog`, `Record` with the typed views, `parse_log_dir`, `parse_log_files` (sort by the `[n]` suffix), tolerant parsing (unknown types and fields kept and counted, never fatal), the tick-rate fit, `Replay` and `replay(log, group, trace)`, `report_markdown`, and `run_cli` with the exact argument syntax, output and exit codes of the plan.
- `src/testdata/missionlog/synthetic/`: the synthetic fixture the plan describes (two files `[0]` and `[1]`, every typed kind, one unknown `AType`, one unknown field), plus whatever small synthetic `.Group` / log pairs the replay tests need. Keep each file small.
- The JSON sidecar must be written and read without a new crate (`Cargo.toml` is do-not-touch). First look for existing JSON or sidecar code in `src/` and reuse its style; otherwise write a small writer and reader for exactly the `TraceMap` shape.

Frozen interfaces: keep the 1a stub signatures and the fields of `TraceSelect`, `ObjectiveStyle`, `TraceCarrier`, `TraceSource`, `TraceMap`, `TraceEntry` and `TraceEdge` as they are. You may add items (functions, constants, types, derives). If the plan cannot be met without changing one of these, stop and report instead.

Not in this step (do not build): the probe hook-up and cell T-t (step 0b, `probe.rs`), `probe_traced_every_subtitle_source_has_a_breadcrumb`, any UI (step 9T), any `real_1` fixture (no real log exists yet: build against the documented format and the synthetic fixture only).

Server setting: you have no network. Do not claim the `startup.cfg` key or the log folder is confirmed. Write in the report what the plan says, marked UNVERIFIED, and anything in the repo that bears on it.

Acceptance. Each line is a named test or a command result:
- `trace_off_changes_nothing`: every step 1 baseline fixture is unchanged, and `instrument(` appears only in `trace.rs`, `probe.rs` and tests (a source grep over `src/`).
- `trace_instrument_adds_exactly_one_breadcrumb_per_source`, `trace_breadcrumbs_are_unique_and_inside_the_map` (also pins `TRACE_X0`, `TRACE_Z0` and both bound checks), `trace_entity_source_gets_an_onevent`, `trace_edges_follow_onevents_and_stop_on_loops`, `trace_sidecar_round_trips`, `trace_edges_skip_untraced_mcus`, `trace_spawn_carrier_deletes_its_object`, `trace_edges_do_not_follow_state_links`, `trace_edge_is_unconditional_only_without_gates_and_counters`: each defined exactly as in the plan's 0T acceptance list.
- `missionlog_parses_synthetic_fixture`; `missionlog_real_fixture_parses` present and `#[ignore = "needs a real log (0L or flight 0)"]`.
- `replay_unobserved_successor_behind_a_gate_is_not_a_dead_link`, `replay_dead_link_on_an_unconditional_path`, `replay_never_spawned_counts_per_group`, `replay_tick_rate_fit` (50 ticks/s ± 1 %), `replay_cli_bad_args_exit_2`, `replay_cli_writes_report` (in-process, `--out` to a temp file under `std::env::temp_dir()`, removed afterwards).
- `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline`: at least 499 passed (482 now plus the 17 non-ignored named tests), 0 failed, 6 ignored.

Files in scope: `src/trace.rs`, `src/missionlog.rs`, new files under `src/testdata/missionlog/`. You may add `trace_off_changes_nothing` to `src/baseline_tests.rs` instead of `trace.rs` if that reuses its fixture helpers; there, additions only.

Do not touch: every other file. In particular `src/main.rs`, `src/airtask/`, `src/ui.rs`, `src/template.rs`, `src/flights.rs`, `src/placement.rs`, `src/geo.rs`, `src/ast.rs`, `src/serialize.rs`, `src/testdata/p14_baseline/`, `TemplateExamples/`, `handoff/`, `docs/`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`. No change to any Generate output: nothing outside tests calls `instrument`. If you need a private item from another module made `pub(crate)`, stop and report instead of changing it.
</task>

<git_protocol>
Same-host mode. The orchestrator has already checked out the branch `codex/p14-0t` in this worktree. Edit files in the working tree only. Do not run any git command that writes (no add, commit, branch, checkout, stash, reset, restore, worktree). `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start (other than this prompt file), stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Do not stop after a partial change. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Before you report, run `cargo build --offline`, `cargo clippy --offline` and `cargo test --offline`, and read the warning counts and the test totals from their output. If a count is above its limit or a test fails, fix it inside the files in scope and run all three again. Do not silence a warning in existing code. New code adds no warnings.
</verification_loop>

<action_safety>
Stay inside the files in scope. No refactor, rename, reformat or cleanup of existing code. Do not run `cargo fmt` on the whole crate; format only the code you add.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Read the code. The log format and every t-risk are UNVERIFIED: build a tolerant parser against the documented format and say in the report which field names and meanings you assumed. Where the plan and the code disagree, follow the code for names and signatures of existing items, follow the plan for semantics, and list the difference in the report.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary of the change (5 lines at most).
2. The three command results: build warnings, clippy warnings, tests passed / failed / ignored.
3. The public API you added (types, constants and function signatures) in `trace.rs` and `missionlog.rs`.
4. The log-line format you parse: each typed AType, the field names you read, and which are assumed.
5. The trace grid and spawn-point constants, and how the spawn point was checked as dry land.
6. For each acceptance test: the fixture or stimulus and what it asserts.
7. Each choice you made where the plan gave no exact shape, differences between the plan and the code, the server-setting note, and residual risks.
</structured_output_contract>
