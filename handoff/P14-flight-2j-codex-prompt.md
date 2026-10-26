# Delegation prompt for flight 2J (does a switched-on Complex Trigger report a plane already inside?)

One job, in the worktree `distracted-taussig-b9259d` on `codex/p14-4` (step 4 committed at `0f68254`, plus this prompt, committed). Sent with `codex_start`, model `gpt-6-astra`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1500.

---

<task>
Build probe flight 2J: a no-pilot probe that answers whether an IL-2 `MCU_TR_ComplexTrigger` switched on (MCU_Activate) with a plane of its filtered country already inside reports Object Entered (k1), and again after each switch off and on (k3), and that flies the step 4 threshold cell as built. Probe code only. Product code, the threshold builder, the templates and every flown file stay as they are.

Read first, in this order:
1. `handoff/sessions/2026-10-18-p14-step4.md`, sections "The cell", "Fallback", "Built and reviewed" and "Probe flight 2J: design" (the design; this prompt makes it exact).
2. `docs/decisions/0023-step-4-threshold-one-complex-trigger-per-area.md`; `handoff/P14-probe-results.md`, sections "Flight 1N" and "Flight 2B"; `docs/il2-loader-and-log-facts.md` (all of it: loader-safe strings, the Complex Trigger, mission logic).
3. `CLAUDE.md`, `.cursorrules`.
4. Code: `src/airtask/probe.rs`. Flight 1N's `build_y` is the model for a Complex Trigger cell (reference block, `Country`, OnEvents through relays into `observe`, readouts, `ai_with_close_delete`, `set_flight_plane_country`, `ai_delete_at`). Flight 2B's `build_reflight_enemy` and `TWO_B_ORBIT_OFFSETS_KM` are the model for a pair that loiters on a closed waypoint loop (octagon, radius 2 km, 500 km/h). Flight 2I is the model for a recent flight's `Flight` variant, stem, run sheet, cues, generator, ignored writer, schedule and tests. `src/airtask/threshold.rs` (`build_side`, its test harness with stand-in consumers), `src/airtask/assemble.rs` (`emit_side_logic`), `src/airtask/testkit.rs` (walker; `TriggerActivation`, `ScriptedTrack.country`).

What to build, in `src/airtask/probe.rs`:

A. **`Flight::TwoJ`**, stem `P14_Probe_2J`, not frozen and not yet regenerating flown files, with a run sheet, cues, `generate_probe_2j()`, and an ignored writer `write_p14_probe_2j` that writes the files and `target/p14/P14_Probe_2J_schedule.md`. Add it to `Flight::ALL` and every exhaustive match. Every earlier flight's output stays exactly as it is now (the flown-file test still compares 52 files, byte-identical).

B. **Every aircraft is on the NATO side** (countries 601 and 602), so nothing fights. Loitering pairs fly flight 2B's octagon loop (radius 2 km), **centred on their cell's centre** (not 20 km east), at 500 km/h and a height you choose and state (2B used 8000 m for its Yak; pick what keeps the pair on the loop), Plane `Engageable` as 2B. You may use 2B's Yak-9P pair with its country changed (flight 1G and 1N showed a changed `Country` takes effect and counts for its coalition) or another built-in pair; state which and why. Cells are at least 40 km apart, over land, and each pair's whole route stays at least 15 km from every other cell's centre (tested).

C. **Six cells** (times in mission seconds; name constants for every number):

| Cell | Trigger or cell | Aircraft | Schedule | Expected (k1 holds) |
|---|---|---|---|---|
| KA | Complex Trigger `KA-CT`: reference shape as 1N (no `ObjectScript` / `ObjectName`), `Country = 601`, `Radius = 6000`, `Cylinder = 1`; OnEvent 58 and 60 each into a relay that is observed, with "entered 2+" style readouts as 1N | a 601 pair activated at 30 s on the loop | switched off by a probe-owned Deactivate at 5 s; on at 180 s, off at 200 s, on at 240 s, off at 260 s, on at 300 s, off at 320 s; each switch observed | Entered for each aircraft within a few seconds of each switch-on (three times); nothing while off |
| KA control | plain `MCU_CheckZone` `KA-Z`, Closer 1, 6000 m, `[2]`, same centre | — | pulsed at 190 s | fires (proves the pair is inside) |
| KB | `KB-CT`, as KA | a 601 pair flying straight through the centre (`ai_with_close_delete`, start 30 km before the centre, end 30 km past it), activated so it is inside 6 km from about 530 s to 595 s (compute and state) | off from 5 s; on at 700 s (after the pair has left); off at 720 s; control zone `KB-Z` pulsed while the pair is predicted inside | trigger silent throughout; control fires |
| KC | `KC-CT`, as KA (`Country = 601`) | a **602** pair activated at 30 s on the loop | off from 5 s; on at 180 s, off at 200 s; control zone `KC-Z` pulsed at 190 s | trigger silent (country filter holds on switch-on); control fires |
| KD | the step 4 cell `THD01`, built by `threshold::build_side(AirSide::Dprk, …)` with `ThresholdSpec::default()` and `PresenceCheckImpl::TriggerReactivate`, untouched after building | a 601 pair activated at 60 s, 20 km from the centre, flying in to the loop | pair deleted at 600 s | ARM on arrival in r_in; PASS about 60 s later; re-check every 120 s while it loiters; FAIL (COLD ON, REARM ON) at the first re-check after the delete |
| KE | `THD02`, same build call as KD (areas `[KD, KE]`) | a 602 pair, as KD | deleted at 600 s | never ARM, never HOT |
| KF | the `ZonePulse` cell, built by its own `build_side` call with its own `SideLogic`; rename its MCU names `THD01 …` to `THD03 …` after building so every name in the probe is unique (nothing else changes) | a 601 pair, as KD | deleted at 600 s | PASS about 60.1 s after arrival; re-checks; FAIL after the delete |

For KD to KF: one stand-in HOT relay and one COLD relay per cell, registered in `hot_consumers` / `cold_consumers` under the cell's area number before `build_side`, plus one stand-in pass subscriber; the HOT relay is in `init_off` as `ShellIds::register` does. A **consumer poll**: from 60 s to 780 s every 20 s, a probe timer pulses each cell's HOT and COLD stand-ins; each stand-in's output is observed ("THD01 poll HOT", "THD01 poll COLD", …), so the log shows the area state over time and that exactly one of the pair passes. Observe (breadcrumb) each cell's `TRIGGER` OnEvent targets (`ARM`, and `SEEN` for TriggerReactivate), `INNER` for ZonePulse, `CHECK ON`, `CLOSE DELAY`, `HOT ON`, `COLD ON`, `REARM ON` and the subscriber. Mission Begin, `AT D INIT` and `AT D INIT OFF` come from `emit_side_logic` (one call per `SideLogic`; keep names unique, renaming the KF set if needed, and report it). Add the cells' observations through the probe's own observe mechanism, so `threshold.rs` is not edited.

D. **Timing and close.** The host leaves at **13:30** (810 s). Safety close at **900 s**: one `MCU_Delete` per aircraft entity and a final cue. Walker test with scripted tracks (a loop track per loitering pair from its predicted arrival, KB's straight track at 500 km/h or the speed you use, each pair's track moved outside at its delete time; `ScriptedTrack.country` set per pair), `DeactivateRunningTimer::Cancels`, Restart: under `TriggerActivation::ReportsInside` assert the expected column for every cell with predicted times (KA Entered after each switch-on; KD ARM, PASS, each re-check, FAIL after the delete; KE nothing; KF PASS and FAIL; poll outputs HOT or COLD as predicted, never both and never neither); under `TriggerActivation::Silent` assert KA, KD silent after their first switch-on and KF unchanged (the fallback does not depend on k1). Where the walker differs from this prompt's times, follow the walker and report it.

E. **Cues** (subtitles; loader-safe text):
- 0 s: "Flight 2J: spectate. No recording needed. Leave the server at 13:30"
- 180 s: "Flight 2J: KA and KC triggers switched on"
- 600 s: "Flight 2J: KD KE KF pairs deleted. Expect KD and KF to go COLD by 13:00"
- 780 s: "Flight 2J: leave the server at 13:30"
- 900 s: "Flight 2J closed. Every aircraft has a Delete. End the mission normally"

F. **No spawn** (`spawns()` empty). Map icons per cell as recent flights do, NATO side.

G. **Schedule** (`target/p14/P14_Probe_2J_schedule.md`): per cell the centre (km and X, Z), the aircraft and country, activation, predicted arrival in 6 km, each switch, predicted outputs (ReportsInside and Silent columns), deletes; aircraft alive per interval; the walker pulse schedule as 2I's.

H. **Tests** (`flight2j_…`), each stating its own literals: the table (cells, countries, radii, switch times); the KA, KB, KC triggers equal the 1N shape apart from name, position, `Radius` and `Country`, with only OnEvent outputs; the KD/KE cells are byte-equal to a plain `threshold::build_side` call with the same inputs (same for KF before its rename); unique names and indexes; separation and land; at most 12 aircraft alive; the walker runs of D; the cues in the shared tables; every earlier test unchanged and passing.

Acceptance:
- Baseline first: `cargo build --offline` / `cargo clippy --offline` warnings (expected 68 / 250), `cargo test --offline` (expected 739 passed / 0 failed / 28 ignored). After: no new warnings; 0 failed; passed = baseline + new tests; ignored = baseline + 1.
- `cargo test --offline write_p14_probe_2j -- --ignored` writes the Group, traced Group, `.eng`, `.trace.json`, `.spawns.json` and schedule, and touches nothing under `handoff/`.
- Every file under `handoff/p14-probes/` unchanged; `flown_probes_regenerate_byte_for_byte_except_flight2` passes.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_2J_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --name P14_Probe_2J_check --title "P14 probe 2J check" --out target/p14/missions/P14_Probe_2J_check --no-resave` exits 0 (use the installed Python 3 if `python` is not on PATH), every `Index` unique, every reference resolved. Never without `--no-resave`. Nothing copied into the game folder.

Files in scope: `src/airtask/probe.rs` only. Do not touch anything else (in particular `threshold.rs`, `testkit.rs`, `shell.rs`, the templates, `src/testdata/`, `handoff/`). If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `codex/p14-4` is checked out in this worktree and its working tree is clean when you start. Edit files in the working tree only. No git command that writes. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not weaken a test to make it pass.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run build, clippy, test, the ignored writer, the byte comparison and the mission builder check, and read the counts. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now: const tables with named fields, shared helpers, named constants, no flag arguments, failures that name the cell. No refactor outside the change; extract the smallest helper that keeps every earlier flight byte-identical.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Where this prompt and the code disagree, follow the code and report each difference.
</missing_context_gating>

<structured_output_contract>
Return: 1. Summary (five lines). 2. Command results (baseline and final warnings, tests, byte comparison, mission builder). 3. Files changed. 4. Tests added or changed. 5. The run sheet per cell (centre, aircraft and country, activation, predicted arrival, switches, predicted outputs under ReportsInside and Silent, deletes), most aircraft alive. 6. Choices (aircraft type, height, loop, renames) and residual risks.
</structured_output_contract>
