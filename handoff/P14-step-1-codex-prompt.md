<task>
Build the rest of P14 step 1 (core helpers and baselines) in this repository (Rust 2024 binary crate, egui, nom). The step 1a skeleton is already on this branch.

Read these first, in this order:
1. `handoff/P14-air-tasking.md`: §8 "General rules for every step"; §8 "Step 1" (the part from "Rest of step 1" to the end of its acceptance list); §7 "Byte-identical guarantee"; §3.1 step 2 (`rotate_tree`) and step 5 (`snap_air_attack_areas`); §4.1 and §4.2 (timing); §5 "`in_theatre.rs` (D20, D34)"; decisions D34 and D40.
2. `CLAUDE.md` and `.cursorrules`.
3. `docs/historical-reference/korea-1950-53-unit-reference.md` §2.2, §3 and §9, for the `in_theatre` dates and citations.

Work in this order. The order matters for part A.

**Part A. Baselines first, before any other edit.**
- `src/baseline_tests.rs`: the fixture inputs written as code, the ignored writer `write_p14_baseline`, and the test `existing_outputs_match_baseline`.
  - Fixtures, in `src/testdata/p14_baseline/`: `template.Group`, `recon.Group`, `fighter.Group`, `exclusive.Group`, `airfield_mp.Group`, `base_map.Group`, plus each file's locale sidecars where the generator writes them.
  - Each fixture calls the same crate function that its tab's primary button calls. Read the call in `src/ui.rs` and use the same function with the same kind of arguments: `generate_unit_template` → `template::generate_template`; `generate_recon_file` → `recon::generate_recon_ex`; `generate_fighter_file` → `pack::generate_pack` and the override step that follows it; `generate_bomber_file` → `bombers::link_bomber_plans_with`; the Airfield tab's primary button → the `airfield` function it calls; `generate_front_file` → `frontlines::generate_front`. If a function that the button path needs is private to `ui.rs`, do not change `ui.rs`: rebuild the same input in the test from public crate functions, and list it in the report.
  - `template.Group`: the seat list includes an F-80C (`f80c10`) with one waypoint at `model_spec::suggested_waypoint_speed_kmh(["f80c10"])` (660).
  - `base_map.Group`: a `FrontOptions` with fighters, ships, ground packs, armies and attack arrows, built with `..FrontOptions::default()`.
  - Inputs that are files come from `TemplateExamples/` by a path relative to `CARGO_MANIFEST_DIR`. No input depends on the clock, on a random seed drawn at run time, on the height store or on a path outside the repo.
  - The comparison is byte for byte, and on a mismatch the message names the fixture and the first byte offset that differs.
- `src/ui_tests.rs`: add only the ignored writer `write_p14_baseline_ui` and the test `map_generate_without_air_matches_baseline`, with the inputs that §7 gives (direct field sets on `h.app`, two fixed points in each of `east_objectives` and `nato_objectives`, one `attack_arrows` entry, a fixed `front_t`, `terrain_apply = false`, the save path from `dialog::answer`). The fixture is `src/testdata/p14_baseline/base_map_ui.Group`.
- Run both writers once (`cargo test --offline -- --ignored write_p14_baseline` and the `_ui` one), before you edit any file of part B. Then run each writer a second time and confirm with `git status` / a byte compare that no fixture changed (the writers are deterministic).

**Part B. Core helpers.**
- `src/placement.rs`: `pub(crate) fn rotate_tree(root: &mut Il2Entity, pivot: (f64, f64), theta_deg: f64)` as §3.1 step 2 describes, with `#[allow(dead_code)] // P14`. It reuses `set_coord` and `add_yori`. It adds θ to `YOri` only on the nodes that `apply_group_heading` turns. It never changes `XOri`, `ZOri` or `YPos`. No existing function changes.
- `src/weapon_range.rs`: `pub fn snap_air_attack_areas(entity: &mut Il2Entity, x: f64, z: f64)`, the same code as `snap_ground_attack_areas` with the predicate `AttackAir == 1`, with `#[allow(dead_code)] // P14`. `snap_ground_attack_areas` does not change.
- `src/airtask/timing.rs`: `pub const F80_CRUISE_OVERRIDE_KMH: Option<f64> = None;`, the bodies of `f80_transit_s` and `derived_timing`, and one inner helper that takes the override as an argument so that the test can pass `Some(700.0)`. The speed is the override if it is `Some`, else `model_spec::spec_for("f80c10")` cruise, read at run time. `timing.rs` never writes to `model_spec`.
- `src/in_theatre.rs`: the table of §5 (one row per plane id in `model_spec`; `DateRange`, side, citation, `sourced`), and the lookup functions that the four tests need. A row without a sourced date uses the war span 1950-06-25 → 1953-07-27 and `sourced = false`. Do not invent a date. `mig15bis` follows D34. Leave the La-9 comment for R4 P11.

Acceptance. Each line is a named test or a command result:
- `existing_outputs_match_baseline` and `map_generate_without_air_matches_baseline` pass.
- `rotate_tree_quarter_turn`: by 90° about (40000, 40000), WP 1 moves from (44000, h, 40000) to (40000, h, 44000); `YPos` is unchanged; plane `YOri` gets +90; MCU `YOri` is unchanged. Use a built-in template from `TemplateExamples/Historical1950/`.
- `snap_air_attack_areas_moves_only_air`: on `1950_US_F80_AirAlert_4ship.Group`, only nodes with `AttackAir = 1` move; the ground AttackArea is untouched.
- `f80_transit_and_derived_timing`: `f80_transit_s(12000.0)` is within 0.1 s of 118.0; `derived_timing` gives W of 100 / 120 / 160 for R of 10 / 12 / 16 km, H = 2W, r_in = R/2, and applies the `Option` overrides.
- `timing_override_wins_and_model_spec_is_not_written`: with `Some(700.0)` passed to the inner helper, the transit of 12000 m is within 0.1 s of 123.4; the source text of `timing.rs` holds no write to `model_spec`.
- `in_theatre_la11_invalid_in_1950`: `la11` on 1950-08-01 is invalid and its citation holds "[S9 p.686]".
- `in_theatre_mig15bis_from_1950_11_01`: invalid on 1950-10-31, valid on 1950-11-01, and the citation holds "stand-in".
- `in_theatre_all_builtins_valid_on_1950_07_01`: every plane model in the six files of `TemplateExamples/Historical1950/` is valid on 1950-07-01. Read the models from the files.
- `in_theatre_every_row_is_cited_or_unsourced`: each row has a citation or `sourced = false`, and every plane id of `model_spec` has a row.
- `cargo build --offline`: at most 69 warnings.
- `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline`: at least 482 passed (472 on the base commit plus the ten named tests), 0 failed, 5 ignored (3 on the base commit plus the two writers).

Files in scope (you may create or edit only these):
`src/placement.rs`, `src/weapon_range.rs`, `src/airtask/timing.rs`, `src/in_theatre.rs`, `src/baseline_tests.rs`, `src/ui_tests.rs` (the two additions only), and new files under `src/testdata/p14_baseline/`.

Do not touch:
- `src/ui.rs`, `src/airtask/mod.rs` and every other `src/airtask/*.rs` file, `src/model_spec.rs`, `src/template.rs`, `src/recon.rs`, `src/frontlines.rs`, `src/mapnet.rs`, `src/trace.rs`, `src/missionlog.rs`, `src/main.rs`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, `TemplateExamples/`, `handoff/`, `docs/`, `HANDOFF.md`, `USER_MANUAL.md`.
- Any existing function or existing test. The diff of `placement.rs`, `weapon_range.rs` and `ui_tests.rs` holds additions only.
- The output of every Generate path.
- If a type in `src/airtask/mod.rs` must change for this step, stop and report. Do not change it.
</task>

<git_protocol>
Same-host mode. The orchestrator has already checked out the branch `codex/p14-1`. Edit files in the working tree only. Do not run any git command that writes (no add, commit, branch, checkout, stash, reset, restore). `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Do not stop after a partial change. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Before you report, run `cargo build --offline`, `cargo clippy --offline` and `cargo test --offline`, and read the warning counts and the test totals from their output. If a count is above its limit or a test fails, fix it inside the files in scope and run all three again. Do not silence a warning in existing code. Do not write a fixture again after part B starts: if a baseline test fails after part B, the fault is in part B.
</verification_loop>

<action_safety>
Stay inside the files-in-scope list. No refactor, rename, reformat or cleanup of existing code. Do not run `cargo fmt` on the whole crate; format only the code you add.
</action_safety>

<missing_context_gating>
Do not guess repository facts or historical dates. Read the code and the reference. If the plan and the code disagree, follow the code for names and signatures of existing items, follow the plan for new items, and list the difference in the report.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary of the change (5 lines at most).
2. Files touched, one per line, each with "new" or "edited".
3. The three command results: build warnings, clippy warnings, tests passed / failed / ignored.
4. For each fixture: the function it calls, its inputs in one line, and its size in bytes.
5. The `in_theatre` rows: id, range, sourced, citation.
6. Each choice you made where the plan gave no exact shape, differences between the plan and the code, and residual risks.
</structured_output_contract>
