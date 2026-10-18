# Delegation prompt for step 3b (settle step 3 to decision 0017)

One job, in the worktree `Claude-IL2MissionUtility-3b` on branch `codex/p14-3b`. The orchestrator has cut that branch from `claude/p14-air-tasking` and merged the held `codex/p14-3` into it (`--no-ff`); the merged tree does not compile yet (13 errors, listed below). Sent with `codex_start`, `model` = `gpt-6-astra`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1500.

---

<task>
Settle P14 step 3 (sortie shell, budget and assembly) on this branch, to decision 0017. Step 3 was built before flight 0; since then decisions 0011 to 0016 changed what it must do, and the code it depends on moved.

Read first, in this order:
1. `docs/decisions/0017-step-3-activate-mode-own-delete-event-13.md` (what to build; it overrides the plan where they differ), then 0011, 0013, 0014, 0015 in `docs/decisions/`.
2. `handoff/P14-air-tasking.md`: §8 "General rules for every step"; §8 "Step 3" (the whole section); §3.0, §3.2, §3.3, §3.4; §4.3; §5; §9 risks g, u, v.
3. `handoff/P14-probe-results.md`: "Strategy defaults set by flight 0 (0d table)", and the sections for flights 1G, 1J, 1N (cell with V2) and 1P.
4. `docs/il2-loader-and-log-facts.md`, `CLAUDE.md`, `.cursorrules`.
5. Code: `src/airtask/shell.rs`, `budget.rs`, `assemble.rs` (the held step 3), `mod.rs`, `library.rs` (`inspect_sortie`, `SortieShape`: `delete_timer` and `delete` are now `Option`), `place.rs`, `testkit.rs` (the walker; a kill now pulses only `OnKilled` (13) hooks), `probe.rs` (all of it: flight 1P's `Flight`, run sheet, writer and close pattern are the model for flight 2), `src/trace.rs`, `src/template.rs` (`attach_event`, `EntityEvent`, the MCU builders, `generate_template`), and the six built-ins in `TemplateExamples/Historical1950/`.

The 13 compile errors on this branch now: nine in `shell.rs` where `shape.delete_timer` is used as `i32` (lines 189, 271, 386, 639, 645, 832, 923, 963, 1105); three uses of `LossesImpl::Type4` (327, 955, 1468); one use of the removed `locale::P14_PROBE_TRACE_LCNAME` (1210). Fix them by building what is below, not by wrapping them in `if let`.

What to build:

A. Activate-mode only (`library.rs`). `inspect_sortie` refuses a template whose bring-up is a counter into an `MCU_Spawner` with the error "Air Tasking needs an Activate-mode template", followed by the existing `REGENERATE_TEMPLATE` text as other refusals are. Remove the acceptance paths that only Spawn-mode templates reach (the Spawner lookup, the repeat-mode counter, cooldown and reset checks, `repeat_kill_events`) and report each removal. Step 2 tests that accept a Spawn-mode or repeat-mode template become refusal tests. New test `inspect_sortie_refuses_spawn_mode` (a single-plane Spawn-mode template written by `generate_template` in the test, Repeat off and on).

B. The shell's own clean-up (`shell.rs`). Per copy the shell writes `SNjj DELETE DELAY` (`MCU_Timer`, 0.5 s) and `SNjj DELETE` (`MCU_Delete`, Objects = every plane's entity in the copy, the escort's included). The template's `DELAYED END ORDERS` timer targets `SNjj DELETE DELAY` in place of the template's delete timer. `SNjj DELETE DELAY` targets `SNjj DELETE` and `SNjj LANDED`, and is registered in `SideLogic.budget_events` (so the budget builder links it to `BUD s MARK` and `BUD s CHECK`). If the template has a Delete chain, its delete timer and `MCU_Delete` are removed from the copy and no link to them is left. Copy the new blocks from existing blocks of the same type, as the codebase does.

C. Losses on event 13 (`shell.rs`, `mod.rs`). One `OnEvent` of type 13 (`EntityEvent::OnKilled`) per plane entity into `SNjj LOSSES` (`Counter` = number of planes, `Dropcount 0`). Remove `LossesImpl::Latch` from `mod.rs` and its code and test; that is the only change allowed in `mod.rs`. The wrapped copy has no other `OnEvent` and no `OnReport` on any entity (this covers event 7: no abort on it).

D. The flight 0 placeholders. Replace each `// P14 G0: default pending flight 0` comment with the setting flight 0 confirmed, citing it briefly (e.g. `// flight 0: J4 confirmed`), per the 0d table: `ZoneOutImpl::CheckZone` default (Watchdog allowed), losses on 13 (decision 0011), `BudgetImpl::HeadCount` (E1, E2, E4), `CensusImpl::Spaced` (E7), relay latches as written (J4), `TraceCarrier` default Objective C0/S0 (decision 0007). No line may keep the old marker.

E. Tests. Keep every step 3 test the plan names, except: remove `shell_wraps_a_spawn_mode_template`, `shell_sweeps_repeat_mode_machinery` and `shell_losses_latch_counts_each_plane_once_across_types_4_2_0`. Change:
- `shell_losses_counts_every_plane`: type 13; counts 4 / 4 / 4 / 4 / 4 / 8.
- `shell_delete_timer_releases_the_budget`: as B (the shell's `DELETE DELAY`, its targets, its registration; `SNjj DELETE` lists exactly the plane entities).
- `shell_adds_17_mcus_per_copy` becomes `shell_adds_19_mcus_per_copy`: the 17 of plan §3.2 plus `DELETE DELAY` and `DELETE`, plus one `RTB 1` waypoint; `shell_keeps_template_rtb_timer` becomes 18. State in the test how many template MCUs were removed.
- `shell_delete_timer_still_targets_delete` becomes `shell_replaces_the_template_delete_chain`: for a built-in (it has a chain) and for an Activate-mode template regenerated by `generate_template` in the test (it has none), the copy holds exactly one `MCU_Delete`, `SNjj DELETE`, and no link to a removed MCU (`assert_links_resolve`).
Add:
- `shell_writes_only_the_losses_hooks`: every `OnEvent` in a wrapped copy is type 13 into `LOSSES`, one per plane, for all six built-ins.
- `shell_deletion_frees_the_budget` (walker, with `ReverseTargets` and the three re-trigger modes): `DONE HUB` at t; `SNjj DELETE` fires at t + 60.6 s; `FLYING` is on until then and off after; a request after that starts.
- `shell_losses_all_planes_triggers_done` must kill through the walker's `kill` (event 13), every plane of Armed Recon (escort included), and see `DONE HUB` pass once.

F. Flight 2 (Probe 2), in `probe.rs`, following flight 1P's pattern: `Flight::Two`, stem `P14_Probe_2`, a run sheet, cues, `generate_probe_2()`, an ignored writer `write_p14_probe_2`. Remove the old `write_p14_sortie_probe` from `shell.rs`; the load writer `write_p14_load_probe` moves to `probe.rs` too. Cells, per plan step 3 "Needs the user in game" except where this says otherwise:
- `T-s1` one placed HVAR sortie, started by a timer at 60 s; `T-s2` one placed AirAlert sortie at 90 s. Each has a stand-in area whose `HOT` / `COLD` relays a timer switches to HOT at 10 min, and subtitles on `START`, `DONE HUB`, `EXTEND` and `SNjj DELETE`. No Spawn-mode sortie.
- `T-k` (N = 2) and `T-k1` (N = 1) exactly as the plan's step 3 lists them, built with `budget.rs`.
- `T-l`, an aircraft lost with no damage: two single AI aircraft, each with `AiRTBDecision = 0` and breadcrumbs on its `OnEvent` 13 (through an `MCU_Counter` with `Counter 1`, `Dropcount 0`, as `LOSSES` is), 2, 0 and 5. `T-l1`: air start at 150 m and cruise speed over land near K14 with `Fuel` 0. `T-l2`: a waypoint below the top of a ridge, only if the repository holds a terrain height for the point you choose (say where it came from); otherwise leave `T-l2` out and say so. Nothing damages them.
- Close as flight 1P does: one `MCU_Delete` per aircraft entity at the end.
The load file is the acceptance plan scaled to 24 copies per side, as the plan says.

G. Documents, to match 0017 and nothing more: `handoff/P14-air-tasking.md` §3.2 (accepted shapes S1 to S4 and S7 on Activate mode; the Delete chain; event 13 only; MCU counts), §3.3 where it names the delete timer, step 3's section (test list, Probe 2 as flight 2 with `T-l`), risk g and risk u rows; `docs/src-guide.md` for `shell.rs`, `budget.rs`, `assemble.rs`.

Acceptance:
- `cargo build --offline`: 0 errors, at most 69 warnings. `cargo clippy --offline`: at most 251 warnings. New code adds no warnings.
- `cargo test --offline`: 0 failed; the total passed is 577 (before the merge) plus every step 3 test kept or added, less none of the 577 except the step 2 tests that A turns into refusal tests (list them).
- `cargo test --offline write_p14_probe_ -- --ignored` writes the flight 2 files and the load file; every earlier flight generates byte for byte the same files as before (compare against `handoff/p14-probes/*/`).
- `grep -rn "P14 G0" src/` finds nothing.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_2_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10 --name P14_Probe_2_check --title "P14 probe 2 check" --out target/p14/missions/P14_Probe_2_check --no-resave` exits 0, every `Index` unique, every reference resolved. Throwaway check script, not added to the repo.
- The step 1 baselines (`src/testdata/p14_baseline/`) do not change: Generate output is untouched.

Files in scope: `src/airtask/shell.rs`, `src/airtask/budget.rs`, `src/airtask/assemble.rs`, `src/airtask/library.rs`, `src/airtask/probe.rs`, `src/airtask/mod.rs` (only removing `LossesImpl::Latch`), `src/airtask/place.rs` (tests only, if a test relies on a Spawn-mode template), `handoff/P14-air-tasking.md` and `docs/src-guide.md` (the sections named in G).

Do not touch: every other file. In particular `src/template.rs`, `src/flights.rs`, `src/airtask/testkit.rs`, `src/trace.rs`, `src/missionlog.rs`, `src/ui.rs`, `src/locale.rs`, `src/testdata/` (all of it, including `p14_probe_inputs/` and `p14_baseline/`), `TemplateExamples/`, `handoff/p14-probes/`, every other file under `handoff/` and `docs/`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The orchestrator has checked out `codex/p14-3b` in this worktree. Edit files in the working tree only. No git command that writes (no add, commit, branch, checkout, stash, reset, restore, worktree, merge). `git status`, `git diff` and `git log` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not change a test to match a count that disagrees with this prompt or the plan; report the count and the reason.
</completeness_contract>

<verification_loop>
Make the probe byte baseline before you edit `probe.rs`. Then run `cargo build --offline`, `cargo clippy --offline`, `cargo test --offline`, `cargo test --offline write_p14_probe_ -- --ignored`, the byte comparison and the mission builder check, and read the counts from their output. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor. Never run the mission builder without `--no-resave`.
</verification_loop>

<action_safety>
Write in the style the code has now: const tables with named fields that read like the plan's tables, shared helpers, named constants, no flag arguments, no behaviour keyed on a bare number, failures that name the template, copy or missing part. Tests state their own literals. No refactor, rename or reformat of code outside the change.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Where the plan and the code disagree on names, indexes or values, follow the code for facts and decision 0017 then the plan for semantics, and list each difference.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary, five lines at most.
2. Command results: build and clippy warnings, tests passed / failed / ignored, the byte comparisons, the mission builder check, `grep "P14 G0"`.
3. Every file changed, one line each on why.
4. Tests removed, renamed, changed and added, by name.
5. The MCUs one wrapped HVAR copy adds and removes (names and counts); the budget's MCUs at N = 3 and N = 1.
6. Each `P14 G0` line and what replaced it.
7. Flight 2: the run sheet, a schedule of every command and pulse in time order, and for `T-l2` where the height came from or why it is left out.
8. Choices you made where this prompt gave no exact shape, differences between plan and code, residual risks.
</structured_output_contract>
