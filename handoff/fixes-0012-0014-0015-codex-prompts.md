# Delegation prompts: freezing flight 1L's inputs, then decisions 0012, 0014 and 0015

Four jobs, run one after another on branch `codex/fixes-0012-0014-0015` (from `claude/p14-air-tasking`). The orchestrator commits after each, so each lands as its own commit. Shared rules are at the end and are sent with every job.

---

## Job 1. Freeze flight 1L's generated inputs

<task>
Flight 1L's cell T-w is built by calling `generate_template` when the probe is generated. The next jobs change what `generate_template` writes, and the flown flight must still regenerate byte for byte (`handoff/p14-probes/flight1l/`). Make the 1L builder independent of the live generator.

- Write each of the six W groups, exactly as `generate_template` returns it today and before the probe touches it (no re-indexing, no placing, no edits), to `src/testdata/p14_probe_inputs/flight1l/W1.Group` … `W6.Group`, with the app's own serializer. Add an ignored writer test that produced them, with a comment that it must not be re-run after the generator changes (it would overwrite the frozen inputs with different bytes); make the test refuse to overwrite an existing file.
- Change the 1L builder to load those six files (parse them with the existing parser) instead of calling `generate_template`. Everything after that point stays as it is.
- Add a `README.md` in that folder: what the files are, which commit of the generator wrote them, that flight 1L was flown from them on 2026-10-13, and that they are never regenerated.

Acceptance: every `target/p14/P14_Probe_*` file for all nine flights byte-for-byte identical to before (baseline first, compare after); `cargo test --offline` at least 547 passed, 0 failed, and one more ignored test than before; warnings not above 69 build, 251 clippy.

Files in scope: `src/airtask/probe.rs`; new files under `src/testdata/p14_probe_inputs/flight1l/`.
</task>

---

## Job 2. Decision 0012: an aircraft's death event is `OnKilled` (13)

<task>
Read `docs/decisions/0012-aircraft-death-event-outside-air-tasking.md` (all of it, including "Evidence added") and the "Flight 1L" section of `handoff/P14-probe-results.md`.

Change: `EntityEvent::default_for(UnitKind::Plane)` returns `OnKilled` (13), as it already does for every other kind. Follow that through everything that assumed 4:
- `src/template.rs`: the default, its tests (three were named in the record), `inspect_hooks`, and anything that reads a loaded template's death event.
- `src/flights.rs` (Fighter Pack): it re-points the generated death events to per-flight `DeathCount` clones; make sure it finds them by the new type and that its tests state 13.
- `src/airtask/library.rs` and `src/airtask/testkit.rs`: the Repeat-sortie shape check accepts only type 4 into `DeathCount`, and the test walker fires type 4 on a kill. Both become 13. Say what you decided for a user template that still carries type 4 (accept and warn, or refuse with a message that names the event) and why; do not silently accept it as working.
- `USER_MANUAL.md`: the lines that name the death event (the record names lines 98 and 114; find them by content).
- Baselines: the generated-file baselines change by one line per aircraft (`Type = 4;` becomes `Type = 13;` in the `OnEvent` that targets `DeathCount`). Re-record them, and report for each baseline file how many lines changed and that no other line did.

Write the failing tests first. Do not touch the 16 example files under `TemplateExamples/`, and do not change Fighter Pack's double feed into `DeathCount` (that is decision 0016, still open).

Acceptance: `cargo test --offline` passes with no fewer tests than before; warnings not above 69 build, 251 clippy; the nine probe flights regenerate byte for byte; a diff of each re-recorded baseline shows only `Type = 4;` → `Type = 13;` lines.
</task>

---

## Job 3. Decision 0014 (with 0013): `Trigger Delete` lists entities in Spawn mode and is not written in Activate mode

<task>
Read `docs/decisions/0013-template-builder-delete-lists-plane-indexes.md`, `0014-no-delete-in-activate-mode.md`, and the "Flight 1L" section of `handoff/P14-probe-results.md` (groups W3 to W6).

Change in `generate_template`:
- **Spawn mode** (`BringUp::Spawn`, with or without `allow_multiple_spawns`): `Trigger Delete`'s `Objects` are the units' entity indexes (`entity_ids`), not their block indexes.
- **Activate mode:** `DELETE DELAY` and `Trigger Delete` are not written. `DELAYED END ORDERS` then targets `Deactivate Units` only. Indexes of later nodes shift accordingly; that is expected.

Follow it through:
- every place that looks the two nodes up by name (the consumed-names list near `mark_consumed`, layout code, `inspect` code, Fighter Pack in `src/flights.rs`, Exclusive and Recon code that reads a loaded template's clean-up) must cope with a template that has no Delete, and with an older template that has one;
- `src/airtask/library.rs` checks a sortie's shape and expects one Delete command. The six built-in sorties are files and still have theirs, so the check keeps passing for them; make it state clearly what it requires and why, and do not loosen it silently;
- `USER_MANUAL.md` line 121 (find it by content: the clean-up chain) and any other line that says a group is deleted;
- baselines: re-record them and report, per baseline file, what changed (in Spawn mode one `Objects` list; in Activate mode two nodes fewer and the index shifts that follow).

Write the failing tests first: one for each mode, asserting the `Objects` of `Trigger Delete` against the entity indexes in Spawn mode, and the absence of both nodes in Activate mode with `DELAYED END ORDERS` still reaching `Deactivate Units`.

Do not touch the example files under `TemplateExamples/`.

Acceptance: `cargo test --offline` passes with no fewer tests than before; warnings not above 69 build, 251 clippy; the nine probe flights regenerate byte for byte.
</task>

---

## Job 4. Decision 0015: Air Tasking places sortie aircraft with `AiRTBDecision = 0`

<task>
Read `docs/decisions/0015-air-tasking-sorties-keep-ai-rtb-decision-off.md` and the "Flight 1L" section of `handoff/P14-probe-results.md` (cell T-r).

Change: `place_air_sortie` (find it under `src/airtask/`) sets `AiRTBDecision = 0` on every `Plane` block of the copy it places, whatever the template file has. The template files are not changed. Name the value with a constant and a comment that cites decision 0015. Fail loudly if a placed copy has a `Plane` block without the key.

Tests first: for each of the six built-in sorties, the placed copy has `AiRTBDecision = 0` on every plane, the count of planes changed equals the sortie's plane count, and the source template is untouched.

Update `handoff/P14-air-tasking.md` only where it describes `place_air_sortie`'s output (§3.1) with one sentence and the decision number.

Acceptance: `cargo test --offline` passes with no fewer tests than before; warnings not above 69 build, 251 clippy; the nine probe flights regenerate byte for byte.

Files in scope: files under `src/airtask/`; `handoff/P14-air-tasking.md` (§3.1, one sentence).
</task>

---

## Sent with every job

<git_protocol>
Same-host mode. The orchestrator has checked out the branch in this worktree. Edit files in the working tree only. No git command that writes. `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something in the task cannot be done as written, stop and say what and why; do not work around it.
</completeness_contract>

<verification_loop>
Run `cargo test --offline`, `cargo build --offline`, `cargo clippy --offline` and `cargo test --offline write_p14_probe_ -- --ignored`, and compare every `target/p14/P14_Probe_*` file with a baseline copy made before you edited. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Stay inside the files the task names. Match the style of the code around your change. Named constants, no flag arguments, failures that say what is missing. Tests state their own literals.
</action_safety>

<structured_output_contract>
Return: (1) summary, four lines at most; (2) command results and the probe byte comparison; (3) every file changed, with one line on why; (4) for each baseline or fixture re-recorded: the file, the number of changed lines, and proof that only the expected lines changed; (5) every decision you had to make that the task did not make for you; (6) residual risks.
</structured_output_contract>
