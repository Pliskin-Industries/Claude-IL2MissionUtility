# STATE — IL-2 Mission Utility (Pliskin-Industries fork)

> Read this first. It is **overwritten** at the end of every session and stays one page.
> Last updated: 2026-10-28: decision 0037 A built and merged to `main` (PR #39): with Air Tasking in the mission, Map Generate moves the Map's own AI planes off the player countries.
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
| `main` | v0.6.0 + upstream v0.7.0; P14 steps 1a to 8 (PRs #31 to #33); acceptance flights A0 and A1, decision 0038 (PR #34); step 9 (PR #35); decisions 0026, 0037, 0040 (PRs #36, #37); step 9T (PR #38); decision 0037 A built (PR #39); decisions 0001 to 0041 | equal to `origin/main` |

The merged branches (`claude/p14-acceptance-early`, `claude/p14-acceptance-a1-545b0f`, `claude/p14-shell-quiet-a24002`, `claude/sad-cannon-eba4b4`, `claude/p14-decisions-0037-0040`, `claude/p14-decision-0026`, `claude/gracious-euler-f73335`, `claude/p14-0037-map-ai-countries`) can be deleted once their idle sessions are archived; their worktrees are under `.claude/worktrees/`.

Last recorded test run on the decision 0037 branch (`e8f5eb8`): **1029 passed, 0 failed, 32 ignored** (`cargo test --offline`, 2026-10-28, re-run by the coordinating session) (`cargo test --offline`, 2026-10-28, re-run by the coordinating session). Warnings: 81 build, 306 clippy. Flown-probe byte regeneration, `existing_outputs_match_baseline` and `map_generate_without_air_matches_baseline` pass. Record each new run on this line.

## P14 Air Tasking ("Wing Commander")

Plan: `handoff/P14-air-tasking.md` (§8 steps, §10 build order, §2.1 user decisions U1 to U24). Flight results: `handoff/P14-probe-results.md` (flights 0 to 2J, A0, A1). Facts that cost a flight: `docs/il2-loader-and-log-facts.md`. Step notes: `handoff/sessions/` (`…-p14-step4` to `…-step8`, `2026-10-27-p14-acceptance-a0.md`, `…-a1.md`, `2026-10-28-p14-step9.md`, `…-step9t.md`, `…-0037.md`).

- **Built (on `main`):** sortie shell, budget and assembly (step 3); threshold (step 4); clock (step 5); zone front end (step 6); export (step 7, `src/airtask/export.rs`); the Air Tasking rail tab (step 8, Ctrl 7, `src/ui/air.rs`); the Map dock's Air tab, and Map Generate writing Air Tasking when the plan is included (step 9, `src/ui/air.rs`); the header's Trace build checkbox, the Replay window and the Map's Replay layer (step 9T; run live on the A1 logs); with Air Tasking in the mission, the Map's own AI planes move from 601 to 602 and 501 to 503 (decision 0037 A; not yet flown). An app-made base map with Air Tasking loaded in the Mission Editor without an error (2026-10-28); it has not been hosted or flown.
- **Acceptance test moved ahead of step 9** (decision 0036). **Flight A0 flown 2026-10-27** (no pilot): the whole group loads; the clock opens on time; nothing launches over an empty area. **Flight A1 flown 2026-10-28** (Claude, a NATO player at Kimpo K14 inside the DPRK area): the DPRK cycle runs end to end (HOT and presence checks, a zone start and a clock start of Il-10 copies, transit, on station, the budget counting two, deletion); no dead links.
- **A1 findings:** (1) a player who spawns inside an area (ground start) is not seen; only flying in across the ring counts. (2) When the last watched plane dies, `Zone Out` fires and ends the sorties. (3) **After that early end, the copy's timers keep running** (`Zone IN`, `EXTEND`, `ON STATION`, `COLD` logged after deletion; no second `DONE HUB`): harmless on trace, and fixed by decision 0038: a copy's `DONE HUB OFF` now also switches off its timers and zones (Generate output changes there only; MCU count unchanged). The next acceptance flight checks that nothing from a copy logs after its `DONE HUB` except `DELETE DELAY`. (4) Il-10 gunners shot down the player orbiting at 500 m. (5) This PC cannot spawn twice in one session, so the parked case and a respawn were not flown.
- **Formations:** AI losses in flights 2C to 2I were wingmen crossing their leader, a game bug (0021). Workarounds: Safe density (0019), Pairs at Loose (0020 A) except a Cover flight (0022 A2).
- **Remaining steps:** piloted acceptance A2 → 10 docs → 12 multi-player. Upstream integration (decision 0026: Template Builder's Strike/CAP as sorties, his target types) waits for eszyman's answer to the proposal.
- **Step 9 notes:** the UI's checks run one `airtask::check` per plan change (a two-entry cache: the live plan and the settings draft). Load clears the Air Tasking pool and both undo histories and keeps the library and settings. Map undo carries the plan only for dock edits (decision 0039).
- **Still unseen in the game:** the single extension and turn for home (P9), a budget refusal, a Yak copy, a player who lands inside an area.

## Waiting on the user

1. **Send the Air Tasking proposal to eszyman?** Draft: `handoff/upstream-air-tasking-proposal-draft.md` (decision 0026, accepted 2026-10-28: Air Tasking orchestrates; Template Builder's flights and Strike/CAP become its sorties; his target types are adopted). Where to post it (new issue or issue #1) and when is yours. Integration waits for his answer; the rest of the build does not.
2. **Three game bug reports are drafted, not posted** (`handoff/il2-bug-report-drafts.md`).
3. **The six built-in sorties** still carry the old Delete list in their files; Air Tasking replaces it on the copies it places (0017).

## Next steps

1. A piloted session by the user (one spawn per session on this PC; start a Tacview track with LCtrl+R), on a mission made by the app itself as a trace build, read afterwards with Replay: fly in, land or loiter, see P9 and a budget refusal, and check decisions 0038 and 0037 (the Map's own fighters start no sorties); a NATO-side flight (A2). Then step 10.

## Traps

- **Send nothing to the upstream author** (`eszyman/IL2MissionUtility`: issues, comments, files, load tests, patches) until Air Tasking has a working concept and UI (user, 2026-10-18). Pulling and merging his commits is fine. The proposal draft is the first thing to send, and only on the user's word.
- **Decision numbers:** sessions write records in parallel. Before writing one, check `main` **and** every open branch (`git ls-tree --name-only <branch> docs/decisions/`). Taken: 0001 to 0041. Next free: **0042**.
- Before generating any probe or mission file, read `docs/il2-loader-and-log-facts.md`. Each item there cost a flight attempt.
- `.gitattributes` is `* -text` because `.Group` fixtures are line-ending sensitive. Do not remove it.
- Do not push `v*` tags casually: they start the release build.
- Nothing is pushed or merged to `main` without the user (`docs/decisions/0005`). A hook blocks `git push` (even a branch delete) while the checkout is on `main`.
- The game here is 1.005 (`261013`). Its `MissionResaver.exe` crashes: build probes with `--no-resave`. This PC is a test bed: its load times and tick delay are not used.
- Tacview on this PC: set `tacviewrecord = 1` with the game closed, and start a track with LCtrl+R.
- Flown probe flights regenerate byte for byte from frozen inputs (`src/testdata/p14_probe_inputs/`, `handoff/p14-probes/`), except those frozen as flown (2F, 2H, 2I and the acceptance flights A0, A1, since decision 0038): those are checked by parse-and-links tests only. Never edit those files or re-run their writers. Flight names are never reused.
- Only one session writes `STATE.md` at a time; other sessions put their facts in a session note in `handoff/sessions/`.
- Stage is not declared yet (the session-start check treats the repo as `prototype`); `/promote` sets it.
