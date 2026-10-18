<task>
Build P14 step 0e in this repository (Rust 2024 binary crate, egui, nom): three new in-game probe files (flights 1N, 1D, 1G) that replace flight 1B, a spawn list per probe, and a builder option that turns a spawn list into air-start spawn points.

Read these first, in this order:
1. `handoff/P14-air-tasking.md`: §8 "General rules for every step"; §8 "Step 0. In-game MCU probe" (0b, the run sheets, the Cells table, the Step 0 acceptance list); §8 "Step 0e. Probes reshaped for an autopilot pilot" (the spec for this task: flights, spawn points, run sheets, cells, acceptance).
2. `docs/il2-loader-and-log-facts.md` (all of it; the last two sections describe the blocks this task adds) and `docs/decisions/0010-reshape-the-remaining-probes-for-an-autopilot-pilot.md`.
3. `CLAUDE.md` and `.cursorrules`.
4. `src/airtask/probe.rs` (all of it: `Flight`, `Cell`, `RUN_SHEET_*`, `CUES`, `Probe` and its helpers `zone`, `observe`, `readout`, `icon`, `ai`, `at`, `switch_at`, `build_n`, `build_g`, `build`, `write_probe`, the ignored writers and every test), `src/locale.rs` near `P14_PROBE_LC_START` and `p14_probe_locale`, `src/trace.rs` (`instrument`, `TraceSelect`, `TraceSource`, `write_trace_sidecar`), `src/template.rs` (`mcu`, `timer`, `counter`, `checkzone`, `attach_event`).
5. The block shapes to copy, key for key and in the same order: `TemplateExamples/Reference/ComplexTrigger_reference.Group` (note `Enabled = 1;` twice, the 21 `EventsFilter…` keys, the `Country` line, and the `OnEvents` block), `TemplateExamples/Reference/CommandDamage_reference.Group`, and the `MCU_CMD_AttackTarget` block in `TemplateExamples/CorrectedFlightfromTemplate.Group`.
6. `tools/p14/build_probe_mission.py` (all of it) and, for the spawn airfield shape, the two `Airfield` blocks with their `MCU_TR_Entity` in `C:\Program Files\IL2Series\game\data\Multiplayer\Dogfight\_test_dogfight_IL-3.Mission` (read only; each has `StartType = 0` entries named `Airstart`).

What to build.

A. In `src/airtask/probe.rs` (test-only code, as the rest of the file):
- `Flight::OneN`, `Flight::OneD`, `Flight::OneG` with stems `P14_Probe_1N`, `P14_Probe_1D`, `P14_Probe_1G`; `RUN_SHEET_1N`, `RUN_SHEET_1D`, `RUN_SHEET_1G`; their `CUES`; `generate_probe_1n()`, `generate_probe_1d()`, `generate_probe_1g()`; all exactly as the step 0e run sheets and Cells table define them.
- Remove `Flight::OneB`, `RUN_SHEET_1B`, its cues, `generate_probe_1b`, `write_p14_probe_1b` and the `P14 DPRK SPAWN` icon. Reuse `build_g` for T-g2 (it gains G4, the hunter pair, the Attack command and the Damage commands, and the new times).
- Cell T-n2: as `build_n` but radius 5000 and no ZA4. Flight 1A keeps its own T-n unchanged.
- Cells Y-N and Y-D: one builder with the side as a parameter. A Complex Trigger has no `Targets`; its outputs are `OnEvent` entries (58 = Object Entered, 60 = Object Left). The unfiltered trigger has no `Country` line at all. No `ObjectScript` and no `ObjectName` line in either trigger (the reference file has one of each only to show the shape). Each "entered" and "left" subtitle and each readout is a traced source, like every other per-event subtitle in the file.
- AI pairs for Y-N, Y-D and the hunter pair: `probe_flight` with 2 planes, then set `Country` on both `Plane` blocks (and on anything else in the copied flight that carries the old country, if the file has such a key). Each Y pair has its own start time and its own delete time (not the cell close): extend `Probe::ai` or add a sibling so a flight can be deleted at a given time; the cell close must still delete it if it is somehow alive.
- A spawn list per flight: a `Spawn` struct (`name`, `country`, `km`, `heading_deg`, `altitude_m`, `script`) and `fn spawns(self) -> &'static [Spawn]` on `Flight` (empty for flights 0, 1A and 1G). The ignored writers write `<stem>.spawns.json` and `<stem>_traced.spawns.json` (same content; a JSON array of objects with `name`, `country`, `x`, `z`, `heading_deg`, `altitude_m`, `script`, where `x`, `z` are world metres). Flights 0 and 1A write `[]`.
- Ignored writers `write_p14_probe_1n`, `write_p14_probe_1d`, `write_p14_probe_1g`.

B. In `tools/p14/build_probe_mission.py`:
- `--spawns <file.json>` (optional, repeatable is not needed). For each entry, add a spawn airfield copied from the game's dogfight test mission: the `Airfield` block whose `Country` matches the entry, together with the `MCU_TR_Entity` it links to (`LinkTrId` / `MisObjID`), renumbered past every index already used, `Name` set to the entry's `name`, `XPos` / `ZPos` set to `x` / `z`, `YOri` set to `heading_deg`, and its `Planes` list reduced to one `Plane` entry: that airfield's `StartType = 0` entry for `script`, with `Altitude` set to `altitude_m`. Fail with a clear message if the country has no airfield in the test mission, or the script has no `StartType = 0` entry there. An entry's `name` goes through the same loader-safe check as other quoted strings.
- `--no-resave`: write the `.Mission` and the `.eng` and stop before running `MissionResaver.exe` (exit code 0). Your sandbox cannot run the resaver usefully; the orchestrator runs it.
- `--ref` stays optional. With `--spawns` and no `--ref`, the mission has only the probe and the spawn airfields.
- Keep the existing behaviour for every existing argument: the flight 0 and flight 1A command lines in `handoff/p14-probes/*/README.md` must write the same `.Mission` text as before.

Acceptance. Each line is a named test or a command result:
- Every test named in "Step 0e acceptance (tests)" exists and passes, covering flights 0, 1A, 1N, 1D, 1G. `probe_cells_match_the_table` and `probe_run_sheets_match_the_tables` check every property, position and time that the step 0e tables state, the same way the existing assertions do for flights 0 and 1A.
- `probe_spawn_lists_match_the_run_sheets` (new).
- Flights 0 and 1A: `generate_probe_0()` and `generate_probe_1a()` serialize to exactly the bytes they do now, plain and traced. Prove it: before you edit, run `cargo test --offline write_p14_probe_ -- --ignored` and keep copies of the `P14_Probe_0*` and `P14_Probe_1A*` files outside `target/p14/`; after your edits run the writers again and compare byte for byte. Report the result.
- `cargo test --offline write_p14_probe_ -- --ignored` writes, for each of the five flights, the plain `.Group`, the traced `.Group`, the `.trace.json`, the `.eng` files and the two `.spawns.json` files.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_1N_traced.Group --spawns target/p14/P14_Probe_1N_traced.spawns.json --name P14_Probe_1N_check --title "P14 probe 1N check" --out target/p14/missions/P14_Probe_1N_check --no-resave` exits 0, and the written `.Mission` holds one `Airfield` named `1N NATO` at the run-sheet position with `Country = 601`, `YOri = 0`, exactly one `Plane` entry with `StartType = 0`, `Altitude = 3050` and the `f80c10` script, every `Index` unique, and every `LinkTrId` / `MisObjID` / `Targets` / `Objects` reference resolving. The same for 1D (`1D USSR`, 501, `mig15bis`). Check these with a short throwaway script and report what it printed; do not add that script to the repo.
- `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline`: at least 544 passed (543 now plus `probe_spawn_lists_match_the_run_sheets`), 0 failed, 10 ignored (8 now, minus `write_p14_probe_1b`, plus the three new writers).

Files in scope: `src/airtask/probe.rs`; `src/locale.rs` (only if the new subtitles or icons need it, additions only); `tools/p14/build_probe_mission.py`.

Do not touch: every other file. In particular `src/trace.rs`, `src/missionlog.rs`, `src/template.rs`, `src/recon.rs`, `src/duplicate.rs`, `src/placement.rs`, `src/mapnet.rs`, `src/weapon_range.rs`, `src/ui*`, `src/testdata/`, `TemplateExamples/`, `handoff/`, `docs/`, `STATE.md`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, and anything under `C:\Program Files\IL2Series`. No new dependency (write the JSON by hand or with what the crate already uses). No change to any Generate output: nothing outside tests calls the probe code. If you need a private item from another module made `pub(crate)`, stop and report instead of changing it.
</task>

<git_protocol>
Same-host mode. The orchestrator has already checked out the branch `codex/p14-0e` in this worktree. Edit files in the working tree only. Do not run any git command that writes (no add, commit, branch, checkout, stash, reset, restore, worktree). `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Do not stop after a partial change. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Before you report, run `cargo build --offline`, `cargo clippy --offline` and `cargo test --offline`, then the ignored writers and the two `--no-resave` builds, and read the warning counts and the test totals from their output. If a count is above its limit or a test fails, fix it inside the files in scope and run them again. Do not silence a warning in existing code. New code adds no warnings.
</verification_loop>

<action_safety>
Stay inside the files in scope. No refactor, rename, reformat or cleanup of existing code beyond what removing flight 1B needs. Do not run `cargo fmt` on the whole crate; format only the code you add. Do not run `MissionResaver.exe`, the game or the editor.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour, and do not write any block from memory: every key and its order comes from a file named above. Where the plan and the code disagree (names, signatures), follow the code for existing items and the plan for semantics, and list the difference in the report. If the step 0e tables leave something open, choose the simplest reading, and list the choice.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary of the change (5 lines at most).
2. The command results: build warnings, clippy warnings, tests passed / failed / ignored, the byte-for-byte comparison for flights 0 and 1A, the files the writers produced with their sizes, and what the throwaway check printed for the two `--no-resave` builds.
3. For each cell of the three new run sheets: its key MCUs (names, indexes) and times, and anything the tables left open that you chose.
4. How the spawn list is written and how the builder turns an entry into an airfield (which keys it changes).
5. Differences between the plan and the code, and residual risks.
</structured_output_contract>
