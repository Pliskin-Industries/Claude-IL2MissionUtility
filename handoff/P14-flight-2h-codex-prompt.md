# Delegation prompt for flight 2H (does Pairs at Loose hold for the F-51 ArmedRecon?)

One job, in the worktree `distracted-taussig-b9259d` on `claude/p14-flight-2h` (cut from `claude/p14-flight-2g-be3801` at `fdcac1d`, plus this prompt and the design note, committed). Sent with `codex_start`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1200. Model: the config default.

---

<task>
Build probe flight 2H: a no-pilot probe on flight 2F's path that flies the F-51 ArmedRecon built-in three times at once, once as shipped (Pairs at Safe) and twice at Pairs Loose, over empty targets. It measures whether decision 0020's option A (Pairs fly Loose) holds for this built-in. Probe code only. The shell, the library, placement, the templates, `src/template.rs` and every flown file stay as they are.

Read first, in this order:
1. `handoff/sessions/2026-10-18-flight-2h.md` (the design; this prompt makes it exact).
2. `docs/decisions/0020-pairs-formations-after-flight-2e.md`; `handoff/P14-probe-results.md`, sections "Flight 2E" (cell EA) and "Flight 2F".
3. `CLAUDE.md`, `.cursorrules`.
4. Code: `src/airtask/probe.rs`. Flight 2F is the model: `Flight::TwoF`, `RUN_SHEET_2F`, `TWO_F_SORTIES`, `TWO_F_*`, `ProbeTarget`, `EmptyTargetVariant::BuiltInFormation`, `set_placed_probe_formation`, `build_empty_target_probe`, `builtin_formation_schedule`, `append_probe_formations`, `append_probe_aircraft_intervals`, `write_empty_target_schedule`, `write_probe`, the `flight2f_…` tests and the shared run-sheet and cue tables. Flight 2G (`Flight::TwoG`, `flight2g_…` tests) is the most recent example of adding a flight. Flight 2E's EA row (`TWO_E_SORTIES`) is this built-in as shipped. Also `place.rs`, `library.rs` (`inspect_sortie`, shape S3), `testkit.rs` (the walker).
5. `TemplateExamples/Historical1950/1950_US_F51_ArmedRecon_2plus2.Group` (S3: Red pair lead entity 7 attacks, AttackArea 1,200 s; Blue pair lead entity 11 covers it with `MCU_CMD_Cover`; Formation commands Objects [7] and [11], both `FormationType = 19`, `FormationDensity = 1`; `WP 1` Objects [7]).

What to build, in `src/airtask/probe.rs`:

A. **`Flight::TwoH`**, stem `P14_Probe_2H`, not frozen and not yet regenerating flown files (as `TwoG` was before it flew), with a run sheet, cues, `generate_probe_2h()`, and an ignored writer `write_p14_probe_2h` that writes the files and the schedule (`target/p14/P14_Probe_2H_schedule.md`) as flight 2F does. Add it to `Flight::ALL` and every exhaustive match. Flights 0 to 2G must still produce exactly the output they do now.

B. **Three cells, one placed and wrapped F-51 ArmedRecon each**, from `1950_US_F51_ArmedRecon_2plus2.Group` (NATO, S3, 4 aircraft, `attack_time_s` 1200), through `build_empty_target_probe` with `EmptyTargetVariant::BuiltInFormation` (own stand-in area, COLD; no HOT switch, no enemy, no spawn):

| Cell | Prefix | After placement: `FormationType`, `FormationDensity` | Target km (x, z) | START s |
|---|---|---|---|---|
| HS | SN01 | 19, 1 (Pairs, Safe: as shipped) | (230, −90): flight 2E EA's target | 60 |
| HL1 | SN02 | 19, 2 (Pairs, Loose) | A (230, 0): `TWO_F_TARGET_A` | 90 |
| HL2 | SN03 | 19, 2 (Pairs, Loose) | B (140, 0): `TWO_F_TARGET_B` | 120 |

12 aircraft, all alive together.

- **Transit**: a named constant 318.5 s, with its source in a comment (flight 2E EA reached `WP 1` at 378.5 s for START 60 s in all three runs). `WP 1` predicted at START + 318.5 s.
- **`empty_target_source`'s checks** (already generic for `BuiltInFormation`): S3 and 1200. If `inspect_sortie` refuses the file or gives another shape or time, stop and report the exact message; do not change `library.rs`. If `set_placed_probe_formation` rejects the S3 copy (it requires each Formation command's `Objects` to name one inspected flight lead), stop and report.
- **Paths**, checked in tests with the placed copies' own lead Plane and `WP 1` positions: all three targets over land; every pair of paths at least 45 km apart (A–B 48.3 km end to end, both about 90 km from HS's line).

C. **Timing.** The host leaves at **28:00** (1,680 s). Walker test (Restart, scripted `WP 1` arrivals at the predicted times): each copy's `ON STATION` starts once at its predicted time, and its `DONE HUB` and `DELETE DELAY` fire at the predicted times (HS 1579.0 / 1639.6, HL1 1609.0 / 1669.6, HL2 1639.0 / 1699.6; follow the walker where it differs and report the difference); every attack has ended before 1,680 s. Do not change the shipped 1,200 s attack. Safety net: close every cell at **2,100 s** with one `MCU_Delete` per aircraft entity, and a final cue.

D. **Cues** (subtitles, flight 2F's style):
- 0 s: "Flight 2H: spectate, click the sky, then LCtrl+R to record. Leave the server at 28:00"
- at each START: "Flight 2H: HS starts (Pairs, Safe)", "Flight 2H: HL1 starts (Pairs, Loose)", "Flight 2H: HL2 starts (Pairs, Loose)"
- 1,650 s: "Flight 2H: leave the server at 28:00"
- 2,100 s: "Flight 2H closed. Every aircraft has a Delete. End the mission normally"

E. **Breadcrumbs** as flight 2F (`observe_sortie_probe`), including the attack timer(s) and `WP 1`.

F. **No spawn.** `Flight::TwoH.spawns()` is empty.

G. **Schedule**: per cell the formation as set, prefix, target, START, predicted `WP 1` (say it is from 2E's measured transit), `ON STATION`, `DONE HUB`, `DELETE DELAY`, target and spawn (X, Z); the aircraft alive in each interval; the Formation commands as written (`append_probe_formations`). Reuse flight 2F's schedule code where it fits; extract the smallest helper needed and keep 2F's and 2G's schedules unchanged.

H. **Tests** (`flight2h_…`), each stating its own literals:
- cells, keys, prefixes, formations, transit, starts and targets match the table; land and path separation as in B;
- per cell: the source given to placement equals the parsed file and inspects as S3 with 1200 s (placed copies inspect as S7: do not require S3 there); in the placed copy both `MCU_CMD_Formation` commands have the row's `FormationType` and `FormationDensity` and the file's `Objects`; every Plane `Country` is the file's;
- **only the Formation commands differ**: each cell's sortie equals the same file placed and wrapped as shipped (same cell, prefix and ids) except `FormationType` and `FormationDensity` on its Formation commands; for HS nothing differs;
- side logic and icons are NATO in every cell;
- the walker run in C;
- the cues, with their times and texts, in the shared run-sheet and cue tables;
- the existing byte and frozen-file tests and the `flight2d_…` to `flight2g_…` tests still pass unchanged (the flown-file test still compares 52 files).

Acceptance:
- Before editing, record the baseline: `cargo build --offline` and `cargo clippy --offline` warning counts (expected 69 / 251), `cargo test --offline` passed / failed / ignored (expected 686 / 0 / 26). After: warnings not above the baseline; new code adds none.
- `cargo test --offline`: 0 failed; passed = baseline + the new tests; ignored = baseline + 1.
- `cargo test --offline write_p14_probe_2h -- --ignored` writes `target/p14/P14_Probe_2H.Group`, `_traced.Group`, their `.eng`, the `.trace.json` sidecar, `.spawns.json` and the schedule, and touches nothing under `handoff/`.
- Every flown probe file under `handoff/p14-probes/` (flights 0 to 2G) is unchanged and still regenerates byte for byte where it did before.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_2H_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --name P14_Probe_2H_check --title "P14 probe 2H check" --out target/p14/missions/P14_Probe_2H_check --no-resave` exits 0 (if `python` is not on PATH, use the installed Python 3 executable), every `Index` unique, every reference resolved. Never run it without `--no-resave`. Do not copy anything into the game folder.

Files in scope: `src/airtask/probe.rs` only.

Do not touch: every other file, in particular `src/airtask/shell.rs`, `library.rs`, `place.rs`, `testkit.rs`, `budget.rs`, `assemble.rs`, `mod.rs`, `src/template.rs`, `src/trace.rs`, `src/testdata/`, `TemplateExamples/`, `handoff/` (all of it), `docs/`, `tools/`, `STATE.md`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, and the game folder. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `claude/p14-flight-2h` is checked out in this worktree and its working tree is clean when you start. Edit files in the working tree only. No git command that writes. `git status`, `git diff` and `git log` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not weaken a test to make it pass; report the value and the reason.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run `cargo build --offline`, `cargo clippy --offline`, `cargo test --offline`, the ignored writer, the byte comparison of `handoff/p14-probes/` and the mission builder check, and read the counts from their output. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now: const tables with named fields that read like this prompt's tables, shared helpers, named constants, no flag arguments, no behaviour keyed on a bare number, failures that name the cell or missing part. No refactor, rename or reformat of code outside the change; where flight 2F's code is reused, extract the smallest helper that keeps every earlier flight's output byte-identical.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Where this prompt and the code disagree, follow the code for facts and report each difference.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary, five lines at most.
2. Command results: baseline and final build and clippy warnings, tests passed / failed / ignored, the byte comparison, the mission builder check.
3. Every file changed, one line each on why.
4. Tests added or changed, by name.
5. The run sheet: per cell the formation as set, prefix, target, START, predicted `WP 1`, `ON STATION` start and end, predicted `DONE HUB` and `DELETE DELAY`, target (X, Z), spawn (X, Z); the most aircraft alive at once.
6. Each cell's Formation nodes as written (index, `FormationType`, `FormationDensity`, `Objects`, who targets them).
7. Choices you made where this prompt gave no exact shape, and residual risks.
</structured_output_contract>
