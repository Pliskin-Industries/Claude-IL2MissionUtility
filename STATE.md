# STATE — IL-2 Mission Utility (Pliskin-Industries fork)

> Read this first. It is **overwritten** at the end of every session and stays one page.
> Last updated: 2026-10-27, on `claude/p14-acceptance-early`: PR #33 (upstream v0.7.0, P14 steps 7 and 8) merged; acceptance flight A0 flown.
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
| `main` | v0.6.0 + upstream v0.7.0; P14 steps 1a to 8 (PRs #31, #32, #33); decisions 0001 to 0035 (0026 Proposed) | `b5acb35`, equal to `origin/main` |
| `claude/p14-acceptance-early` | decision 0036, acceptance flight A0 (frozen inputs, logs `real_44`, results), this file | Local, not pushed; no code change |

All merged branches and worktrees were removed on 2026-10-27. Three empty folders under `.claude/worktrees/` stay locked by idle sessions until those are archived.

Last recorded test run on `main` (`b5acb35`): **976 passed, 0 failed, 31 ignored** (`cargo test --offline`, 2026-10-27). Warnings: 81 build, 306 clippy. Flown-probe byte regeneration, `existing_outputs_match_baseline` and `map_generate_without_air_matches_baseline` pass. Record each new run on this line.

## P14 Air Tasking ("Wing Commander")

Plan: `handoff/P14-air-tasking.md` (§8 steps, §10 build order, §2.1 user decisions U1 to U24). Flight results: `handoff/P14-probe-results.md` (flights 0 to 2J, A0). Facts that cost a flight: `docs/il2-loader-and-log-facts.md`. Step notes: `handoff/sessions/` (`…-p14-step4` to `…-step8`, `2026-10-27-p14-acceptance-a0.md`).

- **Built (on `main`):** sortie shell, budget and assembly (step 3); threshold (step 4); clock (step 5); zone front end (step 6); export (step 7, `src/airtask/export.rs`); the Air Tasking rail tab (step 8, Ctrl 7, `src/ui/air.rs`).
- **Not wired yet:** Map Generate still passes `air_packs: Vec::new()` in `generate_front_file` (`src/ui/map.rs`). Step 9 replaces it.
- **Acceptance test moved ahead of step 9** (decision 0036). **Flight A0 flown 2026-10-27** (no pilot): the whole Air Tasking group (84 aircraft, 1,092 MCUs, 40 check zones) loads; clock slots opened on time and the slot latch held; nothing launched over an empty area; no dead links. Next: the piloted sessions, A1 onward (the user, solo, plan §8 step 11).
- **Before A1:** the mission needs player air starts (`--spawns`). And the base map holds the Map's own NATO Fighters Wave 1 (18 fighters of country 601, launched when a DPRK-side plane comes within 16 km of (110,000; 280,000)); being country 601, it can start DPRK sorties like a NATO player would.
- **Formations:** AI losses in flights 2C to 2I were wingmen crossing their leader, a game bug (0021). Workarounds: Safe density (0019), Pairs at Loose (0020 A) except a Cover flight (0022 A2).
- **Remaining steps:** acceptance A1+ → 9 Map dock Air tab and Map Generate wiring (then one load check of an app-made mission) → 9T replay UI → 10 docs → 12 multi-player.
- **Step 9 notes:** `validate`, `totals` and `preview` each run a full build (cache per plan fingerprint); Load clears Map objectives, so pool entries aimed at them become orphaned.
- **Still unseen in the game:** a sortie start, its extension and its turn for home (A1).

## Waiting on the user

1. **For A1:** which plane and air start per side, and whether the acceptance base map keeps the NATO Fighters Wave 1 pack.
2. **Push `claude/p14-acceptance-early` and merge it** (records only).
3. **Decision 0026 (Proposed):** Air Tasking schedules upstream's strikes and CAPs instead of placing its own. 0028 (accepted) points the same way; whether 0028 supersedes 0026 is yours.
4. **Three game bug reports are drafted, not posted** (`handoff/il2-bug-report-drafts.md`).
5. **The six built-in sorties** still carry the old Delete list in their files; Air Tasking replaces it on the copies it places (0017).

## Next steps

1. Settle item 1, build the A1 mission with spawns, and fly A1 (DPRK) and A2 (NATO), about 45 min each.
2. Fix anything they find (re-fly under a new flight name), then step 9, 9T, 10.

## Traps

- **Send nothing to the upstream author** (`eszyman/IL2MissionUtility`: issues, comments, files, load tests, patches) until Air Tasking has a working concept and UI (user, 2026-10-18). Pulling and merging his commits is fine.
- **Decision numbers:** sessions write records in parallel. Before writing one, check `main` **and** every open branch (`git ls-tree --name-only <branch> docs/decisions/`). Taken: 0001 to 0036. Next free: **0037**.
- Before generating any probe or mission file, read `docs/il2-loader-and-log-facts.md`. Each item there cost a flight attempt.
- `.gitattributes` is `* -text` because `.Group` fixtures are line-ending sensitive. Do not remove it.
- Do not push `v*` tags casually: they start the release build.
- Nothing is pushed or merged to `main` without the user (`docs/decisions/0005`). A hook blocks `git push` (even a branch delete) while the checkout is on `main`.
- The game here is 1.005 (`261013`). Its `MissionResaver.exe` crashes: build probes with `--no-resave`. This PC is a test bed: its load times and tick delay are not used.
- Tacview on this PC: set `tacviewrecord = 1` with the game closed, and start a track with LCtrl+R.
- Flown probe flights regenerate byte for byte from frozen inputs (`src/testdata/p14_probe_inputs/`, `handoff/p14-probes/`). Never edit those files or re-run their writers. Flight names are never reused. A0's frozen files (`handoff/p14-probes/acceptance-a0/`) are not yet under the regeneration test.
- Only one session writes `STATE.md` at a time; other sessions put their facts in a session note in `handoff/sessions/`.
- Stage is not declared yet (the session-start check treats the repo as `prototype`); `/promote` sets it.
