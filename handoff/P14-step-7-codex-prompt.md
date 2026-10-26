# Delegation prompt for P14 step 7 (export integration, decisions 0032 to 0035)

One job, in the worktree `optimistic-hugle-8e0d98` on `codex/p14-7` (cut from `claude/upstream-merge-0.7` `2e634b5`; head after this prompt is committed). Sent with `codex_start`, model `gpt-6-astra`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1500.

---

<task>
Build P14 step 7: wire the Air Tasking builders of steps 3 to 6 into one export, and the export into the Map base map, as the approved scope and decisions 0032 to 0035 (accepted by the user 2026-10-26) describe.

Read first, in this order:
1. `handoff/sessions/2026-10-26-p14-step7.md` (the approved scope: what to build, files, named tests, the two test plans). Then `docs/decisions/0032` to `0035` (totals limits, the D30 rule within r_in, the acceptance mission as a trace build, one pack rooted at `Air Tasking`). These win over the plan where they differ.
2. `docs/decisions/0023` (item 6: the app's AI country is set in step 7's export) and `0024`.
3. `handoff/P14-air-tasking.md`: §3.0 (names, `SideLogic`, build order, the output group tree), §5 ("Seed", "Target resolution", the frozen types), §6.1 (readiness, warnings vs errors, `rail_only`), §7 (export integration; note `generate_front_file` is now in `src/ui/map.rs`, not `ui.rs`), §8 "Step 7".
4. `handoff/sessions/2026-10-18-p14-step5.md` and `2026-10-22-p14-step6.md`, "For the next steps" (how step 7 calls the clock and the zone and their `validate_schedule` / `validate_zone`).
5. `docs/il2-loader-and-log-facts.md` (do not change it), `CLAUDE.md`, `.cursorrules`.
6. Code: `src/airtask/mod.rs` (the frozen types and the four stub functions), `assemble.rs` (`assemble_side`, `resolve`, `emit_side_logic`, `SideCopies`; tests' `context()`, `test_plan()`, `finish()`), `clock.rs` (`build_side`, `validate_schedule`, the full-side test harness), `zone.rs` (`build_side`, `validate_zone`), `threshold.rs` (`build_side`, the four country constants), `budget.rs` (`build_side`: it asserts the budget is 1..=6), `shell.rs` (`group`, `by_id_mut`), `library.rs` (`builtin_library`, `inspect_sortie`, `BUILTINS`), `place.rs`, `timing.rs` (`derived_timing`), `testkit.rs` (`assert_links_resolve`, `walker_every_deactivated_out_has_a_rearm_or_is_single_use`), `probe.rs` (`build_load_probe`, how probes write files and trace maps; read only), `src/frontlines.rs` (`FrontOptions`, `generate_front`, `stamp_rooted_packs`, `inspect_base_map`, `ImportedBaseMap`, `looks_like_fighter_pack`, `group_has_units`, `MapAirPack`, `dense_base`), `src/trace.rs` (`instrument`, `TraceSelect`, `write_trace_sidecar`), `src/locale.rs`, `src/baseline_tests.rs` (`base_map_fixture`, `existing_outputs_match_baseline`), `src/ui/map.rs` (`generate_front_file`, the Load path), `src/mapclip.rs` (`point_north_of_front`), `src/mapfighters.rs` (`rtb_ao_point`).

What to build:

A. **`src/airtask/export.rs`** (new, private module) holds the bodies; `mod.rs` gets `mod export;` and the four frozen functions (`build_air_packs`, `preview`, `validate`, `totals`) become one-line calls into it. Their signatures do not change (step 8's UI calls them). Do not change any type in `mod.rs`.

B. **`build_air_packs(plan, ctx) -> Result<Vec<MapAirPack>, String>`.**
   - Per side, DPRK then NATO, one `SideLogic` per side, one id allocator for the whole export: `assemble::assemble_side` → `clock::build_side` → `zone::build_side` → `threshold::build_side(side, &copies.areas, &plan.threshold, &plan.strategies, …)` → `budget::build_side(side, plan.budget.per_side[side.idx()], …)` → `assemble::emit_side_logic`; every group returned goes into `copies.trees[0]`, as the clock test harness does.
   - A side with no eligible entry (assembled `entries` empty) contributes nothing: skip its clock, zone, threshold, budget and emit, and leave its groups out.
   - **AI country (0023 item 6):** after emit, every `Plane` in the side's Sorties tree gets `Country` = `NATO_AI_COUNTRY` (602) or `DPRK_AI_COUNTRY` (503). Use the constants; remove their two `#[allow(dead_code)] // P14 step 7` lines in `threshold.rs` (the only change in that file). Do not do it in `place_air_sortie` or `assemble_side`: the flown probe files must stay byte-identical.
   - **Shape (0035 A):** one `MapAirPack` whose root is a group named `Air Tasking` holding, in this order, `Air Tasking DPRK Logic`, `Air Tasking NATO Logic`, `Air Tasking DPRK Sorties`, `Air Tasking NATO Sorties` (only the present sides). `Ok(vec![])` when neither side has an eligible entry.
   - Deterministic: the same plan (with `seed`) and context give an identical tree. No clock time, no `placement_seed()`.
   - **No panic for any plan:** check every input that a builder asserts on (budget 1..=6 on a side with entries, and the like) before calling it, and return `Err` with the same plain text `validate` gives. `build_air_packs` returns `Err` exactly when `validate` has an `Error` other than the `rail_only` one; for the `rail_only` case it returns `Ok(vec![])`.

C. **`validate(plan, ctx) -> Vec<AirIssue>`.** Plain messages, starting "Air Tasking …"; name the entry as "Entry n" (pool index + 1) and the side with `AirSide::label`. Build at most once per call; no recursion between `validate`, `totals` and `build_air_packs`.
   - Errors: no enabled entry that is valid for the date and not orphaned (`rail_only = true`; the only `rail_only` issue; also when the pool is empty); sortie id not in the library; target on the wrong side; runs outside 1..=6; weight outside 1..=10; budget outside 1..=6 on a side that has eligible entries; a template that fails to place, inspect or wrap (the builder's message); the threshold spec errors (the text `threshold::build_side` gives); the errors of `clock::validate_schedule` and `zone::validate_zone` per side; the D30 case when `strategies.ai_counts` is true.
   - Warnings: orphaned target (the entry is skipped); sortie out of date for `ctx.date` (skipped); the warnings of `validate_schedule` / `validate_zone`; **D30 (0033 B):** for each eligible entry of one side, its path spawn → target and target → RTB passes within r_in (`derived_timing(&plan.threshold).inner_m`) of an area centre of the other side (the targets of that side's eligible entries): one issue per (entry, area), naming both; **totals (0032):** one Warning per limit the plan's totals (both sides summed) exceed (`>`), naming the total and the limit; **D41:** two user templates (`SortieSource::File`) used by enabled entries whose locale sidecars hold different text under one id: one Warning per (id, pair of files), naming both files and the id (read sidecars with the existing `locale.rs` functions; a missing sidecar is not an issue).
   - `rail_only` is false on every issue except the one.

D. **`totals(plan, ctx) -> AirTotals`:** walk the `build_air_packs` output, per side by its groups: copies = children of the side's Sorties group; aircraft = `Plane` blocks; MCUs = every `MCU_*` block (copies' own included); check zones = `MCU_CheckZone` blocks. A plan that fails to build gives `AirTotals::default()`.

E. **`preview(plan, ctx) -> AirPreview`** (geometry only; no build): per eligible entry, an `AirLine` spawn → target, an `AirPoint` spawn and an `AirPoint` RTB (`entry` = pool index); per (side, area) two `AirRing`s, R (`plan.threshold.radius_m`, `inner = false`) and r_in (`inner = true`); per side with `schedule.enabled` and at least one eligible `Clock` or `Both` entry, `slot_times[side]` = one `SlotMark` per slot `first + k·period` (seconds) inside 0–3 h, `jitter_span_s = (jitter_steps − 1)·jitter_step·60`, `quiet_pct`. The spawn point must equal the lead's spawn in the built copy: target resolution, RTB, heading and spawn distance come from one helper shared with `assemble_side`. You may extract that helper from `assemble.rs` (and expose a pure spawn-point function from `place.rs` if one is needed) as long as `assemble_side`'s output does not change by a byte; that is the only change allowed in those two files.

F. **`src/frontlines.rs`:**
   - `FrontOptions.air_packs: Vec<MapAirPack>` (doc comment like its neighbours), `Vec::new()` in `Default`; the "enable at least one layer" check in `generate_front` also counts `air_packs`.
   - Stamp `opts.air_packs` with `stamp_rooted_packs` after the ground packs and before `aoi_border_group`, in the same style. An empty list must not change `next_id`.
   - `ImportedBaseMap.air_tasking_found: bool`. In `inspect_base_map`'s loop over top-level groups, a group whose name starts with `Air Tasking` sets it and is skipped, tested **before** `looks_like_fighter_pack` and `group_has_units`.
   - Remove the `#[allow(dead_code)] // P14` on `MapAirPack` if no warning results; otherwise keep it and say so.
G. **`src/ui/map.rs`:** add exactly one line, `air_packs: Vec::new(),`, to the `FrontOptions` literal in `generate_front_file`. Nothing else in the UI.

H. **Test plans, pinned in `export.rs` `mod tests` (`pub(crate)` if `baseline_tests.rs` needs them):**
   - `export_test_plan()`: one NATO HVAR `Clock` entry on NATO objective 1, one DPRK Il-10 `Both` entry on arrow 1; default settings; `seed = 7`; `ctx.date = (1950, 9, 15)`; the `base_map.Group` fixture's inputs (its `FrontOptions`, its arrow, its front as the UI would densify it for `AirContext.front`), plus fixed objective points. If the fixture's arrow turns out NATO by `point_north_of_front`, give the test context its own DPRK arrow instead and say so; never change the fixture.
   - `acceptance_test_plan()` (with its context): as the scope note says. 2 DPRK + 2 NATO objectives at fixed points on land; NATO AirAlert `Clock` obj 1, HVAR `Clock` obj 2, CloseSupport `Zone` obj 1, ArmedRecon `Both` obj 2; DPRK Yak `Zone` obj 1, Il-10 `Both` obj 2; one NATO entry's path placed within r_in of one DPRK area so `validate` gives exactly one D30 Warning and no Error; `seed = 7`; default settings; a Map date at which all six built-ins are valid.

I. **Tests**, with these exact names (scope note, "Tests"): `generate_front_with_empty_air_packs_matches_baseline`, `generate_front_with_disabled_air_plan_matches_baseline`, `export_plan_has_air_tasking_groups`, `export_is_deterministic`, `export_sets_the_ai_country`, `stamp_leaves_air_pack_formations_unchanged`, `inspect_export_keeps_armies_and_flags_air`, `inspect_baseline_has_no_air_tasking`, `load_then_generate_has_one_air_tasking_group`, `validate_clean_plan_has_no_issues`, `validate_flags_orphaned_target`, `validate_flags_out_of_date_sortie` (La-11 synthesized by changing a built-in copy's `Script` / `Model` to `la11` and loading it as a user template from a temp file), `validate_flags_d30_path_conflict`, `validate_d30_is_an_error_when_ai_counts`, `validate_orphaned_only_pool_issue_is_rail_only`, `validate_empty_pool_is_an_error`, `validate_budget_zero_is_an_error`, `validate_warns_above_totals_limits` (30 entries × 3 runs: one Warning per limit passed, no Error), `validate_warns_on_locale_id_conflict` (two temp user templates with sidecars), `validate_and_build_agree`, `totals_match_the_generated_pack` (both plans; recount independently of `totals`; pin the acceptance plan's numbers per side), `preview_counts_follow_the_plan`, `preview_spawn_matches_the_built_copy` (± 1 m, every eligible entry of the acceptance plan), `arrow_side_matches_generate_front` (for the test arrows, `point_north_of_front` on `ctx.front` and on the `dense_base` `generate_front` would build agree; the assertion message says which arrow disagrees), `acceptance_mission_has_zone_entries_and_trace`, and the ignored `write_p14_acceptance_mission`.
   - `export_plan_has_air_tasking_groups`: in the `generate_front` output: exactly one top-level `Air Tasking` group with the four sub-groups; `assert_links_resolve`; unique indexes; `walker_every_deactivated_out_has_a_rearm_or_is_single_use` over the stamped Air Tasking group.
   - `export_sets_the_ai_country`: every Plane in the pack is 602 or 503 by side; `place_air_sortie` / `assemble_side` output still carries the template's own country.
   - `generate_front_with_*_matches_baseline` and the inspect / load tests compare with the committed `base_map.Group` fixture (`baseline_tests.rs`); put them where the private fixture is reachable.
   - **0034 B, the acceptance mission:** `write_p14_acceptance_mission` (ignored) generates the base map with the acceptance plan's pack through `generate_front`, instruments it with `trace::instrument` (decision points, plus breadcrumbs on every `CLK s SLOT OPEN`, `ENii GO`, `ZNii CHANCE`, `ZNii MISS WAIT`, `ZNii FIRE`, `ZNii DENY WAIT`, `ZNii COOLDOWN`, `ZNii ARM`, `THsaa INNER`, `THsaa INNER CLOSE OUT`, and each copy's `START`, `ON STATION`, `EXTEND`, `COLD`, `Zone Out`, `LOSSES`, `HARD STOP`, `DONE HUB`, `DELETE DELAY`; select them by index so nothing else is instrumented), and writes `target/p14/P14_Acceptance_1.Group`, its locale sidecars and the trace sidecar, the way the probe writer does (read `probe.rs`; do not change it). Leave `plan.debug_subtitles` false. `acceptance_mission_has_zone_entries_and_trace` (not ignored) builds the same tree in memory: per side at least one `Zone` and one `Both` entry's `ZN` cell; every named breadcrumb present; `assert_links_resolve` on the traced tree; `validate(acceptance_test_plan())` has exactly one D30 Warning and no Error. Run the ignored writer once yourself and report the files it wrote.

Acceptance:
- Baseline first: `cargo build --offline` and `cargo clippy --offline` warnings (expected 81 / 306), `cargo test --offline` (expected 876 passed / 0 failed / 30 ignored). After: no new build or clippy warnings, 0 failed, every named test present (list them), the new passed / ignored counts, and the wall time the new non-ignored tests add (report any single test over 20 s).
- `existing_outputs_match_baseline`, `map_generate_without_air_matches_baseline` and `flown_probes_regenerate_byte_for_byte_except_flight2` (52 files) pass; nothing under `src/testdata/` or `handoff/p14-probes/` changes. No existing test changes.
- The totals of `export_test_plan()` and `acceptance_test_plan()`, per side (copies, aircraft, MCUs, check zones), in your report.

Files in scope: `src/airtask/export.rs` (new), `src/airtask/mod.rs` (`mod export;` and the four delegating bodies only), `src/airtask/threshold.rs` (the two allow lines only), `src/airtask/assemble.rs` and `src/airtask/place.rs` (the shared-helper extraction of E only, no output change), `src/frontlines.rs`, `src/ui/map.rs` (one line), `src/baseline_tests.rs` (new tests only; fixtures and existing tests unchanged). Do not touch `clock.rs`, `zone.rs`, `budget.rs`, `shell.rs`, `library.rs`, `timing.rs`, `testkit.rs`, `probe.rs`, `src/ui.rs`, `src/ui_tests.rs`, `src/trace.rs`, any document, fixture or probe file. If the work needs a change outside these, stop and report exactly what and why instead of making it.
</task>

<git_protocol>
Same-host mode. The branch `codex/p14-7` is checked out in this worktree and its working tree is clean when you start. Edit files in the working tree only. No git command that writes. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, or the scope note and the code disagree, stop and say what and why. Do not weaken a test to make it pass.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run build, clippy, test and the byte check, and read the counts. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate (`rustfmt` on the files you changed is fine, but not on `src/ui/map.rs` or `src/frontlines.rs` if it would reformat lines you did not change); do not run MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now (`clock.rs`, `zone.rs`, `assemble.rs`). No refactor outside the change. Messages a user reads are plain words, no MCU names.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Where this prompt and the code disagree, follow the code and report each difference.
</missing_context_gating>

<structured_output_contract>
Return: 1. Summary (five lines). 2. Command results (baseline and final warnings and tests, byte checks, added test time). 3. Files changed. 4. Tests added, by name. 5. The D36 totals of both plans, per side. 6. The acceptance writer's output files. 7. Choices, differences from the scope note, and residual risks.
</structured_output_contract>
