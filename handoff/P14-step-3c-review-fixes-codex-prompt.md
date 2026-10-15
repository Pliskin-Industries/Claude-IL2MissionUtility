# Delegation prompt: step 3c review fixes

Third job in `Claude-IL2MissionUtility-3b` on `codex/p14-3b`. The step 3c work is uncommitted in this tree; the orchestrator re-ran its checks (638 passed, 0 failed, 21 ignored; 69 / 251 warnings; flights 0–1P byte-identical; flight 2 frozen files unchanged; both missions build with no dangling reference) and ran a standards review and a spec review. These are the confirmed findings. `handoff/P14-step-3c-codex-prompt.md` and `handoff/P14-step-3c-count-correction.md` still apply in full. Writer command: `cargo test --offline write_p14_ -- --ignored --skip write_p14_baseline` (the baseline writers must not run).

<task>
Fix each finding. All are in `src/airtask/probe.rs` unless named otherwise.

1. **Tick rate.** `write_reflight_schedule` divides by a bare `20.0`; use `testkit::TICKS_PER_SECOND` as the rest of the probe code does.
2. **Report text restates constants.** The schedule/report text spells out "8000 m", "country 503", "0.1 s", "0.5 s", "HVAR 286 s, AirAlert 279 s" and "15.5 to 19.5 km". Format each from the constant that owns it (`TWO_B_ENEMY_ALTITUDE_M`, `COUNTRY_DPRK`, `TWO_B_BEHAVIOUR_DELAY_S`, `TWO_B_ORDERS_DELAY_S`, the `TWO_B_*_TRANSIT_S`, `TWO_B_ORBIT_OFFSETS_KM`).
3. **Failures name the wrong flight.** `add_sortie_probe` serves flights 2 and 2B but panics with "flight 2 …" and `expect("flight 2 built-in sortie must parse")`. Name the flight it is building.
4. **Bare numbers in non-test code:** `rotate_tree(&mut flight, pivot, 45.0)`, `set_property("Priority", "2")`, `("StartType", "0")`, `&shape.planes[..2]`, and in `reflight_walk_input` `40_000.0`, `17_500.0`, `coalition: 1`. Name each (constant or table field).
5. **Failures that do not name the missing part:** `block_ids(&source, "MCU_CMD_Formation")[0]`, `entities[0]`, `pos_xz().unwrap()`, `by_id(...).name().unwrap()`. Each failure names the flight, cell and missing part.
6. **Duplicated values.** `TWO_B_SORTIES`' `EnemyWindow` start times (120.0, 150.0) restate `RUN_SHEET_2B`'s `arm_s`; the cue times 556, 586, 736 restate sums as literals; the schedule path is a literal where `Flight::stem()` exists. Derive each from its source.
7. **Repeated switches on flight status.** `assert_ne!(flight, Flight::Two)`, `if flight == Flight::TwoB` / `Flight::Load` inside the shared writer, and `matches!(flight, Two | TwoB | Load)` in the byte test: put the facts on `Flight` as table-style methods beside `stem()` / `sheet()` (for example whether a flight is frozen, whether it writes a schedule), and use them.
8. **`reference_block` special case.** `if block == "MCU_CMD_Behaviour"` keys behaviour on a string in a shared helper; put the expected count in the existing match/table instead.
9. **Enemy orbit margin (spec review).** The orbit's inner corner is 15.5 km from the attack point, 0.5 km inside the margin, and an AI overshoot on a 90° corner could bring the Yaks to about 14 km. Move the orbit so every waypoint is at least 17 km from the attack point and still inside 30 km of the target, and keep the turns gentler than 90° if the waypoint count allows. Update the test that checks the isolation.
10. **`shell.rs`, judgement call accepted:** the XPos/ZPos re-centring of Zone IN and Zone Out repeats; do it once for both zones.
11. Remove the empty `{ … }` scope in `flight2_frozen_files_parse_and_links_resolve`.

Not to change (reviewed and accepted): `Engageable = 0` on the enemy `Plane` blocks as well as the Behaviour; the load summary line in `write_probe`; the byte-check test writing under `target/`; `runtime(&f, hot)` (existing helper).

Files in scope: `src/airtask/probe.rs`, `src/airtask/shell.rs`, and `handoff/P14-air-tasking.md` only if a value it states changes (the orbit). Do not touch `docs/decisions/`, the two prompt files, the correction note, or anything else.

Acceptance: build at most 69 warnings, clippy at most 251, `cargo test --offline` 0 failed with no fewer than 638 passed, the writer command above writes flight 2B and the load file, flights 0–1P byte-identical, flight 2 frozen files unchanged, step 1 baselines unchanged, and the mission builder check (`--no-resave`) on `P14_Probe_2B_traced.Group` exits 0 with every Index unique and every reference resolved.
</task>

<git_protocol>
Same-host mode. Edit files in the working tree only. No git command that writes. The uncommitted changes you find are the step 3c work plus the orchestrator's edits to `docs/decisions/0018-…`, `handoff/P14-step-3c-count-correction.md` and this file; they are expected.
</git_protocol>

<verification_loop>
Run build, clippy, the full suite, the writer command, the byte comparison and the mission builder check; fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<structured_output_contract>
Return: (1) summary, five lines at most; (2) command results and byte comparisons; (3) for each finding 1–11 what changed, or why not; (4) the new enemy orbit: each waypoint's distance to its attack point and target, and the turn angles; (5) tests added, removed or renamed; (6) residual risks.
</structured_output_contract>
