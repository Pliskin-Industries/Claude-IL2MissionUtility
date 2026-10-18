<task>
Build P14 step 0h in this repository: one new in-game probe file, flight 1K (what can keep an aircraft from being deleted).

Read first, in this order:
1. `handoff/P14-air-tasking.md` §8 "Step 0h. What can keep an aircraft from being deleted (flight 1K)" (the spec: two cells, two tables, how to read it, acceptance), and just above it steps 0f and 0g.
2. `src/airtask/probe.rs`: all of it. `build_d` and the 1J builder are the pattern for cell T-d3; `build_g` (the hunter flight and its repeated `MCU_CMD_AttackTarget`) is the pattern for cell T-d4.
3. `handoff/P14-probe-results.md`, sections "Flight 1G", "Flight 1H" and "Flight 1J": what is already known.
4. `docs/il2-loader-and-log-facts.md`, `CLAUDE.md`, `.cursorrules`.

What to build, all in `src/airtask/probe.rs`.
- `Flight::OneK`, stem `P14_Probe_1K`, `RUN_SHEET_1K` with the two cells `T-d3` and `T-d4`, the two cues (00:00 and 16:00), `generate_probe_1k()`, an ignored writer `write_p14_probe_1k`, an empty spawn list.
- **Cell T-d3** exactly as its table: eight 2-ship formations K0 to K7. A const table with one row per pair that reads like the plan's table: name, centre, what happens at 02:00 (nothing; a Delete of the wingman; a Damage command at a given level aimed at the lead), what happens at 04:00 (nothing; a Delete of the lead only; of the wingman only; of both), and whether the 04:30 detector hits it. The Damage level is a named type or named constants (`Damage = 0`, `1`, `2`), not a bare number at the call site; the existing kill helper keeps writing level 2.
- **Cell T-d4** exactly as its table: three 2-ship formations K8, K9, K10, each with a 2-ship hunter flight written as `build_g` writes G1's hunters (country 503, MiG-15bis, `AILevel` 4, starting 5 km behind the pair's start, `MCU_CMD_AttackTarget` from the hunter lead to the pair's lead). The Attack is issued at 01:30 and every 90 s after it, the last one before the pair's Delete (K8: 01:30 and 03:00; K9 and K10: 01:30 to 14:30). Share the hunter code with `build_g` through a helper if, and only if, flight 1G's files stay byte-identical. A const table with one row per pair: name, centre, Delete time, Delete wiring (one Delete for both; one per aircraft), detector time.
- Plane `Name`s are the labels (`K0a` … `K10b`; hunters `K8Ha`, `K8Hb`, …); entities `<label> entity`; every plane, hunters included, has OnEvent 13 → `<label> ev13`, traced.
- The cells close at 20:00 with one `MCU_Delete` per aircraft, hunters included.
- Icons: one `MCU_Icon` per pair centre (`T-d3 K0` … `T-d4 K10`), `COALITION_NATO`.
- Tests: every test named in "Step 0h acceptance (tests)", each row asserted against the test's own literals.

Acceptance:
- `cargo test --offline`: at least 547 passed, 0 failed, 13 ignored (12 now plus the new writer). `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline write_p14_probe_ -- --ignored` writes the 1K files (plain and traced `.Group`, `.eng`, `.trace.json`, `.spawns.json` holding `[]`).
- Flights 0, 1A, 1N, 1D, 1G, 1H and 1J: every generated file byte-for-byte identical to before your change. Make the baseline copy before you edit, compare after, report the result.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_1K_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10 --name P14_Probe_1K_check --title "P14 probe 1K check" --out target/p14/missions/P14_Probe_1K_check --no-resave` exits 0; in the written `.Mission` every `Index` is unique and every `Targets` / `Objects` / `TarId` / `LinkTrId` / `MisObjID` reference resolves. Check with a throwaway script; do not add it to the repo. Do not run the builder without `--no-resave`: the game's resaver crashes since the game's update 1.005.

Files in scope: `src/airtask/probe.rs`; `src/locale.rs` only if a new string needs it.
Do not touch: every other file.
</task>

<git_protocol>
Same-host mode. The orchestrator has checked out the branch `codex/p14-0h` in this worktree. Edit files in the working tree only. No git command that writes. `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Run the three cargo commands, the ignored writers, the comparison and the `--no-resave` build before you report. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Stay inside the files in scope. Write in the style the file has now: const tables with named fields that read like the plan's tables, shared `Probe` helpers for per-aircraft work, named constants for times, distances, countries, coalitions, event types and damage levels, no flag arguments, no behaviour keyed on a bare number. Tests state their own literals.
</action_safety>

<missing_context_gating>
Do not write any block from memory: every key and its order comes from an existing builder or a reference file. Where a table leaves something open, choose the simplest reading and list the choice.
</missing_context_gating>

<structured_output_contract>
Return: (1) summary, five lines at most; (2) command results, the byte-for-byte comparison, the writer's files with sizes, and what the throwaway check printed; (3) the two const tables as written; (4) a schedule: every Damage, Delete and Attack command with its time and its Objects (by plane name), in time order; (5) anything a table left open that you chose, and residual risks.
</structured_output_contract>
