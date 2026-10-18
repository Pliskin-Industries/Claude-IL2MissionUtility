# Delegation prompt for flight 2C (why HVAR sorties lose aircraft over an empty target)

One job, in the worktree `adoring-mahavira-fad55e` on `codex/p14-2c` (cut from `claude/p14-hvar-empty-target`, which is `codex/p14-3b` plus the options note). Sent with `codex_start`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1200. Model: the config default.

---

<task>
Build probe flight 2C: a no-pilot probe that tells apart the causes of the HVAR losses seen in flight 2B. Probe code only. The shell, the library, the templates and every flown file stay as they are.

Read first, in this order:
1. `handoff/sessions/2026-10-15-hvar-empty-target.md` (why this flight exists; its section 5 is the design this prompt makes exact).
2. `handoff/P14-probe-results.md`, sections "Flight 2B" and its runs 2 and 3.
3. `CLAUDE.md`, `.cursorrules`, `docs/il2-loader-and-log-facts.md`.
4. Code: `src/airtask/probe.rs` (flight 2B is the model: `Flight::TwoB`, `RUN_SHEET_2B`, `TWO_B_SORTIES`, `add_sortie_probe`, `build_sortie_reflight`, `write_reflight_schedule`, `write_probe`, `spawn_json`, the flight 2B tests), `library.rs` (`inspect_sortie`, the S1 and S2 shapes, how attack timers and `attack_time_s` are found), `shell.rs`, `place.rs` (`place_air_sortie`, how air and ground AttackAreas are put on the target), `testkit.rs` (the walker).
5. The two templates: `TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group` and `1950_US_F80_AirAlert_4ship.Group`.

What to build, in `src/airtask/probe.rs`:

A. **`Flight::TwoC`**, stem `P14_Probe_2C`, with a run sheet, cues, `generate_probe_2c()`, and an ignored writer `write_p14_probe_2c` that calls `write_probe(Flight::TwoC, TraceCarrier::default())` and writes a run sheet and schedule as flight 2B does (`target/p14/P14_Probe_2C_schedule.md`). Share flight 2B's code where the shape is the same; do not change what flight 2B generates.

B. **Twelve cells, one placed and wrapped NATO sortie each** (built as `add_sortie_probe` does, each with its own stand-in area, which stays COLD; no HOT switch, no enemy aircraft anywhere in the mission). Four variants, three copies each:

| Variant | Source | Change to the parsed source before placement |
|---|---|---|
| C0 | HVAR | none (as shipped) |
| C1 | HVAR | `PayloadId = 2` on all four Planes (guns and tanks, no rockets; AirAlert's payload on the same model). Nothing else. |
| C2 | HVAR | fix A′: the ground AttackArea (38) gets `Time = 120`. Add one `MCU_Timer` (`Time = 120`, otherwise as timer 37) and one `MCU_CMD_AttackArea` (`AttackAir = 1`, `AttackGround = 0`, `AttackGTargets = 0`, `AttackArea = 5000`, `Time = 600`, `Priority = 1`, Objects = the lead's entity 7). Wiring: timer 37 → [AttackArea 38, the new timer] (it now targets the new timer instead of `Mission Complete 1`); the new timer → [the new AttackArea, `Mission Complete 1`]; `WP 1` → [timer 37, the new timer]. This is the S2 shape AirAlert has, with a 120 s second timer. |
| C3 | AirAlert | none (as shipped) |

For C2, `inspect_sortie` must accept the changed source as S2, with the new timer as the navigation lead's attack timer and `attack_time_s = 600`. So `ON STATION` starts from the new timer (about WP 1 + 120 s) and runs 600 s. If `inspect_sortie` refuses it, stop and report the exact message; do not change `library.rs`.

Cell keys `C0a`, `C0b`, `C0c`, `C1a` … `C3c`; shell prefixes `SN01` to `SN12` in that order. Starts 5 s apart from 60 s in that order (`C0a` 60 s … `C3c` 115 s).

**Targets** on a 3 × 4 grid in `world()` km from K14: rows x = 140, 185, 230; columns z = −90, −45, 0, 45. Assign so each variant has one target in each row and no two in one column:
- row 140: C0a, C1a, C2a, C3a (z = −90, −45, 0, 45)
- row 185: C1b, C2b, C3b, C0b
- row 230: C2c, C3c, C0c, C1c
Every target must be over land (check it the way `RUN_SHEET_2`'s T-l comment says it was checked) and at least 40 km from every other target. If a grid point fails, move it by the smallest whole 5 km step that passes, keep the assignment, and report it. Report the ground height at each target if the repository can compute it (decision 0003), else say it cannot.

C. **Timing.** Predict WP 1 from flight 2's measured transits (HVAR 286 s, AirAlert 279 s from `START`, the constants flight 2B uses). The host leaves the server at **15:00** so that the end-of-mission list (written when the mission ends) still holds every live aircraft with its height and rocket count. Check in a test that at 900 s every copy has been on station at least 300 s after its predicted WP 1 and no copy's `DONE HUB` has fired (the first `COLD` comes at about 946 s). As a safety net, close every cell at 1,800 s with one `MCU_Delete` per aircraft entity still in the file, and a final cue, as flight 2B does.

D. **Cues** (subtitles, flight 2B's style): at 0 s "Flight 2C: spectate if you can. Camera on C0a, then LCtrl+R to record. Leave the server at 15:00"; at 870 s "Flight 2C: leave the server now (15:00)"; at 1,800 s the close cue.

E. **Breadcrumbs** as flight 2B: on each copy's `START`, `WP 1`, every timer that issues an AttackArea (C2 and C3 have two), `ON STATION`, `COLD`, `EXTEND`, `HARD STOP`, `LOSSES`, Zone IN, Zone Out, `DONE HUB` and `DELETE DELAY`.

F. **One NATO air-start spawn** in `Flight::spawns` (`spawn_json`), used only if a spectating host cannot record Tacview: name "2C NATO", country USA, F-80C (`f80c10`), 3,050 m, at km (130.0, −90.0), heading toward C0a's target. A NATO player must not trip any copy's Zone IN (they watch the other coalition); confirm that from the files and say so.

G. **Tests** (`flight2c_…`), each stating its own literals:
- the twelve cells, keys, prefixes, starts, targets (distances ≥ 40 km, over land) and variants match the table;
- per variant: payloads of all four Planes; every AttackArea's `AttackAir` / `AttackGround` / `AttackGTargets` / `AttackArea` / `Time`; C2's wiring exactly as in B; `inspect_sortie` gives S2 for C2 and C3, S1 for C0 and C1; `ON STATION` `Time` is 600 for C0, C1 and C2 and 1080 for C3;
- a walker run (Restart, both target orders) with scripted WP 1 arrivals: each copy's `ON STATION` starts once at the predicted time (C2 about WP 1 + 120.5 s; say which re-trigger mode gives which), and the 900 s condition in C holds;
- the spawn row;
- flight 2B still generates the same bytes as its frozen files in `handoff/p14-probes/flight2b/` (if an existing test already covers this, name it instead).

Acceptance:
- Before editing, record the baseline: `cargo build --offline` and `cargo clippy --offline` warning counts, `cargo test --offline` passed / failed / ignored. After: build and clippy warnings not above the baseline; new code adds none.
- `cargo test --offline`: 0 failed; passed = baseline + the new tests.
- `cargo test --offline write_p14_probe_2c -- --ignored` writes `target/p14/P14_Probe_2C.Group`, `_traced.Group`, their `.eng`, `.spawns.json`, the `.trace.json` sidecar and the schedule, and touches nothing under `handoff/`.
- Every flown probe file under `handoff/p14-probes/` (flights 0 to 2B) is unchanged and still regenerates byte for byte where it did before.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_2C_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawns target/p14/P14_Probe_2C_traced.spawns.json --spawn-planes f80c10 --name P14_Probe_2C_check --title "P14 probe 2C check" --out target/p14/missions/P14_Probe_2C_check --no-resave` exits 0 (check the script's real arguments first and adapt only their spelling), every `Index` unique, every reference resolved. Never run it without `--no-resave`. Do not copy anything into the game folder.

Files in scope: `src/airtask/probe.rs` only.

Do not touch: every other file, in particular `src/airtask/shell.rs`, `library.rs`, `place.rs`, `testkit.rs`, `budget.rs`, `assemble.rs`, `mod.rs`, `src/template.rs`, `src/trace.rs`, `src/testdata/`, `TemplateExamples/`, `handoff/` (all of it), `docs/`, `tools/`, `STATE.md`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, and the game folder. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `codex/p14-2c` is checked out in this worktree and clean. Edit files in the working tree only. No git command that writes. `git status`, `git diff` and `git log` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not weaken a test to make it pass; report the value and the reason.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run `cargo build --offline`, `cargo clippy --offline`, `cargo test --offline`, the ignored writer, the byte comparison of `handoff/p14-probes/` and the mission builder check, and read the counts from their output. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now: const tables with named fields that read like this prompt's tables, shared helpers, named constants, no flag arguments, no behaviour keyed on a bare number, failures that name the cell, variant or missing part. No refactor, rename or reformat of code outside the change; where flight 2B's code is reused, extract the smallest helper that keeps flight 2B's output byte-identical.
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
5. The run sheet: per cell the variant, prefix, START, predicted WP 1, `ON STATION` start and end, target (X, Z) with ground height, and the nearest other target's distance; any grid point moved.
6. The C2 copy as written: its two attack timers and two AttackAreas (index, Name, Time, flags, Targets, Objects) and `WP 1`'s Targets.
7. The spawn row, and why a NATO player cannot trip Zone IN.
8. Choices you made where this prompt gave no exact shape, and residual risks.
</structured_output_contract>
