<task>
Build P14 step 0b (the in-game probe generators) in this repository (Rust 2024 binary crate, egui, nom). Steps 1a and 1 are already on this branch; `src/airtask/probe.rs` is the 1a stub.

Step 0T (trace builds, `src/trace.rs`) is being built at the same time on another branch. On this branch `trace::breadcrumb`, `trace::instrument` and `TraceMap::push` are still the 1a stubs (they return nothing and add nothing). Call them exactly as the plan says, through their stub signatures; they fill in when both branches merge. Do not edit `trace.rs`.

Read these first, in this order:
1. `handoff/P14-air-tasking.md`: §8 "General rules for every step"; §8 "Step 0. In-game MCU probe" (0b, the three run sheets, the Cells table, the Step 0 acceptance list); §8 "Step 0T" only for "Probe hook-up (with 0b)", cell T-t and its T1–T7 table, and the `breadcrumb` / `TraceMap::push` / `TraceSelect` signatures; §9 risk rows a, c, e, f, g, h, j, n, o, p, q, r, z, t1–t6; §3.0 "Conventions"; §5 "Builders".
2. `CLAUDE.md` and `.cursorrules`.
3. `src/airtask/probe.rs`, `src/airtask/mod.rs`, `src/airtask/testkit.rs` (`assert_links_resolve`), `src/trace.rs` (stub signatures), `src/template.rs` (`mcu`, `timer`, `timer_random`, `counter`, `checkzone`, `modifier_set_val`, `attach_event`), `src/recon.rs` (`silence_clone_starts` and the comment above it), `src/duplicate.rs` (`duplicate_template`), `src/placement.rs` (`move_anchor_to`), `src/mapnet.rs` (`park_path_waypoints`), `src/weapon_range.rs` (`snap_ground_attack_areas`), `src/parser.rs` (`parse_group_file`), `src/serialize.rs`, `src/locale.rs`, `TemplateExamples/BomberMissions/B29Mission.Group` ≈758-785 (the subtitle block shape), `TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group`, `TemplateExamples/K14 AFB_mp.Group` (the K-14 origin).

What to build, in `src/airtask/probe.rs` (all `#[cfg(test)]` or used only from tests) plus its locale strings:
- `generate_probe_0() -> Il2Entity`, `generate_probe_1a() -> Il2Entity`, `generate_probe_1b() -> Il2Entity`, exactly as 0b and the three run sheets define them: every cell, every cue, readout counters, per-event subtitles (`Duration = 1`) and cue subtitles (`Duration = 20`), `MCU_Icon` markers, the constants `K14_ORIGIN`, `RUN_SHEET_0`, `RUN_SHEET_1A`, `RUN_SHEET_1B`, `CUES`.
- The test-only helper `probe_flight(src, keep_planes, start, exit, next_id)` with the six steps of 0b.
- Cell T-t in flight 0 (T1–T5, T7 as the T-t table; T6 is the whole traced probe), built with `trace::breadcrumb` and registered with `TraceMap::push`, and T-t's `LCName` string.
- The traced variant for each flight per "Probe hook-up (with 0b)": `instrument(probe, &TraceSelect { indexes: <every per-event, readout and cue subtitle's source, except T-t's>, ..}, next_id, carrier, &mut map)`, with flight 0 on `TraceCarrier::Objective(ObjectiveStyle::default())` and a `map` that already holds T-t's breadcrumbs. Flights 1A and 1B use a `carrier` parameter or constant that defaults to the same, since the flight 0 results are not in yet. Cue breadcrumbs set `expected_s` (via the `TraceSource` you pass) to the cue time.
- Ignored writers `write_p14_probe_0`, `write_p14_probe_1a`, `write_p14_probe_1b`: each writes the plain file, the traced file, the traced file's `.trace.json` (via `trace::write_trace_sidecar`; on this branch it returns `Err`, so the writer must report that error clearly and still write the other files) and the `.eng` locale file into `target/p14/`, with the names in 0b.

Acceptance. Each line is a named test or a command result:
- `probe_files_parse_and_links_resolve`, `probe_subtitles_are_unique`, `probe_cells_match_the_table`, `probe_run_sheets_match_the_tables`: each defined exactly as in the Step 0 acceptance list, **except** the T-t breadcrumb assertions ("T-t has T1–T5 and T7 breadcrumbs at the T-t times, T4's six variants 4 s apart, T5's two firings, and T7's one breadcrumb with two same-tick inputs at 05:25"). Those go in a separate test `probe_tt_breadcrumbs_match_the_table`, fully written, marked `#[ignore = "needs step 0T trace bodies"]`. `probe_cells_match_the_table` still checks T-t's timers and times, which do not depend on the breadcrumbs.
- The three ignored writers run: `cargo test --offline write_p14_probe_ -- --ignored` writes every file except the `.trace.json` sidecars, and reports the sidecar error.
- `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline`: at least 486 passed (482 now plus the four named tests), 0 failed, 9 ignored (5 now plus 3 writers plus `probe_tt_breadcrumbs_match_the_table`).

Files in scope: `src/airtask/probe.rs`; `src/locale.rs` (additions only: the probe's strings or a helper to build its `.eng` table, near ≈89).

Do not touch: every other file. In particular `src/trace.rs`, `src/missionlog.rs`, `src/main.rs`, `src/airtask/mod.rs` and the other `src/airtask/*.rs`, `src/ui.rs`, `src/template.rs`, `src/recon.rs`, `src/duplicate.rs`, `src/placement.rs`, `src/mapnet.rs`, `src/weapon_range.rs`, `src/ast.rs`, `src/serialize.rs`, `src/testdata/`, `TemplateExamples/`, `handoff/`, `docs/`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`. No change to any Generate output: nothing outside tests calls the probe code. `rotate_tree` and `inspect_sortie` are not available to 0b (plan). If you need a private item from another module made `pub(crate)`, stop and report instead of changing it.
</task>

<git_protocol>
Same-host mode. The orchestrator has already checked out the branch `codex/p14-0b` in this worktree. Edit files in the working tree only. Do not run any git command that writes (no add, commit, branch, checkout, stash, reset, restore, worktree). `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start (other than this prompt file), stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Do not stop after a partial change. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Before you report, run `cargo build --offline`, `cargo clippy --offline` and `cargo test --offline`, then the ignored writers, and read the warning counts and the test totals from their output. If a count is above its limit or a test fails, fix it inside the files in scope and run them again. Do not silence a warning in existing code. New code adds no warnings.
</verification_loop>

<action_safety>
Stay inside the files in scope. No refactor, rename, reformat or cleanup of existing code. Do not run `cargo fmt` on the whole crate; format only the code you add.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Read the code and the template files. Where the plan and the code disagree (names, line numbers, MCU names in the HVAR file), follow the code for names and signatures of existing items, follow the plan for semantics, and list the difference in the report.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary of the change (5 lines at most).
2. The command results: build warnings, clippy warnings, tests passed / failed / ignored, and the files the writers produced with their sizes.
3. The public-in-crate API and constants you added.
4. For each cell of the three run sheets: its key MCUs (names, indexes) and times, and anything the table left open that you chose.
5. How T-t and the traced variants call the 0T stubs, and what will change once 0T's bodies merge.
6. Differences between the plan and the code, and residual risks.
</structured_output_contract>
