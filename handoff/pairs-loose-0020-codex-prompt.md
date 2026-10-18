# Delegation prompt: decisions 0020 A and 0022 A2 (Pairs fly Loose, except a flight flying a Cover order)

One job, in the worktree `distracted-taussig-b9259d` on `claude/pairs-loose-0020` (cut from `claude/p14-flight-2h` at `12c14f1`, plus this prompt, committed). Sent with `codex_start`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1200. Model: the config default.

---

<task>
Change what the app writes for AI aircraft formation density, per decisions 0020 (option A) and 0022 (option A2): **every aircraft Formation command of type Pairs (`FormationType = 19`) is written at `FormationDensity = 2` (Loose), except when the flight it commands also flies an `MCU_CMD_Cover`, which keeps `FormationDensity = 1` (Safe). Every other aircraft formation type stays at 1 (Safe), as decision 0019 has it. Vehicle formations do not change.** This is a workaround for a game bug (decision 0021) and must be marked as such in the code.

Read first, in this order:
1. `docs/decisions/0019-air-formations-fly-at-safe-density.md`, `0020-pairs-formations-after-flight-2e.md`, `0021-formation-collisions-are-a-game-bug.md`, `0022-f51-armed-recon-at-loose.md`.
2. `CLAUDE.md`, `.cursorrules`, `docs/src-guide.md` (module map).
3. Commit `077e14b` (`git show 077e14b`): how decision 0019 was built. This change follows the same shape.
4. Code: `src/template.rs` (`SAFE_AIR_FORMATION_DENSITY`, `FormationPreset`, `air_form`, `AIR_FORMATIONS`, `formations_for`, `formation_density`, `set_safe_air_formations`, the Formation branch of the order writer near `OrderKind::Formation`, `generate_template`, `OrderKind::Cover`, and the tests that assert density 1); every caller of `set_safe_air_formations` (`src/airtask/place.rs`, `src/airfield.rs`, `src/bombers.rs`, `src/frontlines.rs`, `src/pack.rs`, `src/recon.rs`); `src/airtask/library.rs` (the built-in test that asserts density 1); `src/airtask/probe.rs` (`Flight::is_frozen`, `regenerates_flown_files`, `flown_probes_regenerate_byte_for_byte_except_flight2`, the `flight2e_…` to `flight2h_…` tests).
5. `TemplateExamples/Historical1950/` (six built-ins; CRLF line endings, `.gitattributes` is `* -text`; `README.md`).

What to build:

A. **One rule, in one shared function in `src/template.rs`.** Rename or replace `set_safe_air_formations` with a function whose name says what it does now (for example `set_air_formation_density`), and keep every existing caller calling it. For each `MCU_CMD_Formation` whose `Objects` include an aircraft entity (found by entity link, as now):
   - `FormationType = 19` (Pairs): density 2, **unless** any entity in the command's `Objects` is also in the `Objects` of an `MCU_CMD_Cover` in the same tree, then density 1;
   - any other type: density 1.
   Name the values as constants (Safe 1 already exists; add Loose 2 and the Pairs type id if not named). Put one comment on the rule naming decisions 0019, 0020, 0022 and that 0021 treats all of it as a workaround for a game bug, to be removed when the game is fixed. Vehicle formations and everything else in the tree stay byte-identical.

B. **Template Builder.** `AIR_FORMATIONS` / `formation_density` give Pairs density 2 (Loose) and every other air type 1. `generate_template` (and any other Template Builder output path that writes Formation orders directly) must end with the same shared function applied to its output, so a Pairs flight that also has a Cover order is written at 1. Check the golden-file or fingerprint tests of Template Builder output and update them only where this rule changes a byte; say which bytes.

C. **Built-ins** (`TemplateExamples/Historical1950/`), byte-exact edits, CRLF kept:
   - `1950_DPRK_Yak_AirfieldRaid_2plus2.Group`: both Formation commands (indexes 35 and 41) `FormationDensity = 2`;
   - `1950_US_F51_ArmedRecon_2plus2.Group`: the Red lead's Formation command (`Objects = [7]`) `FormationDensity = 2`; the Blue lead's (`Objects = [11]`, the lead that flies the Cover order) stays 1;
   - the other four files do not change.
   The files must equal what the shared function writes for them (a test checks it).

D. **Flown probe evidence.** Flights whose regenerated output now changes because placement writes Pairs at Loose (expected: 2E and 2G, whose Pairs cells take placement's value) are frozen like 1M, 2B, 2C and 2D were for 0019: add them to `Flight::is_frozen` with a comment naming 0020 and 0022, and lower the compared count in `flown_probes_regenerate_byte_for_byte_except_flight2` accordingly. Flights 2F and 2H set their formation after placement and are expected to still regenerate byte for byte; if any other flight's output changes, stop and report which file and why before freezing it. Never edit a file under `handoff/p14-probes/` or `src/testdata/`.
   Tests of 2E to 2H that assert "equals the as-shipped placement" for a cell whose shipped value changes must not be deleted or weakened: make them state the old value as the flown evidence and assert the new shipped value separately.

E. **Tests**, each stating its own literals:
   - the rule: Pairs → 2; Pairs whose lead flies a Cover → 1; Pairs whose lead is the *target* of someone else's Cover (but flies none) → 2; Wedge and every other air type in `AIR_FORMATIONS` → 1; an unknown air type → 1; vehicle formations unchanged; applying it twice changes nothing; only `FormationDensity` of aircraft Formation commands changes;
   - the built-ins: the Yak raid 2 and 2, the F-51 ArmedRecon 2 (Red) and 1 (Blue), the other four unchanged; each file equals the shared function applied to itself;
   - placement: a placed Yak raid writes 2 and 2; a placed F-51 ArmedRecon writes 2 and 1;
   - Template Builder: a Pairs flight writes 2; a Pairs flight with a Cover order on its lead writes 1; a Wedge flight writes 1;
   - the existing 0019 tests updated to this rule (not deleted), and every other existing test passing.

F. **Docs, in the same change:** `USER_MANUAL.md` (the sentence "Aircraft formations are always written at Safe density…": say Pairs flights fly Loose except a flight with a Cover order, every other formation Safe, and why in one clause), `TemplateExamples/Historical1950/README.md` ("Formation density"), `docs/src-guide.md` if it names the function. Plain words; DPRK / NATO, never Eastern / Western.

Acceptance:
- Before editing, record the baseline: `cargo build --offline` and `cargo clippy --offline` warning counts (expected 69 / 251), `cargo test --offline` passed / failed / ignored (expected 693 / 0 / 27). After: warnings not above the baseline; new code adds none.
- `cargo test --offline`: 0 failed; report passed / ignored and which tests were added or changed.
- Every file under `handoff/p14-probes/` is unchanged; the flown-file test passes with its new count; 2F and 2H still regenerate byte for byte.
- `cargo test --offline write_p14_probe_2h -- --ignored` still writes its files; nothing under `handoff/` changes.

Files in scope: `src/template.rs`, the callers listed in 4 only where the rename touches them, `src/airtask/library.rs` and `src/airtask/place.rs` tests, `src/airtask/probe.rs` (freezing and the 2E–2H tests only), the two built-in files in C, `USER_MANUAL.md`, `TemplateExamples/Historical1950/README.md`, `docs/src-guide.md`.

Do not touch: `src/airtask/shell.rs`, `budget.rs`, `assemble.rs`, `testkit.rs`, `src/trace.rs`, `src/ui.rs` (unless a label there states the density; then stop and report), `src/testdata/`, `handoff/` (all of it), `docs/decisions/`, `STATE.md`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, the four other built-ins, and the game folder. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `claude/pairs-loose-0020` is checked out in this worktree and its working tree is clean when you start. Edit files in the working tree only. No git command that writes. `git status`, `git diff`, `git log` and `git show` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not weaken a test to make it pass; report the value and the reason.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run `cargo build --offline`, `cargo clippy --offline`, `cargo test --offline` and the ignored 2H writer, and read the counts from their output. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now: named constants, shared helpers, no flag arguments, no behaviour keyed on a bare number, failures that name what is missing. No refactor, rename or reformat outside the change (the one rename of the shared function is in scope).
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Where this prompt and the code disagree, follow the code for facts and report each difference.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary, five lines at most.
2. Command results: baseline and final build and clippy warnings, tests passed / failed / ignored, the flown-file count before and after.
3. Every file changed, one line each on why.
4. Tests added or changed, by name.
5. The rule as written (function name, constants, the comment).
6. Which flights were frozen and the evidence that their output changed; confirmation that 2F and 2H regenerate.
7. Template Builder output bytes that changed (golden or fingerprint), if any.
8. Choices you made where this prompt gave no exact shape, and residual risks.
</structured_output_contract>
