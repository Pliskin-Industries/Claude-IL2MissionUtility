<task>
Build P14 step 0f in this repository: one new in-game probe file, flight 1H (the Delete probe), and the change of the loss event from type 4 to type 13.

Read first, in this order:
1. `handoff/P14-air-tasking.md` §8 "Step 0f. Delete probe (flight 1H)" (the spec: run sheet, the T-d table, the detector, acceptance) and, for conventions, "Step 0e" and the 0b paragraph of "Step 0".
2. `docs/decisions/0011-losses-count-on-event-13.md` and the flight 1G section of `handoff/P14-probe-results.md`.
3. `docs/il2-loader-and-log-facts.md`, `CLAUDE.md`, `.cursorrules`.
4. `src/airtask/probe.rs` (all of it; `build_g` is the closest existing cell), `src/airtask/mod.rs` (`LossesImpl`), `TemplateExamples/Reference/CommandDamage_reference.Group`.

What to build.

A. `src/airtask/probe.rs`
- `Flight::OneH`, stem `P14_Probe_1H`, `RUN_SHEET_1H`, its two cues, `generate_probe_1h()`, an ignored writer `write_p14_probe_1h`, an empty spawn list. Cell key `T-d`.
- Cell T-d exactly as the step 0f table: eight 2-ship flights D0 to D7 from `probe_flight`, centres at (+130, +20 + 8·n), each starting 10 km south of its centre at 01:00.
- Each `Plane` block's `Name` is its label (`D0a`, `D0b`, … `D7b`). Entities are named `<label> entity` as in `build_g`.
- Each plane entity has `OnEvent` 13 → its own per-event subtitle (`<label> ev13`), registered as a traced source like the T-g2 events. No other event types in this cell.
- Kills at 02:00: one `MCU_CMD_Damage` (from the reference file, `Damage = 2`, `Type = 1`) per aircraft named in the "Killed at 02:00" column, Objects = that aircraft's entity.
- Removal at 04:00 per the "Removed at 04:00 by" column. A Delete's `Objects` list holds the entities **in the order the table gives**. D4 has two `MCU_Delete`s (one per aircraft) fired by one timer. D5's list holds D5a's entity only. D6 uses the copied template's own `Deactivate Units` and `DELETE DELAY` as G3 does in `build_g`.
- Detector at 04:30: one `MCU_CMD_Damage` (Complete) per aircraft of D1 to D7 (fourteen commands), each with one entity in Objects. D0 gets none.
- Cell close at 08:00: for this cell the close removes the aircraft with **one `MCU_Delete` per aircraft**, not one per flight.
- Icons: one `MCU_Icon` per flight centre (`T-d D0` … `T-d D7`), coalition `[2]`.

B. `src/airtask/mod.rs`: rename `LossesImpl::Type4` to `LossesImpl::Type13`, still the default, with a doc comment that says a lost aircraft raises OnEvent 13 once and never 4 (flight 1G, decision 0011). Update any use of the old name.

C. Tests: every test named in "Step 0f acceptance (tests)", asserting each row of the T-d table against literals.

Acceptance:
- `cargo test --offline`: at least 544 passed, 0 failed, 11 ignored (10 now plus the new writer). `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline write_p14_probe_ -- --ignored` writes the 1H files (plain and traced `.Group`, `.eng`, `.trace.json`, `.spawns.json` holding `[]`).
- Flights 0, 1A, 1N, 1D and 1G: every generated file byte-for-byte identical to before your change. Make the baseline copy before you edit, compare after, report the result.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_1H_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10 --name P14_Probe_1H_check --title "P14 probe 1H check" --out target/p14/missions/P14_Probe_1H_check --no-resave` exits 0; in the written `.Mission` every `Index` is unique and every `Targets` / `Objects` / `TarId` / `LinkTrId` / `MisObjID` reference resolves. Check with a throwaway script; do not add it to the repo.

Files in scope: `src/airtask/probe.rs`; `src/airtask/mod.rs` (the `LossesImpl` rename only); `src/locale.rs` only if a new string needs it; any other `src/airtask/*.rs` file only where it names `LossesImpl::Type4`.
Do not touch: every other file, including `tools/`, `src/trace.rs`, `src/template.rs`, `handoff/`, `docs/`, `TemplateExamples/`, `STATE.md`.
</task>

<git_protocol>
Same-host mode. The orchestrator has checked out the branch `codex/p14-0f` in this worktree. Edit files in the working tree only. No git command that writes. `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Run the three cargo commands, the ignored writers, the comparison and the `--no-resave` build before you report. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Stay inside the files in scope. Write new code in the style the file has after the step 0e cleanup: named constants for countries and event types, no flag arguments, no bare numbers that have a name.
</action_safety>

<missing_context_gating>
Do not write any block from memory: every key and its order comes from a file named above. Where the step 0f table leaves something open, choose the simplest reading and list the choice.
</missing_context_gating>

<structured_output_contract>
Return: (1) summary, five lines at most; (2) command results, the byte-for-byte comparison, the writer's files with sizes, and what the throwaway check printed; (3) for each of D0 to D7: the two plane names and entity indexes, the Damage command indexes and times, the Delete indexes with their Objects lists in order; the fourteen detector commands' time; (4) anything the table left open that you chose, and residual risks.
</structured_output_contract>
