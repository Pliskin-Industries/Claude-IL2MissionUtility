# STATE — IL-2 Mission Utility (Pliskin-Industries fork)

> Read this first. It is **overwritten** at the end of every session and stays one page.
> Last updated: 2026-10-12 (late), after the user's four rulings; flight 1H is built and waits for the screen.
> Rules for keeping it current: `CLAUDE.md`. History: `handoff/archive/`.

## What this is

A Windows desktop tool (Rust, egui) that builds IL-2 Great Battles **Korea** mission `.Group` files so the mission editor needs less hand wiring. Module map: `docs/src-guide.md`. End-user manual: `USER_MANUAL.md`.

- **Fork:** `Pliskin-Industries/Claude-IL2MissionUtility` (`origin`). **Upstream:** `eszyman/IL2MissionUtility` (`upstream`, the original creator).
- **Backlog:** GitHub Issues on the fork (#1 to #30).
- **Build and test:** `cargo test --offline`, then `cargo build --release`. A fresh machine needs `cargo fetch`, rustup and the Visual Studio C++ Build Tools first.
- **Names:** the user calls P14 Air Tasking **"Wing Commander"** or **"WG/CC"**. That name is not in the code.

## Where the work is

| Branch | Worktree folder | Holds |
|---|---|---|
| `main` | `Claude-IL2MissionUtility` | v0.6.0 plus the P14 plan. 3 ahead and 1 behind `origin/main`. |
| `claude/p14-air-tasking` | `Claude-IL2MissionUtility-fix` | **The active branch.** P14 steps 1a, 1, 1b, 0T, 0b and 2, and the flight 0 results. Has commits that are not pushed. |
| `codex/p14-3` | `Claude-IL2MissionUtility-0t` | P14 step 3, built but **held** and not merged. |
| `trial/upstream-merge` | `Claude-IL2MissionUtility-merge` | Uncommitted trial merge of upstream. Do not build on it. |
| `codex/p14-0b` | `Claude-IL2MissionUtility-0b` | Already merged. A leftover worktree. |
| `claude/docs-layout-trial` | `Claude-IL2MissionUtility-docs` | This document layout. Merged 2026-10-10. A leftover worktree. |

Last recorded test run on the active branch: 545 passed, 0 failed, 11 ignored (`cargo test --offline`, 2026-10-12, after step 0f). Warnings on 2026-10-12: 69 build, 251 clippy. Record each new run on this line.

## In progress: P14 Air Tasking (issues #28, #29)

Plan: `handoff/P14-air-tasking.md` (long; read §8 for steps and §10 for the build order). Its user decisions are §2.1 (U1 to U24).

- **Flight 0 was flown on 2026-10-03 and gate G0 passed.** Results: `handoff/P14-probe-results.md`. Video check: `References/Recordings/flight0-harvest/README.md`.
- **Step 3 is unblocked but not merged.** Its commit (`3224286`) reports 583 passed, 11 ignored. It still needs the lines marked `// P14 G0: default pending flight 0` settled, the probe fixes merged in (it is 24 commits behind), the shape and string guards extended to Probe 2, and a full review.
- **Flight 1A was flown on 2026-10-12 by Claude, as a flying check with an air start.** The T-c cells and ZA4 gave clear answers (c, the re-check and h confirmed; o = fires once; a zone fires for a player). Risk n (cruise speed) was not measured, and the country-filter cells were not in the build. Results: `handoff/P14-probe-results.md`, last section. **The user ruled that it counts as flight 1A: gate G1 is met and step 4 may start** (`docs/decisions/0009`).
- **Step 0e is built, merged (`aa3b048`) and flown.** Flights 1G, 1N and 1D were flown by Claude on 2026-10-12. Results: `handoff/P14-probe-results.md` (its last section is a one-table summary of step 0). Headlines: the country filter works on both sides; a lost aircraft raises OnEvent **13**, not 4; AI F-80Cs fly 733 km/h against the plan's 732; **one aircraft outlived its Delete**, which matters to the budget's hard limit (U22).
- **The user ruled on four items on 2026-10-12.** (1) Decision 0008 is Accepted, option 4: players on 601 or 501, the app's AI on 602 or 503. (2) Losses count on event 13 (decision 0011), and the Delete gets one more probe. (3) The step 0e style findings were fixed (`67c5a13`, merged `4cd5d03`). (4) Ceilings: see "Waiting on the user", item 1.
- **Step 0f, flight 1H (the Delete probe), is built, reviewed and merged (`b192358`). It is not flown yet.** Ten AI pairs, no pilot, 6 minutes. Spec: plan step 0f. The mission `P14_Probe_1H_20261012` is in the game's Dogfight folder. Build command: as flight 1G in `handoff/p14-probes/flight1a/README.md`, with the 1H probe file.
- **Found while reviewing step 0f, not yet tested in game:** `generate_template` fills `Trigger Delete` with the Plane blocks' indexes, where all 20 editor-saved examples list entities. The six built-in sorties carry the same lists. Flight 1H's pairs D8, D9 and D6 show whether such a Delete removes anything.
- **The game's handbook is harvested for the 12 aircraft:** `docs/game-handbook/aircraft-characteristics.md`. **Ceilings from operating documents** are in `docs/historical-reference/aircraft-service-ceilings.md`, with the game's figures beside them. No code was changed.
- **Testing on this PC, with Claude flying (2026-10-12).** The game and editor run here on integrated graphics, and Claude can drive both. **Flight 0 was re-flown here as a calibration and matches the 2026-10-03 results** (`handoff/P14-probe-results.md`, last section), so this PC can fly the next probes. The Complex Trigger reference file is saved in `TemplateExamples/Reference/`. How to drive the game, and what is untested: `handoff/sessions/2026-10-12-il2-korea-on-this-pc.md`. Logs: `src/testdata/missionlog/real_4/`. Claude can also fly level legs, orbits and turns by keyboard.

## Waiting on the user

0. **The screen, for flight 1H.** About 15 minutes with nothing else running. The game was found closed at 21:45 on 2026-10-12; it must be started and signed in first (Claude does not type passwords).
1. **Which ceilings go into `src/model_spec.rs`.** The user asked for figures from operating handbooks. Real documents were found for five aircraft (F-51D, B-29, C-47B, and the official USAF history for the F-80C and F-84E); the other seven have secondary sources only. Choice: change all twelve with each source noted, or only the five. Also which C-47B figure (8,047 m with the high blower, 6,706 m without). Five ceilings would go down, so a saved template with an aircraft above its new ceiling may be capped when reopened. Cruise speeds stay as they are.
1b. **Decision 0012 (`Proposed`): which event counts a destroyed aircraft in Template Builder and Fighter Pack.** Both write type 4, which flight 1G never saw fire. Options: keep 4, switch to 13, or attach both. Any change alters Generate output.
2. **Whether objective breadcrumbs show as map markers.** Nothing shows near K14 on the briefing map (2026-10-12). The far map corner, where the breadcrumbs sit, was not inspected.
3. **Go-ahead for the next steps below.** They were proposed on 2026-10-03 and not confirmed.
4. **Older confirmations still listed in the plan:** §10 item 23 (a smaller Auto-fill) and the "For the user to confirm" list.

## Next steps (proposed order)

1. Finish step 3 as listed above and merge it into `claude/p14-air-tasking`.
2. Merge upstream. It is now 6 commits ahead, including v0.7.0 and the split of the UI into per-tab files. Trial result and the one expected baseline change: `handoff/sessions/2026-10-07-flight-0-follow-up.md`.
3. Fly flight 1H, read it with `il2_mission_utility replay`, and record the result in `handoff/P14-probe-results.md` (logs to `src/testdata/missionlog/real_9/`, flown files to `handoff/p14-probes/flight1h/`). Then settle step 3: it still counts losses on type 4 and must move to 13, and its clean-up must use whichever Delete wiring flight 1H shows to work.
4. Change the ceilings in `src/model_spec.rs` once the user picks (Astra writes it, decision 0005).
5. Scope step 4 (threshold) on a Complex Trigger filtered by country (decision 0008).

## Traps

- Before generating any probe or mission file, read `docs/il2-loader-and-log-facts.md`. Each item there cost a flight attempt.
- `.gitattributes` is `* -text` because `.Group` fixtures are line-ending sensitive. Do not remove it.
- Do not push `v*` tags casually: they start the release build.
- Nothing is pushed or merged to `main` without the user (`docs/decisions/0005`).
- This PC's game is version `261002` since 2026-10-12. A mission built here needs a server on the same version.
