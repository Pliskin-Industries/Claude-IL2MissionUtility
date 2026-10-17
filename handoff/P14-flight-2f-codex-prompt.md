# Delegation prompt for flight 2F (is it the server or the formation? The Yak raid in six formations at low load)

One job, in the worktree `youthful-swartz-88badf` on `claude/p14-flight-2f` (cut from `claude/p14-flight-2e` at `f185199`, plus this prompt and the design note, committed). Sent with `codex_start`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1200. Model: the config default.

---

<task>
Build probe flight 2F: a no-pilot probe on flight 2E's path that flies the Yak airfield raid six times in turn, one formation each (Pairs and Wedge, each at Dense, Safe and Loose), over empty targets, with at most two raids (8 aircraft) in the air at once. It measures whether the wingman collisions seen in flight 2E follow the formation, or happen in any formation when the server is lightly loaded. Probe code only. The shell, the library, placement, the templates, `src/template.rs` and every flown file stay as they are.

Read first, in this order:
1. `handoff/sessions/2026-10-16-flight-2f.md` (the design; this prompt makes it exact).
2. `docs/decisions/0020-pairs-formations-after-flight-2e.md`; `handoff/P14-probe-results.md`, sections "Flight 2E" and "Flight 2D".
3. `CLAUDE.md`, `.cursorrules`.
4. Code: `src/airtask/probe.rs`, flight 2E is the model: `Flight::TwoE`, `RUN_SHEET_2E`, `TWO_E_SORTIES`, `EmptyTargetVariant` (`BuiltIn`, `Formation`), `ProbeFormation`, `EmptyTargetSortie::predicted_transit_s`, `empty_target_source`, `set_probe_formation`, `build_empty_target_probe`, `add_sortie_probe_source`, `close_sortie_probe`, `observe_sortie_probe`, `empty_target_walk_input`, `builtin_sortie_schedule`, `append_probe_formations`, `write_empty_target_schedule`, `write_probe`, the `flight2e_…` tests, and the shared run-sheet and cue tables (search for `(Flight::TwoE, "EA"`). Also `place.rs` (`place_air_sortie` sets density 1 on every Formation command, through `src/template.rs`), `library.rs` (`inspect_sortie`, shape S4), `testkit.rs` (the walker).
5. `TemplateExamples/Historical1950/1950_DPRK_Yak_AirfieldRaid_2plus2.Group` (S4; Formation 35 Objects [7], Formation 41 Objects [11], both `FormationType = 19`, `FormationDensity = 1`; WP speed 500; attack time 300 s).

What to build, in `src/airtask/probe.rs`:

A. **`Flight::TwoF`**, stem `P14_Probe_2F`, not frozen, with a run sheet, cues, `generate_probe_2f()`, and an ignored writer `write_p14_probe_2f` that calls `write_probe(Flight::TwoF, TraceCarrier::default())` and writes the run sheet and schedule as flight 2E does (`target/p14/P14_Probe_2F_schedule.md`). Add it to `Flight::ALL` and to every exhaustive match. Flights 2B to 2E must still produce exactly the output they do now.

B. **Six cells, one placed and wrapped Yak raid each**, all from `1950_DPRK_Yak_AirfieldRaid_2plus2.Group` (DPRK, S4, 4 aircraft, `attack_time_s` 300, measured transit 264.1 s), through `build_empty_target_probe` as flight 2E's cells are (own stand-in area, COLD; no HOT switch, no enemy aircraft, no spawn anywhere):

| Cell | Prefix | After placement: `FormationType`, `FormationDensity` | Target | Target km (x, z) | START s |
|---|---|---|---|---|---|
| PD | SN01 | 19, 0 (Pairs, Dense) | A | (230, 0) | 60 |
| PS | SN02 | 19, 1 (Pairs, Safe: as shipped) | B | (140, 0) | 390 |
| PL | SN03 | 19, 2 (Pairs, Loose) | A | (230, 0) | 720 |
| WD | SN04 | 20, 0 (Wedge, Dense) | B | (140, 0) | 1050 |
| WS | SN05 | 20, 1 (Wedge, Safe) | A | (230, 0) | 1380 |
| WL | SN06 | 20, 2 (Wedge, Loose) | B | (140, 0) | 1710 |

24 aircraft in the mission, at most 8 alive at once. A is flight 2C's C0c (2E's Yak target), B is flight 2C's C2a; keep them exactly. Each target is used by three cells in turn; that is intended.

- **One new `EmptyTargetVariant`** for "built-in as shipped, then a formation set after placement": it names the file, shape, `attack_time_s`, the measured transit and a `ProbeFormation` (reuse the struct; `formation_name` Pairs / Wedge, `editor_label` Dense / Safe / Loose, as flight 2D). Every 2F row uses it, the control too: PS's formation equals the file's own, so its placed copy must come out identical to an as-shipped placement (a test checks this). The source given to placement is the parsed file, untouched, as in 2E.
- **The formation is set after placement**, because `place_air_sortie` overwrites every `FormationDensity` with 1. Set `FormationType` and `FormationDensity` on every `MCU_CMD_Formation` of the cell's placed copy, and nothing else. Prefer doing it in probe code after `add_sortie_probe_source` returns (the placed copy is the `"{cell} sortie"` group under the cell, see `Probe::add`), so that `add_sortie_probe_source` does not change. If that cannot work, make the smallest change to `add_sortie_probe_source` that keeps every earlier caller's output byte-identical, with no flag argument. Fail with the cell name if a placed copy has no Formation command or if a command's `Objects` is not a flight lead of the inspected shape.
- **Prediction**: `WP 1` at START + 264.1 s, the Yak transit measured in flight 2E (`WP 1` at 384.1 s for START 120 s, every run within 0.1 s), as a named constant with that source in a comment. Flight 2E's `BuiltIn` rows keep `TWO_TRANSIT_S`; do not change their prediction.
- **`empty_target_source`'s checks per row**: shape S4 and `attack_time_s` 300. If `inspect_sortie` refuses the file or gives another shape or time, stop and report the exact message; do not change `library.rs`.
- **Paths and reuse**, checked in tests with the placed copies' own lead Plane and `WP 1` positions: both targets over land; the A and B paths (spawn to target) at least 45 km apart (they lie end to end on z = 0, about 48.3 km); for each target, the next cell there starts after the previous cell's predicted `DELETE DELAY` (PD 685.2 before PL 720; PS 1015.2 before WD 1050; PL 1345.2 before WS 1380; WD 1675.2 before WL 1710); and at no predicted moment are more than two cells alive (START to `DELETE DELAY`).

C. **Timing.** The host leaves at **38:30** (2,310 s). Walker test (Restart, scripted `WP 1` arrivals at the predicted times): each copy's `ON STATION` starts once at its predicted time, and its `DONE HUB` and `DELETE DELAY` fire at the predicted times (design table: PD 624.6 / 685.2, PS 954.6 / 1015.2, PL 1284.6 / 1345.2, WD 1614.6 / 1675.2, WS 1944.6 / 2005.2, WL 2274.6 / 2335.2; follow the walker where it differs and report the difference); every attack has ended before 2,310 s. Do not lengthen the shipped 300 s attack. Safety net as 2E: close every cell at **2,700 s** with one `MCU_Delete` per aircraft entity, and a final cue.

D. **Cues** (subtitles, flight 2E's style):
- 0 s: "Flight 2F: spectate, then LCtrl+R to record. Check the menu now and then for the server overload message. Leave the server at 38:30"
- at each START: "Flight 2F: PD starts (Pairs, Dense)", "Flight 2F: PS starts (Pairs, Safe)", "Flight 2F: PL starts (Pairs, Loose)", "Flight 2F: WD starts (Wedge, Dense)", "Flight 2F: WS starts (Wedge, Safe)", "Flight 2F: WL starts (Wedge, Loose)"
- 2,280 s: "Flight 2F: leave the server at 38:30"
- 2,700 s: "Flight 2F closed. Every aircraft has a Delete. End the mission normally"

E. **Breadcrumbs** as flight 2E (`observe_sortie_probe`), including both S4 leads' attack timers and the shared `WP 1`.

F. **No spawn.** `Flight::TwoF.spawns()` is empty; the writer writes `[]` as 2E's does.

G. **Schedule** (`target/p14/P14_Probe_2F_schedule.md`): per cell the formation as set (type, density, names), prefix, target, START, predicted `WP 1` (say it is from 2E's measured transit), `ON STATION`, `DONE HUB`, `DELETE DELAY`, target and spawn (X, Z); the number of aircraft alive in each interval; and the Formation commands as written (`append_probe_formations`).

H. **Tests** (`flight2f_…`), each stating its own literals:
- cells, keys, prefixes, formations, transit, starts and targets match the table; land, path separation and target reuse as in B;
- per cell: the source given to placement equals the parsed file; in the placed copy both `MCU_CMD_Formation` commands have the row's `FormationType` and `FormationDensity` and the same `Objects` (flight leads) as the file; every plane `Country` is 503;
- **only the Formation commands differ**: each 2F cell's sortie equals the same file placed and wrapped as shipped (flight 2E's path, same cell, prefix and ids) except `FormationType` and `FormationDensity` on its Formation commands; for PS nothing differs;
- `inspect_sortie` gives S4 and 300; `ON STATION` `Time` is 300;
- side logic and icons are DPRK in every cell;
- the walker run in C;
- the cues, with their times and texts, in the shared run-sheet and cue tables;
- the existing byte and frozen-file tests (`flown_probes_regenerate_byte_for_byte_except_flight2`, `flight2_frozen_files_parse_and_links_resolve`, `placement_frozen_files_parse_and_links_resolve`, the `flight2d_…` and `flight2e_…` tests) still pass unchanged.

Acceptance:
- Before editing, record the baseline: `cargo build --offline` and `cargo clippy --offline` warning counts (expected 69 / 251), `cargo test --offline` passed / failed / ignored. After: warnings not above the baseline; new code adds none.
- `cargo test --offline`: 0 failed; passed = baseline + the new tests; ignored = baseline + 1.
- `cargo test --offline write_p14_probe_2f -- --ignored` writes `target/p14/P14_Probe_2F.Group`, `_traced.Group`, their `.eng`, the `.trace.json` sidecar, `.spawns.json` and the schedule, and touches nothing under `handoff/`.
- Every flown probe file under `handoff/p14-probes/` (flights 0 to 2E) is unchanged and still regenerates byte for byte where it did before.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_2F_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --name P14_Probe_2F_check --title "P14 probe 2F check" --out target/p14/missions/P14_Probe_2F_check --no-resave` exits 0 (check the script's real arguments first and adapt only their spelling), every `Index` unique, every reference resolved. Never run it without `--no-resave`. Do not copy anything into the game folder.

Files in scope: `src/airtask/probe.rs` only.

Do not touch: every other file, in particular `src/airtask/shell.rs`, `library.rs`, `place.rs`, `testkit.rs`, `budget.rs`, `assemble.rs`, `mod.rs`, `src/template.rs`, `src/trace.rs`, `src/testdata/`, `TemplateExamples/`, `handoff/` (all of it), `docs/`, `tools/`, `STATE.md`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, and the game folder. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `claude/p14-flight-2f` is checked out in this worktree and its working tree is clean when you start. Edit files in the working tree only. No git command that writes. `git status`, `git diff` and `git log` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not weaken a test to make it pass; report the value and the reason.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run `cargo build --offline`, `cargo clippy --offline`, `cargo test --offline`, the ignored writer, the byte comparison of `handoff/p14-probes/` and the mission builder check, and read the counts from their output. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now: const tables with named fields that read like this prompt's tables, shared helpers, named constants, no flag arguments, no behaviour keyed on a bare number, failures that name the cell or missing part. No refactor, rename or reformat of code outside the change; where flight 2E's code is reused, extract the smallest helper that keeps every earlier flight's output byte-identical.
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
7. Where the post-placement formation is set, and why there.
8. Choices you made where this prompt gave no exact shape, and residual risks.
</structured_output_contract>
