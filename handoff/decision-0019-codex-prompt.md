# Delegation prompt for decision 0019 (every AI air formation at density 1, Safe)

One job, in the worktree `distracted-taussig-b9259d` on `claude/formation-safe-0019` (cut from `codex/p14-2d` at `e36ebed`, plus the accepted decision record and this prompt). Sent with `codex_start`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1200. Model: the config default.

---

<task>
Build decision 0019: every AI air formation the app writes or places flies at `FormationDensity = 1` (Safe) instead of 0 (Dense). Three parts, A, B and C. Ground (vehicle) formations do not change.

Read first, in this order:
1. `docs/decisions/0019-air-formations-fly-at-safe-density.md` (the decision; this prompt makes it exact).
2. `handoff/P14-probe-results.md`, section "Flight 2D" (the evidence).
3. `CLAUDE.md`, `.cursorrules`, `docs/decisions/0015-air-tasking-sorties-keep-ai-rtb-decision-off.md` (the precedent for A), `docs/decisions/0018-zone-out-only-after-an-enemy-is-seen.md` (the precedent for freezing a flown probe whose builder changes on purpose).
4. Code: `src/template.rs` (`FormationPreset`, `air_form`, `ground_form`, `AIR_FORMATIONS`, `formation_density`, and where `FormationDensity` is written, about line 3730), `src/airtask/place.rs` (where each placed copy's Planes get `AiRTBDecision`, about line 115), `src/airtask/probe.rs` (`Flight::is_frozen`, `regenerates_flown_files`, the `include_str!` of the HVAR and Yak templates, `flown_probes_regenerate_byte_for_byte_except_flight2` and the flight 2 frozen-file test), `src/airtask/library.rs` (the six built-ins, `include_str!`).

What to build:

A. **Air Tasking.** When a copy is placed, set `FormationDensity = 1` on every `MCU_CMD_Formation` in it, whatever its source template says, in the same pass that sets `AiRTBDecision`. Use a named constant (e.g. in `template.rs` next to the air presets, shared with C), not a bare `1`. No MCU is added or removed. Tests: a copy placed from a template whose Formation commands say 0, and one that says 2, both come out 1; the copy's MCU count and every other property of the Formation commands are unchanged.

B. **The six built-in sorties** in `TemplateExamples/Historical1950/` (`*.Group`, nine `MCU_CMD_Formation` blocks in all): change `FormationDensity = 0;` to `FormationDensity = 1;` in each Formation block and nothing else. The files are CRLF and stored byte for byte (`.gitattributes` is `* -text`): keep every other byte, including line endings. Check with `git diff --stat` and a byte count that each file changes only on those lines. Test: every Formation command in the six built-ins says 1 (a test that reads the files through `library.rs`'s built-in list).

C. **Template Builder.** `air_form` gives density 1, so every air formation Template Builder writes says `FormationDensity = 1`. `formation_density` for an air formation id not in `AIR_FORMATIONS` also gives 1 (today it falls back to 0); ground ids keep today's fallback. Update every existing test literal that expects an air formation's density to be 0, and report each by file and test name. If a golden output file under `src/testdata/` changes, update it only where the sole difference is an air formation's `FormationDensity`, and report each file; anything else changing is a stop-and-report.

D. **Flown probe files stay true to what was flown.** The 56 flown probe files under `handoff/p14-probes/` were built from the Dense templates.
- For flights whose builder reads a built-in template straight from `TemplateExamples/Historical1950/` (not through placement), add byte copies of the templates as they were before this change in `src/testdata/p14_probe_inputs/historical1950_pre0019/` (take them from `git show e36ebed:<path>`; copy only the files a flown probe reads) and point those flights at the copies, so they keep regenerating byte for byte.
- For flights whose copies go through Air Tasking's placement, A changes their builder on purpose. Mark each such flight frozen as decision 0018 did for flight 2 (`is_frozen`), keep their files checked for parse and resolved links the way flight 2's frozen files are, and lower the byte count in `flown_probes_regenerate_byte_for_byte_except_flight2` with a comment naming the flights. List each flight and why.
- Prefer the frozen-input route wherever it keeps a flight regenerating; freeze only what A must change. Never edit a file under `handoff/p14-probes/`.

E. **Every other path that writes an air `MCU_CMD_Formation`** gives density 1. A first pass (job `20261016022606-db6a3723`) found two that copy a loaded template and keep its density: `src/bombers.rs` (`link_bomber_plans_with`, about line 285; e.g. `TemplateExamples/ImprovedCooldownLogic.Group` has an aircraft Formation, type 23, density 0) and `src/recon.rs` (about line 600). Add one shared helper in `src/template.rs` that sets the shared constant on every `MCU_CMD_Formation` whose `Objects` name a Plane's entity, and call it from A's placement pass, from bombers and from recon on the copies they write (never on the loaded source). A Formation command for vehicles keeps its value. Tests for bombers and recon: a loaded template with an aircraft Formation at 0 comes out 1; one with a vehicle Formation keeps its value. Report every path found, changed or not (Fighter Pack goes through Template Builder: confirm with a test or a code reference). A second pass (job `20261016023243-14995624`) found more paths that copy a loaded group into the output unchanged: Airfield Export (`clean_airfield`, `src/airfield.rs` about line 105), recon's `combine_placed_packs`, and the map reference groups and rooted packs (`src/frontlines.rs` about lines 2186 and 2207). Call the helper on the copy each of them writes, with a test each. **If you find any further path, call the helper there too, add a test, and report it; do not stop for it.** Do not put the helper inside the serializer or the parser: the probes, fixtures and byte-for-byte tests serialize trees that must keep their values.

Acceptance:
- Before editing, record the baseline: `cargo build --offline` and `cargo clippy --offline` warning counts, `cargo test --offline` passed / failed / ignored. After: build and clippy warnings not above the baseline; new code adds none.
- `cargo test --offline`: 0 failed; passed = baseline + the new tests (report changed tests separately).
- `git diff --stat -- handoff/` is empty. The six template diffs touch only `FormationDensity` lines (nine in all).
- No `MCU_CMD_Formation` for aircraft anywhere in the app's output says `FormationDensity = 0`: show how you checked (tests and a search).

Files in scope: `TemplateExamples/Historical1950/*.Group` (the `FormationDensity` lines only), `src/template.rs`, `src/airtask/place.rs`, `src/airtask/probe.rs`, `src/bombers.rs`, `src/recon.rs`, `src/airfield.rs`, `src/frontlines.rs`, and any other `src/` file where E finds a path (a helper call and its test only), new files under `src/testdata/p14_probe_inputs/historical1950_pre0019/`, golden files under `src/testdata/` only as C allows, and test literals in other `src/` files only where they assert an air formation's density (report each).

Do not touch: `src/airtask/shell.rs`, `library.rs` (except a test, if that is where B's test belongs), `testkit.rs`, `budget.rs`, `assemble.rs`, `mod.rs`, `src/trace.rs`, any other file under `TemplateExamples/`, `handoff/` (all of it), `docs/`, `tools/`, `STATE.md`, `USER_MANUAL.md`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, and the game folder. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `claude/formation-safe-0019` is checked out in this worktree and clean. Edit files in the working tree only. No git command that writes. `git status`, `git diff`, `git log` and `git show` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not weaken a test to make it pass; report the value and the reason.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run `cargo build --offline`, `cargo clippy --offline`, `cargo test --offline` and the checks above, and read the counts from their output. Fix and re-run until every line holds. Do not run `cargo fmt` or rustfmt on any file, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now: named constants, shared helpers, no flag arguments, no behaviour keyed on a bare number, failures that name the template, flight or missing part. No refactor, rename or reformat of code outside the change.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Where this prompt and the code disagree, follow the code for facts and report each difference.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary, five lines at most.
2. Command results: baseline and final build and clippy warnings, tests passed / failed / ignored.
3. Every file changed, one line each on why.
4. Tests added, and existing tests changed (with the old and new literal).
5. Part D: each flown probe flight, and whether it now reads frozen inputs, was frozen, or was unaffected, with the reason; the new byte count.
6. Part E: every path that writes an air Formation command, and what it gives now.
7. Choices you made where this prompt gave no exact shape, and residual risks.
</structured_output_contract>
