<task>
Build P14 step 0i in this repository: one new in-game probe file, flight 1L. It has three cells; the third runs what `generate_template` writes.

Read first, in this order:
1. `handoff/P14-air-tasking.md` §8 "Step 0i. The app's own logic in the game (flight 1L)" (the spec: three cells, three tables, how to read T-w, acceptance), and steps 0f to 0h above it.
2. `handoff/P14-probe-results.md`, sections "Flight 1H", "Flight 1J", "Flight 1K": what is known.
3. `docs/decisions/0012-aircraft-death-event-outside-air-tasking.md` and `0013-template-builder-delete-lists-plane-indexes.md`.
4. `src/airtask/probe.rs`: all of it (the builders for flights 1H, 1J, 1K and the shared `Probe` helpers are the pattern for cells T-r and T-v; `probe_flight` and `ai_without_close_delete` show how a copied group is cut off from its Mission Begin and started by pulsing one of its own nodes).
5. `src/template.rs`: `TemplateOptions`, `TemplateSeat`, `BringUp`, `FlightRole`, `generate_template`, and the test `repeat_spawn_counts_every_destroyed_unit` (how to build a repeat-spawn group from independent seats); `builtin_plane_catalog()` and the F-80C lookup in `src/baseline_tests.rs`.
6. `docs/il2-loader-and-log-facts.md`, `CLAUDE.md`, `.cursorrules`.

What to build, all in `src/airtask/probe.rs`.
- `Flight::OneL`, stem `P14_Probe_1L`, `RUN_SHEET_1L` with the cells `T-r`, `T-v`, `T-w`, the two cues (00:00 and 24:00), `generate_probe_1l()`, an ignored writer `write_p14_probe_1l`, an empty spawn list.
- **Cell T-r** as its table: four 2-ship formations. `AiRTBDecision` is set on both `Plane` blocks of a pair from the row (the template writes 1; R1 and R2 get 0). A const table with one row per pair, in the style of `K_FORMATIONS`.
- **Cell T-v** as its table: three 1-plane flights. Each entity gets one OnEvent per type 0 to 13, each to its own subtitle `<label> ev<type>`, traced. V2's `Plane` block gets `Fuel = 0.03`. A const table with one row per aircraft. A named constant for the list of event types, with a comment giving the app's name for each (`EntityEvent` in `template.rs`).
- **Cell T-w** as its table: six groups, each from `generate_template` with two independent F-80C seats and no orders, then merged into the probe (re-indexed like any copied group), placed at its centre, cut off from its own `Translator Mission Begin`, and started by the probe's timers pulsing the group's own nodes at the times in the table. Do not remove `DeathCount`, `COOLDOWN`, `SpawnCount`, `Trigger Spawner`, `MISSION END` or anything else from the group; do not pulse its check zones. Apply the "Changed after generation" column by editing the generated nodes in the probe (COOLDOWN `Time`, the type of the unit OnEvents that target `DeathCount`, the `Objects` of `Trigger Delete`). Plane `Name`s `W1a` … `W6b`; a traced OnEvent 13 subtitle per unit entity, added as an extra listener; traced outputs of `DeathCount`, `COOLDOWN` and `SpawnCount` where the group has them. A const table with one row per group that reads like the plan's table (bring-up mode, death event, Delete list kind, and the schedule kind).
- The cells close at 26:00 with one `MCU_Delete` per aircraft entity.
- Icons: one `MCU_Icon` per row centre, `COALITION_NATO`.
- Tests: every test named in "Step 0i acceptance (tests)", each row asserted against the test's own literals.

Hard rule: **do not change `src/template.rs`** or any other file outside `src/airtask/probe.rs` (and `src/locale.rs` only if a new string needs it). If something `generate_template` needs is private, call what is public; if that is impossible, stop and report what would have to become visible, and do not change it.

Report what you find on these, because the flight's reading depends on them:
1. Whether the unit entities in a spawn-mode group are written `Enabled = 0`, and what the group's `Trigger Spawner` lists.
2. What an aircraft with no orders is given to do (waypoints or none), and where the two aircraft are placed relative to the group's centre.
3. Whether anything in a generated group would fire by itself once its Mission Begin is cut off.

Acceptance:
- `cargo test --offline`: at least 547 passed, 0 failed, 14 ignored (13 now plus the new writer). `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline write_p14_probe_ -- --ignored` writes the 1L files (plain and traced `.Group`, `.eng`, `.trace.json`, `.spawns.json` holding `[]`).
- Flights 0, 1A, 1N, 1D, 1G, 1H, 1J and 1K: every generated file byte-for-byte identical to before your change. Make the baseline copy before you edit, compare after, report the result.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_1L_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10 --name P14_Probe_1L_check --title "P14 probe 1L check" --out target/p14/missions/P14_Probe_1L_check --no-resave` exits 0; in the written `.Mission` every `Index` is unique and every `Targets` / `Objects` / `TarId` / `LinkTrId` / `MisObjID` reference resolves, **except** W1, W2, W3 and W5's `Trigger Delete`, whose `Objects` are Plane indexes as the app writes them (they resolve to `Plane` blocks; say so). Check with a throwaway script; do not add it to the repo. Do not run the builder without `--no-resave`.

Files in scope: `src/airtask/probe.rs`; `src/locale.rs` only if a new string needs it.
</task>

<git_protocol>
Same-host mode. The orchestrator has checked out the branch `codex/p14-0i` in this worktree. Edit files in the working tree only. No git command that writes. `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Run the three cargo commands, the ignored writers, the comparison and the `--no-resave` build before you report. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Stay inside the files in scope. Write in the style the file has now: const tables with named fields that read like the plan's tables; the shared `Probe` helpers for per-aircraft work, with a time argument; named constants for times, distances, countries, coalitions, event types and damage levels; no flag arguments; no behaviour keyed on a bare number; failures that name the flight and what is missing. Tests state their own literals.
</action_safety>

<missing_context_gating>
Do not write any block from memory: every key and its order comes from an existing builder, a reference file or `generate_template`'s own output. Where a table leaves something open, choose the simplest reading and list the choice.
</missing_context_gating>

<structured_output_contract>
Return: (1) summary, five lines at most; (2) command results, the byte-for-byte comparison, the writer's files with sizes, and what the throwaway check printed; (3) the three const tables as written; (4) a schedule in time order: every command and every pulse into a W group's node, with its time and its Objects or target node (by name); (5) for each W group: its nodes by name with index, the event type on its units, its `Trigger Delete` Objects, its `COOLDOWN` time; (6) the three findings asked for above; (7) anything a table left open that you chose, and residual risks.
</structured_output_contract>
