# Delegation prompt for the P14 step 7 review fixes

Sent with `codex_start`, model `gpt-6-astra`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1500, in the worktree `optimistic-hugle-8e0d98` on `codex/p14-7` (the step 7 build is uncommitted in the working tree). Findings come from the code-review skill (standards and spec sub-agents), triaged by Claude.

---

<task>
Apply the review fixes below to the uncommitted P14 step 7 work in this working tree (branch `codex/p14-7`). The step's full contract is `handoff/P14-step-7-codex-prompt.md` (after "---"); it still holds, except where a fix below changes it. The current edits are yours from earlier sessions: keep them and change only what the fixes need.

Hard rule for every fix: Generate output must not change by a byte (`existing_outputs_match_baseline`, `map_generate_without_air_matches_baseline`, `flown_probes_regenerate_byte_for_byte_except_flight2` with 52 files). No builder's MCU names, counts or wiring change. No existing test outside step 7's new ones changes.

Fixes:

1. **One owner per input rule, named constants.** The runs range (1..=6, D12), weight range (1..=10, D15) and budget range (1..=6, U20) each get one `pub(crate) const` (a `RangeInclusive<u8>`) in the module that owns the rule: runs in `assemble.rs`, weight in `clock.rs`, budget in `budget.rs`. The builder and `export.rs` both use that constant, and the builders' messages stay byte-identical. Name the 3-hour preview window as a constant in `export.rs`.

2. **Check only what is built.** In `export.rs` `entries()`, check runs and weight only for entries that will be built: after the disabled, out-of-date and orphaned skips. Check weight only for `Clock` and `Both` entries, as `clock::build_side` does. A skipped entry with bad runs must not block Generate.

3. **SingleSlot budget.** Under `BudgetImpl::SingleSlot`, `budget::build_side` forces n = 1. `validate` must not raise the budget Error in that case. Add a test for it: `validate_single_slot_ignores_budget_setting`.

4. **`validate` must not hide clock or zone errors.** Today an earlier Error returns before `clock::validate_schedule` and `zone::validate_zone` run (export.rs ≈243). Restructure so that every side's own checks always run when it has eligible entries:
   - assemble the side, then run its clock and zone validators;
   - build the rest (clock, zone, threshold, budget, emit) only when no blocking Error exists anywhere.
   - Totals warnings stay "only when the plan builds".
   - Add a test: `validate_reports_schedule_error_with_budget_error`: budget 0 on a side plus an invalid period, and both Errors are reported.

5. **Split `evaluate`.** Extract the per-side build into a named function that returns a small struct (`logic`, `sorties`) instead of a `Vec` read back with `remove(1)` / `remove(0)`. Keep one evaluation per public call.

6. **One resolution in `assemble_side`, one speed source.**
   - Call `resolve_placement` once (at the orphan check) and use its side and placement. Remove the second `resolve` and the `.expect`.
   - Preview, D30 and assembly must use the same speed for the spawn distance. Use the library item's `wp_speed_kmh` (`AirSortie.wp_speed_kmh`, what the user saw) in `assemble_side` too, in place of the re-inspected `shape.wp_speed_kmh`. For the built-ins the two are equal, so output stays byte-identical. The byte checks prove it; if they do not pass, stop and report.

7. **Side mappings.**
   - Add `pub fn other(self) -> AirSide` to `impl AirSide` in `mod.rs`. That one method is the only `mod.rs` change besides fix 9.
   - Add `pub(crate) fn ai_country(side: AirSide) -> i32` next to the constants in `threshold.rs`.
   - Use both in `export.rs`. In `assemble.rs`, replace its own `"DPRK"` / `"NATO"` if/else (≈108-112) with `side.label()`.

8. **Group names in one place.**
   - Add `pub const AIR_TASKING_GROUP: &str = "Air Tasking";` beside `MapAirPack` in `frontlines.rs`.
   - Add one helper in `assemble.rs` that builds the side tree names `Air Tasking {label} Logic` / `Sorties`. `assemble_side` and `count_totals` both use it.
   - `export.rs` uses the constant for the root.
   - `inspect_base_map` now matches the name **exactly** (`== AIR_TASKING_GROUP`), not by prefix. Decision 0035 fixed the output shape to one group with that name, and an exact match no longer swallows a user's own group named "Air Tasking …".
   - Add a test: `inspect_keeps_a_user_group_whose_name_only_starts_with_air_tasking`.

9. **`mod.rs` delegates.** Rename `_plan` / `_ctx` to `plan` / `ctx` in the four delegating functions, and replace their stale trailing `// step 7` comments with one short doc line each. The signatures stay as they are.

10. **Locale check.**
    - Add `pub fn ids(&self) -> impl Iterator<Item = i32> + '_` to `LocaleTable` in `src/locale.rs` (keys in order). That is the only `locale.rs` change.
    - Use it in `locale_issues` in place of re-parsing `serialize_locale` output, and drop the comment claiming the type deliberately has no iterator.
    - Check only templates of entries that will be built (eligible), not every enabled entry.

11. **Preview guard.** Drop the guard that re-derives part of the clock's period rule (the `+ 10.0`, ≈441-452). Skip a side's marks only when `first` or `period` is not finite, `first < 0`, or `period <= 0`; that is enough to keep the loop finite. `validate` reports the real schedule rule.

12. **Acceptance plan geometry (step 11 flight).**
    - Remove `heading_override = Some(180.0)` from `acceptance_test_plan()`. It spawns the NATO AirAlert about 100 km inside DPRK territory.
    - Instead, place the objectives so that one NATO entry's natural path (default heading: spawn → target → RTB) passes within r_in of one DPRK area. Every spawn should stay on its own side of the front (`point_north_of_front` on the test front: NATO spawns south, DPRK spawns north). Assert that in `acceptance_mission_has_zone_entries_and_trace`.
    - Keep exactly one D30 Warning and no Error.
    - Re-pin the acceptance totals if they change, and re-run the ignored writer once.

13. **Test helpers.**
    - Use `shell::test_support::named` in place of the local `named` in `export.rs` tests.
    - One test helper derives the test `AirContext.front` the way the UI does (`frontlines::preview_front_xz` for the timeline mark, densified at 4000 m, then `apply_salients`, falling back to the dense line when the composite has fewer than two points; see `git show codex/p14-8:src/ui/air.rs`, `air_front`). Both `export.rs` tests and `baseline_tests.rs` use it, in place of today's two different derivations. Put it in one place, reachable from both.
    - `baseline_tests.rs` `arrow_plan` must not re-implement `resolve`'s arrow-side rule: use `point_north_of_front` on that helper's front, or the export test plan.
    - If `missionlog.rs` has a `pub(crate)` temp-dir guard that fits, reuse it for `TempTemplates`; otherwise leave `TempTemplates` as it is.

Left as they are (do not change): the `error.starts_with("Air Tasking ")` prefixing of builder errors; `#[allow(dead_code)]` on `air_tasking_found` (step 9 reads it).

Files in scope: `src/airtask/{export,mod,assemble,place,threshold,clock,budget}.rs` (in `clock.rs` and `budget.rs` only the constant of fix 1 and its use), `src/frontlines.rs`, `src/locale.rs` (fix 10 only), `src/baseline_tests.rs`. Not: `zone.rs`, `shell.rs`, `library.rs`, `testkit.rs`, `probe.rs`, `ui.rs`, `ui/*.rs` (beyond the existing one line), `ui_tests.rs`, `trace.rs`, documents, fixtures and probe files. If a fix needs more than this, stop and report.
</task>

<git_protocol>Same-host mode: edit files only, no git command that writes.</git_protocol>
<completeness_contract>Apply every fix. Do not weaken a test. If a fix cannot be done as written (for example fix 6 changes output), stop that fix and say what and why.</completeness_contract>
<verification_loop>Run `cargo build --offline` and `cargo clippy --offline` (warnings must stay at 81 / 306 or fewer) and `cargo test --offline` (last run: 901 passed / 0 failed / 31 ignored; report new counts). Run the three byte checks named above and `git diff --check`. Run the ignored `write_p14_acceptance_mission` once. No `cargo fmt` on the whole crate.</verification_loop>
<action_safety>Smallest change per fix, in the existing style. No other refactors.</action_safety>
<structured_output_contract>Return: 1. Summary (five lines). 2. Command results. 3. Per fix 1–13: done / not done, with file:line. 4. Tests added. 5. The acceptance plan's D36 totals per side (and whether they changed). 6. Anything left, with reasons.</structured_output_contract>
