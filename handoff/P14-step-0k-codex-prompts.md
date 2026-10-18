# Delegation prompts for step 0k (decision 0016, then flight 1N)

Two jobs, run one after the other in the worktree `Claude-IL2MissionUtility-0e`. The orchestrator commits after each. Shared rules are at the end and go with both.

---

## Job 1. Decision 0016: each Fighter Pack aircraft is counted once (branch `codex/fix-0016`)

<task>
Read `docs/decisions/0016-fighter-pack-counts-each-aircraft-once.md` (accepted, option B), `docs/decisions/0012-aircraft-death-event-outside-air-tasking.md`, and in `handoff/P14-probe-results.md` the sections "Flight 1L" (cell T-v: the events a kill raises) and "Flight 1M" (cell T-j: a re-triggered timer restarts and fires once).

Change in `src/flights.rs` (`install_pack_logic` and what it calls): each aircraft's death event (`OnKilled`, 13) targets that aircraft's own `Delay Delete n` timer, not the flight's `DeathCount`. `Delay Delete n` keeps targeting `Deactivate n` and `DeathCount`. Nothing else changes: the other Fighter Pack hooks (`FIGHTER_EVENTS`) still go to `CLEANUP n`.

Tests first: for a generated pack, each plane entity has exactly one `OnEvent` of type 13 and it targets that plane's `Delay Delete n`; no `OnEvent` targets `DeathCount` directly; `Delay Delete n` targets `Deactivate n` and its flight's `DeathCount`. Use the shared OnEvent reader beside `attach_event`.

Update `USER_MANUAL.md` where it describes Fighter Pack's counting (find it by content), in one or two plain sentences: a lost aircraft is counted once, when its clean-up timer runs out.

Baselines: `src/testdata/p14_baseline/fighter.Group` (and any other baseline with a Fighter Pack) changes only in the `TarId` of the death `OnEvent` of each aircraft. Re-record it and report, per file, the number of changed lines and that every one is such a `TarId`.

Acceptance: `cargo test --offline` 0 failed, no fewer tests than before; warnings not above 69 build, 251 clippy; the probe flights generate byte for byte the same files as before (compare against `handoff/p14-probes/*/`); nothing under `src/testdata/p14_probe_inputs/` or `TemplateExamples/` changes.

Files in scope: `src/flights.rs`, `USER_MANUAL.md`, Fighter Pack baselines as stated.
</task>

---

## Job 2. Step 0k, flight 1N (branch `codex/p14-0k`, after job 1 is merged)

<task>
Build P14 step 0k in this repository: one new in-game probe file, flight 1N.

Read first, in this order:
1. `handoff/P14-air-tasking.md` §8 "Step 0k. Upstream's go-home fix and Fighter Pack's count (flight 1N)" (the spec), and step 0j above it.
2. `handoff/P14-probe-results.md`, section "Flight 1M".
3. `src/testdata/p14_probe_inputs/upstream_a31ad0a/README.md` and its two strike files.
4. `docs/decisions/0016-fighter-pack-counts-each-aircraft-once.md`.
5. `src/airtask/probe.rs`: all of it. Flight 1M's builders (cell T-u's loader, placer and listeners; cell T-x's frozen inputs and their generator check) are the pattern for this step.
6. `src/flights.rs` (`configure_aircraft`, `install_pack_logic`, `FlightConfig`) for the Fighter Pack flight.
7. `docs/il2-loader-and-log-facts.md`, `CLAUDE.md`, `.cursorrules`.

What to build, in `src/airtask/probe.rs` and new fixture files:
- `Flight::OneN`, stem `P14_Probe_1N`, `RUN_SHEET_1N` with the cells `T-h` and `T-f`, the two cues, `generate_probe_1n()`, an ignored writer `write_p14_probe_1n`, an empty spawn list.
- Cell T-h as its table, on flight 1M's T-u machinery. The row's change is a named variant (no change; once-only gate; no Force Complete; event 7 hooks removed plus gate). The new `MCU_Counter` is copied from an existing counter block.
- Cell T-b as its table: twelve single aircraft from `generate_template`, frozen the same way, with a README naming each.
- Cell T-f as its table. Write the Fighter Pack flight once with an ignored, overwrite-refusing writer into `src/testdata/p14_probe_inputs/flight1n/`, with a README and a test that regenerates it and compares bytes. F2 removes the four event 7 hooks in the probe; F3 also re-points the four death events to `DeathCount` (named changes, applied to copies of the one frozen file). Report the pack's `DeathCount` `Counter`, its `COOLDOWN` time, the node that starts it, and where each of its event 7 hooks leads; if the replacement chain needs a zone or a player, stop and report before building the row.
- Icons per row centre, `COALITION_NATO`. Close at 16:00 with one `MCU_Delete` per aircraft entity.
- Tests: every test named in "Step 0k acceptance (tests)", rows against literals.

Acceptance:
- `cargo test --offline`: 0 failed, no fewer tests than before, plus the new ignored writers. `cargo build --offline` at most 69 warnings; `cargo clippy --offline` at most 251.
- `cargo test --offline write_p14_probe_ -- --ignored` writes the 1N files.
- Every earlier flight generates byte for byte the same files as before (baseline first; compare against `handoff/p14-probes/*/` for the flown ones).
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_1N_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10 --name P14_Probe_1N_check --title "P14 probe 1N check" --out target/p14/missions/P14_Probe_1N_check --no-resave` exits 0, every `Index` unique and every reference resolved (upstream `Trigger Delete` lists resolve to `Plane` blocks; say so). Throwaway check script, not added to the repo. Never run the builder without `--no-resave`.

Files in scope: `src/airtask/probe.rs`; new files under `src/testdata/p14_probe_inputs/flight1n/`.
Do not touch: `src/template.rs`, `src/flights.rs`, anything else under `src/testdata/p14_probe_inputs/`, and every other file.
</task>

---

## Sent with both jobs

<git_protocol>
Same-host mode. The orchestrator has checked out the branch in this worktree. Edit files in the working tree only. No git command that writes. `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why.
</completeness_contract>

<verification_loop>
Run `cargo test --offline`, `cargo build --offline`, `cargo clippy --offline` and `cargo test --offline write_p14_probe_ -- --ignored`, and compare generated probe files with a baseline made before you edited. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now: const tables with named fields that read like the plan's tables, shared helpers, named constants, no flag arguments, no behaviour keyed on a bare number, failures that name the flight, group and missing thing. Tests state their own literals.
</action_safety>

<structured_output_contract>
Return: (1) summary, five lines at most; (2) command results and the byte comparisons; (3) every file changed, one line each on why; (4) every baseline or fixture written or re-recorded, with proof that only the expected lines changed; (5) for job 2: the const tables, a schedule of every command and pulse in time order, and the Fighter Pack facts asked for; (6) choices you made and residual risks.
</structured_output_contract>
