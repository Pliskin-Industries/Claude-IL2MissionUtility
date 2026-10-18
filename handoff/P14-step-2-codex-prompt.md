<task>
Build P14 step 2 (the sortie library, `inspect_sortie` and `place_air_sortie`) in this repository (Rust 2024 binary crate, egui, nom). Steps 1a, 1, 1b, 0T and 0b are already on this branch.

Read these first, in this order:
1. `handoff/P14-air-tasking.md`: §8 "General rules for every step"; §8 "Step 2. Library and `place_air_sortie`" (the whole section and its acceptance list); §3.0 "Conventions"; §3.1 (placement); §3.2 (the accepted shapes S1–S7, the refusals and their messages, U21 "tested shapes only"); §5 "Data model" (the `AirSortie` type, the built-in table with side and roles, the "Builders" list) and every decision the step names (D26, D39, T1, T5).
2. `CLAUDE.md` and `.cursorrules`.
3. `src/airtask/mod.rs` (frozen types and the `pub use` lines), `src/airtask/library.rs` and `src/airtask/place.rs` (stubs), `src/airtask/testkit.rs` (`assert_links_resolve`), `src/template.rs` (`generate_template`, `TemplateOptions`, the builders made `pub(crate)` in 1a), `src/placement.rs` (`rotate_tree`, `move_anchor_to`, `set_coord`, `add_yori`), `src/weapon_range.rs` (`snap_ground_attack_areas`, `snap_air_attack_areas`), `src/mapnet.rs` (`is_rtb_waypoint`, `park_path_waypoints`), `src/model_spec.rs` (`suggested_waypoint_speed_kmh`), `src/airtask/timing.rs`, and the six built-in templates under `TemplateExamples/` that §5 names.

What to build:
- `src/airtask/library.rs`: `builtin_library()` (all six, with side and role detection per the §5 table), `load_user_sortie(path)`, `inspect_sortie` finding every part **by wiring, never by name**, the accepted-shape table (S1–S7) and every §3.2 refusal with its message.
- `src/airtask/place.rs`: `place_air_sortie` per §3.1 and the step's acceptance list: lead at the spawn point, rigid rotation of every unsnapped node, plane `YOri` + θ, path waypoints and ground and air AttackAreas snapped onto the target, one RTB per lead (keep and move an existing RTB waypoint, add the missing ones, all targets of the one RTB timer), `DELAYED END ORDERS.Time = 60`, the low-speed rule (D39), and the D26 refusal.

Frozen interfaces: the types, stub signatures and `pub use` lines of `src/airtask/mod.rs` stay as they are. If the step cannot be built without changing one of them, or without making a private item elsewhere `pub(crate)`, stop and report instead.

Acceptance. Each line is a named test or a command result:
- Every test the step's acceptance list names, each defined exactly as there: `library_builtins_load_with_side_and_roles`, `inspect_sortie_finds_the_parts_by_wiring`, `inspect_sortie_does_not_read_names`, `inspect_sortie_accepts_each_tested_shape`, `inspect_sortie_refusals`, `inspect_sortie_accepts_nothing_untested`, `inspect_sortie_escort_first_picks_the_second_lead`, `library_leads_are_entities_with_empty_targets`, `library_speed_alt_attack_time_per_file`, the seven `place_all_builtins_*` tests (`…_lead_at_spawn`, `…_bearing`, `…_offsets_rotate_rigidly`, `…_plane_yori`, `…_waypoints_and_attack_areas_on_target`, `…_rtb_per_lead`, `…_links_resolve`), `place_rejects_two_path_waypoints_per_lead`, `place_rejects_template_without_waypoint`, `place_low_speed_uses_model_spec_and_warns`, `place_low_speed_with_unknown_models_is_refused`, `place_rtb_per_lead_keeps_and_adds`.
- Templates for refusal and shape tests are written by `generate_template` in the test, as the plan says; do not hand-write `.Group` text for them.
- `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline`: at least 537 passed (516 now plus the 21 named tests), 0 failed, 9 ignored.
- The step 1 baseline tests stay green: no Generate output changes.

Files in scope: `src/airtask/library.rs`, `src/airtask/place.rs`.

Do not touch: every other file. In particular `src/airtask/mod.rs` and the other `src/airtask/*.rs`, `src/trace.rs`, `src/missionlog.rs`, `src/ui.rs`, `src/template.rs`, `src/placement.rs`, `src/weapon_range.rs`, `src/mapnet.rs`, `src/model_spec.rs`, `src/ast.rs`, `src/testdata/`, `TemplateExamples/`, `handoff/`, `docs/`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`.
</task>

<git_protocol>
Same-host mode. The orchestrator has already checked out the branch `codex/p14-2` in this worktree. Edit files in the working tree only. Do not run any git command that writes (no add, commit, branch, checkout, stash, reset, restore, worktree). `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start (other than this prompt file), stop and report.
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
Do not guess repository facts. Read the code and the template files. Where the plan and the code disagree (names, indexes, line numbers, values such as speeds, altitudes and attack times), follow the code and the files for facts, follow the plan for semantics, and list each difference in the report. If a pinned value in the acceptance list does not match the file, stop and report it rather than change the test.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary of the change (5 lines at most).
2. The three command results: build warnings, clippy warnings, tests passed / failed / ignored.
3. The public API you added in `library.rs` and `place.rs` (types and signatures).
4. How `inspect_sortie` identifies each part by wiring, one line per part.
5. The accepted-shape table and each refusal with its message.
6. For each acceptance test: the fixture or stimulus and what it asserts.
7. Each choice you made where the plan gave no exact shape, differences between the plan and the code, and residual risks.
</structured_output_contract>
