<task>
Build P14 step 1a, the skeleton commit, in this repository (Rust 2024 binary crate, egui, nom).

Read these first, in this order:
1. `handoff/P14-air-tasking.md`: §5 "Data model" (the exact skeleton source of `airtask/mod.rs` and the "Builders" list), §8 "General rules for every step", §8 "Step 1" (the part headed "First commit: the skeleton (step 1a)"), §8 "Step 0T" (only for the signatures that the `trace.rs` and `missionlog.rs` stubs must carry), §3.6 (the formula of `weighted_random_pct`).
2. `CLAUDE.md` and `.cursorrules`.

Expected end state. Each line is an acceptance check:
- `src/main.rs` declares `mod airtask;`, `mod in_theatre;`, `mod trace;`, `mod missionlog;` and `#[cfg(test)] mod baseline_tests;`, and calls `missionlog::run_cli(&args)` next to the existing `heightprobe::run_cli` hook, in the same style.
- `src/airtask/mod.rs` has the content of the §5 source: every type, derive, `Default`, constant, `mod` line and `pub use` line, with the same names, fields, field order and default values. rustfmt layout is accepted. `weighted_random_pct` has its real body (§3.6: `p_i = round(100 · w_i / Σ_{j≥i} w_j)`, the last entry 100). `build_air_packs`, `preview`, `validate` and `totals` are stubs with the bodies shown in §5.
- One stub file per submodule: `library.rs`, `place.rs`, `shell.rs`, `budget.rs`, `assemble.rs`, `threshold.rs`, `clock.rs`, `zone.rs`, `timing.rs`, `probe.rs`. Each is a doc comment only, except:
  - `library.rs`: `pub fn builtin_library() -> Vec<AirSortie>` returns `vec![]`; `pub fn load_user_sortie(_path: &std::path::Path) -> Result<AirSortie, String>` returns an `Err` that says the function is built in step 2.
  - `timing.rs`: `pub fn f80_transit_s(_radius_m: f64) -> f64` returns `0.0`; `pub fn derived_timing(_t: &ThresholdSpec) -> DerivedTiming` returns the default.
- `src/airtask/testkit.rs` (`#[cfg(test)]`): `pub(crate) fn assert_links_resolve(root: &Il2Entity)` panics, naming the node and the index, when a `Targets` or `Objects` index of any node does not resolve to a node in the tree. Add a doc-comment placeholder for the walker (step 1b builds it).
- `src/in_theatre.rs`: a doc comment and `#![allow(dead_code)] // P14: removed in step 9`.
- `src/baseline_tests.rs`: a doc comment only.
- `src/trace.rs`: module-level `#![allow(dead_code)] // P14: removed in step 9T`. It holds the types and signatures that step 0T names (`TraceSelect`, `TraceCarrier`, `ObjectiveStyle`, `TraceSource`, `TraceMap`, `TraceEntry`, `TraceEdge`, `instrument`, `breadcrumb`, `TraceMap::push`, `write_trace_sidecar`), with stub bodies. Index fields are `i32`. Where step 0T does not give a field list (`ObjectiveStyle`, `TraceSource`), choose the smallest shape that fits the step 0T text, and list your choice in the report.
- `src/missionlog.rs`: `pub fn run_cli(args: &[String]) -> Option<i32>` returns `None` unless the first argument is `replay`; for `replay` the stub prints one line that says the command is built in step 0T and returns `Some(2)`. Put an item-level `#[allow(dead_code)] // P14: removed in step 9T` only on an item that needs it.
- `src/frontlines.rs`: `#[derive(Debug, Clone)] pub struct MapAirPack { pub root: Il2Entity }` next to `MapGroundPack`, with a one-line doc comment and `#[allow(dead_code)] // P14`.
- Visibility-only edits, in place, with no move and no other change: `pub(crate)` on `mcu`, `timer`, `counter`, `checkzone`, `modifier_set_val`, `attach_event` (`src/template.rs`); `clone_named`, `synthesize_mcu`, `silence_clone_starts` (`src/recon.rs`); `set_coord`, `add_yori` (`src/placement.rs`); `is_rtb_waypoint` (`src/mapnet.rs`).
- New `pub(crate) fn timer_random(name: &str, time: f64, pct: u8, next_id: &mut i32, x: f64, z: f64) -> Il2Entity` directly after `timer` in `src/template.rs`, with `#[allow(dead_code)] // P14`. It is `timer` with `Random` set to `pct`. No Generate path calls it.
- The enums whose variants only step 12 uses (`ThresholdImpl`, `CounterResetImpl`, `ZoneOutImpl`) carry an item-level `#[allow(dead_code)] // P14 step 12`.
- Every stub names its unused arguments with a leading underscore.
- New tests, all passing:
  - `weighted_random_pct_values` (in `airtask/mod.rs`): `weighted_random_pct(&[5,5,5]) == [33,50,100]`, `weighted_random_pct(&[1,3]) == [25,100]`, and an empty slice gives an empty `Vec`.
  - `assert_links_resolve_accepts_a_builtin_template` (in `testkit.rs`): parse `TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group` with the crate's parser; the check passes.
  - `assert_links_resolve_rejects_a_dangling_target` (in `testkit.rs`, `#[should_panic]`): the same tree with one `Targets` index that no node has.
- `cargo build --offline`: at most 69 warnings (the count on the base commit).
- `cargo clippy --offline`: at most 251 warnings (the count on the base commit).
- `cargo test --offline`: 472 passed, 0 failed, 3 ignored (469 on the base commit plus the three new tests).

Files in scope (you may create or edit only these):
`src/main.rs`, `src/airtask/mod.rs`, `src/airtask/library.rs`, `src/airtask/place.rs`, `src/airtask/shell.rs`, `src/airtask/budget.rs`, `src/airtask/assemble.rs`, `src/airtask/threshold.rs`, `src/airtask/clock.rs`, `src/airtask/zone.rs`, `src/airtask/timing.rs`, `src/airtask/testkit.rs`, `src/airtask/probe.rs`, `src/in_theatre.rs`, `src/trace.rs`, `src/missionlog.rs`, `src/baseline_tests.rs`, `src/frontlines.rs`, `src/template.rs`, `src/recon.rs`, `src/placement.rs`, `src/mapnet.rs`.

Do not touch:
- `src/ui.rs`, `src/ui_tests.rs`, `src/shell.rs`, `src/theme.rs`, `src/help.rs`, `src/model_spec.rs`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, `TemplateExamples/`, `handoff/`, `docs/`, `HANDOFF.md`, `USER_MANUAL.md`.
- Any body, signature, name or position of an existing function. In `template.rs`, `recon.rs`, `placement.rs` and `mapnet.rs` the diff holds visibility keywords and `timer_random` only.
- The output of every Generate path. For the same inputs the app must write the same `.Group` files as before.
- Do not build anything from the rest of step 1, step 1b, step 0T or step 0b: no `rotate_tree`, no `snap_air_attack_areas`, no timing bodies, no `in_theatre` table, no baseline fixtures, no walker, no parser.
</task>

<git_protocol>
Same-host mode. The orchestrator has already checked out the branch `codex/p14-1a`. Edit files in the working tree only. Do not run any git command that writes (no add, commit, branch, checkout, stash, reset, restore). `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Do not stop after a partial change. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Before you report, run `cargo build --offline`, `cargo clippy --offline` and `cargo test --offline`, and read the warning counts and the test totals from their output. If a count is above its limit or a test fails, fix it inside the files in scope and run all three again. If a `pub use` line gives an unused-import warning, put `#[allow(unused_imports)] // P14: removed in step 9` on that line. Do not silence a warning in existing code. Then read `git diff` for `template.rs`, `recon.rs`, `placement.rs` and `mapnet.rs` and confirm that it holds nothing but the visibility keywords and `timer_random`.
</verification_loop>

<action_safety>
Stay inside the files-in-scope list. No refactor, rename, reformat or cleanup of existing code. Do not run `cargo fmt` on the whole crate; format only the new files.
</action_safety>

<missing_context_gating>
Do not guess repository facts. Read the code. If the plan and the code disagree, follow the code for names and signatures of existing items, follow the plan for new items, and list the difference in the report.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary of the change (5 lines at most).
2. Files touched, one per line, each with "new" or "edited".
3. The three command results: build warnings, clippy warnings, tests passed / failed / ignored.
4. Each choice you made where the plan gave no exact shape (the `trace.rs` stub types, any allow you added).
5. Differences between the plan and the code, and residual risks.
</structured_output_contract>
