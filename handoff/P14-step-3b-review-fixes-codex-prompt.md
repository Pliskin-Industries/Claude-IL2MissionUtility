# Delegation prompt: step 3b review fixes

Second job in the worktree `Claude-IL2MissionUtility-3b` on `codex/p14-3b`. The first job (`20261014220542-8d6620d0`) left its work uncommitted in this tree; the orchestrator re-ran its checks (627 passed, 0 failed, 21 ignored; 69 build / 251 clippy warnings; the 44 flown probe files byte-identical) and ran a standards review and a spec review. These are the confirmed findings. Sent with `codex_start`, `model` = `gpt-6-astra`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1500.

---

<task>
The working tree holds your previous job's uncommitted step 3b work (8 files) plus this prompt file, which the orchestrator wrote. Both are expected; continue from them. `handoff/P14-step-3b-codex-prompt.md` still applies in full (scope, do-not-touch, acceptance, style), except where this prompt widens the scope.

Fix each of the following findings. Each was confirmed against the code.

1. **The shell's Delete chain must not depend on a bundled example file.** `shell.rs` `delete_prototypes()` parses `TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group` at run time and `expect()`s blocks named "DELETE DELAY" and "Trigger Delete". If that file is regenerated under decision 0014 (Activate mode writes no Delete), `wrap_sortie` panics. Build the two blocks with `template::timer` and `template::mcu("MCU_Delete", …)`, as `generate_template` does for Spawn mode (that Delete flew in flight 1L). Use the timer's wait from `template.rs` `DELETE_WAIT`: you may change only that constant's visibility to `pub(crate)` in `src/template.rs`; nothing else in that file. Remove `delete_prototypes()` and the `if node.block_type == "MCU_Timer"` / `"0.50"` branch. If the guard test that compares block key orders with shipped shapes fails on the result, stop and report.

2. **Flight 2 cell T-l1 must not start over an airfield.** It is centred on K14 itself (`centre_km (0.0, 0.0)`), at 150 m with Fuel 0, so the aircraft may glide onto K14's runway (event 5) and never crash. Move the cell so that the start point is over land, at least 15 km from K14 and from every other airfield whose position the repository holds (say which source you used; if it holds none but K14, say so), clear of the other flight 2 cells, and have its cruise waypoint lead away from airfields. Update the test that asserts the start coordinates, the schedule and the run sheet. Flight 2 has not flown; its files are not frozen.

3. **Tests state their own literals** (`<action_safety>`). In `library.rs` tests, `inspect_sortie_accepts_cleanup_without_delete`, the invalid-chain test near line 1227 and `inspect_sortie_refuses_on_killed_repeat_events` assert `format!("{ACTIVATE_MODE}. {REGENERATE_TEMPLATE}")` and the like. Spell the expected message out, as `inspect_sortie_refuses_spawn_mode` does.

4. **No edits outside the change.** In `library.rs` `shape_cases`, restore the unchanged `if !formation` block as it was at HEAD (the `seat.orders` / `.retain(...)` line break) and the deleted comment "Template Builder can share one RTB among leads in the same bucket", except where S5/S6 removal itself requires a change.

5. **Stale plan text.** `handoff/P14-air-tasking.md` still contradicts the code in three places outside the sections the first prompt named: §5 (about line 789, `enum LossesImpl { Type13, Latch }`), the step 0 "0d" table's u/v row (about line 1189, "Spawn-mode templates are accepted"), and step 2 (about line 1684, S5 and S6 accepted, "SPAWN UNITS"). Make each match decision 0017. Search the whole file for any other statement that Spawn-mode templates, S5/S6, `Latch` or event 4 losses are accepted or used by Air Tasking, and fix those too. Do not rewrite the plan's history sections (§11 revision log), which record what was true then.

6. **Duplicated test setup.** `library.rs` tests repeat `let mut opts = options(TestedShape::S1); opts.bring_up = BringUp::Spawn; opts.allow_multiple_spawns = repeat;` six times: one helper.

7. **The load writer duplicates the probe writer.** `write_p14_load_probe` re-implements `write_probe`'s file loop and has drifted from it (no "WROTE" line, `expect` on the sidecar where `write_probe` reports the error). `build_load_probe` builds on `Probe::new(Flight::Two)` and renames the root. Give the load file its own identity and write it through the same path as every probe, so its reporting and error handling are the same. `write_p14_probe_2` calls another `#[test]` function directly: share a plain helper instead. The flown probes must still regenerate byte for byte.

8. **Bare numbers in new code.** `probe.rs`: `build_side(side, 2, …)`, `build_side(AirSide::Nato, 1, …)`, `register(1, …)`, `census_delay(…, 1)`. Name what each number is (the budget N, the copy order), as named constants or fields of the cell tables. In tests, `WalkerInput { until: 200 }` uses raw ticks; use `ticks(...)` as the other tests do.

9. **`SortieMode` has one variant.** If nothing outside the files in scope reads `SortieMode` or `SortieShape.mode`, remove both; otherwise leave them and report who reads them. Leave `LossesImpl` (one variant) as it is: `mod.rs` changes beyond removing `Latch` are not in scope.

10. **Explain the allocator change.** `wrap_sortie` now starts `alloc` at `root.max_index() + 1` rather than the stripped copy's. Add a one-line comment saying why (indexes of the removed template Delete chain are not reused), and replace the weak `assert_ne!(Some(f.ids.delete_timer), shape.delete_timer)` with a test that no index the shell adds equals any index of the removed chain.

Not to change (reviewed and accepted): the reworded shape-mismatch error, `strip` removing `OnReports`, the T-s subtitle on `SNjj DELETE DELAY` (a Delete is a sink), the corrected `wrap_sortie` signature line and "every deletion" in §3.3.

Files in scope: as in the first prompt, plus `src/template.rs` (the visibility of `DELETE_WAIT` only). Do not touch `docs/decisions/`.

Acceptance: every acceptance line of the first prompt still holds (build at most 69 warnings, clippy at most 251, tests 0 failed with no fewer than 627 passed less any test this prompt removes, probe writers run, 44 flown probe files byte-identical, mission builder check exits 0 with `--no-resave`, `P14 G0` absent, step 1 baselines unchanged), and each of the ten findings is fixed or reported as not fixable with the reason.
</task>

<git_protocol>
Same-host mode. Edit files in the working tree only. No git command that writes. `git status`, `git diff` and `git log` are allowed.
</git_protocol>

<verification_loop>
Run `cargo build --offline`, `cargo clippy --offline`, `cargo test --offline`, `cargo test --offline write_p14_ -- --ignored`, the byte comparison against `handoff/p14-probes/*/` and the mission builder check (`--no-resave` only). Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<structured_output_contract>
Return: (1) summary, five lines at most; (2) command results and the byte comparison; (3) for each finding 1–10: what you changed, file and function, or why not; (4) T-l1's new start point, its distance to K14 and to the nearest other airfield, and the source of the airfield positions; (5) tests added, removed or renamed; (6) residual risks.
</structured_output_contract>
