# Delegation prompt for flight 2E (does Safe density hold for Pairs and the propeller built-ins?)

One job, in the worktree `eloquent-lumiere-a004d5` on `claude/p14-flight-2e` (cut from `claude/formation-safe-0019` at `29405cf`, plus this prompt and the design note). Sent with `codex_start`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1200. Model: the config default.

---

<task>
Build probe flight 2E: a no-pilot probe that flies four of the built-in sorties exactly as the app ships them after decision 0019 (every Formation command at `FormationDensity = 1`, Safe) over empty targets, to measure whether Safe also stops wingman collisions in Pairs flights and in the propeller types. Probe code only. The shell, the library, placement, the templates, `src/template.rs` and every flown file stay as they are.

Read first, in this order:
1. `handoff/sessions/2026-10-16-flight-2e.md` (the design; this prompt makes it exact).
2. `docs/decisions/0019-air-formations-fly-at-safe-density.md`; `handoff/P14-probe-results.md`, section "Flight 2D".
3. `CLAUDE.md`, `.cursorrules`, `docs/il2-loader-and-log-facts.md` (the 2026-10-15 section).
4. Code: `src/airtask/probe.rs` (flight 2D is the model: `Flight::TwoD`, `RUN_SHEET_2D`, `TWO_D_SORTIES`, `EmptyTargetVariant`, `empty_target_source`, `build_empty_target_probe`, `add_sortie_probe_source`, `close_sortie_probe`, `observe_sortie_probe`, `empty_target_walk_input`, `formation_schedule`, `write_empty_target_schedule`, `write_probe`, the `flight2d_…` tests, and the run-sheet and cue tables near line 9290 and 9447), `library.rs` (`inspect_sortie`, `SortieShape::side`, shapes S3 and S4), `place.rs` (`place_air_sortie` sets density 1 on every Formation command), `testkit.rs` (the walker).
5. The four templates in `TemplateExamples/Historical1950/`: `1950_US_F51_ArmedRecon_2plus2.Group` (S3), `1950_DPRK_Yak_AirfieldRaid_2plus2.Group` (S4), `1950_DPRK_Il10_KimpoAttack_2x4.Group` (S4), `1950_US_F51_CloseSupport_4ship.Group` (S1).

What to build, in `src/airtask/probe.rs`:

A. **`Flight::TwoE`**, stem `P14_Probe_2E`, not frozen, with a run sheet, cues, `generate_probe_2e()`, and an ignored writer `write_p14_probe_2e` that calls `write_probe(Flight::TwoE, TraceCarrier::default())` and writes the run sheet and schedule as flight 2D does (`target/p14/P14_Probe_2E_schedule.md`). Add it to `Flight::ALL` and to every exhaustive match. Use flight 2D's path; flights 2B, 2C and 2D must still produce the same output as now.

B. **Four cells, one placed and wrapped sortie each**, built through `build_empty_target_probe` as flight 2D's cells are (each with its own stand-in area, which stays COLD; no HOT switch, no enemy aircraft and no spawn anywhere in the mission):

| Cell | Prefix | Built-in file | Side | Shape | Aircraft | `attack_time_s` | START s | Target km (x, z) |
|---|---|---|---|---|---|---|---|---|
| EA | SN01 | `1950_US_F51_ArmedRecon_2plus2.Group` | NATO | S3 | 4 | 1200 | 60 | (230, −90) |
| EY | SN02 | `1950_DPRK_Yak_AirfieldRaid_2plus2.Group` | DPRK | S4 | 4 | 300 | 120 | (230, 0) |
| EI | SN03 | `1950_DPRK_Il10_KimpoAttack_2x4.Group` | DPRK | S4 | 8 | 600 | 180 | (185, 45) |
| EC | SN04 | `1950_US_F51_CloseSupport_4ship.Group` | NATO | S1 | 4 | 900 | 240 | (140, −90) |

That is 20 aircraft, on purpose: the last start takes the count from 16 to 20.

- **No edit of the sortie.** Add an `EmptyTargetVariant` for "built-in as shipped" that names its file and leaves the parsed source untouched before placement. The density comes from the files and from `place_air_sortie`, as for a user.
- **Side from the sortie.** `add_sortie_probe_source` passes `AirSide::Nato` to `build_side` and `emit_side_logic`, and `close_sortie_probe` draws a NATO icon. Take the side from the inspected source shape (`SortieShape::side`) instead, and the icon coalition from that side (`COALITION_NATO` / `COALITION_DPRK`). This is the only change to shared code; every earlier caller is NATO, so its output must not change by one byte.
- **`empty_target_source`'s checks per row**: the shape and `attack_time_s` in the table. If `inspect_sortie` refuses a file or gives another shape or time, stop and report the exact message; do not change `library.rs`.
- **Prediction**: no transit has been measured for these types. Predict `WP 1` at START + `TWO_TRANSIT_S` (300 s, the placement distance), and say in the schedule that it is nominal, not measured. Do not reuse `TWO_B_HVAR_TRANSIT_S`.
- **Targets**: four of flight 2C's twelve (C2c, C0c, C0b and C0a there). Keep them exactly. Each sortie's path runs from its spawn (target minus the placement distance along heading 0) to its target. Check in a test, with the placed copies' own lead Plane and `WP 1` positions: every target over land; every pair of paths at least 45 km apart (closest distance between the two segments; EA and EC lie on one line, about 47 km apart end to end); each NATO path at least 90 km from each DPRK path.

C. **Timing.** The host leaves at **15:00** (900 s). Check in a walker test (Restart, scripted `WP 1` arrivals at the predicted times): each copy's `ON STATION` starts once at its predicted time; at 900 s EA, EI and EC have been on station at least 300 s and their `DONE HUB` has not fired; EY's attack ends before 900 s, so state in the test and the schedule when its `DONE HUB` and `DELETE DELAY` are predicted to fire (that is the shipped 5 min attack; do not lengthen it). As a safety net, close every cell at 1,800 s with one `MCU_Delete` per aircraft entity in the cell (8 for EI), and a final cue, as flight 2D does.

D. **Cues** (subtitles, flight 2D's style):
- 0 s: "Flight 2E: spectate, then LCtrl+R to record. Note the first time the server overload message shows. Leave the server at 15:00" (20 aircraft; the overload time against the start cues is part of the result)
- at each cell's START: "Flight 2E: EA starts (4 aircraft flying)", "Flight 2E: EY starts (8 aircraft flying)", "Flight 2E: EI starts (16 aircraft flying)", "Flight 2E: EC starts (20 aircraft flying)"
- 870 s: "Flight 2E: leave the server now (15:00)"
- 1,800 s: "Flight 2E closed. Every aircraft has a Delete. End the mission normally"

E. **Breadcrumbs** as flight 2D (`observe_sortie_probe`). For S3 and S4 also observe every lead's attack timer (the existing loop covers every timer issuing an AttackArea; confirm it does for both leads) and the shared `WP 1`.

F. **No spawn.** `Flight::TwoE.spawns()` is empty; the writer writes `[]` as flight 2D's does.

G. **Tests** (`flight2e_…`), each stating its own literals:
- the four cells, keys, prefixes, files, sides, starts and targets match the table; land and path separation as in B;
- per cell: the source given to placement equals the parsed built-in file (no probe edit); every `MCU_CMD_Formation` in the placed copy has `FormationDensity = 1`, and its `FormationType` is the file's (EA 19 and 19, EY 19 and 19, EI 20 and 20, EC 20); plane `Country` is 601 for EA and EC, 503 for EY and EI;
- `inspect_sortie` gives the shape and `attack_time_s` of the table; `ON STATION` `Time` equals it;
- the side change: the NATO cells' side logic and icons are NATO, the DPRK cells' are DPRK;
- the walker run in C;
- the cues, with their times and texts, added to the shared run-sheet and cue tables;
- the existing byte and frozen-file tests (`flown_probes_regenerate_byte_for_byte_except_flight2`, `flight2_frozen_files_parse_and_links_resolve`, `placement_frozen_files_parse_and_links_resolve`, the `flight2d_…` tests) still pass unchanged.

Acceptance:
- Before editing, record the baseline: `cargo build --offline` and `cargo clippy --offline` warning counts, `cargo test --offline` passed / failed / ignored. After: build and clippy warnings not above the baseline; new code adds none.
- `cargo test --offline`: 0 failed; passed = baseline + the new tests.
- `cargo test --offline write_p14_probe_2e -- --ignored` writes `target/p14/P14_Probe_2E.Group`, `_traced.Group`, their `.eng`, the `.trace.json` sidecar, `.spawns.json` (if `write_probe` writes one) and the schedule, and touches nothing under `handoff/`.
- Every flown probe file under `handoff/p14-probes/` (flights 0 to 2D) is unchanged and still regenerates byte for byte where it did before.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_2E_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --name P14_Probe_2E_check --title "P14 probe 2E check" --out target/p14/missions/P14_Probe_2E_check --no-resave` exits 0 (check the script's real arguments first and adapt only their spelling), every `Index` unique, every reference resolved. Never run it without `--no-resave`. Do not copy anything into the game folder.

Files in scope: `src/airtask/probe.rs` only.

Do not touch: every other file, in particular `src/airtask/shell.rs`, `library.rs`, `place.rs`, `testkit.rs`, `budget.rs`, `assemble.rs`, `mod.rs`, `src/template.rs`, `src/trace.rs`, `src/testdata/`, `TemplateExamples/`, `handoff/` (all of it), `docs/`, `tools/`, `STATE.md`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, and the game folder. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `claude/p14-flight-2e` is checked out in this worktree; its only uncommitted or new files are this prompt and the design note, which are not yours to change. Edit files in the working tree only. No git command that writes. `git status`, `git diff` and `git log` are allowed. If the working tree holds other changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not weaken a test to make it pass; report the value and the reason.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run `cargo build --offline`, `cargo clippy --offline`, `cargo test --offline`, the ignored writer, the byte comparison of `handoff/p14-probes/` and the mission builder check, and read the counts from their output. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now: const tables with named fields that read like this prompt's tables, shared helpers, named constants, no flag arguments, no behaviour keyed on a bare number, failures that name the cell or missing part. No refactor, rename or reformat of code outside the change; where flight 2D's code is reused, extract the smallest helper that keeps every earlier flight's output byte-identical.
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
5. The run sheet: per cell the file, side, shape, prefix, START, predicted `WP 1`, `ON STATION` start and end, predicted `DONE HUB` and `DELETE DELAY` if before 1,800 s, target (X, Z), spawn (X, Z), and the closest other path's distance.
6. Each cell's Formation nodes as written (index, `FormationType`, `FormationDensity`, `Objects`, who targets them).
7. What the writer does about spawns.
8. Choices you made where this prompt gave no exact shape, and residual risks.
</structured_output_contract>
