# Delegation prompt for flight 2G (can a setting or an MCU stop the wingman crossing its lead?)

One job, in the worktree `distracted-taussig-b9259d` on `claude/p14-flight-2g-be3801` (cut from `claude/p14-flight-2f` at `76b432e`, plus this prompt and the design note, committed). Sent with `codex_start`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1200. Model: the config default. First job `20261017220409-fdfd434b` stopped before editing on a contradiction (H asked for S4 on placed copies, which inspect as S7); H is corrected below and the job re-sent.

---

<task>
Build probe flight 2G: a no-pilot probe on flight 2F's path that flies the Yak airfield raid five times in turn, each as shipped (Pairs at Safe) with one change made after placement, over empty targets, with at most two raids (8 aircraft) in the air at once. It measures whether AI skill, a Formation order at `WP 1`, or moving the attack area ahead of `WP 1` stops the wingman crossing its lead's track. Probe code only. The shell, the library, placement, the templates, `src/template.rs` and every flown file stay as they are.

Read first, in this order:
1. `handoff/sessions/2026-10-17-flight-2g.md` (the design; this prompt makes it exact).
2. `docs/decisions/0021-formation-collisions-are-a-game-bug.md`; `handoff/P14-probe-results.md`, section "Flight 2F".
3. `CLAUDE.md`, `.cursorrules`.
4. Code: `src/airtask/probe.rs`, flight 2F is the model: `Flight::TwoF`, `RUN_SHEET_2F`, `TWO_F_SORTIES`, `TWO_F_*` constants, `ProbeTarget`, `EmptyTargetVariant::BuiltInFormation`, `EmptyTargetSortie::predicted_transit_s`, `empty_target_source`, `set_placed_probe_formation`, `build_empty_target_probe`, `add_sortie_probe_source` (it returns the `ShellIds`), `observe_sortie_probe`, `close_sortie_probe`, `empty_target_walk_input`, `builtin_formation_schedule`, `append_probe_formations`, `write_empty_target_schedule`, `write_probe`, the `flight2f_…` tests, the shared run-sheet and cue tables (search for `Flight::TwoF, time_s`), and `AI_LEVEL_ACE`. Also `place.rs` (`place_air_sortie` puts every path waypoint and every ground AttackArea on the target, and sets density 1 on every Formation command), `shell.rs` (`wrap_sortie`, `ShellIds`), `library.rs` (`inspect_sortie`, shape S4), `testkit.rs` (the walker).
5. `TemplateExamples/Historical1950/1950_DPRK_Yak_AirfieldRaid_2plus2.Group` (S4; Planes Red 11 (6), Red 12 (8), Blue 21 (10), Blue 22 (12), all `AILevel = 2`; leads' entities 7 and 11; Formation 35 Objects [7], Formation 41 Objects [11], both `FormationType = 19`, `FormationDensity = 1`, `FlightSize = 1`, `WaitForWingmen = 1`; AttackArea 38 Objects [7] and 44 Objects [11], ground, 1,500 m, 300 s; `WP 1` (46) Objects [7, 11], Targets [37, 43], the two attack timers).

What to build, in `src/airtask/probe.rs`:

A. **`Flight::TwoG`**, stem `P14_Probe_2G`, not frozen, with a run sheet, cues, `generate_probe_2g()`, and an ignored writer `write_p14_probe_2g` that calls `write_probe(Flight::TwoG, TraceCarrier::default())` and writes the run sheet and schedule as flight 2F does (`target/p14/P14_Probe_2G_schedule.md`). Add it to `Flight::ALL` and every exhaustive match. Flights 0 to 2F must still produce exactly the output they do now.

B. **Five cells, one placed and wrapped Yak raid each**, all from `1950_DPRK_Yak_AirfieldRaid_2plus2.Group` (DPRK, S4, 4 aircraft, `attack_time_s` 300, measured transit 264.1 s: reuse `TWO_F_YAK_TRANSIT_S`), through `build_empty_target_probe` as flight 2F's cells are (own stand-in area, COLD; no HOT switch, no enemy, no spawn). Reuse `TWO_F_TARGET_A` and `TWO_F_TARGET_B`.

| Cell | Prefix | Change after placement | Target | START s |
|---|---|---|---|---|
| GC | SN01 | none (as shipped: the control) | B (140, 0) km | 60 |
| G3 | SN02 | `AILevel = 3` on every Plane of the cell | A (230, 0) km | 390 |
| G4 | SN03 | `AILevel = 4` on every Plane of the cell | B | 720 |
| GX | SN04 | per flight lead, one new `MCU_CMD_Formation`, Pairs at Loose, fired by `WP 1` | A | 1050 |
| GA | SN05 | every ground `MCU_CMD_AttackArea` moved 3,000 m ahead along the approach | B | 1380 |

20 aircraft in the mission, at most 8 alive at once.

- **One new `EmptyTargetVariant`** for "built-in as shipped, then one named change on the placed, wrapped copy": it names the file, shape, `attack_time_s`, the measured transit and the change, as an enum with one case per row (as shipped; AI level; Formation at `WP 1` with its `ProbeFormation`; AttackArea ahead with its distance in metres). Name the constants: the AI levels (`AI_LEVEL_ACE` exists; add Veteran beside it), the 3,000 m offset, Pairs at Loose as a `ProbeFormation` (19, 2, "Pairs", "Loose"). The source given to placement is the parsed file, untouched, in every row.
- **Every change is made after `add_sortie_probe_source` returns**, on the `"{cell} sortie"` group, as `set_placed_probe_formation` is for 2F. `add_sortie_probe_source` does not change. Each change fails with the cell name if what it changes is missing.
  - **AI level:** set `AILevel` on every `Plane` of the placed copy, nothing else. Fail if the copy has no Plane.
  - **Formation at `WP 1`:** for each flight lead of the inspected shape, take the placed copy's own `MCU_CMD_Formation` whose `Objects` is `[lead]`, clone it, give it a new index from `p.next_id`, `Name` "Formation Loose at WP 1", `FormationDensity` 2 (keep `FormationType`, `FlightSize`, `WaitForWingmen`, `Objects`), and put it in the same group as the command it copies. Append both new indexes to the `Targets` of the navigation lead's path waypoint (`shape.path_waypoints[&shape.navigation_lead]`, the shared `WP 1`), after its existing targets. Nothing else changes: the original Formation commands, the attack timers and the shell are untouched. Exactly two MCUs are added. No switch back to Safe (the design note says why). Fail if a lead has no Formation command or more than one.
  - **AttackArea ahead:** move every ground `MCU_CMD_AttackArea` of the placed copy (those `snap_ground_attack_areas` placed on the target) by +3,000 m in X, which is along the approach for heading 0 (check this against the placed lead Plane and `WP 1`: the spawn is at lower X than `WP 1`; fail if not). `WP 1`, its `Area`, the AttackArea radius and `Time`, and every other node stay as placed. Fail if the copy has no ground AttackArea or if an AttackArea is not on `WP 1`'s X/Z before the move.
- **Prediction**: `WP 1` at START + `TWO_F_YAK_TRANSIT_S` in every row (GA moves the area, not `WP 1`).
- **`empty_target_source`'s checks per row**: shape S4 and `attack_time_s` 300, with the cell named. If `inspect_sortie` refuses the file or gives another shape or time, stop and report the exact message; do not change `library.rs`.
- **Paths and reuse**, checked in tests with the placed copies' own lead Plane and `WP 1` positions: both targets over land, and GA's moved AttackArea centre over land; the A and B paths at least 45 km apart; each target's next cell starts after the previous cell's predicted `DELETE DELAY` (GC 685.2 before G4 720; G3 1015.2 before GX 1050; G4 1345.2 before GA 1380); at no predicted moment are more than two cells alive (START to `DELETE DELAY`).

C. **Timing.** The host leaves at **33:00** (1,980 s). Walker test (Restart, scripted `WP 1` arrivals at the predicted times): each copy's `ON STATION` starts once at its predicted time, and its `DONE HUB` and `DELETE DELAY` fire at the predicted times (GC 624.6 / 685.2, G3 954.6 / 1015.2, G4 1284.6 / 1345.2, GX 1614.6 / 1675.2, GA 1944.6 / 2005.2; follow the walker where it differs and report the difference); every attack has ended before 1,980 s. In GX, `WP 1` fires both new Formation commands. Do not lengthen the shipped 300 s attack. Safety net as 2F: close every cell at **2,400 s** with one `MCU_Delete` per aircraft entity, and a final cue.

D. **Cues** (subtitles, flight 2F's style):
- 0 s: "Flight 2G: spectate, click the sky, then LCtrl+R to record. Check the menu now and then for the server overload message. Leave the server at 33:00"
- at each START: "Flight 2G: GC starts (as shipped)", "Flight 2G: G3 starts (Veteran)", "Flight 2G: G4 starts (Ace)", "Flight 2G: GX starts (Loose at WP 1)", "Flight 2G: GA starts (attack area 3 km ahead)"
- 1,950 s: "Flight 2G: leave the server at 33:00"
- 2,400 s: "Flight 2G closed. Every aircraft has a Delete. End the mission normally"

E. **Breadcrumbs** as flight 2F (`observe_sortie_probe`), including both S4 leads' attack timers and the shared `WP 1`. Observing the two new GX Formation commands is not needed (they fire with `WP 1`).

F. **No spawn.** `Flight::TwoG.spawns()` is empty; the writer writes `[]` as 2F's does.

G. **Schedule** (`target/p14/P14_Probe_2G_schedule.md`): per cell the change (in words and values), prefix, target, START, predicted `WP 1` (say it is from 2E's measured transit), `ON STATION`, `DONE HUB`, `DELETE DELAY`, target, AttackArea centre and spawn (X, Z); the number of aircraft alive in each interval; the Formation commands as written (`append_probe_formations`, which must also list GX's two new ones and who targets them); each cell's Plane `AILevel` values.

H. **Tests** (`flight2g_…`), each stating its own literals:
- cells, keys, prefixes, changes, transit, starts and targets match the table; land (targets and GA's moved area), path separation and target reuse as in B;
- per cell: the source given to placement equals the parsed file; every plane `Country` is 503; side logic and icons are DPRK;
- **only the named change differs**: each 2G cell's sortie equals the same file placed and wrapped as shipped (same cell, prefix and ids, through the same path with no change) except: GC nothing; G3 and G4 only `AILevel` on the four Planes; GX only the two added Formation commands and the two added `WP 1` targets; GA only `XPos` of the two ground AttackAreas, each +3,000 m;
- GX: each new command has the copied command's `FormationType` 19, `FlightSize`, `WaitForWingmen` and `Objects`, `FormationDensity` 2; `WP 1`'s targets are its old targets followed by the two new indexes; nothing targets the new commands except `WP 1`;
- each change fails with the cell name on a copy that lacks what it changes (no Plane; a lead with no Formation command; no ground AttackArea);
- as the `flight2f_…` tests do: `inspect_sortie` gives S4 and 300 on the parsed source (placement adds RTB, so a placed copy inspects as S7; do not require S4 there); every AttackArea `Time` and `ON STATION` `Time` is 300 on the wrapped copy;
- the walker run in C, including both GX Formation commands firing at `WP 1`;
- the cues, with their times and texts, in the shared run-sheet and cue tables;
- the existing byte and frozen-file tests (`flown_probes_regenerate_byte_for_byte_except_flight2`, `flight2_frozen_files_parse_and_links_resolve`, `placement_frozen_files_parse_and_links_resolve`, the `flight2d_…`, `flight2e_…` and `flight2f_…` tests) still pass unchanged.

Acceptance:
- Before editing, record the baseline: `cargo build --offline` and `cargo clippy --offline` warning counts (expected 69 / 251), `cargo test --offline` passed / failed / ignored (expected 677 / 0 / 25). After: warnings not above the baseline; new code adds none.
- `cargo test --offline`: 0 failed; passed = baseline + the new tests; ignored = baseline + 1.
- `cargo test --offline write_p14_probe_2g -- --ignored` writes `target/p14/P14_Probe_2G.Group`, `_traced.Group`, their `.eng`, the `.trace.json` sidecar, `.spawns.json` and the schedule, and touches nothing under `handoff/`.
- Every flown probe file under `handoff/p14-probes/` (flights 0 to 2F) is unchanged and still regenerates byte for byte where it did before.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_2G_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --name P14_Probe_2G_check --title "P14 probe 2G check" --out target/p14/missions/P14_Probe_2G_check --no-resave` exits 0 (check the script's real arguments first and adapt only their spelling), every `Index` unique, every reference resolved. Never run it without `--no-resave`. Do not copy anything into the game folder.

Files in scope: `src/airtask/probe.rs` only.

Do not touch: every other file, in particular `src/airtask/shell.rs`, `library.rs`, `place.rs`, `testkit.rs`, `budget.rs`, `assemble.rs`, `mod.rs`, `src/template.rs`, `src/trace.rs`, `src/weapon_range.rs`, `src/testdata/`, `TemplateExamples/`, `handoff/` (all of it), `docs/`, `tools/`, `STATE.md`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, and the game folder. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `claude/p14-flight-2g-be3801` is checked out in this worktree and its working tree is clean when you start. Edit files in the working tree only. No git command that writes. `git status`, `git diff` and `git log` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
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
5. The run sheet: per cell the change, prefix, target, START, predicted `WP 1`, `ON STATION` start and end, predicted `DONE HUB` and `DELETE DELAY`, target (X, Z), AttackArea centre (X, Z), spawn (X, Z); the most aircraft alive at once.
6. GX's Formation nodes as written (index, name, `FormationType`, `FormationDensity`, `Objects`, who targets them) and `WP 1`'s targets; GA's AttackArea nodes before and after the move; G3's and G4's Plane `AILevel` values.
7. Where each change is made, and why there.
8. Choices you made where this prompt gave no exact shape, and residual risks.
</structured_output_contract>
