# Delegation prompt for flight 2D (does a looser formation stop the wingman collisions?)

One job, in the worktree `distracted-taussig-b9259d` on `codex/p14-2d` (cut from `codex/p14-2c` at `f5f3c93`, plus this prompt and the design note). Sent with `codex_start`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1200. Model: the config default.

---

<task>
Build probe flight 2D: a no-pilot probe that measures whether a looser formation stops AI F-80C 4-ships colliding while they fly to and hold an AttackArea over an empty target, with few enough aircraft that the test-bed server is not overloaded. Probe code only. The shell, the library, the templates, `src/template.rs` and every flown file stay as they are.

Read first, in this order:
1. `handoff/sessions/2026-10-15-hvar-empty-target.md`, sections "After flight 2C" and "Flight 2D design" (why this flight exists; this prompt makes the design exact).
2. `handoff/P14-probe-results.md`, section "Flight 2C".
3. `CLAUDE.md`, `.cursorrules`, `docs/il2-loader-and-log-facts.md` (the 2026-10-15 section).
4. Code: `src/airtask/probe.rs` (flight 2C is the model: `Flight::TwoC`, `RUN_SHEET_2C`, `TWO_C_SORTIES`, `EmptyTargetVariant`, `build_empty_target_probe`, `empty_target_walk_input`, `empty_target_schedule`, `write_empty_target_schedule`, `write_probe`, the `flight2c_…` tests), `library.rs` (`inspect_sortie`, how it accepts the optional Formation before the first timer), `place.rs`, `testkit.rs` (the walker).
5. The template: `TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group` (its one `MCU_CMD_Formation`, index 35: `FormationType = 20`, `FormationDensity = 0`, `Objects = [7]`).

What to build, in `src/airtask/probe.rs`:

A. **`Flight::TwoD`**, stem `P14_Probe_2D`, with a run sheet, cues, `generate_probe_2d()`, and an ignored writer `write_p14_probe_2d` that calls `write_probe(Flight::TwoD, TraceCarrier::default())` and writes a run sheet and schedule as flight 2C does (`target/p14/P14_Probe_2D_schedule.md`). Share flight 2C's code where the shape is the same; flight 2C must still generate the same bytes as its frozen files.

B. **Four cells, one placed and wrapped NATO sortie each**, built as flight 2C builds its C0 cells (each with its own stand-in area, which stays COLD; no HOT switch, no enemy aircraft anywhere in the mission). Every cell is the HVAR template as shipped except its one `MCU_CMD_Formation`:

| Cell | Prefix | START s | Target km (x, z) | `FormationType` | `FormationDensity` | Editor label |
|---|---|---|---|---|---|---|
| D0 | SN01 | 60 | (185, −90) | 20 (Wedge) | 0 | Dense (control: as shipped) |
| D1 | SN02 | 120 | (230, −90) | 20 | 1 | Safe |
| D2 | SN03 | 180 | (185, −45) | 20 | 2 | Loose |
| DP | SN04 | 240 | (230, 45) | 19 (Pairs) | 0 | Dense |

Change only those two properties of that one Formation node, on the parsed source before placement. If the source does not have exactly one `MCU_CMD_Formation` whose `Objects` is the navigation lead, fail with a message naming the cell. `inspect_sortie` must accept every changed source as S1 with `attack_time_s = 600`; if it refuses one, stop and report the exact message; do not change `library.rs`.

The targets are four of flight 2C's twelve (C1b, C2c, C2b and C1c there), chosen because their settled height above ground in flight 2C was alike (about 500 to 720 m). Keep them exactly; do not move them. They are over land and at least 45 km apart; a test checks both, as flight 2C's does.

The starts are 60 s apart on purpose: the host notes the first time the "server overload" message shows, and the start cues turn that into an aircraft count.

C. **Timing.** Predict `WP 1` with flight 2B's measured HVAR transit (`TWO_B_HVAR_TRANSIT_S`). The host leaves the server at **15:00** (900 s), so the end-of-mission list holds every live aircraft. Check in a test that at 900 s every copy has been on station at least 300 s after its predicted `WP 1` and no copy's `DONE HUB` has fired. As a safety net, close every cell at 1,800 s with one `MCU_Delete` per aircraft entity still in the file, and a final cue, as flight 2C does.

D. **Cues** (subtitles, flight 2C's style):
- 0 s: "Flight 2D: spectate, then LCtrl+R to record. Note the first time 'server overload' shows. Leave the server at 15:00"
- at each cell's START: "Flight 2D: D0 starts (4 aircraft flying)", then D1 (8), D2 (12), DP (16)
- 870 s: "Flight 2D: leave the server now (15:00)"
- 1,800 s: the close cue, as flight 2C's.

E. **Breadcrumbs** as flight 2C: on each copy's `START`, `WP 1`, the attack timer, `ON STATION`, `COLD`, `EXTEND`, `HARD STOP`, `LOSSES`, Zone IN, Zone Out, `DONE HUB` and `DELETE DELAY`.

F. **No spawn.** `Flight::TwoD` has no spawn rows (Tacview from a spectating host works; see the 2026-10-15 facts). The writer must then write no `.spawns.json`, or an empty one, whichever `write_probe` already does for flights without spawns; say which.

G. **Tests** (`flight2d_…`), each stating its own literals:
- the four cells, keys, prefixes, starts and targets match the table; targets over land and at least 45 km apart;
- per cell: the Formation node's `FormationType` and `FormationDensity` as in the table, and the placed copy differs from a D0 copy built the same way only in those two properties (compare after normalising the names, indices and positions that placement changes anyway; say how);
- `inspect_sortie` gives S1 and `attack_time_s = 600` for every cell; `ON STATION` `Time` is 600;
- a walker run (Restart) with scripted `WP 1` arrivals: each copy's `ON STATION` starts once at the predicted time, and the 900 s condition in C holds;
- the cues, with their times and texts;
- flight 2C still generates the same bytes as its frozen files in `handoff/p14-probes/flight2c/` (if an existing test already covers this, name it instead).

Acceptance:
- Before editing, record the baseline: `cargo build --offline` and `cargo clippy --offline` warning counts, `cargo test --offline` passed / failed / ignored. After: build and clippy warnings not above the baseline; new code adds none.
- `cargo test --offline`: 0 failed; passed = baseline + the new tests.
- `cargo test --offline write_p14_probe_2d -- --ignored` writes `target/p14/P14_Probe_2D.Group`, `_traced.Group`, their `.eng`, the `.trace.json` sidecar and the schedule, and touches nothing under `handoff/`.
- Every flown probe file under `handoff/p14-probes/` (flights 0 to 2C) is unchanged and still regenerates byte for byte where it did before.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_2D_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --name P14_Probe_2D_check --title "P14 probe 2D check" --out target/p14/missions/P14_Probe_2D_check --no-resave` exits 0 (check the script's real arguments first and adapt only their spelling), every `Index` unique, every reference resolved. Never run it without `--no-resave`. Do not copy anything into the game folder.

Files in scope: `src/airtask/probe.rs` only.

Do not touch: every other file, in particular `src/airtask/shell.rs`, `library.rs`, `place.rs`, `testkit.rs`, `budget.rs`, `assemble.rs`, `mod.rs`, `src/template.rs`, `src/trace.rs`, `src/testdata/`, `TemplateExamples/`, `handoff/` (all of it), `docs/`, `tools/`, `STATE.md`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, and the game folder. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `codex/p14-2d` is checked out in this worktree and clean. Edit files in the working tree only. No git command that writes. `git status`, `git diff` and `git log` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not weaken a test to make it pass; report the value and the reason.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run `cargo build --offline`, `cargo clippy --offline`, `cargo test --offline`, the ignored writer, the byte comparison of `handoff/p14-probes/` and the mission builder check, and read the counts from their output. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now: const tables with named fields that read like this prompt's tables, shared helpers, named constants, no flag arguments, no behaviour keyed on a bare number, failures that name the cell or missing part. No refactor, rename or reformat of code outside the change; where flight 2C's code is reused, extract the smallest helper that keeps flight 2C's output byte-identical.
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
5. The run sheet: per cell the formation, prefix, START, predicted `WP 1`, `ON STATION` start and end, target (X, Z), and the nearest other target's distance.
6. Each cell's Formation node as written (index, `FormationType`, `FormationDensity`, `Objects`, who targets it).
7. What the writer does about spawns.
8. Choices you made where this prompt gave no exact shape, and residual risks.
</structured_output_contract>
