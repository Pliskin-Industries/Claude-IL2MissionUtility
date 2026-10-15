# Delegation prompt for step 3c (decision 0018: Zone Out only after an enemy is seen; flight 2B)

One job, in the worktree `Claude-IL2MissionUtility-3b` on `codex/p14-3b` (step 3b built, reviewed and flown as flight 2). Sent with `codex_start`, `model` = `gpt-6-astra`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1500.

---

<task>
Change the sortie shell to decision 0018 and build the re-fly, flight 2B.

Read first, in this order:
1. `docs/decisions/0018-zone-out-only-after-an-enemy-is-seen.md` (what to build), then 0017.
2. `handoff/P14-probe-results.md`, sections "Flight 2" and the flight 0 table's `z` row.
3. `handoff/P14-air-tasking.md` §3.2 (the shell; "Kept and rewired" is what changes) and §8 step 3.
4. `CLAUDE.md`, `.cursorrules`, `docs/il2-loader-and-log-facts.md`.
5. Code: `src/airtask/shell.rs`, `library.rs` (`SortieShape.zone_in` and how Zone IN, its arming and `Self Deactivate` are found), `probe.rs` (flight 2's `Flight::Two`, run sheet, cells, writers and checks are the model for flight 2B), `testkit.rs` (the walker; how check zones are sampled), `budget.rs`, `assemble.rs`, `src/trace.rs`; the template blocks in `TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group` (Zone IN 17, Zone Out 18, `PULSE OUT` 16, `Zone Out ReActivate` 20); the shipped `MCU_CMD_Behaviour` block in `TemplateExamples/DPRK_Scramble.Group`.

What to build:

A. **The shell (`shell.rs`).** Stop stripping the template's Zone IN. In the copy: re-centre Zone IN on the target, set its `Zone` to Zone Out's `Zone`, keep its `Closer = 1` and its `PlaneCoalitions`; set its Targets to exactly Zone Out's arming pair (the `MCU_Activate` and the 0.1 s timer that target Zone Out). The navigation lead's `WP 1` pulses Zone IN and no longer targets the arming pair. Zone IN must be active when WP 1 pulses it: it is not in `SideLogic.init_off`, and nothing in the copy deactivates it (the template's `Self Deactivate` for Zone IN, `ENABLE / PULSE IN`, `Zone In ReActivate` and `COOLDOWN` stay stripped). Nothing else changes. Name it in the copy as the shell names its own MCUs (`SNjj ZONE IN` or keep `Zone IN`; say which and why).

B. **Tests.** Update the shell's structural tests for the new wiring and counts: 20 MCUs per wrapped HVAR copy (was 19), 19 with a template RTB timer (was 18); every wrapped copy of the six built-ins holds exactly two `MCU_CheckZone`, Zone IN and Zone Out, both at the target with the same radius. Add walker scenarios, each also run with `ReverseTargets` and the three re-trigger modes:
- `shell_empty_target_stays_on_station`: no enemy ever inside: `DONE HUB` comes from `COLD` at the end of on-station time (HVAR: WP 1 + 0.5 s + 600 s), not at WP 1;
- `shell_enemy_present_at_arrival_then_leaves`: an enemy inside from before WP 1, gone at WP 1 + 180 s: `DONE HUB` from Zone Out when it is gone;
- `shell_enemy_arrives_after_then_leaves`: an enemy inside from WP 1 + 120 s to WP 1 + 300 s: `DONE HUB` from Zone Out at WP 1 + 300 s (plus the zone's reaction);
- `shell_hot_empty_target_extends_once`: empty target, area HOT: `DONE HUB` from `EXTEND` at the end of the extension.
If the walker cannot express an enemy inside a zone for a time window, stop and report what it lacks; do not change `testkit.rs` (it is out of scope).

C. **Flight 2B (`probe.rs`).** `Flight::TwoB`, stem `P14_Probe_2B`, a run sheet, cues, `generate_probe_2b()`, an ignored writer `write_p14_probe_2b`, following flight 2's pattern. Cells (each sortie placed by `place_air_sortie` and wrapped by the shell, NATO, with its own stand-in area; targets at least 80 km apart from each other, over land, away from K14):
- `T-s1` HVAR, started at 60 s; its area switched HOT at 600 s. No enemy. Expected: on station 600 s, one extension of 600 s, `DONE HUB` from `EXTEND`.
- `T-s2` AirAlert, started at 90 s; area stays COLD. No enemy. Expected: on station 1080 s (both AttackAreas), `DONE HUB` from `COLD`.
- `T-s3` HVAR, started at 120 s; area COLD. A DPRK AI flight (two aircraft) is activated at 30 s, circling inside its target's 35 km zone, and is deleted (one `MCU_Delete` per aircraft entity) at the sortie's predicted WP 1 arrival + 180 s. Expected: Zone IN fires at WP 1, `DONE HUB` from Zone Out when the enemy is gone.
- `T-s4` HVAR, started at 150 s; area COLD. A DPRK AI flight (two aircraft) is activated inside its target's zone at predicted WP 1 arrival + 120 s and deleted at + 300 s. Expected: Zone IN fires when it appears, `DONE HUB` from Zone Out when it is gone.
Predict WP 1 arrival from flight 2's measured transits (HVAR 286 s, AirAlert 279 s from `START`) and say so in the run sheet.
**Enemy flights:** a DPRK aircraft type from the built-in templates, country 503, `AiRTBDecision = 0`, air start at 8,000 m, circling 15 to 20 km from the sortie's attack point and inside 30 km of the target, so they trigger Zone IN but stay well away from the attacking aircraft. Make them non-targetable with an `MCU_CMD_Behaviour` copied from the shipped block in `DPRK_Scramble.Group`, `Engageable = 0`, issued right after activation. Find out what that block's `Filter` value selects (from the repository's code, docs or reference files); if nothing in the repository says, keep the shipped value, set `Engageable = 0`, and report it as unverified. Neither cell may rely on a fight.
**Breadcrumbs:** a trace source on every input that can end a sortie (Zone IN, Zone Out, `LOSSES`, `COLD`, `EXTEND`, `HARD STOP`), and on `START`, `WP 1`, each attack timer, `ON STATION`, `DONE HUB` and `DELETE DELAY`, plus the enemy flights' activation and deletion, so the replay shows which input ended each sortie.
Close every cell at 1,800 s with one `MCU_Delete` per aircraft entity still in the file, and a final cue.

D. **Flight 2 is flown and its files are frozen** (`handoff/p14-probes/flight2/`, the replay input for `src/testdata/missionlog/real_19/`). The shell change makes flight 2's builder produce different bytes. Do not keep the old shell for it. Keep the flown files unchanged, stop regenerating flight 2 (its writer must not overwrite anything under `handoff/`), and make any check that compares regenerated probes with `handoff/p14-probes/` leave flight 2 out with a comment citing decision 0018. Keep a test that the frozen flight 2 files still parse and their links resolve. Report how you did it. Flights 0 to 1P must still regenerate byte for byte.

E. **The load file** (`Flight::Load`) is rebuilt by its writer with the new shell. Report its MCU and check-zone counts per side.

F. **Documents:** `handoff/P14-air-tasking.md` §3.2 ("Kept and rewired", the diagram, "MCUs the shell adds per copy", the notes) and the step 3 test list, to match decision 0018; `docs/src-guide.md` for `shell.rs` if it describes the Zone Out wiring.

Acceptance:
- `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251. New code adds no warnings.
- `cargo test --offline`: 0 failed; passed at least 629 plus the new tests, less only tests this prompt replaces (name them).
- `cargo test --offline write_p14_ -- --ignored` writes flight 2B and the load file and touches nothing under `handoff/`.
- Flights 0 to 1P: their 44 files under `handoff/p14-probes/` regenerate byte for byte. Flight 2's four files are unchanged.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_2B_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10 --name P14_Probe_2B_check --title "P14 probe 2B check" --out target/p14/missions/P14_Probe_2B_check --no-resave` exits 0, every `Index` unique, every reference resolved. Throwaway check script, not added to the repository. Never run the builder without `--no-resave`.
- The step 1 baselines (`src/testdata/p14_baseline/`) do not change.

Files in scope: `src/airtask/shell.rs`, `src/airtask/probe.rs`, `src/airtask/library.rs` (only if finding Zone IN's arming parts needs it), `handoff/P14-air-tasking.md` and `docs/src-guide.md` (the sections named in F).

Do not touch: every other file, in particular `src/airtask/testkit.rs`, `mod.rs`, `budget.rs`, `assemble.rs`, `place.rs`, `src/template.rs`, `src/trace.rs`, `src/testdata/`, `TemplateExamples/`, `handoff/p14-probes/`, `docs/decisions/`, every other file under `handoff/` and `docs/`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `codex/p14-3b` is checked out in this worktree and clean. Edit files in the working tree only. No git command that writes. `git status`, `git diff` and `git log` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not change a test to match a count that disagrees with this prompt or decision 0018; report the count and the reason.
</completeness_contract>

<verification_loop>
Make the probe byte baseline before you edit `probe.rs`. Then run `cargo build --offline`, `cargo clippy --offline`, `cargo test --offline`, `cargo test --offline write_p14_ -- --ignored`, the byte comparison and the mission builder check, and read the counts from their output. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now: const tables with named fields that read like the plan's tables, shared helpers, named constants, no flag arguments, no behaviour keyed on a bare number, failures that name the template, copy or missing part. Tests state their own literals. No refactor, rename or reformat of code outside the change.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Where the plan and the code disagree, follow the code for facts and decision 0018 then the plan for semantics, and list each difference.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary, five lines at most.
2. Command results: build and clippy warnings, tests passed / failed / ignored, the byte comparisons, the mission builder check.
3. Every file changed, one line each on why.
4. Tests removed, renamed, changed and added, by name.
5. The wrapped HVAR copy: MCUs added and removed (names and counts), and its two check zones (position, radius, `Closer`, `PlaneCoalitions`, targets).
6. Flight 2B: the run sheet, every target and enemy position with distances, a schedule of every command and pulse in time order, the `MCU_CMD_Behaviour` block as written and what its `Filter` selects (or that it is unverified).
7. How flight 2 was frozen (D).
8. The load file's MCU and check-zone counts per side.
9. Choices you made where this prompt gave no exact shape, differences between plan and code, residual risks.
</structured_output_contract>
