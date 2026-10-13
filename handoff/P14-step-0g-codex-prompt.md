<task>
Build P14 step 0g in this repository: one new in-game probe file, flight 1J (the Delete probe with single aircraft).

Read first, in this order:
1. `handoff/P14-air-tasking.md` §8 "Step 0g. Delete probe with single aircraft (flight 1J)" (the spec) and, just above it, "Step 0f" (the flight this one follows).
2. `src/airtask/probe.rs`: all of it. `build_d`, `D_FLIGHTS`, `DeleteProbeRow`, `DeleteRemoval`, `Seat`, `damage_at`, the Delete helper and the 1H tests are the pattern to follow.
3. `docs/il2-loader-and-log-facts.md`, `CLAUDE.md`, `.cursorrules`.

What to build, all in `src/airtask/probe.rs`.
- `Flight::OneJ`, stem `P14_Probe_1J`, `RUN_SHEET_1J`, its two cues, `generate_probe_1j()`, an ignored writer `write_p14_probe_1j`, an empty spawn list. Cell key `T-d2`.
- Cell T-d2 exactly as the step 0g table: seven pairs J0 to J6. **Each pair is two separate 1-ship flights** from `probe_flight` (1 plane each), named for example `J2x` and `J2y`; the `x` flight's centre is 1 km west of the pair's centre and the `y` flight's 1 km east; each starts 10 km south of its own centre at 01:00. Use the builder that adds no close Delete of its own.
- Each `Plane` block's `Name` is its label (`J0x` … `J6y`); its entity is `<label> entity`; each entity has OnEvent 13 → `<label> ev13`, traced as in flight 1H.
- Kills at 02:00, removal at 04:00, detector at 04:30 and the 08:00 close as the table gives, with the same helpers and time constants as flight 1H where the value is the same (share the constants; do not copy them).
- A const table with one row per pair, written so that it reads like the plan's table. Reuse `DeleteRemoval` variants where they mean the same thing for a pair of 1-ship flights; if the 1H names (`DeleteBothEntities`, `DeleteLeadEntity`, …) read wrongly for x and y, give this cell its own small enum rather than bending the names. No behaviour keyed on a bare number.
- Icons: one `MCU_Icon` per pair centre (`T-d2 J0` … `T-d2 J6`), coalition `[1]` (the NATO side, which is the side the player sits on; flight 1H's `[2]` was a slip in its spec and stays as flown).
- Tests: every test named in "Step 0g acceptance (tests)", with each row asserted against literals.

Check one thing before you build on it and report what you find: that `probe_flight` with 1 plane writes a valid single-aircraft flight (no formation links to a missing wingman, no `Objects` or `Targets` left pointing at a removed plane). If the template needs care for a 1-ship, say what you did.

Acceptance:
- `cargo test --offline`: at least 546 passed, 0 failed, 12 ignored (11 now plus the new writer). `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline write_p14_probe_ -- --ignored` writes the 1J files (plain and traced `.Group`, `.eng`, `.trace.json`, `.spawns.json` holding `[]`).
- Flights 0, 1A, 1N, 1D, 1G and 1H: every generated file byte-for-byte identical to before your change. Make the baseline copy before you edit, compare after, report the result.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_1J_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10 --name P14_Probe_1J_check --title "P14 probe 1J check" --out target/p14/missions/P14_Probe_1J_check --no-resave` exits 0; in the written `.Mission` every `Index` is unique and every `Targets` / `Objects` / `TarId` / `LinkTrId` / `MisObjID` reference resolves. Check with a throwaway script; do not add it to the repo. Do not run it without `--no-resave`: the game's resaver crashes since the game's update 1.005.

Files in scope: `src/airtask/probe.rs`; `src/locale.rs` only if a new string needs it.
Do not touch: every other file.
</task>

<git_protocol>
Same-host mode. The orchestrator has checked out the branch `codex/p14-0g` in this worktree. Edit files in the working tree only. No git command that writes. `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Run the three cargo commands, the ignored writers, the comparison and the `--no-resave` build before you report. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Stay inside the files in scope. Write in the style the file has now: const tables with named fields, named constants for times, distances, countries and event types, no flag arguments.
</action_safety>

<missing_context_gating>
Do not write any block from memory: every key and its order comes from an existing builder or a reference file. Where the step 0g table leaves something open, choose the simplest reading and list the choice.
</missing_context_gating>

<structured_output_contract>
Return: (1) summary, five lines at most; (2) command results, the byte-for-byte comparison, the writer's files with sizes, and what the throwaway check printed; (3) the const table as written; (4) for each of J0 to J6: the two plane names and entity indexes, the Damage command index, the Delete indexes with their Objects lists in order; the detector count; (5) what you found about a 1-plane `probe_flight`; (6) anything the table left open that you chose, and residual risks.
</structured_output_contract>
