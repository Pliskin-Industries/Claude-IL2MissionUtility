<task>
Build P14 step 0j in this repository: one new in-game probe file, flight 1M.

Read first, in this order:
1. `handoff/P14-air-tasking.md` §8 "Step 0j. Upstream's strike and CAP, and the fixes as generated (flight 1M)" (the spec: four cells, the tables, how to read T-u, acceptance), and step 0i above it.
2. `src/testdata/p14_probe_inputs/upstream_a31ad0a/README.md` and the three `.Group` files beside it.
3. `handoff/P14-probe-results.md`, section "Flight 1L".
4. `docs/decisions/0012…`, `0013…`, `0014…`, `0015…`, `0016…`.
5. `src/airtask/probe.rs`: all of it. The flight 1L builders (`build_r`, `build_v`, `build_w`, the frozen-input loader, `start_group_at`, `single_aircraft`, the watch / kill / detect / remove helpers) and `build_j` are what this step builds on. `place_air_sortie` under `src/airtask/` for cell T-y.
6. `docs/il2-loader-and-log-facts.md`, `CLAUDE.md`, `.cursorrules`.

What to build, in `src/airtask/probe.rs` and new fixture files.
- `Flight::OneM`, stem `P14_Probe_1M`, `RUN_SHEET_1M` with the cells `T-u`, `T-x`, `T-y`, `T-j`, the two cues (00:00 and 14:00), `generate_probe_1m()`, an ignored writer `write_p14_probe_1m`, an empty spawn list.
- **Cell T-u** as its table. Load each source file from `src/testdata/p14_probe_inputs/upstream_a31ad0a/` (never edit those files). A const table with one row per group: name, centre, source file, and the one named change the control rows have (remove the event 7 abort hooks). Place the copy so its `Zone IN` sits on the row's centre and every other node keeps its offset from it. Rename the planes. Add the listeners the spec lists; they are additions only. For the order timers, trace each timer's output; where two groups differ in which timers exist (strike against CAP), drive that from the row or from the file, and fail with the group and node named if an expected node is missing.
- **Cell T-x** as its table. Add an ignored writer test that writes the three groups from the current `generate_template` into `src/testdata/p14_probe_inputs/flight1m/X1.Group` … `X3.Group` (refusing to overwrite an existing file), run it once, and have the builder load them, as flight 1L's frozen inputs are loaded. Add a normal test that generates the three groups again and compares them byte for byte with the files. Add a `README.md` in that folder.
- **Cell T-y** as its table. Y1 goes through `place_air_sortie`; do not set `AiRTBDecision` on Y1 in the probe, and assert in the test that the four planes carry 0 because `place_air_sortie` wrote it. Y2 and Y3 use `probe_flight` and keep the template's 1.
- **Cell T-j**: flight 0's cell and builder, reused unchanged, at the times flight 0 has it.
- The cells close at 16:00 with one `MCU_Delete` per aircraft entity. Icons per row centre, `COALITION_NATO`.
- Tests: every test named in "Step 0j acceptance (tests)", each row asserted against the test's own literals, including the "equal to the source file apart from the listed changes" check for T-u.

Report what you find on these, because the flight's reading depends on them:
1. For each upstream group: the offset of each plane, waypoint and AttackArea from `Zone IN` in the source file, and confirmation that the copy keeps them.
2. Anything in an upstream group that would fire by itself once its Mission Begin is cut off, or that the probe's start (pulsing `MISSION BEGIN`) skips compared with a player entering `Zone IN` (the README lists three nodes; say whether skipping them can change what the flight shows).
3. How `place_air_sortie`'s output is started in the probe, and whether it differs from a `probe_flight` copy in anything but position, heading and `AiRTBDecision`.

Acceptance:
- `cargo test --offline`: 0 failed, no fewer tests than before, and the new ignored writers. `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline write_p14_probe_ -- --ignored` writes the 1M files (plain and traced `.Group`, `.eng`, `.trace.json`, `.spawns.json` holding `[]`).
- The nine earlier flights: every generated file byte-for-byte identical to before your change. Make the baseline copy before you edit, compare after, report the result.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_1M_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10 --name P14_Probe_1M_check --title "P14 probe 1M check" --out target/p14/missions/P14_Probe_1M_check --no-resave` exits 0; in the written `.Mission` every `Index` is unique and every `Targets` / `Objects` / `TarId` / `LinkTrId` / `MisObjID` reference resolves (the upstream groups' `Trigger Delete` lists Plane indexes; they resolve to `Plane` blocks: say so). Check with a throwaway script; do not add it to the repo. Do not run the builder without `--no-resave`.

Files in scope: `src/airtask/probe.rs`; new files under `src/testdata/p14_probe_inputs/flight1m/`; `src/locale.rs` only if a new string needs it.
Do not touch: `src/template.rs`, `src/flights.rs`, anything under `src/testdata/p14_probe_inputs/upstream_a31ad0a/` or `flight1l/`, and every other file.
</task>

<git_protocol>
Same-host mode. The orchestrator has checked out the branch `codex/p14-0j` in this worktree. Edit files in the working tree only. No git command that writes. `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Run the three cargo commands, the ignored writers, the comparison and the `--no-resave` build before you report. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Stay inside the files in scope. Write in the style the file has now: const tables with named fields that read like the plan's tables; the shared `Probe` helpers; named constants for times, distances, countries, coalitions, event types, damage levels and generated node names; no flag arguments; no behaviour keyed on a bare number; failures that name the flight, the group and what is missing. Tests state their own literals.
</action_safety>

<missing_context_gating>
Do not write any block from memory: every key and its order comes from an existing builder, a reference file or a source file. Where a table leaves something open, choose the simplest reading and list the choice.
</missing_context_gating>

<structured_output_contract>
Return: (1) summary, five lines at most; (2) command results, the byte-for-byte comparison, the writer's files with sizes, and what the throwaway check printed; (3) the const tables as written; (4) a schedule in time order: every command and every pulse into a group's node, with its time and its Objects or target node (by name); (5) for each U group: the listeners added (count by kind) and the result of the "equal to the source apart from the listed changes" check; (6) the three findings asked for above; (7) anything a table left open that you chose, and residual risks.
</structured_output_contract>
