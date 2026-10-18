<task>
Fix the findings of the review of the three decision fixes (commits `06b05aa`, `306e2ea`, `cb3154a`). `generate_template`'s output must not change: the Template Builder baselines stay byte for byte as they are now. Branch `codex/fixes-review` is checked out and the working tree is clean.

Read first: `docs/decisions/0012…`, `0013…`, `0014…`, `0015…`, `0016…` (0016 is still open: do not implement it), `handoff/P14-probe-results.md` sections "Flight 1H" and "Flight 1L", then the code named below.

A. **Air Tasking's sortie check and placement** (`src/airtask/library.rs`, `src/airtask/place.rs`)
1. Decision 0014 says the sortie-shape check changes with it. Today the check still requires the template's Delete chain, so every Activate-mode template the current Template Builder writes is refused, and the test helper `generate()` grafts a Plane-index Delete onto generated output so the shape tests pass. Change the check to accept a template **with or without** the Delete chain: with one, the rules stay as they are (exactly one `MCU_Delete`, one timer into it, `DELAYED END` targeting exactly the Deactivate and that timer, Objects equal to all entities or all Plane blocks, never partial or mixed); without one, `DELAYED END` targets exactly the Deactivate. Remove the grafting from the test helper so the shape tests run on what `generate_template` really writes, and keep tests for the six built-in sorties (which have the chain).
2. A placed copy is a one-shot sortie, so its Delete must work. `place_air_sortie` rewrites the placed copy's `Trigger Delete` `Objects` to the copy's entity indexes when the template has the chain with Plane-block indexes, and leaves a copy without the chain alone (the sortie shell adds its own clean-up in a later step). The template file is never changed. Tests first: for each built-in sortie the placed copy's Delete lists exactly its entities, in plane order.
3. `place.rs` validation and mutation use two definitions of "every plane" (`shape.plane_objects` against block type). Use one.
4. The refusal messages give no remedy. Each says what the user can do (for example: load the template in Template Builder and press Generate again). Messages are named constants like their siblings, and no user-facing text mentions a decision number.

B. **Exclusive Activation** (`src/bombers.rs`): `unit_is_covered` counts a Delete that lists unit block indexes as clean-up coverage. Such a Delete does nothing in the game. Keep the coverage logic (the Deactivate covers the units), but where a plan's clean-up Delete lists block indexes, add a warning in the same channel as the existing clean-up warnings that says the Delete will not remove the units and how to fix it. Generated output does not change.

C. **Template Builder messages and manual** (`src/template.rs`, `USER_MANUAL.md`)
5. The warning shown when an older template's aircraft death links are regenerated as event 13: one warning for the template, not one per aircraft, saying what changes for the player (the next wave now comes when the whole group has been destroyed) and no decision number.
6. Loading an older Activate-mode template that has the Delete chain: it is dropped on Generate. Add one note in the same channel that says so and why it is harmless (the units are switched off as before).
7. `USER_MANUAL.md`, the Events list: add that for aircraft "On Plane Destroyed" (4) does not fire in the game and "On Killed" (13) does. One sentence, plain words.

D. **Code quality**
8. `EntityEvent::default_for(_kind: UnitKind)` ignores its argument. Replace it with a named constant for the death event and remove the now-unused `kind` computation at its call sites; keep `EventHook::default_for` only if it still has a reason to take a kind.
9. `AI_RTB_DECISION_OFF: &str = "0"` in `place.rs` duplicates `AiRtbDecision::Disabled.property()` in `src/airtask/probe.rs`. Move the type to a shared place under `src/airtask/` and use it in both.
10. "OnEvent of type T targeting id" is parsed in `flights.rs`, `library.rs`, `read_events` and the testkit. One helper beside `attach_event`, used by all four.
11. Rename `legacy_objects` to say what it holds (the Plane block indexes).
12. "Activate has no Delete" is decided in the generator by `opts.bring_up` and again in a test helper by looking for the name `Activate Units`. The helper takes the mode from the options.
13. Tests that cannot fail or that derive their expectation: in `place.rs`, `assert_eq!(source, before)` where the function never receives `source`, and a tautological `changed` count; in `template.rs`, `2 * pack.count_block_type("MCU_Delete")`; the `contains` assertions on warning and refusal texts (assert the exact text). Fix each so it states its own expected value and would fail if the behaviour broke. The test near the old `template.rs:7021` was switched from an explicit `OnPlaneDestroyed` hook to the default: restore a test that an explicitly chosen `OnPlaneDestroyed` hook is written as type 4.
14. Split `activate_templates_with_or_without_legacy_delete_load_and_link` into one test per reader (loader, Exclusive, Recon, link integrity).
15. Run `rustfmt` on the lines you touch only (not the whole file or crate).

Acceptance:
- `cargo test --offline`: 0 failed, no fewer tests than before. `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- The Template Builder and base-map baselines under `src/testdata/p14_baseline/` are byte for byte unchanged, **except** any baseline that holds copies placed by `place_air_sortie`: there the only change allowed is the `Objects` list of a placed copy's `Trigger Delete` (item 2). Report each such file and line.
- `cargo test --offline write_p14_probe_ -- --ignored` passes. Flights 0 to 1L generate byte-for-byte the same files as before (compare against `handoff/p14-probes/*/`). Flight 1M (not yet flown) may change only in its `place_air_sortie` row; report what changed there.
- Nothing under `src/testdata/p14_probe_inputs/` or `TemplateExamples/` is edited.

Files in scope: `src/template.rs`, `src/flights.rs`, `src/bombers.rs`, `src/airtask/*.rs`, `USER_MANUAL.md`, and baselines only as stated above.
</task>

<git_protocol>
Same-host mode. Edit files in the working tree only. No git command that writes. `git status` and `git diff` are allowed. If the working tree holds changes when you start, stop and report.
</git_protocol>

<completeness_contract>
Fix every item. If one cannot be done as written, leave it and say exactly why.
</completeness_contract>

<verification_loop>
Run the three cargo commands, the ignored probe writers and the comparisons before you report. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<structured_output_contract>
Return: (1) one line per item (1 to 15): done, or not done and why; (2) command results and the comparisons; (3) every user-facing text you wrote or changed, quoted in full; (4) every baseline or probe file that changed, with the lines; (5) anything else you noticed and did not change.
</structured_output_contract>
