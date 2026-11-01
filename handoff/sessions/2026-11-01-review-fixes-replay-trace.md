# Review fixes: Replay and trace builds

Worktree: `codex/review-fixes-replay-trace`, based on `6f02211`. Files only; Claude reviews the diff and commits. No git state changes, frozen input/output edits, new dependencies, or ignored writer runs.

## Findings and tests

UI behavior tests are in `src/ui_tests.rs`; core tests are adjacent to their implementation. The 12 initial regressions failed on the original production code. L's restored all-base-file comparison passed before production changes; its output was already correct and its guard was missing.

| Finding | Changed files | Regression tests | Fix |
|---|---|---|---|
| 1. Sessions merged | `src/missionlog.rs`, `src/ui/replay.rs`, `src/ui_tests.rs` | `review_replay_selects_one_session_newest_first`, `review_parser_refuses_mixed_sessions` | Timestamp grouping, newest initial selection, numeric chunk order, picker only for multiple sessions; core refuses mixed sessions. |
| 2. Trace overwrites mission | `src/trace.rs`, `src/ui.rs`, `src/ui/map.rs`, `src/ui_tests.rs` | `review_sidecar_path_never_aliases_its_input`, `review_trace_save_refuses_non_group_paths_before_writing` | Shared pre-write validation requires `.Group`, case insensitive; derived sidecar paths are distinct. Template, Map, Air, and Fighter save paths are exercised, including existing-file protection. |
| 3. L byte guard removed | `src/ui_tests.rs` | Restored `standalone_l_build_matches_a3` | File-set check plus byte equality of all eight base files; failures name the first differing byte and lengths. |
| 4. Missing evidence hidden | `src/missionlog.rs`, `src/ui/replay.rs`, `src/ui_tests.rs` | `review_replay_marks_discarded_lines_incomplete`, `review_replay_refuses_logs_without_valid_records`, `review_replay_empty_trace_has_no_breadcrumb_coverage` | Incomplete notices include count and first three file:line locations; no valid records is an error; empty traces explicitly have no coverage. |
| 5. Grids collide | `src/trace.rs`, `src/ui.rs`, `src/ui_tests.rs` | `review_trace_bands_are_disjoint`; existing `standalone_l_build_matches_a3` | Map retains its grid; Group adds 40 km X, Air adds 80 km X. Nonempty sidecars record the band. L's A3 comparison adjusts only breadcrumb X by 80 km. |
| 6. Weak stale check | `src/trace.rs`, `src/missionlog.rs`, `src/ui.rs`, `src/ui/map.rs`, `src/ui/replay.rs`, `src/ui_tests.rs` | `review_replay_refuses_a_changed_timer`, `review_replay_legacy_trace_shows_cannot_verify`, `review_trace_metadata_round_trips_and_checks_exact_bytes`; extended `replay_cli_writes_report` | Nonempty Generate sidecars hash exact mission text with FNV-1a 64. Changed timers are refused in UI and CLI; older sidecars show cannot-verify notes. Empty traces are the unresolved exception below. |
| 7. Run uses stale scan | `src/ui/replay.rs`, `src/ui_tests.rs` | `review_replay_rescans_after_picking_an_empty_folder` | Run stays available with a selected folder and rescans on every run, including a new `[1]` chunk. |
| 8. Fallback offset | `src/missionlog.rs`, `src/ui/replay.rs`, `src/airtask/probe.rs` | `review_fallback_tick_rate_uses_three_tick_offset` | No trace or insufficient calibration uses rate 50 and offset 3; `T:503` is 10 s. Existing fallback-time assertions change by 0.06 s; calibrated expectations stay unchanged. |
| 9. Index overflow | `src/trace.rs`, `src/ui.rs`, `src/ui/air.rs`, `src/ui/airfield.rs`, `src/ui/builder.rs`, `src/ui/map.rs`; Result handling in `src/missionlog.rs`, `src/airtask/export.rs`, `src/airtask/probe.rs`, `src/ui_tests.rs` | `review_trace_allocation_overflow_is_atomic`, `review_trace_overflow_shows_error_and_writes_nothing` | Whole allocation checked before mutation; all Generate callers stop with an error before saving. Tests include MCU index `i32::MAX`, a near-limit cursor, and the five-index spawn carrier. |

## Output preservation

- Unchanged baseline assertions: `existing_outputs_match_baseline`, `trace_off_changes_nothing`, `map_generate_without_air_matches_baseline`, `trace_checkbox_off_output_matches_baseline`, `map_generate_with_include_unticked_matches_baseline`, and the empty/disabled Air export baseline tests.
- Standalone ordinary Generate: `air_tasking_standalone_matches_public_builder_and_keeps_seed` still compares the public builder and repeat file bytes.
- Restored `standalone_l_build_matches_a3` pins every frozen base file, including the empty trace sidecar. Existing flown-probe byte-comparison assertions and frozen parse/link checks were not relaxed.
- An additional local before/after audit compared a nonempty Map trace `.Group` and all six language files with files captured before production edits. All seven were byte-identical. The temporary audit assertion was removed; it introduced no golden file or dependence on local artifacts.
- The Air A3 comparison retains its existing checks, with only the requested fixed breadcrumb X-band translation. No new Air golden files.

## Validation

Logs and the temporary before-fix snapshot are under ignored `target/review-validation/` and `target/review-before/`.

- `cargo build`: passed; `Finished dev profile [unoptimized + debuginfo] target(s) in 29.69s`. Existing 81 build warnings.
- `cargo clippy --all-targets`: passed; `Finished dev profile [unoptimized + debuginfo] target(s) in 1m 01s`. Binary 306 warnings, matching the recorded baseline; test target 301 (237 duplicate diagnostics). No diagnostic points at an added or changed line.
- Final full `cargo test`: `test result: ok. 1117 passed; 0 failed; 41 ignored; 0 measured; 0 filtered out; finished in 191.91s`. The ignored tests were not run. `git diff --check` also passed; protected fixture directories have no diff.

## Still in flight

The owner's literal requirements conflict for the 41-byte empty L base trace: finding 3 freezes every base file's bytes, while findings 5/6 ask for extra metadata. Clarification was requested; pending a response, every base file remains byte-identical. Empty Generate traces therefore have no band/fingerprint metadata and Replay marks them unverified with no coverage. See proposed decision 0057. No frozen file was modified and the byte test was not weakened.

Claude reviews and commits the resulting diff. The separate Air behavior work on `codex/airtask-0053-0055` is outside this change.
