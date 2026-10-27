# STATE — IL-2 Mission Utility (Pliskin-Industries fork)

> Read this first. It is **overwritten** at the end of every session and stays one page.
> Last updated: 2026-10-27, on `claude/p14-integration-7-8`: the upstream merge, P14 step 7 and P14 step 8 combined for one PR to `main`.
> Rules for keeping it current: `CLAUDE.md`. History: `handoff/archive/` and git log.

## What this is

A Windows desktop tool (Rust, egui) that builds IL-2 Great Battles **Korea** mission `.Group` files so the mission editor needs less hand wiring. Module map: `docs/src-guide.md`. End-user manual: `USER_MANUAL.md`.

- **Fork:** `Pliskin-Industries/Claude-IL2MissionUtility` (`origin`). **Upstream:** `eszyman/IL2MissionUtility` (`upstream`, the original creator).
- **Backlog:** GitHub Issues on the fork (#1 to #30). P14 is #28 (feature) and #29 (probe flights).
- **Build and test:** `cargo test --offline`, then `cargo build --release`. A fresh machine needs `cargo fetch`, rustup and the Visual Studio C++ Build Tools first.
- **Names:** the user calls P14 Air Tasking **"Wing Commander"** or **"WG/CC"**. That name is not in the code.

## Where the work is

| Branch | Holds | State |
|---|---|---|
| `main` | v0.6.0; P14 steps 1a to 3 with flights 0 to 2I (PR #31); steps 4 to 6 with flight 2J (PR #32); decisions 0001 to 0025, 0029, 0030 | `141af6e`, equal to `origin/main` |
| `claude/p14-integration-7-8` | `main` + upstream v0.7.0 (`claude/upstream-merge-0.7`, decision 0026 Proposed) + step 8 rail tab (`codex/p14-8`; 0027, 0028, 0031) + step 7 export (`codex/p14-7`; 0032 to 0035) + the three step 8 UI tests that waited for step 7, switched on | Pushed; **PR to `main` waits for the user to merge** |
| `claude/upstream-merge-0.7`, `codex/p14-7`, `codex/p14-8` | the three source branches | Fully contained in the integration branch; delete after the PR merges |
| `claude/state-2026-10-26`, `claude/state-refresh-2026-10-18` | earlier STATE drafts | Superseded by this file; delete (`git branch -D`) |
| `trial/upstream-merge` | the old trial merge | Superseded; its worktree can go |

Last recorded test run on `claude/p14-integration-7-8`: **976 passed, 0 failed, 31 ignored** (`cargo test --offline`, 2026-10-27). Warnings: 81 build, 306 clippy, the same as the upstream-merge base. Flown-probe byte regeneration, `existing_outputs_match_baseline` and `map_generate_without_air_matches_baseline` pass. On `main`: 802 / 0 / 30; 68 / 250. Record each new run on this line.

## P14 Air Tasking ("Wing Commander")

Plan: `handoff/P14-air-tasking.md` (§8 steps, §10 build order, §2.1 user decisions U1 to U24). Flight results: `handoff/P14-probe-results.md` (flights 0 to 2J). Facts that cost a flight: `docs/il2-loader-and-log-facts.md`. Step notes: `handoff/sessions/2026-10-18-p14-step4.md`, `…-step5.md`, `…-step8.md`, `2026-10-22-p14-step6.md`, `2026-10-26-p14-step7.md`.

- **Built (on `main`):** sortie shell, budget and assembly (step 3; 0017, 0018); threshold (step 4; 0023 to 0025); clock (step 5; 0029 B); zone front end (step 6; 0030 C).
- **Built (on the integration PR):** step 7 export (`src/airtask/export.rs`: `build_air_packs`, `validate`, `totals`, `preview`; one pack rooted at `Air Tasking`, 0035; totals limits 240 aircraft / 2300 MCUs / 48 check zones as warnings, 0032); step 8 Air Tasking rail tab (Ctrl 7, `src/ui/air.rs`; Generate File writes the `Air Tasking` group, disabled until `validate` has no Error); the Map Load arrow-tip fix (252 m offset).
- **Not wired yet:** Map Generate still passes `air_packs: Vec::new()` in `generate_front_file` (`src/ui/map.rs`). Step 9 replaces it with the real call.
- **Formations:** AI losses in flights 2C to 2I were wingmen crossing their leader, a game bug (0021). Workarounds: Safe density (0019), Pairs at Loose (0020 A) except a Cover flight (0022 A2).
- **Remaining steps:** 9 Map dock Air tab and the Map Generate wiring (in upstream's `src/ui/map.rs`; also: Load clears Map objectives, so pool entries aimed at them become orphaned) → 9T replay UI → 10 docs → 11 solo acceptance flight (`write_p14_acceptance_mission`, ignored test; trace build, 0034) → 12 multi-player. One at a time.
- **Performance note for step 9:** `validate`, `totals` and `preview` each run a full build; cache them per plan fingerprint.
- **Still unseen in the game:** a sortie's extension and its turn for home.

## Waiting on the user

1. **Merge the integration PR into `main`.** It changes what Generate writes only where upstream v0.7.0 changed it (merge plan: `handoff/sessions/2026-10-18-upstream-merge.md`); the Air Tasking tab's output is new.
2. **Decision 0026 (Proposed):** Air Tasking schedules upstream's strikes and CAPs instead of placing its own. 0028 (accepted) points the same way; whether 0028 supersedes 0026 is yours.
3. **Three game bug reports are drafted, not posted** (`handoff/il2-bug-report-drafts.md`).
4. **The six built-in sorties** still carry the old Delete list in their files; Air Tasking replaces it on the copies it places (0017).

## Next steps

1. Merge the PR (item 1), then delete the source branches above.
2. Step 9 (Map dock Air tab, Map Generate wiring), then 9T, then 10; then fly step 11.

## Traps

- **Send nothing to the upstream author** (`eszyman/IL2MissionUtility`: issues, comments, files, load tests, patches) until Air Tasking has a working concept and UI (user, 2026-10-18). Pulling and merging his commits is fine.
- **Decision numbers:** sessions write records in parallel. Before writing one, check `main` **and** every open branch (`git ls-tree --name-only <branch> docs/decisions/`). Taken: 0001 to 0035. Next free: **0036**.
- Before generating any probe or mission file, read `docs/il2-loader-and-log-facts.md`. Each item there cost a flight attempt.
- `.gitattributes` is `* -text` because `.Group` fixtures are line-ending sensitive. Do not remove it.
- Do not push `v*` tags casually: they start the release build.
- Nothing is pushed or merged to `main` without the user (`docs/decisions/0005`).
- The game here is 1.005 (`261013`). Its `MissionResaver.exe` crashes: build probes with `--no-resave`. This PC is a test bed: its load times and tick delay are not used.
- Tacview on this PC: set `tacviewrecord = 1` with the game closed, and start a track with LCtrl+R.
- Flown probe flights regenerate byte for byte from frozen inputs (`src/testdata/p14_probe_inputs/`, `handoff/p14-probes/`). Never edit those files or re-run their writers. Probe flight names are never reused.
- Only one session writes `STATE.md` at a time; other sessions put their facts in a session note in `handoff/sessions/`.
- Stage is not declared yet (the session-start check treats the repo as `prototype`); `/promote` sets it.
